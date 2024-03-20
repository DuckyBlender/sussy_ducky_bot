use log::info;
use std::fmt;
use teloxide::types::Message;

pub enum ModelType {
    // Ollama (text)
    MistralCaveman,   // caveman-mistral (custom model)
    MistralRacist,    // racist-mistral (custom model)
    Mistral,          // dolphin-mistral
    TinyLlama,        // tinyllama
    MistralGreentext, // greentext-mistral
    Lobotomy,         // qwen:0.5b-chat-v1.5-q2

    // Ollama (image recognition)
    // LLaVa7B,  // llava
    // LLaVa13B, // llava:13b

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
            ModelType::Lobotomy => write!(f, "qwen:0.5b-chat-v1.5-q2_K"),
            ModelType::Mixtral => write!(f, "mixtral-8x7b-instruct"),
            ModelType::Online => write!(f, "pplx-7b-online"),
            // ModelType::LLaVa7B => write!(f, "llava"),
            // ModelType::LLaVa13B => write!(f, "llava:13b"),
        }
    }
}

pub fn parse_command(msg: Message, bot_name: String) -> (Option<String>, Option<String>) {
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
pub fn remove_prefix(msg: Message, bot_name: String) -> String {
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
