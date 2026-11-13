//! One run of Chum Scare: the order of a tick and every rule the journal's
//! tab 2 describes. The player walks (`movement.rs`, horror TUNING), loads
//! the revolver with the hands while the scare director works behind them,
//! turns door handles, avoids the cultists, uses the ouija board, survives
//! the chase and leaves. Chance comes only from the seed (CH-3).

use crate::acoustics::{self, Heard};
use crate::fx::{self, int, Fx, ONE};
use crate::geom::{facing, v3, yaw_of, V3};
use crate::input::{Input, ALT, CROUCH, FIRE, FORWARD, SHAKE, SPRINT, USE};
use crate::lights::{Flashlight, Lights};
use crate::mall::{self, mall, FixtureKind, Mall, PropKind, ScareKind};
use crate::movement::{Body, Mover, TUNING};
use crate::rng::Rng;
use crate::trace::{trace_box, Scratch};

pub const HZ: i64 = 60;
/// The view's half-angles, for "outside the view" (CH-4): 50° across the
/// widest screen we draw for, and 20° more for the margin.
pub const HALF_VIEW: i32 = fx::deg(58);
pub const SCARE_MARGIN: i32 = fx::deg(20);
/// Pitch below which the right button brings the hands down to the gun.
pub const LOOK_DOWN: i32 = fx::deg(-40);
pub const PITCH_LIMIT: i32 = fx::deg(85);
/// Mouse hundredths of a pixel to angle units, at sensitivity 100.
pub const TURN_PER_PX: i32 = 20;
/// Mouse hundredths of a pixel to hand units (`ONE` across half the view).
pub const HAND_PER_PX: i32 = 3;
/// How near (m) a handle, the board, a booth or the box must be to use.
pub const REACH: Fx = ONE * 19 / 10;
pub const HEALTH: i32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Title,
    Play,
    Dead,
    Won,
}

/// What the mouse is steering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Look,
    Reload,
    Door(usize),
    Ouija,
    Hidden(usize),
}

/// A chamber: empty, a round, or a spent case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chamber {
    Empty,
    Round,
    Spent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revolver {
    pub chambers: [Chamber; 6],
    /// the chamber under the hammer
    pub at: usize,
    pub open: bool,
    pub box_out: bool,
    pub holding: bool,
    pub box_rounds: i32,
    pub kick: i32,
}

/// The hand while it is out: where it is and was (Verlet), and where the
/// mouse wants it, in screen units (`ONE` is half the view's height).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hand {
    pub at: (Fx, Fx),
    pub was: (Fx, Fx),
    pub want: (Fx, Fx),
}

/// Where the reload's hand targets sit (screen units).
pub const POCKET: (Fx, Fx) = (-ONE * 75 / 100, -ONE * 70 / 100);
pub const BOX_AT: (Fx, Fx) = (-ONE * 45 / 100, -ONE * 35 / 100);
pub const CYLINDER: (Fx, Fx) = (ONE * 20 / 100, -ONE * 15 / 100);
pub const CHAMBER_R: Fx = ONE * 16 / 100;
pub const GRAB: Fx = ONE * 12 / 100;
pub const SEAT: Fx = ONE * 8 / 100;
/// A flick: the hand's speed across in one tick (screen units).
pub const FLICK: Fx = ONE * 7 / 100;
/// The ouija's letters on the board (screen units), and the pull.
pub const LETTERS: [(Fx, Fx); 3] = [
    (-ONE * 45 / 100, ONE * 15 / 100),
    (ONE * 5 / 100, ONE * 32 / 100),
    (ONE * 48 / 100, ONE * 8 / 100),
];
pub const PULL: Fx = ONE * 4 / 100;
/// Closing takes a harder flick than opening, so reaching for the box
/// does not shut the cylinder (R2.5).
pub const SHUT_FLICK: Fx = ONE * 14 / 100;
/// The handle's angle before the hand has one.
pub const NO_ANGLE: i32 = i32::MIN;

pub fn chamber_at(k: usize) -> (Fx, Fx) {
    let a = (k as i32) * fx::TURN / 6 + fx::QUARTER_TURN;
    (
        CYLINDER.0 + fx::mul(CHAMBER_R, fx::cos(a)),
        CYLINDER.1 + fx::mul(CHAMBER_R, fx::sin(a)),
    )
}

