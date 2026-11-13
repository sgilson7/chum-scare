//! What the page draws and plays, built from the world. Every box is a
//! unit cube carried by an affine (17 numbers: three columns, a
//! translation, a colour, a material and a glow); lights, the flashlight
//! and its shadow's matrix are uniforms; each sound is placed for the
//! listener by `acoustics.rs`. The page only uploads these (CH-3, card:spec.R6).

use serde::Serialize;

use crate::fx::{self, int, Fx, ONE};
use crate::geom::{facing, v3, Affine, Mat4, V3};
use crate::lights::State;
use crate::mall::{self, FixtureKind, PropKind, Slab};
use crate::world::{
    chamber_at, Chamber, Mind, Mode, Output, Phase, World, BOX_AT, CYLINDER, LETTERS, POCKET,
};

pub const INSTANCE: usize = 17;
pub const MAX_LIGHTS: usize = 12;
pub const FOV: i32 = fx::deg(70);
pub const NEAR: Fx = ONE / 20;
pub const FAR: Fx = 120 * ONE;

fn push(out: &mut Vec<i32>, a: Affine, rgb: [i32; 3], material: i32, glow: Fx) {
    out.extend_from_slice(&[
        a.c0.x, a.c0.y, a.c0.z, a.c1.x, a.c1.y, a.c1.z, a.c2.x, a.c2.y, a.c2.z, a.t.x, a.t.y, a.t.z,
    ]);
    out.extend_from_slice(&[
        rgb[0] * ONE / 255,
        rgb[1] * ONE / 255,
        rgb[2] * ONE / 255,
        material * ONE,
        glow,
    ]);
}

