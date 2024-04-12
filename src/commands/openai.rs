use log::{error, info};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

use crate::structs::{GPT4Content, GPT4ImageUrl, GPT4Message, GPT4Request, GPT4Response};
use crate::utils::ModelType;

pub async fn openai(
    bot: Bot,
    msg: Message,
    prompt: String,
    model: ModelType,
) -> Result<(), RequestError> {
    // Check if the user is from the owner
    let owner_id = match std::env::var("OWNER_ID") {
        Ok(id) => id.parse().unwrap(),
        Err(_) => {
            bot.send_message(msg.chat.id, "Error: Unable to fetch OWNER_ID")
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(());
        }
    };

    if msg.from().unwrap().id != UserId(owner_id) {
        bot.send_message(msg.chat.id, "You are not the owner")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(());
    }

    // Check if there is an image attached in the reply
    let img_attachment = if let Some(reply) = msg.reply_to_message() {
        reply.photo().map(|photo| photo.last().unwrap())
    } else {
        None
    };

    if prompt.is_empty() && img_attachment.is_none() {
        let bot_msg = bot
            .send_message(msg.chat.id, "No prompt provided")
            .reply_to_message_id(msg.id)
            .await?;

        // Wait 5 seconds and delete the users and the bot's message
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Deleting the messages
        bot.delete_message(msg.chat.id, msg.id).await?;
        bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
        return Ok(());
    }

    info!(
        "Starting OpenAI request function with prompt: {}{}",
        prompt,
        if img_attachment.is_some() {
            " and an image"
        } else {
            ""
        }
    );

    // Send generating... message
    let generating_message = bot
        .send_message(msg.chat.id, "Generating...")
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    let now = std::time::Instant::now();

    // Get the image URL if it exists
    let img_url = if let Some(img_attachment) = img_attachment {
        let img_attachment = bot.get_file(&img_attachment.file.id).await?;
        let teloxide_token = match std::env::var("TELOXIDE_TOKEN") {
            Ok(token) => token,
            Err(_) => {
                bot.edit_message_text(
                    generating_message.chat.id,
                    generating_message.id,
                    "Error: Unable to fetch TELOXIDE_TOKEN",
                )
                .await?;
                return Ok(());
            }
        };
        let img_url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            teloxide_token, img_attachment.path
        );
        Some(img_url)
    } else {
        None
    };

    let mut json = GPT4Request {
        model: model.to_string(),
        messages: vec![GPT4Message {
            role: "user".to_string(),
            content: vec![
                GPT4Content {
                    content_type: "text".to_string(),
                    text: Some("Describe the image in one sentence.".to_string()),
                    image_url: None,
                },
                // this will be pushed if img_url is Some
                // GPT4Content {
                //     content_type: "image_url".to_string(),
                //     text: None,
                //     image_url: Some(GPT4ImageUrl { url: img_url }),
                // },
            ],
        }],
        max_tokens: 300,
    };

    if let Some(img_url) = img_url {
        json.messages[0].content.push(GPT4Content {
            content_type: "image_url".to_string(),
            text: None,
            image_url: Some(GPT4ImageUrl { url: img_url }),
        });
    }

    let openai_key = match std::env::var("OPENAI_KEY") {
        Ok(key) => key,
        Err(_) => {
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                "Error: Unable to fetch OPENAI_KEY",
            )
            .await?;
            return Ok(());
        }
    };

    let res = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(openai_key)
        .json(&json)
        .send()
        .await;
    let elapsed = now.elapsed().as_secs_f32();

    match res {
        Ok(_) => {
            info!("Request to OpenAI sent successfully");
        }
        Err(e) => {
            error!("Error sending request: {}", e);
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                format!("Error: {e}"),
            )
            .await?;
            return Ok(());
        }
    };

    // Parse the response
    let res = res.unwrap().json::<GPT4Response>().await;

    info!("Response: {:#?}", res);

    // Send the response
    match res {
        Ok(res) => {
            let content = res.choices[0].message.content.as_str();

            info!(
                "Replying to message using OpenAIs. Generation took {}s",
                (elapsed * 10.0).round() / 10.0
            );
            bot.edit_message_text(generating_message.chat.id, generating_message.id, content)
                .await?;
            Ok(())
        }
        Err(e) => {
            error!("Error parsing response: {:#?}", e);
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
