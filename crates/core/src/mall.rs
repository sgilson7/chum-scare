//! The mall (journal tab 2, CH-Q11 A): one route from the entrance through
//! the east wing to the centre court and Hollow & Daughters, with the
//! service corridor as a way back. Everything here is data: the brushes a
//! body collides with, the boxes the page draws, the rooms and portals sound
//! travels through (`acoustics.rs`), the doors, the light fixtures
//! (`lights.rs`), the props, the hiding places and the scare points.
//!
//! X runs east along the wing, Z north, Y up; metres in Q16.16.

use crate::fx::{int, Fx, ONE};
use crate::geom::{v3, V3};
use crate::map::{block, Brush, Map, Surface};

/// What a drawn box is made of: its row in `data/look.json`'s materials.
pub const TILE: i32 = 0;
pub const PLASTER: i32 = 1;
pub const CARPET: i32 = 2;
pub const METAL: i32 = 3;
pub const WOOD: i32 = 4;
pub const GLASS: i32 = 5;
pub const CEILING: i32 = 6;
pub const CONCRETE: i32 = 7;
pub const FABRIC: i32 = 8;
pub const PLASTIC: i32 = 9;
pub const PAINT: i32 = 10;
pub const SKIN: i32 = 11;
pub const GLOW: i32 = 12;

/// A box the page draws: corners, colour (0 to 255 each) and material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slab {
    pub lo: V3,
    pub hi: V3,
    pub rgb: [i32; 3],
    pub material: i32,
}

/// A room's echo (journal tab 4's table): its row in `data/acoustics.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reverb {
    StoreSoft = 0,
    StoreHard = 1,
    Corridor = 2,
    Atrium = 3,
    Restroom = 4,
    Concrete = 5,
}

pub const REVERBS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Room {
    pub key: &'static str,
    pub lo: V3,
    pub hi: V3,
    pub reverb: Reverb,
    /// the floor under foot, for footsteps
    pub floor: i32,
}

impl Room {
    pub fn holds(&self, p: V3) -> bool {
        p.x >= self.lo.x && p.x <= self.hi.x && p.z >= self.lo.z && p.z <= self.hi.z
    }
    pub fn centre(&self) -> V3 {
        v3(
            (self.lo.x + self.hi.x) / 2,
            ONE * 3 / 2,
            (self.lo.z + self.hi.z) / 2,
        )
    }
}

/// A doorway between two rooms; `door` names the door that can shut it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Portal {
    pub a: usize,
    pub b: usize,
    pub at: V3,
    pub door: Option<usize>,
}

/// A door turned by its handle (CH-Q10 A). `along_x` says the door's face
/// runs along x; `brush` is its index in the map's brushes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Door {
    pub key: &'static str,
    pub lo: V3,
    pub hi: V3,
    pub handle: V3,
    pub brush: usize,
    /// handle turns needed, in Q16.16 turns (1½ for most, more when stuck)
    pub turns: Fx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureKind {
    Troffer,
    Pendant,
    Neon,
    ExitSign,
    Candle,
    Lamp,
    Moon,
}

/// A light in the mall (journal tab 6). Every one but the moon can break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fixture {
    pub kind: FixtureKind,
    pub at: V3,
    pub rgb: [i32; 3],
    /// brightness, Q16.16, and reach in metres
    pub power: Fx,
    pub range: Fx,
    /// already failing when the game starts
    pub flickers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    Mannequin,
    Car,
    Rack,
    Ouija,
    Spare,
    Booth,
    Directory,
    Clock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prop {
    pub kind: PropKind,
    pub at: V3,
    pub yaw: i32,
}

/// A place the director may use, and what happens there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScareKind {
    DoorCreak,
    Fall,
    Shutter,
    Steps,
    Laugh,
    Glass,
}

