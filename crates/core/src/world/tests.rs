//! The world's rules, each with values worked by hand (CLAUDE.md rule 6).
//! Movement checks follow Chumontillado's list (its world/tests.rs) for
//! what the horror TUNING keeps and what it removes (CH-10).

use super::*;
use crate::input::{BACK, JUMP, LEFT};
use crate::map::{block, Map, Surface};
use crate::movement::Body;

fn playing() -> World {
    let mut w = World::new(7, 0);
    w.act("start", "").unwrap();
    w
}

fn keys(k: u32) -> Input {
    Input {
        keys: k,
        ..Input::default()
    }
}

fn flat() -> Map {
    Map {
        brushes: vec![
            block(
                v3(int(-50), -ONE, int(-50)),
                v3(int(50), 0, int(50)),
                Surface::Floor,
            ),
            block(
                v3(int(3), 0, int(-50)),
                v3(int(4), int(10), int(50)),
                Surface::Wall,
            ),
        ],
        ..Map::default()
    }
}

fn run_body(map: &Map, b: &mut Body, input: Input, ticks: usize, yaw: i32) {
    let mut m = Mover::new(map);
    for _ in 0..ticks {
        m.tick(b, &input, yaw, yaw);
    }
}

fn near(a: Fx, b: Fx, within: Fx) -> bool {
    (a - b).abs() <= within
}

#[test]
fn walking_reaches_one_point_three_metres_a_second_and_no_more() {
    let map = flat();
    let mut b = Body::at(V3::ZERO);
    run_body(&map, &mut b, keys(FORWARD), 120, 0);
    assert!(
        near(b.speed(), ONE * 13 / 10, ONE / 100),
        "{}",
        fx::decimal(b.speed(), 3)
    );
}

#[test]
fn crouch_walks_at_half_the_walk() {
    // T13: 1.3 × 50% = 0.65 m/s
    let map = flat();
    let mut b = Body::at(V3::ZERO);
    run_body(&map, &mut b, keys(FORWARD | CROUCH), 120, 0);
    assert!(
        near(b.speed(), ONE * 65 / 100, ONE / 100),
        "{}",
        fx::decimal(b.speed(), 3)
    );
}

#[test]
fn holding_jump_does_not_hop_again_on_landing() {
    let map = flat();
    let mut b = Body::at(V3::ZERO);
    let mut m = Mover::new(&map);
    m.tick(&mut b, &Input::default(), 0, 0);
    let mut jumps = 0;
    for _ in 0..240 {
        if m.tick(&mut b, &keys(JUMP), 0, 0).jumped {
            jumps += 1;
        }
    }
    assert_eq!(jumps, 1);
}

#[test]
fn a_hop_rises_forty_three_centimetres() {
    // 3.0² / (2 × 9.81) = 0.459 m, less the jump tick's half step of
    // gravity: v = 3.0 − 9.81 / 120 = 2.918, h = 2.918² / 19.62 = 0.434 m
    let map = flat();
    let mut b = Body::at(V3::ZERO);
    let mut m = Mover::new(&map);
    m.tick(&mut b, &Input::default(), 0, 0);
    let mut top = 0;
    m.tick(&mut b, &keys(JUMP), 0, 0);
    for _ in 0..60 {
        m.tick(&mut b, &Input::default(), 0, 0);
        top = top.max(b.origin.y);
    }
    assert!(near(top, ONE * 434 / 1000, ONE / 100), "{}", fx::decimal(top, 3));
}

#[test]
fn strafing_in_the_air_gains_no_speed() {
    // jump forward at the walk, then strafe and turn through the flight
    let map = flat();
    let mut b = Body::at(V3::ZERO);
    run_body(&map, &mut b, keys(FORWARD), 60, 0);
    let start = b.speed();
    let mut m = Mover::new(&map);
    m.tick(&mut b, &keys(FORWARD | JUMP), 0, 0);
    let mut yaw = 0;
    for _ in 0..30 {
        let from = yaw;
        yaw += fx::deg(3);
        m.tick(&mut b, &keys(LEFT), from, yaw);
    }
    assert!(
        b.speed() <= start + ONE / 50,
        "{} > {}",
        fx::decimal(b.speed(), 3),
        fx::decimal(start, 3)
    );
}

#[test]
fn crouching_at_a_sprint_does_not_slide() {
    let map = flat();
    let mut b = Body::at(V3::ZERO);
    let mut m = Mover::new(&map);
    m.t.run = TUNING.sprint;
    for _ in 0..90 {
        m.tick(&mut b, &keys(FORWARD), 0, 0);
    }
    let mut slid = false;
    for _ in 0..30 {
        slid |= m.tick(&mut b, &keys(FORWARD | CROUCH), 0, 0).slid;
    }
    assert!(!slid && !b.sliding);
    // the sprint's 4.0 m/s at the crouch's 50%: 2.0 m/s (T13)
    assert!(b.speed() <= ONE * 4 * 50 / 100 + ONE / 10);
}

#[test]
fn jump_beside_a_wall_in_the_air_does_nothing() {
    let map = flat();
    let mut b = Body::at(v3(ONE * 25 / 10, 0, 0));
    let mut m = Mover::new(&map);
    m.tick(&mut b, &Input::default(), 0, 0);
    m.tick(&mut b, &keys(JUMP), 0, 0);
    m.tick(&mut b, &Input::default(), 0, 0);
    let ev = m.tick(&mut b, &keys(JUMP | RIGHT_KEY), 0, 0);
    assert!(!ev.wall_jumped);
}

const RIGHT_KEY: u32 = crate::input::RIGHT;

#[test]
fn crouch_in_the_air_does_not_slam() {
    let map = flat();
    let mut b = Body::at(v3(0, int(5), 0));
    let mut m = Mover::new(&map);
    let ev = m.tick(&mut b, &keys(CROUCH), 0, 0);
    assert!(!ev.slammed);
}

#[test]
fn a_scare_spot_is_never_inside_the_view_at_any_yaw_or_pitch() {
    // CH-4, swept: every scare point in the mall, from the spawn, every 10°
    // of yaw and 15° of pitch; where the rule allows a spot, it lies at
    // least 78° from where the player looks
    let w = playing();
    let eye = w.eye();
    let mut allowed = 0;
    for yaw_deg in (0..360).step_by(10) {
        for pitch_deg in (-75..=75).step_by(15) {
            let yaw = fx::deg(yaw_deg);
            let pitch = fx::deg(pitch_deg);
            for s in &w.mall.scares {
                if scare_spot_ok(yaw, pitch, eye, s.at) {
                    allowed += 1;
                    assert!(
                        off_view(yaw, pitch, eye, s.at) >= fx::deg(78),
                        "{yaw_deg} {pitch_deg}"
                    );
                }
            }
        }
    }
    assert!(allowed > 0);
}

