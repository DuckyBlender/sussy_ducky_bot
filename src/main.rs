use log::{debug, error, info, warn};
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage, MessageRole},
    Ollama,
};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::env;
use teloxide::{
    prelude::*,
    types::{ChatAction, Me, Message, MessageId, ReplyParameters},
    utils::command::BotCommands,
};
use tokio::sync::watch;
use tokio::time::{self, Duration};

// Define the Displayable trait
trait Displayable {
    fn display(&self) -> String;
}

// Implement the Displayable trait for ChatMessage
impl Displayable for ChatMessage {
    fn display(&self) -> String {
        format!("{}: {}", self.role.display(), self.content)
    }
}

// Implement the Displayable trait for MessageRole
impl Displayable for MessageRole {
    fn display(&self) -> String {
        match self {
            MessageRole::User => "User".to_string(),
            MessageRole::Assistant => "Assistant".to_string(),
            MessageRole::System => "System".to_string(),
        }
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables from `.env` file
    dotenvy::dotenv().ok();
    debug!("Loaded environment variables from .env");

    // Initialize Logging
    init_logging();
    info!("Logging has been initialized.");

    log::info!("Starting Telegram bot...");

    // Initialize the bot with the token from environment variables
    let bot = Bot::from_env();
    info!("Bot instance created.");

    // Fetch bot information
    let me = match bot.get_me().await {
        Ok(user) => {
            info!("Successfully fetched bot information.");
            user
        }
        Err(e) => {
            error!("Failed to get bot info: {}", e);
            panic!("Failed to get bot info");
        }
    };
    info!("Started @{}", me.username());

    // Sync the bot's commands with Telegram
    if let Err(e) = bot.set_my_commands(Command::bot_commands()).await {
        error!("Failed to set commands: {}", e);
        panic!("Failed to set commands");
    }
    info!("Bot commands have been set.");

    // Initialize SQLite Pool
    let pool = match init_db().await {
        Ok(p) => {
            info!("Database pool initialized successfully.");
            p
        }
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            panic!("Failed to initialize database");
        }
    };

    // For development, remove all messages from the database
    sqlx::query!("DELETE FROM messages")
        .execute(&pool)
        .await
        .expect("Failed to delete messages from the database");

    // Initialize Ollama AI service
    let ollama = Ollama::default();
    info!("Successfully connected to Ollama");

    // Build Dispatcher with UpdateHandler
    // Todo: Improve this in the future
    let ollama_clone = ollama.clone();
    let pool_clone = pool.clone();
    let me_clone = me.clone();
    let handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
        let ollama = ollama_clone.clone();
        let pool = pool_clone.clone();
        let me = me_clone.clone();
        async move {
            handle_message(bot, msg, &ollama, &pool, &me)
                .await
                .map_err(|e| {
                    error!("Error handling message: {}", e);
                    e
                })
        }
    });

    Dispatcher::builder(bot, handler).build().dispatch().await;
}

/// Initialize logging using fern
fn init_logging() {
    fern::Dispatch::new()
        // Custom format: [Timestamp Level Target] Message
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        // Set default log level to Warn
        .level(log::LevelFilter::Warn)
        // Override log level for our module to Info
        .level_for("sussy_ducky_bot", log::LevelFilter::Debug)
        // Output to stdout
        .chain(std::io::stdout())
        // Output to a log file
        .chain(fern::log_file("output.log").unwrap())
        // Apply the configuration
        .apply()
        .expect("Failed to initialize logging");
}

/// Initialize the SQLite database and ensure the messages table exists
async fn init_db() -> Result<SqlitePool, sqlx::Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    debug!("Database URL: {}", database_url);

    // Initialize the connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    info!("Connected to the SQLite database.");

    Ok(pool)
}

/// Define bot commands using `BotCommands` derive
#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Display this help text")]
    Help,
    #[command(description = "Ask llama 3.2 1b", alias = "l")]
    Llama(String),
    #[command(description = "Ask qwen 2.5 1.5b", alias = "q")]
    Qwen(String),
}

