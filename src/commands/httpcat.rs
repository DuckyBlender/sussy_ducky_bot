use log::error;
use reqwest::StatusCode;
use teloxide::payloads::SendMessageSetters;
use teloxide::payloads::SendPhotoSetters;
use teloxide::types::InputFile;

use teloxide::RequestError;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot,
};

pub async fn httpcat(bot: Bot, msg: Message, prompt: Option<String>) -> Result<(), RequestError> {
    // Ping http://http.cat/{argument}
    let prompt = match prompt {
        Some(prompt) => prompt,
        None => {
            let bot_msg = bot
                .send_message(msg.chat.id, "Please provide a status code.")
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
    let status_code = prompt.to_string();
    let first_argument = prompt.split(' ').next().unwrap();
    // Check if it's a 3 digit number
    if first_argument.len() != 3 {
        bot.send_message(
            msg.chat.id,
            "Invalid argument: Please provide a 3 digit status code",
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }
    // Download the image
    let res = reqwest::get(format!("https://http.cat/{first_argument}")).await;
    // Send the image
    match res {
        Ok(res) => {
            let body = res.bytes().await.unwrap();
            let buf = body.to_vec();
            let file = InputFile::memory(buf);
            bot.send_chat_action(msg.chat.id, ChatAction::UploadPhoto)
                .await?;
            bot.send_photo(msg.chat.id, file)
                .reply_to_message_id(msg.id)
                .await?;
        }
        Err(e) => {
            // Check which error it is
            match e.status() {
                Some(StatusCode::NOT_FOUND) => {
                    error!("Error: {status_code}");
                    bot.send_message(
                        msg.chat.id,
                        format!("Error: {status_code} is not a valid status code"),
                    )
                    .reply_to_message_id(msg.id)
                    .await?;
                }
                _ => {
                    bot.send_message(msg.chat.id, format!("Error downloading image: {e}"))
                        .reply_to_message_id(msg.id)
                        .await?;
                }
            }
        }
    }

    Ok(())
}