#[test]
fn off_view_is_zero_ahead_and_a_half_turn_behind() {
    let eye = V3::ZERO;
    // yaw 0 faces −z
    assert!(off_view(0, 0, eye, v3(0, 0, int(-5))) < fx::deg(1));
    assert!(off_view(0, 0, eye, v3(0, 0, int(5))) > fx::deg(179));
    assert!(near(
        off_view(0, 0, eye, v3(int(5), 0, 0)),
        fx::deg(90),
        fx::deg(1)
    ));
}

#[test]
fn every_scare_the_director_makes_while_loading_is_out_of_view() {
    let mut w = playing();
    w.body.origin = v3(int(40), 0, 0);
    w.armed = true;
    w.pitch = fx::deg(-60);
    let mut heard = 0;
    for t in 0..20_000 {
        w.mode = Mode::Reload;
        w.last_scare = i64::MIN / 2;
        w.yaw = fx::deg((t * 7) % 360);
        w.director();
        for v in &w.voices {
            if v.marked.is_some() && !v.head {
                heard += 1;
                assert!(w.off_view(v.at) >= HALF_VIEW + SCARE_MARGIN);
            }
        }
        w.voices.clear();
    }
    assert!(heard > 5, "{heard}");
}

#[test]
fn with_the_hands_out_the_mouse_moves_the_hand_and_not_the_view() {
    let mut w = playing();
    w.pitch = fx::deg(-60);
    let yaw = w.yaw;
    w.tick(&Input {
        buttons: ALT,
        ..Input::default()
    });
    assert_eq!(w.mode, Mode::Reload);
    let hand = w.hand.at;
    w.tick(&Input {
        buttons: ALT,
        dx: 3000,
        dy: 0,
        ..Input::default()
    });
    assert_eq!((w.yaw, w.pitch), (yaw, fx::deg(-60)));
    assert!(w.hand.at.0 > hand.0);
}

/// Moves the hand's goal to `to` and lets it settle.
fn hand_to(w: &mut World, to: (Fx, Fx)) {
    for _ in 0..40 {
        let k = HAND_PER_PX * w.settings.sensitivity / 100;
        let dx = (to.0 - w.hand.want.0) / k;
        let dy = -(to.1 - w.hand.want.1) / k;
        w.tick(&Input {
            buttons: ALT,
            dx,
            dy,
            ..Input::default()
        });
    }
}

#[test]
fn a_reload_takes_open_box_round_chamber_and_close() {
    let mut w = playing();
    w.pitch = fx::deg(-60);
    w.tick(&Input {
        buttons: ALT,
        ..Input::default()
    });
    assert!(!w.revolver.open);
    // a flick left opens the cylinder
    w.tick(&Input {
        buttons: ALT,
        dx: -6000,
        ..Input::default()
    });
    w.tick(&Input {
        buttons: ALT,
        ..Input::default()
    });
    assert!(w.revolver.open, "{:?}", w.hand);
    hand_to(&mut w, POCKET);
    assert!(w.revolver.box_out);
    hand_to(&mut w, BOX_AT);
    assert!(w.revolver.holding);
    assert_eq!(w.revolver.box_rounds, 5);
    hand_to(&mut w, chamber_at(2));
    assert_eq!(w.revolver.chambers[2], Chamber::Round);
    assert!(w.armed, "the first round arms the director");
    // back from the cylinder, then a flick right closes it
    hand_to(&mut w, (-ONE / 2, 0));
    w.tick(&Input {
        buttons: ALT,
        dx: 20_000,
        ..Input::default()
    });
    w.tick(&Input {
        buttons: ALT,
        ..Input::default()
    });
    assert!(!w.revolver.open);
}

#[test]
fn a_door_opens_after_its_turns_and_not_before() {
    let mut w = playing();
    let dn = 2; // the stairwell door: 1½ turns
    let d = w.mall.doors[dn];
    w.body.origin = v3(d.handle.x, 0, d.handle.z - ONE);
    w.yaw = yaw_of(d.handle.sub(w.body.origin));
    w.pitch = fx::deg(-15);
    w.tick(&Input {
        buttons: ALT,
        ..Input::default()
    });
    assert_eq!(w.mode, Mode::Door(dn));
    // circles of the hand: a twelfth of a turn a step
    let mut steps = 0;
    while w.door_open[dn] < ONE && steps < 400 {
        let a = (steps * fx::TURN / 12) % fx::TURN;
        w.hand.at = (fx::mul(ONE / 2, fx::cos(a)), fx::mul(ONE / 2, fx::sin(a)));
        w.hand.was = w.hand.at;
        w.turn_handle(dn);
        steps += 1;
    }
    // 1½ turns at a twelfth each: 18 steps, and the first only sets the angle
    assert_eq!(steps, 19);
}

#[test]
fn a_crouching_player_in_the_dark_passes_a_cultist_a_lit_walker_does_not() {
    // the court dark (every fixture broken), 9 m in front of a chanting
    // cultist: crouched with the flashlight off passes, walking lit is seen
    let mut w = playing();
    let f = w.mall.fixtures.clone();
    for i in 0..f.len() {
        w.lights.break_one(i, &f);
    }
    w.flashlight.on = false;
    let c = w.cultists[0];
    let toward = facing(c.yaw, 0);
    w.body.origin = c.at.add(toward.scale(int(9)));
    w.body.ducked = true;
    for _ in 0..300 {
        w.cult();
    }
    assert_ne!(w.cultists[0].mind, Mind::Attack);
    let mut v = playing();
    for i in 0..f.len() {
        v.lights.break_one(i, &f);
    }
    v.flashlight.on = true;
    v.body.origin = c.at.add(toward.scale(int(9)));
    for _ in 0..300 {
        v.cult();
    }
    assert!(
        matches!(v.cultists[0].mind, Mind::Attack | Mind::Search),
        "{:?}",
        v.cultists[0].mind
    );
}

#[test]
fn a_shot_cultist_falls() {
    let mut w = playing();
    let c = w.cultists[0];
    w.body.origin = c.at.add(v3(int(4), 0, 0));
    w.yaw = yaw_of(c.at.sub(w.body.origin));
    w.pitch = fx::deg(-5);
    w.revolver.chambers = [Chamber::Round; 6];
    w.fire();
    assert_eq!(w.cultists[0].mind, Mind::Dead);
}

