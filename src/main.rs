use log::{debug, error, info, warn};
use ollama_rs::{models::LocalModel, Ollama};
use teloxide::prelude::*;
use teloxide::types::Message;
use teloxide::utils::command::BotCommands;
mod commands;
mod utils;

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

    // For development, remove all messages from the database
    sqlx::query!("DELETE FROM messages")
        .execute(&pool)
        .await
        .expect("Failed to delete messages from the database");

    // Initialize Ollama AI service
    let ollama = Ollama::default();
    info!("Successfully connected to Ollama");
    // List models to ensure they are available
    let local_models: Vec<LocalModel> = ollama.list_local_models().await.unwrap();
    let local_models: Vec<String> = local_models.iter().map(|m| m.name.clone()).collect();
    let bot_models = Command::available_models();
    info!("Local models: {:?}", local_models);
    info!("Bot models: {:?}", bot_models);
    // Check if all models are available
    for model in &bot_models {
        if local_models.contains(model) {
            info!("Model {} is available", model);
        } else {
            warn!("Model {} is not available, pulling", model);
            let res = ollama.pull_model(model.to_string(), false).await;
            if let Err(e) = res {
                error!("Failed to pull model {}: {}", model, e);
            }
            info!("Model {} has been pulled", model);
        }
    }

    // Clone necessary components for the handler
    let ollama_clone = ollama.clone();
    let pool_clone = pool.clone();
    let me_clone = me.clone();

    // Build Dispatcher with UpdateHandler
    let handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
        let ollama = ollama_clone.clone();
        let pool = pool_clone.clone();
        let me = me_clone.clone();
        let bot_clone = bot.clone();

        async move {
            tokio::spawn(async move {
                if let Err(e) = handle_message(bot_clone, msg, &ollama, &pool, &me).await {
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
