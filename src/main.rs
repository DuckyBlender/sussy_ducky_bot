use log::{debug, error, info, warn};
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::env;
use teloxide::{
    dispatching::UpdateHandler,
    prelude::*,
    types::{ChatAction, Me, Message, MessageId, ReplyParameters},
    utils::command::BotCommands,
};
use tokio::sync::watch;
use tokio::time::{self, Duration};

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

    // Initialize Ollama AI service
    let ollama = Ollama::default();
    info!("Ollama AI service initialized.");

    // Build Dispatcher with separate handlers for commands and captions
    Dispatcher::builder(bot, make_handler(ollama.clone(), pool.clone(), me.clone()))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
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
        .level_for("sussy_ducky_bot", log::LevelFilter::Info)
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
}

/// Create the dispatcher handler with organized branches
fn make_handler(
    ollama: Ollama,
    pool: SqlitePool,
    me: Me,
) -> UpdateHandler<Box<dyn std::error::Error + Send + Sync>> {
    // Clone for commands handler
    let ollama_clone = ollama.clone();
    let pool_clone = pool.clone();
    
    let handler =
        dptree::entry()
            .endpoint(move |bot, msg| {
                let ollama = ollama_clone.clone();
                let pool = pool_clone.clone();
                let me = me.clone();
                async move { handle_message(bot, msg, &ollama, &pool, &me).await }
            });

    // Return the handler
    handler

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
        if let Some(model) = get_model_from_reply(reply, pool).await {
            // todo: `model` will be useful for more models
            let prompt = extract_prompt(&msg, None, pool).await;
            handle_ollama(bot, msg, prompt, ollama, pool).await?;
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
        let prompt = extract_prompt(&msg, Some(caption.to_string()), pool).await;
        handle_ollama(bot, msg, prompt, ollama, pool).await?;
        return Ok(());
    }

    // If none of the above, do nothing
    Ok(())
}

async fn get_model_from_reply(
    msg: &Message,
    pool: &SqlitePool,
) -> Option<String> {
    let message_id = msg.id.0 as i64;
    let chat_id = msg.chat.id.0;

    let row = sqlx::query!(
        r#"
        SELECT model FROM messages
        WHERE chat_id = ? AND message_id = ? AND sender = 'bot'
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

    // Save the user's command message
    save_message(pool, &msg, "user", None, None).await?;
    info!("Saved user message with ID: {}", msg.id);

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
            handle_ollama(bot, msg, prompt, ollama, pool).await?;
        }
    }

    Ok(())
}

/// Handle the /llama command
async fn handle_ollama(
    bot: Bot,
    msg: Message,
    prompt: String,
    ollama: &Ollama,
    pool: &SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Extract and format the prompt, including conversation history if replying
    let formatted_prompt = extract_prompt(&msg, Some(prompt.clone()), pool).await;
    info!("Formatted Prompt: {}", formatted_prompt);

    // Send initial typing action
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;
    debug!("Sent typing action to chat ID: {}", msg.chat.id);

    // Save the user's prompt
    save_message(pool, &msg, "user", None, None).await?;
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
    const MODEL: &str = "llama3.2:1b";
    let res = ollama
        .generate(GenerationRequest::new(MODEL.to_string(), formatted_prompt))
        .await;
    debug!("Received response from Ollama generator.");

    // Signal the typing indicator task to stop
    let _ = tx.send(true);
    debug!("Signaled typing indicator to stop for chat ID: {}", chat_id);

    match res {
        Ok(response) => {
            let ai_response = response.response;
            debug!("AI Response: {}", ai_response);
            bot.send_message(msg.chat.id, &ai_response)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Sent AI response to chat ID: {}", msg.chat.id);

            // Save the bot's response
            save_message(pool, &msg, "bot", Some(&ai_response), Some(MODEL)).await?;
            info!("Saved bot's AI response.");
        }
        Err(e) => {
            let error_message = format!("Error: {}", e);
            warn!("Error generating AI response: {}", e);
            bot.send_message(msg.chat.id, &error_message)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Sent error message to chat ID: {}", msg.chat.id);

            // Save the error message as bot's response
            // save_message(pool, &msg, "bot", Some(&error_message), None).await?;
            // info!("Saved bot's error response.");
        }
    }

    Ok(())
}