impl ScareKind {
    pub const COUNT: usize = 6;

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScarePoint {
    pub kind: ScareKind,
    pub at: V3,
    /// the least menace that may use it
    pub menace: i32,
}

/// A sign or line the page lays over the view where it stands (tab 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sign {
    pub key: &'static str,
    pub at: V3,
    /// the way its face looks, as a heading (0 looks along −z)
    pub yaw: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mall {
    pub map: Map,
    pub slabs: Vec<Slab>,
    pub rooms: Vec<Room>,
    pub portals: Vec<Portal>,
    pub doors: Vec<Door>,
    pub fixtures: Vec<Fixture>,
    pub props: Vec<Prop>,
    pub hides: Vec<V3>,
    pub scares: Vec<ScarePoint>,
    pub signs: Vec<Sign>,
    pub cultist_posts: Vec<V3>,
    pub spawn: V3,
    pub spawn_yaw: i32,
    /// west of this x in the vestibule is out
    pub exit_x: Fx,
    /// the cursed store's room and the ritual's centre
    pub cursed: usize,
    pub ring: V3,
}

/// Tenths of a metre, for the tables below.
const fn d(n: i32) -> Fx {
    n * ONE / 10
}

const WALL: Fx = ONE * 3 / 10;
/// The cult's ring: its centre (was x 75, z 0, T6) and the posts' radius
/// (was 5 m).
pub const RING: V3 = V3 {
    x: 70 * ONE,
    y: 0,
    z: -8 * ONE,
};
pub const RING_R: Fx = 4 * ONE;
/// The two walkers circle the ring this far out (was 11 m).
pub const PATROL_R: Fx = 6 * ONE;

pub const OLD_TILE: [i32; 3] = [118, 110, 96];
const DIRTY_PLASTER: [i32; 3] = [132, 124, 108];
const STAIN: [i32; 3] = [96, 86, 70];
const DUST_CARPET: [i32; 3] = [82, 64, 62];
const CEIL: [i32; 3] = [70, 68, 62];
const CONC: [i32; 3] = [92, 92, 88];

struct Build {
    map: Map,
    slabs: Vec<Slab>,
}

impl Build {
    fn solid(&mut self, lo: V3, hi: V3, rgb: [i32; 3], material: i32) {
        self.map.brushes.push(block(lo, hi, Surface::Wall));
        self.slabs.push(Slab {
            lo,
            hi,
            rgb,
            material,
        });
    }
    fn floor(&mut self, lo: V3, hi: V3, rgb: [i32; 3], material: i32) {
        self.map.brushes.push(block(lo, hi, Surface::Floor));
        self.slabs.push(Slab {
            lo,
            hi,
            rgb,
            material,
        });
    }
    fn drawn(&mut self, lo: V3, hi: V3, rgb: [i32; 3], material: i32) {
        self.slabs.push(Slab {
            lo,
            hi,
            rgb,
            material,
        });
    }
    /// A wall along x at `z`, from `x0` to `x1`, `h` high, less the gaps.
    fn wall_x(&mut self, z: Fx, x0: Fx, x1: Fx, h: Fx, gaps: &[(Fx, Fx)], rgb: [i32; 3]) {
        let mut x = x0;
        for &(a, b) in gaps.iter().chain([(x1, x1)].iter()) {
            if a > x {
                self.solid(v3(x, 0, z - WALL / 2), v3(a, h, z + WALL / 2), rgb, PLASTER);
            }
            if b < x1 && b > a {
                // the lintel over a gap
                self.solid(v3(a, int(3), z - WALL / 2), v3(b, h, z + WALL / 2), rgb, PLASTER);
            }
            x = b;
        }
    }
    fn wall_z(&mut self, x: Fx, z0: Fx, z1: Fx, h: Fx, gaps: &[(Fx, Fx)], rgb: [i32; 3]) {
        let mut z = z0;
        for &(a, b) in gaps.iter().chain([(z1, z1)].iter()) {
            if a > z {
                self.solid(v3(x - WALL / 2, 0, z), v3(x + WALL / 2, h, a), rgb, PLASTER);
            }
            if b < z1 && b > a {
                self.solid(v3(x - WALL / 2, int(3), a), v3(x + WALL / 2, h, b), rgb, PLASTER);
            }
            z = b;
        }
    }
    /// A door's brush, shut; returns its index.
    fn door(&mut self, lo: V3, hi: V3) -> usize {
        self.map.brushes.push(block(lo, hi, Surface::Wall));
        self.map.brushes.len() - 1
    }
}

/// Where a door's brush goes when it is open: far under the floor.
pub fn open_brush() -> Brush {
    block(v3(0, int(-200), 0), v3(ONE, int(-199), ONE), Surface::Wall)
}

pub fn shut_brush(door: &Door) -> Brush {
    block(door.lo, door.hi, Surface::Wall)
}

fn room(key: &'static str, x0: i32, z0: i32, x1: i32, z1: i32, reverb: Reverb, floor: i32) -> Room {
    Room {
        key,
        lo: v3(d(x0), 0, d(z0)),
        hi: v3(d(x1), int(9), d(z1)),
        reverb,
        floor,
    }
}

/// The mall, built the same way every time.
pub fn mall() -> Mall {
    let mut b = Build {
        map: Map::default(),
        slabs: Vec::new(),
    };
    let i = int;
    // floors (sunk a little so none shares a plane with another)
    b.floor(v3(i(-8), -ONE, i(-5)), v3(i(60), 0, i(5)), OLD_TILE, TILE);
    b.floor(
        v3(i(-8), -ONE - ONE / 50, i(5)),
        v3(i(60), -ONE / 50, i(19)),
        DUST_CARPET,
        CARPET,
    );
    b.floor(
        v3(i(0), -ONE - ONE / 40, i(-15)),
        v3(i(60), -ONE / 40, i(-5)),
        OLD_TILE,
        TILE,
    );
    b.floor(
        v3(i(60), -ONE - ONE / 30, i(-15)),
        v3(i(102), -ONE / 30, i(15)),
        [104, 98, 86],
        TILE,
    );
    // ceilings (drawn only)
    b.drawn(
        v3(i(-8), i(5), i(-5)),
        v3(i(60), i(5) + WALL, i(5)),
        CEIL,
        CEILING,
    );
    b.drawn(v3(i(0), i(4), i(5)), v3(i(60), i(4) + WALL, i(15)), CEIL, CEILING);
    b.drawn(
        v3(i(0), i(4), i(-15)),
        v3(i(60), i(4) + WALL, i(-5)),
        CEIL,
        CEILING,
    );
    b.drawn(
        v3(i(-8), d(35), i(5)),
        v3(i(60), d(35) + WALL, i(19)),
        CEIL,
        CEILING,
    );
    b.drawn(
        v3(i(90), i(4), i(-5)),
        v3(i(102), i(4) + WALL, i(5)),
        CEIL,
        CEILING,
    );
    // the court's roof, with the skylight's frame
    b.drawn(
        v3(i(60), i(9), i(-15)),
        v3(i(70), i(9) + WALL, i(15)),
        CEIL,
        CEILING,
    );
    b.drawn(
        v3(i(80), i(9), i(-15)),
        v3(i(90), i(9) + WALL, i(15)),
        CEIL,
        CEILING,
    );
    b.drawn(
        v3(i(70), i(9), i(-15)),
        v3(i(80), i(9) + WALL, i(-5)),
        CEIL,
        CEILING,
    );
    b.drawn(
        v3(i(70), i(9), i(5)),
        v3(i(80), i(9) + WALL, i(15)),
        CEIL,
        CEILING,
    );
    b.drawn(
        v3(i(70), i(9) + WALL, i(-5)),
        v3(i(80), i(9) + WALL * 2, i(5)),
        [40, 48, 60],
        GLASS,
    );

    // the corridor's north wall: four shop fronts
    let fronts_n = [(i(7), i(13)), (i(21), i(27)), (i(35), i(41)), (i(49), i(55))];
    b.wall_x(i(5), i(-4), i(60), i(5), &fronts_n, DIRTY_PLASTER);
    b.wall_x(i(5), i(-8), i(-7), i(5), &[], DIRTY_PLASTER);
    let fronts_s = [(i(21), i(27)), (i(35), i(41)), (i(49), i(55))];
    b.wall_x(i(-5), i(-8), i(60), i(5), &fronts_s, DIRTY_PLASTER);
    // the vestibule's glass doors (the exit), its side to the stairwell
    b.solid(
        v3(i(-8) - WALL, 0, i(-5)),
        v3(i(-8), i(5), i(5)),
        [60, 70, 76],
        GLASS,
    );
    let stair_door = b.door(v3(i(-7), 0, i(5) - WALL / 2), v3(i(-5), d(25), i(5) + WALL / 2));
    b.wall_x(i(5), i(-5), i(-4), i(5), &[], DIRTY_PLASTER);
    b.drawn(
        v3(i(-7), d(25), i(5) - WALL / 2),
        v3(i(-5), i(5), i(5) + WALL / 2),
        DIRTY_PLASTER,
        PLASTER,
    );
    // store walls, north
    for x in [4, 16, 18, 30, 32, 44, 46, 58] {
        b.wall_z(i(x), i(5), i(15), i(4), &[], STAIN);
    }
    // the back wall, with Kandy Korner's back door
    let kandy_door = b.door(v3(i(51), 0, i(15) - WALL / 2), v3(i(53), d(25), i(15) + WALL / 2));
    b.wall_x(i(15), i(-4), i(60), d(35), &[(i(51), i(53))], STAIN);
    b.drawn(
        v3(i(51), d(25), i(15) - WALL / 2),
        v3(i(53), d(35), i(15) + WALL / 2),
        STAIN,
        PLASTER,
    );
    // stores, south
    for x in [18, 30, 32, 44, 46, 58] {
        b.wall_z(i(x), i(-15), i(-5), i(4), &[], STAIN);
    }
    b.wall_x(i(-15), i(18), i(58), i(4), &[], STAIN);
    b.solid(
        v3(i(0), 0, i(-15)),
        v3(i(18), i(5), i(-5)),
        [58, 54, 48],
        CONCRETE,
    );
    // the stairwell and the service corridor
    b.wall_z(i(-8), i(5), i(19), d(35), &[], CONC);
    b.wall_z(i(-4), i(5), i(15), d(35), &[], CONC);
    b.wall_x(i(19), i(-8), i(60), d(35), &[], CONC);
    b.wall_z(i(60), i(15), i(19), d(35), &[], CONC);
    // the court
    b.wall_z(i(60), i(-15), i(15), i(9), &[(i(-5), i(5))], DIRTY_PLASTER);
    b.wall_x(i(15), i(60), i(90), i(9), &[], DIRTY_PLASTER);
    b.wall_x(i(-15), i(60), i(90), i(9), &[], DIRTY_PLASTER);
    let cursed_door = b.door(v3(i(90) - WALL / 2, 0, -d(6)), v3(i(90) + WALL / 2, d(25), d(6)));
    b.wall_z(i(90), i(-15), i(15), i(9), &[(-d(6), d(6))], DIRTY_PLASTER);
    // Hollow & Daughters
    b.wall_x(i(5), i(90), i(102), i(4), &[], [70, 46, 40]);
    b.wall_x(i(-5), i(90), i(102), i(4), &[], [70, 46, 40]);
    b.wall_z(i(102), i(-5), i(5), i(4), &[], [70, 46, 40]);

    // things to walk round
    // the dry fountain and the raffle car's plinth in the court
    b.solid(
        v3(i(73), 0, i(-12)),
        v3(i(77), d(6), i(-8)),
        [88, 92, 90],
        CONCRETE,
    );
    b.solid(v3(i(72), 0, -d(15)), v3(i(78), d(3), d(15)), [60, 30, 28], CARPET);
    // counters in the stores
    for (x, z) in [(6, 12), (20, 12), (34, 12), (48, 12), (20, -12), (34, -12)] {
        b.solid(
            v3(i(x), 0, i(z)),
            v3(i(x) + i(4), d(10), i(z) + ONE),
            [96, 72, 52],
            WOOD,
        );
    }
    // the ouija's table
    b.solid(v3(i(98), 0, -ONE), v3(i(100), d(8), ONE), [64, 40, 28], WOOD);
    // a bench in the corridor, a fallen sign
    b.solid(v3(i(14), 0, -d(5)), v3(i(17), d(5), d(5)), [74, 58, 44], WOOD);
    b.solid(v3(i(43), 0, i(2)), v3(i(45), d(3), i(3)), [120, 30, 30], METAL);
    // planters
    for x in [8, 33, 57] {
        b.solid(
            v3(i(x), 0, i(-4)),
            v3(i(x) + ONE, d(7), i(-3)),
            [70, 62, 50],
            CONCRETE,
        );
    }

    let rooms = vec![
        room("vestibule", -80, -50, 0, 50, Reverb::Concrete, TILE),
        room("wing_a", 0, -50, 300, 50, Reverb::Corridor, TILE),
        room("wing_b", 300, -50, 600, 50, Reverb::Corridor, TILE),
        room("sole", 40, 50, 160, 150, Reverb::StoreHard, CARPET),
        room("belle", 180, 50, 300, 150, Reverb::StoreSoft, CARPET),
        room("video", 320, 50, 440, 150, Reverb::StoreSoft, CARPET),
        room("kandy", 460, 50, 580, 150, Reverb::StoreHard, CARPET),
        room("tots", 180, -150, 300, -50, Reverb::StoreHard, TILE),
        room("gem", 320, -150, 440, -50, Reverb::StoreHard, TILE),
        room("photo", 460, -150, 580, -50, Reverb::Restroom, TILE),
        room("court", 600, -150, 900, 150, Reverb::Atrium, TILE),
        room("cursed", 900, -50, 1020, 50, Reverb::StoreSoft, CARPET),
        room("service", -80, 150, 600, 190, Reverb::Concrete, CONCRETE),
        room("stair", -80, 50, -40, 150, Reverb::Concrete, CONCRETE),
    ];
    let h = ONE * 3 / 2;
    let p = |a: usize, b: usize, x: i32, z: i32, door: Option<usize>| Portal {
        a,
        b,
        at: v3(d(x), h, d(z)),
        door,
    };
    let portals = vec![
        p(0, 1, 0, 0, None),
        p(1, 2, 300, 0, None),
        p(1, 3, 100, 50, None),
        p(1, 4, 240, 50, None),
        p(2, 5, 380, 50, None),
        p(2, 6, 520, 50, None),
        p(1, 7, 240, -50, None),
        p(2, 8, 380, -50, None),
        p(2, 9, 520, -50, None),
        p(2, 10, 600, 0, None),
        p(10, 11, 900, 0, Some(0)),
        p(6, 12, 520, 150, Some(1)),
        p(12, 13, -60, 150, None),
        p(13, 0, -60, 50, Some(2)),
    ];
    let doors = vec![
        Door {
            key: "door_cursed",
            lo: v3(i(90) - WALL / 2, 0, -d(6)),
            hi: v3(i(90) + WALL / 2, d(25), d(6)),
            handle: v3(i(90) - WALL, d(10), d(4)),
            brush: cursed_door,
            turns: ONE * 2,
        },
        Door {
            key: "door_kandy",
            lo: v3(i(51), 0, i(15) - WALL / 2),
            hi: v3(i(53), d(25), i(15) + WALL / 2),
            handle: v3(i(52) + d(6), d(10), i(15) - WALL),
            brush: kandy_door,
            turns: ONE * 3 / 2,
        },
        Door {
            key: "door_stair",
            lo: v3(i(-7), 0, i(5) - WALL / 2),
            hi: v3(i(-5), d(25), i(5) + WALL / 2),
            handle: v3(i(-6) + d(6), d(10), i(5) - WALL),
            brush: stair_door,
            turns: ONE * 3 / 2,
        },
    ];

    let warm = [255, 214, 160];
    let cool = [214, 232, 255];
    let f =
        |kind: FixtureKind, x: i32, y: i32, z: i32, rgb: [i32; 3], power: i32, range: i32, flickers: bool| {
            Fixture {
                kind,
                at: v3(d(x), d(y), d(z)),
                rgb,
                power: power * ONE / 100,
                range: d(range),
                flickers,
            }
        };
    let mut fixtures = Vec::new();
    // T6: no pendant at 56 m: it lit the first 6 m of the court, where you
    // stop to look in
    for (k, x) in [40, 110, 190, 260, 340, 410, 490].iter().enumerate() {
        fixtures.push(f(
            FixtureKind::Pendant,
            *x,
            46,
            0,
            warm,
            90,
            110,
            k == 2 || k == 5,
        ));
    }
    for (x, z, ok) in [
        (100, 100, true),
        (240, 100, true),
        (380, 100, false),
        (520, 100, true),
        (240, -100, true),
        (380, -100, true),
        (520, -100, false),
    ] {
        fixtures.push(f(
            FixtureKind::Troffer,
            x,
            37,
            z,
            cool,
            if ok { 70 } else { 40 },
            90,
            !ok,
        ));
    }
    fixtures.push(f(FixtureKind::Neon, 100, 42, 46, [255, 70, 90], 35, 50, false));
    fixtures.push(f(FixtureKind::Neon, 520, 42, 46, [255, 90, 200], 35, 50, true));
    fixtures.push(f(FixtureKind::Neon, 240, 42, -46, [90, 200, 255], 30, 50, false));
    fixtures.push(f(
        FixtureKind::ExitSign,
        -60,
        30,
        45,
        [255, 40, 30],
        18,
        30,
        false,
    ));
    fixtures.push(f(FixtureKind::ExitSign, -75, 30, 0, [60, 255, 90], 18, 30, false));
    fixtures.push(f(FixtureKind::Lamp, -40, 32, 170, [255, 200, 150], 45, 70, true));
    fixtures.push(f(FixtureKind::Lamp, 250, 32, 170, [255, 200, 150], 45, 70, false));
    fixtures.push(f(FixtureKind::Lamp, 680, 70, -80, warm, 70, 140, false));
    // T6: the flickering lamp hangs over the cult's side (was x 82, z 8):
    // the cursed door stands in moonlight only
    fixtures.push(f(FixtureKind::Lamp, 840, 70, -120, warm, 70, 140, true));
    fixtures.push(f(FixtureKind::Candle, 990, 9, 5, [255, 170, 90], 40, 50, false));
    fixtures.push(f(
        FixtureKind::Candle,
        960,
        12,
        -40,
        [255, 170, 90],
        30,
        40,
        false,
    ));
    fixtures.push(f(FixtureKind::Moon, 750, 120, 0, [140, 160, 210], 55, 260, false));

    // T6: the ring sits in the court's south-west, off the line between
    // the wing and the cursed store and nearly 18 m from the cursed door;
    // the north half (the raffle car's side) is the way past
    let ring = RING;
    let mut cultist_posts = Vec::new();
    for k in 0..6 {
        let a = k * 65536 / 6;
        let r = RING_R;
        cultist_posts.push(v3(
            ring.x + crate::fx::mul(r, crate::fx::cos(a)),
            0,
            ring.z + crate::fx::mul(r, crate::fx::sin(a)),
        ));
    }
    // the ring's candles: one at each post
    for c in &cultist_posts {
        let toward = c.sub(ring).scale(ONE * 8 / 10);
        let at = ring.add(toward);
        fixtures.push(Fixture {
            kind: FixtureKind::Candle,
            at: v3(at.x, d(3), at.z),
            rgb: [255, 160, 80],
            power: ONE * 30 / 100,
            range: d(45),
            flickers: false,
        });
    }

    let pr = |kind: PropKind, x: i32, z: i32, yaw: i32| Prop {
        kind,
        at: v3(d(x), 0, d(z)),
        yaw: crate::fx::deg(yaw),
    };
    let props = vec![
        pr(PropKind::Mannequin, 200, 80, 180),
        pr(PropKind::Mannequin, 240, 70, 200),
        pr(PropKind::Mannequin, 280, 85, 160),
        pr(PropKind::Mannequin, 120, 60, 180),
        pr(PropKind::Mannequin, 540, -60, 0),
        pr(PropKind::Car, 750, 0, 90),
        pr(PropKind::Rack, 230, 130, 0),
        pr(PropKind::Rack, 360, 70, 0),
        pr(PropKind::Rack, 400, -120, 0),
        pr(PropKind::Ouija, 990, 0, 0),
        pr(PropKind::Spare, 420, 135, 0),
        pr(PropKind::Booth, 285, 135, 0),
        pr(PropKind::Booth, 555, -130, 0),
        pr(PropKind::Booth, 300, 175, 0),
        pr(PropKind::Booth, 880, 130, 0),
        pr(PropKind::Booth, 195, -135, 0),
        pr(PropKind::Directory, 20, 30, 270),
        pr(PropKind::Clock, 640, 140, 180),
    ];
    let hides = props
        .iter()
        .filter(|p| p.kind == PropKind::Booth)
        .map(|p| p.at)
        .collect();

    let s = |kind: ScareKind, x: i32, z: i32, menace: i32| ScarePoint {
        kind,
        at: v3(d(x), d(10), d(z)),
        menace,
    };
    let scares = vec![
        s(ScareKind::DoorCreak, -60, 50, 0),
        s(ScareKind::Fall, 100, 120, 0),
        s(ScareKind::Fall, 160, -30, 0),
        s(ScareKind::Steps, 60, 0, 1),
        s(ScareKind::Steps, 200, 0, 1),
        // CH-Q26 A: three quiet ones in the wings
        s(ScareKind::Shutter, 150, -50, 1),
        s(ScareKind::DoorCreak, 270, 50, 1),
        s(ScareKind::Steps, 290, -30, 1),
        s(ScareKind::Shutter, 240, 50, 1),
        s(ScareKind::Shutter, 380, -50, 1),
        s(ScareKind::Fall, 240, 120, 1),
        s(ScareKind::Laugh, 240, -120, 2),
        s(ScareKind::Glass, 380, -110, 2),
        s(ScareKind::Fall, 380, 120, 2),
        s(ScareKind::Steps, 450, 0, 2),
        s(ScareKind::DoorCreak, 520, 150, 2),
        s(ScareKind::Laugh, 520, -100, 3),
        s(ScareKind::Steps, 650, 100, 3),
        s(ScareKind::Fall, 700, -120, 3),
        s(ScareKind::Glass, 950, -40, 4),
        s(ScareKind::Steps, 940, 30, 4),
    ];
    let sign = |key: &'static str, x: i32, y: i32, z: i32, face: i32| Sign {
        key,
        at: v3(d(x), d(y), d(z)),
        yaw: crate::fx::deg(face),
    };
    // each faces where it is read from: storefront signs on the north wall
    // look south into the wing (0), on the south wall north (180); signs
    // read on the way in look west at the doors (90), the exit east (-90)
    let signs = vec![
        sign("sign_behind", 20, 22, 30, 90),
        sign("sign_open_late", 520, 42, 46, 0),
        sign("sign_try_them", 100, 30, 48, 0),
        sign("sign_lost_child", 340, 14, 30, 90),
        sign("sign_dont_play", 500, 22, -46, 180),
        sign("sign_clock", 640, 40, 140, 0),
        sign("sign_exit", -75, 30, 0, -90),
    ];

