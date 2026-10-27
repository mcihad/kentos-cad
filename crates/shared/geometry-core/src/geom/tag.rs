//! Tags (docs/adr/0344 §1): their drawing from their words, frame and leader, sized on paper at the view's scale.
//!
//! - Its words: each line centred on the tag's middle, `h` high (its size on paper at the scale), a line pitch of
//!   `PITCH × h`, the block's middle on the tag's.
//! - Its frame round the words with a pad of `PAD × h`: a circle round their box, a hexagon (the box with a point each
//!   side as deep as half its height times tan 30°), a rectangle, a rectangle with quarter-circle corners of half its
//!   lesser half-size, or none. Words that are empty still have a frame as wide as `h`, so the tag can be seen and picked.
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

/// A tag's frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Frame {
    #[default]
    Circle,
    Hexagon,
    Rectangle,
    Rounded,
    None,
}

impl Frame {
    fn of(s: &str) -> Option<Frame> {
        Some(match s {
            "circle" => Frame::Circle,
            "hexagon" => Frame::Hexagon,
            "rectangle" => Frame::Rectangle,
            "rounded" => Frame::Rounded,
            "none" => Frame::None,
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
        Frame::Circle | Frame::None => return None,
    })
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
    let (_, b) = half_box(spec, scale);
    for (i, line) in spec.text.split('\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        out.push(piece(Shape::Text {
            p: Vec2::new(at.x, at.y + b - h / 2.0 - i as f64 * PITCH * h),
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