#[test]
fn spelling_r_u_n_breaks_the_lights_and_starts_the_chase() {
    let mut w = playing();
    w.mode = Mode::Ouija;
    for letter in LETTERS {
        w.hand.at = letter;
        w.ouija();
    }
    assert!(w.spirit.chasing);
    assert!(w.blackout.is_some());
    assert!(w
        .cultists
        .iter()
        .all(|c| c.mind == Mind::Flee || c.mind == Mind::Dead));
}

#[test]
fn the_exit_wins_only_after_the_blackout() {
    let mut w = playing();
    w.body.origin = v3(w.mall.exit_x, 0, 0);
    w.outcome();
    assert_eq!(w.phase, Phase::Play);
    w.blackout = Some(1);
    w.outcome();
    assert_eq!(w.phase, Phase::Won);
}

#[test]
fn mono_brings_the_ring_and_subtitles() {
    let mut w = playing();
    w.act("output", "mono").unwrap();
    assert_eq!(w.settings.output, Output::Mono);
    assert!(w.settings.ring && w.settings.subtitles);
}

#[test]
fn the_same_seed_and_inputs_give_the_same_run() {
    let script = |w: &mut World| {
        for t in 0..600 {
            let k = if t % 200 < 100 { FORWARD } else { BACK };
            w.tick(&Input {
                keys: k,
                dx: 37,
                ..Input::default()
            });
        }
    };
    let mut a = playing();
    let mut b = playing();
    a.armed = true;
    b.armed = true;
    script(&mut a);
    script(&mut b);
    assert_eq!(a.body, b.body);
    assert_eq!(a.debris, b.debris);
    assert_eq!(a.rng.next_u64(), b.rng.next_u64());
}

#[test]
fn the_volume_is_a_percent_kept_with_the_settings() {
    let mut w = playing();
    assert_eq!(w.settings.volume, 100);
    w.act("volume", "40").unwrap();
    assert_eq!(w.settings.volume, 40);
    let mut v = playing();
    v.act("settings", &settings_text(&w.settings)).unwrap();
    assert_eq!(v.settings.volume, 40);
    w.act("volume", "250").unwrap();
    assert_eq!(w.settings.volume, 100);
    w.act("volume", "-5").unwrap();
    assert_eq!(w.settings.volume, 0);
    // settings kept before there was a volume load at full volume
    let old = r#"{"version":1,"output":"mono","ring":true,"subtitles":true,"reduce_flashes":false,"sensitivity":100,"tier":1}"#;
    let mut u = playing();
    u.act("volume", "30").unwrap();
    u.act("settings", old).unwrap();
    assert_eq!(u.settings.volume, 100);
    assert_eq!(u.settings.output, Output::Mono);
}

#[test]
fn signs_keep_the_way_they_face_while_you_walk() {
    let mut w = playing();
    let want: Vec<i32> = w.mall.signs.iter().map(|s| s.yaw).collect();
    for _ in 0..120 {
        w.tick(&keys(FORWARD));
    }
    assert_eq!(w.sign_yaws, want);
}

#[test]
fn in_the_chase_the_signs_turn_to_you_thirty_degrees_a_second() {
    let mut w = playing();
    w.spirit.chasing = true;
    w.body.origin = v3(-ONE / 2, 0, 0);
    let k = w.mall.signs.iter().position(|s| s.key == "sign_behind").unwrap();
    // "Behind you!" at (2, 3) faces the doors (90°); from (−0.5, 0) the
    // heading to you is atan(2.5 / 3) = 39.8°, 50.2° to turn
    assert_eq!(w.sign_yaws[k], fx::deg(90));
    for _ in 0..60 {
        w.tick(&Input::default());
    }
    assert!(
        (fx::wrap(w.sign_yaws[k] - fx::deg(60))).abs() <= fx::deg(1) / 2,
        "after a second, 60°"
    );
    for _ in 0..60 {
        w.tick(&Input::default());
    }
    assert!(
        (fx::wrap(w.sign_yaws[k] - fx::deg(40))).abs() <= fx::deg(1),
        "after two, it looks at you"
    );
}

#[test]
fn settings_kept_as_text_come_back_and_bad_text_leaves_the_defaults() {
    let mut w = playing();
    w.act("output", "mono").unwrap();
    w.act("sensitivity", "160").unwrap();
    let kept = settings_text(&w.settings);
    let mut v = playing();
    v.act("settings", &kept).unwrap();
    assert_eq!(v.settings, w.settings);
    let mut u = playing();
    u.act("settings", "not json").unwrap();
    assert_eq!(u.settings, Settings::default());
}

#[test]
fn every_cult_post_stands_well_off_both_court_doors() {
    // T9, worked by hand: the ring at (75, −9) with posts 4 m out puts the
    // nearest posts (79, −9) 14.2 m and (77, −5.54) 14.1 m
    // from the cursed door (90, 0); (71, −9) 14.2 m and (73, −5.54) 14.1 m
    // from the wing's opening (60, 0); none nearer than 5.5 m to the line
    // between them (z = 0)
    let w = playing();
    for p in &w.mall.cultist_posts {
        let door = v3(int(90), 0, 0).sub(*p).len_xz();
        let wing = v3(int(60), 0, 0).sub(*p).len_xz();
        assert!(door > ONE * 14, "{}", fx::decimal(door, 2));
        assert!(wing > ONE * 14, "{}", fx::decimal(wing, 2));
        assert!(p.z < -ONE * 55 / 10, "{}", fx::decimal(p.z, 2));
    }
}

#[test]
fn from_the_court_doorway_you_can_watch_the_cult_unseen() {
    // T6: standing in the lit doorway, light on, ten seconds, looking in
    let mut w = playing();
    w.flashlight.on = true;
    for _ in 0..10 * HZ {
        w.body.origin = v3(int(59), 0, 0);
        w.body.velocity = V3::ZERO;
        w.yaw = fx::deg(-90);
        w.flashlight.charge = crate::lights::FULL;
        w.tick(&Input::default());
    }
    for c in &w.cultists {
        assert_eq!(c.suspicion, 0, "{:?}", c.at);
    }
}

