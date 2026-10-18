//! View marks (docs/adr/0345 §1): their drawing from their form, points and words, sized on paper at the view's scale.
//!
//! - Words `h` = size (2.5 mm unless said) × scale / 1000 high. A bubble's radius `R` = max(1.5 h, half the widest of
//!   its two words' widths + 0.5 h); a line across its middle; its top words' middle 0.75 h above the middle, its
//!   bottom words' 0.75 h below.
//! - A look arrow from a bubble towards `u`: a triangle, its tip 1.7 R from the bubble's middle along `u`, its base the
//!   bubble's points at ±60° from `u`.
//! - Section: its cut line through its points, the end parts 3 h long drawn heavy (twice: as they are and 0.15 h
//!   aside); at each end a bubble 1.2 R past the end along the end part, its arrow to the left of the line's direction
//!   (the right when flipped).
//! - Elevation: a bubble at its point, its arrow at `angle` (degrees from east, counter-clockwise; 90 unless said).
//! - Detail: its circle (dashed) around its point, a bubble at `at` (else at twice its radius up and to the right), a
//!   leader from the circle to the bubble along the line between their middles.
//!
//! `scripts/fixtures/viewmark_cases.py` writes the same without KentOS's code.

use crate::api::json::{FromJson, Json, ToJson, field};
use crate::block::{Piece, PieceStroke};
use crate::entity::Shape;
use crate::jsmath::{atan2, cos, js_hypot, js_max, js_min, sin};
use crate::text::{Font, TextAlign, width_em};
use crate::vec2::Vec2;

/// A mark's words' height on paper by default, millimetres.
pub const SIZE_MM: f64 = 2.5;
const DEG: f64 = std::f64::consts::PI / 180.0;

/// What a view mark marks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Form {
    #[default]
    Section,
    Elevation,
    Detail,
}

impl Form {
    fn of(s: &str) -> Option<Form> {
        Some(match s {
            "section" => Form::Section,
            "elevation" => Form::Elevation,
            "detail" => Form::Detail,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Form::Section => "section",
            Form::Elevation => "elevation",
            Form::Detail => "detail",
        }
    }
}

/// A view mark's fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spec {
    pub form: Form,
    pub radius: Option<f64>,
    pub at: Option<Vec2>,
    pub angle: Option<f64>,
    pub flip: bool,
    pub size: Option<f64>,
    pub top: String,
    pub bottom: String,
}

impl crate::api::json::Flat for Spec {
    const NAMES: &'static [&'static str] = &[
        "form", "radius", "at", "angle", "flip", "size", "words",
    ];

    fn read_flat(v: &Json) -> Result<Self, String> {
        let form = match v.get("form") {
            Json::Str(s) => Form::of(s).ok_or_else(|| format!("bilinmeyen işaret biçimi “{s}”"))?,
            _ => Form::Section,
        };
        let text = |j: &Json| match j {
            Json::Str(s) => s.clone(),
            _ => String::new(),
        };
        let label = v.get("words");
        Ok(Spec {
            form,
            radius: Option::<f64>::from_json(v.get("radius"))?,
            at: Option::<Vec2>::from_json(v.get("at"))?,
            angle: Option::<f64>::from_json(v.get("angle"))?,
            flip: matches!(v.get("flip"), Json::Bool(true)),
            size: Option::<f64>::from_json(v.get("size"))?,
            top: text(label.get("top")),
            bottom: text(label.get("bottom")),
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        field(out, first, "form", self.form.name());
        if let Some(r) = &self.radius {
            field(out, first, "radius", r);
        }
        if let Some(a) = &self.at {
            field(out, first, "at", a);
        }
        if let Some(a) = &self.angle {
            field(out, first, "angle", a);
        }
        if self.flip {
            field(out, first, "flip", &true);
        }
        if let Some(s) = &self.size {
            field(out, first, "size", s);
        }
        if !(self.top.is_empty() && self.bottom.is_empty()) {
            if !*first {
                out.push(',');
            }
            *first = false;
            out.push_str("\"words\":{\"top\":");
            self.top.as_str().write_json(out);
            out.push_str(",\"bottom\":");
            self.bottom.as_str().write_json(out);
            out.push('}');
        }
    }
}

impl ToJson for Spec {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        crate::api::json::Flat::write_flat(self, out, &mut first);
        out.push('}');
    }
}

fn piece(shape: Shape) -> Piece {
    Piece {
        shape,
        color: None,
        line_weight: None,
        by_layer: Vec::new(),
        gates: Vec::new(),
        stroke: PieceStroke::default(),
        attribute: None,
    }
}

fn line(a: Vec2, b: Vec2) -> Piece {
    piece(Shape::Line { a, b })
}

fn words(p: Vec2, text: &str, h: f64) -> Piece {
    piece(Shape::Text {
        p,
        text: text.to_owned(),
        height: h,
        rotation: 0.0,
        align: Some(TextAlign::MiddleCenter),
        width_factor: None,
        mask: None,
        box_width: None,
        line_spacing: None,
        runs: None,
        flow: None,
        face: Default::default(),
        path: None,
    })
}

