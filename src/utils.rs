use log::info;
use std::fmt;
use teloxide::types::Message;

pub enum ModelType {
    // Ollama (local)
    MistralCaveman,   // caveman-mistral (custom model)
    MistralRacist,    // racist-mistral (custom model)
    Mistral,          // dolphin-mistral
    TinyLlama,        // tiny-llama
    MistralGreentext, // greentext-mistral
    Lobotomy,         // tinyllama:1.1b-chat-v0.6-q2_K

    // Perplexity (online)
    Mixtral, // mixtral-8x7b-instruct
    Online,  // pplx-7b-online
}

impl fmt::Display for ModelType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ModelType::Mistral => write!(f, "dolphin-mistral"),
            ModelType::MistralCaveman => write!(f, "caveman-mistral"),
            ModelType::MistralRacist => write!(f, "racist-mistral"),
            ModelType::MistralGreentext => write!(f, "greentext-mistral"),
            ModelType::TinyLlama => write!(f, "tinyllama"),
            ModelType::Lobotomy => write!(f, "tinyllama:1.1b-chat-v0.6-q2_K"),
            ModelType::Mixtral => write!(f, "mixtral-8x7b-instruct"),
            ModelType::Online => write!(f, "pplx-7b-online"),
        }
    }
}

pub fn parse_command(msg: Message) -> (Option<String>, Option<String>) {
    let bot_name = std::env::var("BOT_NAME").unwrap_or("sussy_ducky_bot".to_string());
    let text = msg.text().unwrap_or("");
    let mut iter = text.splitn(2, ' ');
    let command = iter.next().map(std::string::ToString::to_string);
    let args = iter.next().map(std::string::ToString::to_string);

    match &command {
        Some(command) if command.ends_with(&bot_name) => {
            let command = &command[..command.len() - bot_name.len() - 1]; // -1 to remove @
            (Some(command.to_string()), args)
        }
        Some(command) if !command.contains('@') => (Some(command.to_string()), args),
        _ => (None, None),
    }
}

// Remove the command from the message. Supports /command and /command@botname
pub fn remove_prefix(msg: Message) -> String {
    let bot_name = std::env::var("BOT_NAME").unwrap_or("sussy_ducky_bot".to_string());
    let text = msg.text().unwrap_or("");
    let mut iter = text.splitn(2, ' ');
    let command = iter.next().unwrap_or("");
    let args = iter.next().unwrap_or("");
    if command.ends_with(&bot_name) {
        info!("Removed prefix: {}", args);
        args.to_string()
    } else {
        info!("Removed prefix: {}", text);
        text.to_string()
    }
}
