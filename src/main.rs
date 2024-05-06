#![warn(clippy::pedantic, clippy::nursery)]

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
mod apis;

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

#[derive(BotCommands, Clone, Debug)]
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
    #[command(description = "generate image using amazon titan", alias = "img", hide)]
    AmazonTitanImage,
    // outpaint needs to be in a different file (or function in the same file) because it needs much more logic. 1. download image 2. add white borders around the image 3. continuation is obv
    // #[command(description = "outpaint an image using amazon titan", alias="outpaint", hide)]
    // AmazonTitanOutpaint,
    #[command(description = "generate a variation of an image using amazon titan")]
    Clone,
    #[command(description = "claude 3 sonnet multimodal", hide)]
    Claude3,
}

use crate::commands::chatlgbt::chatlgbt;

// Handler function for bot events
async fn handler(
    bot: Bot,
    msg: Message,
    me: Me,
    ollama_client: Ollama,
    aws_client: aws_sdk_bedrockruntime::Client,
) -> Result<(), RequestError> {
    // Check if the message is a command
    let bot_name = me.user.username.unwrap();
    if let Ok(command) = <Commands as BotCommands>::parse(msg.text().unwrap_or_default(), &bot_name) {

        info!("Received command: {:?}", command);
        match command {
            Commands::ChatLGBT => {
                chatlgbt(bot, msg).await?;
            }
            Commands::NoViews => {
                commands::noviews::noviews(bot, msg).await?;
            }
            Commands::Help => {
                commands::help::help(bot, msg).await?;
            }
            _ => {
                bot.send_message(msg.chat.id, "Command not implemented").await?;
            }
        }
    }
    Ok(())
}