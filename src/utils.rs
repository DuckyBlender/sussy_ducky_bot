use comfyui_rs::ClientError;
use enum_iterator::Sequence;
use log::info;
use ollama_rs::{models::create::CreateModelRequest, Ollama};
use std::collections::HashMap;

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
    Brainrot,   // brainrot-llama3
    StableCode, // nuaimat/stablecode:3b
    Json,       // adrienbrault/nous-hermes2pro-llama3-8b:q4_K_M

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

    pub fn return_groq() -> Vec<ModelType> {
        vec![ModelType::Mixtral, ModelType::LLAMA3]
    }

    pub fn return_custom() -> Vec<ModelType> {
        vec![
            ModelType::Caveman,
            ModelType::Racist,
            ModelType::Furry,
            ModelType::Brainrot,
        ]
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ModelType::Bielik => {
                write!(f, "mwiewior/bielik:7b-instruct-v0.1.Q4_K_M.gguf")
            } // for ollama
            ModelType::Json => write!(f, "adrienbrault/nous-hermes2pro-llama3-8b:q4_K_M"), // for ollama. we use this model because it supports JSON output
            ModelType::StableCode => write!(f, "nuaimat/stablecode:3b"), // for ollama
            ModelType::Brainrot => write!(f, "brainrot-llama3"),         // for ollama
            ModelType::Moondream => write!(f, "moondream:1.8b-v2-q4_K_M"), // for ollama
            ModelType::Phi3 => write!(f, "phi3:3.8b-mini-instruct-4k-q4_K_M"), // for ollama
            ModelType::GPT4 => write!(f, "gpt-4-turbo"),                 // for perplexity.ai
            ModelType::Uncensored => write!(f, "dolphin-llama3:8b-v2.9-q4_K_M"), // for ollama
            // ModelType::LLAMA3 => write!(f, "llama3:8b-instruct-q4_K_M"),    // for ollama
            ModelType::LLAMA3 => write!(f, "llama3-70b-8192"), // for groq
            ModelType::Caveman => write!(f, "caveman-llama3"), // for ollama
            ModelType::Racist => write!(f, "duckyblender/racist-phi3"), // for ollama
            ModelType::Furry => write!(f, "furry-llama3"),     // for ollama
            ModelType::TinyLlama => write!(f, "tinyllama:1.1b-chat-v0.6-q8_0"), // for ollama
            ModelType::Lobotomy => write!(f, "qwen:0.5b-chat-v1.5-q2_K"), // ollama
            // ModelType::Mixtral => write!(f, "mixtral-8x7b-instruct"), // for perplexity.ai
            ModelType::Mixtral => write!(f, "mixtral-8x7b-32768"), // for groq.com
            ModelType::Online => write!(f, "sonar-medium-online"), // for perplexity.ai
            ModelType::StableLM2 => write!(f, "stablelm2"),        // for ollama
            ModelType::Dalle3 => write!(f, "dall-e-3"),            // for openai
            ModelType::SDXLTurbo => write!(f, "sdxl-turbo"),       // for comfyui
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

pub async fn process_image_generation(
    prompt: &str,
    model: &ModelType,
) -> Result<HashMap<String, Vec<u8>>, ClientError> {
    let client = comfyui_rs::Client::new("127.0.0.1:8188");
    match model {
        &ModelType::SDXLTurbo => {
            let json_prompt =
                serde_json::from_str(include_str!("../comfyui-rs/jsons/sdxl_turbo_api.json"))
                    .unwrap();
            let mut json_prompt: serde_json::Value = json_prompt;
            json_prompt["6"]["inputs"]["text"] = serde_json::Value::String(prompt.to_string());
            json_prompt["13"]["inputs"]["noise_seed"] =
                serde_json::Value::Number(serde_json::Number::from(rand::random::<u64>()));
            let images = client.get_images(json_prompt).await;
            if images.is_err() {
                return Err(images.err().unwrap());
            }
            Ok(images.unwrap())
        }
        _ => Err(ClientError::CustomError("Model not found".to_string())),
    }
}
