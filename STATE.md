# State

The project's long-term memory, at most two pages. Rewrite it at each milestone (a stage boundary, a finished feature, a deploy) and before a handoff. After a compaction or in a new session, read this file and ask `tools/ask.py` for cards; do not re-read files it already summarises. `scripts/context_check.py` checks its size, its commit and the cards it cites.

Commit: d2838ed

## Where the build is

Stage 8 done: the first version is live at https://sgilson7.github.io/chum-scare/ (gh-pages built from 87c9ab4), checked live in three engines; journal page 7 and Live now record it, republished to the artifact. Answers CH-Q1 to CH-Q22 are recorded (all defaults but CH-Q8 B, T1; CH-Q12 note, T2). Triage round 1: T3 (volume slider) and T4 (signs fixed to their walls, turning to you in the chase) done in 2f4d5cd; journal entry `2026-11-13-fixes-t3-t4`. Round 2: T5 (scare census: `world/census.rs`, `examples/census.rs`, `tools/census.py`; journal page 8; questions CH-Q23 to CH-Q26 waiting) and T6 (cult easier to sneak past) in eb4d0c5, deployed. Round 3: T7 (CH-Q23 to CH-Q26 all A, built: kinds spread, mannequins turn, glimpses 35°–48°, wait at current menace; R6.1 to R6.3). Round 4: T8 (cultists route by doorways, `way_to`, `cult_step`), T9 (ring at (75, −9), walkers 5 m, `SUSPICION_SLOW` 40, louder mutter; census player "wary"), T10 (handle and board prompts). Round 5: T11 (signs rewritten in two passes; styles in `data/display.json`) and T12 (four curios: toy, camera, music box, telephone; the little girl; `World::pick_up`, `use_curio`, `curio_time`, `show_girl`).

## Decisions in force

- Autonomy: deploy when the checks pass (BRIEF.md); no subagents.
- Horror TUNING: walk 1.3, sprint 4.0 (6 s breath), crouch 50% (T13), hop 3.0 m/s; accelerate 8, friction 6, stop_speed 0.5 (R2.4); all speed tech off (CH-10).
- Settings kept by `kits/saves`' store (D17 changed, R3.2); nothing else is stored.
- Signs: a facing per sign (R4.1), turned 30°/s only in the chase (R4.2), drawn by a core-made CSS matrix3d (R4.3).
- Cult (T6): ring (70, −8) r 4, walkers r 6; sees you only inside the court or once hunting; `NOTICE_FLOOR` 1/30; pendant at 56 m removed; door lamp moved to (84, −12). Census: careful 0/200 caught, careless 200/200.
- Director (T7): `last_scare` wait at current menace; `kinds_used`/`last_kind`; `mannequin_to_turn`; `GLIMPSE_NEAR`/`FAR`, `glimpse_wait`. Screen edge at 16:9 is 51°, not `HALF_VIEW` (R6.3).
- Curios (T12): E picks up and puts down, the left button uses, the revolver holstered meanwhile (R8.4); each answers once; the girl may stand behind a counter (R8.5).
- Fuse box (T14): E throws it (court lamps out 40 s, cult blind and deaf 12 s), a shot puts it out for good. Spirit rises in the far back corner and stands 3 s (T15). Darker: ambient 1/250, fog 1/600, exposure ≤ 1.05, lamps ×1.5 (T16).
- Scares (T17 to T19): director scares hush the room 3 s with a `dread` swell; own steps at half; stranger's steps heavy, nearing; knock and whisper kinds; apparitions (eyes, smile, hands) wait at the screen's edge until looked at; spirit glimpses 1.25 s; the spirit stares 3 s, screams, comes at 1 m/s to 3.6 over 20 s, follows your trail, never sits on a doorway.
- T20: director wakes past the doors; ceilings 1, 3, 4, 5, 6 a minute; apparitions nearer, larger, every 15 s; ambient 1/700, fog 1/2000, no dark brightening; court unlit but for the ring's candles, which the fuse box feeds. T21: the spirit routes by the fewest rooms.
- T22: hum and buzz at half, each with a second slow wobble (`wobble2`).
- T23: spirit top 2.6 m/s after 30 s; prefers a fleeing cultist nearer than you unless you are within 5 m; feeds 4 s.
- Notebook R1.1 to R11.6; person rows R1.6 (Blender models, now with the girl and curios), R1.11 (lists to read), R5.5 (play the court), R8.2 (approve the signs), R9.6 (judge the darkness).

## What is next

1. The person plays the court (R5.5), says what stopped them after the cult (R7.2) and reads the signs (R8.2) and judges the darkness (R9.6) and the hum (R11.5); next T row: T24.
2. Open: R2.6 (hand is one Verlet point), R3.4 (dust motes, bounce light); person rows R1.6, R1.11.

## Cards the next session needs

`card:docs.context.how-to-read-builder`, `card:docs.procedure.p3`, `card:entries.kinds.p2`
