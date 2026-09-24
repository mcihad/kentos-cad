//! Dimension layout (`apps/web/src/model/geom/dimension.ts`): extension lines with a
//! gap, the dimension line or arc with oblique ticks, the value readable
//! left to right. `dimensionLabel` (formatting) stays in TypeScript.

use crate::api::Op;
use crate::geom::arc::norm_angle;
use crate::geom::intersect::Edge;
use crate::jsmath::{PI, atan2, cos, js_hypot, js_max, js_max_all, js_min, or, sin};
use crate::op;
use crate::vec2::Vec2;

const SQRT1_2: f64 = std::f64::consts::FRAC_1_SQRT_2;

#[derive(Clone, Debug, PartialEq)]
pub struct DimensionGeom {
    pub a: Vec2,
    pub b: Vec2,
    pub offset: f64,
    pub height: f64,
    pub style: Option<String>,
    pub angle: Option<f64>,
    pub c: Option<Vec2>,
}

crate::json_struct!(DimensionGeom {
    a,
    b,
    offset,
    height,
    style,
    angle,
    c
});

#[derive(Clone, Debug, PartialEq)]
pub struct DimensionLayout {
    pub lines: Vec<[Vec2; 2]>,
    pub d1: Vec2,
    pub d2: Vec2,
    pub text_at: Vec2,
    pub rotation: f64,
    pub value: f64,
    pub unit: &'static str,
    pub prefix: &'static str,
    pub pick: Vec<Edge>,
    pub handle: Vec2,
}

crate::json_struct!(out DimensionLayout { lines, d1, d2, text_at => "textAt", rotation, value, unit, prefix, pick, handle });

fn add(p: Vec2, v: Vec2, k: f64) -> Vec2 {
    Vec2::new(p.x + v.x * k, p.y + v.y * k)
}
fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// Oblique (45°) ticks centred on a point of a line running along u.
fn tick(lines: &mut Vec<[Vec2; 2]>, p: Vec2, u: Vec2, size: f64) {
    let t = Vec2::new(
        ((u.x - u.y) * SQRT1_2 * size) / 2.0,
        ((u.y + u.x) * SQRT1_2 * size) / 2.0,
    );
    lines.push([
        Vec2::new(p.x - t.x, p.y - t.y),
        Vec2::new(p.x + t.x, p.y + t.y),
    ]);
}

/// Readable text along u through `mid`, lifted to the reader's "above".
fn text_along(mid: Vec2, u: Vec2, height: f64) -> (Vec2, f64) {
    let mut rotation = (atan2(u.y, u.x) * 180.0) / PI;
    let mut side = 1.0;
    if rotation > 90.0 || rotation <= -90.0 {
        rotation += if rotation > 0.0 { -180.0 } else { 180.0 };
        side = -1.0; // flipped reading direction: "above" is the other normal
    }
    (
        add(mid, Vec2::new(-u.y, u.x), side * height * 0.35),
        rotation,
    )
}

/// Extension line from a measured point towards (and a little past) the dimension line.
fn extension(lines: &mut Vec<[Vec2; 2]>, from: Vec2, to: Vec2, gap: f64, ext: f64) {
    let l = js_hypot(to.x - from.x, to.y - from.y);
    if l <= gap {
        return;
    }
    let v = Vec2::new((to.x - from.x) / l, (to.y - from.y) / l);
    lines.push([add(from, v, gap), add(to, v, ext)]);
}

pub fn layout_dimension(d: &DimensionGeom) -> Option<DimensionLayout> {
    match d.style.as_deref().unwrap_or("aligned") {
        "linear" => linear(d, d.angle.unwrap_or(0.0)),
        "angular" => angular(d),
        "radius" => radial(d, false),
        "diameter" => radial(d, true),
        _ => aligned(d),
    }
}

fn aligned(d: &DimensionGeom) -> Option<DimensionLayout> {
    let dx = d.b.x - d.a.x;
    let dy = d.b.y - d.a.y;
    let length = js_hypot(dx, dy);
    if length < 1e-9 {
        return None;
    }
    straight(d, Vec2::new(dx / length, dy / length), d.offset)
}

fn linear(d: &DimensionGeom, angle_deg: f64) -> Option<DimensionLayout> {
    let t = (angle_deg * PI) / 180.0;
    straight(d, Vec2::new(cos(t), sin(t)), d.offset)
}

