// Import necessary dependencies
use log::{error, info};
use teloxide::{
    prelude::*,
    types::{BotCommand, True},
    RequestError,
};

// Import custom modules
mod structs;
use structs::*;

mod utils;
use utils::{parse_command, ModelType};

mod commands;
use commands::*;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    info!("Starting command bot...");

    // If the --download flag is present in the command line arguments, download the models
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--download" {
        info!("Downloading models...");
        // Run commands and make sure the models are downloaded
        let models = [
            ModelType::Mistral,
            ModelType::TinyLlama,
            ModelType::Lobotomy,
        ];

        // Run ollama pull model
        for model in models.iter() {
            let model = model.to_string();
            info!("Downloading model: {}", model);
            let output = std::process::Command::new("ollama")
                .arg("pull")
                .arg(&model)
                .output()
                .expect("Failed to download model");
            info!("Model download output: {:?}", output);
        }

        // Create the custom models
        let custom_models = [
            ModelType::MistralCaveman,
            ModelType::MistralRacist,
            ModelType::MistralGreentext,
        ];

        // Create the model eg: ollama create caveman-mistral -f ./custom_models/caveman/Modelfile
        for model in custom_models.iter() {
            let model = model.to_string();
            info!("Creating custom model: {}", model);
            let output = std::process::Command::new("ollama")
                .arg("create")
                .arg(&model)
                .arg("-f")
                .arg(format!("./custom_models/{}/Modelfile", model))
                .output()
                .expect("Failed to create custom model");
            info!("Custom model creation output: {:?}", output);
        }
    } else {
        info!("Running without --download flag")
    }

    let bot = Bot::from_env();

    let commands = Commands::new();

    // Set the bot commands
    if let Err(err) = commands.set_commands(&bot).await {
        error!("Failed to set commands: {}", err);
    }

    // Start the bot's event loop
    teloxide::repl(bot, handler).await
}

// Struct to hold the bot commands
struct Commands(Vec<BotCommand>);

impl Commands {
    #[rustfmt::skip]
    pub fn new() -> Self {
        Self(
            vec![
                BotCommand::new("solar", "Generate text using the 10.7B solar LLM. This should be the best model in this bot."),
                BotCommand::new("mistral", "Generate text using 7B dolphin-mistral LLM."),
                BotCommand::new("caveman", "Generate text using 7B dolphin-mistral LLM in caveman language [CUSTOM PROMPT MODEL]"),
                BotCommand::new("racist", "Generate racist responses using 7B dolphin-mistral LLM [CUSTOM PROMPT MODEL]"),
                BotCommand::new("lobotomy", "Geterate nonsense text using 300MB qwen:0.5b-chat-v1.5-q2 LLM"),
                BotCommand::new("tinyllama", "Generate text using 1.1B 8Q tinyllama-openorca LLM",),
                BotCommand::new("greentext", "Generate a 4chan greentext about a topic"),
                BotCommand::new("help", "Show available commands"),
                BotCommand::new("ping", "Check the bot's latency"),
                BotCommand::new("httpcat", "Get an image of a cat for a given HTTP status code",),
                BotCommand::new("noviews", "Get a random video with no views (or very few views)"),
                // BotCommand::new("online", "Generate text using the pplx-7b-online model from PerplexityAI [TESTING]"),
                // BotCommand::new("mixtral", "Generate text using the mixtral-8x7b-instruct model from PerplexityAI [TESTING]"),
            ]
        )
    }

    // Set the bot commands
    pub async fn set_commands(&self, bot: &Bot) -> Result<True, RequestError> {
        bot.set_my_commands(self.0.clone()).await
    }

    // Generate the help message with available commands
    pub fn help_message(&self) -> String {
        let header = "⚠️ SINCE THIS BOT IS SELF-HOSTED, IT CAN BE QUITE SLOW. ⚠️\nThis bot is open source and uses <code>ollama</code>! Check it out at https://github.com/DuckyBlender/sussy_ducky_bot\nOh and the bot works with replies too! (for example you can reply to a photo with /llava)\nCommands marked with [TESTING] are only available for the owner\n\n";
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
    if msg.text().is_some() {
        let (command, args) = parse_command(
            msg.clone(),
            bot.get_me().await.unwrap().username.clone().unwrap(),
        );
        let command = command.unwrap_or(String::new());
        let args = args.unwrap_or(String::new());

        handle_command(bot.clone(), msg.clone(), command, args).await?;
    }

    Ok(())
}

// Handler function for bot events
async fn handle_command(
    bot: Bot,
    msg: Message,
    command: String,
    args: String,
) -> Result<(), RequestError> {
    if msg.text().is_some() {
        // List of all HTTP status codes
        let status_codes = vec![
            "100", "103", "200", "201", "204", "206", "301", "302", "303", "304", "307", "308",
            "401", "403", "404", "406", "407", "409", "410", "412", "416", "418", "425", "451",
            "500", "501", "502", "503", "504",
        ];

        // Check if the command has any 3 digit numbers in it. If so, respond with a cat image
        for status_code in status_codes {
            if msg.text().unwrap().contains(&format!(" {} ", status_code))
                || msg
                    .text()
                    .unwrap()
                    .starts_with(&format!("{} ", status_code))
                || msg.text().unwrap().ends_with(&format!(" {}", status_code))
                || msg.text() == Some(status_code)
            {
                httpcat(bot.clone(), msg.clone(), status_code.to_string()).await?;
            }
        }

        // Handle different bot commands
        match command.as_str() {
            "/mistral" | "/m" => {
                tokio::spawn(ollama(bot.clone(), msg, args.clone(), ModelType::Mistral));
            }
            "/caveman" | "/cv" => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg,
                    args.clone(),
                    ModelType::MistralCaveman,
                ));
            }
            "/tinyllama" => {
                tokio::spawn(ollama(bot.clone(), msg, args.clone(), ModelType::TinyLlama));
            }
            "/lobotomy" => {
                tokio::spawn(ollama(bot.clone(), msg, args.clone(), ModelType::Lobotomy));
            }
            "/help" | "/h" => {
                tokio::spawn(help(bot.clone(), msg));
            }
            "/ping" => {
                tokio::spawn(ping(bot.clone(), msg));
            }
            "/httpcat" => {
                tokio::spawn(httpcat(bot.clone(), msg, args.clone()));
            }
            "/online" => {
                tokio::spawn(perplexity(
                    bot.clone(),
                    msg,
                    args.clone(),
                    ModelType::Online,
                ));
            }
            "/mixtral" => {
                tokio::spawn(perplexity(
                    bot.clone(),
                    msg,
                    args.clone(),
                    ModelType::Mixtral,
                ));
            }
            "/racist" => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg,
                    args.clone(),
                    ModelType::MistralRacist,
                ));
            }
            "/solar" => {
                tokio::spawn(ollama(bot.clone(), msg, args.clone(), ModelType::Solar));
            }
            "/greentext" => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg,
                    args.clone(),
                    ModelType::MistralGreentext,
                ));
            }
            "/noviews" => {
                tokio::spawn(noviews(bot.clone(), msg));
            }

            _ => {
                // If the command is not recognized, do nothing
            }
        }
    } else {
        // If the message is not text, do nothing
    }
    Ok(())
}
