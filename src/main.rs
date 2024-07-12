// #![warn(
//     clippy::all,
//     clippy::pedantic,
//     clippy::nursery,
// )]

use aws_config::BehaviorVersion;
use log::info;

use ollama_rs::Ollama;
use teloxide::{prelude::*, types::Me, utils::command::BotCommands, RequestError};
mod models;

use models::ModelType;

mod commands;
use commands::*;

use crate::models::setup_models;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "info,aws_config=warn,tracing=warn");
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
        .region("us-east-1")
        .load()
        .await;

    let bedrock = aws_sdk_bedrockruntime::Client::new(&config);

    let handler = dptree::entry()
        // .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_message().endpoint(handler));
    // Get the bot commands
    bot.set_my_commands(Commands::bot_commands()).await.unwrap();

    info!(
        "{} has started!",
        bot.get_me().send().await.unwrap().user.username.unwrap()
    );
    // Start the bots event loop
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![ollama, bedrock])
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
        description = "[🖥️] generate uncensored text using llama3-alpha-centauri-uncensored",
        alias = "u"
    )]
    Uncensored,
    #[command(description = "[🖥️] generate caveman-like text", alias = "cv")]
    Caveman,
    #[command(description = "[🖥️] generate text using the phi3 LLM", alias = "phi")]
    Phi3,
    #[command(description = "[🖥️] generate racist responses using custom fine-tuned LLM")]
    Racist,
    #[command(description = "[🖥️] generate uwu furry text")]
    Furry,
    #[command(
        description = "[🖥️] generate nonsense text using a highly quantized 300MB LLM",
        hide
    )]
    Lobotomy,
    #[command(description = "[🖥️] generate text using the tinyllama LLM")]
    TinyLlama,
    #[command(description = "show available commands")]
    Help,
    #[command(description = "check the bots latency")]
    Ping,
    #[command(description = "get an image of a cat for a given HTTP status code")]
    HttpCat,
    #[command(description = "get a random youtube video with no views")]
    NoViews,
    #[command(description = "[☁️] generate text using the mixtral model")]
    Mixtral,
    #[command(description = "[🖥️] generate text using the stablelm2 model")]
    StableLM2,
    #[command(
        description = "[☁️] nonsense api which responds with earlier user inputs: https://chatlgbtchatbot.neocities.org/",
        alias = "lgbt",
        hide
    )]
    ChatLGBT,
    #[command(description = "[☁️] generate text using the pplx-7b-online model", hide)]
    Online,
    #[command(description = "[☁️] multimodal GPT-4-vision", alias = "gpt", hide)]
    GPT4,
    #[command(description = "[☁️] DALLE 3", alias = "dalle", hide)]
    Dalle3,
    #[command(description = "[☁️] generate text using 70B LLAMA 3 model", aliases = ["llama", "l"])]
    LLAMA3,
    #[command(
        description = "[🖥️] respond to an image using the Moondream model",
        alias = "v"
    )]
    Moondream,
    #[command(description = "[🖥️] jsonify text", alias = "json")]
    Jsonify,
    #[command(
        description = "[☁️] generate text using Amazon Titan Lite",
        alias = "titanlite",
        hide
    )]
    AmazonTitanTextLite,
    #[command(
        description = "[☁️] generate text using Amazon Titan Express",
        alias = "titan",
        hide
    )]
    AmazonTitanText,
    #[command(description = "[☁️] generate image using amazon titan", alias = "img", hide)]
    AmazonTitanImage,
    // outpaint needs to be in a different file (or function in the same file) because it needs much more logic. 1. download image 2. add white borders around the image 3. continuation is obv
    // #[command(description = "outpaint an image using amazon titan", alias="outpaint", hide)]
    // AmazonTitanOutpaint,
    #[command(description = "[☁️] generate a variation of an image using amazon titan")]
    Clone,
    #[command(description = "[☁️] claude 3.5 multimodal model", alias = "claude", hide)]
    Claude3,
    #[command(description = "[🖥️] respond to an image using llava phi3", alias = "llava")]
    Vision,
    #[command(
        description = "[🖥️] custom bawialniaGPT model (nonsense model)",
        alias = "bawialnia"
    )]
    BawialniaGPT,
    #[command(description = "[🖥️] fine-tuned polish lobotomy", alias = "lobotomypl", hide)]
    Lobotomia,
    #[command(description = "[🖥️] generate multilingual text using 8B aya model")]
    Aya,
    #[command(description = "[🖥️] summarize text")]
    Summarize,
    #[command(
        description = "[🖥️] finish a story using the 656k tinystories model",
        alias = "story"
    )]
    TinyStories,
    #[command(
        description = "[🖥️] generate text using Gemma2 9B model",
        alias = "gemma",
    )]
    Gemma2,
    #[command(description = "[🖥️] generate text using the uncensored Smegmma LLM (GEMMA2 RP FINETUNE)")]
    Smegmma,
}

// Handler function for bot events
async fn handler(
    bot: Bot,
    msg: Message,
    me: Me,
    ollama_client: Ollama,
    bedrock_client: aws_sdk_bedrockruntime::Client,
) -> Result<(), RequestError> {
    if let Some(text) = msg.text() {
        let trimmed_text = text
            .split_once(' ')
            .map(|x| x.1)
            .unwrap_or_default()
            .trim()
            .to_string();
        match BotCommands::parse(text, me.username()) {
            Ok(Commands::Lobotomia) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::PolishLobotomy,
                    ollama_client,
                ));
            }
            Ok(Commands::Gemma2) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Gemma2,
                    ollama_client,
                ));
            }
            Ok(Commands::Smegmma) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Smegmma,
                    ollama_client,
                ));
            }
            Ok(Commands::TinyStories) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::TinyStories,
                    ollama_client,
                ));
            }
            Ok(Commands::Summarize) => {
                tokio::spawn(summarize(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ollama_client,
                ));
            }
            Ok(Commands::Aya) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Aya,
                    ollama_client,
                ));
            }

            Ok(Commands::BawialniaGPT) => {
                tokio::spawn(ollama(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::BawialniaGPT,
                    ollama_client,
                ));
            }
            Ok(Commands::Claude3) => {
                tokio::spawn(bedrock(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Claude3,
                    bedrock_client,
                ));
            }
            Ok(Commands::Moondream) => {
                tokio::spawn(vision(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Moondream,
                    ollama_client,
                ));
            }
            Ok(Commands::Vision) => {
                tokio::spawn(vision(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::Phi3Llava,
                    ollama_client,
                ));
            }
            Ok(Commands::AmazonTitanImage) => {
                tokio::spawn(bedrock(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::AmazonTitanImage,
                    bedrock_client,
                ));
            }
            Ok(Commands::AmazonTitanText) => {
                tokio::spawn(bedrock(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::AmazonTitanText,
                    bedrock_client,
                ));
            }
            Ok(Commands::AmazonTitanTextLite) => {
                tokio::spawn(bedrock(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::AmazonTitanTextLite,
                    bedrock_client,
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
            Ok(Commands::Clone) => {
                tokio::spawn(bedrock(
                    bot.clone(),
                    msg.clone(),
                    get_prompt(trimmed_text, &msg),
                    ModelType::AmazonTitanImageVariation,
                    bedrock_client,
                ));
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
