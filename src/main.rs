use teloxide::{prelude::*, utils::command::BotCommands, RequestError};
mod ollama;
use ollama::*;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    dotenv::dotenv().ok();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();
    // Set the commands
    bot.set_my_commands(Command::bot_commands())
        .await
        .expect("Failed to set commands");

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Display this text")]
    Help,
    #[command(description = "Generate using Mistral 7B")]
    Mistral(String),
    #[command(description = "Alias for /mistral", prefix = "m")]
    M(String),
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Mistral(prompt) | Command::M(prompt) => mistral(bot, msg, prompt).await?,
    };

    Ok(())
}

async fn mistral(bot: Bot, msg: Message, prompt: String) -> Result<Message, RequestError> {
    // If the prompt is empty, check if there is a reply
    let prompt = if prompt.is_empty() {
        if let Some(reply) = msg.reply_to_message() {
            reply.text().unwrap_or("").to_string()
        } else {
            bot.send_message(msg.chat.id, "No prompt provided").await?;
            return Ok(msg);
        }
    } else {
        prompt
    };

    // Check if prompt is nothing
    if prompt.is_empty() {
        bot.send_message(msg.chat.id, "No prompt provided").await?;
        return Ok(msg);
    }

    // Send typing action
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    // Send the request
    let res = reqwest::Client::new()
        .post("http://localhost:11434/api/generate")
        .json(&OllamaRequest {
            model: "mistral".to_string(),
            prompt,
            stream: false,
            images: None,
        })
        .send()
        .await;

    match res {
        Ok(_) => {}
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Error: {}", e))
                .await?;
            return Ok(msg);
        }
    };

    // Parse the response
    let res = res.unwrap().json::<OllamaResponse>().await;

    // Send the response
    match res {
        Ok(res) => {
            bot.send_message(msg.chat.id, res.response)
                .reply_to_message_id(msg.id)
                .await
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Error: {}", e))
                .reply_to_message_id(msg.id)
                .await
        }
    }
}
