use std::env;

use log::{error, info};
use teloxide::payloads::SendMessageSetters;
use teloxide::{requests::Requester, types::Message, Bot, RequestError};

use crate::commands::perplexity::{PerplexityRequest, PerplexityRequestMessage};
use crate::ModelType;

pub async fn groq(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
) -> Result<(), RequestError> {
    info!("Starting groq request function");

    // Check if the model is one of groqs models
    let groq_models = ModelType::return_groq();
    if !groq_models.contains(&model) {
        bot.send_message(msg.chat.id, "Error: Invalid model")
            .reply_to_message_id(msg.id)
            .await?;
        error!("Invalid model: {model}. This should not happen!");
        return Ok(());
    }

    let prompt = match prompt {
        Some(prompt) => prompt,
        None => {
            let bot_msg = bot
                .send_message(msg.chat.id, "No prompt provided")
                .reply_to_message_id(msg.id)
                .await?;

            // Wait 5 seconds
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // Deleting the messages
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
            return Ok(());
        }
    };

    let system_prompt = match model {
        ModelType::Rushify => "Rewrite the users text to look much more rushed, filled with grammatical errors, bad grammar and typos.",
        _ => "Be precise and concise.",
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
        .bearer_auth(env::var("GROQ_KEY").unwrap_or_default())
        .json(&PerplexityRequest {
            // this should be openai but perplexity works too
            model: model.to_string(),
            messages: vec![
                PerplexityRequestMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                PerplexityRequestMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
            temperature: 0.2,
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
    let res = res.unwrap().json::<serde_json::Value>().await;

    // Send the response
    match res {
        Ok(res) => {
            let content = res["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default();
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
