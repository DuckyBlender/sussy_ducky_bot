#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_lines)]

use apis::{ImageRequest, OpenAIClient, TogetherClient};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::Engine as _;
use lambda_http::{run, service_fn, Error};

use std::env;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, InputFile, Message, ReplyParameters, UpdateKind};
use teloxide::utils::command::BotCommands;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

mod apis;

mod utils;
use utils::{download_and_encode_image, find_prompt, get_image_from_message, parse_webhook};

#[derive(BotCommands, Clone, Debug, PartialEq)]
#[command(rename_rule = "lowercase", description = "Models from OpenRouter")]
enum BotCommand {
    #[command(description = "display this text")]
    Help,
    #[command(description = "welcome message")]
    Start,
    #[command(description = "caveman version of llama3.1")]
    Caveman,
    #[command(description = "llama3.1 70b or llama 3.2 12b vision", alias = "l")]
    Llama,
    #[command(description = "llama 3.2 1b", alias = "1b")]
    Lobotomy,
    #[command(description = "free flux[schnell] from together.ai")]
    Flux,
    #[command(description = "llama 3.1 405b", aliases = ["405b", "405"])]
    Llama405,
    // #[command(description = "cunnyGPT degenerate copypastas", alias = "cunnygpt")]
    // CunnyGPT,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_env_filter(EnvFilter::new("sussy_ducky_bot=debug"))
        .init();

    info!("Cold-starting the Lambda function");

    // Setup telegram bot (we do it here because this place is a cold start)
    let bot = Bot::new(env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set!"));
    info!("Telegram bot initialized");

    // Set commands
    let res = bot.set_my_commands(BotCommand::bot_commands()).await;

    match res {
        Ok(_) => info!("Bot commands set successfully"),
        Err(e) => warn!("Failed to set commands: {:?}", e),
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
            error!("Failed to parse webhook: {:?}", e);
            return Ok(lambda_http::Response::builder()
                .status(200)
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
                return handle_command(bot.clone(), message, command).await;
            }

            // Secret bawialnia easter egg
            if let UpdateKind::Message(message) = &update.kind {
                if message.text().is_some()
                    && (message.chat.id == ChatId(-1001865084475)
                        || message.chat.id == ChatId(-1001641972650))
                {
                    let random: f64 = rand::random();
                    debug!("Random number: {}", random);
                    if random < 0.001 {
                        // 0.1% chance of triggering
                        // this has a bug, if the message starts with a command, the bot will respond with an error
                        return handle_command(bot.clone(), message, BotCommand::Caveman).await;
                    }
                }
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
    command: BotCommand,
) -> Result<lambda_http::Response<String>, lambda_http::Error> {
    info!("Handling command: {:?}", command);

    // Help & start command
    if command == BotCommand::Help || command == BotCommand::Start {
        bot.send_message(message.chat.id, BotCommand::descriptions().to_string())
            .await
            .unwrap();
        return Ok(lambda_http::Response::builder()
            .status(200)
            .body(String::new())
            .unwrap());
    }

    if command == BotCommand::Flux {
        // Just the prompt, no image
        let Some(msg_text) = find_prompt(message).await else {
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
        };

        // Send typing indicator
        bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::Typing)
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
            error!("Failed to submit request: {:?}", e);
            bot.send_message(message.chat.id, format!("error: {e:?}"))
                .reply_parameters(ReplyParameters::new(message.id))
                .await
                .unwrap();
            return Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap());
        }

        let response = res.unwrap();
        // Get the inference time and JSON
        let inference_time = &response.data[0].timings.inference;
        let base64 = &response.data[0].b64_json;
        info!(
            "Inference time: {:.2}s for prompt: {}",
            inference_time, msg_text
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
            .caption(format!("{msg_text} (in {inference_time:.2}s)"))
            .reply_parameters(ReplyParameters::new(message.id))
            .await;

        if let Err(e) = res {
            error!("Failed to send message: {:?}", e);
        }

        return Ok(lambda_http::Response::builder()
            .status(200)
            .body(String::new())
            .unwrap());
    }

    // Now all that is left is the OpenAI-compatible models

    // Get the image file, if any
    let img = get_image_from_message(message);

    let msg_text = match find_prompt(message).await {
        Some(prompt) => prompt,
        None => {
            if img.is_some() {
                info!("No prompt found in the message or reply message, but image found");
                // Return msg_text as an empty string
                String::new()
            } else {
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
        }
    };

    let base64_img = match img {
        Some(photo) => Some(download_and_encode_image(&bot, &photo).await.unwrap()),
        None => None,
    };

    // Send typing indicator
    bot.send_chat_action(message.chat.id, ChatAction::Typing)
        .await
        .unwrap();

    // Send the request
    let client = OpenAIClient::new();
    let res = client
        .openai_request(&msg_text, base64_img.as_deref(), command)
        .await;

    // Catch error
    if let Err(e) = res {
        bot.send_message(message.chat.id, format!("error: {e:?}"))
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
    let res = bot
        .send_message(message.chat.id, &response_text)
        .reply_parameters(ReplyParameters::new(message.id))
        .await;

    if let Err(e) = res {
        error!("Failed to send message: {:?}", e);
    }

    Ok(lambda_http::Response::builder()
        .status(200)
        .body(String::new())
        .unwrap())
}
