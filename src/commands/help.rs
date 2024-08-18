use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::requests::ResponseResult;

use teloxide::types::{ParseMode, ReplyParameters};
use teloxide::utils::command::BotCommands;
use teloxide::{types::Message, Bot};

use crate::Commands;

pub async fn help(bot: Bot, msg: Message) -> ResponseResult<Message> {
    bot.send_message(msg.chat.id, Commands::descriptions().to_string())
        .reply_parameters(ReplyParameters::new(msg.id))
        .parse_mode(ParseMode::Html)
        .await
}