/// A box from `lo` to `hi` in the frame `at`.
fn part(out: &mut Vec<i32>, at: &Affine, lo: V3, hi: V3, rgb: [i32; 3], material: i32, glow: Fx) {
    let c = v3((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, (lo.z + hi.z) / 2);
    let s = hi.sub(lo);
    let local = Affine::translate(c).then(&Affine::scale(s));
    push(out, at.then(&local), rgb, material, glow);
}

fn cm(n: i32) -> Fx {
    n * ONE / 100
}

/// What never moves: the mall's boxes, uploaded once.
pub fn static_boxes(slabs: &[Slab]) -> Vec<i32> {
    let mut out = Vec::new();
    for s in slabs {
        part(&mut out, &Affine::IDENTITY, s.lo, s.hi, s.rgb, s.material, 0);
    }
    out
}

fn at_yaw(p: V3, yaw: i32) -> Affine {
    Affine::translate(p).then(&Affine::rot_y(yaw))
}

fn mannequin(out: &mut Vec<i32>, f: &Affine, possessed: bool) {
    let skin = [196, 186, 170];
    part(
        out,
        f,
        v3(cm(-14), 0, cm(-6)),
        v3(cm(-4), cm(88), cm(6)),
        skin,
        mall::PLASTIC,
        0,
    );
    part(
        out,
        f,
        v3(cm(4), 0, cm(-6)),
        v3(cm(14), cm(88), cm(6)),
        skin,
        mall::PLASTIC,
        0,
    );
    part(
        out,
        f,
        v3(cm(-19), cm(88), cm(-10)),
        v3(cm(19), cm(150), cm(10)),
        skin,
        mall::PLASTIC,
        0,
    );
    part(
        out,
        f,
        v3(cm(-26), cm(95), cm(-5)),
        v3(cm(-19), cm(148), cm(5)),
        skin,
        mall::PLASTIC,
        0,
    );
    part(
        out,
        f,
        v3(cm(19), cm(95), cm(-5)),
        v3(cm(26), cm(148), cm(5)),
        skin,
        mall::PLASTIC,
        0,
    );
    part(
        out,
        f,
        v3(cm(-4), cm(150), cm(-4)),
        v3(cm(4), cm(158), cm(4)),
        skin,
        mall::PLASTIC,
        0,
    );
    part(
        out,
        f,
        v3(cm(-10), cm(158), cm(-11)),
        v3(cm(10), cm(182), cm(11)),
        skin,
        mall::PLASTIC,
        0,
    );
    if possessed {
        part(
            out,
            f,
            v3(cm(-6), cm(170), cm(-12)),
            v3(cm(-2), cm(173), cm(-11)),
            [255, 30, 20],
            mall::GLOW,
            ONE * 3,
        );
        part(
            out,
            f,
            v3(cm(2), cm(170), cm(-12)),
            v3(cm(6), cm(173), cm(-11)),
            [255, 30, 20],
            mall::GLOW,
            ONE * 3,
        );
    }
}

fn cultist(out: &mut Vec<i32>, p: V3, yaw: i32, dead: bool) {
    let robe = [64, 18, 20];
    let f = if dead {
        at_yaw(p, yaw)
            .then(&Affine::translate(v3(0, cm(20), 0)))
            .then(&Affine::rot_x(-fx::QUARTER_TURN))
    } else {
        at_yaw(p, yaw)
    };
    part(
        out,
        &f,
        v3(cm(-28), 0, cm(-22)),
        v3(cm(28), cm(145), cm(22)),
        robe,
        mall::FABRIC,
        0,
    );
    part(
        out,
        &f,
        v3(cm(-16), cm(145), cm(-16)),
        v3(cm(16), cm(178), cm(16)),
        [40, 10, 12],
        mall::FABRIC,
        0,
    );
    part(
        out,
        &f,
        v3(cm(-10), cm(152), cm(-17)),
        v3(cm(10), cm(170), cm(-16)),
        [210, 205, 190],
        mall::PLASTIC,
        0,
    );
    if !dead {
        part(
            out,
            &f,
            v3(cm(20), cm(95), cm(-30)),
            v3(cm(26), cm(115), cm(-24)),
            [235, 225, 200],
            mall::WOOD,
            0,
        );
        part(
            out,
            &f,
            v3(cm(21), cm(115), cm(-29)),
            v3(cm(25), cm(121), cm(-25)),
            [255, 180, 80],
            mall::GLOW,
            ONE * 4,
        );
    }
}

fn spirit(out: &mut Vec<i32>, p: V3, yaw: i32, size: Fx) {
    let s = fx::div(size, ONE * 11 / 10);
    let f = at_yaw(p, yaw).then(&Affine::scale(v3(s, s, s)));
    let dark = [10, 6, 6];
    part(
        out,
        &f,
        v3(cm(-16), 0, cm(-10)),
        v3(cm(16), cm(70), cm(10)),
        dark,
        mall::FABRIC,
        0,
    );
    part(
        out,
        &f,
        v3(cm(-13), cm(70), cm(-12)),
        v3(cm(13), cm(98), cm(12)),
        [214, 206, 196],
        mall::PLASTIC,
        0,
    );
    part(
        out,
        &f,
        v3(cm(-34), cm(30), cm(-4)),
        v3(cm(-16), cm(66), cm(4)),
        dark,
        mall::FABRIC,
        0,
    );
    part(
        out,
        &f,
        v3(cm(16), cm(30), cm(-4)),
        v3(cm(34), cm(66), cm(4)),
        dark,
        mall::FABRIC,
        0,
    );
    part(
        out,
        &f,
        v3(cm(-7), cm(84), cm(-13)),
        v3(cm(-3), cm(88), cm(-12)),
        [255, 20, 10],
        mall::GLOW,
        ONE * 4,
    );
    part(
        out,
        &f,
        v3(cm(3), cm(84), cm(-13)),
        v3(cm(7), cm(88), cm(-12)),
        [255, 20, 10],
        mall::GLOW,
        ONE * 4,
    );
}

fn car(out: &mut Vec<i32>, f: &Affine, possessed: bool) {
    let paint = [104, 30, 28];
    part(
        out,
        f,
        v3(cm(-90), cm(30), cm(-225)),
        v3(cm(90), cm(95), cm(225)),
        paint,
        mall::PAINT,
        0,
    );
    part(
        out,
        f,
        v3(cm(-80), cm(95), cm(-90)),
        v3(cm(80), cm(145), cm(110)),
        [40, 46, 52],
        mall::GLASS,
        0,
    );
    for (x, z) in [(-80, -150), (80, -150), (-80, 150), (80, 150)] {
        part(
            out,
            f,
            v3(cm(x - 12), 0, cm(z - 32)),
            v3(cm(x + 12), cm(62), cm(z + 32)),
            [20, 20, 20],
            mall::METAL,
            0,
        );
    }
    let glow = if possessed { ONE * 5 } else { 0 };
    part(
        out,
        f,
        v3(cm(-80), cm(60), cm(-227)),
        v3(cm(-50), cm(78), cm(-225)),
        [255, 240, 200],
        mall::GLOW,
        glow,
    );
    part(
        out,
        f,
        v3(cm(50), cm(60), cm(-227)),
        v3(cm(80), cm(78), cm(-225)),
        [255, 240, 200],
        mall::GLOW,
        glow,
    );
}

/// What moves or changes: doors, props, cultists, the spirit, debris.
pub fn dynamic_boxes(w: &World) -> Vec<i32> {
    let mut out = Vec::new();
    for (i, d) in w.mall.doors.iter().enumerate() {
        let wood = [78, 56, 40];
        if w.door_open[i] < ONE {
            part(&mut out, &Affine::IDENTITY, d.lo, d.hi, wood, mall::WOOD, 0);
            let a = (w.door_turns[i] as i64 * fx::TURN as i64 / ONE as i64) as i32;
            let hf = Affine::translate(d.handle).then(&Affine::rot_z(a));
            part(
                &mut out,
                &hf,
                v3(cm(-2), cm(-2), cm(-4)),
                v3(cm(2), cm(9), cm(4)),
                [150, 140, 110],
                mall::METAL,
                0,
            );
        } else {
            // swung open about its first corner
            let s = d.hi.sub(d.lo);
            let along_x = s.x > s.z;
            let lo = d.lo;
            let f = Affine::translate(lo).then(&Affine::rot_y(if along_x {
                -fx::QUARTER_TURN
            } else {
                fx::QUARTER_TURN
            }));
            part(&mut out, &f, V3::ZERO, s, wood, mall::WOOD, 0);
        }
    }
    for (pi, p) in w.mall.props.iter().enumerate() {
        let f = at_yaw(p.at, p.yaw);
        let possessed = w.possessed == Some(pi);
        match p.kind {
            PropKind::Mannequin => mannequin(&mut out, &f, possessed),
            PropKind::Car => car(&mut out, &f, possessed),
            PropKind::Rack => {
                part(
                    &mut out,
                    &f,
                    v3(cm(-60), cm(140), cm(-3)),
                    v3(cm(60), cm(144), cm(3)),
                    [120, 120, 118],
                    mall::METAL,
                    0,
                );
                part(
                    &mut out,
                    &f,
                    v3(cm(-62), 0, cm(-3)),
                    v3(cm(-58), cm(144), cm(3)),
                    [120, 120, 118],
                    mall::METAL,
                    0,
                );
                part(
                    &mut out,
                    &f,
                    v3(cm(58), 0, cm(-3)),
                    v3(cm(62), cm(144), cm(3)),
                    [120, 120, 118],
                    mall::METAL,
                    0,
                );
                for k in 0..5 {
                    let x = cm(-50 + k * 24);
                    let c = [
                        [90, 70, 60],
                        [60, 70, 90],
                        [100, 90, 60],
                        [70, 50, 70],
                        [80, 80, 80],
                    ][k as usize];
                    part(
                        &mut out,
                        &f,
                        v3(x - cm(8), cm(60), cm(-18)),
                        v3(x + cm(8), cm(138), cm(18)),
                        c,
                        mall::FABRIC,
                        0,
                    );
                }
            }
            PropKind::Ouija => {
                part(
                    &mut out,
                    &f,
                    v3(cm(-40), cm(80), cm(-28)),
                    v3(cm(40), cm(82), cm(28)),
                    [150, 112, 66],
                    mall::WOOD,
                    0,
                );
                for (k, l) in LETTERS.iter().enumerate() {
                    let lit = if (k as i32) < w.letters { ONE * 3 } else { 0 };
                    let c = v3(fx::mul(l.0, cm(36)), cm(82), -fx::mul(l.1, cm(50)));
                    part(
                        &mut out,
                        &f,
                        c.sub(v3(cm(3), 0, cm(3))),
                        c.add(v3(cm(3), cm(1), cm(3))),
                        [120, 10, 10],
                        mall::GLOW,
                        lit,
                    );
                }
                let h = if w.mode == Mode::Ouija {
                    w.hand.at
                } else {
                    (0, -ONE / 3)
                };
                let c = v3(fx::mul(h.0, cm(36)), cm(82), -fx::mul(h.1, cm(50)));
                part(
                    &mut out,
                    &f,
                    c.sub(v3(cm(6), 0, cm(5))),
                    c.add(v3(cm(6), cm(2), cm(5))),
                    [190, 170, 130],
                    mall::WOOD,
                    0,
                );
            }
            PropKind::Spare => {
                if !w.spare_taken {
                    part(
                        &mut out,
                        &f,
                        v3(cm(-6), cm(100), cm(-4)),
                        v3(cm(6), cm(108), cm(4)),
                        [170, 150, 96],
                        mall::WOOD,
                        ONE / 8,
                    );
                }
            }
            PropKind::Booth => {
                let c = [86, 30, 42];
                part(
                    &mut out,
                    &f,
                    v3(cm(-55), 0, cm(-55)),
                    v3(cm(-50), cm(210), cm(55)),
                    c,
                    mall::FABRIC,
                    0,
                );
                part(
                    &mut out,
                    &f,
                    v3(cm(50), 0, cm(-55)),
                    v3(cm(55), cm(210), cm(55)),
                    c,
                    mall::FABRIC,
                    0,
                );
                part(
                    &mut out,
                    &f,
                    v3(cm(-55), 0, cm(50)),
                    v3(cm(55), cm(210), cm(55)),
                    c,
                    mall::FABRIC,
                    0,
                );
                part(
                    &mut out,
                    &f,
                    v3(cm(-55), cm(205), cm(-55)),
                    v3(cm(55), cm(210), cm(55)),
                    [60, 50, 40],
                    mall::WOOD,
                    0,
                );
            }
            PropKind::Directory => {
                part(
                    &mut out,
                    &f,
                    v3(cm(-50), 0, cm(-8)),
                    v3(cm(50), cm(200), cm(8)),
                    [44, 42, 40],
                    mall::METAL,
                    0,
                );
                part(
                    &mut out,
                    &f,
                    v3(cm(-42), cm(90), cm(-9)),
                    v3(cm(42), cm(190), cm(-8)),
                    [190, 180, 150],
                    mall::GLOW,
                    ONE / 10,
                );
            }
            PropKind::Clock => {
                part(
                    &mut out,
                    &f,
                    v3(cm(-40), cm(330), cm(-6)),
                    v3(cm(40), cm(410), cm(6)),
                    [210, 200, 170],
                    mall::PLASTIC,
                    ONE / 20,
                );
            }
        }
    }
    // the fixtures themselves: lit, flickering or dead
    for (i, f) in w.mall.fixtures.iter().enumerate() {
        if f.kind == FixtureKind::Moon {
            continue;
        }
        let b = w.lights.brightness(i, flicks(w, i));
        let (lo, hi) = match f.kind {
            FixtureKind::Troffer => (v3(cm(-60), cm(-3), cm(-30)), v3(cm(60), cm(3), cm(30))),
            FixtureKind::Pendant => (v3(cm(-18), cm(-12), cm(-18)), v3(cm(18), cm(6), cm(18))),
            FixtureKind::Neon => (v3(cm(-90), cm(-6), cm(-2)), v3(cm(90), cm(6), cm(2))),
            FixtureKind::ExitSign => (v3(cm(-20), cm(-8), cm(-3)), v3(cm(20), cm(8), cm(3))),
            FixtureKind::Candle => (v3(cm(-2), cm(-10), cm(-2)), v3(cm(2), cm(2), cm(2))),
            _ => (v3(cm(-14), cm(-14), cm(-14)), v3(cm(14), cm(6), cm(14))),
        };
        let lit = if w.lights.states[i] == State::Broken {
            [40, 38, 34]
        } else {
            f.rgb
        };
        part(&mut out, &Affine::translate(f.at), lo, hi, lit, mall::GLOW, b * 2);
        if f.kind == FixtureKind::Pendant {
            part(
                &mut out,
                &Affine::translate(f.at),
                v3(-cm(1), cm(6), -cm(1)),
                v3(cm(1), cm(60), cm(1)),
                [30, 30, 30],
                mall::METAL,
                0,
            );
        }
    }
    for c in &w.cultists {
        cultist(&mut out, c.at, c.yaw, c.mind == Mind::Dead);
    }
    if w.spirit.chasing || w.spirit.glimpse > 0 {
        let toward = w.body.origin.sub(w.spirit.at);
        spirit(&mut out, w.spirit.at, crate::geom::yaw_of(toward), w.spirit.size);
    }
    for d in &w.debris {
        let f = at_yaw(d.at, d.yaw);
        part(
            &mut out,
            &f,
            v3(cm(-60), 0, cm(-30)),
            v3(cm(60), cm(4), cm(30)),
            [150, 146, 130],
            mall::CEILING,
            0,
        );
    }
    out
}

/// The camera: forward, right, up, and the matrix, with the flinch's jolt.
pub struct Camera {
    pub eye: V3,
    pub fwd: V3,
    pub right: V3,
    pub up: V3,
    pub vp: Mat4,
}

pub fn camera(w: &World, aspect: Fx, pending_yaw: i32, pending_pitch: i32) -> Camera {
    let jolt = fx::mul(w.flinch, fx::sin((w.tick as i32) * fx::TURN / 7)) / 40;
    let yaw = w.yaw + pending_yaw + jolt * 300;
    let pitch = (w.pitch + pending_pitch).clamp(-crate::world::PITCH_LIMIT, crate::world::PITCH_LIMIT);
    let mut eye = w.eye();
    // the walk's bob: a dip at each foot down
    let phase = (w.walked as i64 * fx::TURN as i64 / (crate::movement::TUNING.stride as i64 * 2)) as i32;
    if w.body.on_ground {
        eye.y -= fx::mul(fx::sin(phase).abs(), cm(2));
    }
    if let Mode::Hidden(h) = w.mode {
        let at = w.mall.hides[h];
        eye = v3(at.x, cm(160), at.z);
    }
    let fwd = facing(yaw, pitch);
    let right = fwd.cross(V3::UP).norm();
    let up = right.cross(fwd);
    let proj = Mat4::perspective(FOV, aspect, NEAR, FAR);
    let vp = proj.mul(&Mat4::look(eye, fwd, V3::UP));
    Camera {
        eye,
        fwd,
        right,
        up,
        vp,
    }
}

/// The hands, the revolver and the flashlight, in the world, before the camera.
pub fn held_boxes(w: &World, cam: &Camera) -> Vec<i32> {
    let mut out = Vec::new();
    if matches!(w.mode, Mode::Hidden(_)) || w.phase != Phase::Play {
        return out;
    }
    let frame = Affine {
        c0: cam.right,
        c1: cam.up,
        c2: cam.fwd.neg(),
        t: cam.eye,
    };
    let phase = (w.walked as i64 * fx::TURN as i64 / (crate::movement::TUNING.stride as i64 * 2)) as i32;
    let bob_y = -fx::mul(fx::sin(phase).abs(), cm(2));
    let sway = fx::mul(fx::sin(phase), cm(1));
    let breath = fx::mul(fx::sin((w.tick as i32) * fx::TURN / 240), cm(1) / 2);
    // the idle fidget: a turn of the wrist every few seconds (seeded by tick)
    let fid_t = (w.tick % 420) as i32;
    let fidget = if fid_t < 24 {
        fx::mul(fx::sin(fid_t * fx::TURN / 24), fx::deg(4))
    } else {
        0
    };
    let skin = [150, 112, 92];
    let steel = [110, 108, 104];
    // hand space to the view: ±0.3 m across and 0.18 m up a unit, centred a
    // little below the middle, 0.42 m out (the view's half-height there is
    // 0.42 × tan 35° = 0.29 m)
    let hand_xy = |h: (Fx, Fx)| v3(fx::mul(h.0, cm(30)), fx::mul(h.1, cm(18)) - cm(6), -cm(42));
    let hands = w.hands_busy();

    // the revolver, right hand
    let gun_at = if w.mode == Mode::Reload {
        v3(cm(8), cm(-17), -cm(40))
    } else {
        v3(cm(20) + sway, cm(-19) + bob_y + breath, -cm(42))
    };
    let raise = fx::mul(w.gun_up, fx::deg(55));
    let kick = w.revolver.kick * fx::deg(2);
    let tilt = if w.mode == Mode::Reload {
        fx::deg(15)
    } else {
        raise + kick
    };
    let roll = if w.mode == Mode::Reload {
        fx::deg(70)
    } else {
        fidget
    };
    if !matches!(w.mode, Mode::Door(_) | Mode::Ouija) {
        let g = frame
            .then(&Affine::translate(gun_at))
            .then(&Affine::rot_z(roll))
            .then(&Affine::rot_x(tilt));
        part(
            &mut out,
            &g,
            v3(cm(-2), cm(-1), -cm(18)),
            v3(cm(2), cm(2), cm(0)),
            steel,
            mall::METAL,
            0,
        );
        part(
            &mut out,
            &g,
            v3(cm(-2), cm(-3), cm(-1)),
            v3(cm(2), cm(3), cm(6)),
            steel,
            mall::METAL,
            0,
        );
        part(
            &mut out,
            &g,
            v3(cm(-2), cm(-12), cm(4)),
            v3(cm(2), cm(-2), cm(8)),
            [74, 46, 28],
            mall::WOOD,
            0,
        );
        part(
            &mut out,
            &g,
            v3(cm(-4), cm(-10), cm(3)),
            v3(cm(4), cm(-2), cm(9)),
            skin,
            mall::SKIN,
            0,
        );
        let cyl_x = if w.revolver.open { cm(-5) } else { 0 };
        part(
            &mut out,
            &g,
            v3(cyl_x - cm(3), cm(-3), cm(-2)),
            v3(cyl_x + cm(3), cm(3), cm(4)),
            [90, 90, 92],
            mall::METAL,
            0,
        );
    }
    if w.mode == Mode::Reload {
        // the cylinder face, its six chambers, the box, the hand
        let c = hand_xy(CYLINDER);
        let f = frame.then(&Affine::translate(c));
        part(
            &mut out,
            &f,
            v3(-cm(6), -cm(6), cm(-1)),
            v3(cm(6), cm(6), cm(0)),
            [86, 86, 88],
            mall::METAL,
            0,
        );
        for k in 0..6 {
            let p = hand_xy(chamber_at(k)).sub(c);
            let (rgb, glow) = match w.revolver.chambers[k] {
                Chamber::Round => ([200, 160, 70], ONE / 6),
                Chamber::Spent => ([120, 90, 50], 0),
                Chamber::Empty => ([12, 12, 12], 0),
            };
            part(
                &mut out,
                &f,
                p.sub(v3(cm(1), cm(1), cm(0))),
                p.add(v3(cm(1), cm(1), cm(1))),
                rgb,
                mall::METAL,
                glow,
            );
        }
        // the guide fades as rounds go in (journal tab 2)
        let guide = (ONE / 3 - w.loaded_now * ONE / 9).max(0);
        if !w.revolver.box_out {
            let p = hand_xy(POCKET);
            part(
                &mut out,
                &frame,
                p.sub(v3(cm(3), cm(3), 0)),
                p.add(v3(cm(3), cm(3), cm(1))),
                [60, 50, 40],
                mall::FABRIC,
                guide,
            );
        } else {
            let p = hand_xy(BOX_AT);
            part(
                &mut out,
                &frame,
                p.sub(v3(cm(4), cm(3), 0)),
                p.add(v3(cm(4), cm(2), cm(3))),
                [170, 150, 96],
                mall::WOOD,
                guide / 2,
            );
        }
        let h = hand_xy(w.hand.at);
        part(
            &mut out,
            &frame,
            h.sub(v3(cm(3), cm(4), 0)),
            h.add(v3(cm(3), cm(1), cm(3))),
            skin,
            mall::SKIN,
            0,
        );
        if w.revolver.holding {
            part(
                &mut out,
                &frame,
                h.sub(v3(cm(1) / 2, 0, cm(1))),
                h.add(v3(cm(1) / 2, cm(3), cm(1) / 2)),
                [200, 160, 70],
                mall::METAL,
                ONE / 6,
            );
        }
    }
    if let Mode::Door(_) = w.mode {
        let h = hand_xy((w.hand.at.0 / 3, w.hand.at.1 / 3 + ONE / 2));
        part(
            &mut out,
            &frame,
            h.sub(v3(cm(4), cm(4), 0)),
            h.add(v3(cm(4), cm(2), cm(4))),
            skin,
            mall::SKIN,
            0,
        );
    }
    // the flashlight, left hand: down to the side while the hands are busy
    let shake = if w.flashlight.shaking {
        w.flashlight.last_dx_sign * cm(3)
    } else {
        0
    };
    let fl_at = if hands {
        v3(cm(-26), cm(-32), -cm(30))
    } else {
        v3(cm(-22) - sway + shake, cm(-24) + bob_y - breath, -cm(40))
    };
    let fl = frame
        .then(&Affine::translate(fl_at))
        .then(&Affine::rot_x(if hands { -fx::deg(40) } else { 0 }));
    part(
        &mut out,
        &fl,
        v3(-cm(2), -cm(2), -cm(16)),
        v3(cm(2), cm(2), cm(2)),
        [40, 40, 38],
        mall::METAL,
        0,
    );
    let lens = if w.flashlight.on {
        w.flashlight.brightness(false) * 3
    } else {
        0
    };
    part(
        &mut out,
        &fl,
        v3(-cm(2), -cm(2), -cm(17)),
        v3(cm(2), cm(2), -cm(16)),
        [250, 230, 180],
        mall::GLOW,
        lens,
    );
    // the charge needle on its body
    let needle = fx::mul(
        ONE - fx::div(w.flashlight.charge, crate::lights::FULL),
        fx::deg(120),
    ) - fx::deg(60);
    let nf = fl
        .then(&Affine::translate(v3(0, cm(2), -cm(6))))
        .then(&Affine::rot_z(needle));
    part(
        &mut out,
        &nf,
        v3(-cm(1) / 4, 0, -cm(1) / 4),
        v3(cm(1) / 4, cm(1), cm(1) / 4),
        [200, 40, 30],
        mall::GLOW,
        ONE / 4,
    );
    part(
        &mut out,
        &fl,
        v3(-cm(3), -cm(4), -cm(4)),
        v3(cm(3), cm(1), cm(3)),
        skin,
        mall::SKIN,
        0,
    );
    // what the hands hold catches the beam's spill, so it reads in the dark
    let spill = w.flashlight.brightness(false) / 9 + ONE / 30;
    for k in (INSTANCE - 1..out.len()).step_by(INSTANCE) {
        out[k] = out[k].max(spill);
    }
    out
}

/// A light the page draws this frame: where, its reach, its colour times
/// its brightness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lit {
    pub at: V3,
    pub range: Fx,
    pub rgb: V3,
}

