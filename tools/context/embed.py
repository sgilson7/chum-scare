#!/usr/bin/env python3
"""Embed texts with the optional local model; run by tools/ask.py, not by hand.

Reads a JSON list of strings on stdin and prints a JSON list of unit vectors.
It runs under the embedding environment that ./scripts/setup-embed.sh makes
(~/.cache/builder/embed-venv), never under the project's own Python, so
ask.py itself stays standard-library only.

    echo '["how do saves load"]' | ~/.cache/builder/embed-venv/bin/python tools/context/embed.py MODEL [query|doc]

MODEL is a model2vec name (minishlab/potion-retrieval-32M) or fastembed:NAME
(fastembed:BAAI/bge-small-en-v1.5), whose queries get the model's query form.
"""

import json
import sys


def encode(name, texts, mode):
    if name.startswith("fastembed:"):
        from fastembed import TextEmbedding

        model = TextEmbedding(name.split(":", 1)[1])
        run = model.query_embed if mode == "query" else model.embed
        return [list(map(float, v)) for v in run(texts)]
    from model2vec import StaticModel

    model = StaticModel.from_pretrained(name)
    return [list(map(float, v)) for v in model.encode(texts, show_progress_bar=False)]


def main(argv):
    name = argv[1] if len(argv) > 1 else "fastembed:BAAI/bge-small-en-v1.5"
    mode = argv[2] if len(argv) > 2 else "doc"
    texts = json.load(sys.stdin)
    out = []
    for v in encode(name, texts, mode) if texts else []:
        norm = sum(x * x for x in v) ** 0.5 or 1.0
        out.append([round(x / norm, 5) for x in v])
    json.dump(out, sys.stdout, separators=(",", ":"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
