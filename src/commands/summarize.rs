// /summarize command - summarizes a youtube video or given text.

use log::{error, info, warn};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use teloxide::payloads::SendMessageSetters;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};
use tokio_stream::StreamExt;

use crate::commands::ollama::INTERVAL_SEC;
use crate::ModelType;

pub async fn summarize(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    ollama_client: Ollama,
) -> Result<(), RequestError> {
    // Check if the prompt is a youtube video or text
    let prompt = match prompt {
        Some(prompt) => prompt,
        None => {
            // If it's not in the prompt, check the reply
            if let Some(reply) = msg.reply_to_message() {
                if let Some(text) = reply.text() {
                    text.to_string()
                } else {
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
            } else {
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
        }
    };

    let generating_message = bot
        .send_message(msg.chat.id, "Summarizing text...")
        .reply_to_message_id(msg.id)
        .await?;

    info!("Starting summarization command");

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    let text = prompt;

    // Summarize the text using phi-3
    let model = ModelType::Phi3;

    // Send the stream request using ollama-rs
    let before_request = std::time::Instant::now();
    let request = GenerationRequest::new(model.to_string(), text)
        .system("Summarize this text to the best of your abilities.".to_string());
    let stream = ollama_client.generate_stream(request).await;

    match stream {
        Ok(_) => {
            info!(
                "Stream request for model {} successful, incoming token responses..",
                model
            );
        }
        Err(e) => {
            error!("Stream request failed: {}", e);
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                format!("Failed to generate response: {}", e),
            )
            .await?;
            return Ok(());
        }
    }

    let mut stream = stream.unwrap(); // safe unwrap

    // Create a repeating interval that yields every 5 seconds
    let mut now = std::time::Instant::now();

    // Create a string to hold the entire responseAppend [...] when the bot is still recieving
    let mut entire_response = String::new();
    let mut current_message_content = String::new();

    // Parse the response and edit the message every 5 seconds
    'response_loop: while let Some(Ok(res)) = stream.next().await {
        for ele in res {
            // Append the new response to the entire response
            entire_response.push_str(&ele.response);

            // Check if 5 seconds have passed since last edit
            if now.elapsed().as_secs() >= INTERVAL_SEC {
                // Check if the message is identical. Don't know if this is necessary but it's here for now
                if current_message_content == entire_response {
                    continue;
                }

                // Update the current string
                current_message_content.clone_from(&entire_response);

                // Edit the message
                bot.edit_message_text(
                    generating_message.chat.id,
                    generating_message.id,
                    current_message_content.clone() + " [...]",
                )
                .await?;

                // Send the typing indicator
                bot.send_chat_action(msg.chat.id, ChatAction::Typing)
                    .await?;

                // Reset the timer
                now = std::time::Instant::now();
            }

            // If the response is done, break the loop
            if ele.done {
                break 'response_loop;
            }
        }
    }

    info!("Final response received");

    if entire_response.is_empty() {
        warn!("No response received!");
        entire_response = "<no response>".to_string();
    }

    // Edit the message one last time
    bot.edit_message_text(
        generating_message.chat.id,
        generating_message.id,
        entire_response.clone().trim_end(),
    )
    .await?;

    let elapsed = before_request.elapsed().as_secs_f32();

    info!(
        "Generated ollama response.\n - Time elapsed: {:.2}s\n - Model: {}\n - Gen. Length: {}",
        elapsed,
        model,
        entire_response.len()
    );

    Ok(())
}
