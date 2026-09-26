//! Dik in and Dik çık: the web's `PerpendicularInTool` and
//! `PerpendicularOutTool` (Netcad's perpendiculars,
//! `apps/web/src/tools/perpTools.ts`) on their shared base, step for step
//! (docs/adr/0057). In surveying terms: dik ayak along the reference line
//! from its start, dik boy square to it, positive to the right.
//!
//! - First the reference: the straight edge clicked (Dik in also takes an
//!   arc or a circle, the perpendicular then runs through its centre); its
//!   start (A) is the end nearer the click. Başka hat (H) picks another.
//! - Dik in: every point shown (snaps apply) or typed drops a perpendicular
//!   onto the line or its extension: “Dik inildi: dik boy 7.000 m.”
//! - Dik çık: the foot on the line (shown, or its distance from A typed),
//!   then the length (shown, or typed: right plus, left minus): “Dik
//!   çıkıldı: dik ayak 5.000 m, dik boy -4.000 m.”
//! - Enter, a quick right click or Esc leaves.
//!
//! Every perpendicular is a line written through `cad.entities.create`, its
//! own object and one undo step named after the tool. The feet and the
//! offsets are the shared core's (`side_offsets`, `side_point`,
//! `radial_point`).

use kentos_contracts::{CreateOperation, Entity, EntityGeometry};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge};
use kentos_geometry_core::geom::survey::{side_offsets, side_point};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::tools::drawing::radial_point;
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, Label, Marker, MarkerShape, Pointer, Preview, Stroke, Tag, Tone, Tool,
};

/// The tools' ids: their commands are `tool.perpIn` and `tool.perpOut`.
pub const IN_ID: &str = "perpIn";
pub const IN_LABEL: &str = "Dik in";
pub const OUT_ID: &str = "perpOut";
pub const OUT_LABEL: &str = "Dik çık";

/// How far the reference line's extension runs either way, in view diagonals (the web's `drawRef`).
const REACH: f64 = 2.0;

/// The reference: a straight edge from its start `a` (the end nearer the
/// click) towards `b`, or a circle's (an arc's) centre and radius.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Reference {
    Seg { a: Vec2, b: Vec2 },
    Arc { c: Vec2, r: f64 },
}

impl Reference {
    /// The edge of `e` nearest to `p`, oriented from the end nearer to `p`
    /// (the web's `refAt`); straight edges only for Dik çık.
    fn at(e: &Entity, p: Vec2, straight_only: bool) -> Option<Reference> {
        let mut best: Option<(Edge, f64)> = None;
        for edge in entity_edges(&shape(e)) {
            if straight_only && !matches!(edge, Edge::Seg { .. }) {
                continue;
            }
            let d = closest_on_edge(&edge, p).d;
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((edge, d));
            }
        }
        match best?.0 {
            Edge::Arc { c, r, .. } => Some(Reference::Arc { c, r }),
            Edge::Seg { a, b } => {
                let (a, b) = if dist(a, p) <= dist(b, p) {
                    (a, b)
                } else {
                    (b, a)
                };
                (dist(a, b) > 1e-9).then_some(Reference::Seg { a, b })
            }
        }
    }

    /// The foot of the perpendicular from `p`: on the unbounded line, or on
    /// the circle towards the centre (none at the centre).
    fn foot(self, p: Vec2) -> Option<Vec2> {
        match self {
            Reference::Seg { a, b } => {
                let o = side_offsets(a, b, p)?;
                side_point(a, b, o.absis, 0.0)
            }
            Reference::Arc { c, r } => radial_point(c, r, p),
        }
    }
}

/// Every object with an edge can be the reference, on any layer (the web's `pickEdge`).
fn any(_: &Entity, _: &Document) -> bool {
    true
}

/// What writing a perpendicular came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Added {
    Written,
    /// It has no length: nothing is written.
    Short,
    /// The command refused and said why.
    Refused,
}

/// Dik in or Dik çık.
#[derive(Clone, Debug)]
pub struct Perpendicular {
    out: bool,
    reference: Option<Reference>,
    /// The pointer's world point once a reference is picked.
    hover: Option<Vec2>,
    /// Dik çık: the foot's distance from the start, once given.
    absis: Option<f64>,
    reach: f64,
}

impl Perpendicular {
    pub fn perpendicular_in() -> Self {
        Self::new(false)
    }

    pub fn perpendicular_out() -> Self {
        Self::new(true)
    }

    fn new(out: bool) -> Self {
        Self {
            out,
            reference: None,
            hover: None,
            absis: None,
            reach: 0.0,
        }
    }

