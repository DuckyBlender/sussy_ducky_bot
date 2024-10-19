use base64::{engine::general_purpose, Engine as _};
use reqwest::Url;
use teloxide::{
    net::Download,
    prelude::*,
    types::{MessageId, ParseMode, PhotoSize, ReplyParameters, Sticker},
};
use tracing::{debug, error, info, warn};

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
    // Check if the message itself has text
    if let Some(msg_text) = message.text() {
        let cleaned_text = remove_command(msg_text);
        if !cleaned_text.is_empty() {
            return Some(cleaned_text);
        }
    }

    // Check if the message is a reply to another message
    if let Some(reply) = message.reply_to_message() {
        // First, check if the reply message has text
        if let Some(reply_text) = reply.text() {
            let cleaned_text = remove_command(reply_text);
            if !cleaned_text.is_empty() {
                return Some(cleaned_text);
            }
        }

        // If no text, check for a caption in the reply
        if let Some(reply_caption) = reply.caption() {
            let cleaned_caption = remove_command(reply_caption);
            if !cleaned_caption.is_empty() {
                return Some(cleaned_caption);
            }
        }
    }

    // If no valid text or caption found, log the warning and return None
    warn!("No valid text or caption found in the message or reply");
    None
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

pub fn split_string(input: &str, max_length: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current_chunk = String::new();
    let mut current_length = 0;

    for word in input.split_whitespace() {
        if current_length + word.len() + 1 > max_length && !current_chunk.is_empty() {
            result.push(current_chunk);
            current_chunk = String::new();
            current_length = 0;
        }

        if current_length > 0 {
            current_chunk.push(' ');
            current_length += 1;
        }

        current_chunk.push_str(word);
        current_length += word.len();
    }

    if !current_chunk.is_empty() {
        result.push(current_chunk);
    }

    result
}

pub fn escape_markdown(text: &str) -> String {
    let mut escaped_text = String::new();
    for c in text.chars() {
        match c {
            '[' | ']' | '(' | ')' | '~' | '>' | '#' | '+' | '-' | '=' | '|' | '{' | '}' | '.'
            | '!' => {
                escaped_text.push('\\');
                escaped_text.push(c);
            }
            _ => escaped_text.push(c),
        }
    }

    escaped_text
}

// This function sends a message in Markdown format if it's less than 4096 characters. If it's longer, it splits the message into chunks of 4096 characters and sends them separately.
pub async fn safe_send(bot: Bot, chat_id: ChatId, reply_to_msg_id: MessageId, text: &str) {
    // Try sending the message as Markdown if it's less than 4096 characters
    if text.len() <= 4096 {
        let escaped_text = escape_markdown(text);
        let result = bot
            .send_message(chat_id, escaped_text)
            .reply_parameters(ReplyParameters::new(reply_to_msg_id))
            .parse_mode(ParseMode::MarkdownV2)
            .send()
            .await;

        // If sending as Markdown succeeds, return
        match result {
            Ok(_) => return,
            Err(err) => {
                warn!(
                    "Failed to send as Markdown: {:?}, trying as plain text...",
                    err
                );
            }
        }
    }

    // If sending as Markdown fails or the text is too long, log a warning and try sending as plain text. We can now split the string.
    let split_text = split_string(text, 4096);
    if split_text.len() > 1 {
        info!(
            "Splitting the message into {} part(s) since it's too long",
            split_text.len()
        );
    }

    for text in split_text {
        let res = bot
            .send_message(chat_id, text)
            .reply_parameters(ReplyParameters::new(reply_to_msg_id))
            .send()
            .await;

        if let Err(err) = res {
            error!("Failed to send message: {:?}", err);
        }
    }
}

/// Function to remove `/g/` from the given URL
pub fn remove_g_segment(mut url: Url) -> reqwest::Url {
    // Get the current path segments
    let path_segments: Vec<String> = url
        .path_segments()
        .map_or_else(Vec::new, |c| c.map(String::from).collect());

    // Remove the "/g/" from the path segments
    let path_segments: Vec<String> = path_segments
        .into_iter()
        .filter(|segment| segment != "g")
        .collect();

    // Clear the existing path and add the modified path segments back
    url.set_path("");
    url.path_segments_mut()
        .expect("Cannot be base URL")
        .extend(path_segments);

    // Convert back to URL
    let url = Url::parse(url.as_ref()).expect("Failed to parse URL");
    url
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

    #[test]
    fn test_remove_g_segment() {
        let original_url = Url::parse("https://tiger-lab-t2v-turbo-v2.hf.space/g/gradio_api/file=/tmp/gradio/285a20f83d99030cb2639db6c8d6e84ad97670be17e82bc0bd70490dc3346a70/tmp.mp4").unwrap();
        let expected_url = Url::parse("https://tiger-lab-t2v-turbo-v2.hf.space/gradio_api/file=/tmp/gradio/285a20f83d99030cb2639db6c8d6e84ad97670be17e82bc0bd70490dc3346a70/tmp.mp4").unwrap();

        let result = remove_g_segment(original_url);
        assert_eq!(result, expected_url);
    }
}
