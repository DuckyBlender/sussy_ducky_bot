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

// Struct to hold the bot commands
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(
        alias = "u",
        description = "Generate uncensored text using 8B dolphin-llama3 LLM. This should be the best local model here."
    )]
    Uncensored,
    #[command(
        alias = "cv",
        description = "Generate caveman-like text using 8B dolphin-llama3 LLM in caveman language [CUSTOM PROMPT MODEL]"
    )]
    Caveman,
    #[command(description = "Generate text using the 3.8B phi3 LLM", alias = "phi")]
    Phi3,
    #[command(
        description = "Generate racist responses using 8B dolphin-llama3 LLM [CUSTOM PROMPT MODEL]"
    )]
    Racist,
    #[command(
        description = "Generate uwu furry text using 8B dolphin-llama3 LLM [CUSTOM PROMPT MODEL]"
    )]
    Furry,
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
    #[command(description = "Generate text using the mixtral-8x7b-instruct model from groq.com")]
    Mixtral,
    #[command(description = "Generate text using the gemma-7b-it model from groq.com")]
    Gemma,
    #[command(description = "Generate code using the codegemma 7b model")]
    CodeGemma,
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
        description = "Generate text using the pplx-7b-online model from PerplexityAI [DEV ONLY]",
        hide
    )]
    Online,
    #[command(
        alias = "gpt",
        description = "Multimodel GPT-4-vision [DEV ONLY]",
        hide,
        hide_aliases
    )]
    GPT4,
    #[command(
        alias = "dalle",
        description = "DALLE 3 [DEV ONLY]",
        hide,
        hide_aliases
    )]
    Dalle3,
    #[command(
        description = "Clone an image using GPT-4-Turbo and DALLE 2 [DEV ONLY]",
        hide
    )]
    Clone,
    #[command(description = "Generate Polish text using the 7B-bielik model")]
    Bielik,
    #[command(description = "SDXL-Turbo locally on GTX950M [VERY EXPERIMENTAL]", aliases = ["sdxl", "img", "sd"])]
    SdxlTurbo,
    #[command(description = "Generate text using the 70B LLAMA 3 model from GroqCloud. This should be the best model here by FAR.", aliases = ["llama"])]
    LLAMA3,
    #[command(description = "Respond to an image using Phi-3 LLAVA model from ollama")]
    Vision,
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
            // Ok(Command::Vision) => {
            //     tokio::spawn(vision(
            //         bot.clone(),
            //         msg.clone(),
            //         ModelType::Vision,
            //         ollama_client,
            //     ));
            // }
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
            Ok(Command::Gemma) => {
                tokio::spawn(groq(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Gemma,
                ));
            }
            Ok(Command::CodeGemma) => {
                tokio::spawn(groq(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::CodeGemma,
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
