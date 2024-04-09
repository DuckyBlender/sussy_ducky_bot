// curl -X POST -d "input=put some text here" https://chatlgbtapi.bemani.radom.pl/

use teloxide::{
    payloads::SendMessageSetters, requests::Requester, types::Message, Bot, RequestError,
};

pub async fn chatlgbt(bot: Bot, msg: Message, prompt: String) -> Result<(), RequestError> {
    // This is too fast for the typing indicator
    let url = "https://chatlgbtapi.bemani.radom.pl/";

    let client = reqwest::Client::new();
    let body = client
        .post(url)
        .body(format!("input={prompt}"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // Send the response
    bot.send_message(msg.chat.id, body)
        .reply_to_message_id(msg.id)
        .await?;
    Ok(())
}
