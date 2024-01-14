use std::{sync::Arc, time::Duration};

use log::info;
use teloxide::{prelude::*, utils::command::BotCommands, RequestError};
mod ollama;
use ollama::*;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

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
    #[command(description = "Generate using Mistral 7B")]
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

    if prompt.is_empty() {
        bot.send_message(msg.chat.id, "No prompt provided").await?;
        return Ok(msg);
    }

    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    // Send the first message
    let first_text = "Generating...";
    let first_message = Some(
        bot.send_message(msg.chat.id, first_text)
            .reply_to_message_id(msg.id)
            .await?,
    );

    let res = reqwest::Client::new()
        .post("http://localhost:11434/api/generate")
        .json(&OllamaRequest {
            model: "mistral".to_string(),
            prompt,
            stream: true,
            images: None,
        })
        .send()
        .await;

    let mut res = match res {
        Ok(res) => res,
        Err(e) => {
            bot.edit_message_text(msg.chat.id, first_message.unwrap().id, e.to_string())
                .await
                .unwrap();
            return Ok(msg);
        }
    };

    // Variable for keeping track of the whole message
    let message = Arc::new(Mutex::new(String::new()));

    // Spawn a thread to update the message every 5 seconds
    let first_message_clone = first_message.clone();
    let message_clone = message.clone();
    let bot_clone: Bot = bot.clone();

    info!("Starting thread");
    let thread = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let message = message_clone.lock().await;
            let first_message = first_message_clone.clone();
            if let Some(first_message) = first_message {
                bot_clone
                    .edit_message_text(msg.chat.id, first_message.id, message.clone())
                    .await
                    .unwrap();
            }
        }
    });

    info!("Starting stream");
    // Loop through the stream
    while let Some(chunk) = res.chunk().await.unwrap() {
        // Convert the chunk to a string
        let chunk = String::from_utf8(chunk.to_vec()).unwrap();
        // Convert to json
        // {"model":"mistral","created_at":"2024-01-14T11:32:48.111912999Z","response":" today","done":false}
        let chunk: serde_json::Value = serde_json::from_str(&chunk).unwrap();
        let text = chunk["response"].as_str().unwrap();

        // Add the chunk to the message
        message.lock().await.push_str(text);

        // TODO: If the message is too long, send it and start a new one
    }

    // Stop the thread
    info!("Stopping thread");
    thread.abort();

    info!("Submitting final edit");
    // Update the message with the final result
    let message = message.lock().await;
    let message = message.clone();
    let first_message = first_message.clone();
    if let Some(first_message) = first_message {
        bot.edit_message_text(msg.chat.id, first_message.id, message)
            .await
            .unwrap();
    }

    Ok(msg)
}
