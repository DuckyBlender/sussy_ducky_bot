use log::{debug, info};

use teloxide::{
    prelude::*,
    types::{BotCommand, True},
    RequestError,
};

mod structs;
use structs::{OllamaRequest, OllamaResponse, TTSRequest};

mod utils;
use utils::{parse_command, parse_command_in_caption};

mod commands;
use commands::{help, httpcat, llava, mistral, ping, start, tts};

const TTS_VOICES: [&str; 6] = ["alloy", "echo", "fable", "onyx", "nova", "shimmer"];

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    info!("Starting command bot...");

    let bot = Bot::from_env();

    set_commands(&bot).await.unwrap();

    teloxide::repl(bot, handler).await;
}

async fn set_commands(bot: &Bot) -> Result<True, RequestError> {
    let commands = vec![
        BotCommand::new("mistral", "Generate text using Mistral7B"),
        BotCommand::new("llava", "Generate text from image using Llava"),
        BotCommand::new("help", "Show available commands"),
        BotCommand::new("ping", "Check the bot's latency"),
        BotCommand::new(
            "httpcat",
            "Get an image of a cat for a given HTTP status code",
        ),
        BotCommand::new("tts", "Text to speech using random OpenAI voice"),
        BotCommand::new(
            "caveman",
            "Generate text using Mistral7B in caveman language",
        ),
    ];

    bot.set_my_commands(commands).await
}

async fn handler(bot: Bot, msg: &'static Message) -> ResponseResult<()> {
    // info!("Received message: {}", msg.text().unwrap_or(""));

    // Check if the message is a message or an image with a caption
    if msg.photo().is_some() && msg.caption().is_some() {
        info!("Message is an image with a caption");
        let (command, args) = parse_command_in_caption(&msg);
        // debug!("Command: {:?}, args: {:?}", command, args);
        match command {
            Some("/llava" | "/l") => {
                let prompt = args.unwrap_or("").to_string();
                debug!("Executing llava command with prompt: {}", prompt);
                llava(bot, msg.clone(), prompt).await?;
            }
            _ => {}
        }
    } else if msg.text().is_some() {
        // info!("Message is a text message");
        let (command, args) = parse_command(&msg);
        match command {
            Some("/mistral") | Some("/m") => {
                tokio::spawn(mistral(
                    bot.clone(),
                    msg.clone(),
                    args.unwrap_or("").to_string(),
                    false,
                ));
            }
            Some("/caveman") => {
                tokio::spawn(mistral(
                    bot.clone(),
                    msg.clone(),
                    args.unwrap_or("").to_string(),
                    true,
                ));
            }
            Some("/llava") | Some("/l") => {
                tokio::spawn(llava(
                    bot.clone(),
                    msg.clone(),
                    args.unwrap_or("").to_string(),
                ));
            }
            Some("/help") | Some("/h") => {
                tokio::spawn(help(bot.clone(), &msg));
            }
            Some("/start") => {
                tokio::spawn(start(bot.clone(), &msg));
            }
            Some("/ping") => {
                tokio::spawn(ping(bot.clone(), &msg));
            }
            Some("/httpcat") => {
                tokio::spawn(httpcat(bot.clone(), &msg, args));
            }
            Some("/tts") => {
                tokio::spawn(tts(bot.clone(), &msg, args));
            }
            _ => {
                // If the command is not recognized, do nothing
            }
        }
    } else {
        // info!("Message is not a text message nor an image with a caption");
    }

    Ok(())
}
