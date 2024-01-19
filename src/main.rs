use log::{debug, info};

use teloxide::{
    prelude::*,
    types::{BotCommand, True},
    RequestError,
};

mod structs;
use structs::{OllamaRequest, OllamaResponse, TTSRequest};

mod utils;
use utils::{parse_command, parse_command_in_caption, ModelType};

mod commands;
use commands::*;

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

#[rustfmt::skip]
async fn set_commands(bot: &Bot) -> Result<True, RequestError> {
    let commands = vec![
        BotCommand::new("mistral", "Generate text using mistral LLM"),
        BotCommand::new("dolphin", "Generate text using dolphin-mistral LLM"),
        BotCommand::new("orca", "Generate text using mistral-openorca LLM"),
        BotCommand::new("tinyllama", "Generate text using tinyllama LLM (experimental)",),
        BotCommand::new("llava", "Generate text from image using llava multi-modal LLM",),
        BotCommand::new("help", "Show available commands"),
        BotCommand::new("ping", "Check the bot's latency"),
        BotCommand::new("httpcat", "Get an image of a cat for a given HTTP status code",),
        BotCommand::new("tts", "Text to speech using random OpenAI voice"),
        BotCommand::new("caveman", "Generate text using mistral LLM in caveman language",
        ),
    ];

    bot.set_my_commands(commands).await
}

async fn handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    // info!("Received message: {}", msg.text().unwrap_or(""));

    // Check if the message is a message or an image with a caption
    if msg.photo().is_some() && msg.caption().is_some() {
        info!("Message is an image with a caption");
        let (command, args) = parse_command_in_caption(msg.clone());
        let command = command.unwrap_or(String::new());
        let args = args.unwrap_or(String::new());
        let msg = msg.clone(); // Clone the message here

        match command.as_str() {
            "/llava" | "/l" => {
                let prompt = args;
                debug!("Executing llava command with prompt: {}", prompt);
                llava(bot, msg.clone(), prompt).await?;
            }
            _ => {}
        }
    } else if msg.text().is_some() {
        // info!("Message is a text message");
        let (command, args) = parse_command(msg.clone());
        let command = command.unwrap_or(String::new());
        let args = args.unwrap_or(String::new());
        let msg = msg.clone(); // Clone the message here
        match command.as_str() {
            "/mistral" | "/m" => {
                ollama(bot.clone(), msg, args.clone(), ModelType::MistralStandard).await?;
            }
            "/caveman" => {
                ollama(bot.clone(), msg, args.clone(), ModelType::MistralCaveman).await?;
            }
            "/dolphin" => {
                ollama(bot.clone(), msg, args.clone(), ModelType::MistralDolphin).await?;
            }
            "/orca" => {
                ollama(bot.clone(), msg, args.clone(), ModelType::MistralOpenOrca).await?;
            }
            "/tinyllama" => {
                ollama(bot.clone(), msg, args.clone(), ModelType::TinyLlama).await?;
            }
            "/llava" | "/l" => {
                llava(bot.clone(), msg, args.clone()).await?;
            }
            "/help" | "/h" => {
                help(bot.clone(), msg).await?;
            }
            "/ping" => {
                ping(bot.clone(), msg).await?;
            }
            "/httpcat" => {
                httpcat(bot.clone(), msg, args.clone()).await?;
            }
            "/tts" => {
                tts(bot.clone(), msg, args.clone()).await?;
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
