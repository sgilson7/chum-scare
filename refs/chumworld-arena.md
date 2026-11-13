# Chumworld Arena

Digest of `chumworld-arena` for later Builder projects. Read this instead of its transcripts; open a file only when this page names it.

## What it is

Live: https://sgilson7.github.io/chumworld-arena/.

A first-person sword-duel game for a desktop browser. Its combat is CHUMWORLD's, copied at d9cb5b5: a physics-simulated arm (Verlet points and sticks) steered by the mouse, WASD for the feet, seen from inside the helmet and paced like Chivalry (stamina, feints, ripostes, a kick). Knights in plate fight best-of-three duels in a stone ring against a behaviour-tree pilot or another browser online (rollback). Later rounds added a ladder of opponents, a Watch mode, a Single Player Campaign map, and more weapons and fighting styles.

## What worked

- Copying a module at a named commit, then proving the port with a parity test before changing it (CA-2; `crates/core/tests/parity.rs`).
- Rollback built on the online kit's lockstep session, kept as the reference rollback must match (CA-5; `crates/core/src/rollback.rs`).
- Pilots that return only an input record, tested by replaying records with the pilot removed (CA-3; `crates/core/src/pilot.rs`).
- Watch mode for tuning AI: two pilots fight while their behaviour trees are drawn (`crates/core/src/watch.rs`; journal "Watch mode: two fighters duel while you read their trees", "Four new fighting styles, built and measured in watch mode").
- Probes for timing budgets, with numbers in `MEASUREMENTS.md` (CA-6; `crates/core/tests/rollback_probe.rs`, `tick_probe.rs`, `duel_probe.rs`).
- Costume models handed to background agents in their own worktrees once the plate contract was fixed (CA-7).

## What to copy

- **Simulated arm**: `crates/core/src/arm.rs` (Jakobsen-style position Verlet, six points per fighter, sticks and hand pins, fixed relaxation order, Q16.16). Fighter, sword states and gait: `player.rs`, `motion.rs`.
- **Hit feel**: `crates/core/src/feel.rs` (hit-stop, camera knock, sparks, stagger), all on the tick and decaying to exactly zero so a still view draws the same pixels.
- **Parity test for a port**: record a scripted duel in a scratch copy of the source project and replay it tick for tick (`crates/core/tests/parity.rs`). Notebook rows for each changed number: `card:docs.procedure.the-second-order-notebook.p3`.
- **Rollback**: `crates/core/src/rollback.rs` (whole `Sim` is `Clone`, snapshot before each tick, predict last input, re-simulate on mismatch; 8-tick window, 2-tick input delay), tested against `online.rs`'s lockstep session; `versus.rs` handles host, join, rematch, silence, desync. `card:spec.D23`, `card:kits.online.the-core-side`. Relays only in `web/net/config.js` and the CSP (CA-4, `crates/core/tests/online_fence.rs`): `card:docs.online.the-rule`.
- **Behaviour-tree pilots from data**: `crates/core/src/pilot.rs`, `data/pilots.json`, with `pilot::Trace` for drawing which rule fired.
- **Flicker scan**: `crates/core/src/flicker.rs` finds coplanar faces in the static scene (ported from Long Time Comin; swept along one axis so large ground meshes scan fast).
- **Armour as a rule table**: `crates/core/src/plate.rs` (`plate::covered`; cut, thrust, blunt damage kinds), `wounds.rs`.
- Probes are not checks: `card:docs.checks.probes-are-not-checks`. Several agents at once: `card:docs.procedure.several-agents-at-once`.
- Feel is the person's call from play; when the person asked for no stops, play stops became journal entries with shots for later review (CA-10, R1.3).

## Where things are

- Kits: online (in `web/net/`)
- Core modules: `crates/core/src/arm.rs`, `crates/core/src/player.rs`, `crates/core/src/motion.rs`, `crates/core/src/game.rs`, `crates/core/src/wounds.rs`, `crates/core/src/blade.rs`, `crates/core/src/rollback.rs`, `crates/core/src/pilot.rs`, `crates/core/src/chivalry.rs` (numbers in `data/chivalry.json`)
- Journal: `/Users/samgilson/Documents/ChumworldArena/chumworld-arena-journal`
- Folder: `/Users/samgilson/Documents/ChumworldArena/chumworld-arena`
