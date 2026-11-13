# Chumontillado

Digest of `chumontillado` for later Builder projects. Read this instead of its transcripts; open a file only when this page names it.

## What it is

Live: https://sgilson7.github.io/chumontillado/.

A first-person movement shooter in the browser for up to four players online, one life a round, first to 5, built on Chumworld Arena's engine and primitive-built figures, painted after the Spanish masters and Picasso. Movement is Source-style (a Q16.16 port of CSSurf's CGameMovement): bunny hops, surf ramps, slide, slide jump, three wall jumps a flight, slam and slam bounce. Ten guns, melee weapons (katana with lunge and parry, navaja, guadaña, puñal), eight maps, bots in empty seats, rollback online play by room code or invite link. Desktop keyboard and mouse.

## What worked

- Movement first, every move with a hand-worked expected value as a test (CS-4): `crates/core/src/world/tests.rs`, 28 break rows in `tests/breaks.json`.
- Porting a known-good movement (CSSurf, Source 2013) to fixed point instead of writing it new (SECOND-ORDER R1.3).
- All feel numbers in one struct, `movement::TUNING`, each change commented with its triage row and old value (T1 to T3).
- Writing down where hand-worked test values were wrong (R1.8) and where tests did not test what their names said (R1.9).
- Journal entries "Why these games feel good, from your talk" and "The feel, as I would build it" before building.
- A bot moves only through the input record a player makes (CS-3), so rollback and dropped seats stay in step.

## What to copy

- **Movement**: `crates/core/src/movement.rs`, `Mover::tick(body, input, yaw_from, yaw)`. Order per tick: `unstick`, `duck`, slam check, `slide`, half gravity (`start_gravity`), `jump` or `wall_jump`, ground `friction`, `walk_move` or `air_move`, `categorize_position`, other half gravity (`finish_gravity`).
  - Keys (`crates/core/src/input.rs`): **Space = `JUMP`**, held hops on every landing; a fresh press in the air beside a wall is a wall jump. **Shift or C = `CROUCH`** (not Ctrl: Ctrl+W closes the tab, R1.5); at speed on the ground it is a slide, a fresh press high in the air a slam.
  - `Mover::slide`: starts when crouch is held on the ground at `slide_from` (6 m/s), boosts to 140% of run speed unless a slide began within `slide_rest` ticks; ends below `slide_until`, in the air, or on release.
  - `Mover::jump`: slide jump (during a slide or `slide_jump_ticks` after) adds `slide_jump` along and `slide_jump_up`; a jump within `slam_bounce_ticks` of a slam's landing uses v = √(2gh) of a share of the fall.
  - `Mover::wall_jump`: 8 traces at `wall_reach`, removes velocity into the wall, adds `wall_out` and `wall_up`, max `wall_jumps` (3) a flight; needs a fresh press (R1.7).
  - `Mover::air_move`: air acceleration in `air_substeps` sub-steps with the yaw turning between them; `air_accelerate` caps the wish at `air_cap` but accelerates with the uncapped speed (strafe gain); below run speed `air_steer` steers outright; above it, W alone gives CPMA-style `air_control`.
  - `Mover::try_move`, `clip_velocity`, `categorize_position`: Source's collide-and-slide with rampbug fixes (8 bumps, `SAME_PLANE`, `snag_offset`); ground only when normal y ≥ 0.7.
- The view turns every display frame by mouse movement not yet given to a tick (`World::camera`'s `pending`, test `the_mouse_turns_the_view_on_the_frame_it_moves_before_any_tick_takes_it`): `card:kits.realtime.input-between-ticks.p2`.
- Movement tests to copy as a list of what to check: `crates/core/src/world/tests.rs` (e.g. `hopping_and_strafing_builds_speed_past_the_run_and_landing_late_does_not`, `a_surf_ramp_gives_no_footing_and_speeds_a_surfer_along_it`, `holding_jump_against_a_wall_does_not_spend_the_wall_jumps`). Break rows: `card:claude.r6`.
- Copy a module at a named commit, then record each changed number as a notebook row (CS-2): `card:docs.procedure.the-second-order-notebook.p3`.
- Online rollback with bots on the same input record: `crates/core/src/rollback.rs`, `versus.rs`; `card:spec.D23`, `card:kits.online.the-core-side`.
- Gotcha: sink ramp and slope feet 5 cm into the floor so no two surfaces share a plane (R1.11, test `the_static_scene_draws_no_two_surfaces_in_one_plane`; `crates/core/src/flicker.rs`).
- Falls: anyone 20 m below a map is felled, and walls hold at any height (journal T27, T28).

## Where things are

- Kits: none named (online code under `web/net/`)
- Core modules: `crates/core/src/movement.rs` (movement and `TUNING`), `crates/core/src/input.rs`, `crates/core/src/world.rs`, `crates/core/src/world/tests.rs`, `crates/core/src/weapons.rs`, `crates/core/src/rollback.rs`, `crates/core/src/maps.rs`
- Notebook: `SECOND-ORDER.md` (rows R1.3 to R1.11 on movement)
- Journal: `/Users/samgilson/Documents/ChumworldShooter/chumontillado-journal`
- Folder: `/Users/samgilson/Documents/ChumworldShooter/chumontillado`
