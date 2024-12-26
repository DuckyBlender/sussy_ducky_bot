use log::{debug, error, info};
use strum::IntoEnumIterator;
use std::sync::Arc;
use std::time::Duration;
use teloxide::{prelude::*, types::ReplyParameters};
use teloxide::types::Message;
use teloxide::utils::command::BotCommands;

mod commands;
mod ollama;
mod ratelimit;
mod utils;
// mod image_utils;

use commands::Command;
use ratelimit::{RateLimitResult, RateLimiter};
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

    // Initialize rate limiter
    let mut rate_limiter = RateLimiter::new();
    
    // Add rate limits for each command
    for cmd in Command::iter() {
        if let Some(limit) = cmd.rate_limit() {
            rate_limiter.add_limit(&cmd.to_string(), limit);
        }
    }
    let rate_limiter = Arc::new(rate_limiter);

    // Clone necessary components for the handler
    let pool_clone = pool.clone();
    let me_clone = me.clone();

    // Build Dispatcher with UpdateHandler
    let handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {  
        let pool = pool_clone.clone();  
        let me = me_clone.clone();  
        let bot_clone = bot.clone();  
        let rate_limiter = rate_limiter.clone();  
  
        async move {  
            tokio::spawn(async move {  
                // Get user ID from the message  
                let user_id = match msg.from {  
                    Some(ref user) => user.id.0,  
                    None => {  
                        error!("Message without user ID received");  
                        return;  
                    }  
                };  
  
                // Check for command and rate limit  
                if let Some(text) = msg.text() {  
                    if let Ok(cmd) = Command::parse(text, me.username()) {  
                        // Check rate limit using user_id instead of chat_id  
                        match rate_limiter.check_rate_limit(user_id, &cmd.to_string()).await {  
                            RateLimitResult::Allowed => {  
                                if let Err(e) = handle_message(bot_clone, msg, &pool, &me).await {  
                                    error!("Error handling message: {}", e);  
                                }  
                            }  
                            RateLimitResult::Exceeded { seconds_remaining } => {  
                                let bot_msg = bot_clone  
                                    .send_message(  
                                        msg.chat.id,  
                                        format!(  
                                            "Rate limit exceeded. Please wait {} seconds before using this command again.",  
                                            seconds_remaining  
                                        ),  
                                    )  
                                    .reply_parameters(ReplyParameters::new(msg.id))  
                                    .await;  
  
                                if let Ok(sent_msg) = bot_msg {  
                                    // Clone values for the deletion task  
                                    let bot_for_deletion = bot_clone.clone();  
                                    let chat_id = msg.chat.id;  
                                    let message_id = sent_msg.id;  
                                      
                                    // Schedule message deletion  
                                    tokio::spawn(async move {  
                                        tokio::time::sleep(Duration::from_secs(seconds_remaining)).await;  
                                        if let Err(e) = bot_for_deletion.delete_message(chat_id, message_id).await {  
                                            error!("Failed to delete rate limit message: {}", e);  
                                        }  
                                    });  
                                }  
                            }  
                        }  
                        return;  
                    }  
                }  
  
                // Handle non-command messages  
                if let Err(e) = handle_message(bot_clone, msg, &pool, &me).await {  
                    error!("Error handling message: {}", e);  
                }  
            });  
            Ok::<(), std::convert::Infallible>(())  
        }  
    });

    info!(
        "Started @{} bot",
        me.username.clone().unwrap_or("unknown".to_string())
    );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![])
        .build()
        .dispatch()
        .await;
}
