use log::{error, info};
use teloxide::payloads::SendMessageSetters;
use teloxide::{requests::Requester, types::Message, Bot, RequestError};

use crate::apis::groq::{generate_groq, GroqModels};
use crate::utils::{delete_both_delay, get_prompt};

pub async fn groq(bot: Bot, msg: Message, model: GroqModels) -> Result<(), RequestError> {
    info!("Starting groq request function");

    let prompt = get_prompt(msg.clone()).await;
    if prompt.is_none() {
        let bot_msg = bot
            .send_message(msg.chat.id, "No prompt provided")
            .reply_to_message_id(msg.id)
            .await?;
        delete_both_delay(bot, msg, bot_msg).await;
        return Ok(());
    }
    let prompt = prompt.unwrap();

    // groq is too fast for a generating message 🔥
    // Send "typing indicator" to show that the bot is typing
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    let now = std::time::Instant::now();
    // Send the request to the Perplexity API
    let res = generate_groq(&prompt, model, 0.2, 1).await;
    let elapsed = now.elapsed().as_secs_f32();

    let res = match res {
        Ok(res) => {
            info!("Request sent successfully");
            res
        }
        Err(e) => {
            error!("Error sending request: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(());
        }
    };

    info!(
        "Replying to message using groq. Generation took {}s",
        (elapsed * 10.0).round() / 10.0
    );
    bot.send_message(msg.chat.id, res)
        .reply_to_message_id(msg.id)
        .await?;
    Ok(())
}
