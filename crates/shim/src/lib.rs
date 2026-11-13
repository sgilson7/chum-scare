//! tool_shim: moves values between the page and `tool_core::api::Game`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core and a conversion of its arguments or result: the core's
//! Q16.16 integers become floats divided by its `ONE`, the page's
//! millisecond clock becomes microseconds and its pixels hundredths.
//! `crates/core/tests/boundary.rs` fails if a branch, a loop or the `?`
//! operator appears here.

use tool_core::{api, fx};
use wasm_bindgen::prelude::*;

const ONE: f32 = fx::ONE as f32;

fn floats(v: Vec<i32>) -> Vec<f32> {
    v.into_iter().map(|x| x as f32 / ONE).collect()
}

#[wasm_bindgen]
pub struct Game {
    inner: api::Game,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, minute_of_day: i32) -> Game {
        Game { inner: api::Game::new(seed, minute_of_day) }
    }

    pub fn resize(&mut self, css_w: f64, css_h: f64, dpr: f64) -> Vec<i32> {
        self.inner.resize(css_w as i32, css_h as i32, (dpr * 100.0) as i32).to_vec()
    }

    pub fn advance(&mut self, now_ms: f64, bits: u32, dx: f64, dy: f64) {
        self.inner.advance((now_ms * 1000.0) as i64, bits, (dx * 100.0) as i32, (dy * 100.0) as i32)
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        self.inner.act(name, value)
    }

    pub fn numbers(&self) -> String {
        self.inner.numbers()
    }

    pub fn ui(&self) -> String {
        self.inner.ui()
    }

    pub fn sounds(&self) -> String {
        self.inner.sounds()
    }

    pub fn static_boxes(&self) -> Vec<f32> {
        floats(self.inner.static_boxes())
    }

    pub fn dynamic_boxes(&self) -> Vec<f32> {
        floats(self.inner.dynamic_boxes())
    }

    pub fn held_boxes(&self) -> Vec<f32> {
        floats(self.inner.held_boxes())
    }

    pub fn uniforms(&self) -> Vec<f32> {
        floats(self.inner.uniforms())
    }

    pub fn listener(&self) -> Vec<f32> {
        floats(self.inner.listener())
    }

    pub fn noise(&mut self, len: u32) -> Vec<f32> {
        floats(self.inner.noise(len))
    }

    pub fn impulse(&mut self, decay_ms: u32, rate: u32, len: u32) -> Vec<f32> {
        floats(self.inner.impulse(decay_ms, rate, len))
    }
}
