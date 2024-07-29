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

use crate::models::ModelType;

pub async fn fal(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
) -> Result<(), RequestError> {
    // Check if the user is from the owner
    // if msg.from().unwrap().id != UserId(std::env::var("OWNER_ID").unwrap().parse().unwrap()) {
    //     bot.send_message(
    //         msg.chat.id,
    //         "You are not the owner. Please mention @DuckyBlender if you want to use this command!",
    //     )
    //     .reply_to_message_id(msg.id)
    //     .await?;
    //     return Ok(());
    // }

    // Check if the model is supported by fal
    let supported_models = ModelType::return_fal();
    if !supported_models.contains(&model) {
        error!("Model {} is not supported by fal.ai", model.to_string());
        bot.send_message(
            msg.chat.id,
            format!("Model {} is not supported by fal.ai! Congrats you successfully broke the bot somehow!", model),
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    // If the model is AuraFlow, check if the user is the owner
    if model == ModelType::AuraFlow && msg.from().unwrap().id != UserId(std::env::var("OWNER_ID").unwrap().parse().unwrap()) {
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

    info!("Starting fal.ai function!");

    // Send generating... message
    let generating_message = bot
        .send_message(msg.chat.id, "Generating...")
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Send the response to the image gen
    let fal_res = reqwest::Client::new()
        .post(format!("https://fal.run/fal-ai/{}", model))
        .header(
            "Authorization",
            format!("Key {}", std::env::var("FAL_KEY").unwrap()),
        )
        .json(&json!({
            "prompt": prompt,
            // "expand_prompt": true,
            "enable_safety_checker": false,
        }))
        .send()
        .await;

    match fal_res {
        Ok(_) => {
            info!("Request to fal.ai recieved successfully! | Prompt: {} | Model: {}", prompt, model.to_string());

            // Parse the response
            let fal_res = fal_res.unwrap().json::<serde_json::Value>().await;

            if fal_res.is_err() {
                let err = fal_res.err().unwrap();
                error!("Error from fal.ai: {}", err);
                bot.edit_message_text(
                    generating_message.chat.id,
                    generating_message.id,
                    format!("Error: {}", err),
                )
                .await?;
                return Ok(());
            }

            let fal_res = fal_res.unwrap();
            let elapsed_fal;

            match model {
                ModelType::StableAudio => {
                    // Get the audio data
                    let audio_url = fal_res["audio_file"]["url"].as_str().unwrap_or_default().to_string();
                    let audio_filename = fal_res["audio_file"]["file_name"].as_str().unwrap_or_default().to_string();
                    elapsed_fal = None;

                    if audio_url.is_empty() {
                        error!("No audio URL received");
                        error!("fal_res: {fal_res}");
                        bot.edit_message_text(
                            generating_message.chat.id,
                            generating_message.id,
                            "Error: No audio URL received".to_string(),
                        )
                        .await?;
                        return Ok(());
                    }

                    info!("Audio URL: {audio_url}");
                    // Download the audio
                    let audio = reqwest::get(audio_url).await.unwrap().bytes().await.unwrap();

                    // Send the audio
                    let res = bot.send_audio(msg.chat.id, InputFile::memory(audio).file_name(audio_filename))
                        .caption(prompt.to_string())
                        .reply_to_message_id(msg.id)
                        .await;
                    match res {
                        Ok(_) => {
                            info!("Audio sent successfully!");
                        }
                        Err(e) => {
                            error!("Error sending audio: {}", e);
                            bot.edit_message_text(
                                generating_message.chat.id,
                                generating_message.id,
                                format!("Error: {}", e),
                            )
                            .await?;
                        }
                    }
                    bot.delete_message(generating_message.chat.id, generating_message.id)
                        .await?;
                }
                _ => {
                    // Get the image data
                    let img_url = fal_res["images"][0]["url"].as_str().unwrap_or_default();
                    elapsed_fal = Some(fal_res["timings"]["inference"].as_f64().unwrap_or_default());

                    if img_url.is_empty() {
                        error!("No image URL received");
                        error!("fal_res: {fal_res}");
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
                        .caption(prompt.to_string())
                        .reply_to_message_id(msg.id)
                        .await?;
                    bot.delete_message(generating_message.chat.id, generating_message.id)
                        .await?;
                }
            }

            // If elapsed is None, output "?" instead of the time
            let elapsed_fal = match elapsed_fal {
                Some(elapsed_fal) => ((elapsed_fal * 10.0).round() / 10.0).to_string(),
                None => "?".to_string(),
            };

            info!(
                "Replying to message using fal.ai | Generation took {}s. | Model: {}",
                elapsed_fal, model.to_string()
            );

            Ok(())
        }
        Err(e) => {
            error!("Error sending request to fal.ai: {}", e);
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
