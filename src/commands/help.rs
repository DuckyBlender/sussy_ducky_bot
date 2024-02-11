use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use teloxide::requests::ResponseResult;

use teloxide::types::ParseMode;
use teloxide::{types::Message, Bot};

use crate::Commands;

pub async fn help(bot: Bot, msg: Message) -> ResponseResult<Message> {
    let help_text = Commands::new().help_message();
    bot.send_message(msg.chat.id, help_text)
        .reply_to_message_id(msg.id)
        .parse_mode(ParseMode::Html)
        .await
}
