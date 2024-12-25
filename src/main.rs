use log::{debug, error, info};
use teloxide::prelude::*;
use teloxide::types::Message;
use teloxide::utils::command::BotCommands;
mod commands;
mod utils;
mod ollama;
mod markdown;

use commands::Command;
use utils::{handle_message, init_db, init_logging};

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

    // Verify ollama models
    if let Err(e) = ollama::verify_ollama_models().await {
        error!("Failed to verify ollama models: {}", e);
        panic!("Failed to verify ollama models");
    }

    // Clone necessary components for the handler
    let pool_clone = pool.clone();
    let me_clone = me.clone();

    // Build Dispatcher with UpdateHandler
    let handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
        let pool = pool_clone.clone();
        let me = me_clone.clone();
        let bot_clone = bot.clone();

        async move {
            tokio::spawn(async move {
                if let Err(e) = handle_message(bot_clone, msg, &pool, &me).await {
                    error!("Error handling message: {}", e);
                }
            });
            Ok::<(), std::convert::Infallible>(())
        }
    });

    info!("Started @{} bot", me.username.clone().unwrap_or("unknown".to_string()));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![])
        .build()
        .dispatch()
        .await;
}