// Handle all incoming messages
async fn handle_message(
    bot: Bot,
    msg: Message,
    ollama: &Ollama,
    pool: &SqlitePool,
    me: &Me,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // This is going to be a huge function, here is all it needs to do
    // - check if message is a reply to a bot ai response, if so reply with the same model and conversation history
    // - check if message is a command, if so save the message for future reply context and handle the command with handle_command()
    // - check if message is a caption, if so save the message for future reply context and handle the caption with handle_command()
    // - if none of the above, don't save the message and don't do anything

    // Check if the message is a reply to a known bot response
    if let Some(reply) = msg.reply_to_message() {
        if let Some(model) = get_model_from_msg(reply, pool).await {
            handle_ollama(bot, msg, ollama, pool, model).await?;
            return Ok(());
        }
    }

    // Check if the message is a command
    if let Some(text) = msg.text() {
        if let Ok(cmd) = Command::parse(text, me.username()) {
            handle_command(bot, msg, cmd, ollama, pool).await?;
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

    row.map(|r| r.model)?
}

/// Handle incoming commands
async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    ollama: &Ollama,
    pool: &SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Received command: {:?}", cmd);

    match cmd {
        Command::Help => {
            let help_text = Command::descriptions().to_string();
            debug!("Sending help text to user.");
            bot.send_message(msg.chat.id, &help_text)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Help text sent to chat ID: {}", msg.chat.id);
            info!("Saved bot's help response.");
        }
        Command::Llama(prompt) => {
            debug!("Handling /llama command with prompt: {}", prompt);
            handle_ollama(bot, msg, ollama, pool, "llama3.2:1b".to_string()).await?;
        }
        Command::Qwen(prompt) => {
            debug!("Handling /qwen command with prompt: {}", prompt);
            handle_ollama(bot, msg, ollama, pool, "qwen2.5:1.5b".to_string()).await?;
        }
    }

    Ok(())
}

/// Handle the /llama command
async fn handle_ollama(
    bot: Bot,
    msg: Message,
    ollama: &Ollama,
    pool: &SqlitePool,
    model: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Extract and format the prompt, including conversation history if replying
    // Check if the reply is a known message, else use extract_prompt()
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
    info!("Conversation history: {:?}", conversation_history);
    // for debug
    bot.send_message(msg.chat.id, conversation_history.clone().into_iter().map(|m| m.display()).collect::<Vec<String>>().join("\n").to_string())
        .await?;

    // Send initial typing action
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;
    debug!("Sent typing action to chat ID: {}", msg.chat.id);

    // Save the user's prompt
    save_message(pool, &msg, None).await?;
    info!("Saved user's llama prompt message.");

    // Create a watch channel to signal the typing indicator task when done
    let (tx, rx) = watch::channel(false);
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
            let _ = time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Generate the AI response
    let res = ollama
        .send_chat_messages(ChatMessageRequest::new(
            model.to_string(),
            conversation_history,
        ))
        .await;
    debug!("Received response from Ollama generator.");

    // Signal the typing indicator task to stop
    let _ = tx.send(true);
    debug!("Signaled typing indicator to stop for chat ID: {}", chat_id);

    match res {
        Ok(response) => {
            let ai_response = response.message.unwrap_or(ChatMessage {
                role: MessageRole::Assistant,
                content: "<no response>".to_string(), // todo: this should be handled better, without saving to db
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

/// Extract and format the prompt based on the message content and conversation history
async fn extract_prompt(msg: &Message) -> String {
    let mut prompt = String::new();

    // If the message is a reply, include the replied message's content and history
    if let Some(reply) = &msg.reply_to_message() {
        debug!("Message is a reply to message ID: {}", reply.id);
        // Todo: maybe we should fetch the conversation history here?

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

/// Fetch the conversation history for the given chat and reply message ID
async fn get_conversation_history(
    chat_id: ChatId,
    reply_to_message_id: MessageId,
    pool: &SqlitePool,
) -> Result<Vec<ChatMessage>, sqlx::Error> {
    debug!(
        "Fetching conversation history for chat ID: {}, reply message ID: {}",
        chat_id.0, reply_to_message_id.0
    );

    let mut history = Vec::new();
    let mut current_message_id = Some(reply_to_message_id.0 as i64);

    // I know this is probably slow but it's fine for now. An alternative would be 
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

/// Save a user or bot message to the database. If a model is None, it's a user message.
async fn save_message(
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

    debug!(
        "Saving message - Chat ID: {}, User ID: {}, Message ID: {}, Reply To: {:?}, Model: {:?}, Content: {}",
        chat_id,
        user_id,
        message_id,
        reply_to,
        model,
        content
    );

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

/// Extract the text content from a message, handling different message types
fn get_message_content(msg: &Message) -> String {
    if let Some(text) = msg.text() {
        debug!("Extracted text from message ID: {}", msg.id);
        text.to_string()
    } else if let Some(caption) = msg.caption() {
        debug!("Extracted caption from message ID: {}", msg.id);
        caption.to_string()
    } else {
        warn!("Unsupported message type for message ID: {}", msg.id);
        "Unsupported message type".to_string()
    }
}
