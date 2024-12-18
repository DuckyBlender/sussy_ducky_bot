#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_lines)]

use apis::{HuggingFaceClient, ImageRequest, OpenAIClient, TogetherClient};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::Engine as _;
use lambda_http::{run, service_fn, Error};
use reqwest::Url;
use serde_json::json;

use std::env;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, InputFile, Message, ReplyParameters, UpdateKind};
use teloxide::utils::command::BotCommands;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

mod apis;

mod utils;
use utils::{
    download_and_encode_image, find_prompt, get_image_from_message, parse_webhook,
    remove_g_segment, safe_send,
};

#[derive(BotCommands, Clone, Debug, PartialEq)]
#[command(rename_rule = "lowercase", description = "Models from OpenRouter")]
enum BotCommand {
    #[command(description = "display this text")]
    Help,
    #[command(description = "welcome message")]
    Start,
    #[command(description = "caveman version of llama3.1")]
    Caveman,
    #[command(description = "llama3.1 70b or llama 3.2 90b vision", alias = "l")]
    Llama,
    #[command(description = "llama 3.2 1b", alias = "1b")]
    Lobotomy,
    #[command(description = "free flux[schnell] from together.ai")]
    Flux,
    #[command(description = "free and fast text-to-video", aliases = ["videogen", "video"])]
    T2V,
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
        if let Some(text) = message.text().or_else(|| message.caption()) {
            debug!("Received text or caption: {}", text);
            if let Ok(command) = BotCommand::parse(text, bot.get_me().await.unwrap().username()) {
                info!("Parsed command: {:?}", command);
                return handle_command(bot.clone(), message, command).await;
            }
        }
    }

    // Secret bawialnia easter egg
    if let UpdateKind::Message(message) = &update.kind {
        if message.text().is_some()
            && (message.chat.id == ChatId(-1001865084475)
                || message.chat.id == ChatId(-1001641972650))
        {
            let random: f64 = rand::random();
            // debug!("Random number: {}", random);
            if random < 0.001 {
                // 0.1% chance of triggering
                // this has a bug, if the message starts with a command, the bot will respond with an error
                return handle_command(bot.clone(), message, BotCommand::Caveman).await;
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

    match command {
        BotCommand::Help | BotCommand::Start => {
            let help_text = BotCommand::descriptions().to_string();
            safe_send(bot, message.chat.id, message.id, &help_text).await;
            Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap())
        }

        // This command is extremely sketchy and temporary
        BotCommand::T2V => {
            // Just the prompt, no image
            let Some(msg_text) = find_prompt(message).await else {
                warn!("No prompt found in the message or reply message");
                safe_send(bot, message.chat.id, message.id, "Please provide a prompt.").await;

                return Ok(lambda_http::Response::builder()
                    .status(200)
                    .body(String::new())
                    .unwrap());
            };

            // Send typing indicator
            bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::Typing)
                .await
                .unwrap();

            // Send the request to the HF API
            let client = HuggingFaceClient::new();

            let seed = rand::random::<u32>();

            let url = Url::parse("https://tiger-lab-t2v-turbo-v2.hf.space").unwrap();
            let data = json!([
                msg_text, // Text
                7.5,      // Guidance scale
                0.5,      // Percentage of steps to apply motion guidance
                16,       // Number of inference steps
                16,       // Number of video frames
                seed,     // Seed
                false,    // Randomize seed
                "bf16"    // torch.dtype
            ]);

            let output = client.request(url, data).await;

            match output {
                Ok(event_id) => {
                    // Send the response
                    let url = event_id[0]["video"]["url"].as_str().unwrap();
                    info!("Video URL: {}", url);

                    let url = Url::parse(url).unwrap();

                    let url = remove_g_segment(url);

                    // Send sending video indicator
                    bot.send_chat_action(message.chat.id, ChatAction::UploadVideo)
                        .await
                        .unwrap();
                    let res = bot
                        .send_video(message.chat.id, InputFile::url(url))
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

                Err(e) => {
                    error!("Failed to submit request: {:?}", e);
                    safe_send(bot, message.chat.id, message.id, &format!("error: {e:?}")).await;

                    Ok(lambda_http::Response::builder()
                        .status(200)
                        .body(String::new())
                        .unwrap())
                }
            }
        }

        BotCommand::Flux => {
            // Just the prompt, no image
            let Some(msg_text) = find_prompt(message).await else {
                warn!("No prompt found in the message or reply message");
                safe_send(bot, message.chat.id, message.id, "Please provide a prompt.").await;

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
                safe_send(bot, message.chat.id, message.id, &format!("error: {e:?}")).await;
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

            Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap())
        }
        _ => {
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
                        safe_send(bot, message.chat.id, message.id, "Please provide a prompt.")
                            .await;

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
                error!("Failed to submit request: {:?}", e);
                safe_send(bot, message.chat.id, message.id, &format!("error: {e:?}")).await;

                return Ok(lambda_http::Response::builder()
                    .status(200)
                    .body(String::new())
                    .unwrap());
            }

            let response_text = res.unwrap();

            // Check if empty response
            if response_text.is_empty() {
                warn!("Empty response from API");
                safe_send(bot, message.chat.id, message.id, "<no text>").await;

                return Ok(lambda_http::Response::builder()
                    .status(200)
                    .body(String::new())
                    .unwrap());
            }

            // Safe send the response
            safe_send(bot, message.chat.id, message.id, &response_text).await;

            Ok(lambda_http::Response::builder()
                .status(200)
                .body(String::new())
                .unwrap())
        }
    }
}
