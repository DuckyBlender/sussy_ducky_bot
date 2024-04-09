use enum_iterator::Sequence;
use std::fmt;
use teloxide::types::Message;

#[derive(Debug, PartialEq, Sequence)]
pub enum ModelType {
    // Ollama (text)
    MistralCaveman, // caveman-mistral (custom model)
    MistralRacist,  // racist-mistral (custom model)
    Mistral,        // dolphin-mistral
    TinyLlama,      // tinyllama
    Lobotomy,       // qwen:0.5b-chat-v1.5-q2_K
    Solar,          // solar

    // Ollama (image recognition)
    // LLaVa7B,  // llava
    // LLaVa13B, // llava:13b

    // Perplexity (online)
    // Mixtral, // mixtral-8x7b-instruct
    Online, // pplx-7b-online

    // Groq
    Mixtral, // mixtral-8x7b-32768
    Gemma,   // gemma-7b-it
}

impl ModelType {
    // pub fn return_all() -> Vec<ModelType> {
    //     enum_iterator::all::<ModelType>().collect()
    // }

    pub fn return_ollama() -> Vec<ModelType> {
        vec![
            ModelType::Mistral,
            ModelType::TinyLlama,
            ModelType::Lobotomy,
            ModelType::Solar,
        ]
    }

    // pub fn return_perplexity() -> Vec<ModelType> {
    //     vec![ModelType::Online]
    // }

    // pub fn return_groq() -> Vec<ModelType> {
    //     vec![ModelType::Mixtral, ModelType::Gemma]
    // }

    pub fn return_custom() -> Vec<ModelType> {
        vec![ModelType::MistralCaveman, ModelType::MistralRacist]
    }
}

impl fmt::Display for ModelType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ModelType::Mistral => write!(f, "dolphin-mistral"), // for ollama
            ModelType::MistralCaveman => write!(f, "caveman-mistral"), // for ollama
            ModelType::MistralRacist => write!(f, "racist-mistral"), // for ollama
            ModelType::TinyLlama => write!(f, "tinyllama:1.1b-chat-v0.6-q8_0"),     // for ollama
            ModelType::Lobotomy => write!(f, "qwen:0.5b-chat-v1.5-q2_K"), // ollama
            // ModelType::Mixtral => write!(f, "mixtral-8x7b-instruct"), // for perplexity.ai
            ModelType::Mixtral => write!(f, "mixtral-8x7b-32768"), // for groq.com
            ModelType::Gemma => write!(f, "gemma-7b-it"),          // for groq.com
            ModelType::Online => write!(f, "sonar-medium-online"), // for perplexity.ai
            ModelType::Solar => write!(f, "solar"),                // for ollama
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
