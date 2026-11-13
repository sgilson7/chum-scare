//! A map: convex brushes, each the set of points behind every one of its
//! planes, as Source's (and CSSurf's `map.rs`) are. A player's box is swept
//! against them in `trace.rs`; the page draws each as a cube or a wedge.
//!
//! Y is up, lengths are metres in Q16.16. A face is ground when its
//! normal's y is at least `movement::GROUND_NORMAL_Y` (0.7, about 45.6°); a
//! steeper face is a surf ramp: no footing, so the air rules hold on it.
//!
//! M1's map is the practice room (`practice`): a floor, walls to jump off,
//! a walkable slope up to a balcony, two surf ramps below the balcony, a
//! wall-jump shaft and a few blocks, for judging the movement before any
//! weapon exists (PLAN.md M1).

use crate::fx::{self, Fx, ONE};

/// The disc's sides and rim corners, ported from Chumontillado's `mesh.rs`
/// at c48d8dd (its drawing stays behind; R2.2).
mod mesh {
    use crate::fx;
    pub const DISC_SIDES: i32 = 32;
    pub fn disc_corner(j: i32, r: i32) -> (i32, i32) {
        let a = fx::TURN * (2 * j + 1) / (2 * DISC_SIDES);
        (fx::mul(fx::sin(a), r), fx::mul(fx::cos(a), r))
    }
}
use crate::geom::{v3, Affine, V3};

/// A face's plane: the brush lies where `n · p <= d`. `n` has length `ONE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plane {
    pub n: V3,
    pub d: Fx,
}

/// How far past a face a point may read and still be on it (0.25 mm).
const HOLD_TOLERANCE: Fx = ONE / 4096;

/// What a brush is, for its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Floor,
    Wall,
    Ramp,
    Slope,
    Block,
    Trim,
}

