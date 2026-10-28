//! A column's temporary distances and its move by one (docs/adr/0712): on each of its four sides, the clear distance to
//! the nearest face running parallel to it (a wall's or another column's) on its level, and the column moved square to
//! that side so the distance is what was typed, the face across fixed.
//!
//! - **Sides:** a column's box's, from its corner `k` to the next (`geom::arch::column_box`'s order: the side below its
//!   middle first, then right, above, left, as it is turned): a rectangle's own, a round column's bounding square's, an
//!   L's box's (docs/adr/0725 §3).
//! - **Gaps:** on side `k`, outward, the nearest straight edge running parallel to it (within 0.001°) and overlapping
//!   it along its line by more than [`TIE`]: a wall's facing face (a wall on the same level, or both on none), or
//!   another column's side; not behind the side by more than [`TIE`]. Measured square to the side at the middle of the
//!   overlap. Equal distances go to the walls before the columns, then to the one given first, then its lower edge.
//! - **Move:** the column's reference point moved along the side's outward normal by the gap less the distance typed.

use crate::geom::arch::{ColumnSpec, column_box};
use crate::geom::wall::left_face;
use crate::geom::wall_join::WallIn;
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::ops::wall_shift::{PARALLEL, TIE, edge, edge_count, level};
use crate::vec2::Vec2;

/// A column as the rule takes it: its reference point, its shape and its level.
#[derive(Clone, Copy, Debug)]
pub struct ColumnIn<'a> {
    pub at: Vec2,
    pub spec: &'a ColumnSpec,
    pub level: Option<&'a str>,
}

/// What a gap is measured to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// A wall (its index in the walls given), its path's edge, and whether its left face is the one facing.
    Wall { wall: usize, edge: usize, left_face: bool },
    /// Another column (its index in the columns given) and its side.
    Column { column: usize, side: usize },
}

/// A gap on one side of a column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gap {
    pub side: usize,
    pub target: Target,
    /// The clear distance.
    pub distance: f64,
    /// Where it is measured: on the column's side, and on the face across.
    pub from: Vec2,
    pub to: Vec2,
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn along(p: Vec2, d: Vec2, t: f64) -> Vec2 {
    Vec2::new(p.x + d.x * t, p.y + d.y * t)
}

fn unit(a: Vec2, b: Vec2) -> Option<Vec2> {
    let l = js_hypot(b.x - a.x, b.y - a.y);
    (l > 0.0).then(|| Vec2::new((b.x - a.x) / l, (b.y - a.y) / l))
}

/// The four sides of a column's box, each from a corner to the next, counter-clockwise: a round one's bounding square's,
/// an L's box's.
pub fn sides(c: &ColumnIn<'_>) -> [(Vec2, Vec2); 4] {
    let k = column_box(c.at, c.spec);
    [(k[0], k[1]), (k[1], k[2]), (k[2], k[3]), (k[3], k[0])]
}

fn take(slot: &mut Option<Gap>, g: Gap) {
    match slot {
        Some(b) if b.distance <= g.distance + TIE => {}
        _ => *slot = Some(g),
    }
}

/// The overlap along a side (from `a` along `u`, `l` long) of an edge from `c` to `d`: its middle's distance from `a`;
/// none when they overlap by no more than [`TIE`].
fn overlap(a: Vec2, u: Vec2, l: f64, c: Vec2, d: Vec2) -> Option<f64> {
    let (tc, td) = (dot(sub(c, a), u), dot(sub(d, a), u));
    let lo = js_max(0.0, js_min(tc, td));
    let hi = js_min(l, js_max(tc, td));
    (hi - lo > TIE).then_some((lo + hi) / 2.0)
}

/// The gaps of column `k` among `columns`, side by side; none on a side with nothing parallel facing it.
pub fn gaps(walls: &[WallIn<'_>], columns: &[ColumnIn<'_>], k: usize) -> [Option<Gap>; 4] {
    let mut best: [Option<Gap>; 4] = [None; 4];
    let Some(me) = columns.get(k) else {
        return best;
    };
    for (side, (a, b)) in sides(me).into_iter().enumerate() {
        let Some(u) = unit(a, b) else { continue };
        let l = js_hypot(b.x - a.x, b.y - a.y);
        // Counter-clockwise sides: outward is to the right.
        let n = Vec2::new(u.y, -u.x);
        for (j, w) in walls.iter().enumerate() {
            if level(w) != me.level {
                continue;
            }
            for e in 0..edge_count(w) {
                let Some((c, d)) = edge(w, e) else { continue };
                let Some(v) = unit(c, d) else { continue };
                if cross(u, v).abs() > PARALLEL {
                    continue;
                }
                let Some(mid) = overlap(a, u, l, c, d) else {
                    continue;
                };
                // The wall's faces as distances outward from the side: its left face is to its own left.
                let sgn = if dot(Vec2::new(-v.y, v.x), n) > 0.0 { 1.0 } else { -1.0 };
                let sd = dot(sub(c, a), n);
                let lf = left_face(w.spec);
                let faces = [(sd + sgn * lf, true), (sd + sgn * (lf - w.spec.width), false)];
                let Some((near, left)) = faces
                    .into_iter()
                    .filter(|(f, _)| *f >= -TIE)
                    .min_by(|x, y| x.0.total_cmp(&y.0))
                else {
                    continue;
                };
                let base = along(a, u, mid);
                take(
                    &mut best[side],
                    Gap {
                        side,
                        target: Target::Wall { wall: j, edge: e, left_face: left },
                        distance: js_max(0.0, near),
                        from: base,
                        to: along(base, n, near),
                    },
                );
            }
        }
        for (j, other) in columns.iter().enumerate() {
            if j == k || other.level != me.level {
                continue;
            }
            for (s, (c, d)) in sides(other).into_iter().enumerate() {
                let Some(v) = unit(c, d) else { continue };
                if cross(u, v).abs() > PARALLEL {
                    continue;
                }
                let Some(mid) = overlap(a, u, l, c, d) else {
                    continue;
                };
                let f = dot(sub(c, a), n);
                if f < -TIE {
                    continue;
                }
                let base = along(a, u, mid);
                take(
                    &mut best[side],
                    Gap {
                        side,
                        target: Target::Column { column: j, side: s },
                        distance: js_max(0.0, f),
                        from: base,
                        to: along(base, n, f),
                    },
                );
            }
        }
    }
    best
}

/// Column `k`'s centre once side `side` is `distance` from what it faces; why not (no such side, nothing facing it).
pub fn moved(
    walls: &[WallIn<'_>],
    columns: &[ColumnIn<'_>],
    k: usize,
    side: usize,
    distance: f64,
) -> Result<Vec2, String> {
    if side > 3 {
        return Err(format!("Kolonun {side}. yanı yok; yanları 0 ile 3 arasında."));
    }
    let me = columns.get(k).ok_or_else(|| "Kolon yok.".to_owned())?;
    let Some(g) = gaps(walls, columns, k)[side] else {
        return Err(
            "Kolonun bu yanında ona paralel bir duvar ya da kolon yok; mesafe ölçülemiyor.".into(),
        );
    };
    let (a, b) = sides(me)[side];
    let u = unit(a, b).ok_or_else(|| "Kolonun bu yanı sıfır uzunlukta.".to_owned())?;
    let n = Vec2::new(u.y, -u.x);
    Ok(along(me.at, n, g.distance - distance))
}
