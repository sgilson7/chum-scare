#!/usr/bin/env bash
# Install the optional local embedding model that tools/ask.py uses for
# hybrid ranking (keyword plus cosine similarity). Nothing else needs it:
# without it, ask.py ranks by keywords alone and every check still runs.
#
#   ./scripts/setup-embed.sh          make ~/.cache/builder/embed-venv and fetch the model once
#
# The environment is shared by every Builder project on this machine. The
# model (BAAI/bge-small-en-v1.5 through fastembed, about 70 MB, run on the CPU
# with ONNX) is downloaded once and then used offline. BUILDER_EMBED_MODEL
# names another: fastembed:NAME, or a model2vec name.
set -euo pipefail
cd "$(dirname "$0")/.."
VENV="$HOME/.cache/builder/embed-venv"
MODEL="${BUILDER_EMBED_MODEL:-fastembed:BAAI/bge-small-en-v1.5}"
PYBIN=""
for c in python3.13 python3.12 python3.11 python3; do
  if command -v "$c" >/dev/null && "$c" -c 'import sys; sys.exit(0 if (3, 9) <= sys.version_info[:2] <= (3, 13) else 1)'; then PYBIN="$c"; break; fi
done
[ -n "$PYBIN" ] || { echo "no Python 3.9-3.13 found for the embedding environment"; exit 1; }
[ -x "$VENV/bin/python" ] || "$PYBIN" -m venv "$VENV"
"$VENV/bin/pip" install -q --disable-pip-version-check fastembed model2vec
echo '["warm up"]' | HF_HUB_OFFLINE=0 "$VENV/bin/python" tools/context/embed.py "$MODEL" >/dev/null
echo "embedding model ready: $MODEL in $VENV"
python3 tools/ask.py --warm
