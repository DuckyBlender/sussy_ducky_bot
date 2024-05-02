use log::info;

use ollama_rs::Ollama;
use teloxide::{prelude::*, types::Me, utils::command::BotCommands, RequestError};

mod structs;

mod utils;

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

    let handler = dptree::entry()
        // .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_message().endpoint(handler));
    // Start the bot's event loop
    info!(
        "{} has started!",
        bot.get_me().send().await.unwrap().user.username.unwrap()
    );
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
    description = "Bot commands. Most of the local models have Q4_K_M quantization. Some joke commands are hidden. Contact: @DuckyBlender"
)]
enum Command {
    #[command(alias = "u", description = "generate uncensored text")]
    Uncensored,
    #[command(alias = "cv", description = "generate caveman-like text")]
    Caveman,
    #[command(description = "generate text using the phi3 LLM")]
    Phi3,
    #[command(description = "generate racist responses")]
    Racist,
    #[command(description = "generate uwu furry text")]
    Furry,
    #[command(description = "generate nonsense text")]
    Lobotomy,
    #[command(description = "generate text using the tinyllama LLM")]
    TinyLlama,
    #[command(description = "show available commands")]
    Help,
    #[command(description = "check the bot's latency")]
    Ping,
    #[command(description = "get an image of a cat for a given HTTP status code")]
    HttpCat,
    #[command(description = "get a random youtube video with no views")]
    NoViews,
    #[command(description = "generate text using the mixtral model")]
    Mixtral,
    #[command(description = "generate text using the stablelm2 model")]
    StableLM2,
    #[command(
        description = "nonsense api which responds with earlier user inputs: https://chatlgbtchatbot.neocities.org/",
        hide
    )]
    ChatLGBT,
    #[command(description = "generate text using the pplx-7b-online model", hide)]
    Online,
    #[command(description = "multimodel GPT-4-vision", hide)]
    GPT4,
    #[command(description = "DALLE 3", hide)]
    Dalle3,
    #[command(description = "clone an image using GPT-4 and DALLE 3", hide)]
    Clone,
    #[command(description = "generate Polish text using the bielik model")]
    Bielik,
    #[command(description = "SDXL-Turbo locally on GTX950M [BETA]")]
    SdxlTurbo,
    #[command(description = "generate text using 70B LLAMA 3 model")]
    LLAMA3,
    #[command(description = "respond to an image using the Moondream model")]
    Vision,
    #[command(description = "brainrotify text")]
    Brainrot,
    #[command(description = "generate code using 3B stablecode")]
    StableCode,
}

// Handler function for bot events
async fn handler(
    bot: Bot,
    msg: Message,
    me: Me,
    ollama_client: Ollama,
) -> Result<(), RequestError> {
    if let Some(text) = msg.text() {
        let trimmed_text = text
            .split_once(' ')
            .map(|x| x.1)
            .unwrap_or_default()
            .trim()
            .to_string();
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Uncensored) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Uncensored,
                    ollama_client,
                ));
            }
            Ok(Command::Brainrot) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Brainrot,
                    ollama_client,
                ));
            }
            Ok(Command::Vision) => {
                tokio::spawn(vision(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Moondream,
                    ollama_client,
                ));
            }
            Ok(Command::Phi3) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Phi3,
                    ollama_client,
                ));
            }
            Ok(Command::GPT4) => {
                tokio::spawn(openai(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::GPT4,
                ));
            }
            Ok(Command::Dalle3) => {
                tokio::spawn(dalle(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Dalle3,
                ));
            }
            Ok(Command::Furry) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Furry,
                    ollama_client,
                ));
            }
            Ok(Command::SdxlTurbo) => {
                tokio::spawn(comfyui(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::SDXLTurbo,
                ));
            }
            Ok(Command::Clone) => {
                tokio::spawn(clone_img(bot.clone(), msg, ModelType::GPT4));
            }
            Ok(Command::Caveman) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Caveman,
                    ollama_client,
                ));
            }
            Ok(Command::TinyLlama) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::TinyLlama,
                    ollama_client,
                ));
            }
            Ok(Command::Lobotomy) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Lobotomy,
                    ollama_client,
                ));
            }
            Ok(Command::Help) => {
                tokio::spawn(help(bot.clone(), msg));
            }
            Ok(Command::Ping) => {
                tokio::spawn(ping(bot.clone(), msg));
            }
            Ok(Command::HttpCat) => {
                tokio::spawn(httpcat(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                ));
            }
            Ok(Command::ChatLGBT) => {
                tokio::spawn(chatlgbt(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                ));
            }
            Ok(Command::NoViews) => {
                tokio::spawn(noviews(bot.clone(), msg.clone()));
            }
            Ok(Command::Mixtral) => {
                tokio::spawn(groq(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Mixtral,
                ));
            }
            Ok(Command::StableLM2) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::StableLM2,
                    ollama_client,
                ));
            }
            Ok(Command::LLAMA3) => {
                tokio::spawn(groq(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::LLAMA3,
                ));
            }
            Ok(Command::Bielik) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Bielik,
                    ollama_client,
                ));
            }

            Ok(Command::Online) => {
                tokio::spawn(perplexity(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Online,
                ));
            }
            Ok(Command::Racist) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Racist,
                    ollama_client,
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// If the prompt is empty, check the reply
fn get_prompt(prompt: String, msg: &Message) -> Option<String> {
    if prompt.is_empty() {
        msg.reply_to_message()
            .map(|reply| reply.text().unwrap_or_default().to_string())
    } else {
        Some(prompt)
    }
}
