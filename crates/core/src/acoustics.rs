//! Where a sound seems to come from, how loud and how muffled (journal tab
//! 4, CH-5). The mall is rooms joined by portals; a sound reaches a listener
//! by the shortest route through them, or straight through the walls,
//! whichever is louder. Through portals it seems to come from the last
//! portal before the listener, at the whole route's length; each bend at a
//! portal muffles it, a shut door more. The same answer tells a cultist how
//! loud your footsteps are where it stands (tab 2's stealth).

use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};
use crate::mall::{Portal, Room};

/// Loudness is `REF / distance`, at most 1 (2 m).
pub const REF: Fx = 2 * ONE;
/// The low-pass cut-off with nothing in the way, and its floor (Hz).
pub const OPEN_HZ: i32 = 18_000;
pub const SHUT_HZ: i32 = 700;
pub const WALL_HZ: i32 = 600;
/// What a 90° bend costs: the cut-off falls by this much (Hz) and the gain
/// to this share; a straight route costs nothing.
pub const BEND_HZ: i32 = 16_800;
pub const BEND_GAIN: Fx = ONE / 2;
/// A shut door lets this share through; a wall this share.
pub const SHUT_GAIN: Fx = ONE * 12 / 100;
pub const WALL_GAIN: Fx = ONE * 6 / 100;
/// A shut door also lengthens the route by this much, so an open way round
/// is taken when it is not much longer.
pub const SHUT_EXTRA: Fx = 8 * ONE;
/// The reverb sends: the room a sound is made in, and the listener's own.
pub const SEND_SOURCE: Fx = ONE * 40 / 100;
pub const SEND_LISTENER: Fx = ONE * 15 / 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heard {
    /// 0 to `ONE`
    pub gain: Fx,
    pub cutoff_hz: i32,
    /// where the page places it
    pub apparent: V3,
    pub distance: Fx,
    /// the rooms whose reverb it carries, with their sends
    pub source_room: usize,
    pub listener_room: usize,
}

fn dist_xz(a: V3, b: V3) -> Fx {
    v3(a.x - b.x, 0, a.z - b.z).len_xz()
}

/// The angle a route turns at `at`, coming from `from` and going to `to`,
/// in `fx` angle units (a quarter turn is `fx::QUARTER_TURN`).
pub fn bend(from: V3, at: V3, to: V3) -> i32 {
    let a = at.sub(from);
    let b = to.sub(at);
    let dot = fx::mul(a.x, b.x) + fx::mul(a.z, b.z);
    let cross = fx::mul(a.x, b.z) - fx::mul(a.z, b.x);
    fx::wrap(fx::atan2(cross, dot)).abs()
}

/// Gain and cut-off after a bend: linear in the angle up to a quarter turn.
pub fn bent(angle: i32) -> (Fx, i32) {
    let share = (angle.min(fx::QUARTER_TURN) as i64 * ONE as i64 / fx::QUARTER_TURN as i64) as Fx;
    let gain = ONE - fx::mul(share, ONE - BEND_GAIN);
    let hz = OPEN_HZ - (BEND_HZ as i64 * share as i64 / ONE as i64) as i32;
    (gain, hz)
}

pub fn distance_gain(d: Fx) -> Fx {
    if d <= REF {
        ONE
    } else {
        fx::div(REF, d)
    }
}

