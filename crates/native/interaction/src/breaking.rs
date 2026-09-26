//! Kır: the web's `BreakTool` (`apps/web/src/tools/pathEditTools.ts`) on its
//! edge-picking base, step for step (docs/adr/0047):
//!
//! - click the object at the first break point (object snaps apply: an
//!   intersection or an end snapped picks the object it belongs to);
//! - then the second break point: the part between the two goes; Aynı
//!   noktadan böl (Enter) splits the object at the first point instead;
//! - Esc drops the picked object; with none, the tool leaves.
//!
//! The object is replaced by its pieces, the first in its place, through
//! `cad.entities.edit`: “Aradaki kısım silindi.”, “Nesne bölündü: 2 parça.”
//! The pieces are the shared core's (`break_entity`).

use kentos_contracts::{EditOperation, Entity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::breaking::break_entity;
use kentos_geometry_core::ops::curve_cuts::Cut;
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Tone, Tool};

/// The break tool's id: its command is `tool.break`.
pub const ID: &str = "break";
pub const LABEL: &str = "Kır";

/// The kinds the break tool takes, off locked layers.
fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(
        e,
        Entity::Line(_)
            | Entity::Polyline(_)
            | Entity::Polygon(_)
            | Entity::Arc(_)
            | Entity::Circle(_)
            | Entity::Ellipse(_)
            | Entity::Xline(_)
            | Entity::Ray(_)
    ) && edge::unlocked(e, doc)
}

/// The break tool.
#[derive(Clone, Debug, Default)]
pub struct Break {
    /// The object picked and the first break point.
    target: Option<(Slot, Vec2)>,
    drawn: Preview,
}

impl Break {
    pub fn new() -> Self {
        Self::default()
    }

    /// The pieces between `p1` and `p2`, from the core.
    fn pieces(slot: Slot, p1: Vec2, p2: Vec2, cx: &Context<'_>) -> Option<Cut> {
        cx.doc
            .get(slot)
            .map(|e| break_entity(&edge::core(e), p1, p2))
    }

    fn commit(&mut self, p2: Vec2, cx: &mut Context<'_>) {
        let Some((slot, p1)) = self.target.take() else {
            return;
        };
        self.drawn = Preview::default();
        match Self::pieces(slot, p1, p2, cx) {
            None => {}
            Some(Cut::Error(error)) => cx.say(Level::Warn, error),
            Some(Cut::Pieces(pieces)) => {
                let shapes: Vec<_> = pieces.into_iter().map(|p| p.shape).collect();
                if edge::replace(EditOperation::Break, slot, &shapes, cx) {
                    let text = if dist(p1, p2) < 1e-9 {
                        format!("Nesne bölündü: {} parça.", shapes.len())
                    } else {
                        "Aradaki kısım silindi.".to_owned()
                    };
                    cx.say(Level::Success, text);
                }
            }
        }
    }
}

impl Tool for Break {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        match self.target {
            Some(_) => Prompt::new(LABEL, "ikinci kırılma noktasını belirtin")
                .option("Aynı noktadan böl", "Enter"),
            None => Prompt::new(
                LABEL,
                "nesneyi ilk kırılma noktasından seçin (kesişim ve uç kenetleri çalışır)",
            ),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Both break points snap: an intersection, an end, a point on the object.
    fn snaps(&self) -> bool {
        true
    }

    /// Perpendicular and tangent snaps are taken from the first point.
    fn snap_from(&self) -> Option<Vec2> {
        self.target.map(|(_, p1)| p1)
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some((slot, p1)) = self.target else {
            edge::hover(p, cx, editable);
            return;
        };
        // The whole object dashed red, the pieces that stay solid on top.
        self.drawn = Preview::default();
        let Some(Cut::Pieces(pieces)) = Self::pieces(slot, p1, p.world, cx) else {
            return;
        };
        let Some(e) = cx.doc.get(slot) else { return };
        let out = pieces.iter().fold(
            Outline::of(&shape(e), Some([5.0, 3.0]), 2.0, Tone::Danger),
            |out, piece| out.and(Outline::of(&piece.shape, None, 1.5, Tone::Accent)),
        );
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            ..Preview::default()
        };
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.target.is_some() {
            return self.commit(p.world, cx);
        }
        // The object under the cursor, or the one a snapped point belongs to.
        let snapped = p
            .snap
            .and_then(|s| {
                let slot = Slot(s.id as u32);
                (s.id >= 0.0 && s.id.fract() == 0.0).then_some(slot)
            })
            .filter(|slot| cx.doc.get(*slot).is_some_and(|e| editable(e, cx.doc)));
        let Some(slot) = edge::pick(p, cx, editable).or(snapped) else {
            cx.say(
                Level::Warn,
                "Kırılacak düzenlenebilir bir çizgi, çoklu çizgi, yay ya da daireye tıklayın.",
            );
            return;
        };
        if let Some(e) = cx.doc.get(slot).cloned()
            && edge::refuse_holed(&e, "kırma", cx)
        {
            return;
        }
        self.target = Some((slot, p.world));
        cx.selection.set_hover(None);
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Aynı noktadan böl: the object splits at the first point. With none picked, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        match self.target {
            Some((_, p1)) => {
                self.commit(p1, cx);
                Flow::Stay
            }
            None => Flow::Exit,
        }
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.target.is_none() {
            return false;
        }
        self.target = None;
        self.drawn = Preview::default();
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