/// Its words' height at 1:`scale`, metres.
pub fn height(spec: &Spec, scale: f64) -> f64 {
    spec.size.unwrap_or(SIZE_MM) * scale / 1000.0
}

fn word_width(t: &str, h: f64) -> f64 {
    if t.is_empty() { 0.0 } else { width_em(t, Font::DEFAULT) * h }
}

/// A bubble's radius at 1:`scale`.
pub fn bubble_radius(spec: &Spec, scale: f64) -> f64 {
    let h = height(spec, scale);
    let w = js_max(word_width(&spec.top, h), word_width(&spec.bottom, h));
    js_max(1.5 * h, w / 2.0 + 0.5 * h)
}

/// A detail's bubble's middle.
pub fn detail_bubble(points: &[Vec2], spec: &Spec) -> Option<Vec2> {
    if let Some(a) = spec.at {
        return Some(a);
    }
    let c = *points.first()?;
    let r = spec.radius?;
    let k = 2.0 * r * std::f64::consts::FRAC_1_SQRT_2;
    Some(Vec2::new(c.x + k, c.y + k))
}

fn unit(a: Vec2, b: Vec2) -> Option<Vec2> {
    let l = js_hypot(b.x - a.x, b.y - a.y);
    (l > 0.0).then(|| Vec2::new((b.x - a.x) / l, (b.y - a.y) / l))
}

/// A bubble at `c`, its two words and, toward `u`, its look arrow.
fn bubble(out: &mut Vec<Piece>, c: Vec2, spec: &Spec, scale: f64, u: Option<Vec2>) {
    let h = height(spec, scale);
    let r = bubble_radius(spec, scale);
    out.push(piece(Shape::Circle { c, r }));
    out.push(line(Vec2::new(c.x - r, c.y), Vec2::new(c.x + r, c.y)));
    if !spec.top.is_empty() {
        out.push(words(Vec2::new(c.x, c.y + 0.75 * h), &spec.top, h));
    }
    if !spec.bottom.is_empty() {
        out.push(words(Vec2::new(c.x, c.y - 0.75 * h), &spec.bottom, h));
    }
    if let Some(u) = u {
        let a = atan2(u.y, u.x);
        let at = |t: f64| Vec2::new(c.x + r * cos(t), c.y + r * sin(t));
        out.push(piece(Shape::Polygon {
            pts: vec![
                Vec2::new(c.x + 1.7 * r * u.x, c.y + 1.7 * r * u.y),
                at(a + 60.0 * DEG),
                at(a - 60.0 * DEG),
            ],
            bulges: None,
            holes: None,
            parts: None,
        }));
    }
}

/// Its pieces at 1:`scale`.
pub fn pieces(points: &[Vec2], spec: &Spec, scale: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    let h = height(spec, scale);
    let r = bubble_radius(spec, scale);
    match spec.form {
        Form::Section => {
            let p = points;
            if p.len() < 2 {
                return out;
            }
            for w in p.windows(2) {
                out.push(line(w[0], w[1]));
            }
            let n = p.len();
            // Each end: its part from the end inward, the heavy stroke beside it and the bubble past it.
            for (end, inner) in [(p[0], p[1]), (p[n - 1], p[n - 2])] {
                let Some(d) = unit(inner, end) else { continue };
                let along = unit(p[0], p[1]).unwrap_or(d);
                let side = if spec.flip { -1.0 } else { 1.0 };
                // The look side: left of the line's first part, the right when flipped.
                let look = Vec2::new(-along.y * side, along.x * side);
                let len = js_min(js_hypot(end.x - inner.x, end.y - inner.y), 3.0 * h);
                let start = Vec2::new(end.x - d.x * len, end.y - d.y * len);
                let off = 0.15 * h;
                out.push(line(
                    Vec2::new(start.x + look.x * off, start.y + look.y * off),
                    Vec2::new(end.x + look.x * off, end.y + look.y * off),
                ));
                let c = Vec2::new(end.x + d.x * 1.2 * r, end.y + d.y * 1.2 * r);
                out.push(line(end, Vec2::new(c.x - d.x * r, c.y - d.y * r)));
                bubble(&mut out, c, spec, scale, Some(look));
            }
        }
        Form::Elevation => {
            let Some(&c) = points.first() else {
                return out;
            };
            let a = spec.angle.unwrap_or(90.0) * DEG;
            bubble(&mut out, c, spec, scale, Some(Vec2::new(cos(a), sin(a))));
        }
        Form::Detail => {
            let (Some(&c), Some(rr), Some(b)) = (points.first(), spec.radius, detail_bubble(points, spec))
            else {
                return out;
            };
            let mut ring = piece(Shape::Circle { c, r: rr });
            ring.stroke.line_type = Some("dashed".into());
            out.push(ring);
            if let Some(u) = unit(c, b) {
                let from = Vec2::new(c.x + u.x * rr, c.y + u.y * rr);
                let to = Vec2::new(b.x - u.x * r, b.y - u.y * r);
                if js_hypot(b.x - c.x, b.y - c.y) > rr + r {
                    out.push(line(from, to));
                }
            }
            bubble(&mut out, b, spec, scale, None);
        }
    }
    out
}

