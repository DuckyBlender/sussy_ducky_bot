use log::{debug, error, info};

use teloxide::{
    prelude::*,
    types::{BotCommand, True},
    RequestError,
};

mod structs;
use structs::{OllamaRequest, OllamaResponse};

mod utils;
use utils::{parse_command, parse_command_in_caption, ModelType};

mod commands;
use commands::*;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    info!("Starting command bot...");

    let bot = Bot::from_env();

    let commands = Commands::new();

    if let Err(err) = commands.set_commands(&bot).await {
        error!("Failed to set commands: {}", err);
    }

    teloxide::repl(bot, handler).await;
}

struct Commands(Vec<BotCommand>);

impl Commands {
    #[rustfmt::skip]
    pub fn new() -> Self {
        Self(
            vec![
                BotCommand::new("uncensored", "Generate text using 7B dolphin-mistral LLM"),
                BotCommand::new("mistral", "Generate text using 7B mistral-openorca LLM"),
                BotCommand::new("tinyllama", "Generate text using 1B tinyllama LLM [EXPERIMENTAL]",),
                BotCommand::new("llava", "Generate text from image using 7B llava multi-modal LLM",),
                BotCommand::new("help", "Show available commands"),
                BotCommand::new("ping", "Check the bot's latency"),
                BotCommand::new("httpcat", "Get an image of a cat for a given HTTP status code",),
                BotCommand::new("caveman", "Generate text using mistral LLM in caveman language"),
                BotCommand::new("online", "Generate text using the pplx-7b-online model from PerplexityAI [TESTING]"),
                BotCommand::new("mixtral", "Generate text using the mixtral-8x7b-instruct model from PerplexityAI [TESTING]"),
                BotCommand::new("img", "Generate an image using the Amazon Titan Image Generator G1 [TESTING]"),
            ]
        )
    }

    pub async fn set_commands(&self, bot: &Bot) -> Result<True, RequestError> {
        bot.set_my_commands(self.0.clone()).await
    }

    pub fn help_message(&self) -> String {
        let header = "⚠️ SINCE THIS BOT IS SELF-HOSTED, IT CAN BE QUITE SLOW. REWRITE IS IN PROGRESS ⚠️\nThis bot is open source! Check it out at https://github.com/DuckyBlender/sussy_ducky_bot\nOh and the bot works with replies too! (for example you can reply to a photo with /llava)\nCommands marked with [TESTING] are only available for the owner\n\n";
        let mut help_message = String::from(header);
        help_message.push_str("<b>Available commands:</b>\n");
        for command in &self.0 {
            help_message.push_str(&format!(
                "<b>/{}</b>: {}\n",
                command.command, command.description
            ));
        }
        help_message
    }
}

async fn handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    // info!("Received message: {}", msg.text().unwrap_or(""));

    // Check if the message is a message or an image with a caption
    if msg.photo().is_some() && msg.caption().is_some() {
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
        let (command, args) = parse_command(msg.clone());
        let command = command.unwrap_or(String::new());
        let args = args.unwrap_or(String::new());
        let msg = msg.clone(); // Clone the message here
        match command.as_str() {
            "/mistral" | "/m" => {
                ollama(bot.clone(), msg, args.clone(), ModelType::Mistral).await?;
            }
            "/uncensored" | "/u" => {
                ollama(bot.clone(), msg, args.clone(), ModelType::MistralUncensored).await?;
            }
            "/caveman" => {
                ollama(bot.clone(), msg, args.clone(), ModelType::MistralCaveman).await?;
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
            "/online" => {
                perplexity(bot.clone(), msg, args.clone(), ModelType::Online).await?;
            }
            "/mixtral" => {
                perplexity(bot.clone(), msg, args.clone(), ModelType::Mixtral).await?;
            }
            "/img" => {
                img(bot.clone(), msg).await?;
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
