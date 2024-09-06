use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use lambda_http::{run, service_fn, Body, Error, Request};
use mime::Mime;
use utils::delete_message_delay;
use utils::split_string;
use core::str;
use std::env;
use std::str::FromStr;
use teloxide::types::ChatAction;
use teloxide::types::Message;
use teloxide::types::ReplyParameters;
use teloxide::types::UpdateKind;
use teloxide::utils::command::BotCommands;
use teloxide::{net::Download, prelude::*};
use tracing::{debug, error, info, warn};
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
    #[command(description = "llama3.1 70b", alias="l")]
    Llama,
    #[command(description = "llava 7b vision model", alias="v")]
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

    // Set commands
    let res = bot.set_my_commands(BotCommand::bot_commands()).await;

    if let Err(e) = res {
        warn!("Failed to set commands: {:?}", e);
    }

    // Run the Lambda function
    run(service_fn(|req| handler(req, &bot, &dynamodb, &kms))).await
}

async fn handler(
    req: lambda_http::Request,
    bot: &Bot,
    dynamodb: &aws_sdk_dynamodb::Client,
    kms: &aws_sdk_kms::Client,
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
                return handle_command(bot.clone(), message, command, dynamodb).await;
            }
        }
    }

    // Handle audio messages
    handle_audio_message(update, bot, dynamodb, kms).await
}

async fn handle_command(
    bot: Bot,
    message: &Message,
    command: BotCommand,
    dynamodb: &aws_sdk_dynamodb::Client,
) -> Result<lambda_http::Response<String>, lambda_http::Error> {
    match command {
        BotCommand::Help => {
            bot.send_message(message.chat.id, BotCommand::descriptions().to_string())
                .await
                .unwrap();
        }
        BotCommand::Start => {
            bot.send_message(message.chat.id, "Welcome!").await.unwrap();
                .await
                .unwrap();
        }
        BotCommand::Caveman => {
            bot.send_message(message.chat.id, "todo: Caveman version of llama3.1").await.unwrap();
        }
        BotCommand::Llama => {
            bot.send_message(message.chat.id, "todo: llama3.1 70b").await.unwrap();
        }
        BotCommand::Llava => {
            bot.send_message(message.chat.id, "todo: llava 7b vision model").await.unwrap();
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