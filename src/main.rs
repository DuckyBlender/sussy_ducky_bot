use lambda_http::{run, service_fn, Body, Error, Request};

use base64::{engine::general_purpose, Engine as _};
use reqwest::Client as ReqwestClient;
use serde_json::{json, Value};
use std::env;
use teloxide::net::Download;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{Message, PhotoSize, ReplyParameters, UpdateKind};
use teloxide::utils::command::BotCommands;
use tracing::{debug, error, info, warn};
use tracing_subscriber::fmt;

const BASE_URL: &str = "https://api.groq.com/openai/v1";

#[derive(BotCommands, Clone, Debug, PartialEq)]
#[command(rename_rule = "lowercase")]
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
            let response = send_chat_request(client, groq_key, system_prompt, &msg_text).await;
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
            let response = send_vision_request(client, groq_key, &base64_img, &msg_text).await;
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

pub async fn parse_webhook(input: Request) -> Result<Update, Error> {
    debug!("Parsing webhook");
    let body = input.body();
    let body_str = match body {
        Body::Text(text) => text,
        not => {
            error!("Expected Body::Text(...) got {:?}", not);
            panic!("expected Body::Text(...) got {:?}", not);
        }
    };
    let body_json: Update = serde_json::from_str(body_str)?;
    debug!("Successfully parsed webhook");
    Ok(body_json)
}

async fn send_chat_request(
    client: &ReqwestClient,
    groq_key: &str,
    system_prompt: &str,
    msg_text: &str,
) -> anyhow::Result<Value> {
    info!("Sending chat request to Groq API");
    let request_body = json!({
        "model": "llama-3.1-70b-versatile",
        "messages": [
            {
                "role": "system",
                "content": system_prompt
            },
            {
                "role": "user",
                "content": msg_text
            }
        ],
        "max_tokens": 1024
    });

    debug!("Chat request body: {:?}", request_body);

    let response = client
        .post(format!("{}/chat/completions", BASE_URL))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", groq_key))
        .json(&request_body)
        .send()
        .await?;

    debug!("Received response status: {:?}", response.status());

    let response_body = response.text().await?;
    debug!("Response body: {}", response_body);

    let json_response: Value = serde_json::from_str(&response_body)?;
    Ok(json_response)
}

async fn send_vision_request(
    client: &ReqwestClient,
    groq_key: &str,
    base64_img: &str,
    msg_text: &str,
) -> anyhow::Result<Value> {
    info!("Sending vision request to Groq API");
    let request_body = json!({
        "model": "llava-v1.5-7b-4096-preview",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": msg_text
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/jpeg;base64,{}", base64_img)
                        }
                    }
                ]
            }
        ],
        "max_tokens": 1024
    });

    debug!("Vision request body: {:?}", request_body);

    let response = client
        .post(format!("{}/chat/completions", BASE_URL))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", groq_key))
        .json(&request_body)
        .send()
        .await?;

    debug!("Received vision response status: {:?}", response.status());

    let response_body = response.text().await?;
    debug!("Vision response body: {}", response_body);

    let json_response: Value = serde_json::from_str(&response_body)?;
    Ok(json_response)
}

fn get_image_from_message(message: &Message) -> Option<PhotoSize> {
    if let Some(photo) = message.photo() {
        let photo = photo.first().unwrap();
        Some(photo.clone())
    } else if let Some(photo) = message.reply_to_message().and_then(|m| m.photo()) {
        let photo = photo.first().unwrap();
        return Some(photo.clone());
    } else {
        return None;
    }
}

async fn download_and_encode_image(bot: &Bot, photo: &PhotoSize) -> anyhow::Result<String> {
    let mut buf: Vec<u8> = Vec::new();
    let file = bot.get_file(&photo.file.id).await?;
    bot.download_file(&file.path, &mut buf).await?;

    let base64_img = general_purpose::STANDARD.encode(&buf).to_string();

    Ok(base64_img)
}

async fn remove_command(text: &str) -> String {
    let mut words = text.split_whitespace();
    // if first starts with /
    let text = if let Some(word) = words.next() {
        if word.starts_with('/') {
            words.collect::<Vec<&str>>().join(" ")
        } else {
            text.to_string()
        }
    } else {
        text.to_string()
    };
    text.trim().to_string()
}

async fn find_prompt(message: &Message) -> Option<String> {
    // msg_text contains the text of the message or reply to a message with text
    let msg_text = message.text();

    if msg_text.is_none() {
        warn!("No text found in the message");
        return None;
    }

    let msg_text = msg_text.unwrap();
    let msg_text = remove_command(msg_text).await;
    let msg_text = if msg_text.is_empty() {
        // Find in reply message
        if let Some(reply) = message.reply_to_message() {
            if let Some(text) = reply.text() {
                text
            } else {
                warn!("No text found in the reply message");
                return None;
            }
        } else {
            warn!("No text found in the message & no reply message");
            return None;
        }
    } else {
        &msg_text
    };

    debug!("Message text: {}", msg_text);
    Some(msg_text.to_string())
}
