#![allow(clippy::upper_case_acronyms)]

use std::env;

use log::info;

use ollama_rs::Ollama;
use teloxide::{prelude::*, utils::command::BotCommands, RequestError};
mod models;

use models::ModelType;

mod commands;
use commands::*;
use utils::get_prompt;
mod utils;

use crate::models::setup_models;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env::set_var("RUST_LOG", "info,aws_config=warn,tracing=warn");
    pretty_env_logger::init();
    info!("Starting command bot...");

    // If the --download flag is present in the command line arguments, download the models
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--download" {
        info!("Running with --download flag");
        setup_models().await;
    } else {
        info!("Running without --download flag")
    }

    let bot = Bot::from_env();
    let ollama = Ollama::default();

    let handler = dptree::entry().branch(Update::filter_message().endpoint(handler));
    // Get the bot commands
    bot.set_my_commands(Commands::bot_commands()).await.unwrap();

    info!(
        "{} has started!",
        bot.get_me().send().await.unwrap().user.username.unwrap()
    );

    // Start the bots event loop
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![ollama])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "Bot commands. Local models use q4_K_M quantization unless specified otherwise. Some joke commands are hidden. [🖥️] = local (GTX 1660S), [☁️] = cloud model"
)]
enum Commands {
    #[command(
        description = "[🖥️] generate uncensored text using fine-tuned llama3",
        alias = "u"
    )]
    Uncensored,
    #[command(description = "[🖥️] generate caveman-like text", alias = "cv")]
    Caveman,
    #[command(description = "[🖥️] generate text using the phi3 LLM", alias = "phi")]
    Phi3,
    #[command(description = "[🖥️] generate racist responses using custom fine-tuned LLM")]
    Racist,
    #[command(
        description = "[🖥️] generate nonsense text using a highly quantized 300MB LLM",
        hide
    )]
    Lobotomy,
    #[command(description = "show available commands")]
    Help,
    #[command(description = "check the bots latency")]
    Ping,
    #[command(description = "get an image of a cat for a given HTTP status code")]
    HttpCat,
    #[command(description = "get a random youtube video with no views")]
    NoViews,
    #[command(description = "[☁️] generate text using perplexity.ai", hide)]
    Online,
    #[command(description = "[☁️] multimodal GPT4o mini", aliases = ["gpt4", "gpt4o", "gpt4omini"])]
    GPT,
    #[command(description = "[☁️] generate text using 70B LLAMA 3 model", aliases = ["llama", "l"])]
    LLAMA3,
    #[command(description = "[🖥️] jsonify text", alias = "json")]
    Jsonify,
    #[command(
        description = "[🖥️] fine-tuned polish lobotomy",
        alias = "lobotomypl",
        hide
    )]
    Lobotomia,
    #[command(description = "[🖥️] summarize text")]
    Summarize,
    #[command(
        description = "[🖥️] generate text using Gemma2 9B model",
        alias = "gemma"
    )]
    Gemma2,
    #[command(
        description = "[🖥️] generate multilingual text using the 9B GLM4 LLM",
        alias = "glm"
    )]
    GLM4,
    #[command(description = "[☁️] generate images using the sdxl-turbo for free", aliases = ["sdxl"])]
    Img,
    #[command(
        description = "[☁️] generate 30-second audio using stable audio open",
        alias = "audio"
    )]
    StableAudio,
    #[command(
        description = "[☁️] rushify text using llama 3.1 8B model",
        alias = "rush"
    )]
    Rushify,
    #[command(description = "[☁️] generate high-quality images using the FLUX.1[schnell] model")]
    Flux,
}

// Handler function for bot events
async fn handler(bot: Bot, msg: Message, ollama_client: Ollama) -> Result<(), RequestError> {
    // Handle commands
    if let Some(text) = &msg.text() {
        if let Ok(command) = Commands::parse(text, bot.get_me().await.unwrap().username()) {
            return handle_command(bot.clone(), msg.clone(), command, ollama_client).await;
        }
    }

    Ok(())
}

// Handle the command
async fn handle_command(
    bot: Bot,
    msg: Message,
    command: Commands,
    ollama_client: Ollama,
) -> Result<(), RequestError> {
    let trimmed_text = msg
        .text()
        .unwrap_or_default()
        .split_once(' ')
        .map(|x| x.1)
        .unwrap_or_default()
        .trim()
        .to_string();

    match command {
        Commands::Help => {
            tokio::spawn(help(bot.clone(), msg));
        }
        Commands::Flux => {
            tokio::spawn(fal(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::FluxShnell,
            ));
        }
        Commands::Rushify => {
            tokio::spawn(groq(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::Rushify,
            ));
        }
        Commands::Img => {
            tokio::spawn(fal(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                // ModelType::SDXLTurbo,
                ModelType::SDXL,
            ));
        }
        Commands::StableAudio => {
            tokio::spawn(fal(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::StableAudio,
            ));
        }
        Commands::GLM4 => {
            tokio::spawn(ollama(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::GLM4,
                ollama_client,
            ));
        }

        Commands::Lobotomia => {
            tokio::spawn(ollama(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::PolishLobotomy,
                ollama_client,
            ));
        }
        Commands::Gemma2 => {
            tokio::spawn(ollama(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::Gemma2,
                ollama_client,
            ));
        }
        Commands::Summarize => {
            tokio::spawn(summarize(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ollama_client,
            ));
        }
        Commands::Jsonify => {
            tokio::spawn(ollama(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::Json,
                ollama_client,
            ));
        }
        Commands::Uncensored => {
            tokio::spawn(ollama(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::Uncensored,
                ollama_client,
            ));
        }
        Commands::Phi3 => {
            tokio::spawn(ollama(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::Phi3,
                ollama_client,
            ));
        }
        Commands::GPT => {
            tokio::spawn(openai(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::GPT4oMini,
            ));
        }
        Commands::Caveman => {
            tokio::spawn(ollama(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::Caveman,
                ollama_client,
            ));
        }
        Commands::Lobotomy => {
            tokio::spawn(ollama(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::Lobotomy,
                ollama_client,
            ));
        }
        Commands::Ping => {
            tokio::spawn(ping(bot.clone(), msg));
        }
        Commands::HttpCat => {
            tokio::spawn(httpcat(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
            ));
        }
        Commands::NoViews => {
            tokio::spawn(noviews(bot.clone(), msg.clone()));
        }
        Commands::LLAMA3 => {
            tokio::spawn(groq(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::LLAMA3,
            ));
        }

        Commands::Online => {
            tokio::spawn(perplexity(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::Online,
            ));
        }
        Commands::Racist => {
            tokio::spawn(ollama(
                bot.clone(),
                msg.clone(),
                get_prompt(trimmed_text, &msg),
                ModelType::Racist,
                ollama_client,
            ));
        }
    }

    Ok(())
}