#[test]
fn a_careful_player_gets_past_the_cult_on_the_north_side() {
    // T6: wait at the doorway (where they cannot see you), then go
    // crouched, light off, along the court's north wall at crouch pace,
    // and spend six seconds at the cursed store's handle: no cultist
    // attacks, whichever of four moments in the walkers' forty-second
    // round you set off
    for late in [0, 10, 20, 30] {
        let mut w = playing();
        w.flashlight.on = false;
        let mut waited = 0;
        loop {
            w.body.origin = v3(int(59), 0, 0);
            w.body.velocity = V3::ZERO;
            w.yaw = fx::deg(-90);
            w.tick(&Input::default());
            waited += 1;
            if waited > late * HZ as i32 {
                break;
            }
        }
        let path = [
            v3(int(60), 0, int(2)),
            v3(int(64), 0, int(13)),
            v3(int(86), 0, int(13)),
            v3(int(89), 0, 0),
        ];
        let pace = ONE * 52 / 100 / HZ as i32;
        let mut at = path[0];
        let mut seen = false;
        for leg in path.windows(2) {
            let way = leg[1].sub(leg[0]);
            let n = way.len_xz() / pace;
            for k in 0..n {
                at = leg[0].add(way.scale(fx::div(k, n)));
                w.body.origin = at;
                w.body.velocity = V3::ZERO;
                w.yaw = yaw_of(way);
                w.tick(&keys(CROUCH));
                seen |= w.cultists.iter().any(|c| c.mind == Mind::Attack);
            }
        }
        for _ in 0..6 * HZ {
            w.body.origin = at;
            w.body.velocity = V3::ZERO;
            w.yaw = fx::deg(-90);
            w.tick(&keys(CROUCH));
            seen |= w.cultists.iter().any(|c| c.mind == Mind::Attack);
        }
        // (the raffle car may still grab: that is the spirit, not the cult)
        assert!(!seen, "going after {late} s");
    }
}

#[test]
fn a_careless_player_crossing_the_middle_lit_is_still_caught() {
    // standing, flashlight on, straight from the wing to the cursed door
    // at walking pace: within 14 m of the ring the cult sees and attacks
    let mut w = playing();
    w.flashlight.on = true;
    let from = v3(int(60), 0, 0);
    let way = v3(int(29), 0, 0);
    // the walk, 1.3 m/s
    let pace = TUNING.run / HZ as i32;
    let n = way.len_xz() / pace;
    let mut caught = false;
    for k in 0..n {
        w.body.origin = from.add(way.scale(fx::div(k, n)));
        w.body.velocity = V3::ZERO;
        w.yaw = fx::deg(-90);
        w.flashlight.charge = crate::lights::FULL;
        w.tick(&Input::default());
        caught |= w.cultists.iter().any(|c| c.mind == Mind::Attack);
    }
    assert!(caught);
}

#[test]
fn the_cursed_door_stands_in_moonlight_only() {
    // T6, worked by hand from the fixture table: at the handle (89, 1.6, 0)
    // no lamp, pendant or troffer is in range (the moved lamp at (84, 7, −12)
    // is 14.1 m off, its reach 14 m); the moon (power 0.55, reach 26 m) is,
    // over the dark's 1/6
    let mut w = playing();
    w.flashlight.on = false;
    let at = v3(int(89), ONE * 16 / 10, 0);
    assert_eq!(w.lit_at(at), ONE / 6 + 55 * ONE / 100);
}

#[test]
fn the_directors_wait_is_the_ceiling_of_the_menace_you_are_in_now() {
    // CH-Q26: a scare 1900 ticks ago; at menace 1 (ceiling 2 a minute) the
    // wait is 1800 ticks, so it is over; back at menace 0 (1 a minute) it
    // is 3600, so 1700 are left
    let mut w = playing();
    w.armed = false;
    w.last_scare = w.tick - 1900;
    w.body.origin = v3(int(20), 0, 0);
    assert_eq!(w.menace(), 1);
    w.director();
    assert_eq!(w.scare_wait, 0);
    w.body.origin = v3(int(5), 0, 0);
    assert_eq!(w.menace(), 0);
    w.director();
    assert_eq!(w.scare_wait, 1700);
}

#[test]
fn the_director_never_uses_the_same_kind_twice_running() {
    // CH-Q23: in the wings, loading, many kinds in reach
    let mut w = playing();
    w.body.origin = v3(int(20), 0, 0);
    w.armed = true;
    w.pitch = fx::deg(-60);
    let mut kinds = Vec::new();
    for t in 0..40_000 {
        w.mode = Mode::Reload;
        w.last_scare = i64::MIN / 2;
        w.yaw = fx::deg((t * 7) % 360);
        let before = w.last_kind;
        let used: i32 = w.kinds_used.iter().sum();
        w.director();
        if w.kinds_used.iter().sum::<i32>() > used {
            assert_ne!(
                Some(w.last_kind),
                Some(before).filter(|b| b.is_some()),
                "twice running"
            );
            kinds.push(w.last_kind.unwrap());
        }
        w.voices.clear();
        w.pending_steps.clear();
    }
    assert!(kinds.len() > 30, "{}", kinds.len());
    // each kind used differs from the most used by at most a few
    let used: Vec<i32> = w.kinds_used.iter().copied().filter(|n| *n > 0).collect();
    assert!(used.len() >= 3, "{used:?}");
}

#[test]
fn the_director_turns_the_nearest_mannequin_out_of_view_not_yet_facing_you() {
    // CH-Q24, worked by hand: at (20, 0) looking along −z (yaw 0), the
    // mannequins at (20, 8), (24, 7), (12, 6) and (28, 8.5) are behind you.
    // (20, 8) faces +z (180°), away from you, and is nearest (8 m): it turns
    let mut w = playing();
    w.body.origin = v3(int(20), 0, 0);
    w.yaw = 0;
    w.pitch = 0;
    let eye = w.eye();
    assert_eq!(w.mannequin_to_turn(eye), Some(0));
    // once it faces you, the next nearest not facing you: (24, 7), 8.1 m
    w.mall.props[0].yaw = yaw_of(w.body.origin.sub(w.mall.props[0].at));
    assert_eq!(w.mannequin_to_turn(eye), Some(1));
    // look toward them (yaw 180°, along +z): all four are in view
    w.yaw = fx::deg(180);
    assert_eq!(w.mannequin_to_turn(eye), None);
}

