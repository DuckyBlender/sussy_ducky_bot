use enum_iterator::Sequence;
use log::{error, info};
use ollama_rs::Ollama;

#[derive(Debug, PartialEq, Sequence)]
pub enum ModelType {
    // Ollama (text)
    Uncensored,
    Caveman,
    Racist,
    TinyLlama,
    Lobotomy,
    StableLM2,
    Phi3,
    Json,
    PolishLobotomy,
    TinyStories,
    Gemma2,
    GLM4,

    // Fal.ai
    SDXLTurbo,
    SDXL,
    StableAudio,
    AuraFlow,
    FluxShnell,

    // Perplexity (online)
    Online,

    // Groq (fast LLMs, free)
    LLAMA3,
    Rushify,

    // OpenAI (best LLMs, paid)
    // GPT4o,
    GPT4oMini,
    Dalle3,

    // Bedrock
    AmazonTitanText,
    AmazonTitanTextLite,
    AmazonTitanImage,
    AmazonTitanImageVariation,
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
            ModelType::Uncensored,
            ModelType::Phi3,
            ModelType::Racist,
            ModelType::Json,
            ModelType::PolishLobotomy,
            ModelType::TinyStories,
            ModelType::Caveman,
            ModelType::Gemma2,
            ModelType::GLM4,
            ModelType::TinyStories,
            ModelType::Caveman,
        ]
    }

    pub fn return_fal() -> Vec<ModelType> {
        vec![
            ModelType::SDXLTurbo,
            ModelType::SDXL,
            ModelType::StableAudio,
            ModelType::AuraFlow,
            ModelType::FluxShnell,
        ]
    }

    pub fn return_openai() -> Vec<ModelType> {
        vec![ModelType::GPT4oMini, ModelType::Dalle3]
    }

    pub fn return_perplexity() -> Vec<ModelType> {
        vec![ModelType::Online]
    }

    pub fn return_bedrock() -> Vec<ModelType> {
        vec![
            ModelType::AmazonTitanText,
            ModelType::AmazonTitanTextLite,
            ModelType::AmazonTitanImage,
            ModelType::AmazonTitanImageVariation,
            ModelType::Claude3,
        ]
    }

    pub fn return_groq() -> Vec<ModelType> {
        vec![ModelType::LLAMA3, ModelType::Rushify]
    }

    pub fn owner_only() -> Vec<ModelType> {
        vec![
            ModelType::Online,
            ModelType::AmazonTitanText,
            ModelType::AmazonTitanTextLite,
            ModelType::AmazonTitanImage,
            ModelType::AmazonTitanImageVariation,
            ModelType::Claude3,
            ModelType::StableAudio,
            ModelType::AuraFlow,
        ]
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ModelType::PolishLobotomy => write!(f, "duckyblender/polish-lobotomy"), // for ollama
            ModelType::TinyStories => write!(f, "duckyblender/tinystories-656k"),   // for ollama
            ModelType::Json => write!(f, "phi3:3.8b-mini-4k-instruct-q4_K_M"),      // for ollama
            ModelType::Phi3 => write!(f, "phi3:3.8b-mini-4k-instruct-q4_K_M"),      // for ollama
            ModelType::GPT4oMini => write!(f, "gpt-4o-mini"),                       // for openai
            ModelType::Uncensored => write!(f, "mannix/llama3.1-8b-abliterated"),   // for ollama
            ModelType::LLAMA3 => write!(f, "llama-3.1-70b-versatile"),              // for groq
            ModelType::Caveman => write!(f, "caveman-llama3"),                      // for ollama
            ModelType::Racist => write!(f, "duckyblender/racist-phi3"),             // for ollama
            ModelType::TinyLlama => write!(f, "tinyllama:1.1b-chat-v0.6-q8_0"),     // for ollama
            ModelType::Lobotomy => write!(f, "qwen:0.5b-chat-v1.5-q2_K"),           // ollama
            ModelType::Online => write!(f, "llama-3-sonar-large-32k-online"), // for perplexity.ai
            ModelType::StableLM2 => write!(f, "stablelm2"),                   // for ollama
            ModelType::Dalle3 => write!(f, "dall-e-3"),                       // for openai
            ModelType::AmazonTitanText => write!(f, "amazon.titan-text-express-v1"), // for bedrock
            ModelType::AmazonTitanTextLite => write!(f, "amazon.titan-text-lite-v1"), // for bedrock
            ModelType::AmazonTitanImage => write!(f, "amazon.titan-image-generator-v1"), // for bedrock
            ModelType::AmazonTitanImageVariation => write!(f, "amazon.titan-image-generator-v1"), // for bedrock
            ModelType::Claude3 => write!(f, "anthropic.claude-3-5-sonnet-20240620-v1:0"), // for bedrock
            ModelType::Gemma2 => write!(f, "gemma2:9b-instruct-q4_K_M"), // for ollama
            ModelType::GLM4 => write!(f, "glm4:9b-chat-q4_K_M"),         // for ollama
            ModelType::SDXLTurbo => write!(f, "fast-lightning-sdxl"),    // for fal.ai
            ModelType::SDXL => write!(f, "fast-sdxl"),                   // for fal.ai
            ModelType::StableAudio => write!(f, "stable-audio"),         // for fal.ai
            ModelType::AuraFlow => write!(f, "aura-flow"),               // for fal.ai
            ModelType::FluxShnell => write!(f, "flux/schnell"),          // for fal.ai
            ModelType::Rushify => write!(f, "llama-3.1-8b-instant"),     // for groq
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
