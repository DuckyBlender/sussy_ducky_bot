use base64::{engine::general_purpose, Engine as _};
use teloxide::{net::Download, prelude::*, types::PhotoSize};
use tracing::*;

pub fn get_image_from_message(message: &Message) -> Option<PhotoSize> {
    if let Some(photo) = message.photo() {
        debug!("Photo found in the message");
        let photo = photo.last().unwrap();
        Some(photo.clone())
    } else if let Some(photo) = message.reply_to_message().and_then(|m| m.photo()) {
        debug!("Photo found in the reply message");
        let photo = photo.last().unwrap();
        return Some(photo.clone());
    } else {
        debug!("No photo found in the message or reply message");
        return None;
    }
}

pub async fn download_and_encode_image(bot: &Bot, photo: &PhotoSize) -> anyhow::Result<String> {
    let mut buf: Vec<u8> = Vec::new();
    let file = bot.get_file(&photo.file.id).await?;
    bot.download_file(&file.path, &mut buf).await?;

    let base64_img = general_purpose::STANDARD.encode(&buf).to_string();

    Ok(base64_img)
}

pub async fn remove_command(text: &str) -> String {
    let mut words = text.split_whitespace();
    // if first starts with /
    let text = if let Some(word) = words.next() {
        if word.starts_with('/') {
            words.collect::<Vec<&str>>().join(" ")
        } else {
            text.to_string()
        }
    } else {
        text.to_string()
    };
    text.trim().to_string()
}

pub async fn find_prompt(message: &Message) -> Option<String> {
    // msg_text contains the text of the message or reply to a message with text
    let msg_text = message.text();

    if msg_text.is_none() {
        warn!("No text found in the message");
        return None;
    }

    let msg_text = msg_text.unwrap();
    let msg_text = remove_command(msg_text).await;
    let msg_text = if msg_text.is_empty() {
        // Find in reply message
        if let Some(reply) = message.reply_to_message() {
            if let Some(text) = reply.text() {
                text
            } else {
                warn!("No text found in the reply message");
                return None;
            }
        } else {
            warn!("No text found in the message & no reply message");
            return None;
        }
    } else {
        &msg_text
    };

    debug!("Message text: {}", msg_text);
    Some(msg_text.to_string())
}

pub async fn parse_webhook(
    input: lambda_http::http::Request<lambda_http::Body>,
) -> Result<Update, lambda_http::Error> {
    debug!("Parsing webhook");
    let body = input.body();
    let body_str = match body {
        lambda_http::Body::Text(text) => text,
        not => {
            error!("Expected Body::Text(...) got {:?}", not);
            panic!("expected Body::Text(...) got {:?}", not);
        }
    };
    let body_json: Update = serde_json::from_str(body_str)?;
    debug!("Successfully parsed webhook");
    Ok(body_json)
}

pub fn escape_markdown(input: &str) -> String {
    let chars_to_escape = [
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
    ];
    let mut result = String::new();
    for c in input.chars() {
        if chars_to_escape.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }

    result
}
