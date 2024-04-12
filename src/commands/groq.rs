use log::{error, info};
use teloxide::payloads::SendMessageSetters;
use teloxide::{requests::Requester, types::Message, Bot, RequestError};

use crate::structs::{GPT4Content, PerplexityRequest, PerplexityRequestMessage};
use crate::utils::ModelType;

pub async fn groq(
    bot: Bot,
    msg: Message,
    prompt: String,
    model: ModelType,
) -> Result<(), RequestError> {
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

    // groq is too fast for generating message 🔥
    // Send "typing indicator" to show that the bot is typing
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    let now = std::time::Instant::now();
    // Send the request to the Perplexity API
    let res = reqwest::Client::new()
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .bearer_auth(std::env::var("GROQ_KEY").unwrap_or_default())
        .json(&PerplexityRequest {
            // groq uses the same struct as perplexity
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
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(());
        }
    };

    // Parse the response
    let res = res.unwrap().json::<GPT4Content>().await;

    // Send the response
    match res {
        Ok(res) => {
            let content = res.text.unwrap_or_default();
            if content.is_empty() {
                bot.send_message(msg.chat.id, "No response from groq")
                    .reply_to_message_id(msg.id)
                    .await?;
                return Ok(());
            }

            info!(
                "Replying to message using groq. Generation took {}s",
                (elapsed * 10.0).round() / 10.0
            );
            bot.send_message(msg.chat.id, content)
                .reply_to_message_id(msg.id)
                .await?;
            Ok(())
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await?;
            Ok(())
        }
    }
}
