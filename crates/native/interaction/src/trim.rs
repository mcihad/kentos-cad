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
//! A trim replaces the object by its pieces, the first in its place; an
//! extend gives it its new geometry; both through `cad.entities.edit`. What
//! is cut and grown is the shared geometry store's (`trim_preview`,
//! `extend_preview`: the boundaries near the target, then `trim_entity`,
//! `extend_entity`), as the web asks it.

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::ops::curve_cuts::{Cut, Geometry};
use kentos_geometry_core::ops::offset::through_distance;
use kentos_geometry_core::ops::path::{nearest_s, path_of};
use kentos_geometry_core::tools::point_text::point_from_text;
use kentos_native_application::geometry::shape;

use crate::edge::{self, Hover, Outline};
use crate::fence::{self, Fence};
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tone, Tool};
use crate::{Vec2, js_trim};

/// The trim tool's id: its command is `tool.trim`.
pub const TRIM_ID: &str = "trim";
/// The extend tool's id: its command is `tool.extend`.
pub const EXTEND_ID: &str = "extend";

/// How near a crossing must lie to a piece to be on it, metres.
const ON_PIECE: f64 = 1e-6;

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

    /// Çit: what the fence crosses trimmed or extended, all as one edit. What
    /// could not be done is said with the first reason; with nothing done,
    /// nothing is written.
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
        let (mut changes, mut objects, mut skipped) = (Vec::new(), 0, 0);
        let mut why: Option<String> = None;
        for (slot, crossings) in hits {
            let Some(e) = cx.doc.get(slot).cloned() else {
                continue;
            };
            let work = match self.act {
                Act::Trim => self.fence_trim(slot, &e, &crossings, cx),
                Act::Extend => self.fence_extend(slot, &e, &crossings, cx),
            };
            skipped += work.skipped;
            if why.is_none() {
                why = work.why;
            }
            if !work.changes.is_empty() {
                objects += 1;
                changes.extend(work.changes);
            }
        }
        self.fence.clear();
        if changes.is_empty() {
            let reason = why.map(|w| format!(" {w}")).unwrap_or_default();
            cx.say(
                Level::Warn,
                format!("Çit hiçbir nesneyi {}; nesne değişmedi.{reason}", verb_of(self.act)),
            );
            return;
        }
        let operation = match self.act {
            Act::Trim => EditOperation::Trim,
            Act::Extend => EditOperation::Extend,
        };
        if edge::write(operation, changes, cx).is_none() {
            return;
        }
        let doc = &*cx.doc;
        cx.selection.retain(|s| doc.get(s).is_some());
        cx.selection.set_hover(None);
        let mut text = format!("Çit: {objects} nesne {done}.");
        if skipped > 0 {
            text.push_str(&format!(" {skipped} kesişim işlenemedi"));
            match why {
                Some(w) => text.push_str(&format!(": {w}")),
                None => text.push('.'),
            }
        }
        cx.say(Level::Success, text);
    }

    /// The trims of one object crossed by the fence, one after another: each
    /// crossing cuts the piece it is on, so what an earlier one took away is not cut twice.
    fn fence_trim(&self, slot: Slot, e: &Entity, crossings: &[Vec2], cx: &Context<'_>) -> Work {
        let mut work = Work::default();
        if matches!(e, Entity::Polygon(p) if p.holes.as_ref().is_some_and(|x| !x.is_empty())) {
            work.skipped = crossings.len();
            work.why = Some(
                "Adalı alanda budama yapılamaz; önce Patlat ile halkalarına ayırın.".to_owned(),
            );
            return work;
        }
        let (chosen, view) = (self.chosen(), cx.view.visible());
        let mut pieces = vec![shape(e)];
        for &at in crossings {
            let Some(i) = piece_at(&pieces, at) else {
                continue;
            };
            let target = CoreEntity::new(pieces[i].clone());
            let cut = cx.spatial.store().trim_preview(
                &target,
                at,
                Some(f64::from(slot.0)),
                chosen.as_deref(),
                &view,
            );
            match cut {
                Cut::Pieces(now) => {
                    pieces.splice(i..=i, now.into_iter().map(|p| p.shape));
                    work.done = true;
                }
                Cut::Error(error) => work.fail(error),
            }
        }
        if work.done {
            work.changes = edge::replace_changes(cx.doc, slot, &pieces);
        }
        work
    }

    /// The extends of one object crossed by the fence: each end at most once,
    /// the one nearest the crossing.
    fn fence_extend(&self, slot: Slot, e: &Entity, crossings: &[Vec2], cx: &Context<'_>) -> Work {
        let mut work = Work::default();
        let (chosen, view) = (self.chosen(), cx.view.visible());
        let mut current = shape(e);
        let mut used = [false; 2];
        for &at in crossings {
            let Some(path) = path_of(&current).filter(|p| !p.closed) else {
                work.fail("Yalnız açık çizgiler, çoklu çizgiler ve yaylar uzatılabilir.".to_owned());
                continue;
            };
            let end = usize::from(nearest_s(&path, at) > path.length / 2.0);
            if std::mem::replace(&mut used[end], true) {
                continue;
            }
            let target = CoreEntity::new(current.clone());
            let grown = cx.spatial.store().extend_preview(
                &target,
                at,
                Some(f64::from(slot.0)),
                chosen.as_deref(),
                &view,
            );
            match grown {
                Geometry::Ok(g) => {
                    current = g.shape;
                    work.done = true;
                }
                Geometry::Error(error) => work.fail(error),
            }
        }
        if let (true, Some(geometry)) = (work.done, edge::geometry(&current)) {
            work.changes = vec![EntityEdit::Update {
                uid: edge::uid(cx.doc, slot),
                geometry,
            }];
        }
        work
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

/// What the fence did to one object: the edits, what it could not do and why.
#[derive(Default)]
struct Work {
    changes: Vec<EntityEdit>,
    done: bool,
    skipped: usize,
    why: Option<String>,
}

impl Work {
    fn fail(&mut self, error: String) {
        self.skipped += 1;
        self.why.get_or_insert(error);
    }
}

/// What the tool does to an object, for a sentence: `budayamadı`, `uzatamadı`.
fn verb_of(act: Act) -> &'static str {
    match act {
        Act::Trim => "budayamadı",
        Act::Extend => "uzatamadı",
    }
}

/// The piece of a cut object a crossing lies on, if any is left there.
fn piece_at(pieces: &[Shape], at: Vec2) -> Option<usize> {
    pieces
        .iter()
        .enumerate()
        .map(|(i, p)| (i, through_distance(p, at)))
        .filter(|(_, d)| *d < ON_PIECE)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
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
            let Some(p) = point_from_text(text, last, last, |d| cx.track_along(d)) else {
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