fn d2((ax, ay): (Fx, Fx), (bx, by): (Fx, Fx)) -> Fx {
    fx::length(ax - bx, ay - by)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mind {
    Chant,
    Look,
    Search,
    Attack,
    Flee,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cultist {
    pub at: V3,
    pub yaw: i32,
    pub post: V3,
    pub mind: Mind,
    pub suspicion: Fx,
    pub goal: V3,
    pub quiet: i32,
    pub cooldown: i32,
    pub patrol: bool,
    pub angle: i32,
    pub scream: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spirit {
    pub at: V3,
    pub size: Fx,
    pub chasing: bool,
    /// ticks since it last perceived the player
    pub unseen: i32,
    pub last_known: V3,
    pub prey: Option<usize>,
    pub glimpse: i32,
    pub chase_ticks: i32,
}

/// A sound made this tick, before it is placed for the listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voice {
    pub recipe: &'static str,
    pub at: V3,
    /// loudness at the source, `ONE` for a gunshot
    pub loud: Fx,
    /// in the head: placed relative to the listener, not the world
    pub head: bool,
    /// shown on the direction ring with a subtitle
    pub marked: Option<&'static str>,
}

/// An important sound for the ring and subtitles: its key, its angle from
/// the view (angle units), and the tick it was heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    pub key: &'static str,
    pub angle: i32,
    pub tick: i64,
}

/// Debris the director leaves (a fallen tile, a rack on its side).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Debris {
    pub at: V3,
    pub yaw: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Headphones,
    Speakers,
    Mono,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub output: Output,
    pub ring: bool,
    pub subtitles: bool,
    pub reduce_flashes: bool,
    /// percent
    pub sensitivity: i32,
    /// 0 low, 1 medium, 2 high
    pub tier: i32,
    /// the whole mix, percent (T3)
    pub volume: i32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            output: Output::Headphones,
            ring: false,
            subtitles: false,
            reduce_flashes: false,
            sensitivity: 100,
            tier: 1,
            volume: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct World {
    pub mall: Mall,
    pub phase: Phase,
    pub mode: Mode,
    pub body: Body,
    pub yaw: i32,
    pub pitch: i32,
    pub tick: i64,
    pub rng: Rng,
    pub revolver: Revolver,
    pub hand: Hand,
    pub flashlight: Flashlight,
    pub lights: Lights,
    /// where each sign faces; fixed until the chase turns them (T4)
    pub sign_yaws: Vec<i32>,
    pub door_open: Vec<Fx>,
    pub door_turns: Vec<Fx>,
    pub door_angle: i32,
    pub cultists: Vec<Cultist>,
    pub spirit: Spirit,
    /// the prop index the spirit is in, and whether it was seen last tick
    pub possessed: Option<usize>,
    pub was_seen: bool,
    pub possess_wait: i32,
    pub car_done: bool,
    pub armed: bool,
    pub loaded_now: i32,
    pub scare_wait: i32,
    pub pending_steps: Vec<(i64, V3)>,
    pub debris: Vec<Debris>,
    pub voices: Vec<Voice>,
    pub marks: Vec<Mark>,
    pub health: i32,
    pub hurt: i32,
    pub flinch: Fx,
    pub breath: i32,
    pub stride: Fx,
    pub step_side: bool,
    pub walked: Fx,
    pub spare_taken: bool,
    pub letters: i32,
    pub letter_tick: i64,
    pub blackout: Option<i64>,
    pub old_buttons: u32,
    pub old_keys: u32,
    pub settings: Settings,
    pub checkpoint: Option<Box<World>>,
    pub prompt: &'static str,
    pub seen_hide: bool,
    pub minute_of_day: i32,
    pub deaths: i32,
    /// the gun's muzzle raised (`ONE`) or level (0): up while walking
    pub gun_up: Fx,
    /// the eye's adaptation to the dark, `ONE` for a lit room
    pub exposure: Fx,
}

impl World {
    pub fn new(seed: u64, minute_of_day: i32) -> World {
        let m = mall();
        let mut cultists = Vec::new();
        for (k, p) in m.cultist_posts.iter().enumerate() {
            let face = yaw_of(m.ring.sub(*p));
            cultists.push(Cultist {
                at: *p,
                yaw: face,
                post: *p,
                mind: Mind::Chant,
                suspicion: 0,
                goal: *p,
                quiet: 0,
                cooldown: 0,
                patrol: k == 1 || k == 4,
                angle: (k as i32) * fx::TURN / 6,
                scream: 0,
            });
        }
        let body = Body::at(m.spawn);
        World {
            phase: Phase::Title,
            mode: Mode::Look,
            body,
            yaw: m.spawn_yaw,
            pitch: 0,
            tick: 0,
            rng: Rng::new(seed),
            revolver: Revolver {
                chambers: [Chamber::Empty; 6],
                at: 0,
                open: false,
                box_out: false,
                holding: false,
                box_rounds: 6,
                kick: 0,
            },
            hand: Hand::default(),
            flashlight: Flashlight::default(),
            lights: Lights::new(&m.fixtures),
            sign_yaws: m.signs.iter().map(|s| s.yaw).collect(),
            door_open: vec![0; m.doors.len()],
            door_turns: vec![0; m.doors.len()],
            door_angle: 0,
            cultists,
            spirit: Spirit {
                at: v3(int(96), 0, int(3)),
                size: ONE * 11 / 10,
                chasing: false,
                unseen: 0,
                last_known: V3::ZERO,
                prey: None,
                glimpse: 0,
                chase_ticks: 0,
            },
            possessed: None,
            was_seen: true,
            possess_wait: 600,
            car_done: false,
            armed: false,
            loaded_now: 0,
            scare_wait: 0,
            pending_steps: Vec::new(),
            debris: Vec::new(),
            voices: Vec::new(),
            marks: Vec::new(),
            health: HEALTH,
            hurt: 0,
            flinch: 0,
            breath: TUNING.breath_ticks,
            stride: 0,
            step_side: false,
            walked: 0,
            spare_taken: false,
            letters: 0,
            letter_tick: 0,
            blackout: None,
            old_buttons: 0,
            old_keys: 0,
            settings: Settings::default(),
            checkpoint: None,
            prompt: "",
            seen_hide: false,
            minute_of_day,
            deaths: 0,
            gun_up: ONE,
            exposure: ONE,
            mall: m,
        }
    }

    pub fn eye(&self) -> V3 {
        self.body.origin.add(v3(0, self.body.eye(), 0))
    }

    pub fn facing(&self) -> V3 {
        facing(self.yaw, self.pitch)
    }

    pub fn menace(&self) -> i32 {
        self.mall.menace_at(self.body.origin)
    }

    pub fn hands_busy(&self) -> bool {
        matches!(self.mode, Mode::Reload | Mode::Door(_) | Mode::Ouija)
    }

    /// The angle between where the player looks and `p`, in angle units.
    pub fn off_view(&self, p: V3) -> i32 {
        off_view(self.yaw, self.pitch, self.eye(), p)
    }

    fn clear_line(&self, a: V3, b: V3) -> bool {
        let mut sc = Scratch::default();
        !trace_box(&self.mall.map, &mut sc, a, b, V3::ZERO, V3::ZERO).hit()
    }

    /// Whether the player sees `p`: in front, inside the view, nothing in
    /// the way, and not hiding.
    pub fn sees(&self, p: V3) -> bool {
        !matches!(self.mode, Mode::Hidden(_))
            && self.off_view(p) < HALF_VIEW
            && self.clear_line(self.eye(), p)
    }

    /// How `from` is heard at `to`.
    pub fn hear(&self, from: V3, to: V3) -> Heard {
        let blocked = !self.clear_line(v3(from.x, ONE, from.z), v3(to.x, ONE, to.z));
        acoustics::hear(
            &self.mall.rooms,
            &self.mall.portals,
            &self.door_open,
            from,
            to,
            blocked,
        )
    }

    fn say(&mut self, recipe: &'static str, at: V3, loud: Fx, marked: Option<&'static str>) {
        self.voices.push(Voice {
            recipe,
            at,
            loud,
            head: false,
            marked,
        });
        if let Some(key) = marked {
            let bearing = yaw_of(at.sub(self.eye()));
            self.marks.push(Mark {
                key,
                angle: bearing,
                tick: self.tick,
            });
        }
    }

    fn say_head(&mut self, recipe: &'static str, offset: V3, loud: Fx) {
        self.voices.push(Voice {
            recipe,
            at: offset,
            loud,
            head: true,
            marked: None,
        });
    }

    /// Settings and screens, from the page (`act`).
    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "start" => {
                if self.phase == Phase::Title {
                    self.phase = Phase::Play;
                    self.checkpoint = Some(Box::new(self.snapshot()));
                }
            }
            "retry" => {
                if let Some(c) = self.checkpoint.clone() {
                    let settings = self.settings;
                    let deaths = self.deaths;
                    let keep = c.clone();
                    *self = *c;
                    self.settings = settings;
                    self.deaths = deaths;
                    self.checkpoint = Some(keep);
                    self.phase = Phase::Play;
                }
            }
            "again" => {
                let settings = self.settings;
                let seed = self.rng.next_u64();
                *self = World::new(seed, self.minute_of_day);
                self.settings = settings;
            }
            "output" => {
                self.settings.output = match value {
                    "headphones" => Output::Headphones,
                    "speakers" => Output::Speakers,
                    "mono" => {
                        // CH-Q16 A: Mono brings the ring and subtitles
                        self.settings.ring = true;
                        self.settings.subtitles = true;
                        Output::Mono
                    }
                    _ => return Err(format!("unknown output {value}")),
                }
            }
            "ring" => self.settings.ring = value == "on",
            "subtitles" => self.settings.subtitles = value == "on",
            "flashes" => self.settings.reduce_flashes = value == "reduce",
            "sensitivity" => {
                let v: i32 = value.parse().map_err(|_| format!("bad sensitivity {value}"))?;
                self.settings.sensitivity = v.clamp(20, 300);
            }
            "tier" => {
                let v: i32 = value.parse().map_err(|_| format!("bad tier {value}"))?;
                self.settings.tier = v.clamp(0, 2);
            }
            "volume" => {
                let v: i32 = value.parse().map_err(|_| format!("bad volume {value}"))?;
                self.settings.volume = v.clamp(0, 100);
            }
            "clock" => {
                let v: i32 = value.parse().map_err(|_| format!("bad clock {value}"))?;
                self.minute_of_day = v.rem_euclid(24 * 60);
            }
            "settings" => {
                // CH-Q6 A (kits/saves, settings only): text the page kept;
                // anything unreadable leaves the defaults
                self.settings = settings_from(value).unwrap_or(self.settings);
            }
            _ => return Err(format!("unknown action {name}")),
        }
        Ok(())
    }

    fn snapshot(&self) -> World {
        let mut w = self.clone();
        w.checkpoint = None;
        w.voices.clear();
        w.marks.clear();
        w
    }

    /// One tick. Voices and marks from the last tick are cleared first.
    pub fn tick(&mut self, input: &Input) {
        self.voices.clear();
        self.tick += 1;
        self.marks.retain(|m| self.tick - m.tick < 150);
        if self.phase != Phase::Play {
            self.old_buttons = input.buttons;
            self.old_keys = input.keys;
            return;
        }
        let fresh_button = |b: u32| input.button(b) && self.old_buttons & b == 0;
        let fresh_fire = fresh_button(FIRE);
        let alt = input.button(ALT);
        let fresh_alt = fresh_button(ALT);
        let fresh_use = input.key(USE) && self.old_keys & USE == 0;
        self.old_buttons = input.buttons;
        self.old_keys = input.keys;
        self.prompt = "";

        // the flashlight first: while F is held the mouse shakes it
        let shake_held = input.key(SHAKE);
        let ev = self.flashlight.tick(shake_held, input.dx);
        if ev.pumped {
            let p = v3(-ONE / 4, -ONE / 3, -ONE / 3);
            self.say_head("dynamo", p, ONE * 5 / 10);
            self.noise(self.body.origin, ONE * 35 / 100);
        }
        if ev.toggled {
            self.say_head("click_light", v3(-ONE / 4, -ONE / 3, -ONE / 3), ONE / 3);
        }

        self.modes(input, alt, fresh_alt, fresh_use);
        self.steer(input, shake_held);
        if !matches!(self.mode, Mode::Hidden(_) | Mode::Ouija) {
            self.walk(input);
        }
        match self.mode {
            Mode::Reload => self.reload(),
            Mode::Door(dn) => self.turn_handle(dn),
            Mode::Ouija => self.ouija(),
            _ => {}
        }
        if fresh_fire && self.mode == Mode::Look && !self.flashlight.shaking {
            self.fire();
        }
        self.revolver.kick = (self.revolver.kick - 1).max(0);
        self.flinch = fx::approach(self.flinch, 0, ONE / 36);
        self.hurt = (self.hurt - 1).max(0);

        self.director();
        self.possession();
        self.cult();
        self.chase();
        let broke = self.lights.advance(self.tick, &self.mall.fixtures);
        for i in broke {
            let at = self.mall.fixtures[i].at;
            self.say("pop_glass", at, ONE * 7 / 10, None);
        }
        self.steps_due();
        let moving = self.body.speed() > ONE / 3;
        let want = if moving && self.revolver.kick == 0 { ONE } else { 0 };
        self.gun_up = fx::approach(self.gun_up, want, ONE / 24);
        // the eye adapts over about three seconds
        let lit = self.lit_at(self.eye()).min(ONE * 2);
        self.exposure = fx::approach(self.exposure, ONE * 3 / 2 - lit / 2, ONE / 180);
        self.outcome();
    }

    fn modes(&mut self, input: &Input, alt: bool, fresh_alt: bool, fresh_use: bool) {
        // hiding: E in and out
        if let Mode::Hidden(h) = self.mode {
            self.prompt = "prompt_leave_hide";
            if fresh_use {
                self.mode = Mode::Look;
                self.body.origin = self.mall.hides[h].add(v3(0, 0, -ONE * 12 / 10));
                self.say("booth", self.mall.hides[h], ONE / 3, None);
            }
            return;
        }
        let origin = self.body.origin;
        let near_hide = self
            .mall
            .hides
            .iter()
            .position(|h| h.sub(origin).len_xz() < REACH);
        let spare = self
            .mall
            .props
            .iter()
            .find(|p| p.kind == PropKind::Spare)
            .map(|p| p.at);
        let near_spare = !self.spare_taken && spare.map(|s| s.sub(origin).len_xz() < REACH).unwrap_or(false);
        if self.mode == Mode::Look {
            if near_spare {
                self.prompt = "prompt_take_box";
            } else if near_hide.is_some() {
                self.prompt = "prompt_hide";
            }
        }
        if fresh_use && self.mode == Mode::Look {
            if near_spare {
                // CH-Q8 B: one more box of six, off the route
                self.spare_taken = true;
                self.revolver.box_rounds += 6;
                self.say("box_rattle", origin, ONE / 3, None);
                return;
            }
            if let Some(h) = near_hide {
                self.seen_hide = self.spirit.chasing && self.spirit.unseen == 0;
                self.mode = Mode::Hidden(h);
                self.flashlight.on = false;
                self.body.velocity = V3::ZERO;
                self.say("booth", self.mall.hides[h], ONE / 3, None);
                return;
            }
        }
        if fresh_alt && self.mode == Mode::Look {
            let eye = self.eye();
            let door = self.mall.doors.iter().enumerate().position(|(i, d)| {
                self.door_open[i] < ONE
                    && d.handle.sub(eye).len() < REACH
                    && self.off_view(d.handle) < fx::deg(35)
            });
            let board = self
                .mall
                .props
                .iter()
                .find(|p| p.kind == PropKind::Ouija)
                .map(|p| p.at.add(v3(0, ONE * 8 / 10, 0)));
            let near_board = board
                .map(|b| b.sub(eye).len() < REACH * 3 / 2 && self.off_view(b) < fx::deg(40))
                .unwrap_or(false);
            if let Some(dn) = door {
                self.mode = Mode::Door(dn);
                self.hand = Hand {
                    at: (0, 0),
                    was: (0, 0),
                    want: (0, ONE / 4),
                };
                self.door_angle = NO_ANGLE;
            } else if near_board && self.letters < 3 && self.blackout.is_none() {
                self.mode = Mode::Ouija;
                self.hand = Hand {
                    at: (0, -ONE / 3),
                    was: (0, -ONE / 3),
                    want: (0, -ONE / 3),
                };
            } else if self.pitch < LOOK_DOWN {
                self.mode = Mode::Reload;
                self.loaded_now = 0;
                self.hand = Hand {
                    at: (ONE / 2, -ONE / 2),
                    was: (ONE / 2, -ONE / 2),
                    want: (ONE / 2, -ONE / 2),
                };
            }
        }
        if !alt && self.hands_busy() {
            if self.mode == Mode::Reload {
                if self.revolver.holding {
                    self.revolver.holding = false;
                    self.revolver.box_rounds += 1;
                }
                if self.revolver.open {
                    self.revolver.open = false;
                    self.say_head("cylinder_close", v3(ONE / 5, -ONE / 3, -ONE / 2), ONE / 3);
                }
                self.revolver.box_out = false;
            }
            if let Mode::Door(dn) = self.mode {
                if self.door_open[dn] < ONE {
                    self.door_turns[dn] = 0;
                }
            }
            if self.mode == Mode::Ouija && self.letters < 3 {
                self.letters = 0;
            }
            self.mode = Mode::Look;
        }
        let _ = input;
    }

    /// The mouse: the view, or the hand when it is out.
    fn steer(&mut self, input: &Input, shaking: bool) {
        let s = self.settings.sensitivity;
        if self.hands_busy() {
            let k = HAND_PER_PX * s / 100;
            let w = self.hand.want;
            self.hand.want = (
                (w.0 + input.dx * k).clamp(-ONE, ONE),
                (w.1 - input.dy * k).clamp(-ONE, ONE),
            );
            // Verlet: carry on as it was moving, damped, and pull toward the mouse
            let (ax, ay) = self.hand.at;
            let (px, py) = self.hand.was;
            let (tx, ty) = self.hand.want;
            let nx = ax + fx::mul(ax - px, ONE * 55 / 100) + fx::mul(tx - ax, ONE * 30 / 100);
            let ny = ay + fx::mul(ay - py, ONE * 55 / 100) + fx::mul(ty - ay, ONE * 30 / 100);
            self.hand.was = self.hand.at;
            self.hand.at = (nx, ny);
            return;
        }
        if shaking {
            return;
        }
        let k = TURN_PER_PX * s / 100;
        let hidden = matches!(self.mode, Mode::Hidden(_));
        let dx = if hidden { input.dx / 3 } else { input.dx };
        self.yaw = (self.yaw - dx * k / 100).rem_euclid(fx::TURN);
        let lim = if hidden { fx::deg(25) } else { PITCH_LIMIT };
        self.pitch = (self.pitch - input.dy * k / 100).clamp(-lim, lim);
    }

    fn walk(&mut self, input: &Input) {
        let mut mover = Mover::new(&self.mall.map);
        let sprinting = input.key(SPRINT)
            && input.key(FORWARD)
            && self.breath > 0
            && !input.key(CROUCH)
            && !self.hands_busy();
        if sprinting {
            mover.t.run = TUNING.sprint;
            self.breath -= 1;
            if self.breath == 0 {
                self.voices.push(Voice {
                    recipe: "gasp",
                    at: v3(0, -ONE / 10, -ONE / 5),
                    loud: ONE / 2,
                    head: true,
                    marked: None,
                });
            }
        } else {
            self.breath = (self.breath + TUNING.breath_ticks / TUNING.breath_refill_ticks.max(1) + 1)
                .min(TUNING.breath_ticks);
        }
        if self.hands_busy() {
            // loading or turning a handle: a slow shuffle
            mover.t.run = TUNING.run / 2;
        }
        let before = self.body.origin;
        mover.tick(&mut self.body, input, self.yaw, self.yaw);
        let moved = self.body.origin.sub(before).len_xz();
        if self.body.on_ground && moved > 0 {
            self.walked += moved;
            self.stride += moved;
            if self.stride >= TUNING.stride / 2 {
                self.stride -= TUNING.stride / 2;
                self.step_side = !self.step_side;
                let floor = self.mall.rooms[self.mall.room_of(self.body.origin)].floor;
                let recipe = match floor {
                    mall::CARPET => "step_carpet",
                    mall::CONCRETE => "step_concrete",
                    _ => "step_tile",
                };
                let loud = if self.body.ducked {
                    ONE * 25 / 100
                } else if sprinting {
                    ONE
                } else {
                    ONE * 60 / 100
                };
                let side = if self.step_side { ONE / 6 } else { -ONE / 6 };
                self.say_head(recipe, v3(side, -ONE * 15 / 10, -ONE / 5), loud);
                if self.revolver.box_rounds > 0 && !self.body.ducked {
                    self.say_head("box_rattle", v3(-ONE / 4, -ONE, 0), ONE * 12 / 100);
                }
                self.noise(self.body.origin, loud);
            }
        }
    }

    /// A noise the cultists and the spirit can hear: the player's steps,
    /// shots, the dynamo.
    fn noise(&mut self, at: V3, loud: Fx) {
        let n = self.cultists.len();
        for i in 0..n {
            let c = self.cultists[i];
            if c.mind == Mind::Dead || c.mind == Mind::Flee {
                continue;
            }
            let h = self.hear(at, c.at.add(v3(0, ONE * 16 / 10, 0)));
            let mut heard = fx::mul(h.gain, loud);
            // the chant covers quiet sounds near the ring (journal tab 2)
            if c.mind == Mind::Chant && at.sub(self.mall.ring).len_xz() < int(9) {
                heard = heard * 3 / 10;
            }
            if heard > ONE / 25 {
                let c = &mut self.cultists[i];
                c.suspicion += heard * 3;
                c.goal = at;
                c.quiet = 0;
            }
        }
        if self.spirit.chasing {
            let h = self.hear(at, self.spirit.at);
            if fx::mul(h.gain, loud) > ONE / 30 {
                self.spirit.last_known = at;
                self.spirit.unseen = 0;
                self.spirit.prey = None;
            }
        }
    }

    fn reload(&mut self) {
        let r = &mut self.revolver;
        let hand = self.hand.at;
        let speed = self.hand.at.0 - self.hand.was.0;
        let mut sounds: Vec<(&'static str, Fx)> = Vec::new();
        if !r.open && speed < -FLICK {
            r.open = true;
            sounds.push(("cylinder_open", ONE * 4 / 10));
            if r.chambers.contains(&Chamber::Spent) {
                for c in r.chambers.iter_mut() {
                    if *c == Chamber::Spent {
                        *c = Chamber::Empty;
                    }
                }
                sounds.push(("cases_fall", ONE * 3 / 10));
            }
        } else if r.open && !r.holding && speed > SHUT_FLICK {
            r.open = false;
            sounds.push(("cylinder_close", ONE * 4 / 10));
        } else if r.open {
            if !r.box_out && d2(hand, POCKET) < GRAB {
                r.box_out = true;
                sounds.push(("box_out", ONE / 4));
            } else if r.box_out && !r.holding && r.box_rounds > 0 && d2(hand, BOX_AT) < GRAB {
                r.holding = true;
                r.box_rounds -= 1;
                sounds.push(("round_pick", ONE / 5));
            } else if r.holding {
                for k in 0..6 {
                    if r.chambers[k] == Chamber::Empty && d2(hand, chamber_at(k)) < SEAT {
                        r.chambers[k] = Chamber::Round;
                        r.holding = false;
                        sounds.push(("round_seat", ONE * 3 / 10));
                        self.armed = true;
                        self.loaded_now += 1;
                        break;
                    }
                }
            }
        }
        for (s, l) in sounds {
            self.say_head(s, v3(ONE / 6, -ONE / 3, -ONE / 2), l);
        }
    }

    fn turn_handle(&mut self, dn: usize) {
        let door = self.mall.doors[dn];
        let (x, y) = self.hand.at;
        if fx::length(x, y) < ONE / 10 {
            return;
        }
        let a = fx::atan2(y, x);
        if self.door_angle == NO_ANGLE {
            self.door_angle = a;
            return;
        }
        let step = fx::wrap(a - self.door_angle);
        self.door_angle = a;
        // turns in Q16.16: a full turn of the angle is `ONE`
        self.door_turns[dn] += (step as i64 * ONE as i64 / fx::TURN as i64) as Fx;
        if self.tick % 20 == 0 && step != 0 {
            self.say("handle_rattle", door.handle, ONE / 4, None);
        }
        if self.door_turns[dn].abs() >= door.turns {
            self.door_open[dn] = ONE;
            self.mall.map.brushes[door.brush] = mall::open_brush();
            self.say("door_open", door.handle, ONE / 2, None);
            self.mode = Mode::Look;
        }
    }

    fn ouija(&mut self) {
        if self.letters >= 3 {
            return;
        }
        // the planchette pulls the hand toward the next letter
        let goal = LETTERS[self.letters as usize];
        let (wx, wy) = self.hand.want;
        self.hand.want = (wx + fx::mul(goal.0 - wx, PULL), wy + fx::mul(goal.1 - wy, PULL));
        if d2(self.hand.at, goal) < SEAT {
            self.letters += 1;
            self.letter_tick = self.tick;
            let at = self.eye();
            self.say_head("slam", v3(0, 0, -ONE), ONE);
            self.flinch = ONE;
            let _ = at;
            if self.letters == 3 {
                self.begin_chase();
            }
        }
    }

    fn begin_chase(&mut self) {
        let from = self
            .mall
            .props
            .iter()
            .find(|p| p.kind == PropKind::Ouija)
            .map(|p| p.at)
            .unwrap_or(self.mall.ring);
        self.blackout = Some(self.tick);
        self.lights.wave = Some((self.tick + HZ, from));
        self.spirit.chasing = true;
        self.spirit.at = v3(int(100), 0, int(-4));
        self.spirit.size = ONE * 11 / 10;
        self.spirit.last_known = self.body.origin;
        self.spirit.unseen = 0;
        self.spirit.chase_ticks = 0;
        self.possessed = None;
        for c in self.cultists.iter_mut() {
            if c.mind != Mind::Dead {
                c.mind = Mind::Flee;
                c.scream = 0;
            }
        }
        self.mode = Mode::Look;
        self.checkpoint = Some(Box::new(self.snapshot()));
    }

    fn fire(&mut self) {
        let eye = self.eye();
        let r = &mut self.revolver;
        if r.open {
            return;
        }
        r.at = (r.at + 1) % 6;
        if r.chambers[r.at] != Chamber::Round {
            self.say_head("dry_click", v3(ONE / 5, -ONE / 4, -ONE / 2), ONE / 4);
            return;
        }
        r.chambers[r.at] = Chamber::Spent;
        r.kick = 10;
        self.flinch = ONE / 3;
        self.say("shot", eye, ONE, None);
        self.noise(eye, ONE * 3);
        let dir = self.facing();
        let mut sc = Scratch::default();
        let far = int(60);
        let end = eye.add(dir.scale(far));
        let wall = trace_box(&self.mall.map, &mut sc, eye, end, V3::ZERO, V3::ZERO);
        let wall_d = fx::mul(wall.fraction, far);
        // the nearest thing the shot meets before the wall
        let mut best: Option<(Fx, Target)> = None;
        let mut consider = |d: Option<Fx>, t: Target| {
            if let Some(d) = d {
                if d < wall_d && best.map(|(bd, _)| d < bd).unwrap_or(true) {
                    best = Some((d, t));
                }
            }
        };
        if let Some(pi) = self.possessed {
            let p = self.mall.props[pi];
            let (r, h) = if p.kind == PropKind::Car {
                (ONE * 16 / 10, ONE * 15 / 10)
            } else {
                (ONE * 4 / 10, ONE * 19 / 10)
            };
            consider(ray_column(eye, dir, p.at, r, h), Target::Possessed);
        }
        for (i, c) in self.cultists.iter().enumerate() {
            if c.mind != Mind::Dead {
                consider(
                    ray_column(eye, dir, c.at, ONE * 4 / 10, ONE * 18 / 10),
                    Target::Cultist(i),
                );
            }
        }
        for (i, f) in self.mall.fixtures.iter().enumerate() {
            if f.kind != FixtureKind::Moon && self.lights.states[i] != crate::lights::State::Broken {
                consider(ray_ball(eye, dir, f.at, ONE * 35 / 100), Target::Fixture(i));
            }
        }
        match best.map(|(_, t)| t) {
            Some(Target::Possessed) => self.exorcise(),
            Some(Target::Cultist(i)) => {
                // T2: the cultists can be shot
                let c = &mut self.cultists[i];
                c.mind = Mind::Dead;
                let at = c.at;
                self.say("body_fall", at, ONE * 6 / 10, None);
                for o in self.cultists.iter_mut() {
                    if o.mind != Mind::Dead && o.mind != Mind::Flee {
                        o.mind = Mind::Search;
                        o.suspicion = ONE * 2;
                        o.goal = eye;
                    }
                }
            }
            Some(Target::Fixture(i)) => {
                if self.lights.break_one(i, &self.mall.fixtures) {
                    let at = self.mall.fixtures[i].at;
                    self.say("pop_glass", at, ONE * 7 / 10, None);
                }
            }
            None => {
                let at = eye.add(dir.scale(wall_d));
                self.say("ricochet", at, ONE / 3, None);
            }
        }
    }

    fn exorcise(&mut self) {
        if let Some(pi) = self.possessed.take() {
            let at = self.mall.props[pi].at.add(v3(0, ONE * 15 / 10, 0));
            self.say("wisp", at, ONE * 7 / 10, Some("sub_wisp"));
            if self.mall.props[pi].kind == PropKind::Car {
                self.car_done = true;
            }
            self.possess_wait = 30 * HZ as i32;
        }
    }

    /// The scare director (journal tab 2): armed by the first round, it acts
    /// while the hands are busy (and now and then otherwise), always out of
    /// view (CH-4) and only where a sound can reach you.
    fn director(&mut self) {
        self.scare_wait = (self.scare_wait - 1).max(0);
        if !self.armed || self.scare_wait > 0 || self.spirit.chasing {
            return;
        }
        let menace = self.menace();
        let busy = self.hands_busy();
        // chance a tick, in 1/10000ths: busy rises with each round loaded
        let per = if busy {
            40 + menace * 25 + self.loaded_now * 20
        } else if menace >= 1 {
            4 + menace * 3
        } else {
            0
        };
        if per == 0 || !self.rng.chance(per as u32, 10_000) {
            return;
        }
        let eye = self.eye();
        let mut choices = Vec::new();
        for (i, s) in self.mall.scares.iter().enumerate() {
            if s.menace > menace || s.at.sub(eye).len() > int(25) {
                continue;
            }
            if !scare_spot_ok(self.yaw, self.pitch, eye, s.at) {
                continue;
            }
            if self.hear(s.at, eye).gain < ONE / 60 {
                continue;
            }
            choices.push(i);
        }
        if menace >= 4 && busy {
            // breath on your neck: right behind the head (journal tab 4)
            self.say_head("breath", v3(ONE / 6, 0, ONE * 3 / 10), ONE * 4 / 10);
            self.marks.push(Mark {
                key: "sub_breath",
                angle: self.yaw + fx::deg(160),
                tick: self.tick,
            });
            self.flinch = ONE / 2;
            self.scare_wait = 3600 / per_minute(menace);
            return;
        }
        if choices.is_empty() {
            return;
        }
        let pick = choices[self.rng.below(choices.len() as u32) as usize];
        let s = self.mall.scares[pick];
        match s.kind {
            ScareKind::DoorCreak => self.say("door_creak", s.at, ONE * 6 / 10, Some("sub_door")),
            ScareKind::Fall => {
                self.say("fall_crash", s.at, ONE * 8 / 10, Some("sub_fall"));
                let yaw = self.rng.below(fx::TURN as u32) as i32;
                self.debris.push(Debris {
                    at: v3(s.at.x, 0, s.at.z),
                    yaw,
                });
            }
            ScareKind::Shutter => self.say("shutter", s.at, ONE * 7 / 10, Some("sub_shutter")),
            ScareKind::Steps => {
                for k in 0..3 {
                    self.pending_steps.push((self.tick + 20 + k * 22, s.at));
                }
                self.marks.push(Mark {
                    key: "sub_steps",
                    angle: yaw_of(s.at.sub(eye)),
                    tick: self.tick,
                });
            }
            ScareKind::Laugh => self.say("laugh", s.at, ONE * 5 / 10, Some("sub_laugh")),
            ScareKind::Glass => {
                self.say("pop_glass", s.at, ONE * 7 / 10, Some("sub_light"));
                let near = self
                    .mall
                    .fixtures
                    .iter()
                    .enumerate()
                    .filter(|(i, f)| {
                        f.at.sub(s.at).len() < int(8)
                            && self.lights.states[*i] != crate::lights::State::Broken
                    })
                    .map(|(i, _)| i)
                    .next();
                if let Some(i) = near {
                    self.lights.break_one(i, &self.mall.fixtures);
                }
            }
        }
        // a mannequin turns to face you, somewhere you are not looking
        if menace >= 2 {
            let p = self.body.origin;
            let yaw = self.yaw;
            let pitch = self.pitch;
            let eye2 = eye;
            if let Some(m) = self
                .mall
                .props
                .iter_mut()
                .find(|m| m.kind == PropKind::Mannequin && off_view(yaw, pitch, eye2, m.at) > HALF_VIEW)
            {
                m.yaw = yaw_of(p.sub(m.at));
            }
        }
        self.scare_wait = 3600 / per_minute(menace);
    }

    fn steps_due(&mut self) {
        let now = self.tick;
        let due: Vec<V3> = self
            .pending_steps
            .iter()
            .filter(|(t, _)| *t == now)
            .map(|(_, p)| *p)
            .collect();
        self.pending_steps.retain(|(t, _)| *t > now);
        for p in due {
            self.say("step_other", p, ONE * 6 / 10, None);
        }
    }

    /// The spirit in a mannequin or the raffle car: it moves only while you
    /// do not see it; one hit drives it out (CH-Q14 A).
    fn possession(&mut self) {
        if self.spirit.chasing {
            return;
        }
        let menace = self.menace();
        self.possess_wait = (self.possess_wait - 1).max(0);
        if self.possessed.is_none() && self.possess_wait == 0 && menace >= 2 {
            let me = self.body.origin;
            let car = self.mall.props.iter().position(|p| p.kind == PropKind::Car);
            let pick = if menace >= 3 && !self.car_done {
                car
            } else {
                self.mall
                    .props
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.kind == PropKind::Mannequin)
                    .filter(|(_, p)| {
                        let d = p.at.sub(me).len_xz();
                        d > int(6) && d < int(22)
                    })
                    .map(|(i, _)| i)
                    .next()
            };
            if let Some(pi) = pick {
                self.possessed = Some(pi);
                self.was_seen = true;
                let at = self.mall.props[pi].at.add(v3(0, ONE, 0));
                let (recipe, sub) = if Some(pi) == car {
                    ("horn", "sub_horn")
                } else {
                    ("plastic_knock", "sub_knock")
                };
                self.say(recipe, at, ONE * 7 / 10, Some(sub));
            }
            self.possess_wait = 20 * HZ as i32;
        }
        let Some(pi) = self.possessed else { return };
        let p = self.mall.props[pi];
        let head = p.at.add(v3(0, ONE * 15 / 10, 0));
        let seen = self.sees(head);
        let me = self.body.origin;
        let toward = v3(me.x - p.at.x, 0, me.z - p.at.z);
        let dist = toward.len_xz();
        if p.kind == PropKind::Car {
            if !seen && dist > int(2) {
                let step = fx::div(ONE / HZ as i32, dist.max(1));
                let m = &mut self.mall.props[pi];
                m.at = m.at.add(v3(fx::mul(toward.x, step), 0, fx::mul(toward.z, step)));
                m.yaw = yaw_of(toward);
            }
        } else if self.was_seen && !seen && dist > int(2) {
            // one step closer each time you look away
            let step = fx::div(ONE, dist.max(1));
            let m = &mut self.mall.props[pi];
            m.at = m.at.add(v3(fx::mul(toward.x, step), 0, fx::mul(toward.z, step)));
            m.yaw = yaw_of(toward);
            let knock = m.at.add(v3(0, ONE, 0));
            self.say("plastic_knock", knock, ONE * 4 / 10, None);
        }
        self.was_seen = seen;
        let reach = if p.kind == PropKind::Car {
            ONE * 25 / 10
        } else {
            ONE * 13 / 10
        };
        if dist < reach && !matches!(self.mode, Mode::Hidden(_)) {
            self.harm("sub_grab");
            self.possessed = None;
            self.possess_wait = 30 * HZ as i32;
            if p.kind == PropKind::Car {
                self.car_done = true;
            }
        }
    }

    fn harm(&mut self, key: &'static str) {
        if self.hurt > 0 {
            return;
        }
        self.health -= 1;
        self.hurt = HZ as i32;
        self.flinch = ONE;
        self.say_head("hit", v3(0, 0, -ONE / 3), ONE);
        self.marks.push(Mark {
            key,
            angle: self.yaw,
            tick: self.tick,
        });
    }

    fn cult(&mut self) {
        let me = self.body.origin;
        let eye = self.eye();
        let hidden = matches!(self.mode, Mode::Hidden(_));
        let lit = self.lit_at(eye);
        let n = self.cultists.len();
        for i in 0..n {
            let mut c = self.cultists[i];
            if c.mind == Mind::Dead {
                continue;
            }
            let ceye = c.at.add(v3(0, ONE * 16 / 10, 0));
            let to_me = v3(me.x - c.at.x, 0, me.z - c.at.z);
            let dist = to_me.len_xz();
            // sight: in its cone, a clear line, how lit you are
            let off = fx::wrap(yaw_of(to_me) - c.yaw).abs();
            let mut sight = 0;
            if !hidden && dist < int(25) && off < fx::deg(55) && self.clear_line(ceye, eye) {
                let mut s = fx::div(lit, dist.max(ONE));
                if self.body.ducked {
                    s /= 2;
                }
                sight = s;
            }
            match c.mind {
                Mind::Flee => {
                    c.scream -= 1;
                    if c.scream <= 0 {
                        c.scream = 150 + self.rng.below(120) as i32;
                        self.say("scream", ceye, ONE, Some("sub_scream"));
                    }
                    let away = v3(c.at.x - self.spirit.at.x, 0, c.at.z - self.spirit.at.z);
                    let l = away.len_xz().max(1);
                    let step = ONE * 3 / HZ as i32;
                    let next = c.at.add(v3(
                        fx::div(fx::mul(away.x, step), l),
                        0,
                        fx::div(fx::mul(away.z, step), l),
                    ));
                    if self.mall.rooms[10].holds(next) {
                        c.at = next;
                    }
                    c.yaw = yaw_of(away);
                }
                _ => {
                    c.suspicion += sight / 20;
                    c.quiet += 1;
                    if sight > 0 {
                        c.goal = me;
                        c.quiet = 0;
                    }
                    if c.quiet > 8 * HZ as i32 {
                        c.suspicion = fx::approach(c.suspicion, 0, ONE / 120);
                    }
                    let keeps_attacking = c.mind == Mind::Attack && c.quiet < 8 * HZ as i32;
                    let starts_attacking = c.suspicion >= ONE && sight > ONE / 12 && dist < int(14);
                    c.mind = if keeps_attacking || starts_attacking {
                        Mind::Attack
                    } else if c.suspicion >= ONE {
                        Mind::Search
                    } else if c.suspicion >= ONE * 3 / 10 {
                        Mind::Look
                    } else {
                        Mind::Chant
                    };
                    let (goal, speed) = match c.mind {
                        Mind::Attack => (me, ONE * 32 / 10),
                        Mind::Search => (c.goal, ONE * 14 / 10),
                        Mind::Look => (c.at, 0),
                        _ => {
                            if c.patrol {
                                c.angle = (c.angle + fx::TURN / (HZ as i32 * 40)) % fx::TURN;
                                let r = int(11);
                                (
                                    self.mall.ring.add(v3(
                                        fx::mul(r, fx::cos(c.angle)),
                                        0,
                                        fx::mul(r, fx::sin(c.angle)),
                                    )),
                                    ONE,
                                )
                            } else {
                                (c.post, ONE)
                            }
                        }
                    };
                    if c.mind == Mind::Look
                        || (c.mind == Mind::Chant && !c.patrol && c.at.sub(c.post).len_xz() < ONE / 4)
                    {
                        let target = if c.mind == Mind::Look {
                            c.goal
                        } else {
                            self.mall.ring
                        };
                        c.yaw = turn_toward(c.yaw, yaw_of(target.sub(c.at)), fx::deg(3));
                    } else {
                        let way = v3(goal.x - c.at.x, 0, goal.z - c.at.z);
                        let l = way.len_xz();
                        if l > ONE / 2 && speed > 0 {
                            let step = speed / HZ as i32;
                            let next = c.at.add(v3(
                                fx::div(fx::mul(way.x, step), l),
                                0,
                                fx::div(fx::mul(way.z, step), l),
                            ));
                            c.at = next;
                            c.yaw = turn_toward(c.yaw, yaw_of(way), fx::deg(6));
                        }
                    }
                    if c.mind == Mind::Attack {
                        c.cooldown -= 1;
                        if dist < ONE * 13 / 10 && c.cooldown <= 0 && !hidden {
                            c.cooldown = 90;
                            self.cultists[i] = c;
                            self.say("dagger", ceye, ONE * 6 / 10, None);
                            self.harm("sub_stab");
                            continue;
                        }
                    }
                    if c.mind == Mind::Look && self.cultists[i].mind == Mind::Chant {
                        self.say("mutter", ceye, ONE * 4 / 10, Some("sub_mutter"));
                    }
                    if c.mind == Mind::Attack && self.cultists[i].mind != Mind::Attack {
                        self.say("shout", ceye, ONE, Some("sub_shout"));
                        for o in 0..n {
                            if o != i && self.cultists[o].mind != Mind::Dead {
                                self.cultists[o].suspicion = self.cultists[o].suspicion.max(ONE);
                                self.cultists[o].goal = me;
                            }
                        }
                    }
                }
            }
            self.cultists[i] = c;
        }
    }

    /// How lit the player is, for the cultists: the flashlight on counts
    /// three times a lamp overhead.
    pub fn lit_at(&self, p: V3) -> Fx {
        let mut lit = ONE / 6;
        if self.flashlight.on {
            lit += 3 * self.flashlight.brightness(false);
        }
        for (i, f) in self.mall.fixtures.iter().enumerate() {
            let b = self.lights.brightness(i, false);
            if b > 0 && f.at.sub(p).len() < f.range {
                lit += fx::mul(f.power, b);
            }
        }
        lit
    }

    /// After R U N: the spirit hunts by sight and sound, grows, kills the
    /// cultists it reaches, and loses you after twelve quiet seconds.
    fn chase(&mut self) {
        if self.spirit.chasing {
            // T4: while you run back, every sign turns to look at you
            for (s, yaw) in self.mall.signs.iter().zip(self.sign_yaws.iter_mut()) {
                let to = sign_yaw_to(s.at, self.body.origin);
                *yaw = turn_toward(*yaw, to, SIGN_TURN);
            }
        }
        if !self.spirit.chasing {
            // glimpses at the edge of the view (menace 3 and 4)
            self.spirit.glimpse = (self.spirit.glimpse - 1).max(0);
            if self.menace() >= 3 && self.spirit.glimpse == 0 && self.rng.chance(1, 900) {
                let side = if self.rng.chance(1, 2) {
                    fx::deg(55)
                } else {
                    -fx::deg(55)
                };
                let dir = facing(self.yaw + side, 0);
                self.spirit.at = self.body.origin.add(dir.scale(int(7)));
                self.spirit.glimpse = 20;
            }
            return;
        }
        let s = &mut self.spirit;
        s.chase_ticks += 1;
        // 1.1 m to 4 m over three minutes
        let grow = (s.chase_ticks as i64 * (ONE * 29 / 10) as i64 / (180 * HZ)) as Fx;
        s.size = (ONE * 11 / 10 + grow).min(int(4));
        s.unseen += 1;
        let me = self.body.origin;
        let hidden = matches!(self.mode, Mode::Hidden(_));
        let spirit_eye = s.at.add(v3(0, s.size, 0));
        let sees_me = !hidden && me.sub(s.at).len_xz() < int(25) && self.clear_line(spirit_eye, self.eye());
        let s = &mut self.spirit;
        if sees_me || (self.flashlight.on && me.sub(s.at).len_xz() < int(14)) {
            s.unseen = 0;
            s.last_known = me;
            s.prey = None;
        }
        // a screaming cultist nearer than where it thinks you are draws it
        let mut prey = s.prey;
        if s.unseen > HZ as i32 {
            let mut best = s.last_known.sub(s.at).len_xz();
            for (i, c) in self.cultists.iter().enumerate() {
                if c.mind == Mind::Flee {
                    let d = c.at.sub(s.at).len_xz();
                    if d < best && d < int(30) {
                        best = d;
                        prey = Some(i);
                    }
                }
            }
        }
        s.prey = prey;
        let goal = match prey {
            Some(i) => self.cultists[i].at,
            None => s.last_known,
        };
        // through the rooms by their portals; it passes shut doors
        let next = self.next_waypoint(self.spirit.at, goal);
        let s = &mut self.spirit;
        let way = v3(next.x - s.at.x, 0, next.z - s.at.z);
        let l = way.len_xz();
        let lost = s.unseen > 12 * HZ as i32;
        let speed = if lost { ONE * 12 / 10 } else { ONE * 36 / 10 };
        if l > ONE / 4 {
            let step = speed / HZ as i32;
            s.at = s.at.add(v3(
                fx::div(fx::mul(way.x, step), l),
                0,
                fx::div(fx::mul(way.z, step), l),
            ));
        } else if lost {
            // searching: on to a scare point near where it last knew you
            let k = self.rng.below(self.mall.scares.len() as u32) as usize;
            self.spirit.last_known = self.mall.scares[k].at;
        }
        if self.tick % 90 == 0 {
            let at = self.spirit.at.add(v3(0, self.spirit.size, 0));
            self.say("spirit_breath", at, ONE * 8 / 10, Some("sub_spirit"));
        }
        if let Some(i) = prey {
            if self.cultists[i].at.sub(self.spirit.at).len_xz() < ONE {
                self.cultists[i].mind = Mind::Dead;
                let at = self.cultists[i].at.add(v3(0, ONE, 0));
                self.say("scream_cut", at, ONE, Some("sub_scream"));
                self.spirit.prey = None;
            }
        }
        let reach = self.spirit.size / 3 + ONE / 2;
        if me.sub(self.spirit.at).len_xz() < reach {
            if hidden && !self.seen_hide {
                return;
            }
            self.health = 0;
            self.flinch = ONE;
        }
    }

    /// The next point toward `goal` through the portals (the spirit's way).
    fn next_waypoint(&self, from: V3, goal: V3) -> V3 {
        let all_open: Vec<Fx> = vec![ONE; self.mall.doors.len()];
        let h = acoustics::hear(&self.mall.rooms, &self.mall.portals, &all_open, goal, from, false);
        let a = self.mall.room_of(from);
        let b = self.mall.room_of(goal);
        if a == b {
            return goal;
        }
        // the apparent place of the goal heard from here lies through the
        // first portal on the way: walk to that portal
        let toward = h.apparent;
        let mut best = goal;
        let mut low = i32::MAX;
        for p in &self.mall.portals {
            if p.a == a || p.b == a {
                let dd = p.at.sub(toward).len_xz() + p.at.sub(from).len_xz() / 4;
                if dd < low {
                    low = dd;
                    best = p.at;
                }
            }
        }
        best
    }

    fn outcome(&mut self) {
        if self.health <= 0 {
            self.phase = Phase::Dead;
            self.deaths += 1;
            self.mode = Mode::Look;
            return;
        }
        if self.body.origin.x < self.mall.exit_x + ONE && self.body.origin.z.abs() < int(5) {
            if self.blackout.is_some() {
                self.phase = Phase::Won;
            } else {
                self.prompt = "prompt_not_yet";
            }
        }
    }
}

