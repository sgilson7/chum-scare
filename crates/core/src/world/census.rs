//! The scare census (T5): a stand-in player walks the route through the
//! real world, tick by tick, and every scary thing the world does is
//! written down with where it was relative to the view. Nothing here
//! decides a rule: the census only reads what `World::tick` did.

use super::*;
use serde::Serialize;

/// One thing the player could hear or see.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Event {
    /// seconds since the doors, in tenths
    pub t: i32,
    /// "director", "possession", "spirit", "cult", "ritual"
    pub source: &'static str,
    /// the recipe or what moved ("glimpse", "mannequin_turn", …)
    pub what: &'static str,
    /// "sound" or "sight"
    pub sense: &'static str,
    /// degrees between the view and the event
    pub off: i32,
    /// metres away, in tenths
    pub dist: i32,
    pub menace: i32,
    pub busy: bool,
    pub stage: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Census {
    pub seed: u64,
    pub player: &'static str,
    pub events: Vec<Event>,
    /// tenths of a second spent at each menace (0 to 4), before the chase
    pub at_menace: [i32; 5],
    /// tenths of a second with the hands busy, before the chase
    pub busy: i32,
    /// tenths of a second in each stage
    pub stages: Vec<(&'static str, i32)>,
    pub hurt: i32,
    pub escaped: bool,
    pub length: i32,
}

#[derive(Debug, Clone, Copy)]
enum Step {
    /// walk to (x, z) metres at a speed in cm/s, crouched or not, light on or off
    Walk(i32, i32, i32, bool, bool),
    /// stand and look around for this many seconds
    Look(i32),
    /// load this many rounds, this many tenths of a second each
    Load(i32, i32),
    /// turn the cursed store's handle for this many seconds
    Door(i32),
    /// the ouija board, this many seconds, then the chase
    Ouija(i32),
}

/// The route the stand-in player takes (stage, step).
fn route(player: &str) -> Vec<(&'static str, Step)> {
    use Step::*;
    let careful = player == "methodical";
    let (walk, look) = if careful { (130, 4) } else { (260, 1) };
    let mut r = vec![("load", Load(6, if careful { 35 } else { 25 }))];
    let wing: &[(i32, i32)] = &[
        (12, 0),
        (24, 0),
        (24, 8),
        (24, 0),
        (38, 0),
        (38, 9),
        (38, 0),
        (50, 0),
        (58, 0),
    ];
    for (k, &(x, z)) in wing.iter().enumerate() {
        // the methodical player hears the chant and puts the light out
        // before the court's doorway; the brisk one does not
        let light = !(careful && k == wing.len() - 1);
        r.push(("wings", Walk(x, z, walk, false, light)));
        if careful || k % 3 == 2 {
            r.push(("wings", Look(look)));
        }
        if careful && k == 4 {
            r.push(("load", Load(2, 35)));
        }
    }
    let crouch = 52;
    if careful {
        for &(x, z) in &[(60, 2), (64, 13), (86, 13), (89, 0)] {
            r.push(("court", Walk(x, z, crouch, true, false)));
        }
    } else {
        for &(x, z) in &[(60, 2), (66, 10), (84, 10), (89, 0)] {
            r.push(("court", Walk(x, z, 130, false, true)));
        }
    }
    r.push(("door", Door(if careful { 8 } else { 5 })));
    r.push(("cursed", Walk(98, 0, 130, false, true)));
    r.push(("ouija", Ouija(10)));
    for &(x, z) in &[(89, 0), (60, 0), (30, 0), (0, 0), (-7, 0)] {
        r.push(("chase", Walk(x, z, 300, false, true)));
    }
    r
}

/// Degrees in an angle.
fn degrees(a: i32) -> i32 {
    (a as i64 * 360 / fx::TURN as i64) as i32
}

/// Where a recipe comes from, when it is one that frightens.
fn source_of(v: &Voice, blackout: bool) -> Option<&'static str> {
    Some(match v.recipe {
        "door_creak" | "fall_crash" | "shutter" | "laugh" | "step_other" | "breath" => "director",
        "pop_glass" if v.marked == Some("sub_light") => "director",
        "pop_glass" if blackout => "ritual",
        "slam" => "ritual",
        "plastic_knock" | "horn" => "possession",
        "spirit_breath" | "scream_cut" | "wisp" => "spirit",
        "mutter" | "shout" | "scream" | "dagger" => "cult",
        _ => return None,
    })
}

