use aws_config::BehaviorVersion;
use log::info;

use ollama_rs::Ollama;
use teloxide::{
    prelude::*,
    types::{Me, MessageId},
    utils::command::BotCommands,
    RequestError,
};
mod utils;

use tokio::sync::Mutex;
use utils::ModelType;

mod commands;
use commands::*;

use crate::utils::setup_models;

lazy_static::lazy_static! {
    pub static ref CURRENT_TASKS: Mutex<Vec<MessageId>> = Mutex::new(vec![]);
}

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
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region("us-west-2")
        .load()
        .await;

    let client = aws_sdk_bedrockruntime::Client::new(&config);

    let handler = dptree::entry()
        // .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_message().endpoint(handler));
    // Get the bot commands
    bot.set_my_commands(Commands::bot_commands()).await.unwrap();

    info!(
        "{} has started!",
        bot.get_me().send().await.unwrap().user.username.unwrap()
    );
    // Start the bot's event loop
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![ollama, client])
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
enum Commands {
    #[command(description = "generate uncensored text", alias = "u")]
    Uncensored,
    #[command(description = "generate caveman-like text", alias = "cv")]
    Caveman,
    #[command(description = "generate text using the phi3 LLM", alias = "phi")]
    Phi3,
    #[command(description = "generate racist responses using custom fine-tuned LLM")]
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
    #[command(
        description = "generate text using the pplx-7b-online model",
        alias = "lgbt",
        hide
    )]
    Online,
    #[command(description = "multimodal GPT-4-vision", alias = "gpt", hide)]
    GPT4,
    #[command(description = "DALLE 3", alias = "dalle", hide)]
    Dalle3,
    #[command(description = "clone an image using GPT-4 and DALLE 3", hide)]
    Clone,
    #[command(description = "generate Polish text using the bielik model")]
    Bielik,
    #[command(description = "SDXL-Turbo locally on GTX950M [BETA]", alias = "img")]
    SdxlTurbo,
    #[command(description = "generate text using 70B LLAMA 3 model", aliases = ["llama", "l"])]
    LLAMA3,
    #[command(
        description = "respond to an image using the Moondream model",
        alias = "v"
    )]
    Vision,
    #[command(description = "generate code using 3B stablecode")]
    StableCode,
    #[command(description = "jsonify text", alias = "json")]
    Jsonify,
    #[command(description = "generate text using Command R", hide)]
    CommandR,
    #[command(description = "generate text using Command R+", hide)]
    CommandRPlus,
    #[command(
        description = "generate text using Amazon Titan Lite",
        alias = "titanlite",
        hide
    )]
    AmazonTitanTextLite,
    #[command(
        description = "generate text using Amazon Titan Express",
        alias = "titan",
        hide
    )]
    AmazonTitanText,
}

// Handler function for bot events
async fn handler(
    bot: Bot,
    msg: Message,
    me: Me,
    ollama_client: Ollama,
    aws_client: aws_sdk_bedrockruntime::Client,
) -> Result<(), RequestError> {
    if let Some(text) = msg.text() {
        let trimmed_text = text
            .split_once(' ')
            .map(|x| x.1)
            .unwrap_or_default()
            .trim()
            .to_string();
        match BotCommands::parse(text, me.username()) {
            Ok(Commands::AmazonTitanText) => {
                tokio::spawn(bedrock(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::AmazonTitanText,
                    aws_client,
                ));
            }
            Ok(Commands::AmazonTitanTextLite) => {
                tokio::spawn(bedrock(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::AmazonTitanTextLite,
                    aws_client,
                ));
            }
            Ok(Commands::CommandR) => {
                tokio::spawn(bedrock(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::CommandR,
                    aws_client,
                ));
            }
            Ok(Commands::CommandRPlus) => {
                tokio::spawn(bedrock(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::CommandRPlus,
                    aws_client,
                ));
            }
            Ok(Commands::StableCode) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::StableCode,
                    ollama_client,
                ));
            }
            Ok(Commands::Jsonify) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Json,
                    ollama_client,
                ));
            }
            Ok(Commands::Uncensored) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Uncensored,
                    ollama_client,
                ));
            }
            // Ok(Commands::Vision) => {
            //     tokio::spawn(vision(
            //         bot.clone(),
            //         msg.clone(),
            //         get_prompt(trimmed_text, &msg),
            //         ModelType::Moondream,
            //         ollama_client,
            //     ));
            // }
            Ok(Commands::Phi3) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Phi3,
                    ollama_client,
                ));
            }
            Ok(Commands::GPT4) => {
                tokio::spawn(openai(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::GPT4,
                ));
            }
            Ok(Commands::Dalle3) => {
                tokio::spawn(dalle(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Dalle3,
                ));
            }
            Ok(Commands::Furry) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Furry,
                    ollama_client,
                ));
            }
            Ok(Commands::SdxlTurbo) => {
                tokio::spawn(comfyui(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::SDXLTurbo,
                ));
            }
            Ok(Commands::Clone) => {
                tokio::spawn(clone_img(bot.clone(), msg, ModelType::GPT4));
            }
            Ok(Commands::Caveman) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Caveman,
                    ollama_client,
                ));
            }
            Ok(Commands::TinyLlama) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::TinyLlama,
                    ollama_client,
                ));
            }
            Ok(Commands::Lobotomy) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Lobotomy,
                    ollama_client,
                ));
            }
            Ok(Commands::Help) => {
                tokio::spawn(help(bot.clone(), msg));
            }
            Ok(Commands::Ping) => {
                tokio::spawn(ping(bot.clone(), msg));
            }
            Ok(Commands::HttpCat) => {
                tokio::spawn(httpcat(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                ));
            }
            Ok(Commands::ChatLGBT) => {
                tokio::spawn(chatlgbt(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                ));
            }
            Ok(Commands::NoViews) => {
                tokio::spawn(noviews(bot.clone(), msg.clone()));
            }
            Ok(Commands::Mixtral) => {
                tokio::spawn(groq(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Mixtral,
                ));
            }
            Ok(Commands::StableLM2) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::StableLM2,
                    ollama_client,
                ));
            }
            Ok(Commands::LLAMA3) => {
                tokio::spawn(groq(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::LLAMA3,
                ));
            }
            Ok(Commands::Bielik) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Bielik,
                    ollama_client,
                ));
            }

            Ok(Commands::Online) => {
                tokio::spawn(perplexity(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Online,
                ));
            }
            Ok(Commands::Racist) => {
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
