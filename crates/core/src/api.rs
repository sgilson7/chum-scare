//! The door into the core: the one object the shim holds. The page gives it
//! the clock, the keys and the mouse each frame; it runs whole ticks and
//! hands back what to draw and play. Settings, screens and flags come back
//! through one function, `act(name, value)`, and the screen is described by
//! `ui()` (card:spec.R4). Strings out are JSON; numbers out are Q16.16
//! integers the shim divides by `ONE`.

use serde_json::json;

use crate::fx::{self, ONE};
use crate::input::{Input, ACTIONS, BUTTON_SHIFT};
use crate::view::{self, Camera, INSTANCE, MAX_LIGHTS, UNIFORMS};
use crate::world::{World, HZ, TURN_PER_PX};

/// The most ticks one frame may run, so a stalled tab does not race.
pub const MAX_TICKS_PER_FRAME: i64 = 5;
const TICK_US: i64 = 1_000_000 / HZ;

pub struct Game {
    world: World,
    last_us: Option<i64>,
    owed_us: i64,
    /// mouse movement not yet given to a tick: the view turns by it now
    pending: (i32, i32),
    keys: u32,
    aspect: i32,
    width: i32,
    height: i32,
    noise_seed: u64,
}

impl Game {
    /// A seed the page drew and the computer's minute of the day (for the
    /// clocks in the mall, journal tab 5).
    pub fn new(seed: u32, minute_of_day: i32) -> Game {
        Game {
            world: World::new(seed as u64, minute_of_day),
            last_us: None,
            owed_us: 0,
            pending: (0, 0),
            keys: 0,
            aspect: ONE * 16 / 9,
            width: 1,
            height: 1,
            noise_seed: seed as u64 ^ 0x5EED,
        }
    }

    /// CSS size and pixel ratio (hundredths) in; the drawing buffer's size
    /// out, smaller on the Low tier.
    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_pct: i32) -> [i32; 2] {
        let scale = [60, 85, 100][self.world.settings.tier.clamp(0, 2) as usize];
        let dpr = dpr_pct.clamp(100, 200);
        self.width = (css_w * dpr / 100 * scale / 100).max(1);
        self.height = (css_h * dpr / 100 * scale / 100).max(1);
        self.aspect = fx::div(self.width * ONE, self.height * ONE);
        [self.width, self.height]
    }

    /// The clock (µs), the held bits, and the mouse since the last frame
    /// (hundredths of a CSS pixel).
    pub fn advance(&mut self, now_us: i64, bits: u32, dx: i32, dy: i32) {
        self.keys = bits;
        self.pending.0 += dx;
        self.pending.1 += dy;
        let step = match self.last_us {
            Some(prev) => (now_us - prev).clamp(0, TICK_US * MAX_TICKS_PER_FRAME),
            None => TICK_US,
        };
        self.last_us = Some(now_us);
        self.owed_us += step;
        while self.owed_us >= TICK_US {
            self.owed_us -= TICK_US;
            let m = crate::input::Mouse { dx: self.pending.0, dy: self.pending.1, wheel: 0 };
            self.pending = (0, 0);
            let input = Input::from_page(bits, m);
            self.world.tick(&input);
        }
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        match self.world.act(name, value) {
            Ok(()) => json!({ "ok": true }).to_string(),
            Err(e) => json!({ "error": e }).to_string(),
        }
    }

    fn camera(&self) -> Camera {
        // the view turns by the mouse not yet given to a tick
        let looking = self.world.mode == crate::world::Mode::Look && !self.world.flashlight.shaking;
        let (py, pp) = if looking {
            let k = TURN_PER_PX * self.world.settings.sensitivity / 100;
            (-self.pending.0 * k / 100, -self.pending.1 * k / 100)
        } else {
            (0, 0)
        };
        view::camera(&self.world, self.aspect, py, pp)
    }

    pub fn static_boxes(&self) -> Vec<i32> {
        view::static_boxes(&self.world.mall.slabs)
    }

    pub fn dynamic_boxes(&self) -> Vec<i32> {
        view::dynamic_boxes(&self.world)
    }

    pub fn held_boxes(&self) -> Vec<i32> {
        view::held_boxes(&self.world, &self.camera())
    }

    pub fn uniforms(&self) -> Vec<i32> {
        view::uniforms(&self.world, &self.camera())
    }

    /// The listener: position, forward, up (for Web Audio).
    pub fn listener(&self) -> Vec<i32> {
        let c = self.camera();
        vec![c.eye.x, c.eye.y, c.eye.z, c.fwd.x, c.fwd.y, c.fwd.z, c.up.x, c.up.y, c.up.z]
    }

    /// This tick's sounds and the beds, placed for the listener (JSON).
    pub fn sounds(&self) -> String {
        let c = self.camera();
        json!({ "voices": view::sounds(&self.world, &c), "beds": view::beds(&self.world, &c) }).to_string()
    }

    pub fn ui(&self) -> String {
        serde_json::to_string(&view::ui(&self.world, &self.camera())).unwrap_or_default()
    }

    /// White noise for the page's sounds: `len` samples from −1 to 1.
    pub fn noise(&mut self, len: u32) -> Vec<i32> {
        let mut r = crate::rng::Rng::new(self.noise_seed);
        (0..len).map(|_| r.between(-ONE, ONE)).collect()
    }

    /// A room's echo as an impulse response: noise falling by 60 dB over
    /// `decay_ms`, `len` samples at `rate` (journal tab 4).
    pub fn impulse(&mut self, decay_ms: u32, rate: u32, len: u32) -> Vec<i32> {
        let mut r = crate::rng::Rng::new(self.noise_seed ^ decay_ms as u64);
        let total = (decay_ms as i64 * rate as i64 / 1000).max(1);
        (0..len as i64)
            .map(|i| {
                // amplitude 10^(−3 i / total), by halving every total / 10 samples
                let halvings = i * 10 / total;
                let frac = (i * 10 % total) * ONE as i64 / total;
                let amp = if halvings > 30 { 0 } else { (ONE >> halvings) - ((ONE >> halvings) as i64 * frac / ONE as i64 / 2) as i32 };
                fx::mul(r.between(-ONE, ONE), amp)
            })
            .collect()
    }

    /// Every layout number the page needs (D3: the page holds none).
    pub fn numbers(&self) -> String {
        let actions: Vec<_> = ACTIONS.iter().map(|(n, b)| json!([n, b])).collect();
        let shadow = [512, 1024, 2048][self.world.settings.tier.clamp(0, 2) as usize];
        let uniforms: Vec<_> = UNIFORMS.iter().map(|(n, c, l)| json!([n, c, l])).collect();
        json!({
            "one": ONE,
            "instance": INSTANCE,
            "max_lights": MAX_LIGHTS,
            "attributes": [["aC0", 3, 0], ["aC1", 3, 3], ["aC2", 3, 6], ["aT", 3, 9], ["aColor", 3, 12], ["aMaterial", 1, 15], ["aGlow", 1, 16]],
            "uniforms": uniforms,
            "actions": actions,
            "button_shift": BUTTON_SHIFT,
            "listener": 9,
            "cube": cube(),
            "cube_stride": 6,
            "shadow_size": shadow,
            "ms_to_us": 1000,
            "px_to_hundredths": 100,
            "pct": 100,
            "thousand": 1000,
        })
        .to_string()
    }
}