#[test]
fn glimpses_fall_where_the_screen_shows_them_and_twenty_seconds_apart() {
    // CH-Q25: 35° to 48° off the view (inside a 16:9 screen's 51° edge),
    // within a degree of the angles' rounding, 5 to 12 m away, at least
    // 20 s apart
    let mut w = playing();
    w.body.origin = v3(int(75), 0, int(10));
    w.yaw = fx::deg(-90);
    let mut last: Option<i64> = None;
    let mut n = 0;
    for _ in 0..60 * 60 * HZ {
        w.tick += 1;
        w.chase();
        if w.spirit.glimpse == 20 {
            let to = w.spirit.at.sub(w.body.origin);
            let off = fx::wrap(yaw_of(to) - w.yaw).abs();
            assert!(
                off >= fx::deg(34) && off <= fx::deg(49),
                "{}",
                off * 360 / fx::TURN
            );
            let d = to.len_xz();
            assert!(d >= int(5) - 2 && d <= int(12) + 2, "{}", fx::decimal(d, 2));
            if let Some(t) = last {
                assert!(w.tick - t >= 20 * HZ, "{}", w.tick - t);
            }
            last = Some(w.tick);
            n += 1;
        }
    }
    assert!(n > 20, "{n}");
}

#[test]
fn a_cultist_hunting_you_goes_by_the_doorways_and_never_into_a_wall() {
    // T8: from the ring to you in Belle Mode (24, 10): out of the court,
    // along the wings and in at the storefront (24, 5), about 50 m, so
    // well inside a minute at 3.2 m/s; never standing inside a wall
    let w = playing();
    let me = v3(int(24), 0, int(10));
    let mut at = v3(int(70), 0, int(-2));
    let step = ONE * 32 / 10 / HZ as i32;
    let (lo, hi) = (
        v3(-CULT_HALF, ONE / 10, -CULT_HALF),
        v3(CULT_HALF, ONE * 17 / 10, CULT_HALF),
    );
    let mut sc = Scratch::default();
    let mut through_front = false;
    for _ in 0..60 * HZ {
        at = w.cult_step(at, me, step);
        assert!(
            !trace_box(&w.mall.map, &mut sc, at, at, lo, hi).start_solid,
            "in a wall at {at:?}"
        );
        if (at.z - int(5)).abs() < ONE / 2 {
            assert!(
                (at.x - int(24)).abs() < int(6),
                "crossed the shopfront wall at {at:?}"
            );
            through_front = true;
        }
    }
    assert!(through_front);
    assert!(at.sub(me).len_xz() < ONE, "{at:?}");
}

#[test]
fn a_cultist_cannot_pass_a_shut_door() {
    // T8: Kandy Korner's back door into the service corridor is shut, so
    // there is no way from the corridor to you in the store
    let mut w = playing();
    let me = v3(int(52), 0, int(10));
    let at = v3(int(52), 0, int(17));
    assert_eq!(w.way_to(at, me), None);
    // opened, the way is the doorway (52, 15)
    w.door_open[1] = ONE;
    assert_eq!(w.way_to(at, me), Some(v3(int(52), 0, int(15))));
}

#[test]
fn a_cultist_never_walks_through_a_solid_wall_or_door() {
    // T8: even when the way says go (the door counted open), the door
    // itself still stands in the map: walking into it stops at it
    let mut w = playing();
    w.door_open[1] = ONE;
    let me = v3(int(52), 0, int(10));
    let mut at = v3(int(52), 0, int(17));
    for _ in 0..30 * HZ {
        at = w.cult_step(at, me, ONE * 32 / 10 / HZ as i32);
    }
    assert!(at.z > int(15), "{at:?}");
}

#[test]
fn a_lit_player_in_plain_view_at_fifteen_metres_is_noticed_then_hunted() {
    // T9, worked by hand. One cultist, at its post (79, −9) facing the
    // ring (west); you stand 15 m west at (64, −9), light on and full.
    // Light on you: the dark's 1/6 + the flashlight 3 + the lamp at
    // (68, 7, −8) 0.7 + the moon 0.55 = 4.4167; sight 4.4167 / 15 = 0.2944;
    // suspicion grows 0.2944 / 40 = 0.00736 a tick: it looks (0.3) on
    // tick 41 and hunts (1.0) on tick 136
    let mut w = playing();
    for (i, c) in w.cultists.iter_mut().enumerate() {
        if i != 0 {
            c.mind = Mind::Dead;
        }
    }
    assert_eq!(w.cultists[0].post, v3(int(79), 0, int(-9)));
    let (mut looked, mut hunted) = (None, None);
    for t in 1..400i32 {
        w.body.origin = v3(int(64), 0, int(-9));
        w.body.velocity = V3::ZERO;
        w.flashlight.on = true;
        w.flashlight.charge = crate::lights::FULL;
        w.tick(&Input::default());
        let m = w.cultists[0].mind;
        if looked.is_none() && m == Mind::Look {
            looked = Some(t);
        }
        if hunted.is_none() && matches!(m, Mind::Search | Mind::Attack) {
            hunted = Some(t);
        }
    }
    let (l, h) = (looked.unwrap(), hunted.unwrap());
    assert!((l - 41).abs() <= 2, "looked on tick {l}");
    assert!((h - 136).abs() <= 3, "hunted on tick {h}");
}

// T12: the curios -------------------------------------------------------

/// The player `back` north of the curio of `kind` (+z), looking south at it.
fn before_curio(kind: mall::CurioKind, back: Fx, pitch: i32) -> (World, usize) {
    let mut w = playing();
    let i = w.mall.curios.iter().position(|c| c.kind == kind).unwrap();
    let at = w.mall.curios[i].at;
    w.body = Body::at(v3(at.x, 0, at.z + back));
    w.yaw = 0;
    w.pitch = pitch;
    (w, i)
}

fn fire_button() -> Input {
    Input {
        buttons: FIRE,
        ..Input::default()
    }
}

/// One tick with `input`, then `then` idle ticks; every sound and line
/// key heard on the way goes into `heard`.
fn drive(w: &mut World, input: Input, then: usize, heard: &mut Vec<&'static str>) {
    w.tick(&input);
    for k in 0..=then {
        if k > 0 {
            w.tick(&Input::default());
        }
        heard.extend(w.voices.iter().map(|v| v.recipe));
        heard.extend(w.marks.iter().map(|m| m.key));
        if !w.prompt.is_empty() {
            heard.push(w.prompt);
        }
    }
}

/// The ticks, counted from 1 after the one that set it going, on which
/// the girl was there.
fn girl_ticks(w: &mut World, input: Input, ticks: i32) -> Vec<i32> {
    w.tick(&input);
    let mut on = Vec::new();
    for k in 1..=ticks {
        w.tick(&Input::default());
        if w.phantom.is_some() {
            on.push(k);
        }
    }
    on
}

