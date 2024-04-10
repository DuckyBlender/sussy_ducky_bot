use std::collections::HashMap;
use std::sync::Arc;

use log::info;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use teloxide::payloads::SendMessageSetters;
use teloxide::types::ChatId;
use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::utils::ModelType;

const INTERVAL_SEC: u64 = 5;

pub async fn ollama(
    bot: Bot,
    msg: Message,
    prompt: String,
    model_type: ModelType,
    ollama_client: Ollama,
    ollama_queue: Arc<Mutex<HashMap<ChatId, Message>>>,
) -> Result<(), RequestError> {
    // Remove the first word (the command)
    let prompt = prompt
        .split_once(' ')
        .map(|x| x.1)
        .unwrap_or_default()
        .trim()
        .to_string();

    if prompt.is_empty() {
        let bot_msg = bot
            .send_message(msg.chat.id, "Please provide a prompt")
            .reply_to_message_id(msg.id)
            .await?;

        // Wait 5 seconds and delete the users and the bot's message
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Deleting the messages
        bot.delete_message(msg.chat.id, msg.id).await?;
        bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
        return Ok(());
    }

    // if the prompt is exactly "SIUDFNISUDF" then send a test message to the chat
    //     if prompt == "SIUDFNISUDF" {
    //         let message = r#"""
    //         *bold \*text*
    // _italic \*text_
    // __underline__
    // ~strikethrough~
    // ||spoiler||
    // *bold _italic bold ~italic bold strikethrough ||italic bold strikethrough spoiler||~ __underline italic bold___ bold*
    // [inline URL](http://www.example.com/)
    // [inline mention of a user](tg://user?id=123456789)
    // ![👍](tg://emoji?id=5368324170671202286)
    // `inline fixed-width code`
    // ```
    // pre-formatted fixed-width code block
    // ```
    // ```python
    // pre-formatted fixed-width code block written in the Python programming language
    // ```
    // >Block quotation started
    // >Block quotation continued
    // >The last line of the block quotation**
    // >The second block quotation started right after the previous\r
    // >The third block quotation started right after the previous
    //         """#;

    //         bot.send_message(msg.chat.id, message)
    //             // .parse_mode(teloxide::types::ParseMode::MarkdownV2)
    //             .await?;

    //         return Ok(());
    //     }

    // Log the request (as JSON)
    info!(
        "Sending request to ollama using model {} and length: {} chars",
        model_type,
        prompt.len()
    );

    // Send a message to the chat to show that the bot is generating a response
    let generating_message = bot
        .send_message(msg.chat.id, "Generating response...")
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    let now = std::time::Instant::now();

    // After sending the "Generating response..." message, store it in the ollama_queue
    ollama_queue
        .lock()
        .await
        .insert(msg.chat.id, generating_message.clone());

    let waiting_time = now.elapsed().as_secs_f32();
    
    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Send the stream request using ollama-rs
    let before_request = std::time::Instant::now();
    let request = GenerationRequest::new(model_type.to_string(), prompt);
    let stream = ollama_client.generate_stream(request).await;

    match stream {
        Ok(_) => info!("Stream request successful"),
        Err(e) => {
            info!("Stream request failed: {}", e);
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
    let mut current_string = String::new();

    // TODO: Inline markup for stopping the response or regenerating it if it's done
    // This requires a global list of messages that are being edited to keep track of everything.
    // This is quite complicated and I'm not sure how to do it yet
    // Maybe a global mutex from the main function which is constantly updated? I'm not sure

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

                    // Before editing the message, check if it's still in the ollama_queue
                    if let Some(generating_message) = ollama_queue.lock().await.get(&msg.chat.id) {
                        bot.edit_message_text(
                            generating_message.chat.id,
                            generating_message.id,
                            current_string.clone() + " [...]",
                        )
                        .await?;
                    }

                    // Send the typing indicator
                    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
                        .await?;

                    // Reset the timer
                    now = std::time::Instant::now();
                }
            }
        } else {
            // If the stream has no more responses, break the loop
            info!("Final response received");

            if current_string.is_empty() {
                current_string = "<no response>".to_string();
            }

            // Edit the message one last time
            bot.edit_message_text(
                generating_message.chat.id,
                generating_message.id,
                current_string.clone(),
            )
            .await?;

            // After finishing the generation, remove the message from the ollama_queue
            ollama_queue.lock().await.remove(&msg.chat.id);

            // TODO: Stop the typing indicator somehow
            break;
        }
    }

    let elapsed = before_request.elapsed().as_secs_f32();

    info!(
        "Generated ollama response in {} seconds. Waited in queue for {} seconds. Model used: {}",
        elapsed,
        waiting_time,
        model_type
    );

    Ok(())
}
