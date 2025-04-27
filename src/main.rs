#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_lines)]

use apis::{ImageRequest, TogetherClient};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::Engine as _;
use chrono::Local;
use fern::colors::ColoredLevelConfig;
use lambda_http::{run, service_fn, Error};
use log::{debug, error, info, warn};
use std::env;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, InputFile, Message, ReplyParameters, UpdateKind};
use teloxide::utils::command::BotCommands;
mod apis;
mod utils;
use std::panic;
use utils::{
    find_prompt, parse_webhook, safe_send,
};

#[derive(BotCommands, Clone, Debug, PartialEq)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "welcome message")]
    Start,
    #[command(description = "flux[schnell] from together.ai")]
    Flux,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set a custom panic hook so it never returns non-200 to telegram
    panic::set_hook(Box::new(|panic_info| {
        error!("Application panicked: {panic_info:?}");
        std::process::exit(0);
    }));

    // Initialize tracing for logging
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
            ));
        })
        // Set default log level to Warn
        .level(log::LevelFilter::Warn)
        // Override log level for our module to Debug
        .level_for(env!("CARGO_PKG_NAME"), log::LevelFilter::Debug)
        // Output to stdout
        .chain(std::io::stdout())
        // Apply the configuration
        .apply()
        .expect("Failed to initialize logging");

    info!("Cold-starting the Lambda function");

    // Setup telegram bot (we do it here because this place is a cold start)
    let bot = Bot::new(env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set!"));
    info!("Telegram bot initialized");

    // Set commands
    let res = bot.set_my_commands(Command::bot_commands()).await;

    match res {
        Ok(_) => info!("Bot commands set successfully"),
        Err(e) => warn!("Failed to set commands: {e:?}"),
    }

    // Run the Lambda function
    run(service_fn(|req| handler(req, &bot))).await
}

async fn handler(
    req: lambda_http::Request,
    bot: &Bot,
) -> Result<lambda_http::Response<String>, lambda_http::Error> {
    debug!("Received a new request");

    // Parse JSON webhook
    let bot = bot.clone();

    let update = match parse_webhook(&req) {
        Ok(message) => {
            debug!("Successfully parsed webhook");
            message
        }
        Err(e) => {
            error!("Failed to parse webhook: {e:?}");
            return Ok(lambda_http::Response::builder()
                .status(200)
                .body("Failed to parse webhook".into())
                .unwrap());
        }
    };

    // Handle commands
    if let UpdateKind::Message(message) = &update.kind {
        if let Some(text) = message.text().or_else(|| message.caption()) {
            debug!("Received text or caption: {text}");
            if let Ok(command) = Command::parse(text, bot.get_me().await.unwrap().username()) {
                info!("Parsed command: {command:?}");
                return handle_command(bot.clone(), message, command).await;
            }
        }
    }

    debug!("No command found in the message");

    Ok(lambda_http::Response::builder()
        .status(200)
        .body(String::new())
        .unwrap())
}

async fn handle_command(
    bot: Bot,
    message: &Message,
    command: Command,
) -> Result<lambda_http::Response<String>, lambda_http::Error> {
    info!("Handling command: {command:?}");

    match command {
        Command::Help | Command::Start => {
            let help_text = Command::descriptions().to_string();
            safe_send(bot, message.chat.id, message.id, &help_text).await;
            Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap())
        }

        Command::Flux => {
            // Just the prompt, no image
            let (Some(msg_text), _) = find_prompt(message).await else {
                warn!("No prompt found in the message or reply message");
                safe_send(bot, message.chat.id, message.id, "Please provide a prompt.").await;

                return Ok(lambda_http::Response::builder()
                    .status(200)
                    .body(String::new())
                    .unwrap());
            };

            // Send typing indicator
            bot.send_chat_action(message.chat.id, ChatAction::Typing)
                .await
                .unwrap();

            // Send the request
            let client = TogetherClient::new();
            let request = ImageRequest {
                model: "black-forest-labs/FLUX.1-schnell-Free".to_string(),
                prompt: msg_text.clone(),
                width: 1024,
                height: 768,
                steps: 4,
                n: 1,
                response_format: "b64_json".to_string(),
            };

            let res = client.submit_request(request).await;
            if let Err(e) = res {
                error!("Failed to submit request: {e:?}");
                safe_send(bot, message.chat.id, message.id, &format!("error: {e:?}")).await;
                return Ok(lambda_http::Response::builder()
                    .status(200)
                    .body(String::new())
                    .unwrap());
            }

            let response = res.unwrap();
            // Get the base64 image
            let base64 = &response.data[0].b64_json;
            info!(
                "Received image response from Together.ai: {} bytes with prompt: {}",
                base64.len(),
                msg_text
            );
            // Put the base64 image in a memory buffer
            let base64 = BASE64.decode(base64).unwrap();

            // Send typing indicator
            bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::UploadPhoto)
                .await
                .unwrap();

            // Send the response
            let res = bot
                .send_photo(message.chat.id, InputFile::memory(base64))
                .caption(msg_text)
                .reply_parameters(ReplyParameters::new(message.id))
                .await;

            if let Err(e) = res {
                error!("Failed to send message: {e:?}");
            }

            Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap())
        }
    }
}
