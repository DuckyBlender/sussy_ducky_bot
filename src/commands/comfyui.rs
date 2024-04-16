use log::{error, info};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::InputFile;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

use crate::utils::{process_image_generation, ModelType};

pub async fn comfyui(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
) -> Result<(), RequestError> {
    info!("Starting OpenAI DALLE function!");

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

    let models = ModelType::return_comfyui();
    if !models.contains(&model) {
        bot.send_message(
            msg.chat.id,
            "Invalid model. Please use one of the following models: comfyui",
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

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
    let imgs = process_image_generation(&prompt, &model).await;
    let elapsed = now.elapsed().as_secs_f32();

    match imgs {
        Ok(imgs) => {
            let img = imgs.get("image").unwrap();
            let img = InputFile::memory(img.to_vec());
            bot.send_photo(msg.chat.id, img)
                .caption(format!("{prompt} | Generated image in {:.2}s", elapsed))
                .reply_to_message_id(msg.id)
                .await?;
        }
        Err(e) => {
            error!("Error generating image: {}", e);
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                format!("Failed to generate image: {}", e),
            )
            .await?;
        }
    }

    Ok(())
}
