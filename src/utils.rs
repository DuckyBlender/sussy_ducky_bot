use base64::{engine::general_purpose, Engine as _};
use log::{debug, error, info, warn};
use teloxide::{
    net::Download,
    prelude::*,
    types::{MessageId, PhotoSize, ReplyParameters, Sticker},
};

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
    if text.starts_with('/') {
        // Find the end of the first word (first whitespace or end of string)
        let end_of_first_word = text.find(char::is_whitespace).unwrap_or(text.len());
        // Return the rest of the string after the first word
        text[end_of_first_word..].trim_start().to_string()
    } else {
        // Return the original string if it doesn't start with '/'
        text.to_string()
    }
}

pub async fn find_prompt(message: &Message) -> (Option<String>, Option<String>) {
    let mut prompt = None;
    let mut context = None;

    // Check if the message itself has text
    if let Some(msg_text) = message.text() {
        let cleaned_text = remove_command(msg_text);
        if !cleaned_text.is_empty() {
            prompt = Some(cleaned_text);
        }
    }

    // Check if the caption itself has text
    if let Some(caption) = message.caption() {
        let cleaned_caption = remove_command(caption);
        if !cleaned_caption.is_empty() {
            prompt = Some(cleaned_caption);
        }
    }

    // Check if the message is a reply to another message
    if let Some(reply) = message.reply_to_message() {
        // Check if the reply message is from a bot
        if let Some(reply_from) = reply.from.as_ref() {
            if reply_from.is_bot {
                info!("The reply message is from the assistant.");
                // If the reply is from the assistant, extract its text as context
                if let Some(reply_text) = reply.text() {
                    let cleaned_text = remove_command(reply_text);
                    if !cleaned_text.is_empty() {
                        context = Some(cleaned_text);
                    }
                }
            } else {
                // If the reply is not from the assistant, extract its text as prompt.
                // If there was a prompt before, prefix it with the reply text and newlines
                if let Some(reply_text) = reply.text() {
                    let cleaned_text = remove_command(reply_text);
                    if !cleaned_text.is_empty() {
                        if let Some(prompt_text) = prompt {
                            prompt = Some(format!("{cleaned_text}\n\n{prompt_text}"));
                        } else {
                            prompt = Some(cleaned_text);
                        }
                    }
                }
            }
        }

        // If no prompt found yet, check if the reply message has text
        if prompt.is_none() {
            if let Some(reply_text) = reply.text() {
                let cleaned_text = remove_command(reply_text);
                if !cleaned_text.is_empty() {
                    prompt = Some(cleaned_text);
                }
            }
        }

        // If no prompt found yet, check for a caption in the reply
        if prompt.is_none() {
            if let Some(reply_caption) = reply.caption() {
                let cleaned_caption = remove_command(reply_caption);
                if !cleaned_caption.is_empty() {
                    prompt = Some(cleaned_caption);
                }
            }
        }
    }

    // If no valid text or caption found, log the warning
    if prompt.is_none() {
        warn!("No valid text or caption found in the message or reply");
    }

    (prompt, context)
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

    // Split the input into words while preserving whitespace
    let words = input.split_inclusive(char::is_whitespace);

    for word in words {
        // If adding the current word exceeds max_length, finalize the current chunk
        if current_length + word.len() > max_length && !current_chunk.is_empty() {
            result.push(current_chunk);
            current_chunk = String::new();
            current_length = 0;
        }

        // Add the word to the chunk
        current_chunk.push_str(word);
        current_length += word.len();
    }

    // Add the last chunk if it's not empty
    if !current_chunk.is_empty() {
        result.push(current_chunk);
    }

    result
}

// Safe send function to handle long messages
pub async fn safe_send(bot: Bot, chat_id: ChatId, reply_to_msg_id: MessageId, text: &str) {
    // Try sending as plain text. We can now split the string.
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
