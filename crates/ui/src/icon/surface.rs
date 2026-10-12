//! Where an icon is drawn (`draw.rs`, `svg.rs`): Iced's canvas frame (a
//! component's own canvas, [`super::draw`]), or a recording that writes the
//! drawing as an SVG document in the icon's pixels. [`super::Glyph`] draws the
//! document through Iced's SVG pipeline, which rasterizes it with resvg at the
//! screen's pixels and full anti-aliasing, as the browser draws the web's
//! icons. A tessellated canvas path gets only the window's 4× multisampling:
//! its few coverage steps show on a small icon's thin, slanted and curved
//! strokes.

use std::fmt::Write as _;

use iced::widget::canvas::fill::Rule;
use iced::widget::canvas::path::lyon_path::Event;
use iced::widget::canvas::{Fill, Frame, LineCap, LineJoin, Path, Stroke, Style};
use iced::{Color, Point, Size, Vector};

/// What an icon's drawing needs of where it is drawn: the methods of Iced's
/// canvas frame it uses.
pub(super) trait Surface {
    fn fill(&mut self, path: &Path, fill: impl Into<Fill>);
    fn stroke<'a>(&mut self, path: &Path, stroke: impl Into<Stroke<'a>>);
    fn translate(&mut self, by: Vector);
    fn scale_nonuniform(&mut self, by: impl Into<Vector>);
    fn with_save<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R;
}

impl Surface for Frame {
    fn fill(&mut self, path: &Path, fill: impl Into<Fill>) {
        Frame::fill(self, path, fill);
    }

    fn stroke<'a>(&mut self, path: &Path, stroke: impl Into<Stroke<'a>>) {
        Frame::stroke(self, path, stroke);
    }

    fn translate(&mut self, by: Vector) {
        Frame::translate(self, by);
    }

    fn scale_nonuniform(&mut self, by: impl Into<Vector>) {
        Frame::scale_nonuniform(self, by);
    }

    fn with_save<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        Frame::with_save(self, f)
    }
}

/// A drawing recorded as SVG elements in pixels: what a frame of `size` would
/// draw. As Iced's frame does, a transform moves the paths and not the stroke
/// widths; a dash pattern scales with it.
pub(super) struct Recording {
    size: Size,
    /// The current transform: a point (x, y) is drawn at (sx·x + tx, sy·y + ty).
    transform: [f32; 4],
    saved: Vec<[f32; 4]>,
    body: String,
}

impl Recording {
    pub(super) fn new(size: Size) -> Recording {
        Recording {
            size,
            transform: [1.0, 1.0, 0.0, 0.0],
            saved: Vec::new(),
            body: String::new(),
        }
    }

    /// The SVG document: the icon's box in pixels, its elements in the order drawn.
    pub(super) fn document(&self) -> String {
        let (w, h) = (num(self.size.width), num(self.size.height));
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">{}</svg>"#,
            self.body
        )
    }

    fn at(&self, p: Point) -> (String, String) {
        let [sx, sy, tx, ty] = self.transform;
        (num(sx * p.x + tx), num(sy * p.y + ty))
    }

    /// A path's outline (`d`), the transform applied.
    fn data(&self, path: &Path) -> String {
        let mut d = String::new();
        let point = |p: iced::widget::canvas::path::lyon_path::math::Point| Point::new(p.x, p.y);
        for event in path.raw().iter() {
            match event {
                Event::Begin { at } => {
                    let (x, y) = self.at(point(at));
                    let _ = write!(d, "M{x} {y}");
                }
                Event::Line { to, .. } => {
                    let (x, y) = self.at(point(to));
                    let _ = write!(d, "L{x} {y}");
                }
                Event::Quadratic { ctrl, to, .. } => {
                    let ((cx, cy), (x, y)) = (self.at(point(ctrl)), self.at(point(to)));
                    let _ = write!(d, "Q{cx} {cy} {x} {y}");
                }
                Event::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    let ((ax, ay), (bx, by), (x, y)) = (
                        self.at(point(ctrl1)),
                        self.at(point(ctrl2)),
                        self.at(point(to)),
                    );
                    let _ = write!(d, "C{ax} {ay} {bx} {by} {x} {y}");
                }
                Event::End { close, .. } => {
                    if close {
                        d.push('Z');
                    }
                }
            }
        }
        d
    }
}

