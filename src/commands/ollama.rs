use log::{error, info, warn};
use ollama_rs::generation::parameters::FormatType;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};

use teloxide::{
    requests::Requester,
    types::{ChatAction, Message},
    Bot, RequestError,
};

use tokio_stream::StreamExt;

use crate::models::ModelType;

pub const INTERVAL_SEC: u64 = 5;

pub async fn ollama(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
    ollama_client: Ollama,
) -> Result<(), RequestError> {
    // Check if the model is one of ollamas models
    let ollama_models = ModelType::return_ollama();
    if !ollama_models.contains(&model) {
        bot.send_message(msg.chat.id, "Error: Invalid model")
            .reply_to_message_id(msg.id)
            .await?;
        error!("Invalid model: {model}. This should not happen!");
        return Ok(());
    }

    // Check if prompt is empty
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

    //         bot.send_message(msg.chat.id, parse_markdown(message))
    //             // .parse_mode(teloxide::types::ParseMode::MarkdownV2)
    //             .parse_mode(teloxide::types::ParseMode::Html)
    //             .await?;

    //         return Ok(());
    //     }

    // Log the request (as JSON)
    info!(
        "Sending request to ollama using model {} and length: {} chars",
        model,
        prompt.len()
    );

    // Send a message to the chat to show that the bot is generating a response
    let generating_message = bot
        .send_message(msg.chat.id, "Generating response...".to_string())
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    // Send typing indicator
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Send the stream request using ollama-rs
    let before_request = std::time::Instant::now();
    let mut request = GenerationRequest::new(model.to_string(), prompt);
    if model == ModelType::Json {
        request = request.format(FormatType::Json).system(
            "You are a JSONify bot. Convert the text to JSON. Structure it however you like."
                .to_string(),
        );
    }
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

    // TODO: Inline markup for stopping the response or regenerating it if it's done
    // This requires a global list of messages that are being edited to keep track of everything.
    // This is quite complicated and I'm not sure how to do it yet
    // Maybe a global mutex from the main function which is constantly updated? I'm not sure

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

                // There is currently a bug with ollama where infinite newline spaces generate after generating the JSON
                // See https://github.com/ollama/ollama/pull/3784

                if model == ModelType::Json {
                    // Check if the response is valid JSON
                    if let Ok(json) =
                        serde_json::from_str::<serde_json::Value>(&current_message_content)
                    {
                        info!("Valid JSON recieved, breaking loop.");
                        // set entire_response to be pretty printed JSON
                        entire_response = serde_json::to_string_pretty(&json).unwrap();
                        // add ``` to the end of the response
                        entire_response.push_str("\n```");
                        break 'response_loop;
                    }
                }

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

    let mut final_final_response = entire_response.clone().trim_end().to_string();

    if ModelType::Json == model {
        final_final_response = format!("```json\n{}\n```", final_final_response);
        // Edit the message one last time
        bot.edit_message_text(
            generating_message.chat.id,
            generating_message.id,
            final_final_response,
        )
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .await?;
    } else {
        // Edit the message one last time
        bot.edit_message_text(
            generating_message.chat.id,
            generating_message.id,
            final_final_response,
        )
        .await?;
    }

    let elapsed = before_request.elapsed().as_secs_f32();

    info!(
        "Generated ollama response.\n - Time elapsed: {:.2}s\n - Model: {}\n - Gen. Length: {}",
        elapsed,
        model,
        entire_response.len()
    );

    Ok(())
}
