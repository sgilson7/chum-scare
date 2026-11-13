//! Light (journal tab 6, CH-7): every fixture is in the map with a state,
//! a broken fixture gives no light, and the flashlight's charge is a number
//! here. The blackout after R U N breaks every fixture but the moon, in a
//! wave from the cursed store.

use crate::fx::{self, Fx, ONE};
use crate::geom::V3;
use crate::mall::{Fixture, FixtureKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    On,
    Flicker,
    Broken,
}

/// The flashlight: charge in percent (Q16.16), on or off, and the shake.
pub const DRAIN_TICKS: i32 = 180;
pub const LOW: Fx = 25 * ONE;
pub const FULL: Fx = 100 * ONE;
/// Each fast reversal of the mouse while F is held adds 1.5%.
pub const SHAKE_GAIN: Fx = ONE * 3 / 2;
/// A reversal counts when the mouse moved at least this far in the tick
/// (hundredths of a CSS pixel).
pub const SHAKE_FAST: i32 = 600;
/// F held for fewer ticks than this with no reversal is a tap: on or off.
pub const TAP_TICKS: i32 = 15;
/// The blackout's wave: ticks per metre (25 ms a metre at 60 a second).
pub const WAVE_TICKS_PER_M: i32 = 3;
pub const WAVE_PER: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flashlight {
    pub on: bool,
    pub charge: Fx,
    pub held: i32,
    pub reversals: i32,
    pub last_dx_sign: i32,
    pub shaking: bool,
}

impl Default for Flashlight {
    fn default() -> Flashlight {
        Flashlight { on: true, charge: FULL, held: 0, reversals: 0, last_dx_sign: 0, shaking: false }
    }
}

/// What the shake did this tick, for the sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShakeEvents {
    pub toggled: bool,
    pub pumped: bool,
}

impl Flashlight {
    /// One tick: drain while on; with F held, count fast reversals of the
    /// mouse across; on release, a short hold with none toggles the light.
    pub fn tick(&mut self, f_held: bool, dx: i32) -> ShakeEvents {
        let mut ev = ShakeEvents::default();
        if self.on && self.charge > 0 {
            self.charge = (self.charge - ONE / DRAIN_TICKS).max(0);
        }
        if f_held {
            self.held += 1;
            let sign = if dx >= SHAKE_FAST {
                1
            } else if dx <= -SHAKE_FAST {
                -1
            } else {
                0
            };
            if sign != 0 && self.last_dx_sign != 0 && sign != self.last_dx_sign {
                self.reversals += 1;
                self.charge = (self.charge + SHAKE_GAIN).min(FULL);
                ev.pumped = true;
            }
            if sign != 0 {
                self.last_dx_sign = sign;
            }
            self.shaking = self.held >= TAP_TICKS || self.reversals > 0;
        } else {
            if self.held > 0 && self.held < TAP_TICKS && self.reversals == 0 {
                self.on = !self.on;
                ev.toggled = true;
            }
            self.held = 0;
            self.reversals = 0;
            self.last_dx_sign = 0;
            self.shaking = false;
        }
        ev
    }

    /// The beam's brightness, 0 to `ONE`: full above 25%, then falling with
    /// the charge, flickering by `flick` (0 or 1 from the world's seed).
    pub fn brightness(&self, flick: bool) -> Fx {
        if !self.on {
            return 0;
        }
        if self.charge >= LOW {
            return ONE;
        }
        let b = fx::div(self.charge, LOW);
        if flick {
            b / 3
        } else {
            b.max(ONE / 20)
        }
    }
}

/// The fixtures' states, and the blackout's wave once it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lights {
    pub states: Vec<State>,
    /// (tick it started, where from)
    pub wave: Option<(i64, V3)>,
}

impl Lights {
    pub fn new(fixtures: &[Fixture]) -> Lights {
        Lights {
            states: fixtures.iter().map(|f| if f.flickers { State::Flicker } else { State::On }).collect(),
            wave: None,
        }
    }

    pub fn break_one(&mut self, i: usize, fixtures: &[Fixture]) -> bool {
        if fixtures[i].kind == FixtureKind::Moon || self.states[i] == State::Broken {
            return false;
        }
        self.states[i] = State::Broken;
        true
    }

