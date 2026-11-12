//! A chart's drawing list painted (docs/adr/0227 §5; the web's
//! `ui/charts/drawChart.ts`): the core lays the chart out, this only paints
//! its primitives with the theme's colours. The shapes go into one picture
//! in the screen's own pixels with tiny-skia (Skia's antialiasing, as the
//! web's canvas), the bulk points of a scatter splatted straight into it;
//! the texts are the interface's, drawn over it by the canvas. The paths are
//! written once as commands, for the picture and for the canvas's outline.

use std::f32::consts::{FRAC_PI_2, TAU};

use kentos_geometry_core::ops::charts::layout::{Layout, Prim, Role};
use kentos_ui::theme::Tokens;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::palette::color;

/// A path's step, in the chart's pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Step {
    Move(f32, f32),
    Line(f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// A quarter circle's control length as a share of its radius.
const KAPPA: f32 = 0.552_284_8;

/// A rectangle with rounded top or bottom corners (a bar's data end), as
/// the web's `roundRect`: a radius at most half the width and the height.
pub(crate) fn round_rect(x: f32, y: f32, w: f32, h: f32, rt: f32, rb: f32) -> Vec<Step> {
    let rt = rt.min(w / 2.0).min(h).max(0.0);
    let rb = rb.min(w / 2.0).min(h).max(0.0);
    let (kt, kb) = (rt * KAPPA, rb * KAPPA);
    let mut s = vec![Step::Move(x, y + rt)];
    if rt > 0.0 {
        s.push(Step::Cubic(x, y + rt - kt, x + rt - kt, y, x + rt, y));
    }
    s.push(Step::Line(x + w - rt, y));
    if rt > 0.0 {
        s.push(Step::Cubic(
            x + w - rt + kt,
            y,
            x + w,
            y + rt - kt,
            x + w,
            y + rt,
        ));
    }
    s.push(Step::Line(x + w, y + h - rb));
    if rb > 0.0 {
        s.push(Step::Cubic(
            x + w,
            y + h - rb + kb,
            x + w - rb + kb,
            y + h,
            x + w - rb,
            y + h,
        ));
    }
    s.push(Step::Line(x + rb, y + h));
    if rb > 0.0 {
        s.push(Step::Cubic(
            x + rb - kb,
            y + h,
            x,
            y + h - rb + kb,
            x,
            y + h - rb,
        ));
    }
    s.push(Step::Close);
    s
}

/// A point at `a` (clockwise from twelve o'clock) on a circle.
fn at(cx: f32, cy: f32, r: f32, a: f32) -> (f32, f32) {
    (cx + r * a.sin(), cy - r * a.cos())
}

/// An arc from `a0` to `a1` (either way) as cubics, each at most a quarter turn.
fn arc(steps: &mut Vec<Step>, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) {
    let sweep = a1 - a0;
    let n = (sweep.abs() / FRAC_PI_2).ceil().max(1.0) as usize;
    let d = sweep / n as f32;
    let k = 4.0 / 3.0 * (d / 4.0).tan() * r;
    for i in 0..n {
        let (b0, b1) = (a0 + d * i as f32, a0 + d * (i + 1) as f32);
        let (x0, y0) = at(cx, cy, r, b0);
        let (x3, y3) = at(cx, cy, r, b1);
        // The tangent of a clockwise turn at b is (cos b, sin b).
        s_cubic(
            steps,
            x0 + k * b0.cos(),
            y0 + k * b0.sin(),
            x3 - k * b1.cos(),
            y3 - k * b1.sin(),
            x3,
            y3,
        );
    }
}

fn s_cubic(steps: &mut Vec<Step>, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
    steps.push(Step::Cubic(a, b, c, d, e, f));
}

/// A ring's slice: the outer arc clockwise, the inner one back.
pub(crate) fn wedge(cx: f32, cy: f32, r0: f32, r1: f32, a0: f32, a1: f32) -> Vec<Step> {
    let (x, y) = at(cx, cy, r1, a0);
    let mut s = vec![Step::Move(x, y)];
    arc(&mut s, cx, cy, r1, a0, a1);
    if r0 > 0.0 {
        let (x, y) = at(cx, cy, r0, a1);
        s.push(Step::Line(x, y));
        arc(&mut s, cx, cy, r0, a1, a0);
    } else {
        s.push(Step::Line(cx, cy));
    }
    s.push(Step::Close);
    s
}

/// A circle.
pub(crate) fn circle(cx: f32, cy: f32, r: f32) -> Vec<Step> {
    let mut s = vec![Step::Move(cx, cy - r)];
    arc(&mut s, cx, cy, r, 0.0, TAU);
    s.push(Step::Close);
    s
}

/// The steps as a tiny-skia path in the picture's pixels (`k` per chart pixel).
fn skia_path(steps: &[Step], k: f32) -> Option<tiny_skia::Path> {
    let mut b = PathBuilder::new();
    for s in steps {
        match *s {
            Step::Move(x, y) => b.move_to(x * k, y * k),
            Step::Line(x, y) => b.line_to(x * k, y * k),
            Step::Cubic(a, c, d, e, f, g) => b.cubic_to(a * k, c * k, d * k, e * k, f * k, g * k),
            Step::Close => b.close(),
        }
    }
    b.finish()
}

fn paint_of(c: iced::Color, alpha: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8,
        (c.a * alpha * 255.0).round() as u8,
    );
    p.anti_alias = true;
    p
}