fn flicks(w: &World, i: usize) -> bool {
    let h = ((w.tick / 4) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (i as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    (h >> 59) < 4
}

pub fn lights(w: &World, eye: V3) -> Vec<Lit> {
    let mut all = Vec::new();
    for (i, f) in w.mall.fixtures.iter().enumerate() {
        let b = w.lights.brightness(i, flicks(w, i));
        if b == 0 {
            continue;
        }
        let k = fx::mul(f.power, b);
        all.push(Lit {
            at: f.at,
            range: f.range,
            rgb: v3(f.rgb[0] * k / 255, f.rgb[1] * k / 255, f.rgb[2] * k / 255),
        });
    }
    if let Some(pi) = w.possessed {
        let p = w.mall.props[pi];
        if p.kind == PropKind::Car {
            let ahead = facing(p.yaw, 0).scale(cm(260));
            all.push(Lit {
                at: p.at.add(ahead).add(v3(0, cm(70), 0)),
                range: int(14),
                rgb: v3(ONE, ONE * 9 / 10, ONE * 7 / 10),
            });
        }
    }
    for c in &w.cultists {
        if c.mind != Mind::Dead {
            all.push(Lit {
                at: c.at.add(v3(0, cm(120), 0)),
                range: int(4),
                rgb: v3(ONE / 4, ONE / 7, ONE / 20),
            });
        }
    }
    all.sort_by_key(|l| l.at.sub(eye).len() - l.range / 2);
    let keep = [4, 8, 12][w.settings.tier.clamp(0, 2) as usize];
    all.truncate(keep.min(MAX_LIGHTS));
    all
}

/// The uniforms in `numbers()`'s order (see `UNIFORMS`).
pub fn uniforms(w: &World, cam: &Camera) -> Vec<i32> {
    let mut u = Vec::new();
    u.extend_from_slice(&cam.vp.0);
    u.extend_from_slice(&[cam.eye.x, cam.eye.y, cam.eye.z]);
    let ls = lights(w, cam.eye);
    for k in 0..MAX_LIGHTS {
        match ls.get(k) {
            Some(l) => u.extend_from_slice(&[l.at.x, l.at.y, l.at.z, l.range]),
            None => u.extend_from_slice(&[0, 0, 0, ONE]),
        }
    }
    for k in 0..MAX_LIGHTS {
        match ls.get(k) {
            Some(l) => u.extend_from_slice(&[l.rgb.x, l.rgb.y, l.rgb.z, 0]),
            None => u.extend_from_slice(&[0, 0, 0, 0]),
        }
    }
    u.push(ls.len() as i32 * ONE);
    // the flashlight: from the left hand, along the view
    let flick = (w.tick / 3) % 5 == 0;
    let b = if matches!(w.mode, Mode::Hidden(_)) {
        0
    } else {
        w.flashlight.brightness(flick)
    };
    let from = cam.eye.add(cam.right.scale(-cm(20))).add(cam.up.scale(-cm(18)));
    let dir = cam.fwd;
    u.extend_from_slice(&[from.x, from.y, from.z, int(26)]);
    u.extend_from_slice(&[dir.x, dir.y, dir.z, fx::cos(fx::deg(24))]);
    let k = b * 2;
    u.extend_from_slice(&[
        fx::mul(k, ONE),
        fx::mul(k, ONE * 88 / 100),
        fx::mul(k, ONE * 70 / 100),
        fx::cos(fx::deg(9)),
    ]);
    let lp = Mat4::perspective(fx::deg(52), ONE, ONE / 10, int(30));
    let lv = Mat4::look(from, dir, V3::UP);
    u.extend_from_slice(&lp.mul(&lv).0);
    // ambient and fog: darker after the blackout
    let amb = if w.blackout.is_some() { ONE / 90 } else { ONE / 45 };
    u.extend_from_slice(&[amb, amb, fx::mul(amb, ONE * 12 / 10), ONE * 6 / 100]);
    u.extend_from_slice(&[ONE / 100, ONE / 110, ONE / 100, w.exposure]);
    u.extend_from_slice(&[(w.tick as i32) * ONE / 60, 0, 0, w.settings.tier * ONE]);
    u
}

/// Every uniform the scene shader takes: name, the WebGL call, how many
/// values. `uniforms()` writes them in this order.
pub const UNIFORMS: &[(&str, &str, usize)] = &[
    ("uVP", "uniformMatrix4fv", 16),
    ("uEye", "uniform3fv", 3),
    ("uLightPos", "uniform4fv", 4 * MAX_LIGHTS),
    ("uLightCol", "uniform4fv", 4 * MAX_LIGHTS),
    ("uLightCount", "uniform1fv", 1),
    ("uFlashPos", "uniform4fv", 4),
    ("uFlashDir", "uniform4fv", 4),
    ("uFlashCol", "uniform4fv", 4),
    ("uFlashVP", "uniformMatrix4fv", 16),
    ("uAmbient", "uniform4fv", 4),
    ("uFog", "uniform4fv", 4),
    ("uTime", "uniform4fv", 4),
];

/// A sound for the page: its recipe, where to put it, how loud, its
/// cut-off, and its reverb sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Placed {
    pub recipe: &'static str,
    pub x: Fx,
    pub y: Fx,
    pub z: Fx,
    /// thousandths
    pub gain: i32,
    pub hz: i32,
    pub room: usize,
    pub send: i32,
    pub here: usize,
    pub send_here: i32,
    pub seed: u32,
}

pub fn sounds(w: &World, cam: &Camera) -> Vec<Placed> {
    let mut out = Vec::new();
    let lr = w.mall.room_of(cam.eye);
    let here = w.mall.rooms[lr].reverb as usize;
    for (k, v) in w.voices.iter().enumerate() {
        let seed = (w.tick as u32)
            .wrapping_mul(2_654_435_761)
            .wrapping_add(k as u32 * 97);
        if v.head {
            // placed in the head's frame: right, up, back
            let p = cam
                .eye
                .add(cam.right.scale(v.at.x))
                .add(cam.up.scale(v.at.y))
                .add(cam.fwd.scale(-v.at.z));
            out.push(Placed {
                recipe: v.recipe,
                x: p.x,
                y: p.y,
                z: p.z,
                gain: v.loud * 1000 / ONE,
                hz: crate::acoustics::OPEN_HZ,
                room: here,
                send: fx::mul(v.loud, crate::acoustics::SEND_LISTENER) * 1000 / ONE,
                here,
                send_here: 0,
                seed,
            });
            continue;
        }
        let h = w.hear(v.at, cam.eye);
        let gain = fx::mul(h.gain, v.loud);
        out.push(Placed {
            recipe: v.recipe,
            x: h.apparent.x,
            y: h.apparent.y,
            z: h.apparent.z,
            gain: gain * 1000 / ONE,
            hz: h.cutoff_hz,
            room: w.mall.rooms[h.source_room].reverb as usize,
            send: fx::mul(gain, crate::acoustics::SEND_SOURCE) * 1000 / ONE,
            here,
            send_here: fx::mul(gain, crate::acoustics::SEND_LISTENER) * 1000 / ONE,
            seed,
        });
    }
    out
}

/// A sound that keeps playing (room tone, the chant, a buzz, the spirit's
/// breathing), placed every frame; the page starts, moves and stops it by id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bed {
    pub id: &'static str,
    pub recipe: &'static str,
    pub x: Fx,
    pub y: Fx,
    pub z: Fx,
    pub gain: i32,
    pub hz: i32,
}

