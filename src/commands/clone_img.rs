use log::{error, info};
use reqwest::Url;
use serde_json::json;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::InputFile;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

use crate::utils::ModelType;

/// Clone image works like this:
/// 1. The user sends a reply to an image with the command `/clone`
/// 2. GPT-4-turbo generates a response based on the image
/// 3. The bot sends the response to DALLE 2 and generates an image
/// 4. The bot sends the image and the prompt back to the user
pub async fn clone_img(bot: Bot, msg: Message, model: ModelType) -> Result<(), RequestError> {
    // Check if the user is from the owner
    if msg.from().unwrap().id != UserId(std::env::var("OWNER_ID").unwrap().parse().unwrap()) {
        bot.send_message(msg.chat.id, "You are not the owner")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(());
    }

    // Check if there is an image or sticker attached in the reply
    let img_attachment = if let Some(reply) = msg.reply_to_message() {
        reply
            .photo()
            .map(|photo| photo.last().unwrap().file.id.clone())
            .or_else(|| reply.sticker().map(|sticker| &sticker.file.id).cloned())
    } else {
        let bot_msg = bot
            .send_message(msg.chat.id, "No image or sticker provided")
            .reply_to_message_id(msg.id)
            .await?;

        // Wait 5 seconds and delete the users and the bot's message
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Deleting the messages
        bot.delete_message(msg.chat.id, msg.id).await?;
        bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;

        return Ok(());
    };

    if img_attachment.is_none() {
        let bot_msg = bot
            .send_message(msg.chat.id, "No image or sticker provided")
            .reply_to_message_id(msg.id)
            .await?;

        // Wait 5 seconds and delete the users and the bot's message
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Deleting the messages
        bot.delete_message(msg.chat.id, msg.id).await?;
        bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
        return Ok(());
    }

    info!("Starting OpenAI clone image function with image!");

    // Send generating... message
    let generating_message = bot
        .send_message(msg.chat.id, "Summarizing image...")
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Get the image URL if it exists
    let img_url = if let Some(img_attachment) = img_attachment {
        let img_attachment = bot.get_file(&img_attachment).await?;
        let img_url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            std::env::var("TELOXIDE_TOKEN").unwrap(),
            img_attachment.path
        );
        img_url
    } else {
        let bot_msg = bot
            .send_message(msg.chat.id, "No image provided")
            .reply_to_message_id(msg.id)
            .await?;

        // Wait 5 seconds and delete the users and the bot's message
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Deleting the messages
        bot.delete_message(msg.chat.id, msg.id).await?;
        bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;

        return Ok(());
    };

    let json = json!(
            {
                "model": model.to_string(),
                "messages": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "text",
                                "text": "Describe the image in one sentence."
                            },
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": img_url
                                }
                            }
                        ]
                    }
                ],
                "max_tokens": 300,
            }
    );

    let now = std::time::Instant::now();

    let res = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(std::env::var("OPENAI_KEY").unwrap_or_default())
        .json(&json)
        .send()
        .await;
    let elapsed_summary = now.elapsed().as_secs_f32();

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
    let res = res.unwrap().json::<serde_json::Value>().await.unwrap();

    // info!("Vision response: {:#?}", res);

    // Get the content
    let summary = res["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default();

    // Edit the response
    bot.edit_message_text(
        generating_message.chat.id,
        generating_message.id,
        format!("Generating image: {summary}"),
    )
    .await?;

    // Send the response to dalle 3
    let now = std::time::Instant::now();
    let dalle3_res = reqwest::Client::new()
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(std::env::var("OPENAI_KEY").unwrap_or_default())
        .json(&json!({
            "model": "dall-e-3",
            "prompt": summary,
            "size": "1024x1024"
        }))
        .send()
        .await;
    let elapsed_dalle3 = now.elapsed().as_secs_f32();

    match dalle3_res {
        Ok(_) => {
            info!("Request to DALL-E 2 sent successfully");
            // info!("DALL-E 3 response: {:#?}", dalle3_res);

            // Parse the response
            let dalle3_res = dalle3_res.unwrap().json::<serde_json::Value>().await;

            if dalle3_res.is_err() {
                let err = dalle3_res.err().unwrap();
                error!("Error from dalle3: {}", err);
                bot.edit_message_text(
                    generating_message.chat.id,
                    generating_message.id,
                    format!("Error: {}", err),
                )
                .await?;
                return Ok(());
            }

            let dalle3_res = dalle3_res.unwrap();

            // Get the image data
            let img_url = dalle3_res["data"][0]["url"].as_str().unwrap_or_default();
            let revised_prompt = dalle3_res["data"][0]["revised_prompt"]
                .as_str()
                .unwrap_or_default();

            if img_url.is_empty() {
                bot.edit_message_text(
                    generating_message.chat.id,
                    generating_message.id,
                    "Error: No image URL received".to_string(),
                )
                .await?;
                return Ok(());
            }

            // info!("Image URL: {img_url}");

            // Send the image
            bot.delete_message(generating_message.chat.id, generating_message.id)
                .await?;
            bot.send_photo(msg.chat.id, InputFile::url(Url::parse(img_url).unwrap()))
                .caption(format!(
                    "Vision prompt: {summary}\nRevised prompt: {revised_prompt}"
                ))
                .reply_to_message_id(msg.id)
                .await?;

            info!(
                "Replying to message using OpenAI. Recognition took {}s. Generation took {}s. Total time: {}s",
                (elapsed_summary * 10.0).round() / 10.0,
                (elapsed_dalle3 * 10.0).round() / 10.0,
                ((elapsed_summary + elapsed_dalle3) * 10.0).round() / 10.0
            );

            Ok(())
        }
        Err(e) => {
            error!("Error sending request to DALL-E 2: {}", e);
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
