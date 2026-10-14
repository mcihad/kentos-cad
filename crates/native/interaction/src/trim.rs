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
//!   dashed (Uzat);
//! - Çit (C, docs/adr/0140): a fence is drawn instead ([`crate::fence`]) and
//!   every object it crosses is trimmed where it crosses (Buda) or extended
//!   at the end nearest the crossing, once for each end (Uzat), all in one
//!   undo step. An object cut by an earlier crossing is cut again where it
//!   now is; the boundaries are the same as a click's.
//!
//! What a click and a fence do is the semantic commands' (docs/adr/0250):
//! `cad.entities.trim` and `cad.entities.extend` with the picks and the
//! boundaries (those picked, or the view's rectangle), run with the tool's
//! store; the preview is the command's plan. A trim replaces the object by
//! its pieces, the first in its place; an extend gives it its new geometry.

use kentos_contracts::{CommandResult, EntitiesTrim, EntitiesTrimmed, TrimBoundaries, TrimPick};
use kentos_domain::Slot;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, codes, trim};

use crate::edge::{self, Hover, Outline};
use crate::fence::{self, Fence};
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tone, Tool};
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

impl Act {
    fn core(self) -> trim::Act {
        match self {
            Act::Trim => trim::Act::Trim,
            Act::Extend => trim::Act::Extend,
        }
    }
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
    /// Çit: the fence being drawn, when the method is on.
    fence: Fence,
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
            fence: Fence::default(),
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

    /// The command's boundaries: those picked with Sınır seç, else every shown edge in view.
    fn boundaries(&self, cx: &Context<'_>) -> TrimBoundaries {
        match &self.bounds {
            Some(slots) => TrimBoundaries::Objects {
                uids: slots.iter().map(|s| edge::uid(cx.doc, *s)).collect(),
            },
            None => {
                let v = cx.view.visible();
                TrimBoundaries::Area {
                    min: points::wire(Vec2::new(v.min_x, v.min_y)),
                    max: points::wire(Vec2::new(v.max_x, v.max_y)),
                }
            }
        }
    }

    /// What `cad.entities.trim` (`.extend`) is asked for the picks.
    fn request(&self, picks: Vec<(Slot, Vec2)>, cx: &Context<'_>) -> EntitiesTrim {
        EntitiesTrim {
            picks: picks
                .into_iter()
                .map(|(slot, at)| TrimPick {
                    uid: edge::uid(cx.doc, slot),
                    at: points::wire(at),
                })
                .collect(),
            boundaries: Some(self.boundaries(cx)),
            expected_revision: None,
        }
    }

    /// The command for the picks, with the tool's store: the answer as it is.
    fn run(
        &self,
        act: Act,
        picks: Vec<(Slot, Vec2)>,
        cx: &mut Context<'_>,
    ) -> CommandResult<EntitiesTrimmed> {
        let input = self.request(picks, cx);
        let mut ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
        trim::execute(&mut ecx, input, act.core())
    }

    fn trim_at(&mut self, slot: Slot, at: Vec2, cx: &mut Context<'_>) {
        let result = self.run(Act::Trim, vec![(slot, at)], cx);
        if let Some(out) = points::written(result, cx) {
            let left = out.changed.len() + out.created.len();
            cx.say(Level::Success, format!("Budandı: {left} parça kaldı."));
            let doc = &*cx.doc;
            cx.selection.retain(|s| doc.get(s).is_some());
            cx.selection.set_hover(None);
            self.hover = None;
        }
    }

    fn extend_at(&mut self, slot: Slot, at: Vec2, cx: &mut Context<'_>) {
        let result = self.run(Act::Extend, vec![(slot, at)], cx);
        if points::written(result, cx).is_some() {
            cx.say(Level::Success, "Uzatıldı.");
        }
    }

    /// Çit: what the fence crosses trimmed or extended, all as one command (each crossing a pick,
    /// an object's in their order). What could not be done is said with the first reason; with
    /// nothing done, nothing is written.
    fn apply_fence(&mut self, cx: &mut Context<'_>) {
        let hits = fence::crossed(self.fence.points(), cx);
        let (verb, done) = match self.act {
            Act::Trim => ("budanacak", "budandı"),
            Act::Extend => ("uzatılacak", "uzatıldı"),
        };
        if hits.is_empty() {
            cx.say(
                Level::Warn,
                format!("Çit hiçbir nesneyi kesmiyor; çiti {verb} nesnelerin üstünden geçirin."),
            );
            return;
        }
        let picks: Vec<(Slot, Vec2)> = hits
            .into_iter()
            .flat_map(|(slot, crossings)| crossings.into_iter().map(move |at| (slot, at)))
            .collect();
        let result = self.run(self.act, picks, cx);
        self.fence.clear();
        match result {
            CommandResult::Completed { output, warnings } => {
                let doc = &*cx.doc;
                cx.selection.retain(|s| doc.get(s).is_some());
                cx.selection.set_hover(None);
                let objects = match self.act {
                    Act::Trim => output.changed.len() + output.removed.len(),
                    Act::Extend => output.changed.len(),
                };
                let mut text = format!("Çit: {objects} nesne {done}.");
                let skipped: Vec<_> = warnings
                    .iter()
                    .filter(|w| w.code == codes::TRIM_FAILED || w.code == codes::EXTEND_FAILED)
                    .collect();
                if let Some(first) = skipped.first() {
                    text.push_str(&format!(
                        " {} kesişim işlenemedi: {}",
                        skipped.len(),
                        first.message
                    ));
                }
                cx.say(Level::Success, text);
            }
            CommandResult::Failed { error }
                if error.code == codes::TRIM_FAILED || error.code == codes::EXTEND_FAILED =>
            {
                cx.say(
                    Level::Warn,
                    format!(
                        "Çit hiçbir nesneyi {}; nesne değişmedi. {}",
                        verb_of(self.act),
                        error.message
                    ),
                );
            }
            other => {
                points::written(other, cx);
            }
        }
    }