/// Extract and format the prompt based on the message content and conversation history
async fn extract_prompt(msg: &Message, cmd_prompt: Option<String>, pool: &SqlitePool) -> String {
    let mut prompt = String::new();

    // If the message is a reply, include the replied message's content and history
    if let Some(reply) = &msg.reply_to_message() {
        debug!("Message is a reply to message ID: {}", reply.id);
        // Fetch the conversation history
        match get_conversation_history(msg.chat.id, reply.id, pool).await {
            Ok(history) => {
                prompt.push_str(&history);
                prompt.push('\n');
                info!("Fetched conversation history.");
            }
            Err(e) => {
                warn!("Failed to fetch conversation history: {}", e);
            }
        }

        // Append the current user's message without the command prefix
        if let Some(text) = &reply.text() {
            let text = text.trim_start_matches('/');
            let text = text.split_once(' ').map(|x| x.1).unwrap_or(text);
            prompt.push_str(text);
            prompt.push('\n');
            debug!("Appended text from replied message.");
        } else if let Some(caption) = reply.caption() {
            let caption = caption.trim_start_matches('/');
            let caption = caption.split_once(' ').map(|x| x.1).unwrap_or(caption);
            prompt.push_str(caption);
            prompt.push('\n');
            debug!("Appended caption from replied message.");
        }
    } else {
        debug!("Message is not a reply.");
    }

    // Append the current prompt (from command or caption)
    if let Some(cp) = cmd_prompt {
        prompt.push_str(&cp);
        debug!("Appended current command prompt.");
    }

    prompt
}

/// Fetch the conversation history for the given chat and reply message ID
async fn get_conversation_history(
    chat_id: ChatId,
    reply_to_message_id: MessageId,
    pool: &SqlitePool,
) -> Result<String, sqlx::Error> {
    debug!(
        "Fetching conversation history for chat ID: {}, reply message ID: {}",
        chat_id.0, reply_to_message_id.0
    );
    // Fetch all messages in the chat up to the replied message
    let rows = sqlx::query!(
        r#"
        SELECT sender, content FROM messages
        WHERE chat_id = ? AND id <= (
            SELECT id FROM messages WHERE chat_id = ? AND message_id = ?
        )
        ORDER BY id ASC
        "#,
        chat_id.0,
        chat_id.0,
        reply_to_message_id.0
    )
    .fetch_all(pool)
    .await?;

    // Concatenate messages to form the conversation history
    let mut history = String::new();
    for row in rows {
        let sender = row.sender;
        let content = row.content;
        history.push_str(&format!("{}: {}\n", sender, content));
        debug!("Appended to history: {}: {}", sender, content);
    }

    Ok(history)
}

/// Save a user or bot message to the database
///
/// - If `content_override` is `Some`, it uses the provided content (for bot messages).
/// - If `content_override` is `None`, it extracts the content from the `Message`.
async fn save_message(
    pool: &SqlitePool,
    msg: &Message,
    sender: &str,
    content_override: Option<&str>,
    model: Option<&str>,
) -> Result<(), sqlx::Error> {
    let user_id = if sender == "bot" {
        0
    } else {
        msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64
    };
    let content = match content_override {
        Some(content) => content.to_string(),
        None => get_message_content(msg),
    };
    let reply_to = msg.reply_to_message().map(|reply| reply.id.0);
    let chat_id = msg.chat.id.0;
    let message_id = msg.id.0 as i64;
    let reply_to = reply_to as Option<i32>;

    debug!(
        "Saving message - Chat ID: {}, User ID: {}, Message ID: {}, Reply To: {:?}, Sender: {}, Model: {:?}, Content: {}",
        chat_id,
        user_id,
        message_id,
        reply_to,
        sender,
        model,
        content
    );

    // Use SQLx macros for compile-time checked queries
    sqlx::query!(
        r#"
        INSERT INTO messages (chat_id, user_id, message_id, reply_to, sender, model, content)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        chat_id,
        user_id,
        message_id,
        reply_to,
        sender,
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