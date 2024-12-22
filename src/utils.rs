use std::env;
use std::error::Error;

use sqlx::sqlite::SqlitePoolOptions;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, ReplyParameters};
use teloxide::utils::command::BotCommands;
use tokio::time::{self, Duration};
use sqlx::SqlitePool;
use log::{debug, error, info, warn};
use chrono::Local;

use crate::commands::{handle_command, Command};
use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage, MessageRole},
        options::GenerationOptions,
    },
    Ollama,
};

pub const SYSTEM_PROMPT: &str = "Be precise and concise. Don't use markdown.";

pub fn init_logging() {
    fern::Dispatch::new()
        // Custom format: [Timestamp Level Target] Message
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
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

    Ok(pool)
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    ollama: &Ollama,
    pool: &SqlitePool,
    me: &teloxide::types::Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let user_id = msg.from.as_ref().map(|u| u.id);
    if user_id.is_none() {
        warn!("User ID not found in message, ignoring.");
        return Ok(());
    }
    let user_id = user_id.unwrap(); // for future ratelimit

    // Check if the message is a command
    if let Some(text) = msg.text() {
        if let Ok(cmd) = Command::parse(text, me.username()) {
            handle_command(bot, msg, cmd, ollama, pool).await?;
            return Ok(());
        }
    }

    // Check if the message is a reply to a known bot response
    if let Some(reply) = msg.reply_to_message() {
        if let Some(model) = get_model_from_msg(reply, pool).await {
            handle_ollama(bot, msg, ollama, pool, model).await?;
            return Ok(());
        }
    }

    // Check if the message is a caption
    if let Some(caption) = msg.caption() {
        if let Ok(cmd) = Command::parse(caption, me.username()) {
            handle_command(bot, msg, cmd, ollama, pool).await?;
            return Ok(());
        }
    }

    // If none of the above, do nothing
    Ok(())
}

async fn get_model_from_msg(msg: &Message, pool: &SqlitePool) -> Option<String> {
    let message_id = msg.id.0 as i64;
    let chat_id = msg.chat.id.0;

    let row = sqlx::query!(
        r#"
        SELECT model FROM messages
        WHERE chat_id = ? AND message_id = ? AND user_id = 0
        "#,
        chat_id,
        message_id
    )
    .fetch_optional(pool)
    .await
    .ok()?;

    row.and_then(|r| r.model)
}

pub async fn handle_ollama(
    bot: Bot,
    msg: Message,
    ollama: &Ollama,
    pool: &SqlitePool,
    model: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Extract and format the prompt, including conversation history if replying
    let mut conversation_history: Vec<ChatMessage> = Vec::new();
    if let Some(reply) = msg.reply_to_message() {
        if get_model_from_msg(reply, pool).await.is_some() {
            // Known message, fetch conversation history
            conversation_history = get_conversation_history(msg.chat.id, reply.id, pool).await?;
            // add the user's prompt to the conversation history
            conversation_history.push(ChatMessage {
                role: MessageRole::User,
                content: get_message_content(&msg),
                images: None,
            });
        }
    }
    if conversation_history.is_empty() {
        debug!("No conversation history found, extracting prompt from message.");
        conversation_history.push(ChatMessage {
            role: MessageRole::User,
            content: extract_prompt(&msg).await,
            images: None,
        });
    }

    // Check if there is a system prompt in any message
    if !conversation_history.iter().any(|m| m.role == MessageRole::System) {
        // Add system prompt as first message
        conversation_history.insert(
            0,
            ChatMessage {
                role: MessageRole::System,
                content: SYSTEM_PROMPT.to_string(),
                images: None,
            },
        );
    }

    info!("Conversation history: {:?}", conversation_history);

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
    let options = GenerationOptions::default().temperature(0.3);
    let res = ollama
        .send_chat_messages(
            ChatMessageRequest::new(model.to_string(), conversation_history).options(options),
        )
        .await;
    debug!("Received response from Ollama generator.");

    // Signal the typing indicator task to stop
    let _ = tx.send(true);
    debug!("Signaled typing indicator to stop for chat ID: {}", chat_id);

    match res {
        Ok(response) => {
            let ai_response = response.message.unwrap_or(ChatMessage {
                role: MessageRole::Assistant,
                content: "<no response>".to_string(), // TODO: Handle this case better
                images: None,
            });
            let response_str = if ai_response.content.is_empty() {
                warn!("Empty AI response received.");
                "<no response>".to_string()
            } else {
                ai_response.content.clone()
            };

            debug!("AI Response: {}", &response_str);
            let bot_msg = bot
                .send_message(msg.chat.id, &response_str)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Sent AI response to chat ID: {}", msg.chat.id);

            // Save the bot's response
            save_message(pool, &bot_msg, Some(&model)).await?;
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
) -> Result<Vec<ChatMessage>, sqlx::Error> {
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
            history.push(ChatMessage {
                role: if record.user_id == 0 {
                    MessageRole::Assistant
                } else {
                    MessageRole::User
                },
                content: record.content,
                images: None,
            });
            current_message_id = record.reply_to;
        } else {
            break;
        }
    }

    history.reverse();
    Ok(history)
}

pub async fn save_message(
    pool: &SqlitePool,
    msg: &Message,
    model: Option<&str>,
) -> Result<(), sqlx::Error> {
    let user_id = if model.is_some() {
        0
    } else {
        msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64
    };

    let content = get_message_content(msg);
    let content = remove_prefix(&content);

    let reply_to = msg.reply_to_message().map(|reply| reply.id.0);
    let chat_id = msg.chat.id.0;
    let message_id = msg.id.0 as i64;
    let reply_to = reply_to as Option<i32>;

    // debug!(
    //     "Saving message - Chat ID: {}, User ID: {}, Message ID: {}, Reply To: {:?}, Model: {:?}, Content: {}",
    //     chat_id,
    //     user_id,
    //     message_id,
    //     reply_to,
    //     model,
    //     content
    // );

    // Use SQLx macros for compile-time checked queries
    sqlx::query!(
        r#"
        INSERT INTO messages (chat_id, user_id, message_id, reply_to, model, content)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
        chat_id,
        user_id,
        message_id,
        reply_to,
        model,
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