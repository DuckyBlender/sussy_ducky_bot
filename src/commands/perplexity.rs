use log::{error, info};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;

use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

use crate::structs::{PerplexityRequest, PerplexityRequestMessage};
use crate::utils::ModelType;

pub async fn perplexity(
    bot: Bot,
    msg: Message,
    prompt: String,
    model: ModelType,
) -> Result<Message, RequestError> {
    // Check if the user is from the owner
    if msg.from().unwrap().id != UserId(5337682436) {
        bot.send_message(msg.chat.id, "You are not the owner")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(msg);
    }
    info!("Starting perplexity request function");

    let prompt: String = if prompt.is_empty() {
        if let Some(reply) = msg.reply_to_message() {
            reply.text().unwrap_or_default().to_string()
        } else {
            bot.send_message(msg.chat.id, "No prompt provided")
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(msg);
        }
    } else {
        prompt.to_owned()
    };

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Send the request
    let res = reqwest::Client::new()
        .post("https://api.perplexity.ai/chat/completions")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .bearer_auth(std::env::var("PERPLEXITY_KEY").unwrap_or_default())
        .json(&PerplexityRequest {
            model: model.to_string(),
            messages: vec![
                PerplexityRequestMessage {
                    role: "system".to_string(),
                    content: "Be precise and concise.".to_string(),
                },
                PerplexityRequestMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
        })
        .send()
        .await;

    match res {
        Ok(_) => {
            // info!("Request sent successfully");
        }
        Err(e) => {
            error!("Error sending request: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(msg);
        }
    };

    // Parse the response
    let res = res.unwrap().json::<serde_json::Value>().await;

    // Send the response
    match res {
        Ok(res) => {
            let content = res["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default();
            bot.send_message(msg.chat.id, content)
                .reply_to_message_id(msg.id)
                .await
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await
        }
    }
}
