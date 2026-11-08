//! Tags (docs/adr/0344 §1): their drawing from their words, frame and leader, sized on paper at the view's scale.
//!
//! - Its words: each line centred on the tag's middle, `h` high (its size on paper at the scale), a line pitch of
//!   `PITCH × h`, the block's middle on the tag's.
//! - Its frame round the words with a pad of `PAD × h`: a circle round their box, a hexagon (the box with a point each
//!   side as deep as half its height times tan 30°), a rectangle, a rectangle with quarter-circle corners of half its
//!   lesser half-size, or none. Words that are empty still have a frame as wide as `h`, so the tag can be seen and picked.
//! - An ellipse (docs/adr/3027; Mimarlar Odası: "numara elips içinde yazılır") rounds the first line only, through its
//!   padded box's corners (its half axes √2 × (half the line's width + pad) and √2 × (h / 2 + pad)); the lines under it
//!   go down far enough to clear it by `0.2 h` (`ELLIPSE_GAP`): a room's number in its ellipse, its name and area below.
//! - Its leader: from where the line to the point it ties to leaves the frame (a frameless tag's box) to that point;
//!   none when the point is inside.
//!
//! `scripts/fixtures/tag_cases.py` writes the same without KentOS's code.

use crate::api::json::{FromJson, Json, ToJson, field};
use crate::block::{Piece, PieceStroke};
use crate::entity::Shape;
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::text::{Font, TextAlign, width_em};
use crate::vec2::Vec2;

/// A tag's words' height on paper by default, millimetres.
pub const SIZE_MM: f64 = 2.5;
/// The distance between two lines' middles, in words' heights.
pub const PITCH: f64 = 1.5;
/// The gap between the words and the frame, in words' heights.
pub const PAD: f64 = 0.4;
/// tan 30°: a hexagon's point's depth over half its height.
const TAN30: f64 = 0.577_350_269_189_625_8;
/// tan 22.5°: a quarter circle's bulge.
const QUARTER: f64 = 0.414_213_562_373_095_1;
/// The gap between an ellipse and the line under it, in words' heights.
pub const ELLIPSE_GAP: f64 = 0.2;

/// A tag's frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Frame {
    #[default]
    Circle,
    Hexagon,
    Rectangle,
    Rounded,
    None,
    /// Round the first line: a room's number (Mimarlar Odası's room tag).
    Ellipse,
}

impl Frame {
    fn of(s: &str) -> Option<Frame> {
        Some(match s {
            "circle" => Frame::Circle,
            "hexagon" => Frame::Hexagon,
            "rectangle" => Frame::Rectangle,
            "rounded" => Frame::Rounded,
            "none" => Frame::None,
            "ellipse" => Frame::Ellipse,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Frame::Circle => "circle",
            Frame::Hexagon => "hexagon",
            Frame::Rectangle => "rectangle",
            Frame::Rounded => "rounded",
            Frame::None => "none",
            Frame::Ellipse => "ellipse",
        }
    }
}

/// A tag's fields beside its middle.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spec {
    pub text: String,
    pub frame: Frame,
    pub size: Option<f64>,
    pub leader: bool,
    pub anchor: Option<Vec2>,
}

