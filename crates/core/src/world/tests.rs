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
fn crouch_walks_at_forty_percent() {
    // 1.3 × 40% = 0.52 m/s
    let map = flat();
    let mut b = Body::at(V3::ZERO);
    run_body(&map, &mut b, keys(FORWARD | CROUCH), 120, 0);
    assert!(
        near(b.speed(), ONE * 52 / 100, ONE / 100),
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
    assert!(b.speed() <= ONE * 52 / 100 * 4 / 13 * 10 + ONE / 10);
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
        w.scare_wait = 0;
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
fn every_cult_post_stands_well_off_the_cursed_door() {
    // T6, worked by hand: the ring at (70, −8) with posts 4 m out puts the
    // nearest posts 17.9 m ((74, −8): √(16² + 8²)) and 18.6 m ((72, −4.54))
    // from the cursed store's door (90, 0), and none nearer than 4.5 m to
    // the line from the wing to that door (z = 0)
    let w = playing();
    for p in &w.mall.cultist_posts {
        let door = v3(int(90), 0, 0).sub(*p).len_xz();
        assert!(door > ONE * 175 / 10, "{}", fx::decimal(door, 2));
        assert!(p.z < -ONE * 45 / 10, "{}", fx::decimal(p.z, 2));
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
