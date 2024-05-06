use teloxide::payloads::SendMessageSetters;

use teloxide::{requests::Requester, types::Message, Bot, RequestError};

use crate::apis::petittube::get_noviews;

pub async fn noviews(bot: Bot, msg: Message) -> Result<(), RequestError> {
    // Send "typing indicator" to show that the bot is typing
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    let url = get_noviews().await.unwrap(); // currently the function can't return an error

    // Send the video URL as a message
    bot.send_message(msg.chat.id, url)
        .reply_to_message_id(msg.id)
        .await?;
    Ok(())
}
