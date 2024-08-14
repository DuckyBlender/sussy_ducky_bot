use enum_iterator::Sequence;
use log::{error, info};
use ollama_rs::Ollama;

#[derive(Debug, PartialEq, Sequence)]
pub enum ModelType {
    // Ollama (text)
    Uncensored,
    Caveman,
    Racist,
    Lobotomy,
    Phi3,
    Json,
    PolishLobotomy,
    Gemma2,
    GLM4,

    // Fal.ai
    SDXLTurbo,
    SDXL,
    StableAudio,
    FluxShnell,

    // Perplexity (online)
    Online,

    // Groq (fast LLMs, free)
    LLAMA3,
    Rushify,

    // OpenAI (best LLMs, paid)
    // GPT4o,
    GPT4oMini,
}

impl ModelType {
    // pub fn return_all() -> Vec<ModelType> {
    //     enum_iterator::all::<ModelType>().collect()
    // }

    pub fn return_ollama() -> Vec<ModelType> {
        vec![
            ModelType::Lobotomy,
            ModelType::Uncensored,
            ModelType::Phi3,
            ModelType::Racist,
            ModelType::Json,
            ModelType::PolishLobotomy,
            ModelType::Caveman,
            ModelType::Gemma2,
            ModelType::GLM4,
            ModelType::Caveman,
        ]
    }

    pub fn return_fal() -> Vec<ModelType> {
        vec![
            ModelType::SDXLTurbo,
            ModelType::SDXL,
            ModelType::StableAudio,
            ModelType::FluxShnell,
        ]
    }

    pub fn return_openai() -> Vec<ModelType> {
        vec![ModelType::GPT4oMini]
    }

    pub fn return_perplexity() -> Vec<ModelType> {
        vec![ModelType::Online]
    }

    pub fn return_groq() -> Vec<ModelType> {
        vec![ModelType::LLAMA3, ModelType::Rushify]
    }

    pub fn owner_only() -> Vec<ModelType> {
        vec![
            ModelType::Online,
            ModelType::StableAudio,
        ]
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ModelType::PolishLobotomy => write!(f, "duckyblender/polish-lobotomy"),
            ModelType::Json => write!(f, "phi3:3.8b-mini-4k-instruct-q4_K_M"),
            ModelType::Phi3 => write!(f, "phi3:3.8b-mini-4k-instruct-q4_K_M"),
            ModelType::GPT4oMini => write!(f, "gpt-4o-mini"),
            ModelType::Uncensored => write!(f, "mannix/llama3.1-8b-abliterated"),
            ModelType::LLAMA3 => write!(f, "llama-3.1-70b-versatile"),
            ModelType::Caveman => write!(f, "llama-3.1-70b-versatile"),
            ModelType::Racist => write!(f, "duckyblender/racist-phi3"),
            ModelType::Lobotomy => write!(f, "qwen:0.5b-chat-v1.5-q2_K"),
            ModelType::Online => write!(f, "llama-3-sonar-large-32k-online"),
            ModelType::Gemma2 => write!(f, "gemma2:9b-instruct-q4_K_M"),
            ModelType::GLM4 => write!(f, "glm4:9b-chat-q4_K_M"),
            ModelType::SDXLTurbo => write!(f, "fast-lightning-sdxl"),
            ModelType::SDXL => write!(f, "fast-sdxl"),
            ModelType::StableAudio => write!(f, "stable-audio"),
            ModelType::FluxShnell => write!(f, "flux/schnell"),
            ModelType::Rushify => write!(f, "llama-3.1-8b-instant"),
        }
    }
}

pub async fn setup_models() {
    // Get all of the ollama models
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
                error!("Error downloading/verifying model: {}", e);
            }
        }
    }
}
