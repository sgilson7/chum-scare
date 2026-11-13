# ChumontilladoHorror

A first-person horror game on Chumontillado's engine, slower and more methodical. Working name; rename the folder freely.

- `builder/`: Builder, fold-round-4-memory at c836541, work index off (the plain arm). `builder/SOURCE.json` records it.
- `engine/chumontillado@c48d8dd/`: a read-only copy of Chumontillado at its last commit ("T37 live at c3100a9"), to copy modules from. The original in `~/Documents/ChumworldShooter/chumontillado` is untouched.
- `refs/`: digests of Chumontillado and Chumworld Arena.
- `.claude/settings.json`: STATE.md back after a compaction, and the context warning after each tool call.
- `OPENING.md`: the first message to paste, followed by your description.

Start with `cd ~/Documents/ChumontilladoHorror && claude`.
Deploying to the CCCC server goes through `~/Documents/CCCCDeploy`, by hand.

Movement numbers that set the pace, in Chumontillado's `movement::TUNING` (m/s unless noted): `run` 9, `accelerate` 10, `air_accelerate` 40, `friction` 6, `jump` 7.92, `gravity` 18, `duck_speed_pct` 34; the speed tech is `slide_*`, `wall_*` (3 wall jumps), `slam_*`, `air_control` 150 and `air_steer` 60.