    /// The preview under the pointer, from the command's plan: the whole object dashed in the
    /// danger colour and the pieces that stay over it, or the extended object (the web's `preview`).
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        if self.picking {
            return;
        }
        let Some(h) = self.hover else { return };
        let Some(e) = cx.doc.get(h.slot) else { return };
        let act = self.act_for(self.shift);
        let input = self.request(vec![(h.slot, h.at)], cx);
        let CommandResult::Completed { output, .. } =
            trim::plan_in(cx.doc, Some(cx.spatial.store()), &input, act.core())
        else {
            return;
        };
        let shapes = output.changed.iter().chain(&output.created).map(shape);
        let out = match act {
            // What stays red goes.
            Act::Trim => shapes.fold(
                Outline::of(&shape(e), Some([5.0, 3.0]), 2.0, Tone::Danger),
                |out, p| out.and(Outline::of(&p, None, 1.5, Tone::Accent)),
            ),
            Act::Extend => match shapes.into_iter().next() {
                Some(g) => Outline::of(&g, Some([4.0, 3.0]), 1.5, Tone::Accent),
                None => return,
            },
        };
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            ..Preview::default()
        };
    }
}

/// What the tool does to an object, for a sentence: `budayamadı`, `uzatamadı`.
fn verb_of(act: Act) -> &'static str {
    match act {
        Act::Trim => "budayamadı",
        Act::Extend => "uzatamadı",
    }
}

impl Tool for Boundary {
    /// An object is picked (the web's `cursor = 'pick'`).
    fn cursor(&self) -> Cursor {
        if self.fence.on {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }

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
        if self.fence.on {
            let n = self.fence.points().len();
            let step = if n == 0 {
                "çitin ilk noktasını belirtin"
            } else {
                "çitin sonraki noktasını belirtin"
            };
            let prompt = Prompt::new(label, step).then().option("Tıklama", "K");
            return if n >= 2 {
                prompt.option("Uygula", "Enter")
            } else {
                prompt
            };
        }
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

    /// The edge is picked by its edge, so no snaps; a fence is drawn with them.
    fn snaps(&self) -> bool {
        self.fence.on
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.fence.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.fence.on {
            return self.fence.moved(p, cx);
        }
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
        if self.fence.on {
            let point = self.fence.constrain(p, cx);
            return self.fence.add(point, cx);
        }
        if self.picking {
            if let Some(hit) = cx.spatial.pick(p.raw, cx.pick_tolerance()) {
                cx.selection.toggle(hit);
            }
            self.selected = cx.selection.len();
            return;
        }
        let Some(slot) = edge::pick_said(p, cx, edge::unlocked) else {
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

    /// S picks the boundaries, T goes back to every visible edge, C draws a
    /// fence (K goes back to clicking); while fencing, a point is a fence point.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if self.fence.on {
            if key == "K" {
                self.fence.stop();
                return true;
            }
            let last = self.fence.last();
            let Some(p) = cx.typed_point(text, last, last) else {
                return false;
            };
            self.fence.add(p, cx);
            return true;
        }
        if (key == "C" || key == "Ç") && !self.picking {
            self.fence.start();
            self.drawn = Preview::default();
            self.hover = None;
            cx.selection.set_hover(None);
            return true;
        }
        match key.as_str() {
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
        if self.fence.on {
            match self.fence.points().len() {
                0 => return Flow::Exit,
                1 => cx.say(
                    Level::Warn,
                    "Çit için en az iki nokta gerekir; ikinci noktayı da belirtin.",
                ),
                _ => self.apply_fence(cx),
            }
            return Flow::Stay;
        }
        if !self.picking {
            return Flow::Exit;
        }
        let ids = cx.selection.ids().to_vec();
        self.bounds = (!ids.is_empty()).then_some(ids);
        self.picking = false;
        Flow::Stay
    }

    /// Esc while picking: back to the boundaries taken before.
    /// Esc: a fence being drawn is dropped, then the method is left; picking
    /// boundaries goes back to those taken before.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.fence.on {
            if self.fence.points().is_empty() {
                self.fence.stop();
            } else {
                self.fence.clear();
            }
            return true;
        }
        if !self.picking {
            return false;
        }
        self.picking = false;
        cx.selection.set(self.bounds.clone().unwrap_or_default());
        true
    }

    /// Ctrl+Z takes a fence point back; otherwise it undoes the drawing.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.fence.on && self.fence.pop(cx)
    }

    fn preview(&self, _format: &Format) -> Preview {
        if self.fence.on {
            return self.fence.preview(self.act == Act::Trim);
        }
        self.drawn.clone()
    }
}
