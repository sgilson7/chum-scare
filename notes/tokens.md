# Token use

One row per measured session, newest last (`python3 tools/tokens.py`). Each call re-reads the whole context, so re-read ≈ mean context × calls.

| When | Session | Calls | Context at first plan | Peak | Mean | Compactions | Re-read | Above 200k |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 00:00 | 9db705ea | 158 | 154k (call 38) | 517k | 309k | 0 | 48.4M | 44% |
