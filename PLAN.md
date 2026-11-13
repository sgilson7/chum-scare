# Plan

Stage 2, written by the agent from `BRIEF.md`. `B2 Require step-by-step justification`.

**Not approved yet.** The person reviews the design journal's six tabs (`../ChumontilladoHorror-journal/index.html`) and answers CH-Q1 to CH-Q22 first (BRIEF.md, Autonomy). The design for each system is in the journal tab named; this file holds the build order. Rows marked *default* change with the answers.

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 1 | Add `builder/`, `engine/`, `refs/`, `OPENING.md` to `ignore` in `tests/tags.toml`; write `engine/SHA256SUMS` and verify it in `check.sh` | Baseline fails step 7 on them (R1.1); `engine/` stays untouched (CH-2) | `check.sh` passes on the copy; a planted edit in `engine/` fails it |
| 2 | Copy Chumontillado's core modules at c48d8dd (movement, fx, geom, trace, ticks, input, rng, camera, flicker, sound) with their movement tests, unchanged | Prove the port before changing it (both digests) | Copied tests pass unchanged |
| 3 | Apply the horror `TUNING` (journal tab 1), one notebook row per number with its old value | CH-1; CH-Q1 to CH-Q5 | CH-10 tests: no hop on held jump, no strafe gain, no slide, no wall jump, no slam; walk, crouch, sprint speeds hand-worked |
| 4 | The page: WebGL renderer from Chumontillado's `gl.js`, pointer lock, one test room; frame probe | Browser first (CH-8) | Browser check in three engines; frame probe in `MEASUREMENTS.md` |
| 5 | Lights: fixtures with states, flashlight charge and shake, light list, flashlight shadow (tab 6) | CH-7; CH-Q20 to CH-Q22 | Broken fixture gives no light; drain and shake hand-worked; blackout wave timing |
| 6 | Acoustics: rooms, portals, paths, voice records; Web Audio graph with HRTF, reverb, Mono (tab 4) | CH-5, CH-6; CH-Q15 to CH-Q17 | Worked doorway case; shut door loss; Mono in `ui()` and channel count in the browser |
| 7 | Hands (Arena's Verlet arm), revolver and reload, door handle (tab 2) | CH-11; CH-Q8 to CH-Q10 | Reload walk-through; turns counted; mouse moves hands not view |
| 8 | Scare director, spirit menace, possession (tab 2) | CH-3, CH-4; CH-Q14 | No spawn inside the view over every yaw and pitch; seeded replay |
| 9 | Cultists: sight, hearing through acoustics, behaviour trees from data (tab 2) | CH-Q12 | Crouched in the dark passes; lit and walking is seen |
| 10 | The mall route, dust, ouija, R U N, blackout, chase, hiding, exit; signs and fourth-wall lines (tabs 2, 5) | CH-9, CH-12; CH-Q11, CH-Q18, CH-Q19 | Whole-run scripted test; browser check plays the title flow |
| 11 | Settings kept between visits if CH-Q6 is A (`kits/saves/`) | R1.7 | The kit's save test |
| 12 | `check.sh`, create `chum-scare` (CH-Q7), `publish.sh`, browser check against the live address | The grant in BRIEF.md | Live check passes |

## Open questions for the person

CH-Q1 to CH-Q22, as answer chips in the journal.

## Status

- [ ] Read by the person on:
