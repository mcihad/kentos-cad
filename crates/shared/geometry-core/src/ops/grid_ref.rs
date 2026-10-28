//! Grids as the second reference of the temporary distances (docs/adr/0728 §1): a straight grid's line, endless, on the
//! object's level (or on none), measured from the object's axis square to it. One rule for the desktop, the headless
//! server and the shared cases.
//!
//! - **A line's sides** (a wall's edge): the grids parallel to it (within 0.001°), from its axis — the line `axis` to
//!   its left — at a point of it: to its left when the grid is left of the axis or on it (within [`TIE`]), else to its
//!   right; on each side the nearest, a tie going to the grid given first.
//! - **A box's sides** (a column): the grids parallel to a side, from the box's middle along the side's outward normal;
//!   one through the middle on the lower of the two parallel sides (0 or 1).
//! - **Along a segment** (an opening on its wall's face): where grids cross the segment, before and after a point on
//!   it; one at the point counts before.

use crate::jsmath::{js_hypot, js_max};
use crate::ops::wall_shift::{PARALLEL, TIE};
use crate::vec2::Vec2;

/// A straight grid as the rule takes it: two points of its line and its level.
#[derive(Clone, Copy, Debug)]
pub struct GridIn<'a> {
    pub a: Vec2,
    pub b: Vec2,
    pub level: Option<&'a str>,
}

/// A distance to a grid: the grid (its index in the grids given), the distance, where it is measured from (on the
/// object's axis) and to (on the grid's line).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGap {
    pub grid: usize,
    pub distance: f64,
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

/// Whether a grid counts on `level`: on it, or on none.
fn on_level(g: &GridIn<'_>, level: Option<&str>) -> bool {
    g.level.is_none() || g.level == level
}

fn take(slot: &mut Option<GridGap>, g: GridGap) {
    match slot {
        Some(b) if b.distance <= g.distance + TIE => {}
        _ => *slot = Some(g),
    }
}

