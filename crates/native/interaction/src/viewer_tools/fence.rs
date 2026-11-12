//! Çit, Buda and Uzat's fourth way of picking (docs/adr/0140): a fence is
//! drawn (clicks, snapping, ortho and polar tracking from the last one), and
//! every object it crosses is trimmed where it crosses, or extended at the
//! end nearest the crossing. The ribbon starts the method by sending its
//! letter right after the tool, and the same letter works typed: `C`; `K`
//! goes back to clicking on the edge.
//!
//! This is the fence itself: its points, what it crosses, and how it looks.
//! What is done to the crossed objects is Buda's and Uzat's
//! ([`crate::trim`]). The crossings are the shared core's
//! (`ops::trim::fence_crossings`).

use kentos_domain::Slot;
use kentos_geometry_core::geometry::{Bounds, dist};
use kentos_geometry_core::ops::trim::fence_crossings;

use crate::Tracking;
use crate::Vec2;
use crate::edge;
use crate::points::{self, constrain};
use crate::tool::{Context, Marker, MarkerShape, Pointer, Preview, Stroke, Tag, Tone};

/// The fence's dash and gap, logical pixels.
const DASH: [f32; 2] = [7.0, 4.0];
/// Crossings marked at most, in the preview.
const MAX_MARKED: usize = 400;

/// The objects the fence crosses and where, in the fence's order: only what
/// the user may edit (not on a locked layer) and sees.
pub(crate) fn crossed(fence: &[Vec2], cx: &Context<'_>) -> Vec<(Slot, Vec<Vec2>)> {
    let Some(first) = fence.first() else {
        return Vec::new();
    };
    let mut bounds = Bounds {
        min_x: first.x,
        min_y: first.y,
        max_x: first.x,
        max_y: first.y,
    };
    for p in fence {
        bounds.min_x = bounds.min_x.min(p.x);
        bounds.min_y = bounds.min_y.min(p.y);
        bounds.max_x = bounds.max_x.max(p.x);
        bounds.max_y = bounds.max_y.max(p.y);
    }
    let doc = &*cx.doc;
    cx.spatial
        .in_rect(
            Vec2::new(bounds.min_x, bounds.min_y),
            Vec2::new(bounds.max_x, bounds.max_y),
            true,
        )
        .into_iter()
        .filter_map(|slot| {
            let e = doc.get(slot).filter(|e| edge::unlocked(e, doc))?;
            let at = fence_crossings(&edge::core(e), fence);
            (!at.is_empty()).then_some((slot, at))
        })
        .collect()
}

/// A fence being drawn.
#[derive(Clone, Debug, Default)]
pub(crate) struct Fence {
    /// The method is on: clicks are fence points.
    pub on: bool,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    tracking: Option<Tracking>,
    /// Where the fence would cross, cursor included, and how many objects.
    marks: Vec<Vec2>,
    objects: usize,
}

impl Fence {
    pub fn start(&mut self) {
        *self = Self {
            on: true,
            ..Self::default()
        };
    }

    pub fn stop(&mut self) {
        *self = Self::default();
    }

    pub fn points(&self) -> &[Vec2] {
        &self.pts
    }

    pub fn last(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    pub fn clear(&mut self) {
        self.pts.clear();
        self.marks.clear();
        self.objects = 0;
    }

    /// Takes the last point back; false when there is none.
    pub fn pop(&mut self, cx: &Context<'_>) -> bool {
        let popped = self.pts.pop().is_some();
        self.refresh(cx);
        popped
    }

    /// The fence's next point at the pointer: ortho and polar tracking from
    /// the last one, a snapped point exact.
    pub fn constrain(&mut self, p: &Pointer, cx: &Context<'_>) -> Vec2 {
        let (point, tracking) = constrain(self.last(), p, cx);
        self.tracking = tracking;
        point
    }

    /// The pointer moved: the cursor's segment and what it would cross.
    pub fn moved(&mut self, p: &Pointer, cx: &Context<'_>) {
        self.hover = Some(self.constrain(p, cx));
        self.refresh(cx);
    }

    /// A point of the fence, unless it is on the last.
    pub fn add(&mut self, p: Vec2, cx: &Context<'_>) {
        if self.last().is_none_or(|q| dist(q, p) > points::SAME) {
            self.pts.push(p);
        }
        self.refresh(cx);
    }

    /// What the fence, with the cursor's segment, crosses now.
    fn refresh(&mut self, cx: &Context<'_>) {
        let mut line = self.pts.clone();
        line.extend(
            self.hover
                .filter(|h| self.last().is_none_or(|q| dist(q, *h) > points::SAME)),
        );
        let hits = if line.len() >= 2 {
            crossed(&line, cx)
        } else {
            Vec::new()
        };
        self.objects = hits.len();
        self.marks = hits
            .into_iter()
            .flat_map(|(_, at)| at)
            .take(MAX_MARKED)
            .collect();
    }

    /// The fence drawn dashed with its cursor segment, and a mark where it
    /// crosses: a cross for what a trim takes away, a plus for what an
    /// extend reaches.
    pub fn preview(&self, trimming: bool) -> Preview {
        let mut line = self.pts.clone();
        line.extend(self.hover);
        let strokes = if line.len() >= 2 {
            vec![Stroke::dashed(line, false, DASH).width(1.5)]
        } else {
            Vec::new()
        };
        let (shape, tone) = if trimming {
            (MarkerShape::Cross(5.0), Tone::Danger)
        } else {
            (MarkerShape::Plus(5.0), Tone::Accent)
        };
        Preview {
            strokes,
            markers: self
                .marks
                .iter()
                .map(|&at| Marker { at, shape, tone })
                .collect(),
            tag: self.hover.filter(|_| !self.pts.is_empty()).map(|at| Tag {
                at,
                lines: vec![match self.objects {
                    0 => "Kesişen nesne yok".to_owned(),
                    n => format!("{n} nesne"),
                }],
            }),
            tracking: self.tracking,
            ..Preview::default()
        }
    }
}