/// A distance measured along u: the dimension line runs parallel to u at `offset` left of a.
fn straight(d: &DimensionGeom, u: Vec2, offset: f64) -> Option<DimensionLayout> {
    let n = Vec2::new(-u.y, u.x);
    let value = dot(Vec2::new(d.b.x - d.a.x, d.b.y - d.a.y), u).abs();
    if value < 1e-9 {
        return None;
    }
    let d1 = add(d.a, n, offset);
    let d2 = add(
        d.b,
        n,
        offset + dot(n, Vec2::new(d.a.x - d.b.x, d.a.y - d.b.y)),
    );
    let gap = d.height * 0.5;
    let mut lines = Vec::new();
    extension(&mut lines, d.a, d1, gap, d.height * 0.5);
    extension(&mut lines, d.b, d2, gap, d.height * 0.5);
    lines.push([d1, d2]);
    let l = js_hypot(d2.x - d1.x, d2.y - d1.y);
    let along = Vec2::new((d2.x - d1.x) / l, (d2.y - d1.y) / l);
    tick(&mut lines, d1, along, d.height * 0.6);
    tick(&mut lines, d2, along, d.height * 0.6);
    let mid = Vec2::new((d1.x + d2.x) / 2.0, (d1.y + d2.y) / 2.0);
    let (text_at, rotation) = text_along(mid, along, d.height);
    Some(DimensionLayout {
        lines,
        d1,
        d2,
        text_at,
        rotation,
        value,
        unit: "length",
        prefix: "",
        pick: vec![Edge::Seg { a: d1, b: d2 }],
        handle: mid,
    })
}

fn at2(c: Vec2, t: f64, r: f64) -> Vec2 {
    Vec2::new(c.x + cos(t) * r, c.y + sin(t) * r)
}

fn angular(d: &DimensionGeom) -> Option<DimensionLayout> {
    let c = d.c?;
    let r = d.offset.abs();
    if r < 1e-9 {
        return None;
    }
    let t0 = atan2(d.a.y - c.y, d.a.x - c.x);
    let sweep = norm_angle(atan2(d.b.y - c.y, d.b.x - c.x) - t0);
    if sweep < 1e-9 {
        return None;
    }
    let at = |t: f64| Vec2::new(c.x + cos(t) * r, c.y + sin(t) * r);
    let mut lines = Vec::new();
    let gap = d.height * 0.5;
    // Arms are extended to the arc when it lies beyond the measured points.
    for (p, t) in [(d.a, t0), (d.b, t0 + sweep)] {
        let rp = js_hypot(p.x - c.x, p.y - c.y);
        if r > rp + gap {
            lines.push([at2(c, t, rp + gap), at2(c, t, r + d.height * 0.5)]);
        }
    }
    let steps = js_max(8.0, (sweep / (PI / 36.0)).ceil());
    let mut i = 0.0;
    while i < steps {
        lines.push([
            at(t0 + (sweep * i) / steps),
            at(t0 + (sweep * (i + 1.0)) / steps),
        ]);
        i += 1.0;
    }
    let d1 = at(t0);
    let d2 = at(t0 + sweep);
    let tangent = |t: f64| Vec2::new(-sin(t), cos(t));
    tick(&mut lines, d1, tangent(t0), d.height * 0.6);
    tick(&mut lines, d2, tangent(t0 + sweep), d.height * 0.6);
    let tm = t0 + sweep / 2.0;
    let mid = at(tm);
    let (text_at, rotation) = text_along(mid, tangent(tm), d.height);
    Some(DimensionLayout {
        lines,
        d1,
        d2,
        text_at,
        rotation,
        value: sweep,
        unit: "angle",
        prefix: "",
        pick: vec![Edge::Arc {
            c,
            r,
            a0: t0,
            sweep,
        }],
        handle: mid,
    })
}

