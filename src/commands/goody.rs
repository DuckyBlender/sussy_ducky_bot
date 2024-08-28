use std::env;

use log::{error, info};
use reqwest::{Client, Url};
use teloxide::payloads::SendMessageSetters;
use teloxide::types::ReplyParameters;
use teloxide::{requests::Requester, types::Message, Bot, RequestError};
use tokio::time::Instant;

pub async fn goody(bot: Bot, msg: Message, prompt: Option<String>) -> Result<(), RequestError> {
    info!("Starting goodyAI request function");

    let prompt = match prompt {
        Some(prompt) => prompt,
        None => {
            let bot_msg = bot
                .send_message(msg.chat.id, "No prompt provided")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;

            // Wait 5 seconds
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // Deleting the messages
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
            return Ok(());
        }
    };

    // Send "typing indicator" to show that the bot is typing
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

        let client = Client::new();
        let base_url = env::var("GOODY_URL").unwrap();
        let url = Url::parse_with_params(&base_url, &[("prompt", prompt)])
            .expect("Failed to parse URL");
    
        let now = Instant::now();
        let res = client.get(url).send().await;
        let elapsed = now.elapsed().as_secs_f32();

    match res {
        Ok(_) => {
            // info!("Request sent successfully");
        }
        Err(e) => {
            error!("Error sending request: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Parse the response
    let status = res.as_ref().unwrap().status();
    if !status.is_success() {
        error!("Error: {}: {:?}", status, res);
        bot.send_message(msg.chat.id, format!("Error: {status}"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }
    let res = res.unwrap().json::<serde_json::Value>().await;

    // Send the response
    match res {
        Ok(res) => {
            let content = res["response"].as_str().unwrap_or_default();
            info!(
                "Replying to message using goodyAI. Generation took {}s. Response: {}",
                (elapsed * 10.0).round() / 10.0,
                res
            );
            bot.send_message(msg.chat.id, content)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            Ok(())
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            Ok(())
        }
    }
}
