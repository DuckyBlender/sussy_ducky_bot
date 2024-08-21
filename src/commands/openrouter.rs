use std::env;

use crate::ModelType;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::Engine as _;
use log::{error, info};
use serde_json::json;
use teloxide::net::Download;
use teloxide::payloads::SendMessageSetters;
use teloxide::types::ReplyParameters;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

pub async fn openrouter(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
) -> Result<(), RequestError> {
    // Check if the model is openrouter
    let openrouter_models = ModelType::return_openrouter();
    if !openrouter_models.contains(&model) {
        error!("Model {} is not supported by OpenRouter", model.to_string());
        bot.send_message(
            msg.chat.id,
            format!(
                "Model {} is not supported by OpenRouter! Congrats you successfully broke the bot somehow!",
                model
            ),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Check if the model is vision
    let vision_models = ModelType::return_vision();
    let mut vision = false;
    if vision_models.contains(&model) {
        vision = true;
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
        bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
        return Ok(());
    }

    info!(
        "Starting OpenRouter request function with prompt: {}{}",
        if prompt.is_empty() { "None" } else { &prompt },
        if attachment_id.is_some() {
            " and an image"
        } else {
            ""
        }
    );

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    let now = std::time::Instant::now();

    // Get the image URL if it exists
    let base64_img = if let Some(img_attachment) = attachment_id {
        let img_attachment = bot.get_file(&img_attachment).await?;
        let img_url = img_attachment.path;
        let mut buf: Vec<u8> = Vec::new();
        bot.download_file(&img_url, &mut buf).await?;
        let extension = img_url.split('.').last().unwrap();
        let extension = match extension {
            "jpg" => "jpeg",
            _ => extension,
        };
        Some(format!(
            "data:image/{};base64,{}",
            extension,
            BASE64.encode(buf)
        ))
    } else {
        None
    };

    let openrouter_key = match env::var("OPENROUTER_KEY") {
        Ok(key) => key,
        Err(_) => {
            bot.send_message(msg.chat.id, "Error: Unable to fetch OPENROUTER_KEY")
                .await?;
            return Ok(());
        }
    };

    let mut json = json!(
        {
            "model": model.to_string(),
            "messages": [
                {
                    "role": "system",
                    "content": [
                        {
                            "type": "text",
                            "text": "Be precise and concise. Don't be verbose."
                        }
                    ]
                },
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
            "max_tokens": 512,
        }
    );

    if let Some(base64_img) = base64_img {
        if vision {
            json["messages"][0]["content"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": base64_img
                    }
                }));
        }
    }

    // print out the final json
    // info!("Final JSON: {}", json.to_string());

    let res = reqwest::Client::new()
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(openrouter_key)
        .json(&json)
        .send()
        .await;
    let elapsed = now.elapsed().as_secs_f32();

    let status;
    let res = match res {
        Ok(res) => {
            info!("Request to OpenRouter recieved successfully");
            status = res.status();
            res
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
    let json = res.json::<serde_json::Value>().await.unwrap();

    // Check if non 200
    if status != 200 {
        let error = json["message"].as_str().unwrap_or_default();
        error!("Error from OpenRouter: {}", json.to_string());
        let bot_msg = bot
            .send_message(msg.chat.id, format!("Error: code {} - {}", status, error))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;

        // Wait 5 seconds and delete the bots message
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Deleting the messages
        bot.delete_message(bot_msg.chat.id, msg.id).await?;
        return Ok(());
    }

    // Send the response
    // info!("Parsed response: {:?}", json);

    // Check whats the finish_reason
    let finish_reason = json["finish_reason"].as_str().unwrap_or_default();
    if finish_reason == "SAFETY" {
        bot.send_message(msg.chat.id, "Error: Blocked by google")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default();
    info!(
        "Replying to message using OpenRouter ({}). Generation took {}s",
        model.to_string(),
        (elapsed * 10.0).round() / 10.0
    );
    let content = if content.is_empty() {
        "<no response>".to_string()
    } else {
        content.to_string()
    };
    bot.send_message(msg.chat.id, content)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    Ok(())
}
