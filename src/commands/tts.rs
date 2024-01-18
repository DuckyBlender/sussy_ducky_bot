use log::error;
use rand::seq::SliceRandom;
use teloxide::payloads::{SendMessageSetters, SendVoiceSetters};
use teloxide::requests::ResponseResult;
use teloxide::types::InputFile;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot,
};

use crate::{TTSRequest, TTS_VOICES};

pub async fn tts(bot: Bot, msg: Message, args: String) -> ResponseResult<()> {
    // Available TTS voices: alloy, echo, fable, onyx, nova, and shimmer
    // Check if there is a prompt after the command
    // If not, check if there is a reply
    // If not, send an error message

    let prompt;
    if args.is_empty() {
        if let Some(reply) = msg.reply_to_message() {
            if let Some(text) = reply.text() {
                prompt = text.to_string();
            } else {
                bot.send_message(msg.chat.id, "No prompt provided")
                    .reply_to_message_id(msg.id)
                    .await?;
                return Ok(());
            }
        } else {
            bot.send_message(msg.chat.id, "No prompt provided")
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(());
        }
    } else {
        prompt = args.to_string();
    }

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::RecordVoice)
        .await?;

    // Send the request
    let voice = TTS_VOICES.choose(&mut rand::thread_rng()).unwrap();
    let res = reqwest::Client::new()
        .post("https://api.openai.com/v1/audio/speech")
        .bearer_auth(std::env::var("OPENAI_KEY").unwrap())
        .json(&TTSRequest {
            model: "tts-1".to_string(),
            input: prompt,
            voice: (*voice).to_string(),
        })
        .send()
        .await;

    if res.is_err() {
        error!("Error sending request: {}", res.as_ref().err().unwrap());
        bot.send_message(
            msg.chat.id,
            format!("Error: {}", res.as_ref().err().unwrap()),
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    // Check if the request was successful
    if res.as_ref().unwrap().status().is_client_error() {
        let error = res.unwrap().text().await.unwrap();
        error!("Error sending request: {}", error);
        bot.send_message(msg.chat.id, format!("Error: {error}"))
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(());
    }

    bot.send_chat_action(msg.chat.id, ChatAction::RecordVoice)
        .await?;

    // The response is the audio file in mp3
    // Send the response
    let res = res.unwrap();

    let body = res.bytes().await?;
    let buf = body.to_vec();
    bot.send_voice(msg.chat.id, InputFile::memory(buf))
        .caption(format!("Voice: {voice}"))
        .reply_to_message_id(msg.id)
        .await?;

    Ok(())
}