/// Walk `player` ("methodical" or "brisk") through the world made from `seed`.
pub fn census(seed: u64, player: &'static str) -> Census {
    let mut w = World::new(seed, 22 * 60);
    w.act("start", "").unwrap();
    let full = w.health;
    let cursed = w.mall.doors.iter().position(|d| d.key == "cursed").unwrap_or(0);
    let mut c = Census {
        seed,
        player,
        events: Vec::new(),
        at_menace: [0; 5],
        busy: 0,
        stages: Vec::new(),
        hurt: 0,
        escaped: false,
        length: 0,
    };
    let tenths = |w: &World| ((w.tick - 1) * 10 / HZ) as i32;
    let mut look_t = 0i32;
    for (stage, step) in route(player) {
        let start = w.tick;
        // the ticks this step takes, and what each sets before the tick
        let ticks: i64 = match step {
            Step::Walk(x, z, speed, ..) => {
                let d = v3(int(x), 0, int(z)).sub(w.body.origin).len_xz();
                (d as i64 * 100 * HZ / (speed as i64 * ONE as i64)).max(1)
            }
            Step::Look(s) | Step::Door(s) | Step::Ouija(s) => s as i64 * HZ,
            Step::Load(n, each) => n as i64 * each as i64 * HZ / 10,
        };
        let from = w.body.origin;
        let heading = w.yaw;
        for k in 0..ticks {
            // the board spells R U N itself and starts the chase
            if matches!(step, Step::Ouija(_)) && w.spirit.chasing {
                break;
            }
            look_t += 1;
            // a slow look around: ±35° walking, ±80° standing
            let sway = |most: i32, period: i32| {
                fx::mul(fx::deg(most), fx::sin(look_t * fx::TURN / (period * HZ as i32)))
            };
            let mut keys = 0;
            // the right button is held while the hands are out
            let hands = matches!(step, Step::Load(..) | Step::Door(_) | Step::Ouija(_));
            match step {
                Step::Walk(x, z, _, crouch, light) => {
                    let to = v3(int(x), 0, int(z));
                    let way = to.sub(from);
                    w.body.origin = from.add(way.scale(fx::div((k + 1) as i32, ticks as i32)));
                    let ahead = if way.len_xz() > 0 { yaw_of(way) } else { heading };
                    w.yaw = (ahead + if stage == "chase" { 0 } else { sway(35, 7) }).rem_euclid(fx::TURN);
                    w.pitch = 0;
                    w.flashlight.on = light;
                    if crouch {
                        keys |= CROUCH;
                    }
                    if w.mode != Mode::Look && !matches!(w.mode, Mode::Hidden(_)) {
                        w.mode = Mode::Look;
                    }
                }
                Step::Look(_) => {
                    w.yaw = (heading + sway(80, 6)).rem_euclid(fx::TURN);
                    w.pitch = 0;
                }
                Step::Load(n, each) => {
                    // eyes down on the cylinder, a round every `each` tenths
                    w.mode = Mode::Reload;
                    w.pitch = -PITCH_LIMIT;
                    let per = each as i64 * HZ / 10;
                    let done = ((k / per) as i32 + 1).min(n);
                    if k % per == per - 1 {
                        w.armed = true;
                        w.loaded_now = done;
                    }
                    if k == ticks - 1 {
                        w.mode = Mode::Look;
                        w.loaded_now = 0;
                        w.pitch = 0;
                    }
                }
                Step::Door(_) => {
                    w.mode = Mode::Door(cursed);
                    w.yaw = fx::deg(-90);
                    w.pitch = 0;
                    if k == ticks - 1 {
                        w.door_open[cursed] = ONE;
                        let b = w.mall.doors[cursed].brush;
                        w.mall.map.brushes[b] = mall::open_brush();
                        w.mode = Mode::Look;
                    }
                }
                Step::Ouija(_) => {
                    w.mode = Mode::Ouija;
                    w.pitch = -PITCH_LIMIT / 2;
                    if k == ticks - 1 {
                        // the board never finished: end it as it would
                        w.letters = 3;
                        w.mode = Mode::Look;
                        w.pitch = 0;
                        w.begin_chase();
                    }
                }
            }
            w.body.velocity = V3::ZERO;
            w.health = full;
            let seen = Seen::before(&w);
            w.tick(&Input {
                keys,
                buttons: if hands && w.mode != Mode::Look { ALT } else { 0 },
                ..Input::default()
            });
            seen.after(&w, stage, &mut c);
            // counted after the tick: the mode the world kept, not the one set
            if !w.spirit.chasing {
                w.at_menace_tick(&mut c);
            }
            if w.health < full {
                c.hurt += 1;
            }
            if w.phase != Phase::Play {
                w.phase = Phase::Play;
            }
        }
        let t = ((w.tick - start) * 10 / HZ) as i32;
        match c.stages.iter_mut().find(|s| s.0 == stage) {
            Some(s) => s.1 += t,
            None => c.stages.push((stage, t)),
        }
    }
    c.escaped = w.body.origin.x < w.mall.exit_x + ONE;
    c.length = tenths(&w);
    c
}