/// How a brush is drawn: a box, or a triangular prism along x or z with its
/// cross-section's three corners (in the other two axes) and its length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// a box, from its corners as built
    Block { mins: V3, maxs: V3 },
    /// corners as (across, up) pairs; `along_z` says which axis the prism runs
    Prism {
        corners: [(Fx, Fx); 3],
        along_z: bool,
        from: Fx,
        to: Fx,
    },
    /// T28: a flat-sided column of `mesh::DISC_SIDES` about the vertical
    /// through the origin, its faces `apothem` out, from `y0` up to `y1`
    Disc { apothem: Fx, y0: Fx, y1: Fx },
    /// T31: a prism along z whose cross-section, (x, up) as `Prism`'s, falls
    /// or rises as it runs: at `from` it stands on `from_y`, at `to` on
    /// `to_y` (the Tobogán's slide)
    Sheared {
        corners: [(Fx, Fx); 3],
        from: Fx,
        to: Fx,
        from_y: Fx,
        to_y: Fx,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brush {
    pub planes: Vec<Plane>,
    pub mins: V3,
    pub maxs: V3,
    pub surface: Surface,
    /// the shape as built, before `turn`
    pub shape: Shape,
    /// M5: a turn about the vertical and a move, carrying the shape as
    /// built onto where it stands (`turned`); the identity for most
    pub turn: Affine,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Map {
    pub brushes: Vec<Brush>,
    /// where a player starts, and the yaw they face
    pub spawns: Vec<(V3, i32)>,
    /// M5: its name's key in the strings
    pub name: &'static str,
}

impl Map {
    /// T27: the lowest point of any brush: below it there is nothing.
    pub fn lowest(&self) -> Fx {
        self.brushes.iter().map(|b| b.mins.y).min().unwrap_or(0)
    }

    /// T27: inside the walls, across and along: every walled map's floor
    /// (its first brush) reaches a metre past the walls' inner faces each
    /// way. Nobody goes past it at any height, over the walls into the sky
    /// included; shots do. T28: a map with no walls (the Plaza) has no
    /// inside; off its edge is a fall. T37: nor has the Cubo, whose outside
    /// is walked on too.
    pub fn inside(&self) -> Option<(V3, V3)> {
        if !self.brushes.iter().any(|b| b.surface == Surface::Wall) {
            return None;
        }
        let f = &self.brushes[0];
        Some((f.mins.add(v3(ONE, 0, ONE)), f.maxs.sub(v3(ONE, 0, ONE))))
    }

    /// T28: the box round all its floors, across and along.
    pub fn floors(&self) -> (V3, V3) {
        let floors = self.brushes.iter().filter(|b| b.surface == Surface::Floor);
        floors.fold((self.brushes[0].mins, self.brushes[0].maxs), |(lo, hi), b| {
            (
                v3(lo.x.min(b.mins.x), lo.y.min(b.mins.y), lo.z.min(b.mins.z)),
                v3(hi.x.max(b.maxs.x), hi.y.max(b.maxs.y), hi.z.max(b.maxs.z)),
            )
        })
    }
}

fn plane(n: V3, through: V3) -> Plane {
    Plane { n, d: n.dot(through) }
}

/// The six axis planes of a box: also every brush's bevels (`trace.rs`).
fn box_planes(mins: V3, maxs: V3) -> Vec<Plane> {
    vec![
        plane(v3(ONE, 0, 0), maxs),
        plane(v3(-ONE, 0, 0), mins),
        plane(v3(0, ONE, 0), maxs),
        plane(v3(0, -ONE, 0), mins),
        plane(v3(0, 0, ONE), maxs),
        plane(v3(0, 0, -ONE), mins),
    ]
}

/// An axis-aligned box from `mins` to `maxs`.
pub fn block(mins: V3, maxs: V3, surface: Surface) -> Brush {
    Brush {
        planes: box_planes(mins, maxs),
        mins,
        maxs,
        surface,
        shape: Shape::Block { mins, maxs },
        turn: Affine::IDENTITY,
    }
}

/// A point in the world from a cross-section's (across, up) and a distance
/// along the prism.
fn place(along_z: bool, across: Fx, up: Fx, along: Fx) -> V3 {
    if along_z {
        v3(across, up, along)
    } else {
        v3(along, up, across)
    }
}

/// A triangular prism: the cross-section's three corners, given as (across,
/// up) in the plane square to its length, run from `from` to `to` along z
/// (or along x). Its planes are the three sides, the two end caps and the
/// box round it (bevels, which a convex set lies behind anyway).
pub fn prism(corners: [(Fx, Fx); 3], along_z: bool, from: Fx, to: Fx, surface: Surface) -> Brush {
    let mid = (
        (corners[0].0 + corners[1].0 + corners[2].0) / 3,
        (corners[0].1 + corners[1].1 + corners[2].1) / 3,
    );
    let mut planes = Vec::new();
    for k in 0..3 {
        let (a, b) = (corners[k], corners[(k + 1) % 3]);
        // the edge's normal in the cross-section, turned to face away from
        // the middle
        let (ea, eu) = (b.0 - a.0, b.1 - a.1);
        let len = fx::length(ea, eu);
        let (mut na, mut nu) = (fx::div(eu, len), fx::div(-ea, len));
        let outward = fx::mul(na, a.0 - mid.0) + fx::mul(nu, a.1 - mid.1);
        if outward < 0 {
            na = -na;
            nu = -nu;
        }
        let n = place(along_z, na, nu, 0);
        planes.push(plane(n, place(along_z, a.0, a.1, from)));
    }
    let lo = (
        corners.iter().map(|c| c.0).min().unwrap(),
        corners.iter().map(|c| c.1).min().unwrap(),
    );
    let hi = (
        corners.iter().map(|c| c.0).max().unwrap(),
        corners.iter().map(|c| c.1).max().unwrap(),
    );
    let (a, b) = (
        place(along_z, lo.0, lo.1, from.min(to)),
        place(along_z, hi.0, hi.1, from.max(to)),
    );
    let mins = v3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
    let maxs = v3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
    planes.extend(box_planes(mins, maxs));
    Brush {
        planes,
        mins,
        maxs,
        surface,
        shape: Shape::Prism {
            corners,
            along_z,
            from: from.min(to),
            to: from.max(to),
        },
        turn: Affine::IDENTITY,
    }
}

/// The sheared prism's six corners: each of its cross-section's at `from`
/// then at `to`.
fn sheared_corners(shape: &Shape) -> [V3; 6] {
    let Shape::Sheared {
        corners,
        from,
        to,
        from_y,
        to_y,
    } = *shape
    else {
        return [V3::ZERO; 6];
    };
    let at = |c: (Fx, Fx), z: Fx, y: Fx| v3(c.0, c.1 + y, z);
    [
        at(corners[0], from, from_y),
        at(corners[0], to, to_y),
        at(corners[1], from, from_y),
        at(corners[1], to, to_y),
        at(corners[2], from, from_y),
        at(corners[2], to, to_y),
    ]
}

/// T31: a prism along z from `from` to `to`, its cross-section's corners
/// (x, up) standing on `from_y` at one end and `to_y` at the other, so its
/// faces fall or rise as it runs: a stretch of the Tobogán's slide. Its
/// planes are the three sides, the two upright ends and the box round it.
pub fn sheared(corners: [(Fx, Fx); 3], from: Fx, to: Fx, from_y: Fx, to_y: Fx, surface: Surface) -> Brush {
    let shape = Shape::Sheared {
        corners,
        from,
        to,
        from_y,
        to_y,
    };
    let c = sheared_corners(&shape);
    let centre = c.iter().fold(V3::ZERO, |a, p| a.add(p.scale(ONE / 6)));
    let run = c[1].sub(c[0]);
    let mut planes = Vec::new();
    for k in 0..3 {
        let (a, b) = (c[2 * k], c[2 * ((k + 1) % 3)]);
        let mut n = b.sub(a).cross(run).norm();
        if n.dot(a.sub(centre)) < 0 {
            n = n.neg();
        }
        planes.push(plane(n, a));
    }
    let (lo, hi) = (from.min(to), from.max(to));
    planes.push(plane(v3(0, 0, -ONE), v3(0, 0, lo)));
    planes.push(plane(v3(0, 0, ONE), v3(0, 0, hi)));
    let mins = c
        .iter()
        .fold(c[0], |m, p| v3(m.x.min(p.x), m.y.min(p.y), m.z.min(p.z)));
    let maxs = c
        .iter()
        .fold(c[0], |m, p| v3(m.x.max(p.x), m.y.max(p.y), m.z.max(p.z)));
    planes.extend(box_planes(mins, maxs));
    Brush {
        planes,
        mins,
        maxs,
        surface,
        shape,
        turn: Affine::IDENTITY,
    }
}

/// How far out the disc's corners lie.
fn disc_reach(shape: &Shape) -> Fx {
    match *shape {
        Shape::Disc { apothem, .. } => {
            let half_side = fx::HALF_TURN / mesh::DISC_SIDES;
            fx::div(apothem, fx::cos(half_side))
        }
        _ => 0,
    }
}

fn disc_corners(shape: &Shape) -> Vec<(Fx, Fx)> {
    let r = disc_reach(shape);
    (0..mesh::DISC_SIDES).map(|j| mesh::disc_corner(j, r)).collect()
}

/// T28: a flat-sided column of `mesh::DISC_SIDES` round the vertical
/// through the origin, a face `apothem` out each way (sin, cos) of a whole
/// number of sides, as the Plaza's stands stand: the Plaza's floor, one
/// brush, so no two pieces of floor lie in one plane.
pub fn disc(apothem: Fx, y0: Fx, y1: Fx, surface: Surface) -> Brush {
    let shape = Shape::Disc { apothem, y0, y1 };
    let mut planes: Vec<Plane> = (0..mesh::DISC_SIDES)
        .map(|k| {
            let a = fx::TURN * k / mesh::DISC_SIDES;
            let n = v3(fx::sin(a), 0, fx::cos(a));
            plane(n, n.scale(apothem))
        })
        .collect();
    let corners = disc_corners(&shape);
    let reach = |f: fn(&(Fx, Fx)) -> Fx| corners.iter().map(f).max().unwrap_or(0);
    let (rx, rz) = (reach(|c| c.0), reach(|c| c.1));
    let (mins, maxs) = (v3(-rx, y0, -rz), v3(rx, y1, rz));
    planes.extend(box_planes(mins, maxs));
    Brush {
        planes,
        mins,
        maxs,
        surface,
        shape,
        turn: Affine::IDENTITY,
    }
}

/// M5: a brush built about the origin, turned `yaw` about the vertical and
/// moved to `to`: the bullring's stands and shields. Every plane
/// turns with it; the box round the turned corners is added as bevels, as
/// Source adds them, so a box swept against it cannot snag on an edge.
pub fn turned(b: Brush, yaw: i32, to: V3) -> Brush {
    let turn = Affine::translate(to).then(&Affine::rot_y(yaw));
    let corners: Vec<V3> = match b.shape {
        Shape::Block { mins, maxs } => (0..8)
            .map(|k| {
                v3(
                    if k & 1 == 0 { mins.x } else { maxs.x },
                    if k & 2 == 0 { mins.y } else { maxs.y },
                    if k & 4 == 0 { mins.z } else { maxs.z },
                )
            })
            .collect(),
        Shape::Prism {
            corners,
            along_z,
            from,
            to,
        } => corners
            .iter()
            .flat_map(|c| [place(along_z, c.0, c.1, from), place(along_z, c.0, c.1, to)])
            .collect(),
        Shape::Sheared { .. } => sheared_corners(&b.shape).to_vec(),
        Shape::Disc { y0, y1, .. } => disc_corners(&b.shape)
            .iter()
            .flat_map(|&(x, z)| [v3(x, y0, z), v3(x, y1, z)])
            .collect(),
    };
    // (a brush turned before is carried from where it was built)
    let full = turn.then(&b.turn);
    let moved: Vec<V3> = corners.iter().map(|c| full.apply(*c)).collect();
    let mins = moved
        .iter()
        .fold(moved[0], |a, c| v3(a.x.min(c.x), a.y.min(c.y), a.z.min(c.z)));
    let maxs = moved
        .iter()
        .fold(moved[0], |a, c| v3(a.x.max(c.x), a.y.max(c.y), a.z.max(c.z)));
    let mut planes: Vec<Plane> = b
        .planes
        .iter()
        .map(|pl| {
            let n = turn.apply_dir(pl.n);
            // a point on the plane, carried with it
            Plane {
                n,
                d: n.dot(turn.apply(pl.n.scale(pl.d))),
            }
        })
        .collect();
    planes.extend(box_planes(mins, maxs));
    Brush {
        planes,
        mins,
        maxs,
        surface: b.surface,
        shape: b.shape,
        turn: full,
    }
}

impl Surface {
    /// The surface's row in the look's grit table (`data/look.json`,
    /// `grit.materials`): 1 floor, 2 wall, 3 ramp, 4 slope, 5 block, 6 trim.
    pub fn material(self) -> i32 {
        match self {
            Surface::Floor => 1,
            Surface::Wall => 2,
            Surface::Ramp => 3,
            Surface::Slope => 4,
            Surface::Block => 5,
            Surface::Trim => 6,
        }
    }
}

impl Brush {
    /// Whether `p` lies inside the brush or on its face.
    pub fn holds(&self, p: V3) -> bool {
        // a corner on a face may read a few units past it after rounding
        self.planes.iter().all(|pl| pl.n.dot(p) <= pl.d + HOLD_TOLERANCE)
    }
}

impl Map {
    /// The brushes whose boxes reach into the box `lo` to `hi`, by index.
    pub fn brushes_near(&self, lo: V3, hi: V3, out: &mut Vec<u32>) {
        out.clear();
        for (i, b) in self.brushes.iter().enumerate() {
            if b.mins.x <= hi.x
                && b.maxs.x >= lo.x
                && b.mins.y <= hi.y
                && b.maxs.y >= lo.y
                && b.mins.z <= hi.z
                && b.maxs.z >= lo.z
            {
                out.push(i as u32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::int;

    #[test]
    fn a_box_holds_its_middle_and_not_a_point_outside() {
        let b = block(v3(0, 0, 0), v3(int(2), int(2), int(2)), Surface::Block);
        assert!(b.holds(v3(ONE, ONE, ONE)));
        assert!(!b.holds(v3(int(3), ONE, ONE)));
    }

    #[test]
    fn a_prism_holds_its_corners_and_its_middle_and_nothing_past_its_sides() {
        let r = prism(
            [(int(-5), 0), (int(5), 0), (0, int(7))],
            true,
            0,
            int(10),
            Surface::Ramp,
        );
        for (x, y) in [(int(-5), 0), (int(5), 0), (0, int(7)), (0, ONE)] {
            assert!(r.holds(v3(x, y, int(5))), "{x} {y}");
        }
        // just outside the left slope, and past the end cap
        assert!(!r.holds(v3(int(-3), int(4), int(5))));
        assert!(!r.holds(v3(0, ONE, int(11))));
        // its box: 10 across, 7 up, 10 along
        assert_eq!((r.mins, r.maxs), (v3(int(-5), 0, 0), v3(int(5), int(7), int(10))));
    }

    #[test]
    fn the_brushes_near_a_box_are_those_whose_boxes_touch_it() {
        // CH: the practice room is gone with its drawing (R2.2); a 64 m floor
        // and its spawn stand in for it
        let m = Map {
            brushes: vec![block(
                v3(int(-32), -ONE, int(-32)),
                v3(int(32), 0, int(32)),
                Surface::Floor,
            )],
            spawns: vec![(V3::ZERO, 0)],
            ..Map::default()
        };
        let mut out = Vec::new();
        // high in the middle of the room, nothing but nothing
        m.brushes_near(v3(-ONE, int(20), -ONE), v3(ONE, int(21), ONE), &mut out);
        assert!(out.is_empty(), "{out:?}");
        // on the floor at the spawn: the floor
        let (s, _) = m.spawns[0];
        m.brushes_near(s.sub(v3(ONE, ONE, ONE)), s.add(v3(ONE, ONE, ONE)), &mut out);
        assert!(out.contains(&0));
    }

    #[test]
    fn a_block_turned_an_eighth_holds_its_middle_and_not_its_old_corners_and_stops_a_trace_at_its_face() {
        // a 2 m cube about the origin, turned 45° and moved to (10, 0, 0):
        // its corners now lie on the axes, √2 m from its middle
        let b = turned(
            block(v3(-ONE, 0, -ONE), v3(ONE, int(2), ONE), Surface::Wall),
            fx::deg(45),
            v3(int(10), 0, 0),
        );
        assert!(b.holds(v3(int(10), ONE, 0)));
        assert!(
            b.holds(v3(int(10) + ONE * 14 / 10, ONE, 0)),
            "a turned corner, 1.4 m out along x"
        );
        assert!(
            !b.holds(v3(int(11) - ONE / 50, ONE, ONE - ONE / 50)),
            "the old corner is outside now"
        );
        // its box is the turned corners': 1.41 m each way
        assert!(
            (b.maxs.x - int(10) - ONE * 1414 / 1000).abs() < ONE / 100,
            "{}",
            fx::decimal(b.maxs.x, 3)
        );
        // a point traced along −x from 20 m stops at the corner, 11.41
        let m = Map {
            brushes: vec![b],
            ..Map::default()
        };
        let mut sc = crate::trace::Scratch::default();
        let t = crate::trace::trace_box(
            &m,
            &mut sc,
            v3(int(20), ONE, 0),
            v3(0, ONE, 0),
            V3::ZERO,
            V3::ZERO,
        );
        let stop = int(20) - fx::mul(t.fraction, int(20));
        assert!(
            (stop - int(10) - ONE * 1414 / 1000).abs() < ONE / 50,
            "{}",
            fx::decimal(stop, 3)
        );
    }
}
