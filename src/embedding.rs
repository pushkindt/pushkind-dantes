use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Convert a text prompt into an embedding vector.
pub fn prompt_to_embedding(prompt: &str) -> Result<Vec<f32>, fastembed::Error> {
    let options = InitOptions::new(EmbeddingModel::ParaphraseMLMpnetBaseV2);
    let mut model = TextEmbedding::try_new(options)?;
    let embeddings = model.embed(vec![prompt], None)?;
    embeddings.into_iter().next().ok_or_else(|| "empty embedding".into())
}