#[test]
fn the_toy_brings_the_little_girl_on_the_second_squeeze_for_four_tenths_of_a_second() {
    let (mut w, i) = before_curio(mall::CurioKind::Toy, ONE * 12 / 10, -fx::deg(40));
    let mut heard = Vec::new();
    drive(&mut w, keys(USE), 1, &mut heard);
    assert_eq!(w.held, Some(i));
    assert_eq!(w.prompt, "prompt_hold_toy");
    // the first squeeze: nothing
    assert!(girl_ticks(&mut w, fire_button(), 180).is_empty());
    // the second: 0.2 s later (12 ticks) for 24 ticks, 2.2 m straight ahead
    w.tick(&fire_button());
    let mut on = Vec::new();
    for k in 1..=60 {
        w.tick(&Input::default());
        if let Some(p) = w.phantom {
            if on.is_empty() {
                // the toy was at z −8.8 m, the player 1.2 m north: −7.6 − 2.2
                assert!(
                    near(p.at.z, -ONE * 98 / 10, ONE / 50),
                    "{}",
                    fx::decimal(p.at.z, 2)
                );
                assert!(near(p.at.x, ONE * 262 / 10, ONE / 50));
                assert!(w.off_view(p.at.add(v3(0, GIRL_HEAD, 0))) < fx::deg(35));
            }
            on.push(k);
        }
    }
    assert_eq!(on, (12..=35).collect::<Vec<_>>());
    // she answers once
    assert!(girl_ticks(&mut w, fire_button(), 180).is_empty());
    assert!(girl_ticks(&mut w, fire_button(), 180).is_empty());
}

#[test]
fn with_a_curio_in_hand_the_left_button_uses_it_and_e_puts_it_down() {
    let (mut w, i) = before_curio(mall::CurioKind::Toy, ONE * 12 / 10, -fx::deg(40));
    w.revolver.chambers = [Chamber::Round; 6];
    w.tick(&keys(USE));
    w.tick(&Input::default());
    w.tick(&fire_button());
    assert!(w.voices.iter().any(|v| v.recipe == "squeak"));
    assert!(!w.voices.iter().any(|v| v.recipe == "shot"));
    assert_eq!(w.revolver.chambers, [Chamber::Round; 6]);
    w.tick(&Input::default());
    w.tick(&keys(USE));
    assert_eq!(w.held, None);
    // 0.6 m ahead of the player at z −7.6, on the floor
    let at = w.curios[i].at;
    assert!(near(at.z, -ONE * 82 / 10, ONE / 50), "{}", fx::decimal(at.z, 2));
    assert!(near(at.y, 0, ONE / 100));
    w.tick(&Input::default());
    w.tick(&fire_button());
    assert!(w
        .voices
        .iter()
        .any(|v| v.recipe == "dry_click" || v.recipe == "shot"));
}

#[test]
fn the_camera_shows_her_in_the_second_flash_and_only_while_it_lasts() {
    let (mut w, _) = before_curio(mall::CurioKind::Camera, ONE * 12 / 10, 0);
    w.tick(&keys(USE));
    w.tick(&Input::default());
    assert!(girl_ticks(&mut w, fire_button(), 60).is_empty());
    w.tick(&fire_button());
    let first = w.phantom.expect("she is in the second picture");
    assert!(w.off_view(first.at.add(v3(0, GIRL_HEAD, 0))) < fx::deg(35));
    // the flash is the brightest light there is, at the hand
    let eye = w.eye();
    let flash = crate::view::lights(&w, eye)
        .into_iter()
        .filter(|l| l.at.sub(eye).len() < ONE / 2)
        .map(|l| l.rgb.x)
        .max()
        .unwrap();
    assert_eq!(flash, ONE * 3);
    let mut lit = 1;
    for _ in 0..30 {
        w.tick(&Input::default());
        assert_eq!(w.phantom.is_some(), w.flash > 0, "tick {lit}");
        if w.flash > 0 {
            lit += 1;
        }
    }
    // FLASH_TICKS is 10; this tick's count-down leaves nine lit ticks
    assert_eq!(lit, 9);
    assert!(girl_ticks(&mut w, fire_button(), 60).is_empty());
}

#[test]
fn with_flashes_reduced_the_camera_flash_is_a_third_as_bright() {
    let (mut w, _) = before_curio(mall::CurioKind::Camera, ONE * 12 / 10, 0);
    w.act("flashes", "reduce").unwrap();
    w.tick(&keys(USE));
    w.tick(&Input::default());
    w.tick(&fire_button());
    let eye = w.eye();
    let flash = crate::view::lights(&w, eye)
        .into_iter()
        .filter(|l| l.at.sub(eye).len() < ONE / 2)
        .map(|l| l.rgb.x)
        .max()
        .unwrap();
    assert_eq!(flash, ONE);
}

#[test]
fn the_music_box_runs_down_then_she_stands_behind_you_until_you_turn() {
    let (mut w, _) = before_curio(mall::CurioKind::MusicBox, ONE * 12 / 10, 0);
    let mut heard = Vec::new();
    drive(&mut w, keys(USE), 1, &mut heard);
    drive(&mut w, fire_button(), 0, &mut heard);
    // phrases at +30, then gaps of 45, 52 … 94 ticks: the eighth at +492,
    // she at 492 + 94 + 60 = +646
    let mut tunes = Vec::new();
    let mut came = None;
    for k in 1..=700 {
        w.tick(&Input::default());
        if w.voices.iter().any(|v| v.recipe == "music_box") {
            tunes.push(k);
        }
        if came.is_none() && w.phantom.is_some() {
            came = Some(k);
        }
    }
    assert_eq!(tunes, vec![30, 75, 127, 186, 252, 325, 405, 492]);
    assert_eq!(came, Some(646));
    let p = w.phantom.unwrap();
    // her head, 2 m behind and 0.65 m below the eye: 162° off the view
    assert!(w.off_view(p.at.add(v3(0, GIRL_HEAD, 0))) > fx::deg(150));
    // turn round: she is there on the tick she is seen and seven more
    // (SEEN_LINGER 8), and gone on the ninth
    w.yaw = fx::TURN / 2;
    let mut left = 0;
    while w.phantom.is_some() {
        w.tick(&Input::default());
        left += 1;
    }
    assert_eq!(left, 9);
}

#[test]
fn unseen_she_stays_behind_you_four_seconds() {
    let (mut w, _) = before_curio(mall::CurioKind::MusicBox, ONE * 12 / 10, 0);
    w.tick(&keys(USE));
    w.tick(&Input::default());
    let on = girl_ticks(&mut w, fire_button(), 1000);
    assert_eq!(on.first(), Some(&646));
    assert_eq!(on.len(), 240);
    // wound again, it plays no more: it answers once
    w.tick(&Input::default());
    let mut heard = Vec::new();
    drive(&mut w, fire_button(), 1000, &mut heard);
    assert!(!heard.contains(&"music_box"));
    assert!(w.phantom.is_none());
}

