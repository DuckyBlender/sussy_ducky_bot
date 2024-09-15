use apis::send_groq_request;
use lambda_http::{run, service_fn, Error};

use reqwest::Client as ReqwestClient;
use std::env;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{Message, ReplyParameters, UpdateKind};
use teloxide::utils::command::BotCommands;
use tracing::{debug, error, info, warn};
use tracing_subscriber::fmt;

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
    #[command(description = "llava 7b vision model", alias = "v")]
    Llava,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing for logging
    fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .init();

    info!("Starting the Telegram bot application");

    // Setup telegram bot (we do it here because this place is a cold start)
    let bot = Bot::new(env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set!"));
    info!("Telegram bot initialized");

    let groq_key = env::var("GROQ_KEY").expect("GROQ_KEY not set!");
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
    run(service_fn(|req| handler(req, &bot, &client, &groq_key))).await
}

async fn handler(
    req: lambda_http::Request,
    bot: &Bot,
    client: &ReqwestClient,
    groq_key: &str,
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
                return handle_command(bot.clone(), message, command, client, groq_key).await;
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
                return handle_command(bot.clone(), message, BotCommand::Caveman, client, groq_key)
                    .await;
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
    groq_key: &str,
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

    match command {
        BotCommand::Llama | BotCommand::Caveman => {
            info!("Handling Llama or Caveman command");
            // Typing indicator
            bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::Typing)
                .await
                .unwrap();

            let system_prompt = match command {
                BotCommand::Llama => "Be concise and precise. Don't be verbose. Answer in the user's language.",
                BotCommand::Caveman => "You are a caveman. Speak like a caveman would. All caps, simple words, grammar mistakes etc.",
                _ => unreachable!(),
            };

            debug!("System prompt: {}", system_prompt);

            // Send request to groq
            let response =
                send_groq_request(client, groq_key, system_prompt, &msg_text, None).await;
            let response = match response {
                Ok(response) => {
                    debug!("Received response from Groq: {:?}", response);
                    response
                }
                Err(e) => {
                    error!("Failed to get response from AI API: {:?}", e);
                    bot.send_message(message.chat.id, "Failed to get response from API")
                        .reply_parameters(ReplyParameters::new(message.id))
                        .await
                        .unwrap();
                    return Ok(lambda_http::Response::builder()
                        .status(200)
                        .body("Failed to get response from AI API".into())
                        .unwrap());
                }
            };

            let text_response = response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("<no response>");

            info!("Sending response to user: {}", text_response);

            bot.send_message(message.chat.id, text_response)
                .reply_parameters(ReplyParameters::new(message.id))
                .await
                .unwrap();
        }
        BotCommand::Llava => {
            info!("Handling Llava command");
            // If there is no image, return
            let img = match img {
                Some(img) => img,
                None => {
                    warn!("No image provided for Llava command");
                    bot.send_message(message.chat.id, "Please provide an image to analyze.")
                        .reply_parameters(ReplyParameters::new(message.id))
                        .await
                        .unwrap();
                    return Ok(lambda_http::Response::builder()
                        .status(200)
                        .body(String::new())
                        .unwrap());
                }
            };

            // Typing indicator
            bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::Typing)
                .await
                .unwrap();

            debug!("Downloading and processing image");

            // Download and process the image
            let base64_img = match download_and_encode_image(&bot, &img).await {
                Ok(result) => result,
                Err(e) => {
                    error!("Failed to download and encode image: {:?}", e);
                    bot.send_message(message.chat.id, "Failed to process the image")
                        .reply_parameters(ReplyParameters::new(message.id))
                        .await
                        .unwrap();
                    return Ok(lambda_http::Response::builder()
                        .status(200)
                        .body("Failed to process the image".into())
                        .unwrap());
                }
            };

            debug!("Sending vision request with processed image");

            // Send request to groq
            let response =
                send_groq_request(client, groq_key, &base64_img, &msg_text, Some(&base64_img))
                    .await;
            let response = match response {
                Ok(response) => {
                    debug!("Received vision response from Groq: {:?}", response);
                    response
                }
                Err(e) => {
                    error!("Failed to get vision response from AI API: {:?}", e);
                    bot.send_message(message.chat.id, "Failed to get response from API")
                        .reply_parameters(ReplyParameters::new(message.id))
                        .await
                        .unwrap();
                    return Ok(lambda_http::Response::builder()
                        .status(200)
                        .body("Failed to get response from AI API".into())
                        .unwrap());
                }
            };

            let text_response = response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("<no response>");

            info!("Sending vision response to user: {}", text_response);

            bot.send_message(message.chat.id, text_response)
                .reply_parameters(ReplyParameters::new(message.id))
                .await
                .unwrap();
        }
        BotCommand::Help | BotCommand::Start => {
            unreachable!();
        }
    }

    Ok(lambda_http::Response::builder()
        .status(200)
        .body(String::new())
        .unwrap())
}