impl World {
    fn at_menace_tick(&self, c: &mut Census) {
        // counted in ticks, turned to tenths at the end of each second
        if self.tick % (HZ / 10) == 0 {
            c.at_menace[self.menace().clamp(0, 4) as usize] += 1;
            if self.hands_busy() {
                c.busy += 1;
            }
        }
    }
}

/// What the census compares across a tick.
struct Seen {
    glimpse: i32,
    props: Vec<(V3, i32)>,
    possessed: Option<usize>,
    broken: usize,
    last_scare: i64,
}

impl Seen {
    fn before(w: &World) -> Seen {
        Seen {
            glimpse: w.spirit.glimpse,
            props: w.mall.props.iter().map(|p| (p.at, p.yaw)).collect(),
            possessed: w.possessed,
            broken: w
                .lights
                .states
                .iter()
                .filter(|s| **s == crate::lights::State::Broken)
                .count(),
            last_scare: w.last_scare,
        }
    }

    fn after(self, w: &World, stage: &'static str, c: &mut Census) {
        let eye = w.eye();
        let t = ((w.tick - 1) * 10 / HZ) as i32;
        let menace = w.menace();
        let busy = w.hands_busy();
        let mut note = |source, what, sense, at: Option<V3>, off: Option<i32>| {
            let (off, dist) = match at {
                Some(p) => (
                    degrees(off_view(w.yaw, w.pitch, eye, p)),
                    p.sub(eye).len() * 10 / ONE,
                ),
                None => (off.unwrap_or(0), 0),
            };
            c.events.push(Event {
                t,
                source,
                what,
                sense,
                off,
                dist,
                menace,
                busy,
                stage,
            });
        };
        for v in &w.voices {
            let Some(source) = source_of(v, w.blackout.is_some()) else {
                continue;
            };
            if v.head {
                // breath on the neck sits behind the head (160°); the
                // board's slam in front
                let off = if v.recipe == "breath" { 160 } else { 0 };
                note(source, v.recipe, "sound", None, Some(off));
            } else {
                note(source, v.recipe, "sound", Some(v.at), None);
            }
        }
        // the director fired this tick
        if w.last_scare != self.last_scare {
            note("director", "scare", "decision", None, Some(0));
        }
        if w.spirit.glimpse == 20 && self.glimpse != 20 {
            note(
                "spirit",
                "glimpse",
                "sight",
                Some(w.spirit.at.add(v3(0, ONE, 0))),
                None,
            );
        }
        for (i, p) in w.mall.props.iter().enumerate() {
            let (was_at, was_yaw) = self.props[i];
            let head = p.at.add(v3(0, ONE * 15 / 10, 0));
            if p.at != was_at {
                note("possession", "step_closer", "sight", Some(head), None);
            } else if p.yaw != was_yaw && Some(i) != self.possessed {
                note("director", "mannequin_turn", "sight", Some(head), None);
            }
        }
        let broken = w
            .lights
            .states
            .iter()
            .filter(|s| **s == crate::lights::State::Broken)
            .count();
        if broken > self.broken && w.blackout.is_none() {
            note("director", "light_out", "sight", None, Some(0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_census_walks_the_whole_route_and_gets_out() {
        let c = census(11, "methodical");
        assert!(c.escaped, "ends at {}", c.length);
        // the methodical walk: 6 × 3.5 s + 2 × 3.5 s of loading and 8 s at
        // the handle are hands-busy time before the chase (36 s); the board
        // adds up to 10 s more, less when it spells R U N sooner
        assert!(c.busy >= 360 - 6 && c.busy <= 460 + 6, "{}", c.busy);
    }

    #[test]
    fn no_director_scare_while_loading_is_ever_in_view() {
        // CH-4 as the census sees it: every director sound made while the
        // hands are busy is 58° or more off the view
        for seed in 1..6 {
            let c = census(seed, "methodical");
            for e in &c.events {
                if e.source == "director" && e.sense == "sound" && e.busy && e.what != "step_other" {
                    assert!(e.off >= 58, "{e:?}");
                }
            }
        }
    }
}
