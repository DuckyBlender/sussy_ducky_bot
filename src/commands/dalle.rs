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

pub async fn dalle(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
) -> Result<(), RequestError> {
    // Check if the user is from the owner
    if msg.from().unwrap().id != UserId(std::env::var("OWNER_ID").unwrap().parse().unwrap()) {
        bot.send_message(
            msg.chat.id,
            "You are not the owner. Please mention @DuckyBlender if you want to use this command!",
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    // Check if prompt is empty
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

    info!("Starting OpenAI DALLE function!");

    // Send generating... message
    let generating_message = bot
        .send_message(msg.chat.id, "Generating image...")
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Send the response to dalle 3
    let now = std::time::Instant::now();
    let dalle3_res = reqwest::Client::new()
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(std::env::var("OPENAI_KEY").unwrap_or_default())
        .json(&json!({
            "model": model.to_string(),
            "prompt": prompt,
            "size": "1024x1024"
        }))
        .send()
        .await;
    let elapsed_dalle3 = now.elapsed().as_secs_f32();

    match dalle3_res {
        Ok(_) => {
            info!("Request to DALL-E 3 sent successfully");
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
            bot.send_photo(msg.chat.id, InputFile::url(Url::parse(img_url).unwrap()))
                .caption(format!(
                    "<b>User prompt</b>\n{prompt}\n\n<b>Revised prompt</b>\n{revised_prompt}"
                ))
                .reply_to_message_id(msg.id)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            bot.delete_message(generating_message.chat.id, generating_message.id)
                .await?;

            info!(
                "Replying to message using OpenAI DALLE 3. Generation took {}s.",
                (elapsed_dalle3 * 10.0).round() / 10.0
            );

            Ok(())
        }
        Err(e) => {
            error!("Error sending request to DALL-E 3: {}", e);
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
