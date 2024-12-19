use log::{debug, error, info};
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use sqlx::Row;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::env;
use teloxide::{
    dispatching::UpdateHandler,
    prelude::*,
    types::{ChatAction, Message, MessageId, ReplyParameters},
    utils::command::BotCommands,
};
use tokio::sync::watch;
use tokio::time::{self, Duration};

#[tokio::main]
async fn main() {
    // Load environment variables from `.env` file
    dotenvy::dotenv().ok();

    // Initialize Logging
    init_logging();

    log::info!("Starting Telegram bot...");

    // Initialize the bot with the token from environment variables
    let bot = Bot::from_env();

    // Fetch bot information
    let me = bot.get_me().await.expect("Failed to get bot info");
    info!("Started @{}", me.username());

    // Sync the bot's commands with Telegram
    bot.set_my_commands(Command::bot_commands())
        .await
        .expect("Failed to set commands");

    // Initialize SQLite Pool
    let pool = init_db().await;

    // Initialize Ollama AI service
    let ollama = Ollama::default();

    // Build Dispatcher with separate handlers for commands and captions
    Dispatcher::builder(bot, make_handler(ollama.clone(), pool.clone()))
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
async fn init_db() -> SqlitePool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    

    // Create the messages table if it doesn't exist
    // sqlx::query(
    //     r#"
    //     CREATE TABLE IF NOT EXISTS messages (
    //         id INTEGER PRIMARY KEY AUTOINCREMENT,
    //         chat_id INTEGER NOT NULL,
    //         user_id INTEGER NOT NULL,
    //         message_id INTEGER NOT NULL,
    //         reply_to INTEGER,
    //         sender TEXT NOT NULL CHECK(sender IN ('user', 'bot')),
    //         content TEXT NOT NULL,
    //         timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
    //     );
    //     "#,
    // )
    // .execute(&pool)
    // .await
    // .expect("Failed to create messages table");

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to the database")
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
) -> UpdateHandler<Box<dyn std::error::Error + Send + Sync>> {
    // Handler for messages that are commands
    let ollama_clone = ollama.clone();
    let pool_clone = pool.clone();
    let commands_handler =
        dptree::entry()
            .filter_command::<Command>()
            .endpoint(move |bot, msg, cmd| {
                let ollama = ollama_clone.clone();
                let pool = pool_clone.clone();
                async move { answer(bot, msg, cmd, &ollama, &pool).await }
            });

    // Handler for messages with captions (e.g., media with captions)
    let captions_handler = dptree::entry()
        .filter(|msg: Message| msg.caption().is_some())
        .endpoint({
            {
                let ollama = ollama.clone();
                let pool = pool.clone();
                move |bot, msg| {
                    let ollama = ollama.clone();
                    let pool = pool.clone();
                    async move { handle_caption(bot, msg, &ollama, &pool).await }
                }
            }
        });

    // Combine all handlers
    commands_handler.branch(captions_handler)
}

/// Handle incoming commands
async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    ollama: &Ollama,
    pool: &SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Received command: {:?}", cmd);

    // Save the user's command message
    save_message(pool, &msg, "user").await?;

    match cmd {
        Command::Help => {
            let help_text = Command::descriptions().to_string();
            bot.send_message(msg.chat.id, &help_text)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            // Save the bot's response
            save_bot_message(pool, &msg, &help_text).await?;
        }
        Command::Llama(prompt) => {
            handle_llama_command(bot, msg, prompt, ollama, pool).await?;
        }
    }

    Ok(())
}

