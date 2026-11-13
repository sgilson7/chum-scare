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
use crate::mall::{self, mall, CurioKind, FixtureKind, Mall, PropKind, ScareKind};
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

/// T18: what stays at the edge of the view until you look at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sight {
    /// a pair of pale eyes at head height
    Eyes,
    /// a wide smile of teeth, no face around it
    Smile,
    /// two long pale arms reaching toward you
    Hands,
}

impl Sight {
    /// how high it hangs
    pub fn height(self) -> Fx {
        match self {
            Sight::Eyes => ONE * 165 / 100,
            Sight::Smile => ONE * 145 / 100,
            Sight::Hands => ONE * 120 / 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Apparition {
    pub kind: Sight,
    pub at: V3,
    /// the angle off the view it keeps to, and how far it stood
    pub side: i32,
    pub dist: Fx,
    pub age: i32,
    /// ticks left as it goes
    pub fading: i32,
}

/// The court's fuse box (T14): on, thrown off until a tick, or shot dead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breaker {
    On,
    Thrown(i64),
    Shot,
}

/// Where a curio is now and what it has done (T12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurioState {
    pub at: V3,
    pub yaw: i32,
    /// how many times it has been used
    pub uses: i32,
    /// the music box or the telephone has answered; it does not again (the
    /// toy and the camera count their uses instead)
    pub spent: bool,
}

/// The little girl a curio shows (T12): where she stands, how many ticks
/// she has left, and whether she goes once she is looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phantom {
    pub at: V3,
    pub yaw: i32,
    pub ticks: i32,
    pub until_seen: bool,
}

/// Where a cued sound is made: in the world, in the head, or a distance
/// behind wherever the player is when it is due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    At(V3),
    Head(V3),
    Behind(Fx),
    /// wherever the curio is: on its counter, the floor, or in your hand
    Curio(usize),
}

