//! A leader's layout (docs/adr/0146 §2): where its arrowhead, its landing
//! and its note go, every length in the note's height. One function for
//! both platforms (the web through its WASM, `leaderLayout`);
//! scripts/fixtures/leader_cases.py is its independent reference
//! (fixtures/leader/v1/layout.json).

use crate::api::Op;
use crate::entity::{Entity, Shape, TextPlace};
use crate::jsmath::{PI, cos, js_hypot, sin};
use crate::op;
use crate::text::TextAlign;
use crate::vec2::Vec2;

/// The arrowhead's length, in the note's height (AutoCAD's: the arrow as long as the text is high).
const HEAD: f64 = 1.0;
/// Half the arrowhead's base: its base is a third of its length.
const HEAD_HALF: f64 = 1.0 / 6.0;
/// The dot's radius: its diameter is half the height.
const DOT: f64 = 0.25;
/// The landing's length.
const LANDING: f64 = 2.0;
/// The gap between the landing's end and the note.
const GAP: f64 = 0.5;
/// Segments of the dot's outline, a full turn's as the store draws a circle.
const DOT_SEGMENTS: usize = 72;

/// A leader's arrowhead, placed.
#[derive(Clone, Debug, PartialEq)]
pub enum Head {
    /// A filled triangle: the tip, then its base's two corners.
    Filled { triangle: [Vec2; 3] },
    /// The triangle's two sides: a base corner, the tip, the other base corner.
    Open { lines: [Vec2; 3] },
    /// A filled circle about the tip.
    Dot { center: Vec2, radius: f64 },
    /// No arrowhead.
    None {},
}

crate::json_tagged!(Head, "kind",
    Filled => "filled" { triangle },
    Open => "open" { lines },
    Dot => "dot" { center, radius },
    None => "none" {},
);

/// Where a leader's parts go.
#[derive(Clone, Debug, PartialEq)]
pub struct LeaderLayout {
    pub head: Head,
    /// 1: the landing runs along the note's direction; −1: against it.
    pub side: f64,
    /// From the last vertex to its end; none without a note.
    pub landing: Option<[Vec2; 2]>,
    /// Where the note stands; none without a note.
    pub note_point: Option<Vec2>,
    /// Which point of the note that is: the middle of its left or of its right.
    pub note_align: Option<TextAlign>,
}

crate::json_struct!(out LeaderLayout { head, side, landing, note_point => "notePoint", note_align => "noteAlign" });

fn unit(v: Vec2) -> Option<Vec2> {
    let l = js_hypot(v.x, v.y);
    (l > 0.0).then(|| Vec2::new(v.x / l, v.y / l))
}

/// The layout of a leader through `pts` (the tip first) whose note is
/// `height` high and turned `rotation` degrees; `arrow` its arrowhead's name
/// (`open`, `dot`, `none`; none or any other: a filled arrow). None for a
/// leader without a vertex.
pub fn layout(
    pts: &[Vec2],
    height: f64,
    rotation: f64,
    arrow: Option<&str>,
    has_note: bool,
) -> Option<LeaderLayout> {
    let (&tip, &last) = (pts.first()?, pts.last()?);
    let r = rotation * PI / 180.0;
    let u = Vec2::new(cos(r), sin(r));
    // The first segment that has a length points the arrowhead; with none, the note's direction.
    let d = pts[1..]
        .iter()
        .find_map(|q| unit(Vec2::new(q.x - tip.x, q.y - tip.y)))
        .unwrap_or(u);
    let n = Vec2::new(-d.y, d.x);
    let h = height;
    let base = Vec2::new(tip.x + d.x * HEAD * h, tip.y + d.y * HEAD * h);
    let left = Vec2::new(base.x + n.x * HEAD_HALF * h, base.y + n.y * HEAD_HALF * h);
    let right = Vec2::new(base.x - n.x * HEAD_HALF * h, base.y - n.y * HEAD_HALF * h);
    let head = match arrow {
        Some("open") => Head::Open {
            lines: [left, tip, right],
        },
        Some("dot") => Head::Dot {
            center: tip,
            radius: DOT * h,
        },
        Some("none") => Head::None {},
        _ => Head::Filled {
            triangle: [tip, left, right],
        },
    };
    // The last segment that has a length goes to the right (its projection on the note's direction 0 or more) or left.
    let side = pts[..pts.len() - 1]
        .iter()
        .rev()
        .find_map(|q| {
            let v = Vec2::new(last.x - q.x, last.y - q.y);
            unit(v).map(|_| if v.x * u.x + v.y * u.y >= 0.0 { 1.0 } else { -1.0 })
        })
        .unwrap_or(1.0);
    let along = |k: f64| Vec2::new(last.x + side * u.x * k * h, last.y + side * u.y * k * h);
    Some(LeaderLayout {
        head,
        side,
        landing: has_note.then(|| [last, along(LANDING)]),
        note_point: has_note.then(|| along(LANDING + GAP)),
        note_align: has_note.then_some(if side > 0.0 {
            TextAlign::MiddleLeft
        } else {
            TextAlign::MiddleRight
        }),
    })
}

/// A leader shape's layout; none for any other shape.
pub fn layout_of(s: &Shape) -> Option<LeaderLayout> {
    let Shape::Leader {
        pts,
        text,
        height,
        rotation,
        arrow,
        ..
    } = s
    else {
        return None;
    };
    layout(pts, *height, *rotation, arrow.as_deref(), text.is_some())
}

/// Its note as a text stands (docs/adr/0146 §2): at the note's point,
/// aligned on the middle of its left or right, its height and turn. None
/// for any other shape and for a leader without a note.
pub fn note_place(s: &Shape) -> Option<TextPlace<'_>> {
    let Shape::Leader {
        text: Some(text),
        height,
        rotation,
        ..
    } = s
    else {
        return None;
    };
    let l = layout_of(s)?;
    Some(TextPlace::line(
        l.note_point?,
        text,
        *height,
        *rotation,
        l.note_align,
        None,
    ))
}

/// The line it draws: through its vertices and, with a note, on to the landing's end.
pub fn drawn_path(pts: &[Vec2], l: &LeaderLayout) -> Vec<Vec2> {
    let mut out = pts.to_vec();
    if let Some([_, end]) = l.landing {
        out.push(end);
    }
    out
}

/// The arrowhead's area: the filled triangle, or the dot's outline; none
/// for an open arrowhead and for none.
pub fn head_ring(head: &Head) -> Option<Vec<Vec2>> {
    match head {
        Head::Filled { triangle } => Some(triangle.to_vec()),
        Head::Dot { center, radius } => Some(
            (0..DOT_SEGMENTS)
                .map(|i| {
                    let t = 2.0 * PI * i as f64 / DOT_SEGMENTS as f64;
                    Vec2::new(center.x + radius * cos(t), center.y + radius * sin(t))
                })
                .collect(),
        ),
        Head::Open { .. } | Head::None {} => None,
    }
}

/// The points its arrowhead reaches: the triangle's corners, the dot's box.
pub fn head_reach(head: &Head) -> Vec<Vec2> {
    match head {
        Head::Filled { triangle } => triangle.to_vec(),
        Head::Open { lines } => lines.to_vec(),
        Head::Dot { center, radius } => vec![
            Vec2::new(center.x - radius, center.y - radius),
            Vec2::new(center.x + radius, center.y + radius),
        ],
        Head::None {} => Vec::new(),
    }
}

pub(crate) static OPS: &[Op] = &[op!("leaderLayout", |e: Entity| layout_of(&e.shape))];
