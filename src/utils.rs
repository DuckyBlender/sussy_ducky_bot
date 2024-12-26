use std::env;
use std::error::Error;

use chrono::Local;
use fern::colors::ColoredLevelConfig;
use log::{debug, error, info, warn};
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::Content::Text;
use openai_api_rs::v1::chat_completion::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionMessageForResponse,
    ChatCompletionRequest, FinishReason, MessageRole,
};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, MessageId, ParseMode, ReplyParameters};
use teloxide::utils::command::BotCommands;
use teloxide::utils::markdown;
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

    // If the message starts with //, ignore it
    if msg.text().unwrap_or_default().starts_with("//") {
        return Ok(());
    }

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

pub async fn handle_stats(bot: Bot, msg: Message, pool: &SqlitePool) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id = msg.chat.id.0;

    // Create a formatted stats message
    let mut response = String::from("📊 *Statistics*\n\n");
    
    // Add user stats
    response.push_str("👥 *Top Users*:\n");
    if msg.chat.is_private() {
        // Global stats for DMs
        let user_stats = sqlx::query!(
            r#"
            SELECT username, COUNT(*) as message_count
            FROM messages 
            WHERE user_id != 0 AND username IS NOT NULL
            GROUP BY username
            ORDER BY message_count DESC
            LIMIT 5
            "#
        )
        .fetch_all(pool)
        .await?;

        for (i, user) in user_stats.iter().enumerate() {
            let username = markdown::escape(user.username.as_deref().unwrap_or("Unknown"));
            response.push_str(&format!(
                "{}\\. {} \\- {} messages\n",
                i + 1,
                username,
                user.message_count
            ));
        }

        // Second query for top models
        let model_stats = sqlx::query!(
            r#"
            SELECT model, provider, COUNT(*) as usage_count
            FROM messages
            WHERE model IS NOT NULL AND provider IS NOT NULL
            GROUP BY model, provider
            ORDER BY usage_count DESC
            LIMIT 5
            "#
        )
        .fetch_all(pool)
        .await?;

        response.push_str("\n🤖 *Top Models*:\n");
        for (i, model) in model_stats.iter().enumerate() {
            let model_name = markdown::escape(model.model.as_deref().unwrap_or("Unknown"));
            let provider_name = markdown::escape(model.provider.as_deref().unwrap_or("Unknown"));
            response.push_str(&format!(
                "{}\\. {} \\({}\\) \\- {} uses\n",
                i + 1,
                model_name,
                provider_name,
                model.usage_count
            ));
        }

        // Add context about whether these are global or chat-specific stats
        response.push_str("\n_Showing global user statistics and model usage_");
    } else {
        // Chat-specific stats
        let user_stats = sqlx::query!(
            r#"
            SELECT username, COUNT(*) as message_count
            FROM messages 
            WHERE chat_id = ? AND user_id != 0 AND username IS NOT NULL
            GROUP BY username
            ORDER BY message_count DESC
            LIMIT 5
            "#,
            chat_id
        )
        .fetch_all(pool)
        .await?;

        for (i, user) in user_stats.iter().enumerate() {
            let username = markdown::escape(user.username.as_deref().unwrap_or("Unknown"));
            response.push_str(&format!(
                "{}\\. {} \\- {} messages\n",
                i + 1,
                username,
                user.message_count
            ));
        }

        // Second query for top models
        let model_stats = sqlx::query!(
            r#"
            SELECT model, provider, COUNT(*) as usage_count
            FROM messages
            WHERE chat_id = ? AND model IS NOT NULL AND provider IS NOT NULL
            GROUP BY model, provider
            ORDER BY usage_count DESC
            LIMIT 5
            "#,
            chat_id
        )

        .fetch_all(pool)
        .await?;

        response.push_str("\n🤖 *Top Models*:\n");
        for (i, model) in model_stats.iter().enumerate() {
            let model_name = markdown::escape(model.model.as_deref().unwrap_or("Unknown"));
            let provider_name = markdown::escape(model.provider.as_deref().unwrap_or("Unknown"));
            response.push_str(&format!(
                "{}\\. {} \\({}\\) \\- {} uses\n",
                i + 1,
                model_name,
                provider_name,
                model.usage_count
            ));
        }

        // Add context about whether these are global or chat-specific stats
        response.push_str("\n_Showing chat-specific statistics_");
    }

    
    // Send the formatted message
    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

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