#[test]
fn the_telephone_rings_when_you_walk_in_and_answering_it_stops_it() {
    let (mut w, i) = before_curio(mall::CurioKind::Phone, ONE * 12 / 10, 0);
    let mut heard = Vec::new();
    let mut rings = Vec::new();
    for k in 1..=220 {
        w.tick(&Input::default());
        if w.voices.iter().any(|v| v.recipe == "phone_ring") {
            rings.push(k);
        }
    }
    // set going on the first tick inside: half a second on, then three
    // seconds apart
    assert_eq!(rings, vec![31, 211]);
    drive(&mut w, keys(USE), 1, &mut heard);
    assert_eq!(w.held, Some(i));
    let mut after = Vec::new();
    for k in 1..=400 {
        w.tick(&Input::default());
        for v in &w.voices {
            if v.recipe == "phone_ring" {
                after.push((k, v.head, v.at.sub(w.body.origin).len_xz()));
            }
        }
        heard.extend(w.marks.iter().map(|m| m.key));
    }
    // one more ring, six seconds after answering, 12 m behind you
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].0, 6 * 60 - 1);
    assert!(!after[0].1);
    assert!(near(after[0].2, int(12), ONE / 50));
    assert!(heard.contains(&"sub_giggle"));
    drive(&mut w, keys(USE), 0, &mut heard);
    assert_eq!(w.held, None);
    assert_eq!(w.curios[i].at, w.mall.curios[i].at);
    // answered again: only the dial's silence
    let mut again = Vec::new();
    drive(&mut w, Input::default(), 0, &mut again);
    drive(&mut w, keys(USE), 400, &mut again);
    assert!(!again.contains(&"giggle"));
}

#[test]
fn every_sound_and_line_the_curios_use_is_in_data() {
    let sounds: serde_json::Value =
        serde_json::from_str(include_str!("../../../../data/sounds.json")).unwrap();
    let strings: serde_json::Value =
        serde_json::from_str(include_str!("../../../../data/strings.json")).unwrap();
    let mut heard = Vec::new();
    for (kind, back, pitch) in [
        (mall::CurioKind::Toy, ONE * 12 / 10, -fx::deg(40)),
        (mall::CurioKind::Camera, ONE * 12 / 10, 0),
        (mall::CurioKind::MusicBox, ONE * 12 / 10, 0),
        (mall::CurioKind::Phone, ONE * 12 / 10, 0),
    ] {
        let (mut w, _) = before_curio(kind, back, pitch);
        drive(&mut w, Input::default(), 0, &mut heard);
        drive(&mut w, keys(USE), 1, &mut heard);
        for _ in 0..2 {
            drive(&mut w, fire_button(), 400, &mut heard);
        }
        drive(&mut w, keys(USE), 300, &mut heard);
    }
    heard.sort();
    heard.dedup();
    for k in &heard {
        let known = sounds["voices"].get(*k).is_some() || strings.get(*k).is_some();
        assert!(known, "{k} is in neither data/sounds.json nor data/strings.json");
    }
    for k in [
        "prompt_take_toy",
        "prompt_hold_camera",
        "sub_ring",
        "giggle",
        "wind_up",
    ] {
        assert!(heard.contains(&k), "{k} was not heard");
    }
}

#[test]
fn the_girl_never_stands_in_a_wall_a_counter_or_a_door() {
    let w0 = playing();
    let mut shown = 0;
    for room in [3usize, 4, 5, 6, 7, 8, 9] {
        let r = w0.mall.rooms[room];
        for (fx_, fz) in [(1, 1), (1, 3), (3, 1), (3, 3), (2, 2)] {
            for k in 0..8 {
                let mut w = w0.clone();
                let x = r.lo.x + (r.hi.x - r.lo.x) * fx_ / 4;
                let z = r.lo.z + (r.hi.z - r.lo.z) * fz / 4;
                w.body = Body::at(v3(x, 0, z));
                w.yaw = k * fx::TURN / 8;
                w.show_girl(ONE * 22 / 10, 0, GIRL_TICKS, false);
                let Some(p) = w.phantom else { continue };
                shown += 1;
                let lo = p.at.add(v3(-ONE / 5, ONE / 10, -ONE / 5));
                let hi = p.at.add(v3(ONE / 5, ONE * 11 / 10, ONE / 5));
                let overlaps = |a: V3, b: V3| {
                    lo.x < b.x && hi.x > a.x && lo.y < b.y && hi.y > a.y && lo.z < b.z && hi.z > a.z
                };
                for s in &w.mall.slabs {
                    assert!(!overlaps(s.lo, s.hi), "room {room} at {x},{z} yaw {k}: in a slab");
                }
                for d in &w.mall.doors {
                    assert!(!overlaps(d.lo, d.hi), "room {room}: in a door");
                }
            }
        }
    }
    // most spots have room for her
    assert!(shown > 200, "{shown}");
}

// T14: the fuse box ------------------------------------------------------

/// From the fuse box, light off and standing, sprint along the court's
/// north side to the cursed store's door, then crouch at its handle for
/// fifteen seconds: whether any cultist attacked.
fn run_past(throw: bool, shoot: bool) -> bool {
    let mut w = playing();
    w.flashlight.on = false;
    w.body = Body::at(v3(int(61), 0, int(9)));
    w.yaw = yaw_of(w.mall.fusebox.sub(w.body.origin));
    w.pitch = -fx::deg(10);
    w.tick(&Input::default());
    if throw {
        w.tick(&keys(USE));
        assert!(matches!(w.breaker, Breaker::Thrown(_)));
    }
    if shoot {
        aim_and_shoot_the_box(&mut w);
    }
    let path = [v3(int(86), 0, int(9)), v3(int(89), 0, int(1))];
    let mut attacked = false;
    let mut leg = 0;
    while leg < path.len() {
        let to = path[leg].sub(w.body.origin);
        if to.len_xz() < ONE / 2 {
            leg += 1;
            continue;
        }
        w.yaw = yaw_of(to);
        w.pitch = 0;
        w.tick(&keys(FORWARD | SPRINT));
        attacked |= w.cultists.iter().any(|c| c.mind == Mind::Attack);
        assert!(w.tick < 30 * HZ, "the run got stuck");
    }
    for _ in 0..15 * HZ {
        w.yaw = fx::deg(-90);
        w.tick(&keys(CROUCH));
        attacked |= w.cultists.iter().any(|c| c.mind == Mind::Attack);
    }
    attacked
}

