//! The dimensions of docs/adr/0147 §2: ordinate (Koordinat), arc length (Yay
//! uzunluğu), jogged radius (Kırıklı yarıçap), azimuth (Semt) and slope
//! (Eğim). Every length is in the value's height h, as the older kinds':
//! the gap h/2, the ticks 0.6h, the value 0.35h above its line. An arc
//! length's and an arrow's value keeps off what it measures: when its
//! "above" faces it, it goes as far under its line. The independent
//! reference is `scripts/fixtures/dimension_cases.py`
//! (fixtures/dimension/v1/layout.json).

use super::{DimensionGeom, DimensionLayout, add, at2, dimension_measure, dot, text_along, tick};
use crate::geom::arc::norm_angle;
use crate::geom::intersect::Edge;
use crate::jsmath::{PI, TAU, atan2, cos, js_hypot, js_max, js_min, sin};
use crate::vec2::Vec2;

fn mid(p: Vec2, q: Vec2) -> Vec2 {
    Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0)
}

fn segs(lines: &[[Vec2; 2]]) -> Vec<Edge> {
    lines.iter().map(|&[a, b]| Edge::Seg { a, b }).collect()
}

/// A value's reading direction and its up, from its turn in degrees.
fn frame(rotation: f64) -> (Vec2, Vec2) {
    let turn = (rotation * PI) / 180.0;
    (
        Vec2::new(cos(turn), sin(turn)),
        Vec2::new(-sin(turn), cos(turn)),
    )
}

/// The value at `at` (over its line through m), or under the line when its
/// up faces from `away`: its box (`top` over its baseline, 0.23h under it)
/// as far under the line as it is over it, its top 0.12h away.
fn away_from(m: Vec2, at: Vec2, rotation: f64, away: Vec2, h: f64, top: f64) -> Vec2 {
    let (_, up) = frame(rotation);
    if dot(up, away) >= 0.0 {
        at
    } else {
        add(m, up, -(0.12 * h + top))
    }
}

/// A point's Y (`angle` 0 or none) or X (90): the line runs across the
/// measured axis towards b, starting h/2 from the point; when b is off the
/// point's line and there is room it goes h straight, h across and on to b
/// (AutoCAD's jog). The value is on the last part.
pub(super) fn ordinate(d: &DimensionGeom) -> Option<DimensionLayout> {
    let (a, b, h) = (d.a, d.b, d.height);
    let g = h / 2.0;
    let (unit, prefix) = dimension_measure(Some("ordinate"), d.angle);
    let y = prefix == "Y=";
    // e: the measured axis; n: the line's.
    let (e, n) = if y {
        (Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0))
    } else {
        (Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0))
    };
    let ab = Vec2::new(b.x - a.x, b.y - a.y);
    let along = dot(ab, n);
    let u = if along >= 0.0 { n } else { Vec2::new(-n.x, -n.y) };
    let (l, s) = (along.abs(), dot(ab, e));
    if l <= g {
        return None;
    }
    let start = add(a, u, g);
    let (lines, tail) = if s.abs() > 1e-9 && l >= g + 2.0 * h {
        let j1 = add(a, u, g + h);
        let j2 = add(add(a, e, s), u, g + 2.0 * h);
        (vec![[start, j1], [j1, j2], [j2, b]], (j2, b))
    } else {
        (vec![[start, b]], (start, b))
    };
    let t = js_hypot(tail.1.x - tail.0.x, tail.1.y - tail.0.y);
    let v = if t > 0.0 {
        Vec2::new((tail.1.x - tail.0.x) / t, (tail.1.y - tail.0.y) / t)
    } else {
        u
    };
    let (text_at, rotation) = text_along(mid(tail.0, tail.1), v, h);
    Some(DimensionLayout {
        pick: segs(&lines),
        lines,
        d1: start,
        d2: b,
        text_at,
        rotation,
        value: if y { a.x } else { a.y },
        unit,
        prefix,
        handle: b,
    })
}

