# State

The project's long-term memory, at most two pages. Rewrite it at each milestone (a stage boundary, a finished feature, a deploy) and before a handoff. After a compaction or in a new session, read this file and ask `tools/ask.py` for cards; do not re-read files it already summarises. `scripts/context_check.py` checks its size, its commit and the cards it cites.

Commit: d9861de

## Where the build is

Stage 8 done: the first version is live at https://sgilson7.github.io/chum-scare/ (gh-pages built from 87c9ab4), checked live in three engines; journal page 7 and Live now record it, republished to the artifact. Answers CH-Q1 to CH-Q22 are recorded (all defaults but CH-Q8 B, T1; CH-Q12 note, T2). Triage round 1: T3 (volume slider) and T4 (signs fixed to their walls, turning to you in the chase) done in 2f4d5cd; journal entry `2026-11-13-fixes-t3-t4`. Round 2: T5 (scare census: `world/census.rs`, `examples/census.rs`, `tools/census.py`; journal page 8; questions CH-Q23 to CH-Q26 waiting) and T6 (cult easier to sneak past) in eb4d0c5, deployed. Round 3: T7 (CH-Q23 to CH-Q26 all A, built: kinds spread, mannequins turn, glimpses 35°–48°, wait at current menace; R6.1 to R6.3). Round 4: T8 (cultists route by doorways, `way_to`, `cult_step`), T9 (ring at (75, −9), walkers 5 m, `SUSPICION_SLOW` 40, louder mutter; census player "wary"), T10 (handle and board prompts).

## Decisions in force

- Autonomy: deploy when the checks pass (BRIEF.md); no subagents.
- Horror TUNING: walk 1.3, sprint 4.0 (6 s breath), crouch 40%, hop 3.0 m/s; accelerate 8, friction 6, stop_speed 0.5 (R2.4); all speed tech off (CH-10).
- Settings kept by `kits/saves`' store (D17 changed, R3.2); nothing else is stored.
- Signs: a facing per sign (R4.1), turned 30°/s only in the chase (R4.2), drawn by a core-made CSS matrix3d (R4.3).
- Cult (T6): ring (70, −8) r 4, walkers r 6; sees you only inside the court or once hunting; `NOTICE_FLOOR` 1/30; pendant at 56 m removed; door lamp moved to (84, −12). Census: careful 0/200 caught, careless 200/200.
- Director (T7): `last_scare` wait at current menace; `kinds_used`/`last_kind`; `mannequin_to_turn`; `GLIMPSE_NEAR`/`FAR`, `glimpse_wait`. Screen edge at 16:9 is 51°, not `HALF_VIEW` (R6.3).
- Notebook R1.1 to R7.2; person rows R1.6 (Blender models), R1.11 (lists to read), R5.5 (play the court).

## What is next

1. The person plays the court (R5.5) and says what stopped them after the cult (R7.2); next T row: T11.
2. Open: R2.6 (hand is one Verlet point), R3.4 (dust motes, bounce light); person rows R1.6, R1.11.

## Cards the next session needs

`card:docs.context.how-to-read-builder`, `card:docs.procedure.p3`, `card:entries.kinds.p2`
