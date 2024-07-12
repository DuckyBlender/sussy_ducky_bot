// File for AWS Bedrock commands
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::Engine as _;
use log::error;
use log::info;
use log::warn;
use serde_json::json;
use teloxide::net::Download;
use teloxide::payloads::SendMessageSetters;
use teloxide::payloads::SendPhotoSetters;
use teloxide::types::UserId;
use teloxide::{requests::Requester, types::Message, Bot, RequestError};

use crate::ModelType;
use aws_sdk_bedrockruntime::primitives::Blob;
use std::str;

pub async fn bedrock(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
    aws_client: aws_sdk_bedrockruntime::Client,
) -> Result<(), RequestError> {
    info!("Starting AWS bedrock function");

    // Check if the user is from the owner
    if msg.from().unwrap().id != UserId(std::env::var("OWNER_ID").unwrap().parse().unwrap()) {
        bot.send_message(
            msg.chat.id,
            "You are not the owner. Please mention @DuckyBlender if you want to use this command!",
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    // Check if the model is one of bedrocks models
    let bedrock_models = ModelType::return_bedrock();
    if !bedrock_models.contains(&model) {
        bot.send_message(msg.chat.id, "Error: Invalid model")
            .reply_to_message_id(msg.id)
            .await?;
        error!("Invalid model: {model}. This should not happen!");
        return Ok(());
    }

    let mut prompt = match prompt {
        Some(prompt) => Some(prompt),
        None => {
            // Image Variation has an optional prompt
            if model != ModelType::AmazonTitanImageVariation {
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
            } else {
                None
            }
        }
    };

    let mut img = String::new();
    // These models CAN have an image attached
    if model == ModelType::AmazonTitanImageVariation || model == ModelType::Claude3 {
        // Check if there is an image or sticker attached in the reply
        let img_attachment = if let Some(reply) = msg.reply_to_message() {
            info!("There is a reply to the message");
            reply
                .photo()
                .map(|photo| photo.last().unwrap().file.id.clone())
                .or_else(|| reply.sticker().map(|sticker| &sticker.file.id).cloned())
        } else {
            info!("There is no reply to the message");
            None
        };

        // This NEEDS an image attached
        if img_attachment.is_none() && model == ModelType::AmazonTitanImageVariation {
            let bot_msg = bot
                .send_message(msg.chat.id, "No image or sticker provided")
                .reply_to_message_id(msg.id)
                .await?;

            // Wait 5 seconds and delete the users and the bots message
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // Deleting the messages
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
            return Ok(());
        }

        // Download the image
        if img_attachment.is_some() {
            let img_attachment = img_attachment.unwrap();
            let img_file = bot.get_file(&img_attachment).await.unwrap();
            let img_url = img_file.path;
            let mut buf: Vec<u8> = Vec::new();
            bot.download_file(&img_url, &mut buf).await.unwrap();
            img = BASE64.encode(&buf);
        }
    }

    // Send a message to the chat to show that the bot is generating a response
    let generating_message = bot
        .send_message(msg.chat.id, "Generating response...")
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    // Send "typing indicator" to show that the bot is typing
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    let now = std::time::Instant::now();

    // Send the request to the AWS Bedrock API
    let json_body = match model {
        // amazon text models
        ModelType::AmazonTitanText | ModelType::AmazonTitanTextLite => {
            info!("Creating JSON for AmazonTitanText");
            json!(
                {
                    "inputText": prompt.clone().unwrap(),
                    // Other default parameters:
                    // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-text.html
                }
            )
        }

        // cohere commandR and commandR+ models
        // ModelType::CommandR | ModelType::CommandRPlus => {
        //     info!("Creating JSON for CommandR");
        //     json!({
        //         "message": prompt.clone().unwrap()
        //         // Other default parameters:
        //         // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-cohere-command-r-plus.html

        //     })
        // }

        ModelType::AmazonTitanImage => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-image.html
            info!("Creating JSON for AmazonTitanImage");
            let mut json = json!({
                "taskType": "TEXT_IMAGE",
                "textToImageParams": {
                    "text": prompt.clone().unwrap(),
                    // "negativeText": ""
                },
                "imageGenerationConfig": {
                    "numberOfImages": 1,
                    "height": 512,
                    "width": 512,
                    // rest of the default parameters in the URL above
                }
            });

            // if the "text" field is empty, remove it
            if json["textToImageParams"]["text"]
                .as_str()
                .unwrap()
                .is_empty()
            {
                json["textToImageParams"]
                    .as_object_mut()
                    .unwrap()
                    .remove("text");
            }

            json
        }

        ModelType::AmazonTitanImageVariation => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-image.html
            info!("Creating JSON for AmazonTitanImageVariation");
            let mut json = json!({
                 "taskType": "IMAGE_VARIATION",
                 "imageVariationParams": {
                     "text": prompt.clone().unwrap_or_default(),
                    //  "negativeText": "",
                     "images": [img],
                     "similarityStrength": 0.7, // default
                 },
                 "imageGenerationConfig": {
                    "numberOfImages": 1,
                    "height": 512,
                    "width": 512,
                    "cfgScale": 8.0
                }

            });
            // if the "text" field is empty, remove it
            if json["imageVariationParams"]["text"]
                .as_str()
                .unwrap()
                .is_empty()
            {
                json["imageVariationParams"]
                    .as_object_mut()
                    .unwrap()
                    .remove("text");
            }
            json
        }

        ModelType::Claude3 => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-anthropic-claude-messages.html (multimodal section)
            info!("Creating JSON for Claude3");
            let mut json = json!({
                "anthropic_version": "bedrock-2023-05-31",
                "max_tokens": 1024,
                "messages": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "text",
                                "text": prompt.clone().unwrap(), // safe unwrap
                            },
                            // {
                            //     "type": "image",
                            //     "source": {
                            //         "type": "base64",
                            //         "media_type": "image/jpeg",
                            //         "data": img,
                            //     },
                            // },
                        ],
                    }
                ],
            });

            // If there is an image, add it to the JSON
            if !img.is_empty() {
                info!("Adding image to the JSON");
                json["messages"][0]["content"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/jpeg",
                            "data": img,
                        },
                    }));
            }

            // If there is an image and the prompt is empty, replace it with "What is in this image?"
            if !img.is_empty() && prompt.clone().unwrap().is_empty() {
                info!("Replacing prompt with 'What is in this image?'");
                json["messages"][0]["content"][0]["text"] = "What is in this image?".into();
                prompt = Some("What is in this image?".to_string());
            }

            json
        }
        _ => {
            unreachable!();
        }
    };

    info!(
        "Sending request to bedrock with prompt: \"{}\" and model: \"{:?}\"",
        prompt.clone().unwrap_or_default(),
        model
    );
    let result = aws_client
        .invoke_model()
        .model_id(model.to_string())
        .content_type("application/json")
        .body(Blob::new(serde_json::to_string(&json_body).unwrap()))
        .send()
        .await;

    // Check for what reason the result is blocked
    if let Err(e) = result {
        // Return the response
        let err = e.into_service_error();
        warn!("Error: {}", err);
        bot.edit_message_text(
            msg.chat.id,
            generating_message.id,
            format!("Error: {}", err),
        )
        .await?;

        return Ok(());
    }
    let result = result.unwrap();

    // Convert the blob to a JSON
    let output_str = str::from_utf8(result.body().as_ref()).unwrap();
    let output_json: serde_json::Value = serde_json::from_str(output_str).unwrap();
    let output = match model {
        ModelType::AmazonTitanText | ModelType::AmazonTitanTextLite => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-text.html
            output_json["results"][0]["outputText"].as_str().unwrap()
        }
        // ModelType::CommandR | ModelType::CommandRPlus => {
        //     // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-cohere-command-r-plus.html
        //     output_json["text"].as_str().unwrap()
        // }
        ModelType::AmazonTitanImage | ModelType::AmazonTitanImageVariation => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-image.html
            output_json["images"][0].as_str().unwrap()
        }
        ModelType::Claude3 => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-anthropic-claude-messages.html
            output_json["content"][0]["text"].as_str().unwrap()
        }
        _ => {
            unreachable!();
        }
    };

    // Calculate the time taken
    let elapsed = now.elapsed().as_secs_f32();

    info!(
        "Replying to message using bedrock. Generation took {}s",
        (elapsed * 10.0).round() / 10.0
    );

    // Send the message
    match model {
        ModelType::AmazonTitanImage | ModelType::AmazonTitanImageVariation => {
            // Convert to bytes and send as a photo
            let output_bytes = BASE64.decode(output).unwrap();
            let output_file = teloxide::types::InputFile::memory(output_bytes);
            bot.send_photo(msg.chat.id, output_file)
                .caption(prompt.unwrap_or_default()) // blank prompt if it doesn't exist
                .reply_to_message_id(msg.id)
                .await?;
            bot.delete_message(generating_message.chat.id, generating_message.id)
                .await?;
        }
        _ => {
            bot.edit_message_text(msg.chat.id, generating_message.id, output)
                .await?;
        }
    };

    Ok(())
}
