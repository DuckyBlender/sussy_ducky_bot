use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use log::{error, info, warn};
use teloxide::payloads::{SendAudioSetters, SendMessageSetters};
use teloxide::types::InputFile;
use teloxide::{requests::Requester, types::Message, Bot, RequestError};
use tokio::pin;
use tokio_stream::StreamExt;
use comfyui_api::api::Api;
use comfyui_api::models::{Prompt, Update};

pub async fn stable_audio(
    bot: Bot,
    message: Message,
    prompt: Option<String>,
) -> Result<(), RequestError> {
    info!("Starting stable audio command!");

    // Check if prompt is empty
    let prompt = match prompt {
        Some(prompt) => prompt,
        None => {
            let bot_msg = bot
                .send_message(message.chat.id, "No prompt provided")
                .reply_to_message_id(message.id)
                .await?;

            // Wait 5 seconds
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // Deleting the messages
            bot.delete_message(message.chat.id, message.id).await?;
            bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
            return Ok(());
        }
    };

    // Read prompt from disk
    let json = std::fs::read_to_string("comfyui_nodes/stable_audio_api.json")
        .expect("Failed to read prompt from disk");

    // Replace <PROMPT> with the actual prompt
    let json = json.replace("<PROMPT>", &prompt);

    let json: Prompt = serde_json::from_str(&json).expect("Failed to parse prompt");

    let api = Api::default();
    info!("API created with default host/port");
    let prompt_api = api.prompt().expect("Failed to create prompt API");
    info!("Prompt API created");
    let history = api.history().expect("Failed to create history API");
    info!("History API created");
    // let view_api = api.view().expect("Failed to create view API");
    // info!("View API created");

    let websocket = api.websocket().expect("Failed to create websocket API");
    info!("Websocket API created");
    let stream = websocket
        .updates()
        .await
        .expect("Failed to get updates stream");

    info!("Sending prompt...");
    // Send typing indicator
    bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::Typing)
        .await?;
    let response = prompt_api.send(&json).await.expect("Failed to send prompt");

    info!("Prompt sent, id: {}", response.prompt_id);
    info!("Waiting for updates...");

    pin!(stream);

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(msg) => match msg {
                Update::ExecutionStart(data) => {
                    info!("Execution started: {:?}", data);
                }
                Update::Executing(data) => {
                    if let Some(node) = data.node {
                        info!("Executing: {:#?}", json.workflow[&node]);
                    } else if let Some(ref prompt_id) = data.prompt_id {
                        info!("Nothing left to execute.");
                        let task = history
                            .get_prompt(prompt_id)
                            .await
                            .expect("Failed to get prompt");
                        info!("Number: {}", task.prompt.num);
                        bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::UploadVoice)
                            .await?;

                        // This is a god-awful solution, but it unfortunately works
                        // We can't use the library normally, because we have audio as output, not an image
                        // So we have to send the audio file from the filesystem
                        let audio_path = "C:\\Users\\Alan\\Downloads\\ComfyUI_windows_portable_nvidia_cu121_or_cpu\\ComfyUI_windows_portable\\ComfyUI\\output\\audio";
                        // Wait 0.5s to make sure the file is written to disk
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        send_newest_audio_file(audio_path, &bot, &message).await?;
                        // return Ok(());

                        // for (key, value) in task.outputs.nodes.iter() {
                        //     if let NodeOutputOrUnknown::NodeOutput(output) = value {
                        //         info!("Node: {}", key);
                        //         for image in output.images.iter() {
                        //             info!("Generated image: {:?}", image);
                        //         }
                        //     }
                        // }
                        // break;
                    }
                }
                Update::ExecutionCached(data) => {
                    info!("Execution cached: {:?}", data.nodes);
                }
                Update::Executed(data) => {
                    // let _image = view_api
                    //     .get(&data.output.images[0])
                    //     .await
                    //     .expect("Failed to get image");
                    // for image in data.output.images.iter() {
                    //     info!("Generated image: {:?}", image);
                    //     bot.send_message(message.chat.id, image.filename.clone())
                    //         .reply_to_message_id(message.id)
                    //         .await?;
                    // }
                }
                Update::ExecutionInterrupted(data) => {
                    warn!("Execution interrupted: {:#?}", data);
                    break;
                }
                Update::ExecutionError(data) => {
                    error!("Execution error: {:#?}", data);
                    break;
                }
                Update::Progress(data) => {
                    if data.value == data.max {
                        info!("100%");
                    } else {
                        info!("{}%", (data.value as f64 / data.max as f64) * 100.0);
                    }
                }
                Update::Status { status } => {
                    info!("Status: {} queued.", status.exec_info.queue_remaining);
                }
            },
            Err(e) => {
                error!("Error occurred: {:#?}", e);
            }
        }
    }

    Ok(())
}

// Function to get the newest file in a directory
async fn send_newest_audio_file(directory: &str, bot: &Bot, message: &Message) -> std::io::Result<()> {
    let mut files: Vec<(PathBuf, SystemTime)> = fs::read_dir(directory)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() {
                let modified = fs::metadata(&path).ok()?.modified().ok()?;
                Some((path, modified))
            } else {
                None
            }
        })
        .collect();

    // Sort files by modified time in descending order
    files.sort_by(|a, b| b.1.cmp(&a.1));

    // Get the newest file
    if let Some((newest_file, _)) = files.first() {
        let audio_path = newest_file.display().to_string();
        info!("Audio path: {}", audio_path);
        let inputfile = InputFile::memory(std::fs::read(audio_path)?);
        bot.send_audio(message.chat.id, inputfile)
            .reply_to_message_id(message.id)
            .await
            .expect("Failed to send audio file");
    }

    Ok(())
}