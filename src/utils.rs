use teloxide::{payloads::SendMessageSetters, requests::Requester, types::Message, Bot, RequestError};
use log::{info, warn};
use std::env;

use crate::models::ModelType;

pub async fn check_owner(bot: &Bot, msg: &Message, model: &ModelType) -> Result<(), RequestError> {
    // Check if the model is owner-only
    let gated_models = ModelType::owner_only();
    if gated_models.contains(model) {
        // Check if the user is the owner
        if msg.from().unwrap().id.0 != env::var("OWNER_ID").unwrap().parse::<u64>().unwrap() {
            warn!("Model {} is owner-only!", model.to_string());
            bot.send_message(msg.chat.id, format!("Model {} is owner-only!", model))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(());
        } else {
            info!("Model {} is owner-only but the user is the owner", model.to_string());
        }
    }
    Ok(())
}

/// If the prompt is empty, check the reply
pub fn get_prompt(prompt: String, msg: &Message) -> Option<String> {
    if prompt.is_empty() {
        msg.reply_to_message()
            .map(|reply| reply.text().unwrap_or_default().to_string())
    } else {
        Some(prompt)
    }
}
