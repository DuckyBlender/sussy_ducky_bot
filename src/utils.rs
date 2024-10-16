use base64::{engine::general_purpose, Engine as _};
use teloxide::{
    net::Download,
    prelude::*,
    types::{PhotoSize, Sticker},
};
use tracing::{debug, error, warn};

pub enum Media {
    Photo(PhotoSize),
    Sticker(Sticker),
}

pub fn get_image_from_message(message: &Message) -> Option<Media> {
    if let Some(photo) = message.photo() {
        debug!("Photo found in the message");
        let photo = photo.last().unwrap();
        Some(Media::Photo(photo.clone()))
    } else if let Some(sticker) = message.sticker() {
        debug!("Sticker found in the message");
        Some(Media::Sticker(sticker.clone()))
    } else if let Some(photo) = message.reply_to_message().and_then(|m| m.photo()) {
        debug!("Photo found in the reply message");
        let photo = photo.last().unwrap();
        return Some(Media::Photo(photo.clone()));
    } else if let Some(sticker) = message.reply_to_message().and_then(|m| m.sticker()) {
        debug!("Sticker found in the reply message");
        return Some(Media::Sticker(sticker.clone()));
    } else {
        debug!("No photo or sticker found in the message or reply message");
        return None;
    }
}

pub async fn download_and_encode_image(bot: &Bot, media: &Media) -> anyhow::Result<String> {
    let mut buf: Vec<u8> = Vec::new();

    // Determine whether it's a photo or a sticker
    let file_id = match media {
        Media::Photo(photo) => &photo.file.id,
        Media::Sticker(sticker) => &sticker.file.id,
    };

    let file = bot.get_file(file_id).await?;
    bot.download_file(&file.path, &mut buf).await?;

    let base64_img = general_purpose::STANDARD.encode(&buf).to_string();

    Ok(base64_img)
}

pub fn remove_command(text: &str) -> String {
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
    // First, check the text in the current message
    let msg_text = message.text();

    let msg_text = if let Some(text) = msg_text {
        // If there's text in the message, remove the command and return the prompt
        remove_command(text)
    } else {
        // If no text in the current message, check for a reply message
        if let Some(reply) = message.reply_to_message() {
            // First, check if the reply message has text
            if let Some(text) = reply.text() {
                let msg_text = remove_command(text);
                if !msg_text.is_empty() {
                    return Some(msg_text); // Return the cleaned reply message text
                }
            }

            // If no text, check for a caption in the reply photo or sticker
            if let Some(caption) = reply.caption() {
                let msg_text = remove_command(caption);
                if !msg_text.is_empty() {
                    return Some(msg_text); // Return the cleaned caption
                }
            }

            // No valid text or caption found
            warn!("No text or caption found in the reply message");
            return None;
        }
        // No reply message found either
        warn!("No text found in the message & no reply message");
        return None;
    };

    if msg_text.is_empty() {
        warn!("No valid text or caption found");
        return None;
    }

    debug!("Message text: {}", msg_text);
    Some(msg_text.to_string())
}

pub fn parse_webhook(
    input: &lambda_http::http::Request<lambda_http::Body>,
) -> Result<Update, lambda_http::Error> {
    debug!("Parsing webhook");
    let body = input.body();
    let body_str = match body {
        lambda_http::Body::Text(text) => text,
        not => {
            error!("Expected Body::Text(...) got {:?}", not);
            return Err(lambda_http::Error::from("Expected Body::Text(...)"));
        }
    };
    let body_json: Update = serde_json::from_str(body_str)?;
    debug!("Successfully parsed webhook");
    Ok(body_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_command() {
        let text = "/command text";
        let result = remove_command(text);
        assert_eq!(result, "text");

        let text = "text";
        let result = remove_command(text);
        assert_eq!(result, "text");

        let text = "/command text with spaces";
        let result = remove_command(text);
        assert_eq!(result, "text with spaces");

        let text = "/command@bot text";
        let result = remove_command(text);
        assert_eq!(result, "text");
    }
}
