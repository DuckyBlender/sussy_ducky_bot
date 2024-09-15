use apis::openai_request;
use lambda_http::{run, service_fn, Error};

use reqwest::Client as ReqwestClient;
use std::env;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{Message, ReplyParameters, UpdateKind};
use teloxide::utils::command::BotCommands;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

mod apis;

mod utils;
use utils::*;

#[derive(BotCommands, Clone, Debug, PartialEq)]
#[command(
    rename_rule = "lowercase",
    description = "Models from GroqCloud & OpenRouter"
)]
enum BotCommand {
    #[command(description = "display this text")]
    Help,
    #[command(description = "welcome message")]
    Start,
    #[command(description = "caveman version of llama3.1")]
    Caveman,
    #[command(description = "llama3.1 70b", alias = "l")]
    Llama,
    #[command(description = "llava 7b vision model", aliases = ["v", "vision"])]
    Llava,
    #[command(description = "pixtral 12b vision model", aliases = ["p"])]
    Pixtral,
    #[command(description = "qwen2-vl 7b uncensored vision model", aliases = ["qwen2", "q"])]
    Qwen,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_env_filter(EnvFilter::new("sussy_ducky_bot=debug"))
        .init();

    info!("Starting the Telegram bot application");

    // Setup telegram bot (we do it here because this place is a cold start)
    let bot = Bot::new(env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set!"));
    info!("Telegram bot initialized");

    let client = ReqwestClient::new();
    info!("Groq API client initialized");

    // Set commands
    let res = bot.set_my_commands(BotCommand::bot_commands()).await;

    match res {
        Ok(_) => info!("Bot commands set successfully"),
        Err(e) => warn!("Failed to set commands: {:?}", e),
    }

    // Run the Lambda function
    info!("Starting Lambda function");
    run(service_fn(|req| handler(req, &bot, &client))).await
}

async fn handler(
    req: lambda_http::Request,
    bot: &Bot,
    client: &ReqwestClient,
) -> Result<lambda_http::Response<String>, lambda_http::Error> {
    debug!("Received a new request");

    // Parse JSON webhook
    let bot = bot.clone();

    let update = match parse_webhook(req).await {
        Ok(message) => {
            debug!("Successfully parsed webhook");
            message
        }
        Err(e) => {
            error!("Failed to parse webhook: {:?}", e);
            return Ok(lambda_http::Response::builder()
                .status(400)
                .body("Failed to parse webhook".into())
                .unwrap());
        }
    };

    // Handle commands
    if let UpdateKind::Message(message) = &update.kind {
        if let Some(text) = &message.text() {
            debug!("Received message: {}", text);
            if let Ok(command) = BotCommand::parse(text, bot.get_me().await.unwrap().username()) {
                info!("Parsed command: {:?}", command);
                return handle_command(bot.clone(), message, command, client).await;
            }
        }
    }

    debug!("No command found in the message");

    // Secret bawialnia easter egg
    if let UpdateKind::Message(message) = &update.kind {
        if message.text().is_some()
            && (message.chat.id == ChatId(-1001865084475)
                || message.chat.id == ChatId(-1001641972650))
        {
            let random: f64 = rand::random();
            debug!("Random number: {}", random);
            if random < 0.01 {
                // 1% chance of triggering
                return handle_command(bot.clone(), message, BotCommand::Caveman, client).await;
            }
        }
    }

    Ok(lambda_http::Response::builder()
        .status(200)
        .body(String::new())
        .unwrap())
}

async fn handle_command(
    bot: Bot,
    message: &Message,
    command: BotCommand,
    client: &ReqwestClient,
) -> Result<lambda_http::Response<String>, lambda_http::Error> {
    info!("Handling command: {:?}", command);

    if command == BotCommand::Help || command == BotCommand::Start {
        info!("Sending help or start message");
        bot.send_message(message.chat.id, BotCommand::descriptions().to_string())
            .await
            .unwrap();
        return Ok(lambda_http::Response::builder()
            .status(200)
            .body(String::new())
            .unwrap());
    }

    let msg_text = match find_prompt(message).await {
        Some(prompt) => prompt,
        None => {
            warn!("No prompt found in the message or reply message");
            bot.send_message(
                message.chat.id,
                "Please provide a prompt. It can be in the message or a reply to a message.",
            )
            .reply_parameters(ReplyParameters::new(message.id))
            .await
            .unwrap();
            return Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap());
        }
    };

    // Get the image file, if any
    let img = get_image_from_message(message);

    if img.is_some() {
        debug!("Image file found in the message");
    } else {
        debug!("No image found in the message");
    }

    let base64_img = match img {
        Some(photo) => Some(download_and_encode_image(&bot, &photo).await.unwrap()),
        None => None,
    };

    // Send the request
    let res = openai_request(client, &msg_text, base64_img.as_deref(), command).await;

    // Catch error
    if let Err(e) = res {
        bot.send_message(message.chat.id, format!("error: {:?}", e))
            .reply_parameters(ReplyParameters::new(message.id))
            .await
            .unwrap();

        return Ok(lambda_http::Response::builder()
            .status(200)
            .body(String::new())
            .unwrap());
    }

    let response_text = res.unwrap();

    // Check if empty response
    if response_text.is_empty() {
        warn!("Empty response from API");
        bot.send_message(message.chat.id, "<no text>")
            .reply_parameters(ReplyParameters::new(message.id))
            .await
            .unwrap();
        return Ok(lambda_http::Response::builder()
            .status(200)
            .body(String::new())
            .unwrap());
    }

    // Send the response
    info!("Sending response: {}", response_text);
    bot.send_message(message.chat.id, response_text)
        .reply_parameters(ReplyParameters::new(message.id))
        .await
        .unwrap();

    Ok(lambda_http::Response::builder()
        .status(200)
        .body(String::new())
        .unwrap())
}
