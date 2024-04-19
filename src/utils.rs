use std::collections::HashMap;

use comfyui_rs::ClientError;
use enum_iterator::Sequence;
use log::info;
use ollama_rs::Ollama;

#[derive(Debug, PartialEq, Sequence)]
pub enum ModelType {
    // Ollama (text)
    MistralCaveman, // caveman-mistral (custom model)
    MistralRacist,  // racist-mistral (custom model)
    MistralFurry,
    Mistral,        // dolphin-mistral
    TinyLlama,      // tinyllama
    Lobotomy,       // qwen:0.5b-chat-v1.5-q2_K
    Solar,          // solar
    StableLM2,      // stablelm2
    CodeGemma,      // codegemma
    Bielik,         // bielik
    LLAMA3,         // llama3


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
    Gemma,   // gemma-7b-it

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
            ModelType::Mistral,
            ModelType::TinyLlama,
            ModelType::Lobotomy,
            ModelType::Solar,
            ModelType::StableLM2,
            ModelType::CodeGemma,
            ModelType::Bielik,
            // ModelType::LLAMA3,
        ]
    }

    pub fn return_comfyui() -> Vec<ModelType> {
        vec![ModelType::SDXLTurbo]
    }

    // pub fn return_perplexity() -> Vec<ModelType> {
    //     vec![ModelType::Online]
    // }

    pub fn return_groq() -> Vec<ModelType> {
        vec![ModelType::Mixtral, ModelType::Gemma, ModelType::LLAMA3]
    }

    pub fn return_custom() -> Vec<ModelType> {
        vec![ModelType::MistralCaveman, ModelType::MistralRacist, ModelType::MistralFurry]
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ModelType::Bielik => {
                write!(f, "mwiewior/bielik:7b-instruct-v0.1.Q5_K_M.gguf")
            } // for ollama
            ModelType::GPT4 => write!(f, "gpt-4-turbo"), // for perplexity.ai
            // ModelType::LLAMA3 => write!(f, "llama3:8b-instruct-q5_K_M"),    // for ollama
            ModelType::LLAMA3 => write!(f, "llama3-70b-8192"), // for groq
            ModelType::Mistral => write!(f, "dolphin-mistral:7b-v2.8-q5_K_M"), // for ollama
            ModelType::MistralCaveman => write!(f, "caveman-mistral"), // for ollama
            ModelType::MistralRacist => write!(f, "racist-mistral"), // for ollama
            ModelType::MistralFurry => write!(f, "furry-mistral"), // for ollama
            ModelType::TinyLlama => write!(f, "tinyllama:1.1b-chat-v0.6-q8_0"), // for ollama
            ModelType::Lobotomy => write!(f, "qwen:0.5b-chat-v1.5-q2_K"), // ollama
            // ModelType::Mixtral => write!(f, "mixtral-8x7b-instruct"), // for perplexity.ai
            ModelType::Mixtral => write!(f, "mixtral-8x7b-32768"), // for groq.com
            ModelType::Gemma => write!(f, "gemma-7b-it"),          // for groq.com
            ModelType::CodeGemma => write!(f, "codegemma"),        // for ollama
            ModelType::Online => write!(f, "sonar-medium-online"), // for perplexity.ai
            ModelType::Solar => write!(f, "solar"),                // for ollama
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

    // Create the model eg: ollama create caveman-mistral -f ./custom_models/caveman/Modelfile
    // Todo: change this to use ollama-rs (i tried, the path is not working for some reason)
    for model in custom_models.iter() {
        let model = model.to_string();
        info!("Creating custom model: {}", model);
        let _ = std::process::Command::new("ollama")
            .arg("create")
            .arg(&model)
            .arg("-f")
            .arg(format!("./custom_models/{}/Modelfile", model))
            .output()
            .expect("Failed to create custom model");
        info!("Model {} created!", model);
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
