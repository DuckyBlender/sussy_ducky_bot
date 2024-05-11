use enum_iterator::Sequence;
use log::info;
use ollama_rs::{models::create::CreateModelRequest, Ollama};
use std::collections::HashMap;
use teloxide::prelude::Requester;
use teloxide::RequestError;
use teloxide::{types::Message, utils::command::BotCommands, Bot};

#[derive(Debug, PartialEq, Sequence)]
pub enum ModelType {
    // Ollama (text)
    Uncensored, // dolphin-llama3
    Caveman,    // caveman-llama3 (custom model)
    Racist,     // duckyblender/racist-phi3
    Furry,      // furry-llama3 (custom model)
    TinyLlama,  // tinyllama
    Lobotomy,   // qwen:0.5b-chat-v1.5-q2_K
    StableLM2,  // stablelm2
    Bielik,     // bielik
    Phi3,       // phi3:3.8b-mini-instruct-4k-q4_K_M
    Moondream,  // moondream:1.8b-v2-q4_K_M
    StableCode, // nuaimat/stablecode:3b
    Json,       // phi3:3.8b-mini-instruct-4k-q4_K_M

    // Comfyui (image generation)
    SDXLTurbo,

    // Ollama (image recognition)
    // LLaVa7B,  // llava
    // LLaVa13B, // llava:13b

    // Perplexity (online)
    // Mixtral, // mixtral-8x7b-instruct
    Online, // pplx-7b-online

    // Groq (fast LLMs, free)
    Mixtral, // mixtral-8x7b-32768
    LLAMA3,  // llama3

    // OpenAI (best LLMs, paid)
    GPT4,
    Dalle3,

    // Bedrock
    AmazonTitanText,
    AmazonTitanTextLite,
    AmazonTitanImage,
    AmazonTitanImageVariation,
    CommandR,
    CommandRPlus,
    Claude3,
}

impl ModelType {
    // pub fn return_all() -> Vec<ModelType> {
    //     enum_iterator::all::<ModelType>().collect()
    // }

    pub fn return_ollama() -> Vec<ModelType> {
        vec![
            ModelType::TinyLlama,
            ModelType::Lobotomy,
            ModelType::StableLM2,
            ModelType::Bielik,
            ModelType::Uncensored,
            ModelType::Phi3,
            ModelType::Moondream,
            ModelType::StableCode,
            ModelType::Racist,
            ModelType::Json,
        ]
    }

    pub fn return_comfyui() -> Vec<ModelType> {
        vec![ModelType::SDXLTurbo]
    }

    // pub fn return_perplexity() -> Vec<ModelType> {
    //     vec![ModelType::Online]
    // }

    pub fn return_bedrock() -> Vec<ModelType> {
        vec![
            ModelType::AmazonTitanText,
            ModelType::AmazonTitanTextLite,
            ModelType::CommandR,
            ModelType::CommandRPlus,
            ModelType::AmazonTitanImage,
            ModelType::AmazonTitanImageVariation,
        ]
    }

    pub fn return_groq() -> Vec<ModelType> {
        vec![ModelType::Mixtral, ModelType::LLAMA3]
    }

