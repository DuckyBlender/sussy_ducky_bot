use crate::utils::ModelType;

pub struct GenerationError {
    pub message: String,
}

pub struct GenerationResponse {
    pub message: String,
}

pub struct GenerationRequest {
    pub prompt: Option<String>,
    pub image: Option<String>,
    pub model: ModelType,
}
