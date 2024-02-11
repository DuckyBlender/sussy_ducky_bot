use crate::structs::{
    BedrockImageGenerationConfig, BedrockRequest, BedrockResponse, BedrockTextToImageParams,
};
use aws_sdk_bedrockruntime::primitives::Blob;
use base64::prelude::*;
use log::info;
use serde_json::json;
use std::env;
use teloxide::payloads::{SendMessageSetters, SendPhotoSetters};
use teloxide::prelude::Requester;
use teloxide::requests::ResponseResult;
use teloxide::types::{ChatAction, UserId};
use teloxide::{types::Message, Bot};

pub async fn img(bot: Bot, msg: Message) -> ResponseResult<Message> {
    // Check if the user is from the owner
    if msg.from().unwrap().id != UserId(5337682436) {
        bot.send_message(msg.chat.id, "You are not the owner")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(msg);
    }
    info!("Starting img request function");
    let mut prompt = msg.text().unwrap_or_default().to_string();
    // If the prompt is empty, check if there is a reply
    let prompt: String = if prompt.is_empty() {
        if let Some(reply) = msg.reply_to_message() {
            reply
                .text()
                .unwrap_or_default()
                .to_string()
                .trim()
                .to_string()
        } else {
            bot.send_message(msg.chat.id, "No prompt provided")
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(msg);
        }
    } else {
        prompt = prompt.replace("/img", "").trim().to_string();
        prompt
    };

    // Check if prompt is nothing
    if prompt.is_empty() {
        bot.send_message(msg.chat.id, "No prompt provided")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(msg);
    }

    // Set the region to us-east-1
    // todo: make this better
    env::set_var("AWS_REGION", "us-east-1");
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_bedrockruntime::Client::new(&config);
    let model_id = "amazon.titan-image-generator-v1";

    let body = BedrockRequest {
        taskType: "TEXT_IMAGE".to_string(),
        textToImageParams: BedrockTextToImageParams {
            text: prompt.clone(),
            negativeText: None,
        },
        imageGenerationConfig: BedrockImageGenerationConfig {
            numberOfImages: 1,
            quality: "standard".to_string(),
            height: 512,
            width: 512,
            cfgScale: 8.0,
            seed: rand::random::<u32>(),
        },
    };

    // Send an indicator
    bot.send_chat_action(msg.chat.id, ChatAction::UploadPhoto)
        .await?;

    // Convert to string
    let body = json!(body);
    let body = Blob::new(body.to_string().as_bytes().to_vec());

    let response = client
        .invoke_model()
        .content_type("application/json")
        .model_id(model_id)
        .body(body)
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Error: {:?}", e))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(msg);
        }
    };

    let response = response.body;
    // convert to BedrockResponse
    let response = serde_json::from_slice::<BedrockResponse>(&response.into_inner()).unwrap();
    let image = response.images.first().unwrap();
    let image = BASE64_STANDARD.decode(image).unwrap();
    let image = teloxide::types::InputFile::memory(image);

    bot.send_photo(msg.chat.id, image)
        .reply_to_message_id(msg.id)
        .caption(prompt)
        .await
}
