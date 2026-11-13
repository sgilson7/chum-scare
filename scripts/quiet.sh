#!/usr/bin/env bash
# Run a command with its full output kept in notes/runs/, and print only a summary:
# whether it passed, the step it reached, and the first errors. Read the log only
# when the summary is not enough, and then only the lines you need.
#
#   ./scripts/quiet.sh ./scripts/check.sh
#   ./scripts/quiet.sh cargo test --workspace
set -uo pipefail
cd "$(dirname "$0")/.."
[ $# -gt 0 ] || { echo "usage: ./scripts/quiet.sh COMMAND [ARGS...]"; exit 2; }
mkdir -p notes/runs
name="$(basename "$1" | tr -c 'A-Za-z0-9._-' '-')"
log="notes/runs/$(date +%Y%m%d-%H%M%S)-${name}.log"
"$@" >"$log" 2>&1
code=$?
plain="$(sed $'s/\x1b\\[[0-9;]*m//g' "$log")"
lines="$(printf '%s\n' "$plain" | wc -l | tr -d ' ')"
last_step="$(printf '%s\n' "$plain" | grep -E '^(== |#### )' | tail -1)"
if [ "$code" -eq 0 ]; then
  echo "PASSED: $* ($lines lines in $log)"
  printf '%s\n' "$plain" | grep -v '^\s*$' | tail -3
else
  echo "FAILED (exit $code): $* ($lines lines in $log)"
  [ -n "$last_step" ] && echo "last step: $last_step"
  echo "first errors:"
  printf '%s\n' "$plain" | grep -nE 'FAIL|error|Error|panicked|failed|assert' | head -20
  echo "last lines:"
  printf '%s\n' "$plain" | grep -v '^\s*$' | tail -8
fi
exit "$code"
