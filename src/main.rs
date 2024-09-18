use apis::{FalClient, ImageRequest, OpenAIClient};
use lambda_http::{run, service_fn, Error};
use reqwest::Url;

use std::env;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{InputFile, Message, ParseMode, ReplyParameters, UpdateKind};
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
    #[command(description = "pixtral 12b vision model", aliases = ["v", "pixtral"])]
    Vision,
    #[command(description = "flux[schnell]", hide)]
    Flux,
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

    let update = match parse_webhook(req).await {
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
                return handle_command(bot.clone(), message, BotCommand::Caveman).await;
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

    // Flux command
    // Flux command
if command == BotCommand::Flux {
    // Just the prompt, no image
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

    // Send typing indicator
    bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::Typing)
        .await
        .unwrap();

    // Send the request
    let image_request = ImageRequest {
        prompt: msg_text,
        image_size: "landscape_4_3".to_string(),
        num_inference_steps: 4,
        num_images: 1,
        enable_safety_checker: false,
    };

    let client = FalClient::new();
    let response = match client.submit_request(image_request).await {
        Ok(response) => response,
        Err(err) => {
            error!("Error submitting request: {}", err);
            return Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap());
        }
    };

    let mut total_waiting = 0;

    // Initial wait of 1 second
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    total_waiting += 1;

    let mut status;
    loop {
        status = match client.check_status(&response.request_id).await {
            Ok(status) => status,
            Err(err) => {
                error!("Error checking status: {}", err);
                return Ok(lambda_http::Response::builder()
                    .status(200)
                    .body(String::new())
                    .unwrap());
            }
        };

        debug!("Request status: {:?}", status.status);

        if status.status.to_lowercase() == "completed" {
            info!("Request completed, total waiting time: {} seconds", total_waiting);
            break;
        }

        // Wait for 1 second before checking again
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        total_waiting += 1;
    }

    let result = match client.get_result(&response.request_id).await {
        Ok(result) => result,
        Err(err) => {
            error!("Error fetching result: {}", err);
            return Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap());
        }
    };

    for image in result.images {
        // info!("Image URL: {}", image.url);

        if let Err(err) = bot
            .send_chat_action(message.chat.id, teloxide::types::ChatAction::UploadPhoto)
            .await
        {
            error!("Failed to send chat action: {}", err);
            return Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap());
        }

        if let Err(err) = bot
            .send_photo(
                message.chat.id,
                InputFile::url(Url::parse(&image.url).unwrap()),
            )
            .reply_parameters(ReplyParameters::new(message.id))
            .await
        {
            error!("Failed to send photo: {}", err);
            return Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap());
        }
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
    bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::Typing)
        .await
        .unwrap();

    // Send the request
    let client = OpenAIClient::new();
    let res = client
        .openai_request(&msg_text, base64_img.as_deref(), command)
        .await;

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

    // Escape markdown characters
    let escaped_response_text = escape_markdown(&response_text);

    debug!("Before escaping: {}", response_text);
    debug!("After escaping: {}", escaped_response_text);

    // Try sending the response as markdown
    let res = bot
        .send_message(message.chat.id, escaped_response_text)
        .reply_parameters(ReplyParameters::new(message.id))
        .parse_mode(ParseMode::MarkdownV2)
        .await;

    if let Err(e) = res {
        error!("Failed to send markdown message: {:?}", e);
        bot.send_message(message.chat.id, response_text)
            .reply_parameters(ReplyParameters::new(message.id))
            .await
            .unwrap();
    }

    Ok(lambda_http::Response::builder()
        .status(200)
        .body(String::new())
        .unwrap())
}