pub fn beds(w: &World, cam: &Camera) -> Vec<Bed> {
    let mut out = Vec::new();
    let mut place = |id: &'static str, recipe: &'static str, at: V3, loud: Fx| {
        let h = w.hear(at, cam.eye);
        out.push(Bed {
            id,
            recipe,
            x: h.apparent.x,
            y: h.apparent.y,
            z: h.apparent.z,
            gain: fx::mul(h.gain, loud) * 1000 / ONE,
            hz: h.cutoff_hz,
        });
    };
    if w.phase != Phase::Play {
        return out;
    }
    if w.blackout.is_none() {
        place("hum", "hvac_hum", cam.eye.add(v3(0, int(2), 0)), ONE * 15 / 100);
        let chanting = w
            .cultists
            .iter()
            .filter(|c| matches!(c.mind, Mind::Chant | Mind::Look))
            .count() as i32;
        if chanting > 0 {
            place(
                "chant",
                "chant",
                w.mall.ring.add(v3(0, cm(160), 0)),
                ONE * chanting * 18 / 100,
            );
        }
        // the buzz of the nearest working fixture
        let near = w
            .mall
            .fixtures
            .iter()
            .enumerate()
            .filter(|(i, f)| {
                w.lights.states[*i] != State::Broken
                    && f.kind != FixtureKind::Moon
                    && f.kind != FixtureKind::Candle
            })
            .min_by_key(|(_, f)| f.at.sub(cam.eye).len());
        if let Some((_, f)) = near {
            place("buzz", "ballast_buzz", f.at, ONE * 12 / 100);
        }
    }
    place("drip", "drip", v3(int(52), cm(50), int(-12)), ONE * 35 / 100);
    place("wind", "wind", v3(int(75), int(9), 0), ONE * 30 / 100);
    if let Some(pi) = w.possessed {
        place(
            "possessed",
            "possessed_hum",
            w.mall.props[pi].at.add(v3(0, ONE, 0)),
            ONE * 45 / 100,
        );
    }
    if w.spirit.chasing {
        let s = w.spirit.size;
        place(
            "spirit",
            "spirit_drone",
            w.spirit.at.add(v3(0, s, 0)),
            ONE * 5 / 10 + s / 8,
        );
    }
    if w.breath < crate::movement::TUNING.breath_ticks / 2 {
        let h = Bed {
            id: "pant",
            recipe: "pant",
            x: cam.eye.x,
            y: cam.eye.y - cm(10),
            z: cam.eye.z,
            gain: 300,
            hz: crate::acoustics::OPEN_HZ,
        };
        out.push(h);
    }
    out
}

