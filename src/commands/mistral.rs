use log::{error, info};
use teloxide::payloads::SendMessageSetters;

use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

use crate::utils::MistralType;
use crate::{OllamaRequest, OllamaResponse};

pub async fn mistral(
    bot: Bot,
    msg: Message,
    prompt: String,
    model_type: MistralType,
) -> Result<Message, RequestError> {
    info!("Starting mistral function");
    // If the prompt is empty, check if there is a reply
    let mut prompt: String = if prompt.is_empty() {
        if let Some(reply) = msg.reply_to_message() {
            reply.text().unwrap_or_default().to_string()
        } else {
            bot.send_message(msg.chat.id, "No prompt provided")
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(msg);
        }
    } else {
        prompt
    };

    // Check if prompt is nothing
    if prompt.is_empty() {
        bot.send_message(msg.chat.id, "No prompt provided")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(msg);
    }

    // prompt = format!("[INST] REPLY TO THIS MESSAGE IN CAVEMAN LANGUAGE. MAKE MANY GRAMMATICAL ERRORS. USE ALL CAPS. DON'T USE VERBS [/INST]\n\n"

    let raw: bool;
    if let MistralType::Caveman = model_type {
        prompt = format!("[INST] REPLY TO THIS MESSAGE IN CAVEMAN LANGUAGE. MAKE MANY GRAMMATICAL ERRORS. USE ALL CAPS. DON'T USE VERBS. THE MESSAGE: {prompt}[/INST]");
        raw = true;
    } else {
        raw = false;
    }

    let model = match model_type {
        MistralType::Standard => "mistral",
        MistralType::Caveman => "mistral", // yes this is correct
        MistralType::Dolphin => "dolphin-mistral",
        MistralType::OpenOrca => "mistral-openorca",
    };

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Send the request
    let now = std::time::Instant::now();
    let res = reqwest::Client::new()
        .post("http://localhost:11434/api/generate")
        .json(&OllamaRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            images: None,
            raw,
        })
        .send()
        .await;
    let elapsed = now.elapsed().as_secs_f32();

    match res {
        Ok(_) => {
            // info!("Request sent successfully");
        }
        Err(e) => {
            error!("Error sending request: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(msg);
        }
    };

    // Parse the response
    let res = res.unwrap().json::<OllamaResponse>().await;

    // Send the response
    match res {
        Ok(res) => {
            bot.send_message(
                msg.chat.id,
                // round to one decimal place
                format!(
                    "{}\n\nGeneration time: {}s",
                    res.response,
                    (elapsed * 10.0).round() / 10.0
                ),
            )
            .reply_to_message_id(msg.id)
            .await
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await
        }
    }
}