/// The unit cube's 36 corners with their normals, about the origin.
fn cube() -> Vec<i32> {
    let h = 1;
    let faces: [([i32; 3], [[i32; 3]; 4]); 6] = [
        ([1, 0, 0], [[h, -h, -h], [h, h, -h], [h, h, h], [h, -h, h]]),
        ([-1, 0, 0], [[-h, -h, h], [-h, h, h], [-h, h, -h], [-h, -h, -h]]),
        ([0, 1, 0], [[-h, h, -h], [-h, h, h], [h, h, h], [h, h, -h]]),
        ([0, -1, 0], [[-h, -h, h], [-h, -h, -h], [h, -h, -h], [h, -h, h]]),
        ([0, 0, 1], [[h, -h, h], [h, h, h], [-h, h, h], [-h, -h, h]]),
        ([0, 0, -1], [[-h, -h, -h], [-h, h, -h], [h, h, -h], [h, -h, -h]]),
    ];
    let mut out = Vec::new();
    for (n, q) in faces {
        for k in [0, 1, 2, 0, 2, 3] {
            // corners at ±½ and unit normals, as Q16.16 like every number out
            out.extend_from_slice(&[q[k][0] * ONE / 2, q[k][1] * ONE / 2, q[k][2] * ONE / 2, n[0] * ONE, n[1] * ONE, n[2] * ONE]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_runs_the_ticks_its_time_owes_and_no_more_than_five() {
        let mut g = Game::new(1, 0);
        g.act("start", "");
        g.advance(0, 0, 0, 0);
        let t0 = g.world.tick;
        // 50 ms later: three whole ticks of 16 667 µs
        g.advance(50_000, 0, 0, 0);
        assert_eq!(g.world.tick - t0, 3);
        // a second's stall runs five
        g.advance(1_050_000, 0, 0, 0);
        assert_eq!(g.world.tick - t0, 8);
    }

    #[test]
    fn act_refuses_an_unknown_action_by_name() {
        let mut g = Game::new(1, 0);
        assert!(g.act("fly", "").contains("unknown action fly"));
        assert!(g.act("output", "mono").contains("true"));
    }

    #[test]
    fn the_cube_has_thirty_six_corners() {
        assert_eq!(cube().len(), 36 * 6);
    }

    #[test]
    fn an_impulse_falls_away() {
        let mut g = Game::new(1, 0);
        let ir = g.impulse(1000, 1000, 2000);
        let early: i64 = ir[..100].iter().map(|v| v.abs() as i64).sum();
        let late: i64 = ir[1000..1100].iter().map(|v| v.abs() as i64).sum();
        assert!(late * 100 < early, "{early} {late}");
    }
}
