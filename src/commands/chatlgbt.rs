use teloxide::{
    payloads::SendMessageSetters, requests::Requester, types::Message, Bot, RequestError,
};

use crate::{apis::chatlgbt::chatlgbt_query, utils::get_prompt};


pub async fn chatlgbt(bot: Bot, msg: Message) -> Result<(), RequestError> {
    let prompt = get_prompt(msg.clone()).await;
    let body = chatlgbt_query(prompt).await.unwrap();

    bot.send_message(msg.chat.id, body)
        .reply_to_message_id(msg.id)
        .await?;
    Ok(())
}