#!/usr/bin/env bash
# The full local pre-deploy check. CI runs this same file.
#
#   ./scripts/check.sh                 everything
#   BUILDER_ENGINES=chromium ./scripts/check.sh   one engine while iterating
#
# Each step prints its name first, so a failure names the step that failed.
#
# Quiet by default (rule 22): the full output goes to notes/runs/ and only a
# summary is printed (what passed, the step reached, the first errors).
# BUILDER_LOUD=1 prints everything, as CI does.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ -z "${BUILDER_LOUD:-}${CI:-}${BUILDER_QUIET_INNER:-}" ]; then
  BUILDER_QUIET_INNER=1 exec ./scripts/quiet.sh ./scripts/check.sh "$@"
fi

PY="${BUILDER_PYTHON:-}"
if [ -z "$PY" ]; then
  if [ -x .venv/bin/python ]; then PY=.venv/bin/python; else PY=python3; fi
fi

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

step "1/9 format (cargo fmt --check)"
cargo fmt --all -- --check

step "2/9 lint (cargo clippy, warnings are errors)"
cargo clippy --workspace --all-targets --quiet -- -D warnings

step "3/9 core and boundary tests (cargo test)"
cargo test --workspace --quiet

step "4/9 release WebAssembly build (scripts/build.sh)"
./scripts/build.sh >/dev/null
echo "dist/ assembled"

step "5/9 WebAssembly import audit (scripts/wasm_imports.py)"
"$PY" scripts/wasm_imports.py dist

step "6/9 art pipeline checks, no TeX needed (tests/art_check.py)"
if [ -f tests/art_check.py ]; then "$PY" tests/art_check.py; else echo "no art pipeline in this project"; fi

step "7/9 notebook form, test tags and context (scripts/notebook_check.py, scripts/quick_check.py --guard, scripts/context_check.py)"
"$PY" scripts/notebook_check.py
"$PY" scripts/quick_check.py --guard
"$PY" scripts/context_check.py

step "8/9 design journal (scripts/journal_check.py, tools/journal/present.py --check)"
"$PY" scripts/journal_check.py
"$PY" tools/journal/present.py --check

step "9/9 browser check (tests/browser/check.py)"
"$PY" tests/browser/check.py

printf '\n\033[1mBUILDER CHECK PASSED\033[0m\n'
