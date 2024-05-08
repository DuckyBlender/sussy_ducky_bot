use crate::commands::ollama::INTERVAL_SEC;
use crate::models::ModelType;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::Engine as _;
use image::io::Reader as ImageReader;
use log::{error, info, warn};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::images::Image;
use ollama_rs::Ollama;
use std::io::Cursor;
use teloxide::payloads::SendMessageSetters;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};
use tokio_stream::StreamExt;

/// Vision works like this
/// 1. Downloads the image
/// 2. Sends it to ollama
/// 3. Response
pub async fn vision(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
    ollama_client: Ollama,
) -> Result<(), RequestError> {
    // Check if the model is one of ollama visions models
    let vision_models = ModelType::return_vision();
    if !vision_models.contains(&model) {
        bot.send_message(msg.chat.id, "Error: Invalid model")
            .reply_to_message_id(msg.id)
            .await?;
        error!("Invalid model: {model}. This should not happen!");
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

        // Wait 5 seconds and delete the users and the bots message
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

        // Wait 5 seconds and delete the users and the bots message
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Deleting the messages
        bot.delete_message(msg.chat.id, msg.id).await?;
        bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
        return Ok(());
    }

    info!("Starting vision command");

    // Send generating... message
    let generating_message = bot
        .send_message(msg.chat.id, "Responding to image...")
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

        // Wait 5 seconds and delete the users and the bots message
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Deleting the messages
        bot.delete_message(msg.chat.id, msg.id).await?;
        bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;

        return Ok(());
    };

    // Download the image
    let img_url = reqwest::Url::parse(&img_url).unwrap();
    let response = reqwest::get(img_url.as_str()).await.unwrap();
    // Check if the response is successful
    if !response.status().is_success() {
        error!("Failed to download image: {}", response.status());
        bot.edit_message_text(
            generating_message.chat.id,
            generating_message.id,
            format!("Failed to download image: {}", response.status()),
        )
        .await?;
        return Ok(());
    }
    let bytes = response.bytes().await.unwrap();
    // Check if the bytes are empty
    if bytes.is_empty() {
        error!("Failed to download image: bytes are empty");
        bot.edit_message_text(
            generating_message.chat.id,
            generating_message.id,
            "Failed to download image: bytes are empty".to_string(),
        )
        .await?;
        return Ok(());
    }

    // Load the image
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()
        .unwrap();

    // Convert the image to PNG
    let mut bytes: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();

    // Convert to base64
    let img = BASE64.encode(&bytes);

    // Send the stream request using ollama-rs
    let before_request = std::time::Instant::now();
    // Prompt is prompt, if it's None then it's "What's in this image?"
    let request = GenerationRequest::new(
        model.to_string(),
        prompt.unwrap_or("What's in this image?".to_string()),
    )
    .add_image(Image::from_base64(&img));
    let stream = ollama_client.generate_stream(request).await;

    match stream {
        Ok(_) => info!(
            "Stream request for model {} successful, incoming token responses..",
            model,
        ),
        Err(e) => {
            error!("Stream request failed: {}", e);
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                format!("Failed to generate response: {}", e),
            )
            .await?;
            return Ok(());
        }
    }

    let mut stream = stream.unwrap(); // safe unwrap

    // Create a repeating interval that yields every 5 seconds
    let mut now = std::time::Instant::now();

    // Create a string to hold the entire responseAppend [...] when the bot is still recieving
    let mut entire_response = String::new();
    let mut current_message_content = String::new();

    // Parse the response and edit the message every 5 seconds
    while let Some(Ok(res)) = stream.next().await {
        for ele in res {
            // Append the new response to the entire response
            entire_response.push_str(&ele.response);

            // Check if 5 seconds have passed since last edit
            if now.elapsed().as_secs() >= INTERVAL_SEC {
                // Check if the message is identical. Don't know if this is necessary but it's here for now
                if current_message_content == entire_response {
                    continue;
                }

                // Update the current string
                current_message_content.clone_from(&entire_response);

                // Edit the message
                bot.edit_message_text(
                    generating_message.chat.id,
                    generating_message.id,
                    current_message_content.clone() + " [...]",
                )
                .await?;

                // Send the typing indicator
                bot.send_chat_action(msg.chat.id, ChatAction::Typing)
                    .await?;

                // Reset the timer
                now = std::time::Instant::now();
            }

            // If the response is done, break the loop
            if ele.done {
                info!("Final response received");

                if entire_response.is_empty() {
                    warn!("No response received!");
                    entire_response = "<no response>".to_string();
                }

                // Edit the message one last time
                bot.edit_message_text(
                    generating_message.chat.id,
                    generating_message.id,
                    entire_response.clone(),
                )
                .await?;

                return Ok(());
            }
        }
    }

    let elapsed = before_request.elapsed().as_secs_f32();

    info!("Vision command completed in {:.2}s", elapsed);

    Ok(())
}
