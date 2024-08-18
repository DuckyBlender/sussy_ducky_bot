use std::env;

use crate::ModelType;
use log::{error, info};
use serde_json::json;
use teloxide::payloads::SendMessageSetters;
use teloxide::types::ReplyParameters;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

pub async fn openai(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
) -> Result<(), RequestError> {
    // Check if the model is openai
    let openai_models = ModelType::return_openai();
    if !openai_models.contains(&model) {
        error!("Model {} is not supported by OpenAI", model.to_string());
        bot.send_message(
            msg.chat.id,
            format!(
                "Model {} is not supported by OpenAI! Congrats you successfully broke the bot somehow!",
                model
            ),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Check if prompt is empty
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

    // Check if there is an image or sticker attached in the reply
    let attachment_id = if let Some(reply) = msg.reply_to_message() {
        if let Some(attachment) = reply.photo() {
            Some(attachment.last().unwrap().file.id.clone())
        } else {
            reply.sticker().map(|attachment| attachment.file.id.clone())
        }
    } else {
        None
    };

    if prompt.is_empty() && attachment_id.is_none() {
        let bot_msg = bot
            .send_message(msg.chat.id, "No prompt provided")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;

        // Wait 5 seconds and delete the users and the bots message
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Deleting the messages
        bot.delete_message(msg.chat.id, msg.id).await?;
        bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
        return Ok(());
    }

    info!(
        "Starting OpenAI request function with prompt: {}{}",
        if prompt.is_empty() { "None" } else { &prompt },
        if attachment_id.is_some() {
            " and an image"
        } else {
            ""
        }
    );

    // Send generating... message
    let generating_message = bot
        .send_message(msg.chat.id, "Generating...")
        .reply_parameters(ReplyParameters::new(msg.id))
        .disable_notification(true)
        .await?;

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    let now = std::time::Instant::now();

    // Get the image URL if it exists
    let img_url = if let Some(img_attachment) = attachment_id {
        let img_attachment = bot.get_file(&img_attachment).await?;
        let teloxide_token = match env::var("TELOXIDE_TOKEN") {
            Ok(token) => token,
            Err(_) => {
                bot.edit_message_text(
                    generating_message.chat.id,
                    generating_message.id,
                    "Error: Unable to fetch TELOXIDE_TOKEN",
                )
                .await?;
                return Ok(());
            }
        };
        let img_url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            teloxide_token, img_attachment.path
        );
        Some(img_url)
    } else {
        None
    };

    let mut json = json!(
            {
                "model": model.to_string(),
                "messages": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "text",
                                "text": prompt
                            },
                        ]
                    }
                ],
                "max_tokens": 300,
            }
    );

    if let Some(img_url) = img_url {
        json["messages"][0]["content"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "type": "image_url",
                "image_url": {
                    "url": img_url
                }
            }));
    }

    let openai_key = match env::var("OPENAI_KEY") {
        Ok(key) => key,
        Err(_) => {
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                "Error: Unable to fetch OPENAI_KEY",
            )
            .await?;
            return Ok(());
        }
    };

    let res = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(openai_key)
        .json(&json)
        .send()
        .await;
    let elapsed = now.elapsed().as_secs_f32();

    match res {
        Ok(_) => {
            info!("Request to OpenAI sent successfully");
        }
        Err(e) => {
            error!("Error sending request: {}", e);
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                format!("Error: {e}"),
            )
            .await?;
            return Ok(());
        }
    };

    // Parse the response
    let res = res.unwrap().json::<serde_json::Value>().await;

    let prompt_tokens = res.as_ref().unwrap()["usage"]["prompt_tokens"]
        .as_i64()
        .unwrap_or(0);
    let completion_tokens = res.as_ref().unwrap()["usage"]["completion_tokens"]
        .as_i64()
        .unwrap_or(0);

    // $0.150 / 1M input tokens
    // $0.600 / 1M output tokens
    let prompt_tokens_in_millions = prompt_tokens as f64 / 1_000_000.0;
    let completion_tokens_in_millions = completion_tokens as f64 / 1_000_000.0;

    let input_token_cost = prompt_tokens_in_millions * 0.150;
    let output_token_cost = completion_tokens_in_millions * 0.600;

    let total_price = input_token_cost + output_token_cost;

    info!("Total price for the request: ${}", total_price);

    // Send the response
    match res {
        Ok(res) => {
            let content = res["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default();
            info!(
                "Replying to message using OpenAIs. Generation took {}s",
                (elapsed * 10.0).round() / 10.0
            );
            bot.edit_message_text(generating_message.chat.id, generating_message.id, content)
                .await?;
            Ok(())
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                format!("Error: {e}"),
            )
            .await?;
            Ok(())
        }
    }
}
