# Check times

One row per run of ./scripts/quick-check.sh, newest last. A quick check skips relative to the newest passing `check` row.

| When | Commit | Check | Steps (s) | Total (s) | Result |
| --- | --- | --- | --- | --- | --- |
| 2026-11-13 09:41 | 84bcdd7+ | quick | fmt=0.1 | 0.1 | FAIL at fmt |
| 2026-11-13 09:42 | 84bcdd7+ | quick | fmt=0.1 clippy=0.9 core=3.4 build=1.2 wasm=0.2 browser=36.7 | 42.4 | pass |
