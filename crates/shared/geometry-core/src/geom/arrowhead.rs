//! Arrowheads (docs/adr/0205 §7): AutoCAD's leader arrowheads, each a few
//! filled areas and lines about the tip, every length in the arrowhead's
//! length L. A filled head is a solid area (never stroked: its tip stays
//! sharp); a blank one is drawn as lines. The line to the tip starts at the
//! head's back for closed and boxed heads, at a dot's circle, and at the tip
//! for the open ones. scripts/fixtures/leader_cases.py is the independent
//! reference (fixtures/leader/v1/layout.json).
//!
//! In the head's own frame x runs from the tip along the line (back into
//! the leader) and y a quarter turn counter-clockwise from x; a point (x, y)
//! is `tip + d·x·L + n·y·L`.

use crate::jsmath::{PI, cos, sin, tan};
use crate::vec2::Vec2;

/// Segments of a full circle, as the store draws one.
const CIRCLE_SEGMENTS: usize = 72;

/// An arrowhead's kind by its name in the file (`LeaderArrow`'s); no name
/// or an unknown one: the filled triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowKind {
    /// Dolu üçgen (AutoCAD's Closed filled): the default.
    ClosedFilled,
    /// Boş üçgen (Closed blank).
    ClosedBlank,
    /// Açık ok (Open).
    Open,
    /// İnce açık ok (Open 30).
    Open30,
    /// Dik açık ok (Open 90, Right angle).
    Open90,
    /// Dolu nokta (Dot).
    Dot,
    /// Küçük nokta (Dot small).
    DotSmall,
    /// Boş nokta (Dot blank).
    DotBlank,
    /// Eğik çizgi (Oblique).
    Oblique,
    /// Mimari çentik (Architectural tick).
    ArchTick,
    /// Dolu kare (Box filled).
    BoxFilled,
    /// Boş kare (Box blank).
    BoxBlank,
    /// Dayanak üçgeni (Datum triangle filled).
    DatumFilled,
    /// Yok.
    None,
}

impl ArrowKind {
    /// The kind a leader's `arrow` names (docs/adr/0146 §1, 0205 §7).
    pub fn of(name: Option<&str>) -> ArrowKind {
        match name {
            Some("closed") => ArrowKind::ClosedBlank,
            Some("open") => ArrowKind::Open,
            Some("open30") => ArrowKind::Open30,
            Some("open90") => ArrowKind::Open90,
            Some("dot") => ArrowKind::Dot,
            Some("dotSmall") => ArrowKind::DotSmall,
            Some("dotBlank") => ArrowKind::DotBlank,
            Some("oblique") => ArrowKind::Oblique,
            Some("archTick") => ArrowKind::ArchTick,
            Some("boxFilled") => ArrowKind::BoxFilled,
            Some("boxBlank") => ArrowKind::BoxBlank,
            Some("datumFilled") => ArrowKind::DatumFilled,
            Some("none") => ArrowKind::None,
            _ => ArrowKind::ClosedFilled,
        }
    }
}

/// A line of an arrowhead: its points, and whether it closes on its first.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadLine {
    pub pts: Vec<Vec2>,
    pub closed: bool,
}

crate::json_struct!(out HeadLine { pts, closed });

/// An arrowhead, placed: its filled areas (one ring each) and its lines;
/// `back` is how far from the tip along the line the leader's line starts
/// (0 at the tip), in metres.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrowhead {
    pub fills: Vec<Vec<Vec2>>,
    pub lines: Vec<HeadLine>,
    pub back: f64,
}

crate::json_struct!(out Arrowhead { fills, lines, back });

/// A circle of radius `r` (in L) about the frame's (x, 0), counter-clockwise
/// from the frame's x axis, in the frame.
fn circle(x: f64, r: f64) -> Vec<(f64, f64)> {
    (0..CIRCLE_SEGMENTS)
        .map(|i| {
            let t = 2.0 * PI * i as f64 / CIRCLE_SEGMENTS as f64;
            (x + r * cos(t), r * sin(t))
        })
        .collect()
}

