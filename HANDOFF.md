# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

Commit: 2f4d5cd

## What this is

Chum Scare: a first-person horror game in a desktop browser, built on Chumontillado's engine (copied at c48d8dd) and slowed for methodical play. You walk an abandoned mall with an empty revolver and a shake-charged flashlight, load the revolver by hand while the scare director acts behind you, avoid the cultists, use the ouija board, survive the chase and get out. Published on GitHub Pages from the public repository `chum-scare` (CH-Q7 A).

## Rules that hold it up

| Rule | Guard |
| --- | --- |
| CH-1 feel numbers in `movement::TUNING`, old values beside them | `world/tests.rs` movement tests; SECOND-ORDER R2.3, R2.4 |
| CH-4 no reload or door scare spawns in view | `a_scare_spot_is_never_inside_the_view_at_any_yaw_or_pitch`, `every_scare_the_director_makes_while_loading_is_out_of_view` |
| CH-5 sound placement from rooms and portals | `acoustics.rs` tests (the journal's worked case) |
| CH-6 Mono reaches every sound | `mono_brings_the_ring_and_subtitles`; browser check reads one channel |
| CH-7 a broken light gives none | `lights.rs`, `view.rs` tests |
| CH-10 no bunny hop, strafe gain, slide, wall jump or slam | `world/tests.rs` |
| CH-11 the mouse moves the hand, not the view, while the hands are out | `with_the_hands_out_the_mouse_moves_the_hand_and_not_the_view` |
| T4 signs hang facing one way; they turn only in the chase | `signs_keep_the_way_they_face_while_you_walk`, `in_the_chase_the_signs_turn_…`, `a_sign_hangs_facing_its_way_…` |
| T3 volume is a percent kept with the settings | `the_volume_is_a_percent_kept_with_the_settings` |
| D17 changed: only `web/save/store.js` keeps the settings text | `storage_fence.rs` |

## Shape of the code

| Where | What |
| --- | --- |
| `crates/core/src/movement.rs`, `map.rs`, `trace.rs`, `fx.rs`, `geom.rs`, `rng.rs`, `input.rs` | Chumontillado's, copied at c48d8dd (map trimmed, R2.2) |
| `mall.rs` | the route, rooms, portals, doors, fixtures, props, scare points, signs |
| `world.rs` | one tick: hands, reload, doors, ouija, director, possession, cultists, chase |
| `acoustics.rs`, `lights.rs` | sound paths; fixtures, blackout wave, flashlight |
| `view.rs`, `api.rs` | boxes, uniforms, placed sounds, the screen; the door the shim calls |
| `web/` | `app.js` loop, `gl.js` renderer, `audio.js` Web Audio, `input.js`, `save/store.js` |
| `data/` | strings, sound recipes, reverbs, shader constants, controls, shaders |

## Commands

`cargo test --workspace` (all pass); `./scripts/build.sh` (wasm about 250 KB); `python3 scripts/serve.py` then http://127.0.0.1:8000/; `./scripts/check.sh` (9 steps); `./scripts/publish.sh` (gh-pages).

## What will bite within the hour

- Ground acceleration must stay at or above friction or the walk crawls (R2.4).
- The page and shaders may hold no number but 0 and 1: new numbers go in `data/` or `numbers()`.
- Signs are HTML drawn through a CSS `matrix3d` the core computes (`view::sign_matrix`); the page only applies it (R4.3).
- Headless browsers refuse pointer lock: `web/input.js` falls back to plain mouse movement.

## For the person

- **R1.6**: the spirit, cultist, mannequin and car models, made in Blender in `~/Documents/BuilderRefinedArtstyle` and exported as glTF; the game uses stand-ins until then.
- **R5.5**: play the court and say whether the cult is now too easy or still too hard (the ring, the doorway, the notice floor, the two lights; T6).
- **R1.11**: read and edit the scare list, the store names and the sign texts (journal tab 2, tab 5; `data/strings.json`).

## The journal

`../ChumontilladoHorror-journal/index.html`, published privately at https://claude.ai/artifact/63D3cDkCW4FbLVcZ7v5rth.

## Tokens

| When | Session | Calls | Context at first plan | Peak | Mean | Compactions | Re-read | Above 200k |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 00:00 | 9db705ea | 158 | 154k (call 38) | 517k | 309k | 0 | 48.4M | 44% |
| 2026-11-13 09:45 | 9db705ea | 238 | 154k (call 38) | 552k | 299k | 1 | 70.6M | 45% |
