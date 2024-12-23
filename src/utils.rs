use std::env;
use std::error::Error;

use chrono::Local;
use fern::colors::ColoredLevelConfig;
use log::{debug, error, info, warn};
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::Content::Text;
use openai_api_rs::v1::chat_completion::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionMessageForResponse,
    ChatCompletionRequest, MessageRole,
};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, ReplyParameters};
use teloxide::utils::command::BotCommands;
use tokio::time::{self, Duration};

use crate::commands::{handle_command, AiSource, Command, ModelInfo, SystemMethod};

pub fn init_logging() {
    let colors = ColoredLevelConfig::new();
    fern::Dispatch::new()
        // Custom format: [Timestamp Level Target] Message
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                colors.color(record.level()),
                record.target(),
                message
            ))
        })
        // Set default log level to Warn
        .level(log::LevelFilter::Warn)
        // Override log level for our module to Debug
        .level_for(env!("CARGO_PKG_NAME"), log::LevelFilter::Debug)
        // Output to stdout
        .chain(std::io::stdout())
        // Output to a log file
        .chain(fern::log_file("output.log").unwrap())
        // Apply the configuration
        .apply()
        .expect("Failed to initialize logging");
}

pub async fn init_db() -> Result<SqlitePool, sqlx::Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    debug!("Database URL: {}", database_url);

    // Initialize the connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    info!("Connected to the SQLite database.");

    // Ensure the messages table exists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            chat_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            message_id INTEGER NOT NULL PRIMARY KEY,
            reply_to INTEGER,
            model TEXT,
            content TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Clear the messages table for development
    // sqlx::query!("DELETE FROM messages")
    //     .execute(&pool)
    //     .await?;

    Ok(pool)
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    pool: &SqlitePool,
    me: &teloxide::types::Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let user_id = msg.from.as_ref().map(|u| u.id);
    if user_id.is_none() {
        warn!("User ID not found in message, ignoring.");
        return Ok(());
    }
    // let user_id = user_id.unwrap(); // TODO: ratelimit

    // Check if the message is a command
    if let Some(text) = msg.text() {
        if let Ok(cmd) = Command::parse(text, me.username()) {
            handle_command(bot, msg, cmd, pool).await?;
            return Ok(());
        }
    }

    // Check if the message is a reply to a known bot response
    if let Some(reply) = msg.reply_to_message() {
        if let Some(model) = get_model_from_msg(reply, pool).await {
            let model_info = ModelInfo {
                model_id: model.0,
                model_provider: model.1,
                system_prompt: None,
            };
            handle_ai(bot, msg, pool, model_info).await?;
            return Ok(());
        }
    }

    // Check if the message is a caption
    if let Some(caption) = msg.caption() {
        if let Ok(cmd) = Command::parse(caption, me.username()) {
            handle_command(bot, msg, cmd, pool).await?;
            return Ok(());
        }
    }

    // If none of the above, do nothing
    Ok(())
}

pub async fn handle_stats(
    bot: Bot,
    msg: Message,
    pool: &SqlitePool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id = msg.chat.id;
    let usage_stats = top_usage_stats(pool, Some(chat_id)).await?;
    let command_stats = top_command_stats(pool, Some(chat_id)).await?;
    let stats = format!("{}\n\n{}", usage_stats, command_stats);
    bot.send_message(chat_id, &stats)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    info!("Sent usage statistics to chat ID: {}", chat_id);
    Ok(())
}

async fn get_model_from_msg(msg: &Message, pool: &SqlitePool) -> Option<(String, AiSource)> {
    let message_id = msg.id.0 as i64;
    let chat_id = msg.chat.id.0;

    let row = sqlx::query!(
        r#"
        SELECT model, provider FROM messages
        WHERE chat_id = ? AND message_id = ? AND user_id = 0
        "#,
        chat_id,
        message_id
    )
    .fetch_optional(pool)
    .await
    .ok()?;

    if let Some(record) = row {
        let model = record.model?;
        let provider = record.provider?;
        let source = AiSource::from_string(&provider);
        if source.is_none() {
            error!("Unknown provider: {}", provider);
            return None;
        }
        let source = source.unwrap();
        Some((model, source))
    } else {
        None
    }
}

pub async fn handle_ai(
    bot: Bot,
    msg: Message,
    pool: &SqlitePool,
    model_info: ModelInfo,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Extract and format the prompt, including conversation history if replying
    let mut conversation_history: Vec<ChatCompletionMessage> = Vec::new();
    if let Some(reply) = msg.reply_to_message() {
        if get_model_from_msg(reply, pool).await.is_some() {
            // Known message, fetch conversation history
            conversation_history = get_conversation_history(msg.chat.id, reply.id, pool).await?;
            // add the user's prompt to the conversation history
            conversation_history.push(ChatCompletionMessage {
                role: MessageRole::user,
                content: Text(get_message_content(&msg)),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }
    if conversation_history.is_empty() {
        debug!("No conversation history found, extracting prompt from message.");
        if model_info.system_prompt.is_none() {
            // Add user prompt, no system prompt or it was already injected
            conversation_history.push(ChatCompletionMessage {
                role: MessageRole::user,
                content: Text(extract_prompt(&msg).await),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        } else {
            let system_prompt_info = model_info.system_prompt.as_ref().unwrap();
            let system_prompt = system_prompt_info.0.clone();
            match system_prompt_info.1 {
                SystemMethod::System => {
                    // Add system prompt as first message
                    conversation_history.push(ChatCompletionMessage {
                        role: MessageRole::system,
                        content: Text(system_prompt),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                    // Add user prompt as second message
                    conversation_history.push(ChatCompletionMessage {
                        role: MessageRole::user,
                        content: Text(extract_prompt(&msg).await),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                SystemMethod::Inject => {
                    // Add user prompt as first message with prefix system prompt
                    let system_prompt = system_prompt_info.0.clone();
                    conversation_history.push(ChatCompletionMessage {
                        role: MessageRole::user,
                        content: Text(format!("{}{}", system_prompt, extract_prompt(&msg).await)),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
            }
        }
    }

    info!("Conversation history: {:?}", conversation_history);
    if msg.chat.id == ChatId(5337682436) {
        // Debug info to show conversation history
        let mut full_str = String::new();
        for message in &conversation_history {
            let content = match &message.content {
                Text(text) => text,
                _ => {
                    full_str.push_str("<unsupported content type>\n");
                    continue;
                }
            };
            full_str.push_str(&format!("{:?}: {}\n", message.role, content));
        }
        bot.send_message(msg.chat.id, &full_str)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    }

    // Send initial typing action
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;
    debug!("Sent typing action to chat ID: {}", msg.chat.id);

    // Save the user's prompt
    save_message(pool, &msg, None).await?;
    info!("Saved user's prompt message.");

    // Create a watch channel to signal the typing indicator task when done
    let (tx, mut rx) = tokio::sync::watch::channel(false);
    debug!("Created watch channel for typing indicator.");

    // Clone bot and chat id for the typing indicator task
    let bot_clone = bot.clone();
    let chat_id = msg.chat.id;

    // Spawn the typing indicator background task
    tokio::spawn(async move {
        loop {
            // Check if we should stop
            if *rx.borrow() {
                debug!("Stopping typing indicator task for chat ID: {}", chat_id);
                break;
            }
            // Send typing action
            if let Err(e) = bot_clone
                .send_chat_action(chat_id, ChatAction::Typing)
                .await
            {
                error!("Failed to send chat action: {}", e);
                break;
            }
            debug!("Sent periodic typing action to chat ID: {}", chat_id);
            // Wait for 5 seconds or until notified
            tokio::select! {
                _ = time::sleep(Duration::from_secs(5)) => {},
                _ = rx.changed() => {
                    break;
                }
            }
        }
    });

    // Generate the AI response
    let max_tokens = 512; // 1 token is around 4 chars
    let temperature = 1.0;
    let model_url = model_info.model_provider.to_url();
    let provider_name = model_info.model_provider.to_string();
    let model_id = model_info.model_id.clone();
    let api_key = model_info.model_provider.api_key();
    let client = OpenAIClient::builder()
        .with_api_key(api_key)
        .with_endpoint(model_url.clone())
        .build()
        .unwrap();
    let req = ChatCompletionRequest::new(model_id.clone(), conversation_history);
        // .max_tokens(max_tokens)
        // .temperature(temperature);
    assert!(max_tokens <= 512, "max_tokens must be at most 512");
    info!(
        "Sending request to {} using model {} and url {}",
        provider_name, model_id, model_url
    );
    let result = client.chat_completion(req).await;
    debug!(
        "Received response from {} using model {}",
        provider_name, model_id
    );

    // Signal the typing indicator task to stop
    let _ = tx.send(true);
    debug!("Signaled typing indicator to stop for chat ID: {}", chat_id);

    match result {
        Ok(response) => {
            let ai_response = response.choices.first().unwrap_or(&ChatCompletionChoice {
                index: 0,
                message: ChatCompletionMessageForResponse {
                    role: MessageRole::assistant,
                    content: None,
                    name: None,
                    tool_calls: None,
                },
                finish_reason: None,
                finish_details: None,
            });
            let response_str = if let Some(content) = &ai_response.message.content {
                content.to_string()
            } else {
                warn!("Empty AI response received.");
                "<no response>".to_string()
            };

            debug!("AI Response: {}", &response_str);
            let bot_msg = bot
                .send_message(msg.chat.id, &response_str)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Sent AI response to chat ID: {}", msg.chat.id);

            // Save the bot's response
            save_message(pool, &bot_msg, Some(model_info)).await?;
            info!("Saved bot's AI response.");
        }
        Err(e) => {
            let error_message = format!("Error: {}", e);
            warn!("Error generating AI response: {}", e);
            bot.send_message(msg.chat.id, &error_message)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Sent error message to chat ID: {}", msg.chat.id);
            // Don't save the error message
        }
    }

    Ok(())
}

async fn extract_prompt(msg: &Message) -> String {
    let mut prompt = String::new();

    // If the message is a reply, include the replied message's content and history
    if let Some(reply) = &msg.reply_to_message() {
        debug!("Message is a reply to message ID: {}", reply.id);
        // Append the current user's message without the command prefix
        if let Some(text) = &reply.text() {
            prompt.push_str(&remove_prefix(text));
            prompt.push_str("\n\n");
            debug!("Appended text from replied message.");
        } else if let Some(caption) = reply.caption() {
            prompt.push_str(&remove_prefix(caption));
            prompt.push_str("\n\n");
            debug!("Appended caption from replied message.");
        }
    } else {
        debug!("Message is not a reply.");
    }

    // Append the current user's message without the command prefix
    let content = get_message_content(msg);
    prompt.push_str(&remove_prefix(&content));

    prompt
}

fn remove_prefix(text: &str) -> String {
    // If the first word starts with a slash, remove it
    let mut words = text.split_whitespace();
    let first_word = words.next().unwrap_or_default();
    if first_word.starts_with('/') {
        let text = words.collect::<Vec<&str>>().join(" ");
        return text;
    }
    text.to_string()
}

async fn get_conversation_history(
    chat_id: teloxide::types::ChatId,
    reply_to_message_id: teloxide::types::MessageId,
    pool: &SqlitePool,
) -> Result<Vec<ChatCompletionMessage>, sqlx::Error> {
    debug!(
        "Fetching conversation history for chat ID: {}, reply message ID: {}",
        chat_id.0, reply_to_message_id.0
    );

    let mut history = Vec::new();
    let mut current_message_id = Some(reply_to_message_id.0 as i64);

    // Iterate through the conversation history
    while let Some(message_id) = current_message_id {
        let row = sqlx::query!(
            r#"
            SELECT user_id, content, reply_to FROM messages
            WHERE chat_id = ? AND message_id = ?
            "#,
            chat_id.0,
            message_id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(record) = row {
            history.push(ChatCompletionMessage {
                role: if record.user_id == 0 {
                    MessageRole::assistant
                } else {
                    MessageRole::user
                },
                content: Text(record.content),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
            current_message_id = record.reply_to;
        } else {
            break;
        }
    }

    history.reverse();
    Ok(history)
}

/// Top 10 users global/per chat
pub async fn top_usage_stats(
    pool: &SqlitePool,
    chat_id: Option<ChatId>,
) -> Result<String, sqlx::Error> {
    let mut stats = String::new();

    // Get top 10 users globally
    stats.push_str("Top 10 users globally:\n");
    let global_users = sqlx::query!(
        r#"
        SELECT user_id, COUNT(*) AS count FROM messages
        WHERE user_id > 0
        GROUP BY user_id
        ORDER BY count DESC
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await?;
    for (i, user) in global_users.iter().enumerate() {
        stats.push_str(&format!(
            "{}. User ID: {} - Messages: {}\n",
            i + 1,
            user.user_id,
            user.count
        ));
    }

    // Get top 10 users in the chat
    if let Some(chat_id) = chat_id {
        stats.push_str("\nTop 10 users in this chat:\n");
        let chat_users = sqlx::query!(
            r#"
            SELECT user_id, COUNT(*) AS count FROM messages
            WHERE chat_id = ? AND user_id > 0
            GROUP BY user_id
            ORDER BY count DESC
            LIMIT 10
            "#,
            chat_id.0
        )
        .fetch_all(pool)
        .await?;
        for (i, user) in chat_users.iter().enumerate() {
            stats.push_str(&format!(
                "{}. User ID: {} - Messages: {}\n",
                i + 1,
                user.user_id,
                user.count
            ));
        }
    }

    Ok(stats)
}

/// Top 10 commands global/per chat
pub async fn top_command_stats(
    pool: &SqlitePool,
    chat_id: Option<ChatId>,
) -> Result<String, sqlx::Error> {
    let mut stats = String::new();

    // Get top 10 commands globally
    stats.push_str("Top 10 commands globally:\n");
    let global_commands = sqlx::query!(
        r#"
        SELECT content, COUNT(*) AS count FROM messages
        WHERE user_id = 0
        GROUP BY content
        ORDER BY count DESC
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await?;
    for (i, command) in global_commands.iter().enumerate() {
        stats.push_str(&format!(
            "{}. Command: {} - Count: {}\n",
            i + 1,
            command.content,
            command.count
        ));
    }

    // Get top 10 commands in the chat
    if let Some(chat_id) = chat_id {
        stats.push_str("\nTop 10 commands in this chat:\n");
        let chat_commands = sqlx::query!(
            r#"
            SELECT content, COUNT(*) AS count FROM messages
            WHERE chat_id = ? AND user_id = 0
            GROUP BY content
            ORDER BY count DESC
            LIMIT 10
            "#,
            chat_id.0
        )
        .fetch_all(pool)
        .await?;
        for (i, command) in chat_commands.iter().enumerate() {
            stats.push_str(&format!(
                "{}. Command: {} - Count: {}\n",
                i + 1,
                command.content,
                command.count
            ));
        }
    }

    Ok(stats)
}

pub async fn save_message(
    pool: &SqlitePool,
    msg: &Message,
    model_info: Option<ModelInfo>,
) -> Result<(), sqlx::Error> {
    let user_id: i64;
    let model_id: Option<String>;
    let model_provider_name: Option<String>;
    if model_info.is_some() {
        model_id = Some(model_info.as_ref().unwrap().model_id.clone());
        model_provider_name = Some(model_info.as_ref().unwrap().model_provider.to_string());
        user_id = 0;
    } else {
        model_id = None;
        model_provider_name = None;
        user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    };

    let content = get_message_content(msg);
    let content = remove_prefix(&content);

    let reply_to = msg.reply_to_message().map(|reply| reply.id.0);
    let chat_id = msg.chat.id.0;
    let message_id = msg.id.0 as i64;
    let reply_to = reply_to as Option<i32>;

    // Use SQLx macros for compile-time checked queries
    sqlx::query!(
        r#"
        INSERT INTO messages (chat_id, user_id, message_id, reply_to, model, provider, content)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        chat_id,
        user_id,
        message_id,
        reply_to,
        model_id,
        model_provider_name,
        content
    )
    .execute(pool)
    .await?;

    debug!("Message saved successfully.");
    Ok(())
}

fn get_message_content(msg: &Message) -> String {
    if let Some(text) = msg.text() {
        // debug!("Extracted text from message ID: {}", msg.id);
        text.to_string()
    } else if let Some(caption) = msg.caption() {
        // debug!("Extracted caption from message ID: {}", msg.id);
        caption.to_string()
    } else {
        warn!("Unsupported message type for message ID: {}", msg.id);
        "Unsupported message type".to_string()
    }
}