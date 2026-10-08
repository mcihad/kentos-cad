//! A leader's layout (docs/adr/0146 §2): where its arrowhead, its landing
//! and its note go, every length in the note's height. One function for
//! both platforms (the web through its WASM, `leaderLayout`);
//! scripts/fixtures/leader_cases.py is its independent reference
//! (fixtures/leader/v1/layout.json).

use crate::api::Op;
use crate::entity::{Entity, Shape, TextPlace};
use crate::geom::arrowhead::{ArrowKind, Arrowhead, arrowhead};
use crate::jsmath::{PI, cos, js_hypot, sin};
use crate::op;
use crate::text::TextAlign;
use crate::vec2::Vec2;

/// The landing's length.
const LANDING: f64 = 2.0;
/// The gap between the landing's end and the note.
const GAP: f64 = 0.5;

/// Where a leader's parts go.
#[derive(Clone, Debug, PartialEq)]
pub struct LeaderLayout {
    /// Its arrowhead (docs/adr/0205 §7), its length the note's height
    /// times the leader's arrowhead size.
    pub head: Arrowhead,
    /// Where its line starts: the tip, or on its first segment at the
    /// head's back (a closed, boxed or dotted head's line does not run
    /// through it).
    pub start: Vec2,
    /// The first of its vertices the line goes on to from `start`.
    pub first: usize,
    /// 1: the landing runs along the note's direction; −1: against it.
    pub side: f64,
    /// From the last vertex to its end; none without a note.
    pub landing: Option<[Vec2; 2]>,
    /// Where the note stands; none without a note.
    pub note_point: Option<Vec2>,
    /// Which point of the note that is: the middle of its left or of its right.
    pub note_align: Option<TextAlign>,
}

crate::json_struct!(out LeaderLayout { head, start, first, side, landing, note_point => "notePoint", note_align => "noteAlign" });

fn unit(v: Vec2) -> Option<Vec2> {
    let l = js_hypot(v.x, v.y);
    (l > 0.0).then(|| Vec2::new(v.x / l, v.y / l))
}

/// The layout of a leader through `pts` (the tip first) whose note is
/// `height` high and turned `rotation` degrees; `arrow` its arrowhead's name
/// (none: the filled triangle) and `arrow_size` its length in the note's
/// height (none: 1). None for a leader without a vertex.
pub fn layout(
    pts: &[Vec2],
    height: f64,
    rotation: f64,
    arrow: Option<&str>,
    arrow_size: Option<f64>,
    has_note: bool,
) -> Option<LeaderLayout> {
    let (&tip, &last) = (pts.first()?, pts.last()?);
    let r = rotation * PI / 180.0;
    let u = Vec2::new(cos(r), sin(r));
    // The first segment that has a length points the arrowhead; with none, the note's direction.
    let lead = pts[1..].iter().enumerate().find_map(|(i, q)| {
        let v = Vec2::new(q.x - tip.x, q.y - tip.y);
        unit(v).map(|d| (i + 1, d, js_hypot(v.x, v.y)))
    });
    let d = lead.map_or(u, |(_, d, _)| d);
    let h = height;
    let head = arrowhead(ArrowKind::of(arrow), tip, d, arrow_size.unwrap_or(1.0) * h);
    // The line leaves the head's back; a first segment no longer than that starts at its end.
    let (start, first) = match lead {
        Some((k, d, len)) if head.back > 0.0 && head.back < len => (
            Vec2::new(tip.x + d.x * head.back, tip.y + d.y * head.back),
            k,
        ),
        Some((k, _, _)) if head.back > 0.0 => (pts[k], k + 1),
        _ => (tip, 1),
    };
    // The last segment that has a length goes to the right (its projection on the note's direction 0 or more) or left.
    let side = pts[..pts.len() - 1]
        .iter()
        .rev()
        .find_map(|q| {
            let v = Vec2::new(last.x - q.x, last.y - q.y);
            unit(v).map(|_| {
                if v.x * u.x + v.y * u.y >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            })
        })
        .unwrap_or(1.0);
    let along = |k: f64| Vec2::new(last.x + side * u.x * k * h, last.y + side * u.y * k * h);
    Some(LeaderLayout {
        head,
        start,
        first,
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
        arrow_size,
        ..
    } = s
    else {
        return None;
    };
    layout(
        pts,
        *height,
        *rotation,
        arrow.as_deref(),
        *arrow_size,
        text.is_some(),
    )
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

/// The line it draws: from its start through its vertices and, with a
/// note, on to the landing's end.
pub fn drawn_path(pts: &[Vec2], l: &LeaderLayout) -> Vec<Vec2> {
    let mut out = Vec::with_capacity(pts.len() + 2);
    out.push(l.start);
    out.extend_from_slice(pts.get(l.first..).unwrap_or(&[]));
    if let Some([_, end]) = l.landing {
        out.push(end);
    }
    out
}

/// The points its arrowhead reaches.
pub fn head_reach(l: &LeaderLayout) -> Vec<Vec2> {
    l.head.reach().collect()
}

pub(crate) static OPS: &[Op] = &[op!("leaderLayout", |e: Entity| layout_of(&e.shape))];
