//! tool_core: every rule of Chum Scare. No input or output.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that.
//!
//! Copied from Chumontillado at c48d8dd (engine/chumontillado@c48d8dd):
//! `fx`, `geom`, `rng`, `input`, `trace`, `movement` and `map` (less its
//! drawing, weapons and practice room). Where to start reading: `api.rs` is
//! the door the shim calls through, `world.rs` the order of one tick.

pub mod acoustics;
pub mod api;
pub mod fx;
pub mod geom;
pub mod glossary;
pub mod input;
pub mod lights;
pub mod mall;
pub mod map;
pub mod movement;
pub mod rng;
pub mod trace;
pub mod view;
pub mod world;
