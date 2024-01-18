use base64::prelude::*;
use log::info;
use serde_json::Value;
use teloxide::net::Download;
use teloxide::payloads::SendMessageSetters;

use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

use crate::OllamaRequest;

pub async fn llava(bot: Bot, msg: Message, mut prompt: String) -> Result<Message, RequestError> {
    info!("Starting llava function");

    // info!("Prompt: {}", prompt);

    if prompt.is_empty() {
        prompt = "What is in this image?".to_string();
    }

    let photo = match msg.photo() {
        Some(photos) => photos.last().unwrap(),
        None => {
            // Check if there is a reply
            if let Some(reply) = msg.reply_to_message() {
                if let Some(photo) = reply.photo() {
                    photo.last().unwrap()
                } else {
                    bot.send_message(msg.chat.id, "No image provided")
                        .reply_to_message_id(msg.id)
                        .await?;
                    return Ok(msg.clone());
                }
            } else {
                bot.send_message(msg.chat.id, "No image provided")
                    .reply_to_message_id(msg.id)
                    .await?;
                return Ok(msg.clone());
            }
        }
    };

    info!("Photo: {:?}", photo);

    let file_path = bot.get_file(photo.file.id.clone()).await?.path;
    let mut buf = Vec::new();
    bot.download_file(&file_path, &mut buf).await?;

    let base64_image = BASE64_STANDARD.encode(&buf);

    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    let request_body = &OllamaRequest {
        model: "llava".to_string(),
        prompt: prompt.to_string(),
        stream: false,
        images: Some(vec![base64_image]),
        raw: false,
    };

    // Save this request in json to the disk
    // let request_body_json = json!(request_body);
    // let request_body_json = serde_json::to_string_pretty(&request_body_json).unwrap();
    // use std::fs::File;
    // use std::io::Write;
    // let mut file = File::create("request.json").unwrap();
    // file.write_all(request_body_json.as_bytes()).unwrap();

    let client = reqwest::Client::new();
    let now = std::time::Instant::now();
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request_body)
        .send()
        .await;
    let elapsed = now.elapsed().as_secs_f32();

    match response {
        Ok(response) => {
            let res: Value = response.json().await?;
            // let text = response.text().await?;
            if let Some(response_text) = res["response"].as_str() {
                // info!("Response text: {}", response_text);
                let response_text = format!(
                    "{}\n\nGeneration time: {}s",
                    response_text,
                    (elapsed * 10.0).round() / 10.0
                );

                bot.send_message(msg.chat.id, response_text)
                    .reply_to_message_id(msg.id)
                    .await
            } else {
                bot.send_message(msg.chat.id, "Error: no response")
                    .reply_to_message_id(msg.id)
                    .await
            }
        }
        Err(e) => {
            info!("Error sending request: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await?;

            Err(e.into())
        }
    }
}