/// The length of the arc about c from a, counter-clockwise to b's
/// direction: the dimension arc `offset` out from it (in when negative),
/// its ends' radial extension lines when it is more than h/2 away, the
/// value along it as an angle's, and above the value the arc symbol, half a
/// circle 0.3h across drawn as lines.
pub(super) fn arc_length(d: &DimensionGeom) -> Option<DimensionLayout> {
    let (a, b, h) = (d.a, d.b, d.height);
    let c = d.c?;
    let r = js_hypot(a.x - c.x, a.y - c.y);
    if r < 1e-9 {
        return None;
    }
    let t0 = atan2(a.y - c.y, a.x - c.x);
    let sweep = norm_angle(atan2(b.y - c.y, b.x - c.x) - t0);
    let big = r + d.offset;
    if sweep < 1e-9 || big < 1e-9 {
        return None;
    }
    let g = h / 2.0;
    let k = if d.offset >= 0.0 { 1.0 } else { -1.0 };
    let mut lines = Vec::new();
    if d.offset.abs() > g {
        for t in [t0, t0 + sweep] {
            lines.push([at2(c, t, r + k * g), at2(c, t, big + k * g)]);
        }
    }
    let steps = js_max(8.0, (sweep / (PI / 36.0)).ceil());
    let mut i = 0.0;
    while i < steps {
        lines.push([
            at2(c, t0 + (sweep * i) / steps, big),
            at2(c, t0 + (sweep * (i + 1.0)) / steps, big),
        ]);
        i += 1.0;
    }
    let tangent = |t: f64| Vec2::new(-sin(t), cos(t));
    let (d1, d2) = (at2(c, t0, big), at2(c, t0 + sweep, big));
    tick(&mut lines, d1, tangent(t0), h * 0.6);
    tick(&mut lines, d2, tangent(t0 + sweep), h * 0.6);
    let tm = t0 + sweep / 2.0;
    let handle = at2(c, tm, big);
    let (text_at, rotation) = text_along(handle, tangent(tm), h);
    // Off the arc: outwards for a dimension arc outside it, inwards for one inside.
    let outwards = Vec2::new(
        ((handle.x - c.x) / big) * k,
        ((handle.y - c.y) / big) * k,
    );
    let text_at = away_from(handle, text_at, rotation, outwards, h, 1.6 * h);
    // The symbol in the value's own frame, clear of its mask (1.25h over its baseline).
    let (along, up) = frame(rotation);
    let centre = add(text_at, up, 1.3 * h);
    let rho = 0.3 * h;
    let symbol = |phi: f64| add(add(centre, along, rho * cos(phi)), up, rho * sin(phi));
    for i in 0..12 {
        let i = i as f64;
        lines.push([symbol((PI * i) / 12.0), symbol((PI * (i + 1.0)) / 12.0)]);
    }
    let (unit, prefix) = dimension_measure(Some("arcLength"), None);
    Some(DimensionLayout {
        lines,
        d1,
        d2,
        text_at,
        rotation,
        value: r * sweep,
        unit,
        prefix,
        pick: vec![Edge::Arc {
            c,
            r: big,
            a0: t0,
            sweep,
        }],
        handle,
    })
}

/// The jog's distance from the centre shown, along the radius, that puts it
/// nearest p: within where a jog fits (`None` when none does).
pub(super) fn jog_at(d: &DimensionGeom, p: Vec2) -> Option<f64> {
    let (u, s, t) = jogged_frame(d)?;
    let c = d.c?;
    let along = dot(Vec2::new(p.x - c.x, p.y - c.y), u);
    Some(js_max(0.0, js_min(along, t - s.abs())))
}

/// The radius's direction u from the true centre a to b on the arc, and
/// where the centre shown c is: s across the radius (left positive), t
/// before b along it; none when there is no jog to draw (c not before b, or
/// further across than before it).
fn jogged_frame(d: &DimensionGeom) -> Option<(Vec2, f64, f64)> {
    let (a, b) = (d.a, d.b);
    let c = d.c?;
    let r = js_hypot(b.x - a.x, b.y - a.y);
    if r < 1e-9 {
        return None;
    }
    let u = Vec2::new((b.x - a.x) / r, (b.y - a.y) / r);
    let n = Vec2::new(-u.y, u.x);
    let s = dot(Vec2::new(c.x - b.x, c.y - b.y), n);
    let t = dot(Vec2::new(b.x - c.x, b.y - c.y), u);
    if t <= 1e-9 || s.abs() > t {
        return None;
    }
    Some((u, s, t))
}

