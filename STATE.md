# State

The project's long-term memory, at most two pages. Rewrite it at each milestone (a stage boundary, a finished feature, a deploy) and before a handoff. After a compaction or in a new session, read this file and ask `tools/ask.py` for cards; do not re-read files it already summarises. `scripts/context_check.py` checks its size, its commit and the cards it cites.

Commit: f07054f

## Where the build is

Stage 8 done: the first version is live at https://sgilson7.github.io/chum-scare/ (gh-pages built from 87c9ab4), checked live in three engines; journal page 7 and Live now record it, republished to the artifact. Answers CH-Q1 to CH-Q22 are recorded (all defaults but CH-Q8 B, T1; CH-Q12 note, T2).

## Decisions in force

- Autonomy: deploy when the checks pass (BRIEF.md); no subagents.
- Horror TUNING: walk 1.3, sprint 4.0 (6 s breath), crouch 40%, hop 3.0 m/s; accelerate 8, friction 6, stop_speed 0.5 (R2.4); all speed tech off (CH-10).
- Settings kept by `kits/saves`' store (D17 changed, R3.2); nothing else is stored.
- Notebook R1.1 to R3.5; person rows R1.6 (Blender models), R1.11 (lists to read).

## What is next

1. The person plays the live build and triages; each request becomes a TRIAGE row (T3 on).
2. Open: R2.6 (hand is one Verlet point), R3.4 (dust motes, bounce light); person rows R1.6, R1.11.

## Cards the next session needs

`card:docs.context.how-to-read-builder`, `card:docs.procedure.p3`, `card:entries.kinds.p2`
