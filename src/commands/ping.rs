use log::error;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::requests::ResponseResult;

use teloxide::{types::Message, Bot};

pub async fn ping(bot: Bot, msg: Message) -> ResponseResult<Message> {
    // Ping api.telegram.org and calculate the latency
    let start = std::time::Instant::now();
    let res = reqwest::get("https://api.telegram.org").await;
    let latency = start.elapsed().as_millis();
    match res {
        Ok(_) => {
            bot.send_message(msg.chat.id, format!("Pong! Latency: `{latency}ms`"))
                .reply_to_message_id(msg.id)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await
        }
        Err(e) => {
            error!("Error calculating latency: {e}");
            bot.send_message(msg.chat.id, format!("Error calculating latency: `{e}`"))
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .reply_to_message_id(msg.id)
                .await
        }
    }
}
