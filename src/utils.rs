use enum_iterator::Sequence;
use log::info;


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

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ModelType::Mistral => write!(f, "dolphin-mistral"), // for ollama
            ModelType::MistralCaveman => write!(f, "caveman-mistral"), // for ollama
            ModelType::MistralRacist => write!(f, "racist-mistral"), // for ollama
            ModelType::TinyLlama => write!(f, "tinyllama:1.1b-chat-v0.6-q8_0"), // for ollama
            ModelType::Lobotomy => write!(f, "qwen:0.5b-chat-v1.5-q2_K"), // ollama
            // ModelType::Mixtral => write!(f, "mixtral-8x7b-instruct"), // for perplexity.ai
            ModelType::Mixtral => write!(f, "mixtral-8x7b-32768"), // for groq.com
            ModelType::Gemma => write!(f, "gemma-7b-it"),          // for groq.com
            ModelType::Online => write!(f, "sonar-medium-online"), // for perplexity.ai
            ModelType::Solar => write!(f, "solar"),                // for ollama
        }
    }
}

pub fn setup_models() {
    // Get all of the ollama models
    let custom_models = ModelType::return_custom();
    let ollama_models = ModelType::return_ollama();

    // Download all of the ollama models
    for model in ollama_models.iter() {
        let model = model.to_string();
        info!("Downloading/verifying model: {}", model);
        let _ = std::process::Command::new("ollama")
            .arg("pull")
            .arg(&model)
            .output()
            .expect("Failed to download model");
        info!("Model {} downloaded/verified!", model);
    }

    // Create the model eg: ollama create caveman-mistral -f ./custom_models/caveman/Modelfile
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