/// The settings as the text the page keeps (CH-Q6 A): version first, so a
/// later build can read an older one.
pub fn settings_text(s: &Settings) -> String {
    let out = match s.output {
        Output::Headphones => "headphones",
        Output::Speakers => "speakers",
        Output::Mono => "mono",
    };
    serde_json::json!({
        "version": 1,
        "output": out,
        "ring": s.ring,
        "subtitles": s.subtitles,
        "reduce_flashes": s.reduce_flashes,
        "sensitivity": s.sensitivity,
        "tier": s.tier,
        "volume": s.volume,
    })
    .to_string()
}

pub fn settings_from(text: &str) -> Option<Settings> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if v.get("version")?.as_i64()? != 1 {
        return None;
    }
    let output = match v.get("output")?.as_str()? {
        "speakers" => Output::Speakers,
        "mono" => Output::Mono,
        _ => Output::Headphones,
    };
    Some(Settings {
        output,
        ring: v.get("ring")?.as_bool()?,
        subtitles: v.get("subtitles")?.as_bool()?,
        reduce_flashes: v.get("reduce_flashes")?.as_bool()?,
        sensitivity: (v.get("sensitivity")?.as_i64()? as i32).clamp(20, 300),
        tier: (v.get("tier")?.as_i64()? as i32).clamp(0, 2),
        // settings kept before the volume existed load at full volume
        volume: v
            .get("volume")
            .and_then(|x| x.as_i64())
            .unwrap_or(100)
            .clamp(0, 100) as i32,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Possessed,
    Cultist(usize),
    Fixture(usize),
}

pub fn per_minute(menace: i32) -> i32 {
    [1, 2, 3, 3, 4][menace.clamp(0, 4) as usize]
}

/// How far a sign turns toward you each tick of the chase: 30° a second.
pub const SIGN_TURN: i32 = fx::deg(1) / 2;

/// The heading a sign at `at` needs for its face to look at `p`.
pub fn sign_yaw_to(at: V3, p: V3) -> i32 {
    // facing(yaw) is (−sin, ·, −cos)
    fx::atan2(at.x - p.x, at.z - p.z)
}

pub fn turn_toward(from: i32, to: i32, most: i32) -> i32 {
    let d = fx::wrap(to - from).clamp(-most, most);
    (from + d).rem_euclid(fx::TURN)
}

/// The angle between the view (`yaw`, `pitch` from `eye`) and `p`.
pub fn off_view(yaw: i32, pitch: i32, eye: V3, p: V3) -> i32 {
    let f = facing(yaw, pitch);
    let d = p.sub(eye);
    let l = d.len().max(1);
    let cos = fx::div(f.dot(d), l).clamp(-ONE, ONE);
    let sin = fx::sqrt(fx::mul(ONE - cos, ONE + cos).max(0));
    fx::wrap(fx::atan2(sin, cos)).abs()
}

/// CH-4: a scare may spawn at `p` only 20° or more past the view's edge.
pub fn scare_spot_ok(yaw: i32, pitch: i32, eye: V3, p: V3) -> bool {
    off_view(yaw, pitch, eye, p) >= HALF_VIEW + SCARE_MARGIN
}

/// Distance along a ray to an upright column (radius `r`, height `h`) at
/// `base`, if it meets it.
fn ray_column(o: V3, dir: V3, base: V3, r: Fx, h: Fx) -> Option<Fx> {
    let to = v3(base.x - o.x, 0, base.z - o.z);
    let flat = v3(dir.x, 0, dir.z);
    let fl = flat.len_xz();
    if fl < ONE / 100 {
        return None;
    }
    let along = fx::div(fx::mul(to.x, flat.x) + fx::mul(to.z, flat.z), fl);
    if along <= 0 {
        return None;
    }
    let cx = o.x + fx::div(fx::mul(flat.x, along), fl);
    let cz = o.z + fx::div(fx::mul(flat.z, along), fl);
    if fx::length(cx - base.x, cz - base.z) > r {
        return None;
    }
    let t = fx::div(along, fl);
    let y = o.y + fx::mul(dir.y, t);
    if y < base.y || y > base.y + h {
        return None;
    }
    Some(t)
}

fn ray_ball(o: V3, dir: V3, c: V3, r: Fx) -> Option<Fx> {
    let to = c.sub(o);
    let along = to.dot(dir);
    if along <= 0 {
        return None;
    }
    let closest = o.add(dir.scale(along));
    if closest.sub(c).len() > r {
        return None;
    }
    Some(along)
}

#[cfg(test)]
mod tests;
