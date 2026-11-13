# State

The project's long-term memory, at most two pages. Rewrite it at each milestone (a stage boundary, a finished feature, a deploy) and before a handoff. After a compaction or in a new session, read this file and ask `tools/ask.py` for cards; do not re-read files it already summarises. `scripts/context_check.py` checks its size, its commit and the cards it cites.

Commit: see `git log -1` (first commit, "Opening entry and design journal tabs 1 to 6")

## Where the build is

Stage 0 done, stage 1 brief written, stage 2 waiting. Nothing built. The design journal at `../ChumontilladoHorror-journal/index.html` has six tabs (1 How I use Builder, 2 The game, 3 3D sound research, 4 Our soundscape, 5 Cleansing ritual, 6 Light) and 22 questions CH-Q1 to CH-Q22 as chips (`decisions.json`). `journal_check.py` passes. Waiting for the person's answers.

## Decisions in force

- Autonomy: deploy when the checks pass, to a public repo the person named "Chum Scare" (default `chum-scare`, CH-Q7, R1.3); no subagents; plan NOT approved in advance.
- Rules CH-1 to CH-12 (journal tab 1 table); move agreed ones into CLAUDE.md "Rules agreed" after answers.
- Horror TUNING values: journal tab 1 table (walk 1.3, accel 4, friction 5, stop 1.0, gravity 9.81, jump 3.0, all speed tech off).
- Notebook R1.1 to R1.11; R1.6 and R1.11 are the person's.
- Baseline: 38 tests; check.sh fails only at step 7 tags (R1.1); Playwright venv now installed.

## What is next

1. Read answers (`python3 tools/journal/decisions.py --answers "..."`), write an *agreed* entry, TRIAGE rows for non-defaults, rules into CLAUDE.md/AGENTS.md.
2. Build in PLAN.md order; journal entries per visible change; deploy.

## Cards the next session needs

`card:docs.context.how-to-read-builder`, `card:docs.procedure.the-second-order-notebook.p3`, `card:kits.realtime.input-between-ticks.p2`, `card:entries.kinds.p2`