impl crate::api::json::Flat for Spec {
    const NAMES: &'static [&'static str] = &["text", "shape", "size", "leader", "anchor"];

    fn read_flat(v: &Json) -> Result<Self, String> {
        let frame = match v.get("shape") {
            Json::Str(s) => {
                Frame::of(s).ok_or_else(|| format!("bilinmeyen etiket çerçevesi “{s}”"))?
            }
            _ => Frame::Circle,
        };
        Ok(Spec {
            text: match v.get("text") {
                Json::Str(s) => s.clone(),
                _ => String::new(),
            },
            frame,
            size: Option::<f64>::from_json(v.get("size"))?,
            leader: matches!(v.get("leader"), Json::Bool(true)),
            anchor: Option::<Vec2>::from_json(v.get("anchor"))?,
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        if !self.text.is_empty() {
            field(out, first, "text", self.text.as_str());
        }
        if self.frame != Frame::Circle {
            field(out, first, "shape", self.frame.name());
        }
        if let Some(s) = &self.size {
            field(out, first, "size", s);
        }
        if self.leader {
            field(out, first, "leader", &true);
        }
        if let Some(a) = &self.anchor {
            field(out, first, "anchor", a);
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
        annotation_origin: None,
    }
}

/// Its words' height at 1:`scale`, metres.
pub fn height(spec: &Spec, scale: f64) -> f64 {
    spec.size.unwrap_or(SIZE_MM) * scale / 1000.0
}

/// Its words' box's half width and half height at 1:`scale`, the pad not in it.
pub fn half_box(spec: &Spec, scale: f64) -> (f64, f64) {
    let h = height(spec, scale);
    let lines: Vec<&str> = spec.text.split('\n').collect();
    let n = lines.len().max(1) as f64;
    let w = lines
        .iter()
        .map(|l| {
            if l.is_empty() {
                0.0
            } else {
                width_em(l, Font::DEFAULT) * h
            }
        })
        .fold(h, js_max);
    (w / 2.0, h * (1.0 + PITCH * (n - 1.0)) / 2.0)
}

/// Its frame's ring (its bulges for a rounded one) round its middle; none for a circle or no frame.
fn ring(at: Vec2, spec: &Spec, scale: f64) -> Option<(Vec<Vec2>, Option<Vec<f64>>)> {
    let pad = PAD * height(spec, scale);
    let (w, b0) = half_box(spec, scale);
    let (a, b) = (w + pad, b0 + pad);
    let p = |x: f64, y: f64| Vec2::new(at.x + x, at.y + y);
    Some(match spec.frame {
        Frame::Rectangle => (vec![p(-a, -b), p(a, -b), p(a, b), p(-a, b)], None),
        Frame::Hexagon => {
            let k = b * TAN30;
            (
                vec![
                    p(-a, -b),
                    p(a, -b),
                    p(a + k, 0.0),
                    p(a, b),
                    p(-a, b),
                    p(-a - k, 0.0),
                ],
                None,
            )
        }
        Frame::Rounded => {
            let r = js_min(a, b) / 2.0;
            (
                vec![
                    p(-a + r, -b),
                    p(a - r, -b),
                    p(a, -b + r),
                    p(a, b - r),
                    p(a - r, b),
                    p(-a + r, b),
                    p(-a, b - r),
                    p(-a, -b + r),
                ],
                Some(vec![0.0, QUARTER, 0.0, QUARTER, 0.0, QUARTER, 0.0, QUARTER]),
            )
        }
        Frame::Circle | Frame::None | Frame::Ellipse => return None,
    })
}

/// An ellipse frame's middle's height over the tag's middle and its half axes (across, up), and how far the lines
/// under the first go down to clear it.
fn ellipse(spec: &Spec, scale: f64) -> (f64, f64, f64, f64) {
    let h = height(spec, scale);
    let pad = PAD * h;
    let (_, b) = half_box(spec, scale);
    let first = spec.text.split('\n').next().unwrap_or("");
    let w1 = if first.is_empty() {
        h
    } else {
        js_max(h, width_em(first, Font::DEFAULT) * h)
    };
    let (ea, eb) = (
        std::f64::consts::SQRT_2 * (w1 / 2.0 + pad),
        std::f64::consts::SQRT_2 * (h / 2.0 + pad),
    );
    let extra = js_max(0.0, eb + ELLIPSE_GAP * h - (PITCH * h - h / 2.0));
    (b - h / 2.0, ea, eb, extra)
}

/// A line's middle's height over the tag's middle: lines `PITCH × h` apart from the first, those under an ellipse
/// further down by its clearance.
fn line_y(spec: &Spec, scale: f64, i: usize) -> f64 {
    let h = height(spec, scale);
    let (_, b) = half_box(spec, scale);
    let extra = if spec.frame == Frame::Ellipse && i > 0 {
        ellipse(spec, scale).3
    } else {
        0.0
    };
    b - h / 2.0 - i as f64 * PITCH * h - extra
}

/// A circle frame's radius at 1:`scale`.
pub fn radius(spec: &Spec, scale: f64) -> f64 {
    let (w, b) = half_box(spec, scale);
    js_hypot(w, b) + PAD * height(spec, scale)
}

/// How far from the middle the line towards `d` (a unit direction) leaves the frame: the circle's radius, else the
/// nearest crossing of the ray with the frame's sides (a rounded frame's and no frame's: its box's).
fn exit(at: Vec2, spec: &Spec, scale: f64, d: Vec2) -> f64 {
    if spec.frame == Frame::Circle {
        return radius(spec, scale);
    }
    let pad = PAD * height(spec, scale);
    let (w, b0) = half_box(spec, scale);
    let (a, b) = (w + pad, b0 + pad);
    let pts: Vec<Vec2> = match spec.frame {
        Frame::Hexagon => ring(at, spec, scale).map(|r| r.0).unwrap_or_default(),
        Frame::Ellipse => {
            // The box of the ellipse and the lines under it.
            let (y1, ea, eb, _) = ellipse(spec, scale);
            let h = height(spec, scale);
            let n = spec.text.split('\n').count().max(1);
            let x = js_max(ea, a);
            let top = y1 + eb;
            let bottom = js_min(y1 - eb, line_y(spec, scale, n - 1) - h / 2.0 - pad);
            vec![
                Vec2::new(at.x - x, at.y + bottom),
                Vec2::new(at.x + x, at.y + bottom),
                Vec2::new(at.x + x, at.y + top),
                Vec2::new(at.x - x, at.y + top),
            ]
        }
        _ => vec![
            Vec2::new(at.x - a, at.y - b),
            Vec2::new(at.x + a, at.y - b),
            Vec2::new(at.x + a, at.y + b),
            Vec2::new(at.x - a, at.y + b),
        ],
    };
    let mut best = f64::INFINITY;
    for i in 0..pts.len() {
        let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
        let e = Vec2::new(q.x - p.x, q.y - p.y);
        let den = d.x * e.y - d.y * e.x;
        if den == 0.0 {
            continue;
        }
        let w = Vec2::new(p.x - at.x, p.y - at.y);
        let t = (w.x * e.y - w.y * e.x) / den;
        let u = (w.x * d.y - w.y * d.x) / den;
        if t >= 0.0 && (0.0..=1.0).contains(&u) && t < best {
            best = t;
        }
    }
    best
}

/// Its pieces at 1:`scale`: its frame, its words' lines and its leader.
pub fn pieces(at: Vec2, spec: &Spec, scale: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    match spec.frame {
        Frame::Circle => out.push(piece(Shape::Circle {
            c: at,
            r: radius(spec, scale),
        })),
        Frame::None => {}
        Frame::Ellipse => {
            let (y1, ea, eb, _) = ellipse(spec, scale);
            let (major, ratio) = if ea >= eb {
                (Vec2::new(ea, 0.0), eb / ea)
            } else {
                (Vec2::new(0.0, eb), ea / eb)
            };
            out.push(piece(Shape::Ellipse {
                c: Vec2::new(at.x, at.y + y1),
                major,
                ratio,
                t0: 0.0,
                t1: std::f64::consts::TAU,
            }));
        }
        _ => {
            if let Some((pts, bulges)) = ring(at, spec, scale) {
                out.push(piece(Shape::Polygon {
                    pts,
                    bulges,
                    holes: None,
                    parts: None,
                }));
            }
        }
    }
    let h = height(spec, scale);
    for (i, line) in spec.text.split('\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        out.push(piece(Shape::Text {
            p: Vec2::new(at.x, at.y + line_y(spec, scale, i)),
            text: line.to_owned(),
            height: h,
            rotation: 0.0,
            align: Some(TextAlign::MiddleCenter),
            width_factor: None,
            // Over a mask of the drawing's ground: a door's tag on its wall stays readable (docs/adr/0714).
            mask: Some(true),
            box_width: None,
            line_spacing: None,
            runs: None,
            flow: None,
            face: Default::default(),
            path: None,
        }));
    }
    if spec.leader
        && let Some(to) = spec.anchor
    {
        let l = js_hypot(to.x - at.x, to.y - at.y);
        if l > 0.0 {
            let d = Vec2::new((to.x - at.x) / l, (to.y - at.y) / l);
            let t = exit(at, spec, scale, d);
            if t < l {
                out.push(piece(Shape::Line {
                    a: Vec2::new(at.x + d.x * t, at.y + d.y * t),
                    b: to,
                }));
            }
        }
    }
    out
}
