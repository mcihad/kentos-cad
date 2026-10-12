//! A chart as an SVG document (docs/adr/0227 §7): the drawing list written
//! out with a palette (the light theme's on white paper when saved), so the
//! saved picture is the one on screen on both platforms.

use std::f32::consts::PI;
use std::fmt::Write;

use super::layout::{Align, Baseline, Layout, Prim, Role};

/// The colours of the roles (`#rrggbb`) and the text's family.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub background: String,
    pub surface: String,
    pub grid: String,
    pub axis: String,
    pub ink1: String,
    pub ink2: String,
    pub ink3: String,
    pub series: Vec<String>,
    pub other: String,
    pub selection: String,
    pub font: String,
}

impl Palette {
    pub fn color(&self, role: Role) -> &str {
        match role {
            Role::Surface => &self.surface,
            Role::Grid => &self.grid,
            Role::Axis => &self.axis,
            Role::Ink1 => &self.ink1,
            Role::Ink2 => &self.ink2,
            Role::Ink3 => &self.ink3,
            Role::Series(k) => self
                .series
                .get(k as usize)
                .map_or(self.ink2.as_str(), String::as_str),
            Role::Other => &self.other,
            Role::Selection => &self.selection,
        }
    }
}

/// A number short: at most two fraction digits, no trailing zeros.
fn n(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

fn escape(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn opacity(a: f32) -> String {
    if a >= 1.0 {
        String::new()
    } else {
        format!(r#" fill-opacity="{}" stroke-opacity="{}""#, n(a), n(a))
    }
}

/// A rectangle with its top and bottom corners rounded by `rt` and `rb`.
pub fn rect_path(x: f32, y: f32, w: f32, h: f32, rt: f32, rb: f32) -> String {
    let (rt, rb) = (rt.min(w / 2.0).min(h), rb.min(w / 2.0).min(h));
    let mut d = format!("M{} {}", n(x), n(y + rt));
    if rt > 0.0 {
        let _ = write!(d, "A{r} {r} 0 0 1 {} {}", n(x + rt), n(y), r = n(rt));
    }
    let _ = write!(d, "H{}", n(x + w - rt));
    if rt > 0.0 {
        let _ = write!(d, "A{r} {r} 0 0 1 {} {}", n(x + w), n(y + rt), r = n(rt));
    }
    let _ = write!(d, "V{}", n(y + h - rb));
    if rb > 0.0 {
        let _ = write!(
            d,
            "A{r} {r} 0 0 1 {} {}",
            n(x + w - rb),
            n(y + h),
            r = n(rb)
        );
    }
    let _ = write!(d, "H{}", n(x + rb));
    if rb > 0.0 {
        let _ = write!(d, "A{r} {r} 0 0 1 {} {}", n(x), n(y + h - rb), r = n(rb));
    }
    d.push('Z');
    d
}

/// A ring's slice clockwise from twelve o'clock (radians).
pub fn wedge_path(cx: f32, cy: f32, r0: f32, r1: f32, a0: f32, a1: f32) -> String {
    let at = |r: f32, a: f32| {
        let (s, c) = (
            crate::jsmath::sin(f64::from(a)) as f32,
            crate::jsmath::cos(f64::from(a)) as f32,
        );
        (cx + r * s, cy - r * c)
    };
    // A whole ring is two halves: one arc cannot close a circle.
    if a1 - a0 >= 2.0 * PI - 1e-6 {
        let mid = a0 + PI;
        return format!(
            "{} {}",
            wedge_path(cx, cy, r0, r1, a0, mid),
            wedge_path(cx, cy, r0, r1, mid, a1)
        );
    }
    let large = if a1 - a0 > PI { 1 } else { 0 };
    let (p0, p1, q1, q0) = (at(r1, a0), at(r1, a1), at(r0, a1), at(r0, a0));
    format!(
        "M{} {}A{r1} {r1} 0 {large} 1 {} {}L{} {}A{r0} {r0} 0 {large} 0 {} {}Z",
        n(p0.0),
        n(p0.1),
        n(p1.0),
        n(p1.1),
        n(q1.0),
        n(q1.1),
        n(q0.0),
        n(q0.1),
        r1 = n(r1),
        r0 = n(r0),
    )
}

/// The drawing as an SVG document.
pub fn svg(layout: &Layout, palette: &Palette) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}" font-family="{font}">"#,
        w = n(layout.width),
        h = n(layout.height),
        font = escape(&palette.font)
    );
    let _ = write!(
        out,
        r#"<rect width="{}" height="{}" fill="{}"/>"#,
        n(layout.width),
        n(layout.height),
        palette.background
    );
    for p in &layout.prims {
        match p {
            Prim::Rect {
                x,
                y,
                w,
                h,
                r_top,
                r_bottom,
                fill,
                alpha,
                ..
            } => {
                let _ = write!(
                    out,
                    r#"<path d="{}" fill="{}"{}/>"#,
                    rect_path(*x, *y, *w, *h, *r_top, *r_bottom),
                    palette.color(*fill),
                    opacity(*alpha)
                );
            }
            Prim::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                width,
                alpha,
            } => {
                let _ = write!(
                    out,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" stroke-linecap="round"{}/>"#,
                    n(*x1),
                    n(*y1),
                    n(*x2),
                    n(*y2),
                    palette.color(*stroke),
                    n(*width),
                    opacity(*alpha)
                );
            }
            Prim::Poly {
                pts,
                closed,
                stroke,
                width,
                alpha,
            } => {
                let points: Vec<String> = pts
                    .chunks_exact(2)
                    .map(|c| format!("{},{}", n(c[0]), n(c[1])))
                    .collect();
                let _ = write!(
                    out,
                    r#"<{tag} points="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round" stroke-linecap="round"{}/>"#,
                    points.join(" "),
                    palette.color(*stroke),
                    n(*width),
                    opacity(*alpha),
                    tag = if *closed { "polygon" } else { "polyline" }
                );
            }
            Prim::Circle {
                x,
                y,
                r,
                fill,
                stroke,
                width,
                alpha,
                ..
            } => {
                let _ = write!(
                    out,
                    r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}"{}/>"#,
                    n(*x),
                    n(*y),
                    n(*r),
                    fill.map_or("none", |f| palette.color(f)),
                    stroke.map_or("none", |s| palette.color(s)),
                    n(*width),
                    opacity(*alpha)
                );
            }
            Prim::Wedge {
                cx,
                cy,
                r0,
                r1,
                a0,
                a1,
                fill,
                alpha,
                ..
            } => {
                let _ = write!(
                    out,
                    r#"<path d="{}" fill="{}"{}/>"#,
                    wedge_path(*cx, *cy, *r0, *r1, *a0, *a1),
                    palette.color(*fill),
                    opacity(*alpha)
                );
            }
            Prim::Cross {
                x,
                y,
                size,
                stroke,
                width,
            } => {
                let s = *size;
                let _ = write!(
                    out,
                    r#"<path d="M{} {}L{} {}M{} {}L{} {}" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
                    n(x - s),
                    n(y - s),
                    n(x + s),
                    n(y + s),
                    n(x - s),
                    n(y + s),
                    n(x + s),
                    n(y - s),
                    palette.color(*stroke),
                    n(*width)
                );
            }
            Prim::Text {
                x,
                y,
                text,
                size,
                weight,
                align,
                baseline,
                color,
            } => {
                let anchor = match align {
                    Align::Start => "start",
                    Align::Middle => "middle",
                    Align::End => "end",
                };
                let base = match baseline {
                    Baseline::Top => "text-before-edge",
                    Baseline::Middle => "central",
                    Baseline::Bottom => "text-after-edge",
                };
                let _ = write!(
                    out,
                    r#"<text x="{}" y="{}" font-size="{}" font-weight="{}" text-anchor="{anchor}" dominant-baseline="{base}" fill="{}">{}</text>"#,
                    n(*x),
                    n(*y),
                    n(*size),
                    weight,
                    palette.color(*color),
                    escape(text)
                );
            }
            Prim::Points {
                start,
                count,
                r,
                ring,
                fill,
                alpha,
            } => {
                let colour = palette.color(*fill);
                let stroke = if *ring {
                    format!(r#" stroke="{}" stroke-width="2""#, palette.surface)
                } else {
                    String::new()
                };
                let _ = write!(out, r#"<g fill="{colour}"{stroke}{}>"#, opacity(*alpha));
                let (a, b) = (*start as usize, (*start + *count) as usize);
                for xy in layout.points[a * 2..b * 2].chunks_exact(2) {
                    let _ = write!(
                        out,
                        r#"<circle cx="{}" cy="{}" r="{}"/>"#,
                        n(xy[0]),
                        n(xy[1]),
                        n(*r)
                    );
                }
                out.push_str("</g>");
            }
        }
    }
    out.push_str("</svg>");
    out
}
