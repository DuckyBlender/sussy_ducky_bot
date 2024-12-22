use log::{debug, info};
use ollama_rs::Ollama;
use sqlx::SqlitePool;
use teloxide::prelude::*;
use teloxide::types::ReplyParameters;
use teloxide::utils::command::BotCommands;
use strum::{EnumIter, IntoEnumIterator};

/// Define bot commands using `BotCommands` derive
#[derive(BotCommands, Clone, Debug, EnumIter)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Display this help text")]
    Help,
    #[command(description = "Ask llama 3.2 1b", alias = "l")]
    Llama,
    #[command(description = "Ask qwen 2.5 1.5b", alias = "q")]
    Qwen,
    #[command(description = "Ask racist-phi custom model", alias = "r")]
    Racist,
}

impl Command {
    /// Get the model ID associated with the command
    pub fn model_id(&self) -> Option<&str> {
        match self {
            Command::Help => None,
            Command::Llama => Some("llama3.2:1b"),
            Command::Qwen => Some("qwen2.5:1.5b"),
            Command::Racist => Some("duckyblender/racist-phi3"),
        }
    }

    pub fn available_models() -> Vec<String> {
        // get all from model_id
        let mut models = vec![];
        for cmd in Command::iter() {
            if let Some(model) = cmd.model_id() {
                models.push(model.to_string());
            }
        }
        models
    }

    // Todo: Define ratelimits here
}

/// Handle incoming commands
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    ollama: &Ollama,
    pool: &SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Received command: {:?}", cmd);

    match cmd {
        Command::Help => {
            let help_text = Command::descriptions().to_string();
            debug!("Sending help text to user.");
            bot.send_message(msg.chat.id, &help_text)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Help text sent to chat ID: {}", msg.chat.id);
        }
        _ => {
            if let Some(model) = cmd.model_id() {
                super::utils::handle_ollama(bot, msg, ollama, pool, model.to_string()).await?;
            }
        }
    }

    Ok(())
}