    Mall {
        map: Map {
            brushes: b.map.brushes,
            spawns: vec![(v3(i(-5), 0, 0), crate::fx::deg(-90))],
            name: "mall",
        },
        slabs: b.slabs,
        rooms,
        portals,
        doors,
        fixtures,
        props,
        hides,
        scares,
        signs,
        cultist_posts,
        spawn: v3(i(-5), 0, 0),
        spawn_yaw: crate::fx::deg(-90),
        exit_x: i(-7),
        cursed: 11,
        ring,
    }
}

impl Mall {
    /// The room holding `p`, or the nearest by its middle.
    pub fn room_of(&self, p: V3) -> usize {
        if let Some(i) = self.rooms.iter().position(|r| r.holds(p)) {
            return i;
        }
        let mut best = 0;
        let mut far = i32::MAX;
        for (i, r) in self.rooms.iter().enumerate() {
            let dd = r.centre().sub(p).len_xz();
            if dd < far {
                far = dd;
                best = i;
            }
        }
        best
    }

    /// How menacing the spirit is here (journal tab 2's table): 0 at the
    /// entrance to 4 in Hollow & Daughters.
    pub fn menace_at(&self, p: V3) -> i32 {
        let x = p.x;
        if self.room_of(p) == self.cursed {
            4
        } else if x >= int(60) {
            3
        } else if x >= int(30) {
            2
        } else if x >= int(12) {
            1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_portal_joins_two_rooms_that_hold_or_touch_its_point() {
        let m = mall();
        for p in &m.portals {
            for r in [p.a, p.b] {
                let room = &m.rooms[r];
                let near = p.at.x >= room.lo.x - ONE
                    && p.at.x <= room.hi.x + ONE
                    && p.at.z >= room.lo.z - ONE
                    && p.at.z <= room.hi.z + ONE;
                assert!(near, "{} at {:?}", room.key, p.at);
            }
        }
    }

    #[test]
    fn menace_rises_from_the_entrance_to_the_cursed_store() {
        let m = mall();
        assert_eq!(m.menace_at(v3(int(-5), 0, 0)), 0);
        assert_eq!(m.menace_at(v3(int(20), 0, 0)), 1);
        assert_eq!(m.menace_at(v3(int(40), 0, 0)), 2);
        assert_eq!(m.menace_at(v3(int(75), 0, int(9))), 3);
        assert_eq!(m.menace_at(v3(int(95), 0, 0)), 4);
    }
}