/// Something a curio set going that happens later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    Sound {
        recipe: &'static str,
        place: Place,
        loud: Fx,
        mark: Option<&'static str>,
    },
    /// the girl, `ahead` metres along the view turned by `side`, for
    /// `ticks`; she giggles when she comes
    Girl {
        ahead: Fx,
        side: i32,
        ticks: i32,
        until_seen: bool,
    },
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
    /// the tick of the director's last scare; its wait counts from here at
    /// the menace you are in now (CH-Q26)
    pub last_scare: i64,
    /// how often each kind has been used, and the last (CH-Q23)
    pub kinds_used: [i32; mall::ScareKind::COUNT],
    pub last_kind: Option<ScareKind>,
    /// ticks until the spirit may be glimpsed again (CH-Q25)
    pub spirit_wait: i32,
    /// ticks until the next apparition may come (T18)
    pub glimpse_wait: i32,
    /// T18: the apparition at the edge of the view, if any
    pub apparition: Option<Apparition>,
    /// T17: ticks the room's own sounds stay hushed after a scare
    pub hush: i32,
    pub pending_steps: Vec<(i64, V3)>,
    /// the curios (T12): one state each, the one in your hand, the girl,
    /// the camera's flash and what is still to happen; whether the
    /// telephone has started ringing
    pub curios: Vec<CurioState>,
    pub held: Option<usize>,
    pub phantom: Option<Phantom>,
    pub flash: i32,
    pub cues: Vec<(i64, Cue)>,
    pub rang: bool,
    /// T14: the fuse box, what its lamps were before it cut them, and how
    /// long the cult stays blind in the sudden dark
    pub breaker: Breaker,
    pub breaker_saved: Vec<(usize, crate::lights::State)>,
    pub dazzled: i32,
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
            last_scare: i64::MIN / 2,
            kinds_used: [0; mall::ScareKind::COUNT],
            last_kind: None,
            spirit_wait: 0,
            glimpse_wait: 0,
            apparition: None,
            hush: 0,
            pending_steps: Vec::new(),
            curios: m
                .curios
                .iter()
                .map(|c| CurioState {
                    at: c.at,
                    yaw: c.yaw,
                    uses: 0,
                    spent: false,
                })
                .collect(),
            held: None,
            phantom: None,
            flash: 0,
            cues: Vec::new(),
            rang: false,
            breaker: Breaker::On,
            breaker_saved: Vec::new(),
            dazzled: 0,
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

    /// Where to head for `goal` from `from` (T8): the goal itself when it
    /// is in the same room, else the doorway leading there by the fewest
    /// rooms, through open doors only. None when there is no way.
    pub fn way_to(&self, from: V3, goal: V3) -> Option<V3> {
        let (a, b) = (self.mall.room_of(from), self.mall.room_of(goal));
        if a == b {
            return Some(goal);
        }
        let n = self.mall.rooms.len();
        // breadth first, from the goal's room back, so each room knows the
        // portal that is its next step
        let mut next: Vec<Option<usize>> = vec![None; n];
        let mut seen = vec![false; n];
        let mut queue = std::collections::VecDeque::from([b]);
        seen[b] = true;
        while let Some(r) = queue.pop_front() {
            for (pi, p) in self.mall.portals.iter().enumerate() {
                if let Some(d) = p.door {
                    if self.door_open[d] == 0 {
                        continue;
                    }
                }
                let other = if p.a == r {
                    p.b
                } else if p.b == r {
                    p.a
                } else {
                    continue;
                };
                if !seen[other] {
                    seen[other] = true;
                    next[other] = Some(pi);
                    queue.push_back(other);
                }
            }
        }
        next[a].map(|pi| {
            let at = self.mall.portals[pi].at;
            v3(at.x, 0, at.z)
        })
    }

    /// One step of a cultist toward `goal` (T8): by `way_to`, sliding along
    /// walls, never into one.
    pub fn cult_step(&self, at: V3, goal: V3, step: Fx) -> V3 {
        let Some(mut aim) = self.way_to(at, goal) else {
            return at;
        };
        // on the doorway itself: on through it, toward the goal
        if v3(aim.x - at.x, 0, aim.z - at.z).len_xz() < ONE / 2 {
            aim = goal;
        }
        let way = v3(aim.x - at.x, 0, aim.z - at.z);
        let l = way.len_xz();
        if l == 0 {
            return at;
        }
        let step = step.min(l);
        let d = v3(
            fx::div(fx::mul(way.x, step), l),
            0,
            fx::div(fx::mul(way.z, step), l),
        );
        let mut sc = Scratch::default();
        let (lo, hi) = (
            v3(-CULT_HALF, ONE / 10, -CULT_HALF),
            v3(CULT_HALF, ONE * 17 / 10, CULT_HALF),
        );
        for try_d in [d, v3(d.x, 0, 0), v3(0, 0, d.z)] {
            if try_d.len_xz() == 0 {
                continue;
            }
            let t = trace_box(&self.mall.map, &mut sc, at, at.add(try_d), lo, hi);
            if t.start_solid || !t.hit() {
                return at.add(try_d);
            }
        }
        at
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
            if let Some(i) = self.held {
                self.use_curio(i);
            } else {
                self.fire();
            }
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
        self.curio_time();
        self.fuse_time();
        let moving = self.body.speed() > ONE / 3;
        let want = if moving && self.revolver.kick == 0 { ONE } else { 0 };
        self.gun_up = fx::approach(self.gun_up, want, ONE / 24);
        // the eye adapts over about three seconds
        let lit = self.lit_at(self.eye()).min(ONE * 2);
        // T16: it brightens the dark only a little (was up to 1.5×): the
        // dark stays dark
        let want = (ONE * 11 / 10 - lit / 4).clamp(ONE * 3 / 4, EXPOSURE_DARK);
        self.exposure = fx::approach(self.exposure, want, ONE / 180);
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
        // T12: a curio in the hand comes first: E puts it down
        if let Some(i) = self.held {
            self.prompt = hold_prompt(self.mall.curios[i].kind);
            if fresh_use {
                self.put_down(i);
            }
            return;
        }
        let eye = self.eye();
        let near_curio = (0..self.curios.len())
            .filter(|&i| {
                let c = self.curios[i].at;
                c.sub(origin).len_xz() < REACH && self.off_view(c) < CURIO_LOOK
            })
            .min_by_key(|&i| self.curios[i].at.sub(eye).len());
        let fusebox = self.mall.fusebox;
        let near_box = fusebox.sub(eye).len() < REACH && self.off_view(fusebox) < fx::deg(40);
        if self.mode == Mode::Look {
            if near_spare {
                self.prompt = "prompt_take_box";
            } else if let Some(i) = near_curio {
                self.prompt = take_prompt(self.mall.curios[i].kind);
            } else if near_box && self.breaker == Breaker::On {
                self.prompt = "prompt_breaker";
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
            if let Some(i) = near_curio {
                self.pick_up(i);
                return;
            }
            if near_box && self.breaker == Breaker::On {
                self.say("breaker", self.mall.fusebox, ONE * 3 / 10, None);
                self.cut_power(Breaker::Thrown(self.tick + BREAKER_TICKS));
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
        // T10: say how a handle and the board are used
        if self.prompt.is_empty() {
            if self.mode == Mode::Look && door.is_some() {
                self.prompt = "prompt_turn_handle";
            } else if matches!(self.mode, Mode::Door(_)) {
                self.prompt = "prompt_turning";
            } else if self.mode == Mode::Look && near_board && self.letters < 3 && self.blackout.is_none() {
                self.prompt = "prompt_use_board";
            }
        }
        if fresh_alt && self.mode == Mode::Look {
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
                // T17: heard at half (the cult still hears the full step)
                self.say_head(recipe, v3(side, -ONE * 15 / 10, -ONE / 5), loud / 2);
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
        // T15: it rises in whichever back corner of the store is farther
        // from you (it used to rise 2 m from where you could stand)
        let me = self.body.origin;
        let corners = [v3(int(101), 0, int(-4)), v3(int(101), 0, int(4))];
        self.spirit.at = if corners[0].sub(me).len_xz() >= corners[1].sub(me).len_xz() {
            corners[0]
        } else {
            corners[1]
        };
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
        if self.breaker != Breaker::Shot {
            consider(ray_ball(eye, dir, self.mall.fusebox, ONE / 4), Target::Fusebox);
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
            Some(Target::Fusebox) => {
                // T14: dark for good, but they heard the shot
                self.say("sparks", self.mall.fusebox, ONE * 7 / 10, None);
                self.cut_power(Breaker::Shot);
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
        self.hush = (self.hush - 1).max(0);
        let menace = self.menace();
        // the wait is the ceiling of the menace you are in now, counted from
        // the last scare: walking deeper shortens it (CH-Q26)
        let wait = 3600 / per_minute(menace) as i64;
        self.scare_wait = (self.last_scare + wait - self.tick).clamp(0, wait) as i32;
        // T20: awake once you are past the doors, loaded or not (it used
        // to wait for the first round, and a player who never loaded met
        // no scare at all)
        if !(self.armed || menace >= 1) || self.scare_wait > 0 || self.spirit.chasing {
            return;
        }
        let busy = self.hands_busy();
        // chance a tick, in 1/10000ths: busy rises with each round loaded
        let per = if busy {
            40 + menace * 25 + self.loaded_now * 20
        } else if menace >= 1 {
            // T20: about one in seven seconds once the wait is over (was
            // one in twenty-four)
            15 + menace * 8
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
            self.last_scare = self.tick;
            self.scare_wait = wait as i32;
            return;
        }
        // CH-Q23 A: never the kind just used (when another is in reach),
        // and among the rest the least used
        let fresh: Vec<usize> = choices
            .iter()
            .copied()
            .filter(|&i| Some(self.mall.scares[i].kind) != self.last_kind)
            .collect();
        let choices = if fresh.is_empty() { choices } else { fresh };
        let least = choices
            .iter()
            .map(|&i| self.kinds_used[self.mall.scares[i].kind.index()])
            .min();
        let Some(least) = least else {
            return;
        };
        let choices: Vec<usize> = choices
            .into_iter()
            .filter(|&i| self.kinds_used[self.mall.scares[i].kind.index()] == least)
            .collect();
        let pick = choices[self.rng.below(choices.len() as u32) as usize];
        let s = self.mall.scares[pick];
        self.kinds_used[s.kind.index()] += 1;
        self.last_kind = Some(s.kind);
        // T17: a scare is set apart from the mall's own sounds: the room
        // goes quiet and a low swell comes up under it
        self.hush = HUSH_TICKS;
        self.say_head("dread", v3(0, -ONE / 2, 0), ONE * 5 / 10);
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
                // T17: slow, heavy and coming nearer, unlike your own: four
                // steps 0.65 s apart, each 0.7 m closer to where you are
                let toward = v3(eye.x - s.at.x, 0, eye.z - s.at.z);
                let l = toward.len_xz().max(1);
                for k in 0..4 {
                    // 0.7 m × k along the unit way toward you
                    let along = fx::div(ONE * 7 / 10 * k, l);
                    let at =
                        s.at.add(v3(fx::mul(toward.x, along), 0, fx::mul(toward.z, along)));
                    self.pending_steps.push((self.tick + 30 + k as i64 * 39, at));
                }
                self.marks.push(Mark {
                    key: "sub_steps",
                    angle: yaw_of(s.at.sub(eye)),
                    tick: self.tick,
                });
            }
            ScareKind::Laugh => self.say("laugh", s.at, ONE * 5 / 10, Some("sub_laugh")),
            ScareKind::Knock => self.say("knock", s.at, ONE * 6 / 10, Some("sub_knock_wall")),
            ScareKind::Whisper => self.say("whisper", s.at, ONE * 4 / 10, Some("sub_whisper")),
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
        // a mannequin turns to face you, somewhere you are not looking:
        // the nearest within 15 m that is out of view and not already
        // facing you (CH-Q24 A)
        if menace >= 2 {
            if let Some(i) = self.mannequin_to_turn(eye) {
                let p = self.body.origin;
                let m = &mut self.mall.props[i];
                m.yaw = yaw_of(p.sub(m.at));
            }
        }
        self.last_scare = self.tick;
        self.scare_wait = wait as i32;
    }

    /// The mannequin the director would turn (CH-Q24 A).
    pub fn mannequin_to_turn(&self, eye: V3) -> Option<usize> {
        let p = self.body.origin;
        self.mall
            .props
            .iter()
            .enumerate()
            .filter(|(i, m)| m.kind == PropKind::Mannequin && Some(*i) != self.possessed)
            .filter(|(_, m)| m.at.sub(p).len_xz() < MANNEQUIN_REACH)
            .filter(|(_, m)| {
                off_view(self.yaw, self.pitch, eye, m.at.add(v3(0, ONE * 15 / 10, 0))) > HALF_VIEW
            })
            .filter(|(_, m)| fx::wrap(m.yaw - yaw_of(p.sub(m.at))).abs() > fx::deg(20))
            .min_by_key(|(_, m)| m.at.sub(p).len_xz())
            .map(|(i, _)| i)
    }

    // T12: the curios -----------------------------------------------------

    fn pick_up(&mut self, i: usize) {
        self.held = Some(i);
        let at = self.curios[i].at;
        if self.mall.curios[i].kind != CurioKind::Phone {
            self.say("pick_up", at, ONE / 5, None);
            return;
        }
        // the ringing stops; on the line a hiss, a giggle at your ear, the
        // click of someone hanging up, then another telephone behind you
        self.cues.retain(|(_, c)| {
            !matches!(
                c,
                Cue::Sound {
                    recipe: "phone_ring",
                    ..
                }
            )
        });
        self.say("receiver_up", at, ONE / 4, None);
        if self.curios[i].spent {
            return;
        }
        self.curios[i].spent = true;
        let t = self.tick;
        let sound = |recipe, place, loud, mark| Cue::Sound {
            recipe,
            place,
            loud,
            mark,
        };
        self.cues.push((
            t + HZ / 2,
            sound("line_hiss", Place::Head(EAR), ONE * 3 / 10, None),
        ));
        self.cues.push((
            t + 2 * HZ,
            sound("giggle", Place::Head(EAR), ONE * 4 / 10, Some("sub_giggle")),
        ));
        self.cues.push((
            t + 4 * HZ,
            sound("receiver_click", Place::Head(EAR), ONE * 3 / 10, None),
        ));
        self.cues.push((
            t + 6 * HZ,
            sound(
                "phone_ring",
                Place::Behind(int(12)),
                ONE * 6 / 10,
                Some("sub_ring"),
            ),
        ));
    }

    fn put_down(&mut self, i: usize) {
        self.held = None;
        let home = self.mall.curios[i];
        if home.kind == CurioKind::Phone {
            // on its cord: back on the counter
            self.curios[i].at = home.at;
            self.say("receiver_down", home.at, ONE / 4, None);
            return;
        }
        let ahead = facing(self.yaw, 0).scale(ONE * 6 / 10);
        let mut at = self.body.origin.add(ahead);
        if !self.clear_line(self.eye(), at.add(v3(0, ONE / 10, 0))) {
            at = self.body.origin;
        }
        at.y = self.body.origin.y;
        self.curios[i].at = at;
        self.curios[i].yaw = self.yaw;
        self.say("put_down", at, ONE / 5, None);
    }

    /// The left button with a curio in the hand: each answers once.
    fn use_curio(&mut self, i: usize) {
        self.curios[i].uses += 1;
        let uses = self.curios[i].uses;
        let fresh = !self.curios[i].spent;
        let t = self.tick;
        match self.mall.curios[i].kind {
            CurioKind::Toy => {
                self.say_head("squeak", HAND, ONE * 4 / 10);
                // only the one squeeze: she answers once
                if uses == TOY_SQUEEZES {
                    // she is there for under half a second, then a squeak
                    // answers from behind you
                    self.cues.push((
                        t + HZ / 5,
                        Cue::Girl {
                            ahead: ONE * 22 / 10,
                            side: 0,
                            ticks: GIRL_TICKS,
                            until_seen: false,
                        },
                    ));
                    self.cues.push((
                        t + 2 * HZ,
                        Cue::Sound {
                            recipe: "squeak",
                            place: Place::Behind(int(6)),
                            loud: ONE * 4 / 10,
                            mark: Some("sub_squeak"),
                        },
                    ));
                }
            }
            CurioKind::Camera => {
                self.say_head("shutter_snap", HAND, ONE * 4 / 10);
                self.flash = FLASH_TICKS;
                if uses == CAMERA_FRAMES {
                    // she stands in the flash, and only in the flash
                    self.cues.push((
                        t,
                        Cue::Girl {
                            ahead: ONE * 15 / 10,
                            side: fx::deg(8),
                            // the flash counts down this tick already
                            ticks: FLASH_TICKS - 1,
                            until_seen: false,
                        },
                    ));
                }
            }
            CurioKind::MusicBox => {
                self.say_head("wind_up", HAND, ONE * 3 / 10);
                if fresh {
                    // the tune runs down, phrase by phrase; when it stops
                    // she giggles behind you and stands there until you
                    // turn round, or for four seconds
                    self.curios[i].spent = true;
                    let mut due = t + HZ / 2;
                    let mut gap = HZ * 3 / 4;
                    for _ in 0..TUNE_PHRASES {
                        self.cues.push((
                            due,
                            Cue::Sound {
                                recipe: "music_box",
                                place: Place::Curio(i),
                                loud: ONE * 35 / 100,
                                mark: Some("sub_music"),
                            },
                        ));
                        due += gap;
                        gap += HZ / 8;
                    }
                    self.cues.push((
                        due + HZ,
                        Cue::Girl {
                            ahead: int(2),
                            side: fx::TURN / 2,
                            ticks: 4 * HZ as i32,
                            until_seen: true,
                        },
                    ));
                }
            }
            // the telephone answers when it is picked up
            CurioKind::Phone => {}
        }
    }

    /// Each tick: the telephone starts ringing, the girl fades, cues fall due.
    fn curio_time(&mut self) {
        self.flash = (self.flash - 1).max(0);
        let origin = self.body.origin;
        if !self.rang {
            let phone = self.mall.curios.iter().position(|c| c.kind == CurioKind::Phone);
            if let Some(i) = phone {
                if self.mall.rooms[self.mall.curios[i].room].holds(origin) {
                    self.rang = true;
                    for k in 0..RINGS {
                        self.cues.push((
                            self.tick + HZ / 2 + k * 3 * HZ,
                            Cue::Sound {
                                recipe: "phone_ring",
                                place: Place::Curio(i),
                                loud: ONE * 6 / 10,
                                mark: Some("sub_ring"),
                            },
                        ));
                    }
                }
            }
        }
        if let Some(mut p) = self.phantom {
            p.ticks -= 1;
            let head = p.at.add(v3(0, GIRL_HEAD, 0));
            if p.until_seen && self.off_view(head) < fx::deg(25) && self.sees(head) {
                p.ticks = p.ticks.min(SEEN_LINGER);
                p.until_seen = false;
            }
            self.phantom = if p.ticks > 0 { Some(p) } else { None };
        }
        let now = self.tick;
        let due: Vec<Cue> = self
            .cues
            .iter()
            .filter(|(t, _)| *t <= now)
            .map(|(_, c)| *c)
            .collect();
        self.cues.retain(|(t, _)| *t > now);
        for c in due {
            self.cue(c);
        }
    }

    fn cue(&mut self, c: Cue) {
        match c {
            Cue::Sound {
                recipe,
                place,
                loud,
                mark,
            } => match place {
                Place::At(p) => self.say(recipe, p, loud, mark),
                Place::Head(off) => self.say_head_marked(recipe, off, loud, mark),
                Place::Behind(d) => {
                    let p = self
                        .body
                        .origin
                        .sub(facing(self.yaw, 0).scale(d))
                        .add(v3(0, ONE, 0));
                    self.say(recipe, p, loud, mark);
                }
                Place::Curio(i) => {
                    if self.held == Some(i) {
                        self.say_head_marked(recipe, HAND, loud, mark);
                    } else {
                        let p = self.curios[i].at.add(v3(0, ONE / 10, 0));
                        self.say(recipe, p, loud, mark);
                    }
                }
            },
            Cue::Girl {
                ahead,
                side,
                ticks,
                until_seen,
            } => self.show_girl(ahead, side, ticks, until_seen),
        }
    }

    /// The girl `ahead` along the view turned by `side`: where she would
    /// stand inside a wall or a counter, nearer, then farther (behind the
    /// counter), wherever she stands clear and in plain sight; where there
    /// is no such place, only her giggle.
    fn show_girl(&mut self, ahead: Fx, side: i32, ticks: i32, until_seen: bool) {
        let dir = facing(self.yaw + side, 0);
        let eye = self.eye();
        let step = ONE / 5;
        let nearer = (0..).map(|k| ahead - k * step).take_while(|d| *d >= GIRL_NEAREST);
        let farther = (1..=GIRL_BEYOND / step).map(|k| ahead + k * step);
        let spots: Vec<Fx> = nearer.chain(farther).collect();
        for d in spots {
            let at = self.body.origin.add(dir.scale(d));
            if self.girl_fits(at) && self.clear_line(eye, at.add(v3(0, GIRL_HEAD, 0))) {
                self.phantom = Some(Phantom {
                    at,
                    yaw: yaw_of(self.body.origin.sub(at)),
                    ticks,
                    until_seen,
                });
                self.say("giggle", at.add(v3(0, ONE, 0)), ONE * 4 / 10, Some("sub_giggle"));
                return;
            }
        }
        self.say_head_marked("giggle", v3(0, 0, ONE / 3), ONE * 4 / 10, Some("sub_giggle"));
    }

    /// Whether the girl, standing at `at`, is clear of every wall, counter
    /// and door.
    pub fn girl_fits(&self, at: V3) -> bool {
        let mut sc = Scratch::default();
        let half = v3(ONE / 5, ONE / 2, ONE / 5);
        let mid = at.add(v3(0, ONE * 6 / 10, 0));
        let t = trace_box(
            &self.mall.map,
            &mut sc,
            mid,
            mid.add(v3(0, ONE / 50, 0)),
            half.neg(),
            half,
        );
        !t.start_solid && !t.hit()
    }

    /// A sound in the head that may also go on the ring: its bearing is
    /// the view's heading turned by where it sits in the head.
    fn say_head_marked(&mut self, recipe: &'static str, off: V3, loud: Fx, mark: Option<&'static str>) {
        self.say_head(recipe, off, loud);
        if let Some(key) = mark {
            self.marks.push(Mark {
                key,
                angle: self.yaw + yaw_of(off),
                tick: self.tick,
            });
        }
    }

    // T18: apparitions ----------------------------------------------------

    /// Eyes, a smile or reaching hands at the edge of the view. Each stays
    /// there, kept at the edge while you turn, until you look straight at
    /// it, come within 2 m, or a minute passes; then it goes.
    fn apparitions(&mut self) {
        self.glimpse_wait = (self.glimpse_wait - 1).max(0);
        let eye = self.eye();
        if let Some(mut a) = self.apparition {
            a.age += 1;
            if a.fading > 0 {
                a.fading -= 1;
                self.apparition = if a.fading > 0 { Some(a) } else { None };
                return;
            }
            let off = self.off_view(a.at);
            let near = a.at.sub(eye).len_xz() < APPARITION_NEAREST;
            if off < LOOKED_AT || near || a.age > APPARITION_LIFE {
                a.fading = FADE_TICKS;
                self.glimpse_wait = APPARITION_WAIT;
                self.say("wisp", a.at, ONE / 4, None);
            } else if off > SCREEN_EDGE {
                // gone off the screen as you turned: back to the edge of
                // the view on its side, moving only while unseen
                if let Some(at) = self.apparition_spot(a.kind, a.side, a.dist) {
                    a.at = at;
                }
            }
            self.apparition = Some(a);
            return;
        }
        if self.menace() < 1 || self.glimpse_wait > 0 || !self.rng.chance(1, APPARITION_CHANCE) {
            return;
        }
        let kind = match self.rng.below(3) {
            0 => Sight::Eyes,
            1 => Sight::Smile,
            _ => Sight::Hands,
        };
        let off = GLIMPSE_NEAR + self.rng.below((GLIMPSE_FAR - GLIMPSE_NEAR) as u32) as i32;
        let side = if self.rng.chance(1, 2) { off } else { -off };
        // T20: nearer (was 4 to 8 m)
        let dist = ONE * 25 / 10 + self.rng.below(int(3) as u32) as i32;
        if let Some(at) = self.apparition_spot(kind, side, dist) {
            self.apparition = Some(Apparition {
                kind,
                at,
                side,
                dist,
                age: 0,
                fading: 0,
            });
        }
    }

    /// Where an apparition of `kind` stands `side` off the view and up to
    /// `dist` away: nearer until the eye can see it and it is clear of
    /// every wall; at least 3 m off.
    pub fn apparition_spot(&self, kind: Sight, side: i32, dist: Fx) -> Option<V3> {
        let dir = facing(self.yaw + side, 0);
        let eye = self.eye();
        let mut d = dist;
        while d >= ONE * 2 {
            let at = self.body.origin.add(dir.scale(d)).add(v3(0, kind.height(), 0));
            let mut sc = Scratch::default();
            let half = v3(ONE / 5, ONE / 5, ONE / 5);
            let t = trace_box(
                &self.mall.map,
                &mut sc,
                at,
                at.add(v3(0, ONE / 50, 0)),
                half.neg(),
                half,
            );
            if !t.start_solid && !t.hit() && self.clear_line(eye, at) {
                return Some(at);
            }
            d -= ONE / 2;
        }
        None
    }

    // T14: the fuse box ----------------------------------------------------

    /// The court's lamps go out; every cultist is blind for a moment.
    fn cut_power(&mut self, to: Breaker) {
        if self.breaker == Breaker::On {
            self.breaker_saved = self
                .mall
                .breaker_feeds
                .iter()
                .map(|&i| (i, self.lights.states[i]))
                .collect();
            for &(i, _) in &self.breaker_saved {
                self.lights.states[i] = crate::lights::State::Broken;
            }
            self.dazzled = DAZZLE_TICKS;
            for c in self.cultists.iter() {
                if c.mind != Mind::Dead {
                    let at = c.at.add(v3(0, ONE * 16 / 10, 0));
                    self.voices.push(Voice {
                        recipe: "mutter",
                        at,
                        loud: ONE * 6 / 10,
                        head: false,
                        marked: None,
                    });
                }
            }
        }
        self.breaker = to;
    }

    /// Someone throws the breaker back when its time is up; the lamps come
    /// on as they were, unless the blackout has broken everything since.
    fn fuse_time(&mut self) {
        self.dazzled = (self.dazzled - 1).max(0);
        if let Breaker::Thrown(until) = self.breaker {
            if self.tick >= until {
                self.breaker = Breaker::On;
                if self.blackout.is_none() {
                    for &(i, was) in &self.breaker_saved {
                        self.lights.states[i] = was;
                    }
                }
                self.say("breaker", self.mall.fusebox, ONE * 3 / 10, Some("sub_breaker"));
            }
        }
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
            self.say("step_stranger", p, ONE * 7 / 10, None);
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
        // T6: from the doorway you can watch them; inside the court (or
        // once one is after you) they can see you
        let in_court = self.mall.rooms[10].holds(me);
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
            let looking_out = in_court || matches!(c.mind, Mind::Search | Mind::Attack);
            if !hidden && looking_out && dist < int(25) && off < fx::deg(55) && self.clear_line(ceye, eye) {
                let mut s = fx::div(lit, dist.max(ONE));
                if self.body.ducked {
                    s /= 2;
                }
                // nothing at all while their eyes are lost in the sudden
                // dark (T14); T20: no floor below which you are a shadow,
                // the dark court does that now
                sight = if self.dazzled > 0 { 0 } else { s };
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
                    let goal = c.at.add(v3(fx::div(away.x * 4, l), 0, fx::div(away.z * 4, l)));
                    let next = self.cult_step(c.at, goal, ONE * 3 / HZ as i32);
                    if self.mall.rooms[10].holds(next) {
                        c.at = next;
                    }
                    c.yaw = yaw_of(away);
                }
                _ => {
                    c.suspicion += sight / SUSPICION_SLOW;
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
                                let r = mall::PATROL_R;
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
                        if way.len_xz() > ONE / 2 && speed > 0 {
                            // T8: through doorways, around walls
                            let next = self.cult_step(c.at, goal, speed / HZ as i32);
                            let moved = next.sub(c.at);
                            if moved.len_xz() > 0 {
                                c.yaw = turn_toward(c.yaw, yaw_of(moved), fx::deg(6));
                            }
                            c.at = next;
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
                        // T9: loud enough to hear across the court: your warning
                        self.say("mutter", ceye, ONE * 8 / 10, Some("sub_mutter"));
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
            // glimpses of the spirit at the edge of the view (menace 3 and
            // 4); T18: held 1.25 s (was a third of a second)
            self.spirit.glimpse = (self.spirit.glimpse - 1).max(0);
            self.spirit_wait = (self.spirit_wait - 1).max(0);
            if self.menace() >= 3 && self.spirit_wait == 0 && self.rng.chance(1, 900) {
                // CH-Q25: anywhere from GLIMPSE_NEAR to GLIMPSE_FAR off the
                // view, 5 to 12 m off, where the eye can see it, and not
                // again for 20 s
                let off = GLIMPSE_NEAR + self.rng.below((GLIMPSE_FAR - GLIMPSE_NEAR) as u32) as i32;
                let side = if self.rng.chance(1, 2) { off } else { -off };
                let dir = facing(self.yaw + side, 0);
                let mut d = int(5) + self.rng.below(int(7) as u32) as i32;
                let eye = self.eye();
                while d >= int(5) {
                    let at = self.body.origin.add(dir.scale(d));
                    if self.clear_line(eye, at.add(v3(0, ONE, 0))) {
                        self.spirit.at = at;
                        self.spirit.glimpse = GLIMPSE_TICKS;
                        self.spirit_wait = 20 * HZ as i32;
                        break;
                    }
                    d -= ONE;
                }
            }
            self.apparitions();
            return;
        }
        let s = &mut self.spirit;
        s.chase_ticks += 1;
        // 1.1 m to 4 m over a minute and a half (T19; was three minutes),
        // from when it starts to come
        let since = (s.chase_ticks - RISE_TICKS).max(0);
        let grow = (since as i64 * (ONE * 29 / 10) as i64 / (GROW_TICKS as i64)) as Fx;
        s.size = (ONE * 11 / 10 + grow).min(int(4));
        // T15, T19: it stands where it rose and stares at you, then
        // screams and comes
        if s.chase_ticks <= RISE_TICKS {
            return;
        }
        if s.chase_ticks == RISE_TICKS + 1 {
            let at = s.at.add(v3(0, s.size, 0));
            self.say("spirit_scream", at, ONE, Some("sub_spirit_scream"));
            self.hush = HUSH_TICKS;
        }
        let s = &mut self.spirit;
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
        // T19: slow at first, at full pace after SPEED_RAMP
        let since = s.chase_ticks - RISE_TICKS;
        let hunt = HUNT_FROM + (HUNT_TOP - HUNT_FROM) * since.min(SPEED_RAMP) / SPEED_RAMP;
        let speed = if lost { ONE * 12 / 10 } else { hunt };
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
        } else if !hidden && prey.is_none() {
            // T19: where it last saw you and you are gone: it follows your
            // trail (it used to stand there until it gave up); hiding
            // breaks the trail
            self.spirit.last_known = me;
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
        // T21: by the fewest rooms (breadth first, as the cultists go, T8),
        // through shut doors too; it used to guess the doorway from where
        // the sound seemed to come from, and could pick one that led
        // nowhere, or the one it stood on, again and again
        let (a, b) = (self.mall.room_of(from), self.mall.room_of(goal));
        if a == b {
            return goal;
        }
        let n = self.mall.rooms.len();
        let mut next: Vec<Option<usize>> = vec![None; n];
        let mut seen = vec![false; n];
        let mut queue = std::collections::VecDeque::from([b]);
        seen[b] = true;
        while let Some(r) = queue.pop_front() {
            for (pi, p) in self.mall.portals.iter().enumerate() {
                let other = if p.a == r {
                    p.b
                } else if p.b == r {
                    p.a
                } else {
                    continue;
                };
                if !seen[other] {
                    seen[other] = true;
                    next[other] = Some(pi);
                    queue.push_back(other);
                }
            }
        }
        let Some(pi) = next[a] else {
            return goal;
        };
        let p = self.mall.portals[pi];
        let door = v3(p.at.x, 0, p.at.z);
        // on the doorway itself: on into the room beyond, toward the next
        // doorway there or the goal
        if door.sub(v3(from.x, 0, from.z)).len_xz() < ONE / 2 {
            let beyond = if p.a == a { p.b } else { p.a };
            if beyond == b {
                return goal;
            }
            if let Some(qi) = next[beyond] {
                let q = self.mall.portals[qi].at;
                return v3(q.x, 0, q.z);
            }
            return goal;
        }
        door
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
    Fusebox,
    Cultist(usize),
    Fixture(usize),
}

pub fn per_minute(menace: i32) -> i32 {
    // T20: more past the doors (were 1, 2, 3, 3, 4)
    [1, 3, 4, 5, 6][menace.clamp(0, 4) as usize]
}

/// Suspicion grows by sight ÷ this a tick (was 20, T9): a lit walker
/// gets about eight seconds from the first mutter to the attack.
pub const SUSPICION_SLOW: Fx = 40;

/// Half a cultist's width, for walls (T8).
pub const CULT_HALF: Fx = ONE * 3 / 10;

/// Mannequins farther than this are not turned by the director (CH-Q24).
pub const MANNEQUIN_REACH: Fx = 15 * ONE;
/// Glimpses of the spirit fall this far off the view (CH-Q25): inside a
/// 16:9 screen's edge (51° at a 70° vertical view), never at its centre.
pub const GLIMPSE_NEAR: i32 = fx::deg(35);
pub const GLIMPSE_FAR: i32 = fx::deg(48);

/// How far a sign turns toward you each tick of the chase: 30° a second.
pub const SIGN_TURN: i32 = fx::deg(1) / 2;

/// T12, the curios: one is picked up within `REACH` and this far off the
/// view (a toy on the floor needs a look down).
pub const CURIO_LOOK: i32 = fx::deg(50);
/// the squeeze of the toy that brings the girl
pub const TOY_SQUEEZES: i32 = 2;
/// the picture that has her in it
pub const CAMERA_FRAMES: i32 = 2;
/// how long the camera's flash lights the store, and her in it
pub const FLASH_TICKS: i32 = 10;
/// how long she stands in front of the toy, 0.4 s
pub const GIRL_TICKS: i32 = 24;
/// how long she stays once looked at
pub const SEEN_LINGER: i32 = 8;
/// the nearest she comes, and her head's height
pub const GIRL_NEAREST: Fx = ONE * 8 / 10;
pub const GIRL_HEAD: Fx = ONE * 95 / 100;
/// how much farther than meant she may stand, behind a counter
pub const GIRL_BEYOND: Fx = 2 * ONE;
/// how many phrases the music box plays, how often the telephone rings
pub const TUNE_PHRASES: i32 = 8;
pub const RINGS: i64 = 6;

/// T14: how long the thrown breaker keeps the court dark, and how long the
/// sudden dark blinds the cult
pub const BREAKER_TICKS: i64 = 40 * HZ;
pub const DAZZLE_TICKS: i32 = 12 * HZ as i32;

/// T15: how long the spirit stands where it rose before it hunts
pub const RISE_TICKS: i32 = 3 * HZ as i32;
/// T19: its pace when it starts to come and at full hunt, the ticks
/// between, and the ticks it takes to grow to full size
pub const HUNT_FROM: Fx = ONE;
pub const HUNT_TOP: Fx = ONE * 36 / 10;
pub const SPEED_RAMP: i32 = 20 * HZ as i32;
pub const GROW_TICKS: i32 = 90 * HZ as i32;

/// T16: the most the eye brightens the dark
pub const EXPOSURE_DARK: Fx = ONE;

/// T18: an apparition goes once it is this near the middle of the view
/// ("looked at"), once you are this near, or after a minute; it keeps
/// inside the screen's edge (51° at 16:9); one comes about every ten
/// seconds at menace 1 and up, 25 s after the last went
pub const LOOKED_AT: i32 = fx::deg(12);
/// how long a glimpse of the spirit stays (was 20 ticks)
pub const GLIMPSE_TICKS: i32 = 75;
pub const APPARITION_NEAREST: Fx = 2 * ONE;
pub const APPARITION_LIFE: i32 = 60 * HZ as i32;
pub const SCREEN_EDGE: i32 = fx::deg(51);
pub const FADE_TICKS: i32 = 6;
pub const APPARITION_WAIT: i32 = 15 * HZ as i32;
pub const APPARITION_CHANCE: u32 = 300;

/// T17: how long the room's sounds hush after a director scare, and how
/// low they go
pub const HUSH_TICKS: i32 = 3 * HZ as i32;
pub const HUSH_FLOOR: Fx = ONE * 15 / 100;
/// in the head: the right hand, and the right ear for the receiver
pub const HAND: V3 = V3 {
    x: ONE / 5,
    y: -ONE / 4,
    z: -ONE * 4 / 10,
};
pub const EAR: V3 = V3 {
    x: ONE * 12 / 100,
    y: 0,
    z: 0,
};

fn take_prompt(k: CurioKind) -> &'static str {
    match k {
        CurioKind::Toy => "prompt_take_toy",
        CurioKind::Camera => "prompt_take_camera",
        CurioKind::MusicBox => "prompt_take_music_box",
        CurioKind::Phone => "prompt_take_phone",
    }
}

fn hold_prompt(k: CurioKind) -> &'static str {
    match k {
        CurioKind::Toy => "prompt_hold_toy",
        CurioKind::Camera => "prompt_hold_camera",
        CurioKind::MusicBox => "prompt_hold_music_box",
        CurioKind::Phone => "prompt_hold_phone",
    }
}

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

pub mod census;

#[cfg(test)]
mod tests;
