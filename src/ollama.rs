use std::error::Error;

use log::{error, info, warn};

use crate::commands::Command;

pub async fn verify_ollama_models() -> Result<(), Box<dyn Error + Send + Sync>> {
    let downloaded_models = list_ollama_models().await?;
    let bot_local_models = Command::local_models();
    info!("Downloaded models: {:?}", downloaded_models);
    info!("Bot models: {:?}", bot_local_models);
    // Check if all models are available
    for model in &bot_local_models {
        if downloaded_models.contains(model) {
            info!("Model {} is available", model);
        } else {
            warn!("Model {} is not available, pulling...", model);
            pull_ollama_model(model.to_string()).await?;
        }
    }
    Ok(())
}

async fn pull_ollama_model(model: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    let reqwest = reqwest::Client::new();
    let res = reqwest
        .post("http://localhost:11434/api/pull")
        .json(&serde_json::json!({ "model": model, "stream": false }))
        .send()
        .await;
    if let Err(e) = res {
        error!("Failed to pull model {}: {}", model, e);
        return Err(Box::new(e));
    }
    info!("Model {} has been pulled", model);
    Ok(())
}

async fn list_ollama_models() -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let reqwest = reqwest::Client::new();
    let res = reqwest.get("http://localhost:11434/api/tags").send().await;
    if let Err(e) = res {
        error!(
            "Failed to list models. Do you have ollama downloaded and running? {}",
            e
        );
        return Err(Box::new(e));
    }
    let json = res.unwrap().json::<serde_json::Value>().await;
    if let Err(e) = json {
        error!("Failed to parse JSON: {}", e);
        return Err(Box::new(e));
    }
    let json = json.unwrap();

    let models_array = json["models"].as_array();

    if models_array.is_none() {
        // No models, return empty vec
        return Ok(vec![]);
    }

    let models = models_array.unwrap();
    let mut model_names = Vec::new();
    for model in models {
        let name = model["name"].as_str();
        if name.is_none() {
            error!("Model name not found in JSON: {:?}", model);
            continue;
        }
        model_names.push(name.unwrap().to_string());
    }

    Ok(model_names)
}