impl Surface for Recording {
    fn fill(&mut self, path: &Path, fill: impl Into<Fill>) {
        let fill = fill.into();
        let d = self.data(path);
        if d.is_empty() {
            return;
        }
        let color = solid(&fill.style);
        let rule = if fill.rule == Rule::EvenOdd {
            r#" fill-rule="evenodd""#
        } else {
            ""
        };
        let _ = write!(
            self.body,
            r#"<path d="{d}" fill="{}"{}{rule}/>"#,
            hex(color),
            opacity("fill-opacity", color.a)
        );
    }

    fn stroke<'a>(&mut self, path: &Path, stroke: impl Into<Stroke<'a>>) {
        let stroke = stroke.into();
        let d = self.data(path);
        if d.is_empty() || stroke.width <= 0.0 {
            return;
        }
        let color = solid(&stroke.style);
        let cap = match stroke.line_cap {
            LineCap::Butt => "butt",
            LineCap::Square => "square",
            LineCap::Round => "round",
        };
        let join = match stroke.line_join {
            LineJoin::Miter => "miter",
            LineJoin::Round => "round",
            LineJoin::Bevel => "bevel",
        };
        let mut dash = String::new();
        let segments = stroke.line_dash.segments;
        if !segments.is_empty() {
            // Iced dashes the path before its transform: the pattern scales with it.
            let k = self.transform[0].abs().max(self.transform[1].abs());
            let list: Vec<String> = segments.iter().map(|s| num(s * k)).collect();
            let offset: f32 = segments
                .iter()
                .cycle()
                .take(stroke.line_dash.offset)
                .sum::<f32>()
                * k;
            let _ = write!(dash, r#" stroke-dasharray="{}""#, list.join(" "));
            if offset > 0.0 {
                let _ = write!(dash, r#" stroke-dashoffset="{}""#, num(offset));
            }
        }
        let _ = write!(
            self.body,
            r#"<path d="{d}" fill="none" stroke="{}" stroke-width="{}" stroke-linecap="{cap}" stroke-linejoin="{join}"{}{dash}/>"#,
            hex(color),
            num(stroke.width),
            opacity("stroke-opacity", color.a)
        );
    }

    fn translate(&mut self, by: Vector) {
        let [sx, sy, tx, ty] = self.transform;
        self.transform = [sx, sy, tx + sx * by.x, ty + sy * by.y];
    }

    fn scale_nonuniform(&mut self, by: impl Into<Vector>) {
        let by = by.into();
        let [sx, sy, tx, ty] = self.transform;
        self.transform = [sx * by.x, sy * by.y, tx, ty];
    }

    fn with_save<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.saved.push(self.transform);
        let result = f(self);
        if let Some(t) = self.saved.pop() {
            self.transform = t;
        }
        result
    }
}

/// A style's colour: the solid one (icons draw no gradient).
fn solid(style: &Style) -> Color {
    match style {
        Style::Solid(color) => *color,
        Style::Gradient(_) => Color::BLACK,
    }
}

fn hex(color: Color) -> String {
    let [r, g, b, _] = color.into_rgba8();
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn opacity(name: &str, a: f32) -> String {
    if a >= 1.0 {
        String::new()
    } else {
        format!(r#" {name}="{}""#, num(a.max(0.0)))
    }
}

/// A number with three decimals at most, its zeros trimmed: short, and the same
/// text for the same drawing (the document's hash is its handle's id).
fn num(v: f32) -> String {
    let v = if v.is_finite() { v } else { 0.0 };
    let mut s = format!("{v:.3}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" { "0".to_owned() } else { s }
}