/// The arrowhead of `kind` at `tip`, its line leaving along the unit `d`,
/// `length` (L) long.
pub fn arrowhead(kind: ArrowKind, tip: Vec2, d: Vec2, length: f64) -> Arrowhead {
    let n = Vec2::new(-d.y, d.x);
    let l = length;
    let at = |(x, y): (f64, f64)| {
        Vec2::new(
            tip.x + d.x * x * l + n.x * y * l,
            tip.y + d.y * x * l + n.y * y * l,
        )
    };
    let ring = |pts: &[(f64, f64)]| pts.iter().copied().map(at).collect::<Vec<_>>();
    let line = |pts: &[(f64, f64)], closed: bool| HeadLine {
        pts: ring(pts),
        closed,
    };
    // The closed heads' sides (AutoCAD's: the base a third of the length).
    let triangle = [(0.0, 0.0), (1.0, 1.0 / 6.0), (1.0, -1.0 / 6.0)];
    let square = [(-0.25, -0.25), (0.25, -0.25), (0.25, 0.25), (-0.25, 0.25)];
    let (fills, lines, back) = match kind {
        ArrowKind::ClosedFilled => (vec![ring(&triangle)], vec![], 1.0),
        ArrowKind::ClosedBlank => (vec![], vec![line(&triangle, true)], 1.0),
        ArrowKind::Open => (
            vec![],
            vec![line(
                &[(1.0, 1.0 / 6.0), (0.0, 0.0), (1.0, -1.0 / 6.0)],
                false,
            )],
            0.0,
        ),
        // Its sides 15° off the line: a 30° opening.
        ArrowKind::Open30 => {
            let t = tan(PI / 12.0);
            (
                vec![],
                vec![line(&[(1.0, t), (0.0, 0.0), (1.0, -t)], false)],
                0.0,
            )
        }
        // A right angle: its sides 45° off the line, half the length deep.
        ArrowKind::Open90 => (
            vec![],
            vec![line(&[(0.5, 0.5), (0.0, 0.0), (0.5, -0.5)], false)],
            0.0,
        ),
        ArrowKind::Dot => (vec![ring(&circle(0.0, 0.25))], vec![], 0.25),
        ArrowKind::DotSmall => (vec![ring(&circle(0.0, 0.125))], vec![], 0.125),
        ArrowKind::DotBlank => (vec![], vec![line(&circle(0.0, 0.25), true)], 0.25),
        // A tick across the line at 45°, through the tip.
        ArrowKind::Oblique => (vec![], vec![line(&[(-0.5, -0.5), (0.5, 0.5)], false)], 0.0),
        // The tick as a band an eighth of the length wide.
        ArrowKind::ArchTick => {
            let w = 1.0 / 16.0 / core::f64::consts::SQRT_2;
            (
                vec![ring(&[
                    (-0.5 + w, -0.5 - w),
                    (0.5 + w, 0.5 - w),
                    (0.5 - w, 0.5 + w),
                    (-0.5 - w, -0.5 + w),
                ])],
                vec![],
                0.0,
            )
        }
        ArrowKind::BoxFilled => (vec![ring(&square)], vec![], 0.25),
        ArrowKind::BoxBlank => (vec![], vec![line(&square, true)], 0.25),
        // Its base across the tip, its apex a length back along the line.
        ArrowKind::DatumFilled => (
            vec![ring(&[(0.0, -0.5), (1.0, 0.0), (0.0, 0.5)])],
            vec![],
            1.0,
        ),
        ArrowKind::None => (vec![], vec![], 0.0),
    };
    Arrowhead {
        fills,
        lines,
        back: back * l,
    }
}

impl Arrowhead {
    /// The points it reaches: its areas' corners and its lines' points.
    pub fn reach(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.fills
            .iter()
            .flatten()
            .chain(self.lines.iter().flat_map(|l| l.pts.iter()))
            .copied()
    }
}
