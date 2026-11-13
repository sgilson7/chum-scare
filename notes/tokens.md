# Token use

One row per measured session, newest last (`python3 tools/tokens.py`). Each call re-reads the whole context, so re-read ≈ mean context × calls.

| When | Session | Calls | Context at first plan | Peak | Mean | Compactions | Re-read | Above 200k |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 00:00 | 9db705ea | 158 | 154k (call 38) | 517k | 309k | 0 | 48.4M | 44% |
| 2026-11-13 09:45 | 9db705ea | 238 | 154k (call 38) | 552k | 299k | 1 | 70.6M | 45% |
| 2026-11-13 11:47 | 9db705ea | 335 | 154k (call 38) | 552k | 278k | 1 | 92.2M | 39% |
| 2026-11-13 15:32 | 9db705ea | 464 | 154k (call 38) | 552k | 272k | 2 | 124.9M | 37% |
| 2026-11-13 16:02 | 9db705ea | 539 | 154k (call 38) | 552k | 267k | 2 | 142.7M | 34% |
| 2026-11-13 16:23 | 9db705ea | 571 | 154k (call 38) | 552k | 269k | 2 | 152.5M | 34% |
