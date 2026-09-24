//! Hatch lines clipped to a ring and its holes (`apps/web/src/model/geom/hatch.ts`):
//! even–odd rule, lines on world-anchored multiples of the spacing so
//! neighbouring hatches line up across shared boundaries.

use crate::api::Op;
use crate::jsmath::{PI, cos, js_cmp, js_max, js_min, sin, stable_sort};
use crate::op;
use crate::vec2::Vec2;

/// Upper bound on generated hatch lines; beyond this the spacing is unusable anyway.
pub const MAX_HATCH_LINES: f64 = 20_000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct HatchLines {
    pub segments: Vec<[Vec2; 2]>,
    pub capped: bool,
}

crate::json_struct!(out HatchLines { segments, capped });

#[derive(Clone, Copy)]
struct Local {
    u: f64,
    v: f64,
}

pub fn hatch_lines(ring: &[Vec2], angle_deg: f64, spacing: f64, holes: &[Vec<Vec2>]) -> HatchLines {
    let mut segments = Vec::new();
    if ring.len() < 3 || !(spacing > 0.0) {
        return HatchLines {
            segments,
            capped: false,
        };
    }
    let rad = (angle_deg * PI) / 180.0;
    let c = cos(rad);
    let s = sin(rad);
    // Local frame: u along the lines, v across them.
    let to_local = |p: &Vec2| Local {
        u: p.x * c + p.y * s,
        v: -p.x * s + p.y * c,
    };
    let to_world = |u: f64, v: f64| Vec2::new(u * c - v * s, u * s + v * c);
    let loc: Vec<Local> = ring.iter().map(to_local).collect();
    let mut rings: Vec<Vec<Local>> = vec![loc.clone()];
    rings.extend(
        holes
            .iter()
            .filter(|h| h.len() >= 3)
            .map(|h| h.iter().map(to_local).collect()),
    );
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for p in &loc {
        min_v = js_min(min_v, p.v);
        max_v = js_max(max_v, p.v);
    }
    let first = (min_v / spacing).ceil();
    let last = (max_v / spacing).floor();
    if last - first + 1.0 > MAX_HATCH_LINES {
        return HatchLines {
            segments,
            capped: true,
        };
    }
    let mut xs: Vec<f64> = Vec::new();
    let mut k = first;
    while k <= last {
        let v = k * spacing;
        xs.clear();
        for r in &rings {
            let mut j = r.len() - 1;
            for i in 0..r.len() {
                let a = r[j];
                let b = r[i];
                // Half-open rule so a line through a vertex is counted once.
                if (a.v <= v) != (b.v <= v) {
                    xs.push(a.u + ((v - a.v) / (b.v - a.v)) * (b.u - a.u));
                }
                j = i;
            }
        }
        stable_sort(&mut xs, &mut |p, q| js_cmp(*p - *q, 0.0));
        let mut i = 0;
        while i + 1 < xs.len() {
            if xs[i + 1] - xs[i] > 1e-9 {
                segments.push([to_world(xs[i], v), to_world(xs[i + 1], v)]);
            }
            i += 2;
        }
        k += 1.0;
    }
    HatchLines {
        segments,
        capped: false,
    }
}

pub(crate) static OPS: &[Op] = &[op!(
    "hatchLines",
    |ring: Vec<Vec2>, angle_deg: f64, spacing: f64, holes: Option<Vec<Vec<Vec2>>>| hatch_lines(
        &ring,
        angle_deg,
        spacing,
        &holes.unwrap_or_default()
    )
)];
