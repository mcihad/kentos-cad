//! The drawing's marker shapes (the web's `canvasShapes.ts`), y up: a
//! named shape as a tiny-skia path (what the renderer's atlas fills and
//! strokes) and as polylines (what a PDF writes, docs/adr/0252). Pure: no
//! GPU, no window; the renderer and the plotter share it.

use tiny_skia::{Path, PathBuilder};

/// Shapes drawn as lines only (their fill colour, if any, strokes them).
pub const OPEN: [&str; 6] = ["cross", "x", "line", "arrow", "chevron", "arc"];

/// An arc as cubic pieces, counter-clockwise from `a0` to `a1` (radians), y up.
pub fn arc(b: &mut PathBuilder, r: f32, a0: f32, a1: f32, move_first: bool) {
    let sweep = a1 - a0;
    let n = ((sweep.abs() / (std::f32::consts::PI / 2.0)).ceil() as usize).max(1);
    let step = sweep / n as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let (mut x, mut y) = (r * a0.cos(), r * a0.sin());
    if move_first {
        b.move_to(x, y);
    } else {
        b.line_to(x, y);
    }
    for i in 0..n {
        let t0 = a0 + step * i as f32;
        let t1 = t0 + step;
        let (x1, y1) = (r * t1.cos(), r * t1.sin());
        b.cubic_to(
            x - k * r * t0.sin(),
            y + k * r * t0.cos(),
            x1 + k * r * t1.sin(),
            y1 - k * r * t1.cos(),
            x1,
            y1,
        );
        x = x1;
        y = y1;
    }
}

fn polygon(b: &mut PathBuilder, pts: &[(f32, f32)]) {
    for (i, (x, y)) in pts.iter().enumerate() {
        if i == 0 {
            b.move_to(*x, *y);
        } else {
            b.line_to(*x, *y);
        }
    }
    b.close();
}

fn regular(n: usize, r: f32, rot: f32) -> Vec<(f32, f32)> {
    (0..n)
        .map(|i| {
            let a = rot + i as f32 * 2.0 * std::f32::consts::PI / n as f32;
            (a.cos() * r, a.sin() * r)
        })
        .collect()
}

/// A gear: a body circle with square teeth, one centred on +x (the shaders' field).
fn gear(b: &mut PathBuilder, r: f32, teeth: f32, depth: f32) {
    let n = (teeth.round() as usize).max(3);
    let rb = r * (1.0 - depth);
    let a = 2.0 * std::f32::consts::PI / n as f32;
    let hw = 0.25 * a * (rb + (r - rb) / 2.0);
    let foot = (rb * rb - hw * hw).max(0.0).sqrt();
    let side = hw.atan2(foot);
    let rot = |k: usize, x: f32, y: f32| {
        let t = k as f32 * a;
        (x * t.cos() - y * t.sin(), x * t.sin() + y * t.cos())
    };
    for k in 0..n {
        let pts = [
            rot(k, foot, -hw),
            rot(k, r, -hw),
            rot(k, r, hw),
            rot(k, foot, hw),
        ];
        for (i, (x, y)) in pts.iter().enumerate() {
            if k == 0 && i == 0 {
                b.move_to(*x, *y);
            } else {
                b.line_to(*x, *y);
            }
        }
        arc(b, rb, k as f32 * a + side, (k + 1) as f32 * a - side, false);
    }
    b.close();
}

