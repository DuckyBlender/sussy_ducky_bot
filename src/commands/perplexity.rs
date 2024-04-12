use log::{error, info};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

use crate::structs::{PerplexityRequest, PerplexityRequestMessage, PerplexityResponse};
use crate::utils::ModelType;

pub async fn perplexity(
    bot: Bot,
    msg: Message,
    prompt: String,
    model: ModelType,
) -> Result<(), RequestError> {
    // Check if the user is from the owner
    if msg.from().unwrap().id != UserId(std::env::var("OWNER_ID").unwrap().parse().unwrap()) {
        bot.send_message(msg.chat.id, "You are not the owner")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(());
    }
    info!("Starting perplexity request function");

    // Determine the prompt
    let prompt: String = if prompt.is_empty() {
        if let Some(reply) = msg.reply_to_message() {
            reply.text().unwrap_or_default().to_string()
        } else {
            bot.send_message(msg.chat.id, "No prompt provided")
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(());
        }
    } else {
        prompt.to_owned()
    };

    // Send generating... message
    let generating_message = bot
        .send_message(msg.chat.id, "Generating...")
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    let now = std::time::Instant::now();
    // Send the request to the Perplexity API
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
    let elapsed = now.elapsed().as_secs_f32();

    match res {
        Ok(_) => {
            // info!("Request sent successfully");
        }
        Err(e) => {
            error!("Error sending request: {}", e);
            bot.delete_message(generating_message.chat.id, generating_message.id)
                .await?;
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(());
        }
    };

    // Parse the response
    let res = res.unwrap().json::<PerplexityResponse>().await;

    // Send the response
    match res {
        Ok(res) => {
            let content = res.choices[0].message.content.as_str();
            info!(
                "Replying to message using perplexity. Generation took {}s",
                (elapsed * 10.0).round() / 10.0
            );
            bot.delete_message(generating_message.chat.id, generating_message.id)
                .await?;
            bot.send_message(msg.chat.id, content)
                .reply_to_message_id(msg.id)
                .await?;
            Ok(())
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            bot.delete_message(generating_message.chat.id, generating_message.id)
                .await?;
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await?;
            Ok(())
        }
    }
}