/// A radius whose centre is too far to draw: from the centre shown c along
/// the radius for `offset` (kept where a jog fits), across at 45° onto b's
/// radius, on to b with a tick. The value is on the part reaching the arc
/// when it is 3h long, else on the first.
pub(super) fn jogged(d: &DimensionGeom) -> Option<DimensionLayout> {
    let (a, b, h) = (d.a, d.b, d.height);
    let c = d.c?;
    let (u, s, t) = jogged_frame(d)?;
    let n = Vec2::new(-u.y, u.x);
    let jog = s.abs();
    let ta = js_min(js_max(d.offset, 0.0), t - jog);
    let p1 = add(c, u, ta);
    let p2 = add(add(p1, u, jog), n, -s);
    let mut lines = Vec::new();
    if ta > 1e-9 {
        lines.push([c, p1]);
    }
    if jog > 1e-9 {
        lines.push([p1, p2]);
    }
    let last = js_hypot(b.x - p2.x, b.y - p2.y);
    if last > 1e-9 {
        lines.push([p2, b]);
    }
    let pick = segs(&lines);
    tick(&mut lines, b, u, h * 0.6);
    let part = if last >= 3.0 * h { (p2, b) } else { (c, p1) };
    let (text_at, rotation) = text_along(mid(part.0, part.1), u, h);
    let (unit, prefix) = dimension_measure(Some("jogged"), None);
    Some(DimensionLayout {
        lines,
        d1: c,
        d2: b,
        text_at,
        rotation,
        value: js_hypot(b.x - a.x, b.y - a.y),
        unit,
        prefix,
        pick,
        handle: p1,
    })
}

/// An edge's azimuth (the bearing from north, clockwise) or the slope
/// between its two elevations: an arrow 3h long beside the edge's middle,
/// `offset` to its left, an open head; a slope's points downhill and has no
/// head on the level. The value is above the arrow.
pub(super) fn arrowed(d: &DimensionGeom, slope: bool) -> Option<DimensionLayout> {
    let (a, b, h) = (d.a, d.b, d.height);
    let dd = Vec2::new(b.x - a.x, b.y - a.y);
    let length = js_hypot(dd.x, dd.y);
    if length < 1e-9 {
        return None;
    }
    let u = Vec2::new(dd.x / length, dd.y / length);
    let n = Vec2::new(-u.y, u.x);
    let m = add(mid(a, b), n, d.offset);
    let (mut w, mut head) = (u, true);
    let value = if slope {
        let (za, zb) = (d.za?, d.zb?);
        if zb > za {
            w = Vec2::new(-u.x, -u.y);
        }
        head = za != zb;
        ((zb - za).abs() / length) * 100.0
    } else {
        let t = atan2(dd.x, dd.y);
        if t < 0.0 { t + TAU } else { t }
    };
    let (tail, tip) = (add(m, w, -1.5 * h), add(m, w, 1.5 * h));
    let wn = Vec2::new(-w.y, w.x);
    let mut lines = vec![[tail, tip]];
    if head {
        let back = add(tip, w, -0.5 * h);
        lines.push([tip, add(back, wn, 0.2 * h)]);
        lines.push([tip, add(back, wn, -0.2 * h)]);
    }
    let (mut text_at, rotation) = text_along(m, u, h);
    // Off the edge: on the arrow's side of it (over the arrow when it is on the edge).
    if d.offset != 0.0 {
        let side = if d.offset > 0.0 { 1.0 } else { -1.0 };
        text_at = away_from(
            m,
            text_at,
            rotation,
            Vec2::new(n.x * side, n.y * side),
            h,
            1.15 * h,
        );
    }
    let (unit, prefix) = dimension_measure(Some(if slope { "slope" } else { "azimuth" }), None);
    Some(DimensionLayout {
        lines,
        d1: tail,
        d2: tip,
        text_at,
        rotation,
        value,
        unit,
        prefix,
        pick: vec![Edge::Seg { a: tail, b: tip }],
        handle: m,
    })
}
