//! Alan böl's Hedef alan (H, docs/adr/1069): the area cut by a line parallel to one of its edges, the piece beside the
//! edge of the size typed (the core's `ops::area_cut`). The edge is clicked; the size is typed in the project's area
//! unit; the cut, the two pieces and their sizes are shown; Karşı taraf (T) gives the size to the piece across from the
//! edge; Geri (G) goes back a step; Enter cuts as Alan böl's line does (`cad.areas.edit`, one undo step).
//!
//! Stage one: one area of straight edges with no holes, cut by one straight chord.

use kentos_geometry_core::geom::arrangement::{Area, Source};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::region::{net_area, split_area};
use kentos_geometry_core::ops::area_cut::{Cut, MORE_PIECES, cut_for_area};

use crate::Vec2;
use crate::format::Format;
use crate::tool::{self, Preview, Stroke, Tag, Tone};

pub const STRAIGHT_ONLY: &str = "Hedef alan şimdilik tek bir deliksiz, düz kenarlı alanda çalışır; yaylı ya da delikli alanı çizgiyle kesin.";
pub const ONE_AREA: &str = "Hedef alanla bölmek için tek bir alan seçin.";
pub const NO_EDGE_HERE: &str =
    "Tıklanan yerde alanın kenarı yok; seçili alanın bir kenarına tıklayın.";
/// How far the cut is drawn past the boundary, so that the command meets it there whatever the last digit.
const PAST: f64 = 0.01;

/// The reference edge on the area's outer ring.
#[derive(Clone, Debug)]
struct Edged {
    edge: usize,
    a: Vec2,
    b: Vec2,
}

/// The cut asked for: the size typed, the cut, the two pieces (the edge's first).
#[derive(Clone, Debug)]
struct Asked {
    target: f64,
    cut: Cut,
    pieces: Vec<Area>,
}

/// Hedef alan's state over the one area being cut.
#[derive(Clone, Debug)]
pub struct ByArea {
    area: Area,
    total: f64,
    edge: Option<Edged>,
    /// Karşı taraf (T): the size typed is the piece across from the edge.
    across: bool,
    asked: Option<Asked>,
}

