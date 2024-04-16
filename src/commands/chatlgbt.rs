// curl -X POST -d "input=put some text here" https://chatlgbtapi.bemani.radom.pl/

use teloxide::{
    payloads::SendMessageSetters, requests::Requester, types::Message, Bot, RequestError,
};

pub async fn chatlgbt(bot: Bot, msg: Message, prompt: Option<String>) -> Result<(), RequestError> {
    // Check if the prompt is empty
    let prompt = match prompt {
        Some(prompt) => prompt,
        None => {
            let bot_msg = bot
                .send_message(msg.chat.id, "No prompt provided")
                .reply_to_message_id(msg.id)
                .await?;

            // Wait 5 seconds
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // Deleting the messages
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
            return Ok(());
        }
    };
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
