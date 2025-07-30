from __future__ import annotations

from fastembed import TextEmbedding

_model: TextEmbedding | None = None


def _get_model() -> TextEmbedding:
    global _model
    if _model is None:
        _model = TextEmbedding(
            "sentence-transformers/paraphrase-multilingual-mpnet-base-v2"
        )
    return _model


def prompt_to_embedding(prompt: str) -> list[float]:
    """Convert a text prompt to an embedding vector."""
    model = _get_model()
    embedding = next(model.embed([prompt]))
    return embedding.tolist()


if __name__ == "__main__":
    import sys

    text = " ".join(sys.argv[1:])
    print(prompt_to_embedding(text))