/// The nearest grids parallel to direction `u` on each side of the axis through `at` (its left `n = left(u)`):
/// `[left, right]`.
pub fn beside(grids: &[GridIn<'_>], level: Option<&str>, at: Vec2, u: Vec2) -> [Option<GridGap>; 2] {
    let n = Vec2::new(-u.y, u.x);
    let mut best = [None, None];
    for (i, g) in grids.iter().enumerate() {
        if !on_level(g, level) {
            continue;
        }
        let Some(v) = unit(g.a, g.b) else { continue };
        if cross(u, v).abs() > PARALLEL {
            continue;
        }
        let o = dot(sub(g.a, at), n);
        let left = o >= -TIE;
        take(
            &mut best[usize::from(!left)],
            GridGap {
                grid: i,
                distance: if left { js_max(0.0, o) } else { -o },
                from: at,
                to: along(at, n, o),
            },
        );
    }
    best
}

/// The nearest grid parallel to each of a box's sides, from its middle `at` along the side's outward normal; the
/// sides as `(start, end)` counter-clockwise (outward is to their right).
pub fn around(
    grids: &[GridIn<'_>],
    level: Option<&str>,
    at: Vec2,
    sides: &[(Vec2, Vec2); 4],
) -> [Option<GridGap>; 4] {
    let mut best = [None; 4];
    for (k, (a, b)) in sides.iter().enumerate() {
        let Some(u) = unit(*a, *b) else { continue };
        let n = Vec2::new(u.y, -u.x);
        for (i, g) in grids.iter().enumerate() {
            if !on_level(g, level) {
                continue;
            }
            let Some(v) = unit(g.a, g.b) else { continue };
            if cross(u, v).abs() > PARALLEL {
                continue;
            }
            let o = dot(sub(g.a, at), n);
            // Through the middle: on the lower of the two parallel sides.
            if o < -TIE || (o <= TIE && k >= 2) {
                continue;
            }
            take(
                &mut best[k],
                GridGap {
                    grid: i,
                    distance: js_max(0.0, o),
                    from: at,
                    to: along(at, n, o),
                },
            );
        }
    }
    best
}

/// Where the line of `c`–`d` crosses segment `a`–`b`: the distance from `a` along it, none when parallel or off it.
fn crossing(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<f64> {
    let l = js_hypot(b.x - a.x, b.y - a.y);
    let u = unit(a, b)?;
    let v = unit(c, d)?;
    let den = cross(u, v);
    if den.abs() <= PARALLEL {
        return None;
    }
    let t = cross(sub(c, a), v) / den;
    (-TIE..=l + TIE).contains(&t).then_some(t)
}

/// The nearest grids crossing segment `a`–`b` before (at or before) and after the point `s` metres from `a`:
/// `[before, after]`.
pub fn crossing_at(
    grids: &[GridIn<'_>],
    level: Option<&str>,
    a: Vec2,
    b: Vec2,
    s: f64,
) -> [Option<GridGap>; 2] {
    let Some(u) = unit(a, b) else {
        return [None, None];
    };
    let at = along(a, u, s);
    let mut best = [None, None];
    for (i, g) in grids.iter().enumerate() {
        if !on_level(g, level) {
            continue;
        }
        let Some(t) = crossing(a, b, g.a, g.b) else { continue };
        let before = t <= s + TIE;
        take(
            &mut best[usize::from(!before)],
            GridGap {
                grid: i,
                distance: js_max(0.0, if before { s - t } else { t - s }),
                from: at,
                to: along(a, u, t),
            },
        );
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(a: (f64, f64), b: (f64, f64)) -> GridIn<'static> {
        GridIn {
            a: Vec2::new(a.0, a.1),
            b: Vec2::new(b.0, b.1),
            level: None,
        }
    }

    #[test]
    fn grids_beside_an_axis_nearest_first_and_one_on_it_to_the_left() {
        let grids = [
            g((0.0, 2.0), (9.0, 2.0)),
            g((0.0, -1.5), (9.0, -1.5)),
            g((0.0, 0.0), (9.0, 0.0)),
            g((0.0, 3.0), (1.0, 3.0)),
        ];
        let [left, right] = beside(&grids, None, Vec2::new(4.0, 0.0), Vec2::new(1.0, 0.0));
        assert_eq!(left.map(|x| (x.grid, x.distance)), Some((2, 0.0)), "on the axis: left");
        assert_eq!(right.map(|x| (x.grid, x.distance)), Some((1, 1.5)));
        let [left, _] = beside(&grids[..2], None, Vec2::new(4.0, 0.0), Vec2::new(1.0, 0.0));
        assert_eq!(left.map(|x| x.to), Some(Vec2::new(4.0, 2.0)), "endless: measured at the point");
    }

    #[test]
    fn grids_round_a_box_and_one_through_its_middle_on_the_lower_side() {
        let sides = [
            (Vec2::new(-0.2, -0.2), Vec2::new(0.2, -0.2)),
            (Vec2::new(0.2, -0.2), Vec2::new(0.2, 0.2)),
            (Vec2::new(0.2, 0.2), Vec2::new(-0.2, 0.2)),
            (Vec2::new(-0.2, 0.2), Vec2::new(-0.2, -0.2)),
        ];
        let grids = [
            g((-5.0, 0.0), (5.0, 0.0)),
            g((1.2, -5.0), (1.2, 5.0)),
            g((-0.7, -5.0), (-0.7, 5.0)),
        ];
        let got = around(&grids, None, Vec2::new(0.0, 0.0), &sides);
        assert_eq!(got[0].map(|x| (x.grid, x.distance)), Some((0, 0.0)));
        assert_eq!(got[1].map(|x| (x.grid, x.distance)), Some((1, 1.2)));
        assert!(got[2].is_none());
        assert_eq!(got[3].map(|x| x.grid), Some(2));
        assert!((got[3].unwrap().distance - 0.7).abs() < 1e-12);
    }

    #[test]
    fn grids_crossing_a_segment_before_and_after_a_point() {
        let grids = [
            g((1.0, -9.0), (1.0, 9.0)),
            g((3.0, -9.0), (3.0, -8.0)),
            g((8.0, 0.0), (9.0, 0.0)),
            g((5.0, 1.0), (5.0, 2.0)),
        ];
        let [before, after] = crossing_at(&grids, None, Vec2::new(0.0, 0.0), Vec2::new(6.0, 0.0), 3.0);
        assert_eq!(before.map(|x| (x.grid, x.distance)), Some((1, 0.0)), "at the point: before; endless line");
        assert_eq!(after.map(|x| (x.grid, x.distance)), Some((3, 2.0)));
        let [before, _] =
            crossing_at(&grids[..1], Some("zemin"), Vec2::new(0.0, 0.0), Vec2::new(6.0, 0.0), 3.0);
        assert_eq!(before.map(|x| x.distance), Some(2.0), "a grid of no level counts on every level");
    }
}