    fn name(&self) -> &'static str {
        if self.out { OUT_LABEL } else { IN_LABEL }
    }

    /// The perpendicular from `a` to `b`, one undo step named after the tool.
    fn add_line(&self, a: Vec2, b: Vec2, cx: &mut Context<'_>) -> Added {
        if dist(a, b) < 1e-9 {
            return Added::Short;
        }
        let operation = if self.out {
            CreateOperation::PerpendicularOut
        } else {
            CreateOperation::PerpendicularIn
        };
        let line = EntityGeometry::Line {
            a: wire(a),
            b: wire(b),
        };
        match points::write_objects(vec![line], Some(operation), cx) {
            Some(_) => Added::Written,
            None => Added::Refused,
        }
    }

    /// A point shown or typed against the picked reference.
    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some(reference) = self.reference else {
            return;
        };
        if !self.out {
            let Some(foot) = reference.foot(p) else {
                cx.say(
                    Level::Warn,
                    "Bu noktadan dik inilemez (nokta dairenin merkezinde).",
                );
                return;
            };
            match self.add_line(p, foot, cx) {
                Added::Short => cx.say(Level::Warn, "Nokta zaten hattın üzerinde."),
                Added::Written => {
                    let line =
                        format!("Dik inildi: dik boy {}.", cx.format().length(dist(p, foot)));
                    cx.say(Level::Success, line);
                }
                Added::Refused => {}
            }
            return;
        }
        let Reference::Seg { a, b } = reference else {
            return;
        };
        let Some(o) = side_offsets(a, b, p) else {
            return;
        };
        match self.absis {
            None => self.absis = Some(o.absis),
            Some(_) => self.place(o.ordinat, cx),
        }
    }

    /// Dik çık: the perpendicular of length `ordinat` at the foot given.
    fn place(&mut self, ordinat: f64, cx: &mut Context<'_>) {
        let (Some(Reference::Seg { a, b }), Some(absis)) = (self.reference, self.absis) else {
            return;
        };
        let (Some(foot), Some(end)) = (
            side_point(a, b, absis, 0.0),
            side_point(a, b, absis, ordinat),
        ) else {
            return;
        };
        match self.add_line(foot, end, cx) {
            Added::Short => cx.say(Level::Warn, "Dik boy sıfır olamaz."),
            Added::Refused => {}
            Added::Written => {
                let f = cx.format();
                let line = format!(
                    "Dik çıkıldı: dik ayak {}, dik boy {}.",
                    f.length(absis),
                    f.length(ordinat)
                );
                cx.say(Level::Success, line);
                // The next perpendicular on the same line.
                self.absis = None;
            }
        }
    }

    /// The reference drawn in the snap colour (the web's `drawRef`): a circle
    /// dashed; a line solid between its ends, dashed past them, “A” at its start.
    fn reference_preview(&self, preview: &mut Preview) {
        match self.reference {
            Some(Reference::Arc { c, r }) => {
                let circle =
                    Outline::of(&Shape::Circle { c, r }, Some([4.0, 4.0]), 1.0, Tone::Snap);
                preview.strokes.extend(circle.strokes);
            }
            Some(Reference::Seg { a, b }) => {
                let l = dist(a, b);
                let (ux, uy) = ((b.x - a.x) / l, (b.y - a.y) / l);
                let far = self.reach;
                preview.strokes.push(
                    Stroke::dashed(
                        vec![
                            Vec2::new(a.x - ux * far, a.y - uy * far),
                            Vec2::new(a.x + ux * far, a.y + uy * far),
                        ],
                        false,
                        [4.0, 4.0],
                    )
                    .tone(Tone::Snap),
                );
                preview
                    .strokes
                    .push(Stroke::solid(vec![a, b], false).width(2.0).tone(Tone::Snap));
                preview.labels.push(Label {
                    at: a,
                    text: "A".into(),
                    offset: [6.0, -6.0],
                    tone: Tone::Snap,
                });
            }
            None => {}
        }
    }
}

/// The square corner at a perpendicular's foot.
fn right_angle(foot: Vec2, along: Vec2, up: Vec2) -> Marker {
    Marker {
        at: foot,
        shape: MarkerShape::RightAngle { along, up },
        tone: Tone::Accent,
    }
}