pub async fn form_conversation_history(
    msg: &Message,
    model_info: &ModelInfo,
    pool: &SqlitePool,
) -> Result<Vec<ChatCompletionMessage>, Box<dyn Error + Send + Sync>> {
    let mut conversation_history: Vec<ChatCompletionMessage> = Vec::new();
    if let Some(reply) = msg.reply_to_message() {
        if get_model_from_msg(reply, pool).await.is_some() {
            // Known message, fetch conversation history
            conversation_history = get_conversation_history_db(msg.chat.id, reply.id, pool).await?;
            // add the user's prompt to the conversation history
            conversation_history.push(ChatCompletionMessage {
                role: MessageRole::user,
                content: Text(get_message_content(msg)),
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
                content: Text(extract_prompt(msg).await),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        } else {
            let system_prompt_info = model_info.system_prompt.as_ref().unwrap();
            let system_prompt = system_prompt_info.1.clone();
            match system_prompt_info.0 {
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
                        content: Text(extract_prompt(msg).await),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                SystemMethod::Inject => {
                    // Add user prompt as first message with prefix system prompt
                    let system_prompt = system_prompt_info.1.clone();
                    conversation_history.push(ChatCompletionMessage {
                        role: MessageRole::user,
                        content: Text(format!("{}{}", system_prompt, extract_prompt(msg).await)),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                SystemMethod::InjectInsert => {
                    // Add user prompt as first message, replace <INSERT> with the users prompt
                    let system_prompt = system_prompt_info.1.clone();
                    conversation_history.push(ChatCompletionMessage {
                        role: MessageRole::user,
                        content: Text(
                            system_prompt.replace("<INSERT>", &extract_prompt(msg).await),
                        ),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
            }
        }
    }
    Ok(conversation_history)
}

pub async fn handle_ai(
    bot: Bot,
    msg: Message,
    pool: &SqlitePool,
    model_info: ModelInfo,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Extract and format the prompt, including conversation history if replying
    let conversation_history = form_conversation_history(&msg, &model_info, pool).await?;

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

    // Save the user's prompt (from the conversation history)
    let user_prompt = &conversation_history.last().unwrap().content;
    let user_prompt = match user_prompt {
        Text(text) => text,
        _ => {
            warn!("Unsupported content type for user prompt.");
            return Ok(());
        }
    };
    save_message(pool, &msg, None, Some(user_prompt.to_string())).await?;
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
        .with_json(json!({
        "safetySettings": [
            {"category": "HARM_CATEGORY_UNSPECIFIED", "threshold": "BLOCK_NONE"},
            {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"},
            {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE"},
            {"category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE"},
            {"category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE"},
            {"category": "HARM_CATEGORY_CIVIC_INTEGRITY", "threshold": "BLOCK_NONE"}
        ],
        "provider": {
            "order": [
              "Google AI Studio"
            ],
            "allow_fallbacks": false
          }
        }))
        .build()
        .unwrap();
    let req = ChatCompletionRequest::new(model_id.clone(), conversation_history)
        .max_tokens(max_tokens)
        .temperature(temperature);
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
            let mut response_str = if let Some(content) = &ai_response.message.content {
                content.to_string()
            } else {
                warn!("Empty AI response received.");
                "<no response>".to_string()
            };

            let db_response = response_str.clone();

            // If finish reason is length, append [MAX LENGTH] to the response
            if ai_response.finish_reason == Some(FinishReason::length) {
                info!("AI response reached max length.");
                response_str.push_str(" [MAX LENGTH]");
            } else if ai_response.finish_reason == Some(FinishReason::stop) {
                info!("AI response stopped.");
                // Check if max 4096 chars
                if response_str.len() > 4096 {
                    response_str.truncate(4090);
                    response_str.push_str(" [...]");
                }
            }

            debug!("AI Response: {}", &response_str);
            let bot_msg = bot
                .send_message(msg.chat.id, &response_str)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;

            // Save the bot's response
            save_message(pool, &bot_msg, Some(model_info), Some(db_response)).await?;
            info!("Saved bot's AI response.");
        }
        Err(e) => {
            error!("Error generating AI response: {}", e);
            bot.send_message(
                msg.chat.id,
                "Error generating AI response, please try again later. @DuckyBlender",
            )
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
            info!("Sent error message to chat ID: {}", msg.chat.id);
            // Don't save the error message
        }
    }

    Ok(())
}

/// Returns the context of the message in the format:
/// User: Hello, how are you?
/// Bot: I'm fine, thank you.
pub async fn handle_context(
    bot: Bot,
    msg: Message,
    pool: &SqlitePool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if msg.reply_to_message().is_none() {
        bot.send_message(msg.chat.id, "Please reply to a message to get the context.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        info!("Sent no reply message to chat ID: {}", msg.chat.id);
        return Ok(());
    }
    let reply_msg = msg.reply_to_message().unwrap();
    let model_from_msg = get_model_from_msg(reply_msg, pool).await;
    if model_from_msg.is_none() {
        bot.send_message(msg.chat.id, "No context found for this message.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        info!("Sent no context message to chat ID: {}", msg.chat.id);
        return Ok(());
    }

    let history = get_conversation_history_db(msg.chat.id, reply_msg.id, pool).await?;

    let mut final_msg = String::new();
    for message in &history {
        let content = match &message.content {
            Text(text) => text,
            _ => "<unsupported content type>",
        };
        final_msg.push_str(&format!("{:?}: {}\n", message.role, content));
    }

    if final_msg.len() > 4096 {
        final_msg.truncate(4090);
        final_msg.push_str(" [...]");
    }

    bot.send_message(msg.chat.id, &final_msg)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    info!("Sent context message to chat ID: {}", msg.chat.id);

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

async fn get_conversation_history_db(
    chat_id: ChatId,
    reply_to_message_id: MessageId,
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

/// This function saves a message to the database. Should be refactored cause it's a mess
pub async fn save_message(
    pool: &SqlitePool,
    msg: &Message,
    model_info: Option<ModelInfo>,
    content_override: Option<String>,
) -> Result<(), sqlx::Error> {
    let user_id: i64;
    let model_id: Option<String>;
    let model_provider_name: Option<String>;
    let username_or_first_name: Option<String>;

    if model_info.is_some() {
        model_id = Some(model_info.as_ref().unwrap().model_id.clone());
        model_provider_name = Some(model_info.as_ref().unwrap().model_provider.to_string());
        user_id = 0;
        username_or_first_name = None;
    } else {
        model_id = None;
        model_provider_name = None;
        user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
        username_or_first_name = msg.from.as_ref().and_then(|u| u.username.clone())
            .or_else(|| msg.from.as_ref().map(|u| u.first_name.clone()));
    };

    let content = if let Some(override_content) = content_override {
        override_content
    } else {
        let content = get_message_content(msg);
        remove_prefix(&content)
    };

    let reply_to = msg.reply_to_message().map(|reply| reply.id.0);
    let chat_id = msg.chat.id.0;
    let message_id = msg.id.0 as i64;

    // Use SQLx macros for compile-time checked queries
    sqlx::query!(
        r#"
        INSERT INTO messages (chat_id, user_id, message_id, reply_to, model, provider, content, username)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        chat_id,
        user_id,
        message_id,
        reply_to,
        model_id,
        model_provider_name,
        content,
        username_or_first_name
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
