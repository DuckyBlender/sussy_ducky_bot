use log::{debug, info};
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use teloxide::{
    prelude::*,
    types::{ChatAction, Message, ReplyParameters},
    utils::command::BotCommands,
};
use tokio::sync::watch;
use tokio::time::{self, Duration};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        // Add blanket level filter
        .level(log::LevelFilter::Warn)
        // - and per-module overrides
        .level_for("sussy_ducky_bot", log::LevelFilter::Info)
        // Output to stdout, files, and other Dispatch configurations
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log").unwrap())
        // Apply globally
        .apply()
        .expect("Failed to initialize logging");

    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    // Get bot information
    let me = bot.get_me().await.expect("Failed to get bot info");
    info!("Started @{}", me.username());

    // TODO: Get sqlite connection

    // Get Ollama connection
    let ollama = Ollama::default();

    // Start the dispatcher
    let handler = Update::filter_message()
        // Handle normal commands
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint({
                    let ollama = ollama.clone();
                    move |bot, msg, cmd| {
                        let ollama = ollama.clone();
                        answer(bot, msg, cmd, ollama)
                    }
                }),
        )
        // Handle messages with captions (e.g., photos or documents with captions)
        .branch(
            dptree::entry()
                .filter(|msg: Message| msg.caption().is_some())
                .endpoint({
                    let ollama = ollama.clone();
                    move |bot, msg| {
                        let ollama = ollama.clone();
                        handle_caption(bot, msg, ollama)
                    }
                }),
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "ask llama 3.2 1b", alias = "l")]
    Llama(String),
}

async fn answer(bot: Bot, msg: Message, cmd: Command, ollama: Ollama) -> ResponseResult<()> {
    debug!("Received command: {:?}", cmd);
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?
        }
        Command::Llama(prompt) => {
            handle_llama_command(bot, msg, prompt, ollama).await?;
            return Ok(());
        }
    };

    Ok(())
}

/// Handles the /llama command
async fn handle_llama_command(bot: Bot, msg: Message, prompt: String, ollama: Ollama) -> ResponseResult<()> {
    // Extract the formatted prompt
    let formatted_prompt = extract_prompt(&msg, Some(prompt)).await;
    info!("Prompt: {}", formatted_prompt);

    // Send initial typing action
    bot.send_chat_action(msg.chat.id, ChatAction::Typing).await?;

    // Create a watch channel to signal the typing indicator task when done
    let (tx, rx) = watch::channel(false);

    // Clone bot and chat id for the typing indicator task
    let bot_clone = bot.clone();
    let chat_id = msg.chat.id;

    // Spawn the typing indicator background task
    tokio::spawn(async move {
        loop {
            // Check if we should stop
            if *rx.borrow() {
                break;
            }
            // Send typing action
            if let Err(e) = bot_clone.send_chat_action(chat_id, ChatAction::Typing).await {
                log::error!("Failed to send chat action: {}", e);
                break;
            }
            // Wait for 5 seconds or until notified
            let _ = time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Generate the AI response
    const MODEL: &str = "llama3.2:1b";
    let res = ollama
        .generate(GenerationRequest::new(MODEL.to_string(), formatted_prompt))
        .await;

    // Signal the typing indicator task to stop
    let _ = tx.send(true);

    match res {
        Ok(response) => {
            bot.send_message(msg.chat.id, response.response)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Error: {}", e))
                .await?;
        }
    }

    Ok(())
}

/// Handles messages that contain captions (e.g., images with captions)
async fn handle_caption(bot: Bot, msg: Message, ollama: Ollama) -> ResponseResult<()> {
    // Check if it's a /llama command with a caption
    if let Some(caption) = msg.caption() {
        // Check if there is a command
        if let Ok(cmd) = Command::parse(caption, &bot.get_me().await?.user.username.unwrap()) {
            answer(bot, msg, cmd, ollama).await?;
        }
    }
    Ok(())
}

/// Extracts and formats the prompt based on the message content
async fn extract_prompt(msg: &Message, cmd_prompt: Option<String>) -> String {
    let mut prompt = String::new();

    // If the message is a reply, include the replied message's content
    if let Some(reply) = &msg.reply_to_message() {
        if let Some(text) = &reply.text() {
            // Remove any command prefix
            let text = text.trim_start_matches('/');
            let text = text.split_once(' ').map(|x| x.1).unwrap_or(text);

            prompt.push_str(text);
            prompt.push_str("\n\n");
        } else if let Some(caption) = reply.caption() {
             // Remove any command prefix
             let caption = caption.trim_start_matches('/');
             let caption = caption.split_once(' ').map(|x| x.1).unwrap_or(caption);
 
            prompt.push_str(caption);
            prompt.push_str("\n\n");
        }
    }

    // Append the current prompt (from command or caption)
    if let Some(cp) = cmd_prompt {
        prompt.push_str(&cp);
    }

    prompt
}