impl ByArea {
    /// Over `area`, or why it cannot be cut so.
    pub fn over(area: &Area) -> Result<Self, &'static str> {
        let curved = area
            .outer
            .bulges
            .as_ref()
            .is_some_and(|b| b.iter().any(|&v| v != 0.0));
        if curved || !area.holes.is_empty() || area.outer.pts.len() < 3 {
            return Err(STRAIGHT_ONLY);
        }
        Ok(Self {
            area: area.clone(),
            total: net_area(area),
            edge: None,
            across: false,
            asked: None,
        })
    }

    pub fn has_edge(&self) -> bool {
        self.edge.is_some()
    }

    pub fn has_cut(&self) -> bool {
        self.asked.is_some()
    }

    pub fn across(&self) -> bool {
        self.across
    }

    /// The outer ring's edge nearest to `p` within `tolerance`, taken; false when none is that near.
    pub fn pick_edge(&mut self, p: Vec2, tolerance: f64) -> bool {
        let pts = &self.area.outer.pts;
        let n = pts.len();
        let near = (0..n)
            .map(|i| (i, seg_distance(p, pts[i], pts[(i + 1) % n])))
            .filter(|&(_, d)| d <= tolerance)
            .min_by(|x, y| x.1.total_cmp(&y.1));
        let Some((edge, _)) = near else { return false };
        self.edge = Some(Edged {
            edge,
            a: pts[edge],
            b: pts[(edge + 1) % n],
        });
        self.asked = None;
        true
    }

    /// The cut leaving `target` m² (beside the edge, or across with Karşı taraf), checked against the cut the command
    /// makes: two pieces. Why not, when it cannot be.
    pub fn ask(&mut self, target: f64) -> Result<(), &'static str> {
        let Some(e) = &self.edge else { return Ok(()) };
        let beside = if self.across {
            self.total - target
        } else {
            target
        };
        let cut = cut_for_area(&self.area.outer.pts, e.edge, beside)?;
        let mut pieces = split_area(&self.area, &line_source(&cut));
        if pieces.len() != 2 {
            return Err(MORE_PIECES);
        }
        // The piece beside the edge first: the one of the core's size, else (two equal halves) the one whose middle
        // lies nearer the edge's line.
        let off = |a: &Area| (net_area(a) - cut.near).abs();
        let gap = off(&pieces[1]) - off(&pieces[0]);
        let second = if gap.abs() > 1e-6 * self.total.max(1.0) {
            gap < 0.0
        } else {
            from_edge(e, middle(&pieces[1])) < from_edge(e, middle(&pieces[0]))
        };
        if second {
            pieces.swap(0, 1);
        }
        self.asked = Some(Asked {
            target,
            cut,
            pieces,
        });
        Ok(())
    }

    /// Karşı taraf: the other piece gets the size, the cut worked out again.
    pub fn turn(&mut self) -> Result<(), &'static str> {
        self.across = !self.across;
        match self.asked.as_ref().map(|a| a.target) {
            Some(target) => self.ask(target),
            None => Ok(()),
        }
    }

    /// Geri: the cut goes, then the edge; false with neither.
    pub fn back(&mut self) -> bool {
        if self.asked.take().is_some() {
            return true;
        }
        self.edge.take().is_some()
    }

    /// The line the command cuts with: the cut drawn a little past the boundary at both ends.
    pub fn line(&self) -> Option<Vec<Vec2>> {
        let (a, b) = past(&self.asked.as_ref()?.cut);
        Some(vec![a, b])
    }

    /// The area, its edge, the cut, the piece asked for filled and the rest faint, with the sizes.
    pub fn preview(&self, format: &Format) -> Preview {
        let mut out = Preview::default();
        out.areas.push(tool::Area {
            rings: rings(&self.area),
            fill: 0.0,
            width: 2.0,
            dash: None,
            fill_tone: Tone::Accent,
        });
        let Some(e) = &self.edge else { return out };
        out.strokes.push(
            Stroke::solid(vec![e.a, e.b], false)
                .width(3.0)
                .tone(Tone::Snap),
        );
        let Some(asked) = &self.asked else {
            out.tag = Some(Tag {
                at: Vec2::new((e.a.x + e.b.x) / 2.0, (e.a.y + e.b.y) / 2.0),
                lines: vec![format!("alan {}", format.area(self.total))],
            });
            return out;
        };
        let wanted = usize::from(self.across);
        for (i, piece) in asked.pieces.iter().enumerate() {
            out.areas.push(tool::Area {
                rings: rings(piece),
                fill: if i == wanted { 0.28 } else { 0.10 },
                width: 1.0,
                dash: Some([4.0, 3.0]),
                fill_tone: if i == wanted {
                    Tone::Accent
                } else {
                    Tone::Snap
                },
            });
        }
        out.strokes.push(
            Stroke::solid(vec![asked.cut.a, asked.cut.b], false)
                .width(2.0)
                .tone(Tone::Danger),
        );
        let (near, far) = (asked.cut.near, asked.cut.far);
        let (asked_size, rest) = if self.across {
            (far, near)
        } else {
            (near, far)
        };
        out.tag = Some(Tag {
            at: Vec2::new(
                (asked.cut.a.x + asked.cut.b.x) / 2.0,
                (asked.cut.a.y + asked.cut.b.y) / 2.0,
            ),
            lines: vec![
                format!("istenen {}", format.area(asked_size)),
                format!("kalan {}", format.area(rest)),
                format!("kenara uzaklık {}", format.length(asked.cut.distance)),
            ],
        });
        out
    }
}

/// A point's distance from the segment a–b.
fn seg_distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.x - (a.x + t * dx)).hypot(p.y - (a.y + t * dy))
}

/// A point's distance from the reference edge's line.
fn from_edge(e: &Edged, p: Vec2) -> f64 {
    let (dx, dy) = (e.b.x - e.a.x, e.b.y - e.a.y);
    ((p.x - e.a.x) * dy - (p.y - e.a.y) * dx).abs() / dx.hypot(dy)
}

/// The middle of an area's outer ring (its vertices' mean: enough to tell two pieces apart across a straight cut).
fn middle(a: &Area) -> Vec2 {
    let n = a.outer.pts.len().max(1) as f64;
    let (x, y) = a
        .outer
        .pts
        .iter()
        .fold((0.0, 0.0), |(x, y), p| (x + p.x, y + p.y));
    Vec2::new(x / n, y / n)
}

/// The cut drawn a little past the boundary at both ends, along its own line.
fn past(cut: &Cut) -> (Vec2, Vec2) {
    let (dx, dy) = (cut.b.x - cut.a.x, cut.b.y - cut.a.y);
    let l = dx.hypot(dy);
    let (ux, uy) = (dx / l * PAST, dy / l * PAST);
    (
        Vec2::new(cut.a.x - ux, cut.a.y - uy),
        Vec2::new(cut.b.x + ux, cut.b.y + uy),
    )
}

/// The cut as the command cuts with it (Alan böl's straight line).
fn line_source(cut: &Cut) -> Source {
    let (a, b) = past(cut);
    Source {
        edges: vec![Edge::Seg { a, b }],
        points: Some(vec![a, b]),
        cut: Some(true),
    }
}

/// An area's rings as the preview draws them.
fn rings(a: &Area) -> Vec<Vec<Vec2>> {
    std::iter::once(&a.outer)
        .chain(&a.holes)
        .map(|r| kentos_geometry_core::entity::polygon_ring(&r.pts, r.bulges.as_deref()))
        .collect()
}
