use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use teloxide::requests::ResponseResult;
use teloxide::{types::Message, Bot};

pub async fn help(bot: Bot, msg: Message) -> ResponseResult<Message> {
    bot.send_message(msg.chat.id, "Available commands:\n/mistral or /m: generate text\n/llava or /l: generate text from image")
                    .reply_to_message_id(msg.id)
                    .await
}
