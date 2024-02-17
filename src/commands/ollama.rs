use log::{error, info};
use teloxide::payloads::SendMessageSetters;

use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

use crate::utils::{remove_prefix, ModelType};
use crate::{OllamaChatRequest, OllamaChatRequestMessage, OllamaChatResponse};

pub async fn ollama(
    bot: Bot,
    msg: Message,
    prompt: String,
    model_type: ModelType,
) -> Result<Message, RequestError> {
    info!("Starting ollama function");
    // Form the OllamaChatRequest object
    let mut ollama_request = OllamaChatRequest {
        model: ModelType::to_string(&model_type),
        messages: vec![],
        stream: false,
    };

    // Add the prompt to the request
    ollama_request.messages.push(OllamaChatRequestMessage {
        role: "user".to_string(),
        content: prompt,
    });

    // Get the prompt and add the replies as history. Bot is "assistant" and user is "user".
    // Sadly, this can only work with one message. This is because the bot can't get the replies of the replies.
    let mut history = vec![];

    let mut message = msg.clone();
    while let Some(reply) = message.reply_to_message() {
        let role = if history.len() % 2 == 0 {
            "assistant"
        } else {
            "user"
        };
        // Remove the command from the message using the remove_prefix function
        let content = remove_prefix(reply.clone());
        history.push((role, content));
        message = reply.clone();
    }

    // Add the history to the request
    for (role, content) in history {
        ollama_request.messages.push(OllamaChatRequestMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    // Reverse the messages so that the prompt is first
    ollama_request.messages.reverse();

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Log the request (as JSON)
    info!("Sending request to ollama: {:#?}", ollama_request);

    // Send the request
    let now = std::time::Instant::now();
    let res = reqwest::Client::new()
        .post("http://localhost:11434/api/chat")
        .json(&ollama_request)
        .send()
        .await;
    let elapsed = now.elapsed().as_secs_f32();

    match res {
        Ok(_) => {
            info!("Ollama request was successful");
        }
        Err(e) => {
            error!("Error sending request: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(msg);
        }
    };

    // Parse the response
    let res = res.unwrap().json::<OllamaChatResponse>().await;

    // Send the response
    match res {
        Ok(res) => {
            info!(
                "Replying to message using ollama. Generation took {}s",
                (elapsed * 10.0).round() / 10.0
            );
            bot.send_message(msg.chat.id, res.message.content)
                .reply_to_message_id(msg.id)
                .await
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {e}"))
                .reply_to_message_id(msg.id)
                .await
        }
    }
}
