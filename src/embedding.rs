use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("Empty embedding")]
    EmptyEmbedding,
    #[error("Load model error")]
    LoadModel,
    #[error("Embedding generation error")]
    EmbdeddingGeneration,
}

pub trait PromptEmbedding {
    fn prompt(&self) -> String;
    fn embeddings(&self) -> Result<Vec<f32>, EmbeddingError> {
        prompt_to_embedding(&self.prompt())
    }
}

/// Convert a text prompt into an embedding vector.
pub fn prompt_to_embedding(prompt: &str) -> Result<Vec<f32>, EmbeddingError> {
    let options = InitOptions::new(EmbeddingModel::ParaphraseMLMpnetBaseV2);
    let mut model = TextEmbedding::try_new(options).map_err(|_| EmbeddingError::LoadModel)?;
    let embeddings = model
        .embed(vec![prompt], None)
        .map_err(|_| EmbeddingError::EmbdeddingGeneration)?;
    embeddings
        .into_iter()
        .next()
        .ok_or(EmbeddingError::EmptyEmbedding)
}
