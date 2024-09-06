use async_openai::config::OpenAIConfig;
use async_openai::types::*;
use async_openai::Client;
use core::str;
use lambda_http::{run, service_fn, Body, Error, Request};
use std::env;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::Message;
use teloxide::types::ReplyParameters;
use teloxide::types::UpdateKind;
use teloxide::utils::command::BotCommands;
use tracing::{error, warn};
use tracing_subscriber::fmt;

#[derive(BotCommands, Clone)]
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
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    // Setup telegram bot (we do it here because this place is a cold start)
    let bot = Bot::new(env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set!"));

    let openai_config = OpenAIConfig::new()
        .with_api_key(env::var("GROQ_KEY").unwrap())
        .with_api_base("https://api.groq.com/openai/v1");

    let client = Client::with_config(openai_config);

    // Set commands
    let res = bot.set_my_commands(BotCommand::bot_commands()).await;

    if let Err(e) = res {
        warn!("Failed to set commands: {:?}", e);
    }

    // Run the Lambda function
    run(service_fn(|req| handler(req, &bot, &client))).await
}

async fn handler(
    req: lambda_http::Request,
    bot: &Bot,
    client: &Client<OpenAIConfig>,
) -> Result<lambda_http::Response<String>, lambda_http::Error> {
    // Parse JSON webhook
    let bot = bot.clone();

    let update = match parse_webhook(req).await {
        Ok(message) => message,
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
            if let Ok(command) = BotCommand::parse(text, bot.get_me().await.unwrap().username()) {
                return handle_command(bot.clone(), message, command, client).await;
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
    client: &Client<OpenAIConfig>,
) -> Result<lambda_http::Response<String>, lambda_http::Error> {
    // msg_text contains the text of the message or reply to a message with text
    let msg_text = message
        .text()
        .or_else(|| message.reply_to_message().and_then(|m| m.text()))
        .unwrap_or_default();

    // img contains the url of the image, sticker or reply to a message with an image or sticker
    let img = message
        .photo()
        .and_then(|p| p.last())
        .map(|p| p.file.id.clone())
        .or_else(|| message.sticker().map(|s| s.file.id.clone()))
        .or_else(|| {
            message
                .reply_to_message()
                .and_then(|m| m.photo().and_then(|p| p.last()).map(|p| p.file.id.clone()))
        })
        .or_else(|| {
            message
                .reply_to_message()
                .and_then(|m| m.sticker().map(|s| s.file.id.clone()))
        });

    match command {
        BotCommand::Help => {
            bot.send_message(message.chat.id, BotCommand::descriptions().to_string())
                .await
                .unwrap();
        }
        BotCommand::Start => {
            bot.send_message(message.chat.id, BotCommand::descriptions().to_string())
                .await
                .unwrap();
        }

        BotCommand::Llama | BotCommand::Caveman => {
            // Typing indicator
            bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::Typing)
                .await
                .unwrap();

            let system_prompt = match command {
                BotCommand::Llama => "Be concise and precise. Don't be verbose. Answer in the user's language.",
                BotCommand::Caveman => "You are a caveman. Speak like a caveman would. All caps, simple words, grammar mistakes etc.",
                _ => unreachable!(),
            };

            // Send request to groq
            let request = return_chat_request(system_prompt, msg_text);

            let response = client.chat().create(request).await;
            let response = match response {
                Ok(response) => response,
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

            let text_response = response.choices[0]
                .message
                .content
                .clone()
                .unwrap_or("<no response>".to_string());

            bot.send_message(message.chat.id, text_response)
                .reply_parameters(ReplyParameters::new(message.id))
                .await
                .unwrap();
        }
        BotCommand::Llava => {
            // If there is no image, return
            let img = match img {
                Some(img) => img,
                None => {
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

            // Send request to groq
            let request = return_multimodal_request(&img, msg_text);

            let response = client.chat().create(request).await;
            let response = match response {
                Ok(response) => response,
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

            let text_response = response.choices[0]
                .message
                .content
                .clone()
                .unwrap_or("<no response>".to_string());

            bot.send_message(message.chat.id, text_response)
                .reply_parameters(ReplyParameters::new(message.id))
                .await
                .unwrap();
        }
    }

    Ok(lambda_http::Response::builder()
        .status(200)
        .body(String::new())
        .unwrap())
}

pub async fn parse_webhook(input: Request) -> Result<Update, Error> {
    let body = input.body();
    let body_str = match body {
        Body::Text(text) => text,
        not => panic!("expected Body::Text(...) got {not:?}"),
    };
    let body_json: Update = serde_json::from_str(body_str)?;
    Ok(body_json)
}

fn return_multimodal_request(
    img: &str,
    msg_text: &str,
) -> CreateChatCompletionRequest {
    let mut req = CreateChatCompletionRequestArgs::default()
        .model("llava-v1.5-7b-4096-preview")
        .max_tokens(1024_u32)
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content(vec![
                ChatCompletionRequestMessageContentPartTextArgs::default()
                    .text(msg_text)
                    .build()
                    .unwrap()
                    .into(),
                ChatCompletionRequestMessageContentPartImageArgs::default()
                    .image_url(
                        ImageUrlArgs::default()
                            .url(img)
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap()
                    .into(),
            ])
            .build()
            .unwrap()
            .into()])
        .build()
        .unwrap();

    req.service_tier = None; // groq doesn't support service_tier
    req
}

fn return_chat_request(system_prompt: &str, msg_text: &str) -> CreateChatCompletionRequest {
    let mut req = CreateChatCompletionRequestArgs::default()
        .model("llama-3.1-70b-versatile")
        .max_tokens(1024_u32)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()
                .unwrap()
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(msg_text)
                .build()
                .unwrap()
                .into(),
        ])
        .build()
        .unwrap();

    req.service_tier = None; // groq doesn't support service_tier
    req
}