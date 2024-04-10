use std::{collections::HashMap, sync::Arc};

use log::info;

use ollama_rs::Ollama;
use teloxide::{prelude::*, types::Me, utils::command::BotCommands, RequestError};

mod structs;

mod utils;
use tokio::sync::Mutex;
use utils::ModelType;

mod commands;
use commands::*;

use crate::utils::setup_models;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    info!("Starting command bot...");

    // If the --download flag is present in the command line arguments, download the models
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--download" {
        info!("Running with --download flag");
        setup_models().await;
    } else {
        info!("Running without --download flag")
    }

    let bot = Bot::from_env();
    let ollama = Ollama::default();
    let ollama_queue: HashMap<ChatId, Message> = HashMap::new();
    let ollama_queue = Arc::new(Mutex::new(ollama_queue));

    let handler = dptree::entry()
        // .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_message().endpoint(handler));
    // Start the bot's event loop
    info!(
        "{} has started!",
        bot.get_me().send().await.unwrap().user.username.unwrap()
    );
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![ollama, ollama_queue])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

// Struct to hold the bot commands
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(
        description = "Generate text using the 10.7B solar LLM. This is the best general purpouse model in this bot."
    )]
    Solar,
    #[command(
        alias = "m",
        description = "Generate text using 7B uncensored dolphin-mistral LLM."
    )]
    Mistral,
    #[command(
        alias = "cv",
        description = "Generate text using 7B dolphin-mistral LLM in caveman language [CUSTOM PROMPT MODEL]"
    )]
    Caveman,
    #[command(
        description = "Generate racist responses using 7B dolphin-mistral LLM [CUSTOM PROMPT MODEL]"
    )]
    Racist,
    #[command(description = "Geterate nonsense text using 300MB qwen:0.5b-chat-v1.5-q2_K LLM")]
    Lobotomy,
    #[command(description = "Generate text using 1.1B 8Q tinyllama-openorca LLM")]
    TinyLlama,
    #[command(description = "Show available commands")]
    Help,
    #[command(description = "Check the bot's latency")]
    Ping,
    #[command(description = "Get an image of a cat for a given HTTP status code")]
    HttpCat,
    #[command(description = "Get a random video with no views (or very few views)")]
    NoViews,
    // Online,
    #[command(description = "Generate text using the mixtral-8x7b-instruct model from groq.com")]
    Mixtral,
    #[command(description = "Generate text using the gemma-7b-it model from groq.com")]
    Gemma,
    #[command(
        alias = "stablelm",
        description = "Generate text using the stablelm2 1.6b model"
    )]
    StableLM2,
    #[command(
        alias = "lgbt",
        description = "Goofy ahh bot which responds with earlier user inputs: https://chatlgbtchatbot.neocities.org/"
    )]
    ChatLGBT,
    #[command(
        description = "Generate text using the pplx-7b-online model from PerplexityAI [DEV ONLY]"
    )]
    Online,
}

// Handler function for bot events
async fn handler(
    bot: Bot,
    msg: Message,
    me: Me,
    ollama_client: Ollama,
    ollama_queue: Arc<Mutex<HashMap<ChatId, Message>>>,
) -> Result<(), RequestError> {
    let msg_clone = msg.clone();
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Mistral) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::Mistral,
                    ollama_client,
                    ollama_queue,
                ));
            }
            Ok(Command::Caveman) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::MistralCaveman,
                    ollama_client,
                    ollama_queue,
                ));
            }
            Ok(Command::TinyLlama) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::TinyLlama,
                    ollama_client,
                    ollama_queue,
                ));
            }
            Ok(Command::Lobotomy) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::Lobotomy,
                    ollama_client,
                    ollama_queue,
                ));
            }
            Ok(Command::Help) => {
                tokio::spawn(help(bot.clone(), msg_clone));
            }
            Ok(Command::Ping) => {
                tokio::spawn(ping(bot.clone(), msg_clone));
            }
            Ok(Command::HttpCat) => {
                tokio::spawn(httpcat(bot.clone(), msg_clone, text.to_string()));
            }
            Ok(Command::ChatLGBT) => {
                tokio::spawn(chatlgbt(bot.clone(), msg_clone, text.to_string()));
            }
            Ok(Command::NoViews) => {
                tokio::spawn(noviews(bot.clone(), msg_clone));
            }
            Ok(Command::Solar) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::Solar,
                    ollama_client,
                    ollama_queue,
                ));
            }
            Ok(Command::Mixtral) => {
                tokio::spawn(groq(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::Mixtral,
                ));
            }
            Ok(Command::Gemma) => {
                tokio::spawn(groq(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::Gemma,
                    
                ));
            }
            Ok(Command::StableLM2) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::StableLM2,
                    ollama_client,
                    ollama_queue,
                ));
            }

            Ok(Command::Online) => {
                tokio::spawn(perplexity(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::Online,
                ));
            }
            Ok(Command::Racist) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg_clone,
                    text.to_string(),
                    ModelType::MistralRacist,
                    ollama_client,
                    ollama_queue,
                ));
            }
            _ => {}
        }
    }
    Ok(())
}
