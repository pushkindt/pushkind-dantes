from __future__ import annotations

import numpy as np
from fastembed import TextEmbedding
from numpy.typing import NDArray

_model: TextEmbedding | None = None


def _get_model() -> TextEmbedding:
    global _model
    if _model is None:
        _model = TextEmbedding(
            "intfloat/multilingual-e5-large"
        )
    return _model


def normalize_embedding(embedding: NDArray[np.float32 | np.float64]) -> list[float]:
    """Normalize an embedding vector."""
    norm = np.linalg.norm(embedding)
    if norm == 0.0:
        return embedding
    normalized_embedding = embedding / norm
    return normalized_embedding.tolist()


def prompt_to_embedding(prompt: str) -> list[float]:
    """Convert a text prompt to an embedding vector."""
    model = _get_model()
    embedding = next(model.embed([prompt]))
    return normalize_embedding(embedding)


if __name__ == "__main__":
    import sys

    text = " ".join(sys.argv[1:])
    print(prompt_to_embedding(text))