/// A mark on the direction ring, with its subtitle's key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RingMark {
    pub key: &'static str,
    /// angle from straight ahead, in degrees, positive to the right
    pub degrees: i32,
    /// thousandths of its life left
    pub life: i32,
}

/// A sign or line over the view: its key and where on screen (thousandths
/// from the left and top), its size in thousandths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SignAt {
    pub key: &'static str,
    pub x: i32,
    pub y: i32,
    pub size: i32,
}

/// The screen's description for the page (D5: the page looks each key up
/// in data/strings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ui {
    pub phase: &'static str,
    pub mode: &'static str,
    pub prompt: &'static str,
    pub letters: i32,
    /// thousandths of a second since the last letter slammed
    pub letter_ms: i64,
    pub blackout: bool,
    pub health: i32,
    pub hurt: bool,
    pub marks: Vec<RingMark>,
    pub signs: Vec<SignAt>,
    pub clock: String,
    pub output: &'static str,
    pub mono: bool,
    pub ring: bool,
    pub subtitles: bool,
    pub reduce_flashes: bool,
    pub sensitivity: i32,
    pub tier: i32,
    pub rounds: i32,
    pub box_rounds: i32,
    pub deaths: i32,
    /// centimetres walked since the doors, for the browser check
    pub walked_cm: i32,
}

