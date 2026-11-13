//! The scare census (T5): prints one JSON line per run, for
//! `tools/census.py` to read. Usage: `cargo run --release --example census -- 200`
//! for 200 seeds of each stand-in player.

use tool_core::world::census::census;

fn main() {
    let n: u64 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(50);
    for player in ["methodical", "brisk", "wary"] {
        for seed in 1..=n {
            let c = census(seed * 7919, player);
            println!("{}", serde_json::to_string(&c).unwrap_or_default());
        }
    }
}
