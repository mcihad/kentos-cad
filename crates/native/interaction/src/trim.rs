//! Buda and Uzat: the web's `TrimTool` and `ExtendTool` on their shared
//! `BoundaryEdgeTool` (`apps/web/src/tools/edgeTools.ts`), step for step
//! (docs/adr/0047):
//!
//! - the boundaries are every visible edge in view (AutoCAD's quick mode), or
//!   the objects picked after Sınır seç (S): a click turns one over in the
//!   selection, a confirm (Enter, Space, a quick right click) takes them and
//!   they stay highlighted as the selection; Tüm kenarlar (T) goes back to
//!   every edge; Esc leaves the picking and keeps what was taken before;
//! - a click on an edge trims the part between the boundaries around the
//!   click (Buda) or extends the end nearest the click to the first boundary
//!   (Uzat); with Shift held, the other one;
//! - the preview follows the pointer: the whole object dashed in the danger
//!   colour and the pieces that stay over it (Buda), the extended object
//!   dashed (Uzat).
//!
//! A trim replaces the object by its pieces, the first in its place; an
//! extend gives it its new geometry; both through `cad.entities.edit`. What
//! is cut and grown is the shared geometry store's (`trim_preview`,
//! `extend_preview`: the boundaries near the target, then `trim_entity`,
//! `extend_entity`), as the web asks it.

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::ops::curve_cuts::{Cut, Geometry};
use kentos_native_application::geometry::shape;

use crate::edge::{self, Hover, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Tone, Tool};
use crate::{Vec2, js_trim};

/// The trim tool's id: its command is `tool.trim`.
pub const TRIM_ID: &str = "trim";
/// The extend tool's id: its command is `tool.extend`.
pub const EXTEND_ID: &str = "extend";

/// What a click does without Shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    Trim,
    Extend,
}

/// The trim or the extend tool.
#[derive(Clone, Debug)]
pub struct Boundary {
    act: Act,
    /// The boundaries picked with Sınır seç; none: every visible edge.
    bounds: Option<Vec<Slot>>,
    /// Sınır seç: clicks pick boundaries until a confirm.
    picking: bool,
    /// Shift was held at the last pointer move: the preview shows the other act.
    shift: bool,
    hover: Option<Hover>,
    /// The selection's size while picking, for the prompt.
    selected: usize,
    drawn: Preview,
}

impl Boundary {
    fn with(act: Act) -> Self {
        Self {
            act,
            bounds: None,
            picking: false,
            shift: false,
            hover: None,
            selected: 0,
            drawn: Preview::default(),
        }
    }

    pub fn trim() -> Self {
        Self::with(Act::Trim)
    }

    pub fn extend() -> Self {
        Self::with(Act::Extend)
    }

    fn label_of(act: Act) -> &'static str {
        match act {
            Act::Trim => "Buda",
            Act::Extend => "Uzat",
        }
    }

    /// The other act's name in the prompt and warnings: `uzat`, `buda`.
    fn other_label(&self) -> &'static str {
        match self.act {
            Act::Trim => "uzat",
            Act::Extend => "buda",
        }
    }

    fn action_prompt(&self) -> &'static str {
        match self.act {
            Act::Trim => "silinecek parçaya tıklayın",
            Act::Extend => "uzatılacak ucun yakınına tıklayın",
        }
    }

    /// What a click does: the tool's act, the other with Shift.
    fn act_for(&self, shift: bool) -> Act {
        match (self.act, shift) {
            (Act::Trim, false) | (Act::Extend, true) => Act::Trim,
            _ => Act::Extend,
        }
    }

    fn chosen(&self) -> Option<Vec<f64>> {
        self.bounds
            .as_ref()
            .map(|ids| ids.iter().map(|s| f64::from(s.0)).collect())
    }

    /// The object at `slot` trimmed at `at`: the pieces that stay, from the store.
    fn cut(&self, slot: Slot, e: &Entity, at: Vec2, cx: &Context<'_>) -> Cut {
        let chosen = self.chosen();
        cx.spatial.store().trim_preview(
            &edge::core(e),
            at,
            Some(f64::from(slot.0)),
            chosen.as_deref(),
            &cx.view.visible(),
        )
    }

    /// The object at `slot` extended at the end nearest `at`, from the store.
    fn grow(&self, slot: Slot, e: &Entity, at: Vec2, cx: &Context<'_>) -> Geometry {
        let chosen = self.chosen();
        cx.spatial.store().extend_preview(
            &edge::core(e),
            at,
            Some(f64::from(slot.0)),
            chosen.as_deref(),
            &cx.view.visible(),
        )
    }

    fn trim_at(&mut self, slot: Slot, at: Vec2, cx: &mut Context<'_>) {
        let Some(e) = cx.doc.get(slot).cloned() else {
            return;
        };
        if edge::refuse_holed(&e, "budama", cx) {
            return;
        }
        match self.cut(slot, &e, at, cx) {
            Cut::Error(error) => cx.say(Level::Warn, error),
            Cut::Pieces(pieces) => {
                let shapes: Vec<_> = pieces.into_iter().map(|p| p.shape).collect();
                if edge::replace(EditOperation::Trim, slot, &shapes, cx) {
                    cx.say(
                        Level::Success,
                        format!("Budandı: {} parça kaldı.", shapes.len()),
                    );
                }
                self.hover = None;
            }
        }
    }

    fn extend_at(&mut self, slot: Slot, at: Vec2, cx: &mut Context<'_>) {
        let Some(e) = cx.doc.get(slot).cloned() else {
            return;
        };
        match self.grow(slot, &e, at, cx) {
            Geometry::Error(error) => cx.say(Level::Warn, error),
            Geometry::Ok(grown) => {
                let uid = edge::uid(cx.doc, slot);
                let written = edge::geometry(&grown.shape).and_then(|geometry| {
                    edge::write(
                        EditOperation::Extend,
                        vec![EntityEdit::Update { uid, geometry }],
                        cx,
                    )
                });
                if written.is_some() {
                    cx.say(Level::Success, "Uzatıldı.");
                }
            }
        }
    }

    /// The preview under the pointer: the trim's pieces over the whole object,
    /// or the extended object (the web's `preview`).
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        if self.picking {
            return;
        }
        let Some(h) = self.hover else { return };
        let Some(e) = cx.doc.get(h.slot) else { return };
        let out = match self.act_for(self.shift) {
            Act::Trim => {
                if matches!(e, Entity::Polygon(p) if p.holes.as_ref().is_some_and(|x| !x.is_empty()))
                {
                    return;
                }
                let Cut::Pieces(pieces) = self.cut(h.slot, e, h.at, cx) else {
                    return;
                };
                // The whole object dashed red, the pieces that stay solid on top: what stays red goes.
                pieces.iter().fold(
                    Outline::of(&shape(e), Some([5.0, 3.0]), 2.0, Tone::Danger),
                    |out, p| out.and(Outline::of(&p.shape, None, 1.5, Tone::Accent)),
                )
            }
            Act::Extend => match self.grow(h.slot, e, h.at, cx) {
                Geometry::Ok(g) => Outline::of(&g.shape, Some([4.0, 3.0]), 1.5, Tone::Accent),
                Geometry::Error(_) => return,
            },
        };
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            ..Preview::default()
        };
    }
}