    /// The tick the wave reaches a fixture `d` metres from where it began.
    pub fn wave_reaches(start: i64, d: Fx) -> i64 {
        start + (fx::mul(d, ONE * WAVE_TICKS_PER_M / WAVE_PER) / ONE) as i64
    }

    /// Advance the wave: each fixture it has reached breaks. Returns the
    /// ones that broke this tick (for their pops and glass).
    pub fn advance(&mut self, tick: i64, fixtures: &[Fixture]) -> Vec<usize> {
        let mut broke = Vec::new();
        if let Some((start, from)) = self.wave {
            for (i, f) in fixtures.iter().enumerate() {
                let d = f.at.sub(from).len();
                if tick >= Lights::wave_reaches(start, d) && self.break_one(i, fixtures) {
                    broke.push(i);
                }
            }
        }
        broke
    }

    /// How bright fixture `i` is this tick, 0 to `ONE`: 0 when broken; a
    /// flickering one drops out on `flick` ticks.
    pub fn brightness(&self, i: usize, flick: bool) -> Fx {
        match self.states[i] {
            State::On => ONE,
            State::Flicker => {
                if flick {
                    ONE / 8
                } else {
                    ONE
                }
            }
            State::Broken => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::int;
    use crate::geom::v3;
    use crate::mall::mall;

    #[test]
    fn the_flashlight_drains_one_percent_in_one_hundred_and_eighty_ticks() {
        let mut f = Flashlight::default();
        for _ in 0..180 {
            f.tick(false, 0);
        }
        // 100% − 180 × (1/180)% = 99%, within the rounding of ONE / 180
        assert!((f.charge - 99 * ONE).abs() <= 180, "{}", fx::decimal(f.charge, 4));
    }

    #[test]
    fn ten_fast_reversals_add_fifteen_percent_and_slow_ones_add_nothing() {
        let mut f = Flashlight { charge: 10 * ONE, on: false, ..Flashlight::default() };
        // the first push sets a side; each later push the other way is a reversal
        for k in 0..11 {
            f.tick(true, if k % 2 == 0 { 900 } else { -900 });
        }
        assert_eq!(f.charge, 10 * ONE + 10 * SHAKE_GAIN);
        let mut g = Flashlight { charge: 10 * ONE, on: false, ..Flashlight::default() };
        for k in 0..11 {
            g.tick(true, if k % 2 == 0 { 300 } else { -300 });
        }
        assert_eq!(g.charge, 10 * ONE);
    }

    #[test]
    fn a_short_tap_of_f_toggles_the_light_and_a_shake_does_not() {
        let mut f = Flashlight::default();
        for _ in 0..5 {
            f.tick(true, 0);
        }
        f.tick(false, 0);
        assert!(!f.on);
        for k in 0..5 {
            f.tick(true, if k % 2 == 0 { 900 } else { -900 });
        }
        f.tick(false, 0);
        assert!(!f.on, "a shake is not a tap");
    }

    #[test]
    fn a_broken_fixture_gives_no_light() {
        let m = mall();
        let mut l = Lights::new(&m.fixtures);
        assert_eq!(l.brightness(0, false), ONE);
        assert!(l.break_one(0, &m.fixtures));
        assert_eq!(l.brightness(0, false), 0);
        assert_eq!(l.brightness(0, true), 0);
    }

    #[test]
    fn the_wave_reaches_a_fixture_twenty_metres_off_after_thirty_ticks() {
        // 20 m × 1.5 ticks a metre = 30 ticks = 500 ms
        assert_eq!(Lights::wave_reaches(100, int(20)), 130);
    }

    #[test]
    fn the_blackout_breaks_every_fixture_but_the_moon() {
        let m = mall();
        let mut l = Lights::new(&m.fixtures);
        l.wave = Some((0, v3(int(95), 0, 0)));
        l.advance(100_000, &m.fixtures);
        for (i, f) in m.fixtures.iter().enumerate() {
            let want = if f.kind == FixtureKind::Moon { State::On } else { State::Broken };
            assert_eq!(l.states[i], want, "{:?}", f.kind);
        }
    }
}