/// How `from` is heard at `to`. `open` gives each door's openness (0 shut,
/// `ONE` open); `straight_blocked` says whether walls cut the straight line
/// (the world traces it).
pub fn hear(
    rooms: &[Room],
    portals: &[Portal],
    open: &[Fx],
    from: V3,
    to: V3,
    straight_blocked: bool,
) -> Heard {
    let room_of = |p: V3| rooms.iter().position(|r| r.holds(p)).unwrap_or(0);
    let sr = room_of(from);
    let lr = room_of(to);
    let straight = dist_xz(from, to).max(ONE / 10);
    let direct = if straight_blocked {
        Heard {
            gain: fx::mul(distance_gain(straight), WALL_GAIN),
            cutoff_hz: WALL_HZ,
            apparent: from,
            distance: straight,
            source_room: sr,
            listener_room: lr,
        }
    } else {
        Heard {
            gain: distance_gain(straight),
            cutoff_hz: OPEN_HZ,
            apparent: from,
            distance: straight,
            source_room: sr,
            listener_room: lr,
        }
    };
    if sr == lr {
        return direct;
    }
    match route(rooms.len(), portals, open, from, sr, to, lr) {
        Some(path) => {
            let mut length = 0;
            let mut prev = from;
            let mut gain = ONE;
            let mut hz = OPEN_HZ;
            for (k, &pi) in path.iter().enumerate() {
                let p = &portals[pi];
                length += dist_xz(prev, p.at);
                let next = path.get(k + 1).map(|&n| portals[n].at).unwrap_or(to);
                let (g, h) = bent(bend(prev, p.at, next));
                gain = fx::mul(gain, g);
                hz = hz.min(h);
                if let Some(dn) = p.door {
                    let o = open.get(dn).copied().unwrap_or(ONE);
                    let through = SHUT_GAIN + fx::mul(ONE - SHUT_GAIN, o);
                    gain = fx::mul(gain, through);
                    if o < ONE / 2 {
                        hz = hz.min(SHUT_HZ);
                    }
                }
                prev = p.at;
            }
            length += dist_xz(prev, to);
            let length = length.max(ONE / 10);
            gain = fx::mul(gain, distance_gain(length));
            let last = portals[*path.last().unwrap_or(&0)].at;
            let toward = v3(last.x - to.x, 0, last.z - to.z);
            let along = toward.len_xz().max(1);
            let apparent = v3(
                to.x + fx::div(fx::mul(toward.x, length), along),
                last.y,
                to.z + fx::div(fx::mul(toward.z, length), along),
            );
            let routed = Heard {
                gain,
                cutoff_hz: hz,
                apparent,
                distance: length,
                source_room: sr,
                listener_room: lr,
            };
            if routed.gain >= direct.gain {
                routed
            } else {
                direct
            }
        }
        None => direct,
    }
}

