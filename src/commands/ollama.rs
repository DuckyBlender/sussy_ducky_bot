use log::info;
use ollama_rs::{
    generation::completion::{request::GenerationRequest, GenerationResponseStream},
    Ollama,
};
use teloxide::payloads::SendMessageSetters;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};
use tokio_stream::StreamExt;

use crate::utils::ModelType;

const INTERVAL_SEC: u64 = 5;

pub async fn ollama(
    bot: Bot,
    msg: Message,
    prompt: String,
    model_type: ModelType,
) -> Result<(), RequestError> {
    info!("Starting ollama function");

    if prompt.is_empty() {
        bot.send_message(msg.chat.id, "Please provide a prompt")
            .await?;
        return Ok(());
    }

    // Send a message to the chat to show that the bot is generating a response
    let generating_message = bot
        .send_message(msg.chat.id, "Generating response...")
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Log the request (as JSON)
    info!(
        "Sending request to ollama using model {} and length: {}",
        model_type,
        prompt.len()
    );

    // Send the stream request using ollama-rs
    let before_request = std::time::Instant::now();

    let ollama = Ollama::default();
    let request = GenerationRequest::new(model_type.to_string(), prompt);
    let mut stream: GenerationResponseStream = ollama.generate_stream(request).await.unwrap();

    // Create a repeating interval that yields every 5 seconds
    let mut now = std::time::Instant::now();

    // Create a string to hold the entire responseAppend [...] when the bot is still recieving
    let mut entire_response = String::new();
    let mut current_string = String::new();

    // Parse the response and edit the message every 5 seconds
    loop {
        // Parse the response
        if let Some(Ok(res)) = stream.next().await {
            for ele in res {
                // Append the new response to the entire response
                entire_response.push_str(&ele.response);

                // Check if 5 seconds have passed since last edit
                if now.elapsed().as_secs() >= INTERVAL_SEC {
                    // Check if the message is identical. Don't know if this is necessary but it's here for now
                    if current_string == entire_response {
                        continue;
                    }

                    current_string = entire_response.clone();

                    // If the message is last, force edit the message
                    // Don't know if this actually works
                    if ele.final_data.is_some() {
                        info!("Final response received using method 1");
                        bot.edit_message_text(
                            generating_message.chat.id,
                            generating_message.id,
                            current_string.clone(),
                        )
                        .await?;
                        break;
                    }

                    // Edit the message
                    bot.edit_message_text(
                        generating_message.chat.id,
                        generating_message.id,
                        current_string.clone() + " [...]",
                    )
                    .await?;

                    // Send the typing indicator
                    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
                        .await?;

                    // Reset the timer
                    now = std::time::Instant::now();
                }
            }
        } else {
            // If the stream has no more responses, break the loop
            info!("Final response received using method 2");
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                entire_response,
            )
            .await?;
            break;
        }
    }

    let elapsed = before_request.elapsed().as_secs_f32();

    info!("Generated ollama response in {} seconds", elapsed);

    Ok(())
}