impl Tool for Perpendicular {
    fn id(&self) -> &'static str {
        if self.out { OUT_ID } else { IN_ID }
    }

    fn label(&self) -> &'static str {
        self.name()
    }

    fn prompt(&self) -> Prompt {
        let label = self.name();
        if self.reference.is_none() {
            let kinds = if self.out {
                "çizgi ya da çoklu çizgi kenarı"
            } else {
                "çizgi, çoklu çizgi kenarı, yay ya da daire"
            };
            return Prompt::new(label, format!("referans hatta tıklayın ({kinds})"));
        }
        let step = match (self.out, self.absis) {
            (false, _) => "dik inilecek noktaları gösterin, bitince sağ tıklayın",
            (true, None) => {
                "dik çıkılacak yeri hat üzerinde gösterin ya da A ucundan uzaklığı (dik ayak) yazın"
            }
            (true, Some(_)) => "dik boyu fareyle gösterin ya da yazın (sağa artı, sola eksi)",
        };
        Prompt::new(label, step).option("Başka hat", "H")
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Snaps apply once the reference is picked: the points are the user's.
    fn snaps(&self) -> bool {
        self.reference.is_some()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.reach = points::reach(cx.view, REACH);
        if self.reference.is_none() {
            edge::hover(p, cx, any);
            return;
        }
        self.hover = Some(p.world);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.reach = points::reach(cx.view, REACH);
        if self.reference.is_some() {
            return self.point(p.world, cx);
        }
        let picked = edge::pick(p, cx, any)
            .and_then(|slot| cx.doc.get(slot))
            .and_then(|e| Reference::at(e, p.raw, self.out));
        let Some(reference) = picked else {
            cx.say(
                Level::Warn,
                if self.out {
                    "Düz bir kenara tıklayın: çizgi ya da çoklu çizgi."
                } else {
                    "Bir çizgiye, çoklu çizgi kenarına, yaya ya da daireye tıklayın."
                },
            );
            return;
        };
        self.reference = Some(reference);
        cx.selection.set_hover(None);
        self.absis = None;
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        // Dik çık reads a plain number first: the foot's distance, then the length.
        if self.out
            && matches!(self.reference, Some(Reference::Seg { .. }))
            && let Some(n) = points::plain_number(text)
        {
            match self.absis {
                None => self.absis = Some(n),
                Some(_) => self.place(n, cx),
            }
            return true;
        }
        if upper_tr(js_trim(text)) == "H" {
            self.reference = None;
            return true;
        }
        if self.reference.is_none() {
            return false;
        }
        match point_from_text(text, None, self.hover, |_| None) {
            Some(p) => {
                self.point(p, cx);
                true
            }
            None => false,
        }
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Nothing of its own to take back: Ctrl+Z undoes the drawing.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut preview = Preview::default();
        self.reference_preview(&mut preview);
        let (Some(reference), Some(p)) = (self.reference, self.hover) else {
            return preview;
        };
        if !self.out {
            let Some(foot) = reference.foot(p) else {
                return preview;
            };
            preview
                .strokes
                .push(Stroke::solid(vec![p, foot], false).width(1.5));
            let mut lines = vec![format!("Dik boy {}", format.length(dist(p, foot)))];
            if let Reference::Seg { a, b } = reference
                && let Some(o) = side_offsets(a, b, p)
            {
                lines.push(format!("Dik ayak {}", format.length(o.absis)));
                preview.markers.push(right_angle(foot, b, p));
            }
            preview.tag = Some(Tag { at: p, lines });
            return preview;
        }
        let Reference::Seg { a, b } = reference else {
            return preview;
        };
        let Some(o) = side_offsets(a, b, p) else {
            return preview;
        };
        let absis = self.absis.unwrap_or(o.absis);
        let Some(foot) = side_point(a, b, absis, 0.0) else {
            return preview;
        };
        if self.absis.is_none() {
            preview
                .strokes
                .push(Stroke::solid(vec![a, foot], false).width(2.0));
            preview.tag = Some(Tag {
                at: p,
                lines: vec![format!("Dik ayak {}", format.length(absis))],
            });
            return preview;
        }
        let Some(end) = side_point(a, b, absis, o.ordinat) else {
            return preview;
        };
        preview
            .strokes
            .push(Stroke::solid(vec![foot, end], false).width(1.5));
        preview.markers.push(right_angle(foot, b, end));
        let side = if o.ordinat >= 0.0 { "sağ" } else { "sol" };
        preview.tag = Some(Tag {
            at: p,
            lines: vec![
                format!("Dik ayak {}", format.length(absis)),
                format!("Dik boy {} ({side})", format.length(o.ordinat)),
            ],
        });
        preview
    }
}