/// The portals on the shortest route from room `sr` to room `lr`
/// (Dijkstra over the portals; a shut door lengthens its edge).
fn route(
    rooms: usize,
    portals: &[Portal],
    open: &[Fx],
    from: V3,
    sr: usize,
    to: V3,
    lr: usize,
) -> Option<Vec<usize>> {
    let n = portals.len();
    let cost = |p: &Portal| match p.door {
        Some(dn) if open.get(dn).copied().unwrap_or(ONE) < ONE / 2 => SHUT_EXTRA,
        _ => 0,
    };
    let touches = |p: &Portal, r: usize| p.a == r || p.b == r;
    let mut best = vec![i64::MAX; n];
    let mut back = vec![usize::MAX; n];
    let mut done = vec![false; n];
    for (i, p) in portals.iter().enumerate() {
        if touches(p, sr) {
            best[i] = (dist_xz(from, p.at) + cost(p)) as i64;
        }
    }
    let _ = rooms;
    loop {
        let mut at = usize::MAX;
        let mut low = i64::MAX;
        for i in 0..n {
            if !done[i] && best[i] < low {
                low = best[i];
                at = i;
            }
        }
        if at == usize::MAX {
            break;
        }
        done[at] = true;
        let pa = &portals[at];
        for (j, pj) in portals.iter().enumerate() {
            if done[j] {
                continue;
            }
            let shared = touches(pj, pa.a) || touches(pj, pa.b);
            if shared {
                let c = low + (dist_xz(pa.at, pj.at) + cost(pj)) as i64;
                if c < best[j] {
                    best[j] = c;
                    back[j] = at;
                }
            }
        }
    }
    let mut end = usize::MAX;
    let mut low = i64::MAX;
    for (i, p) in portals.iter().enumerate() {
        if touches(p, lr) && best[i] != i64::MAX {
            let c = best[i] + dist_xz(p.at, to) as i64;
            if c < low {
                low = c;
                end = i;
            }
        }
    }
    if end == usize::MAX {
        return None;
    }
    let mut path = vec![end];
    while back[*path.last()?] != usize::MAX {
        path.push(back[*path.last()?]);
    }
    path.reverse();
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::int;
    use crate::mall::{Reverb, TILE};

    fn two_rooms(door: Option<usize>) -> (Vec<Room>, Vec<Portal>) {
        // room A west of x = 6, room B east of it; the portal at (6, 8)
        let rooms = vec![
            Room {
                key: "a",
                lo: v3(int(-10), 0, int(-10)),
                hi: v3(int(6), int(5), int(20)),
                reverb: Reverb::StoreSoft,
                floor: TILE,
            },
            Room {
                key: "b",
                lo: v3(int(6) + 1, 0, int(-10)),
                hi: v3(int(30), int(5), int(20)),
                reverb: Reverb::Corridor,
                floor: TILE,
            },
        ];
        let portals = vec![Portal {
            a: 0,
            b: 1,
            at: v3(int(6), 0, int(8)),
            door,
        }];
        (rooms, portals)
    }

    fn near(a: Fx, b: Fx, within: Fx) -> bool {
        (a - b).abs() <= within
    }

    #[test]
    fn a_sound_round_a_doorway_is_heard_from_the_doorway_at_the_routes_length() {
        // the journal's worked case: S (0, 0), P (6, 8), L (11, 8). |SP| = 10,
        // |PL| = 5, so the route is 15 m and the sound seems to come from
        // L + 15 × unit(P − L) = (−4, 8). The bend at P: cos θ = 0.6, 53.13°.
        let (rooms, portals) = two_rooms(None);
        let h = hear(&rooms, &portals, &[], v3(0, 0, 0), v3(int(11), 0, int(8)), true);
        assert!(
            near(h.distance, int(15), ONE / 50),
            "{}",
            fx::decimal(h.distance, 3)
        );
        assert!(
            near(h.apparent.x, int(-4), ONE / 20),
            "{}",
            fx::decimal(h.apparent.x, 3)
        );
        assert!(near(h.apparent.z, int(8), ONE / 20));
        // gain: 2/15 × (1 − 0.5 × 53.13/90) = 0.1333 × 0.7049 = 0.0940
        assert!(
            near(h.gain, ONE * 940 / 10_000, ONE / 500),
            "{}",
            fx::decimal(h.gain, 4)
        );
        // cut-off: 18000 − 16800 × 53.13/90 = 8082 Hz
        assert!((h.cutoff_hz - 8082).abs() < 40, "{}", h.cutoff_hz);
    }

    #[test]
    fn a_shut_door_lets_through_twelve_percent_and_only_the_lows() {
        let (rooms, portals) = two_rooms(Some(0));
        let open = hear(
            &rooms,
            &portals,
            &[ONE],
            v3(0, 0, 0),
            v3(int(11), 0, int(8)),
            true,
        );
        let shut = hear(&rooms, &portals, &[0], v3(0, 0, 0), v3(int(11), 0, int(8)), true);
        assert!(
            near(shut.gain, fx::mul(open.gain, SHUT_GAIN), ONE / 1000),
            "{} {}",
            shut.gain,
            open.gain
        );
        assert_eq!(shut.cutoff_hz, SHUT_HZ);
    }

    #[test]
    fn a_near_sound_through_a_thin_wall_beats_a_long_way_round() {
        // source 1 m west of the wall, listener 1 m east, the portal 20 m off
        let rooms = vec![
            Room {
                key: "a",
                lo: v3(int(-10), 0, int(-30)),
                hi: v3(0, int(5), int(30)),
                reverb: Reverb::StoreSoft,
                floor: TILE,
            },
            Room {
                key: "b",
                lo: v3(1, 0, int(-30)),
                hi: v3(int(10), int(5), int(30)),
                reverb: Reverb::Corridor,
                floor: TILE,
            },
        ];
        let portals = vec![Portal {
            a: 0,
            b: 1,
            at: v3(0, 0, int(20)),
            door: None,
        }];
        let h = hear(&rooms, &portals, &[], v3(-ONE, 0, 0), v3(ONE, 0, 0), true);
        // through the wall: 2/2 × 0.06 = 0.06; round: about 2/40 × bends < 0.05
        assert_eq!(h.cutoff_hz, WALL_HZ);
        assert!(near(h.gain, WALL_GAIN, ONE / 1000));
    }

    #[test]
    fn in_one_room_a_sound_is_where_it_is_and_falls_off_past_two_metres() {
        let (rooms, portals) = two_rooms(None);
        let h = hear(&rooms, &portals, &[], v3(0, 0, 0), v3(int(-8), 0, 0), false);
        assert_eq!(h.apparent, v3(0, 0, 0));
        // 2 / 8 = 0.25
        assert!(near(h.gain, ONE / 4, ONE / 1000));
        assert_eq!(h.cutoff_hz, OPEN_HZ);
    }
}