fn radial(d: &DimensionGeom, diameter: bool) -> Option<DimensionLayout> {
    let c = d.a;
    let r = js_hypot(d.b.x - c.x, d.b.y - c.y);
    if r < 1e-9 {
        return None;
    }
    let u = Vec2::new((d.b.x - c.x) / r, (d.b.y - c.y) / r);
    let leader = js_max(0.0, d.offset);
    let end = add(d.b, u, leader);
    let start = if diameter { add(c, u, -r) } else { c };
    let mut lines = vec![[start, end]];
    tick(&mut lines, d.b, u, d.height * 0.6);
    if diameter {
        tick(&mut lines, start, u, d.height * 0.6);
    }
    // The value sits on the leader when there is one, else half-way from the centre to the circle.
    let (p0, p1) = if leader > d.height {
        (d.b, end)
    } else {
        (c, d.b)
    };
    let mid = Vec2::new((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0);
    let (text_at, rotation) = text_along(mid, u, d.height);
    Some(DimensionLayout {
        lines,
        d1: start,
        d2: end,
        text_at,
        rotation,
        value: if diameter { 2.0 * r } else { r },
        unit: "length",
        prefix: if diameter { "Ø " } else { "R " },
        pick: vec![Edge::Seg { a: start, b: end }],
        handle: end,
    })
}

/// Signed perpendicular distance of p from the line a→b (positive = left).
pub fn signed_offset(a: Vec2, b: Vec2, p: Vec2) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let l = or(js_hypot(dx, dy), 1.0);
    (dx * (p.y - a.y) - dy * (p.x - a.x)) / l
}

/// The `offset` that puts the dimension line (arc, leader end) through p.
pub fn dimension_offset_at(d: &DimensionGeom, p: Vec2) -> f64 {
    match d.style.as_deref().unwrap_or("aligned") {
        "linear" => {
            let t = (d.angle.unwrap_or(0.0) * PI) / 180.0;
            -sin(t) * (p.x - d.a.x) + cos(t) * (p.y - d.a.y)
        }
        "angular" => match d.c {
            Some(c) => js_hypot(p.x - c.x, p.y - c.y),
            None => d.offset,
        },
        "radius" | "diameter" => js_max(
            0.0,
            js_hypot(p.x - d.a.x, p.y - d.a.y) - js_hypot(d.b.x - d.a.x, d.b.y - d.a.y),
        ),
        _ => signed_offset(d.a, d.b, p),
    }
}

/// Horizontal (ΔY, 0°) or vertical (ΔX, 90°) for a linear dimension placed at p (AutoCAD's rule).
pub fn linear_angle_for(a: Vec2, b: Vec2, p: Vec2) -> f64 {
    let out_x = js_max_all([js_min(a.x, b.x) - p.x, p.x - js_max(a.x, b.x), 0.0]);
    let out_y = js_max_all([js_min(a.y, b.y) - p.y, p.y - js_max(a.y, b.y), 0.0]);
    if out_x > out_y { 90.0 } else { 0.0 }
}

/// The two arm directions bounding the sector around `p` (counter-clockwise order) from the vertex `c`.
pub fn sector_arms(c: Vec2, u1: Vec2, u2: Vec2, p: Vec2) -> [Vec2; 2] {
    let dirs =
        [u1, u2, Vec2::new(-u1.x, -u1.y), Vec2::new(-u2.x, -u2.y)].map(|u| (u, atan2(u.y, u.x)));
    let tp = atan2(p.y - c.y, p.x - c.x);
    let mut best = [u1, u2];
    let mut best_sweep = f64::INFINITY;
    // The arm just clockwise of p starts the sector, the one just counter-clockwise ends it.
    for (si, s) in dirs.iter().enumerate() {
        let back = norm_angle(tp - s.1);
        for (ei, e) in dirs.iter().enumerate() {
            if ei == si {
                continue;
            }
            let sweep = norm_angle(e.1 - s.1);
            if sweep > 1e-9 && back <= sweep && sweep < best_sweep {
                best_sweep = sweep;
                best = [s.0, e.0];
            }
        }
    }
    best
}

pub(crate) static OPS: &[Op] = &[
    op!("layoutDimension", |d: DimensionGeom| layout_dimension(&d)),
    op!("signedOffset", |a: Vec2, b: Vec2, p: Vec2| signed_offset(
        a, b, p
    )),
    op!("dimensionOffsetAt", |d: DimensionGeom, p: Vec2| {
        dimension_offset_at(&d, p)
    }),
    op!("linearAngleFor", |a: Vec2, b: Vec2, p: Vec2| {
        linear_angle_for(a, b, p)
    }),
    op!("sectorArms", |c: Vec2, u1: Vec2, u2: Vec2, p: Vec2| {
        sector_arms(c, u1, u2, p)
    }),
];