/// Handle the /llama command
async fn handle_llama_command(
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

    // Save the user's prompt
    save_message(pool, &msg, "user").await?;

    // Create a watch channel to signal the typing indicator task when done
    let (tx, rx) = watch::channel(false);

    // Clone bot and chat id for the typing indicator task
    let bot_clone = bot.clone();
    let chat_id = msg.chat.id;

    // Spawn the typing indicator background task
    tokio::spawn(async move {
        loop {
            // Check if we should stop
            if *rx.borrow() {
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
            // Wait for 5 seconds or until notified
            let _ = time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Generate the AI response
    const MODEL: &str = "llama3.2:1b";
    let res = ollama
        .generate(GenerationRequest::new(MODEL.to_string(), formatted_prompt))
        .await;

    // Signal the typing indicator task to stop
    let _ = tx.send(true);

    match res {
        Ok(response) => {
            let ai_response = response.response;
            bot.send_message(msg.chat.id, &ai_response)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            // Save the bot's response
            save_bot_message(pool, &msg, &ai_response).await?;
        }
        Err(e) => {
            let error_message = format!("Error: {}", e);
            bot.send_message(msg.chat.id, &error_message)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            // Save the error message as bot's response
            save_bot_message(pool, &msg, &error_message).await?;
        }
    }

    Ok(())
}

/// Handle messages that contain captions (e.g., photos with captions)
async fn handle_caption(
    bot: Bot,
    msg: Message,
    ollama: &Ollama,
    pool: &SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Extract caption
    if let Some(caption) = msg.caption() {
        // Check if the caption contains a command
        if let Ok(cmd) = Command::parse(caption, &bot.get_me().await?.user.username.unwrap()) {
            // Save the user's caption message
            save_message(pool, &msg, "user").await?;

            // Handle the command
            answer(bot, msg, cmd, ollama, pool).await?;
        }
    }
    Ok(())
}

/// Extract and format the prompt based on the message content and conversation history
async fn extract_prompt(msg: &Message, cmd_prompt: Option<String>, pool: &SqlitePool) -> String {
    let mut prompt = String::new();

    // If the message is a reply, include the replied message's content and history
    if let Some(reply) = &msg.reply_to_message() {
        // Fetch the conversation history
        let history = get_conversation_history(msg.chat.id, reply.id, pool)
            .await
            .unwrap_or_default();
        prompt.push_str(&history);
        prompt.push('\n');

        // Append the current user's message without the command prefix
        if let Some(text) = &reply.text() {
            let text = text.trim_start_matches('/');
            let text = text.split_once(' ').map(|x| x.1).unwrap_or(text);
            prompt.push_str(text);
            prompt.push('\n');
        } else if let Some(caption) = reply.caption() {
            let caption = caption.trim_start_matches('/');
            let caption = caption.split_once(' ').map(|x| x.1).unwrap_or(caption);
            prompt.push_str(caption);
            prompt.push('\n');
        }
    }

    // Append the current prompt (from command or caption)
    if let Some(cp) = cmd_prompt {
        prompt.push_str(&cp);
    }

    prompt
}

/// Fetch the conversation history for the given chat and reply message ID
async fn get_conversation_history(
    chat_id: ChatId,
    reply_to_message_id: MessageId,
    pool: &SqlitePool,
) -> Result<String, sqlx::Error> {
    // Fetch all messages in the chat up to the replied message
    let rows = sqlx::query(
        r#"
        SELECT sender, content FROM messages
        WHERE chat_id = ? AND id <= (
            SELECT id FROM messages WHERE chat_id = ? AND message_id = ?
        )
        ORDER BY id ASC
        "#,
    )
    .bind(chat_id.0)
    .bind(chat_id.0)
    .bind(reply_to_message_id.0)
    .fetch_all(pool)
    .await?;

    // Concatenate messages to form the conversation history
    let mut history = String::new();
    for row in rows {
        let sender: String = row.try_get("sender")?;
        let content: String = row.try_get("content")?;
        history.push_str(&format!(
            "{}: {}\n",
            &sender,
            content
        ));
    }

    Ok(history)
}

/// Save a user or bot message to the database
async fn save_message(pool: &SqlitePool, msg: &Message, sender: &str) -> Result<(), sqlx::Error> {
    let reply_to = msg.reply_to_message().map(|reply| reply.id.0);

    sqlx::query(
        r#"
        INSERT INTO messages (chat_id, user_id, message_id, reply_to, sender, content)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(msg.chat.id.0)
    .bind(msg.from.clone().map(|u| u.id.0).unwrap_or(0) as i64)
    .bind(msg.id.0)
    .bind(reply_to)
    .bind(sender)
    .bind(get_message_content(msg))
    .execute(pool)
    .await?;

    Ok(())
}

/// Save a bot message specifically
async fn save_bot_message(
    pool: &SqlitePool,
    msg: &Message,
    content: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO messages (chat_id, user_id, message_id, reply_to, sender, content)
        VALUES (?, ?, ?, ?, 'bot', ?)
        "#,
    )
    .bind(msg.chat.id.0)
    .bind(0) // Assuming bot's user_id is not tracked
    .bind(msg.id.0)
    .bind(msg.reply_to_message().map(|m| m.id.0))
    .bind(content)
    .execute(pool)
    .await?;

    Ok(())
}

/// Extract the text content from a message, handling different message types
fn get_message_content(msg: &Message) -> String {
    if let Some(text) = &msg.text() {
        text.to_string()
    } else if let Some(caption) = &msg.caption() {
        caption.to_string()
    } else {
        "Unsupported message type".to_string()
    }
}