/// A hairline's place: a line across whole pixels of the picture (the
/// web's half-pixel rule, for any pixel ratio).
fn crisp(c: f32, width: f32) -> f32 {
    (c - width / 2.0).round() + width / 2.0
}

/// The layout's shapes in a picture of `w` × `h` pixels (`k` per chart
/// pixel) on the surface's colour, RGBA, opaque. Texts are left for the canvas.
pub(crate) fn picture(layout: &Layout, t: &Tokens, w: u32, h: u32, k: f32) -> Option<Vec<u8>> {
    let mut pix = Pixmap::new(w.max(1), h.max(1))?;
    let ground = color(t, Role::Surface);
    pix.fill(tiny_skia::Color::from_rgba(
        ground.r, ground.g, ground.b, 1.0,
    )?);
    let fill = |pix: &mut Pixmap, steps: &[Step], role: Role, alpha: f32| {
        if let Some(path) = skia_path(steps, k) {
            pix.fill_path(
                &path,
                &paint_of(color(t, role), alpha),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    };
    let stroke = |pix: &mut Pixmap, steps: &[Step], role: Role, width: f32, alpha: f32| {
        if let Some(path) = skia_path(steps, k) {
            let s = Stroke {
                width: width * k,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            pix.stroke_path(
                &path,
                &paint_of(color(t, role), alpha),
                &s,
                Transform::identity(),
                None,
            );
        }
    };
    for p in &layout.prims {
        match p {
            Prim::Rect {
                x,
                y,
                w,
                h,
                r_top,
                r_bottom,
                fill: role,
                alpha,
                ..
            } => fill(
                &mut pix,
                &round_rect(*x, *y, *w, *h, *r_top, *r_bottom),
                *role,
                *alpha,
            ),
            Prim::Line {
                x1,
                y1,
                x2,
                y2,
                stroke: role,
                width,
                alpha,
            } => {
                // A hairline along an axis covers whole pixels so it stays sharp.
                let wd = (width * k).round().max(1.0);
                let (mut a, mut b) = ((x1 * k, y1 * k), (x2 * k, y2 * k));
                let straight = *width <= 1.0 && (x1 == x2 || y1 == y2);
                if straight {
                    if x1 == x2 {
                        let x = crisp(a.0, wd);
                        (a.0, b.0) = (x, x);
                    }
                    if y1 == y2 {
                        let y = crisp(a.1, wd);
                        (a.1, b.1) = (y, y);
                    }
                }
                let mut pb = PathBuilder::new();
                pb.move_to(a.0, a.1);
                pb.line_to(b.0, b.1);
                if let Some(path) = pb.finish() {
                    let s = Stroke {
                        width: if straight { wd } else { width * k },
                        line_cap: if straight {
                            LineCap::Butt
                        } else {
                            LineCap::Round
                        },
                        ..Stroke::default()
                    };
                    let mut paint = paint_of(color(t, *role), *alpha);
                    paint.anti_alias = !straight;
                    pix.stroke_path(&path, &paint, &s, Transform::identity(), None);
                }
            }
            Prim::Poly {
                pts,
                closed,
                stroke: role,
                width,
                alpha,
            } => {
                let mut steps: Vec<Step> = pts
                    .chunks_exact(2)
                    .enumerate()
                    .map(|(i, q)| {
                        if i == 0 {
                            Step::Move(q[0], q[1])
                        } else {
                            Step::Line(q[0], q[1])
                        }
                    })
                    .collect();
                if *closed {
                    steps.push(Step::Close);
                }
                stroke(&mut pix, &steps, *role, *width, *alpha);
            }
            Prim::Circle {
                x,
                y,
                r,
                fill: f,
                stroke: s,
                width,
                alpha,
                ..
            } => {
                let steps = circle(*x, *y, *r);
                if let Some(role) = f {
                    fill(&mut pix, &steps, *role, *alpha);
                }
                if let Some(role) = s {
                    stroke(&mut pix, &steps, *role, *width, *alpha);
                }
            }
            Prim::Wedge {
                cx,
                cy,
                r0,
                r1,
                a0,
                a1,
                fill: role,
                alpha,
                ..
            } => fill(
                &mut pix,
                &wedge(*cx, *cy, *r0, *r1, *a0, *a1),
                *role,
                *alpha,
            ),
            Prim::Cross {
                x,
                y,
                size,
                stroke: role,
                width,
            } => {
                let steps = [
                    Step::Move(x - size, y - size),
                    Step::Line(x + size, y + size),
                    Step::Move(x - size, y + size),
                    Step::Line(x + size, y - size),
                ];
                stroke(&mut pix, &steps, *role, *width, 1.0);
            }
            Prim::Text { .. } => {}
            Prim::Points {
                start,
                count,
                r,
                ring,
                fill: role,
                alpha,
            } => {
                let from = *start as usize * 2;
                let to = (*start + *count) as usize * 2;
                let pts = layout.points.get(from..to).unwrap_or_default();
                // The surface's ring under every point first, then the points (the web's two passes).
                if *ring {
                    splat(&mut pix, pts, (r + 2.0) * k, k, ground, *alpha);
                }
                splat(&mut pix, pts, r * k, k, color(t, *role), *alpha);
            }
        }
    }
    Some(pix.take())
}

/// Many small discs of radius `r` (the picture's pixels) at the chart's
/// points: each pixel takes the disc's covered share of the colour, over
/// what is there (premultiplied; the ground is opaque).
fn splat(pix: &mut Pixmap, pts: &[f32], r: f32, k: f32, c: iced::Color, alpha: f32) {
    let (w, h) = (pix.width() as i32, pix.height() as i32);
    let a = (c.a * alpha).clamp(0.0, 1.0);
    let rgb = [c.r * 255.0, c.g * 255.0, c.b * 255.0];
    let data = pix.data_mut();
    for q in pts.chunks_exact(2) {
        let (cx, cy) = (q[0] * k, q[1] * k);
        let (x0, x1) = (
            ((cx - r - 1.0).floor() as i32).max(0),
            ((cx + r + 1.0).ceil() as i32).min(w),
        );
        let (y0, y1) = (
            ((cy - r - 1.0).floor() as i32).max(0),
            ((cy + r + 1.0).ceil() as i32).min(h),
        );
        for j in y0..y1 {
            let dy = j as f32 + 0.5 - cy;
            for i in x0..x1 {
                let dx = i as f32 + 0.5 - cx;
                let cover = (r + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
                if cover <= 0.0 {
                    continue;
                }
                let s = a * cover;
                let at = ((j * w + i) * 4) as usize;
                for (ch, v) in rgb.iter().enumerate() {
                    let d = f32::from(data[at + ch]);
                    data[at + ch] = (v * s + d * (1.0 - s)).round() as u8;
                }
                let d = f32::from(data[at + 3]);
                data[at + 3] = (255.0 * s + d * (1.0 - s)).round() as u8;
            }
        }
    }
}
