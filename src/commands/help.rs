use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::requests::ResponseResult;

use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;
use teloxide::{types::Message, Bot};

use crate::Command;

pub async fn help(bot: Bot, msg: Message) -> ResponseResult<Message> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .reply_to_message_id(msg.id)
        .parse_mode(ParseMode::Html)
        .await
}