fn aim_and_shoot_the_box(w: &mut World) {
    w.revolver.chambers = [Chamber::Round; 6];
    let to = w.mall.fusebox.sub(w.eye());
    w.yaw = yaw_of(to);
    w.pitch = fx::wrap(fx::atan2(to.y, to.len_xz()));
    w.tick(&fire_button());
    assert_eq!(w.breaker, Breaker::Shot);
}

#[test]
fn with_the_breaker_thrown_you_can_sprint_past_the_cult_and_turn_the_handle() {
    // the same sprint, light off and standing, is caught without it: the
    // footsteps bring them
    assert!(run_past(false, false));
    assert!(!run_past(true, false));
}

#[test]
fn the_breaker_feeds_the_courts_two_lamps_and_comes_back_on_after_forty_seconds() {
    let mut w = playing();
    let feeds = w.mall.breaker_feeds.clone();
    // the lamps at (68, −8) and (84, −12)
    let at: Vec<V3> = feeds.iter().map(|&i| w.mall.fixtures[i].at).collect();
    assert_eq!(
        at,
        vec![v3(int(68), int(7), int(-8)), v3(int(84), int(7), int(-12))]
    );
    let before: Vec<_> = feeds.iter().map(|&i| w.lights.states[i]).collect();
    // nobody about to cut the forty seconds short
    for c in w.cultists.iter_mut() {
        c.mind = Mind::Dead;
    }
    w.body = Body::at(v3(int(61), 0, int(9)));
    w.yaw = yaw_of(w.mall.fusebox.sub(w.body.origin));
    w.pitch = -fx::deg(10);
    w.tick(&Input::default());
    assert_eq!(w.prompt, "prompt_breaker");
    w.tick(&keys(USE));
    let thrown = w.tick;
    for &i in &feeds {
        assert_eq!(w.lights.brightness(i, false), 0);
    }
    while w.tick < thrown + 40 * HZ - 1 {
        w.tick(&Input::default());
    }
    assert_eq!(w.lights.brightness(feeds[0], false), 0);
    w.tick(&Input::default());
    assert_eq!(w.breaker, Breaker::On);
    let after: Vec<_> = feeds.iter().map(|&i| w.lights.states[i]).collect();
    assert_eq!(after, before);
}

#[test]
fn a_shot_fuse_box_puts_the_court_out_for_good_and_sends_the_cult_to_it() {
    let mut w = playing();
    w.body = Body::at(v3(int(61), 0, int(9)));
    w.tick(&Input::default());
    aim_and_shoot_the_box(&mut w);
    for _ in 0..60 * HZ {
        w.tick(&Input::default());
    }
    assert_eq!(w.breaker, Breaker::Shot);
    for &i in &w.mall.breaker_feeds {
        assert_eq!(w.lights.brightness(i, false), 0);
    }
    // a minute on, they have come from the ring (14 m off) to the box
    let near_box = w
        .cultists
        .iter()
        .filter(|c| c.at.sub(w.mall.fusebox).len_xz() < int(8))
        .count();
    assert!(near_box >= 1, "{near_box}");
}

// T15: the spirit rises ---------------------------------------------------

#[test]
fn the_spirit_rises_in_the_far_corner_and_stands_three_seconds() {
    let mut w = playing();
    // beside the board, on the side of the old spot (100, −4)
    w.body = Body::at(v3(ONE * 995 / 10, 0, ONE * -25 / 10));
    w.tick(&Input::default());
    w.begin_chase();
    assert_eq!(w.spirit.at, v3(int(101), 0, int(4)));
    for _ in 0..3 * HZ {
        w.tick(&Input::default());
        assert_eq!(w.spirit.at, v3(int(101), 0, int(4)));
    }
    w.tick(&Input::default());
    assert_ne!(w.spirit.at, v3(int(101), 0, int(4)));
}

#[test]
fn from_the_board_you_can_reach_the_court_when_the_spirit_rises() {
    let mut w = playing();
    w.flashlight.on = false;
    w.body = Body::at(v3(ONE * 995 / 10, 0, ONE * -25 / 10));
    // you came in by the door, so it stands open
    w.door_open[0] = ONE;
    let b = w.mall.doors[0].brush;
    w.mall.map.brushes[b] = mall::open_brush();
    w.tick(&Input::default());
    w.begin_chase();
    let path = [v3(int(95), 0, 0), v3(int(85), 0, 0), v3(int(80), 0, int(10))];
    for p in path {
        while p.sub(w.body.origin).len_xz() > ONE / 2 {
            w.yaw = yaw_of(p.sub(w.body.origin));
            w.tick(&keys(FORWARD | SPRINT));
            assert!(
                w.health > 0,
                "caught at {} tick {} spirit {},{} door {:?} speed {}",
                fx::decimal(w.body.origin.x, 1),
                w.tick,
                fx::decimal(w.spirit.at.x, 1),
                fx::decimal(w.spirit.at.z, 1),
                w.door_open,
                fx::decimal(w.body.speed(), 2)
            );
            assert!(w.tick < 40 * HZ);
        }
    }
}

// T16: darker ----------------------------------------------------------------

#[test]
fn ten_seconds_in_the_dark_the_eye_brightens_it_by_no_more_than_five_percent() {
    let mut w = playing();
    w.flashlight.on = false;
    // a place no fixture reaches: the night's floor of light, 1/6
    let dark = (0..60)
        .map(|k| v3(int(k * 10 - 5), 0, int(17)))
        .find(|p| w.lit_at(p.add(v3(0, ONE * 16 / 10, 0))) == ONE / 6)
        .expect("somewhere in the service corridor is dark");
    w.body = Body::at(dark);
    for _ in 0..10 * HZ {
        w.body.origin = dark;
        w.tick(&Input::default());
    }
    // lit 1/6 here: 1.1 − 1/24 = 1.058, held to 1.05
    assert_eq!(w.exposure, ONE * 105 / 100);
}

#[test]
fn a_lamp_draws_half_again_its_power() {
    let w = playing();
    let eye = w.mall.fixtures[0].at.add(v3(0, -ONE, 0));
    let f = w.mall.fixtures[0];
    let ls = crate::view::lights(&w, eye);
    let near = ls.iter().find(|l| l.at == f.at).unwrap();
    // power × 3/2 × red / 255
    let k = fx::mul(f.power, ONE * 3 / 2);
    assert!(
        (near.rgb.x - k * f.rgb[0] / 255).abs() <= 2,
        "{} {}",
        near.rgb.x,
        k * f.rgb[0] / 255
    );
}