impl Tool for Boundary {
    fn id(&self) -> &'static str {
        match self.act {
            Act::Trim => TRIM_ID,
            Act::Extend => EXTEND_ID,
        }
    }

    fn label(&self) -> &'static str {
        Self::label_of(self.act)
    }

    fn prompt(&self) -> Prompt {
        let label = self.label();
        if self.picking {
            return Prompt::new(
                label,
                format!(
                    "sınır olacak nesnelere tıklayın, bitince sağ tıklayın ({} seçili)",
                    self.selected
                ),
            )
            .option("Tüm kenarlar", "T");
        }
        let p = Prompt::new(label, self.action_prompt());
        let p = match &self.bounds {
            Some(ids) => p
                .note(format!("sınır: seçilen {} nesne", ids.len()))
                .then()
                .option("Tüm kenarlar", "T")
                .option("Sınır seç", "S"),
            None => p
                .note("sınır: görünen tüm kenarlar")
                .then()
                .option("Sınır seç", "S"),
        };
        p.then().note(format!("Shift+tık: {}", self.other_label()))
    }

    fn point_count(&self) -> usize {
        0
    }

    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.shift = p.shift;
        if self.picking {
            let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
            cx.selection.set_hover(hit);
            return;
        }
        self.hover = edge::hover(p, cx, edge::unlocked);
        self.redraw(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.picking {
            if let Some(hit) = cx.spatial.pick(p.raw, cx.pick_tolerance()) {
                cx.selection.toggle(hit);
            }
            self.selected = cx.selection.len();
            return;
        }
        let Some(slot) = edge::pick(p, cx, edge::unlocked) else {
            let label = if p.shift {
                self.other_label()
            } else {
                self.label()
            };
            cx.say(
                Level::Warn,
                format!("{label} için düzenlenebilir bir kenara tıklayın."),
            );
            return;
        };
        match self.act_for(p.shift) {
            Act::Trim => self.trim_at(slot, p.raw, cx),
            Act::Extend => self.extend_at(slot, p.raw, cx),
        }
        self.drawn = Preview::default();
    }

    /// S picks the boundaries, T goes back to every visible edge.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match upper_tr(js_trim(text)).as_str() {
            "S" => {
                self.picking = true;
                cx.selection.set(self.bounds.clone().unwrap_or_default());
            }
            "T" => {
                self.bounds = None;
                self.picking = false;
                cx.selection.clear();
            }
            _ => return false,
        }
        self.selected = cx.selection.len();
        self.drawn = Preview::default();
        true
    }

    /// Picking boundaries: the selection becomes them (none: every edge
    /// again). Otherwise the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if !self.picking {
            return Flow::Exit;
        }
        let ids = cx.selection.ids().to_vec();
        self.bounds = (!ids.is_empty()).then_some(ids);
        self.picking = false;
        Flow::Stay
    }

    /// Esc while picking: back to the boundaries taken before.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.picking {
            return false;
        }
        self.picking = false;
        cx.selection.set(self.bounds.clone().unwrap_or_default());
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