pub fn clock(minute_of_day: i32, tick: i64) -> String {
    let m = (minute_of_day as i64 + tick / (60 * crate::world::HZ)).rem_euclid(24 * 60) as i32;
    let h = m / 60;
    let (h12, half) = if h == 0 {
        (12, "a.m.")
    } else if h < 12 {
        (h, "a.m.")
    } else if h == 12 {
        (12, "p.m.")
    } else {
        (h - 12, "p.m.")
    };
    format!("{}:{:02} {}", h12, m % 60, half)
}

pub fn ui(w: &World, cam: &Camera) -> Ui {
    let marks = w
        .marks
        .iter()
        .map(|m| {
            // the mark's bearing from the view now, so it turns with you
            let rel = fx::wrap(m.angle - w.yaw);
            RingMark {
                key: m.key,
                degrees: -(rel as i64 * 360 / fx::TURN as i64) as i32,
                life: (1000 - (w.tick - m.tick) * 1000 / 150).max(0) as i32,
            }
        })
        .collect();
    let mut signs = Vec::new();
    for s in &w.mall.signs {
        let d = s.at.sub(cam.eye).len();
        if d > int(14) || w.phase != Phase::Play {
            continue;
        }
        let c = cam.vp.project(s.at);
        if c[3] <= 0 {
            continue;
        }
        let x = fx::div(c[0], c[3]);
        let y = fx::div(c[1], c[3]);
        if x.abs() > ONE || y.abs() > ONE {
            continue;
        }
        let mut sc = crate::trace::Scratch::default();
        let clear = !crate::trace::trace_box(&w.mall.map, &mut sc, cam.eye, s.at, V3::ZERO, V3::ZERO).hit();
        if !clear {
            continue;
        }
        signs.push(SignAt {
            key: s.key,
            x: ((x + ONE) as i64 * 500 / ONE as i64) as i32,
            y: ((ONE - y) as i64 * 500 / ONE as i64) as i32,
            size: (fx::div(int(4), d.max(ONE)) as i64 * 1000 / ONE as i64).min(3000) as i32,
        });
    }
    let loaded = w
        .revolver
        .chambers
        .iter()
        .filter(|c| **c == Chamber::Round)
        .count() as i32;
    Ui {
        phase: match w.phase {
            Phase::Title => "title",
            Phase::Play => "play",
            Phase::Dead => "dead",
            Phase::Won => "won",
        },
        mode: match w.mode {
            Mode::Look => "look",
            Mode::Reload => "reload",
            Mode::Door(_) => "door",
            Mode::Ouija => "ouija",
            Mode::Hidden(_) => "hidden",
        },
        prompt: w.prompt,
        letters: w.letters,
        letter_ms: (w.tick - w.letter_tick) * 1000 / crate::world::HZ,
        blackout: w.blackout.is_some(),
        health: w.health,
        hurt: w.hurt > 0,
        marks,
        signs,
        clock: clock(w.minute_of_day, w.tick),
        output: match w.settings.output {
            Output::Headphones => "headphones",
            Output::Speakers => "speakers",
            Output::Mono => "mono",
        },
        mono: w.settings.output == Output::Mono,
        ring: w.settings.ring,
        subtitles: w.settings.subtitles,
        reduce_flashes: w.settings.reduce_flashes,
        sensitivity: w.settings.sensitivity,
        tier: w.settings.tier,
        rounds: loaded,
        box_rounds: w.revolver.box_rounds,
        deaths: w.deaths,
        walked_cm: (w.walked as i64 * 100 / ONE as i64) as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broken_fixture_is_not_in_the_light_list() {
        let mut w = World::new(1, 0);
        w.act("start", "").unwrap();
        let eye = w.mall.fixtures[0].at;
        let before = lights(&w, eye).iter().any(|l| l.at == w.mall.fixtures[0].at);
        assert!(before);
        let f = w.mall.fixtures.clone();
        w.lights.break_one(0, &f);
        assert!(!lights(&w, eye).iter().any(|l| l.at == w.mall.fixtures[0].at));
    }

    #[test]
    fn the_uniforms_fill_every_slot_the_list_names() {
        let mut w = World::new(1, 0);
        w.act("start", "").unwrap();
        let cam = camera(&w, ONE * 16 / 9, 0, 0);
        let n: usize = UNIFORMS.iter().map(|u| u.2).sum();
        assert_eq!(uniforms(&w, &cam).len(), n);
    }

    #[test]
    fn the_clock_reads_the_computers_time_and_moves_with_the_game() {
        // 2:13 a.m. is minute 133; ten minutes of ticks later, 2:23
        assert_eq!(clock(133, 0), "2:13 a.m.");
        assert_eq!(clock(133, 10 * 60 * 60), "2:23 a.m.");
        assert_eq!(clock(0, 0), "12:00 a.m.");
        assert_eq!(clock(13 * 60 + 5, 0), "1:05 p.m.");
    }
}
