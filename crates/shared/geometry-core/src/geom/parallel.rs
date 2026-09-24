//! Netcad "Paralel çizgi" (`apps/web/src/model/geom/parallel.ts`): the two sides of an
//! axis at a left and a right distance, and the corridor between them as an area.

use crate::api::Op;
use crate::api::json::{Nullable, ToJson, field};
use crate::geom::arrangement::{Area, Ring};
use crate::geom::offset::offset_path;
use crate::geometry::signed_area;
use crate::jsmath::js_hypot;
use crate::op;
use crate::vec2::Vec2;

/// Axis without repeated points (a double click must not make a zero-length leg).
pub fn clean_axis(axis: &[Vec2], closed: bool) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::new();
    for &p in axis {
        match out.last() {
            Some(last) if js_hypot(p.x - last.x, p.y - last.y) <= 1e-9 => {}
            _ => out.push(p),
        }
    }
    if closed && out.len() > 1 {
        let (f, l) = (out[0], out[out.len() - 1]);
        if js_hypot(f.x - l.x, f.y - l.y) <= 1e-9 {
            out.pop();
        }
    }
    out
}

/// The two sides; a side at distance 0 is the axis itself (null).
pub struct Sides {
    pub left: Option<Vec<Vec2>>,
    pub right: Option<Vec<Vec2>>,
}

impl ToJson for Sides {
    fn write_json(&self, out: &mut String) {
        // Both keys are always there, `null` when missing (as the TypeScript returns).
        out.push('{');
        let mut first = true;
        field(out, &mut first, "left", &Nullable(&self.left));
        field(out, &mut first, "right", &Nullable(&self.right));
        out.push('}');
    }
}

pub fn parallel_sides(axis: &[Vec2], left: f64, right: f64, closed: bool) -> Sides {
    let pts = clean_axis(axis, closed);
    if pts.len() < if closed { 3 } else { 2 } {
        return Sides {
            left: None,
            right: None,
        };
    }
    Sides {
        left: (left > 0.0).then(|| offset_path(&pts, left, closed)),
        right: (right > 0.0).then(|| offset_path(&pts, -right, closed)),
    }
}

/// The corridor between the sides as an area: one ring for an open axis
/// (left side out, right side back), a ring with a hole for a closed one.
pub fn corridor_area(axis: &[Vec2], left: f64, right: f64, closed: bool) -> Option<Area> {
    if !(left + right > 0.0) {
        return None;
    }
    let pts = clean_axis(axis, closed);
    if pts.len() < if closed { 3 } else { 2 } {
        return None;
    }
    let l = if left > 0.0 {
        offset_path(&pts, left, closed)
    } else {
        pts.clone()
    };
    let r = if right > 0.0 {
        offset_path(&pts, -right, closed)
    } else {
        pts.clone()
    };
    let ring = |pts: Vec<Vec2>| Ring { pts, bulges: None };
    if !closed {
        let mut ring_pts = l;
        ring_pts.extend(r.into_iter().rev());
        if signed_area(&ring_pts) < 0.0 {
            ring_pts.reverse();
        }
        return Some(Area {
            outer: ring(ring_pts),
            holes: Vec::new(),
        });
    }
    // A closed axis: the larger ring is the outline, the smaller one the hole.
    let (outer, hole) = if signed_area(&l).abs() >= signed_area(&r).abs() {
        (l, r)
    } else {
        (r, l)
    };
    let ccw = |mut ring: Vec<Vec2>, want: bool| {
        if (signed_area(&ring) > 0.0) != want {
            ring.reverse();
        }
        ring
    };
    Some(Area {
        outer: ring(ccw(outer, true)),
        holes: vec![ring(ccw(hole, false))],
    })
}

pub(crate) static OPS: &[Op] = &[
    op!("cleanAxis", |axis: Vec<Vec2>, closed: bool| clean_axis(
        &axis, closed
    )),
    op!("parallelSides", |axis: Vec<Vec2>,
                          left: f64,
                          right: f64,
                          closed: bool| {
        parallel_sides(&axis, left, right, closed)
    }),
    op!("corridorArea", |axis: Vec<Vec2>,
                         left: f64,
                         right: f64,
                         closed: bool| {
        corridor_area(&axis, left, right, closed)
    }),
];
