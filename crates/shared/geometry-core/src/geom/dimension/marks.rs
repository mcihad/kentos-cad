//! The marks of docs/adr/0322: Merkez işareti (a curve's centre cross and its axis lines), Eksen çizgisi (the axis
//! between two straight edges) and Kot (a point's elevation on a level mark). They are dimensions so that their
//! points follow their objects as every dimension's do (docs/adr/0320): a centre mark by the curve rule, an axis by
//! the pair rule, a level's point by its place. Lengths are in the value's height h. The independent reference is
//! `scripts/fixtures/dimension_mark_cases.py`.

use super::{DimensionGeom, DimensionLayout, add, lift, text_along};
use crate::geom::intersect::Edge;
use crate::jsmath::{PI, atan2, cos, js_hypot, sin};
use crate::vec2::Vec2;

/// The cross's arms reach h/2 from the centre; the axis lines start h/4 past them.
const CROSS_GAP: f64 = 0.25;

/// The level mark's triangle: its sides h long, its apex on the point.
const LEVEL_SIDE: f64 = 1.0;

/// A level's value: each character 0.6h wide (`value_width`'s estimate), its sign one more.
const LEVEL_CHAR: f64 = 0.6;

/// A level's decimals when its look gives none.
pub const LEVEL_DECIMALS: u32 = 2;

fn segs(lines: &[[Vec2; 2]]) -> Vec<Edge> {
    lines.iter().map(|&[a, b]| Edge::Seg { a, b }).collect()
}

fn mid(p: Vec2, q: Vec2) -> Vec2 {
    Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0)
}

fn neg(v: Vec2) -> Vec2 {
    Vec2::new(-v.x, -v.y)
}

/// The cross at the centre `a` of the curve through `b`: two arms h long, turned `angle` degrees from a→b; four
/// axis lines from h/4 past the arms to `offset` past the curve, when they reach that far. The arms are the
/// dimension's lines, the axis lines its extension lines (the extension line type: a centre line's dash-dot).
pub(super) fn center_mark(d: &DimensionGeom) -> Option<DimensionLayout> {
    let (a, b, h) = (d.a, d.b, d.height);
    let r = js_hypot(b.x - a.x, b.y - a.y);
    if r < 1e-9 {
        return None;
    }
    let t = atan2(b.y - a.y, b.x - a.x) + d.angle.unwrap_or(0.0) * PI / 180.0;
    let u = Vec2::new(cos(t), sin(t));
    let n = Vec2::new(-u.y, u.x);
    let arm = h / 2.0;
    let mut lines = vec![
        [add(a, u, -arm), add(a, u, arm)],
        [add(a, n, -arm), add(a, n, arm)],
    ];
    let mut ext = Vec::new();
    let from = arm + CROSS_GAP * h;
    let reach = r + d.offset;
    if reach > from {
        for dir in [u, n, neg(u), neg(n)] {
            ext.push(lines.len());
            lines.push([add(a, dir, from), add(a, dir, reach)]);
        }
    }
    Some(DimensionLayout {
        pick: segs(&lines),
        lines,
        d1: a,
        d2: b,
        text_at: a,
        rotation: 0.0,
        value: r,
        unit: "none",
        prefix: "",
        // Its grip: the first axis line's end, `offset` past the curve.
        handle: add(a, u, if reach > from { reach } else { r }),
        fills: None,
        ext,
    })
}

/// The axis from `a` to `b`, `offset` past each end: one extension line.
pub(super) fn center_line(d: &DimensionGeom) -> Option<DimensionLayout> {
    let (a, b) = (d.a, d.b);
    let l = js_hypot(b.x - a.x, b.y - a.y);
    if l < 1e-9 {
        return None;
    }
    let u = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
    let (s, e) = (add(a, u, -d.offset), add(b, u, d.offset));
    let lines = vec![[s, e]];
    Some(DimensionLayout {
        pick: segs(&lines),
        lines,
        d1: s,
        d2: e,
        text_at: mid(a, b),
        rotation: 0.0,
        value: l,
        unit: "none",
        prefix: "",
        // Its grip: the end past `b`.
        handle: e,
        fills: None,
        ext: vec![0],
    })
}

/// How many characters a level's value has: its sign and `decimals` digits after the point of |z|.
fn level_chars(z: f64, decimals: u32) -> usize {
    crate::display::fixed(z.abs(), decimals as usize)
        .chars()
        .count()
        + 1
}

/// The level mark: a triangle h a side, its apex on `a` and its top across the up of `angle` degrees; the shoulder
/// from `b` (from the top's corner on `b`'s side when `b` is the point itself) as long as the value, toward `b`'s side,
/// a leader from the top's middle to `b` when `b` is away from the triangle; the value over the shoulder.
pub(super) fn elevation(d: &DimensionGeom) -> Option<DimensionLayout> {
    let (a, b, h) = (d.a, d.b, d.height);
    let z = d.za?;
    let turn = d.angle.unwrap_or(0.0) * PI / 180.0;
    let u = Vec2::new(cos(turn), sin(turn));
    let v = Vec2::new(-u.y, u.x);
    let side = LEVEL_SIDE * h;
    let top = add(a, v, side * 3f64.sqrt() / 2.0);
    let (left, right) = (add(top, u, -side / 2.0), add(top, u, side / 2.0));
    let mut lines = vec![[a, left], [left, right], [right, a]];
    let away = Vec2::new(b.x - a.x, b.y - a.y);
    let toward = if away.x * u.x + away.y * u.y < 0.0 {
        neg(u)
    } else {
        u
    };
    let at_point = js_hypot(away.x, away.y) < 1e-9;
    let start = if at_point {
        add(top, toward, side / 2.0)
    } else {
        b
    };
    if !at_point {
        lines.push([top, start]);
    }
    let decimals = d.look.decimals.unwrap_or(LEVEL_DECIMALS);
    let width = LEVEL_CHAR * h * level_chars(z, decimals) as f64 + side / 2.0;
    let end = add(start, toward, width);
    lines.push([start, end]);
    let (text_at, rotation) = text_along(mid(start, end), u, lift(d));
    Some(DimensionLayout {
        pick: segs(&lines),
        lines,
        d1: a,
        d2: end,
        text_at,
        rotation,
        value: z,
        unit: "level",
        prefix: "",
        handle: start,
        fills: None,
        ext: Vec::new(),
    })
}