/// A shape's outline, centre (0, 0), y up, half sizes `hw`, `hh` (`shapePath`).
pub fn shape_path(shape: &str, hw: f32, hh: f32, params: [f32; 4]) -> Option<Path> {
    use std::f32::consts::{FRAC_1_SQRT_2, PI};
    let r = hw.min(hh);
    let mut b = PathBuilder::new();
    match shape {
        "square" => polygon(&mut b, &[(-r, -r), (r, -r), (r, r), (-r, r)]),
        "rectangle" => polygon(&mut b, &[(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]),
        "diamond" => polygon(&mut b, &[(0.0, hh), (hw, 0.0), (0.0, -hh), (-hw, 0.0)]),
        "triangle" => {
            let s = 2.0 * r;
            let h = s * 3f32.sqrt() / 2.0;
            polygon(
                &mut b,
                &[
                    (0.0, 2.0 * h / 3.0),
                    (s / 2.0, -h / 3.0),
                    (-s / 2.0, -h / 3.0),
                ],
            );
        }
        "pentagon" => polygon(&mut b, &regular(5, r / (PI / 5.0).cos(), PI / 2.0)),
        "hexagon" => polygon(&mut b, &regular(6, r / (PI / 6.0).cos(), PI / 2.0)),
        "octagon" => polygon(&mut b, &regular(8, r / (PI / 8.0).cos(), PI / 8.0)),
        "star" => {
            let pts: Vec<(f32, f32)> = (0..10)
                .map(|i| {
                    let a = PI / 2.0 + i as f32 * PI / 5.0;
                    let rr = if i % 2 == 1 { r * 0.4 } else { r };
                    (a.cos() * rr, a.sin() * rr)
                })
                .collect();
            polygon(&mut b, &pts);
        }
        "semicircle" => {
            arc(&mut b, r, 0.0, PI, true);
            b.close();
        }
        "quartercircle" => {
            b.move_to(0.0, 0.0);
            arc(&mut b, r, 0.0, PI / 2.0, false);
            b.close();
        }
        "arrowhead" => polygon(&mut b, &[(hw, 0.0), (-hw, hh * 0.8), (-hw, -hh * 0.8)]),
        "chevron" => {
            b.move_to(-hw, hh);
            b.line_to(hw, 0.0);
            b.line_to(-hw, -hh);
        }
        "cross" => {
            b.move_to(-r, 0.0);
            b.line_to(r, 0.0);
            b.move_to(0.0, -r);
            b.line_to(0.0, r);
        }
        "x" => {
            let d = r * FRAC_1_SQRT_2;
            b.move_to(-d, -d);
            b.line_to(d, d);
            b.move_to(-d, d);
            b.line_to(d, -d);
        }
        "line" => {
            b.move_to(-hw, 0.0);
            b.line_to(hw, 0.0);
        }
        "arrow" => {
            b.move_to(-hw, 0.0);
            b.line_to(hw, 0.0);
            b.move_to(hw - hw * 0.5, hh * 0.4);
            b.line_to(hw, 0.0);
            b.line_to(hw - hw * 0.5, -hh * 0.4);
        }
        "gear" => gear(&mut b, r, params[1], params[3]),
        "arc" => {
            let half = PI.min(params[2] / 2.0);
            arc(&mut b, r, PI / 2.0 - half, PI / 2.0 + half, true);
        }
        // circle, ring and anything unknown
        _ => {
            arc(&mut b, r, 0.0, 2.0 * PI, true);
            b.close();
        }
    }
    // A round hole in a closed shape; the even-odd fill leaves it empty and the stroke rings it.
    if params[0] > 0.0 && !OPEN.contains(&shape) {
        let hr = params[0] * r;
        arc(&mut b, hr, 0.0, 2.0 * PI, true);
        b.close();
    }
    b.finish()
}

/// A shape's outline as polylines, centre (0, 0), y up, half sizes `hw`, `hh`: [`shape_path`]
/// with each curve cut into chords of at most π/32 of an arc (the web's `Outline` in
/// app/sheet/mapVectors.ts), each part with whether it closes. A PDF writes a point symbol so.
pub fn shape_outline(
    shape: &str,
    hw: f64,
    hh: f64,
    params: [f64; 4],
) -> Vec<(Vec<[f64; 2]>, bool)> {
    let p = [
        params[0] as f32,
        params[1] as f32,
        params[2] as f32,
        params[3] as f32,
    ];
    let Some(path) = shape_path(shape, hw as f32, hh as f32, p) else {
        return Vec::new();
    };
    // The arcs are cubic quarter turns at most: 16 chords each is π/32.
    const CHORDS: usize = 16;
    // A ring's end is its start within the f32 arithmetic of the path.
    let near = 1e-5 * hw.max(hh).max(1.0);
    let mut out: Vec<(Vec<[f64; 2]>, bool)> = Vec::new();
    let mut at = [0.0f64, 0.0];
    let pt = |q: tiny_skia::Point| [f64::from(q.x), f64::from(q.y)];
    for seg in path.segments() {
        match seg {
            tiny_skia::PathSegment::MoveTo(q) => {
                at = pt(q);
                out.push((vec![at], false));
            }
            tiny_skia::PathSegment::LineTo(q) => {
                at = pt(q);
                if let Some(last) = out.last_mut() {
                    last.0.push(at);
                }
            }
            tiny_skia::PathSegment::QuadTo(c, q) => {
                let (c, q) = (pt(c), pt(q));
                let from = at;
                if let Some(last) = out.last_mut() {
                    for i in 1..=CHORDS {
                        let t = i as f64 / CHORDS as f64;
                        let u = 1.0 - t;
                        last.0.push([
                            u * u * from[0] + 2.0 * u * t * c[0] + t * t * q[0],
                            u * u * from[1] + 2.0 * u * t * c[1] + t * t * q[1],
                        ]);
                    }
                }
                at = q;
            }
            tiny_skia::PathSegment::CubicTo(c1, c2, q) => {
                let (c1, c2, q) = (pt(c1), pt(c2), pt(q));
                let from = at;
                if let Some(last) = out.last_mut() {
                    for i in 1..=CHORDS {
                        let t = i as f64 / CHORDS as f64;
                        let u = 1.0 - t;
                        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                        last.0.push([
                            a * from[0] + b * c1[0] + c * c2[0] + d * q[0],
                            a * from[1] + b * c1[1] + c * c2[1] + d * q[1],
                        ]);
                    }
                }
                at = q;
            }
            tiny_skia::PathSegment::Close => {
                if let Some(last) = out.last_mut() {
                    // A ring's last point is its first: written once.
                    if let (Some(a), Some(z)) = (last.0.first(), last.0.last())
                        && last.0.len() > 1
                        && (a[0] - z[0]).abs() <= near
                        && (a[1] - z[1]).abs() <= near
                    {
                        last.0.pop();
                    }
                    last.1 = true;
                    at = last.0.first().copied().unwrap_or(at);
                }
            }
        }
    }
    out.retain(|(pts, _)| pts.len() >= 2);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_has_polylines_for_a_pdf() {
        for s in crate::batches::SHAPE_IDS {
            let parts = shape_outline(s, 5.0, 5.0, [0.0, 12.0, std::f64::consts::PI, 0.2]);
            assert!(!parts.is_empty(), "{s}");
        }
        // A circle: one closed ring of 64 chords (four quarter turns of 16), on its radius.
        let circle = shape_outline("circle", 5.0, 5.0, [0.0; 4]);
        assert_eq!(circle.len(), 1);
        assert!(
            circle[0].1 && circle[0].0.len() == 64,
            "{}",
            circle[0].0.len()
        );
        assert!(
            circle[0]
                .0
                .iter()
                .all(|p| ((p[0] * p[0] + p[1] * p[1]).sqrt() - 5.0).abs() < 0.01)
        );
        // A cross: two open lines.
        let cross = shape_outline("cross", 5.0, 5.0, [0.0; 4]);
        assert_eq!(
            cross.iter().map(|p| (p.0.len(), p.1)).collect::<Vec<_>>(),
            [(2, false), (2, false)]
        );
        // A triangle points up (y up), closed.
        let t = shape_outline("triangle", 5.0, 5.0, [0.0; 4]);
        assert!(t[0].1 && t[0].0.len() == 3 && t[0].0[0][1] > 0.0);
    }

    #[test]
    fn every_shape_has_an_outline() {
        for s in crate::batches::SHAPE_IDS {
            assert!(
                shape_path(s, 5.0, 5.0, [0.0, 12.0, std::f32::consts::PI, 0.2]).is_some(),
                "{s}"
            );
        }
    }
}