    pub fn return_custom() -> Vec<ModelType> {
        vec![ModelType::Caveman, ModelType::Racist, ModelType::Furry]
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ModelType::Bielik => {
                write!(f, "mwiewior/bielik:7b-instruct-v0.1.Q4_K_M.gguf")
            } // for ollama
            ModelType::Json => write!(f, "phi3:3.8b-mini-instruct-4k-q4_K_M"), // for ollama
            ModelType::StableCode => write!(f, "nuaimat/stablecode:3b"),       // for ollama
            ModelType::Moondream => write!(f, "moondream:1.8b-v2-q4_K_M"),     // for ollama
            ModelType::Phi3 => write!(f, "phi3:3.8b-mini-instruct-4k-q4_K_M"), // for ollama
            ModelType::GPT4 => write!(f, "gpt-4-turbo"),                       // for perplexity.ai
            ModelType::Uncensored => write!(f, "dolphin-llama3:8b-v2.9-q4_K_M"), // for ollama
            ModelType::LLAMA3 => write!(f, "llama3-70b-8192"),                 // for groq
            ModelType::Caveman => write!(f, "caveman-llama3"),                 // for ollama
            ModelType::Racist => write!(f, "duckyblender/racist-phi3"),        // for ollama
            ModelType::Furry => write!(f, "furry-llama3"),                     // for ollama
            ModelType::TinyLlama => write!(f, "tinyllama:1.1b-chat-v0.6-q8_0"), // for ollama
            ModelType::Lobotomy => write!(f, "qwen:0.5b-chat-v1.5-q2_K"),      // ollama
            ModelType::Mixtral => write!(f, "mixtral-8x7b-32768"),             // for groq.com
            ModelType::Online => write!(f, "llama-3-sonar-small-32k-online"),  // for perplexity.ai
            ModelType::StableLM2 => write!(f, "stablelm2"),                    // for ollama
            ModelType::Dalle3 => write!(f, "dall-e-3"),                        // for openai
            ModelType::SDXLTurbo => write!(f, "sdxl-turbo"),                   // for comfyui
            ModelType::AmazonTitanText => write!(f, "amazon.titan-text-express-v1"), // for bedrock
            ModelType::AmazonTitanTextLite => write!(f, "amazon.titan-text-lite-v1"), // for bedrock
            ModelType::CommandR => write!(f, "cohere.command-r-v1:0"),         // for bedrock
            ModelType::CommandRPlus => write!(f, "cohere.command-r-plus-v1:0"), // for bedrock
            ModelType::AmazonTitanImage => write!(f, "amazon.titan-image-generator-v1"), // for bedrock
            ModelType::AmazonTitanImageVariation => write!(f, "amazon.titan-image-generator-v1"), // for bedrock
            ModelType::Claude3 => write!(f, "anthropic.claude-3-sonnet-20240229-v1:0"), // for bedrock
        }
    }
}

pub async fn setup_models() {
    // Get all of the ollama models
    let custom_models = ModelType::return_custom();
    let ollama_models = ModelType::return_ollama();

    let ollama = Ollama::default();

    // Download all of the ollama models
    for model in ollama_models.iter() {
        let model = model.to_string();
        info!("Downloading/verifying model: {}", model);
        let res = ollama.pull_model(model.clone(), false).await;
        match res {
            Ok(_) => {
                info!("Model {} downloaded/verified!", model);
            }
            Err(e) => {
                info!("Error downloading/verifying model: {}", e);
            }
        }
    }

    // Create the model eg: ollama create caveman-llama3 -f ./custom_models/caveman/Modelfile
    for model in custom_models.iter() {
        let model = model.to_string();
        info!("Creating custom model: {}", model);
        let modelfile = format!("./custom_models/{}/Modelfile", model);
        let modelfile = std::fs::read_to_string(modelfile).unwrap();
        let create_model_request = CreateModelRequest::modelfile(model.clone(), modelfile);
        let res = ollama.create_model(create_model_request).await;
        match res {
            Ok(_) => {
                info!("Model {} created!", model);
            }
            Err(e) => {
                info!("Error creating custom model: {}", e);
            }
        }
    }
}

pub async fn get_prompt(msg: Message) -> Option<String> {
    // get_prompt only returns the prompt (stripping out the command and bot mention)
    // if no prompt was found, look at the reply
    let prompt = if let Some(reply) = msg.reply_to_message() {
        reply.text().clone()
    } else {
        msg.text().clone()
    };

    if prompt.is_none() {
        return None;
    }

    // Parse the prompt
    let prompt = prompt.unwrap();
    let trimmed_prompt = prompt.trim_start();
    // If there's a / in the first word of the prompt, remove it
    let trimmed_prompt = if trimmed_prompt.starts_with('/') {
        trimmed_prompt.splitn(2, ' ').nth(1).unwrap()
    } else {
        trimmed_prompt
    };

    Some(trimmed_prompt.to_string())
}

pub async fn delete_both_delay(bot: Bot, msg1: Message, msg2: Message) -> Result<(), RequestError> {
    // Wait x seconds
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    // Deleting the messages
    bot.delete_message(msg1.chat.id, msg1.id).await?;
    bot.delete_message(msg2.chat.id, msg2.id).await?;

    Ok(())
}
