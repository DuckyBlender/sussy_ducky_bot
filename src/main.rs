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

mod apis;
mod commands;
mod structs;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    info!("Starting command bot...");

    let bot = Bot::from_env();

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
    // description = "Bot commands. Most of the local models have Q4_K_M quantization. Some joke commands are hidden. Contact: @DuckyBlender"
)]

// - [ ] `/perplexity` - Llama 3 with internet access
// - [ ] `/claude` - Claude 3 Haiku multimodal
// - [ ] `/noviews` - Get a random youtube video with no views
// - [ ] `/summarize` - Summarize a youtube video

enum Commands {
    #[command(description = "llama 3 with internet access", alias = "online")]
    Perplexity,
    #[command(description = "claude 3 haiku multimodal")]
    Claude,
    #[command(description = "Get a random youtube video with no views")]
    NoViews,
    #[command(description = "Summarize a youtube video")]
    Summarize,
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
    if let Ok(command) = <Commands as BotCommands>::parse(msg.text().unwrap_or_default(), &bot_name)
    {
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
                bot.send_message(msg.chat.id, "Command not implemented")
                    .await?;
            }
        }
    }
    Ok(())
}
