//! Buda and Uzat: the web's `TrimTool` and `ExtendTool` on their shared
//! `BoundaryEdgeTool` (`apps/web/src/tools/edgeTools.ts`, docs/adr/0047),
//! with AutoCAD's TRIM and EXTEND options (docs/adr/0294):
//!
//! - the boundaries are every visible edge in view (AutoCAD's quick mode), or
//!   the objects picked after Sınır seç (S): a click turns one over in the
//!   selection, a confirm (Enter, Space, a quick right click) takes them and
//!   they stay highlighted as the selection; Tüm kenarlar (T) goes back to
//!   every edge; Esc leaves the picking and keeps what was taken before;
//! - a click on an edge trims the part between the boundaries around the
//!   click (Buda) or extends the end nearest the click to the first boundary
//!   (Uzat); with Shift held, the other one. In quick mode Buda erases an
//!   object no boundary cuts (AutoCAD's quick mode); with chosen boundaries
//!   it says so and leaves it;
//! - Uzantılar (U, AutoCAD's EDGEMODE): the boundaries bound along their
//!   extensions too (a line as its whole line, an arc as its circle, an open
//!   polyline's end segments on past its ends); kept for the session, the
//!   two tools share it;
//! - Kesişen (K), or a click where there is nothing: a crossing window from
//!   two corners; Buda takes away every piece between the boundaries that
//!   meets it, Uzat grows every end inside it;
//! - Çit (C, docs/adr/0140): a fence is drawn instead ([`crate::fence`]) and
//!   every object it crosses is trimmed where it crosses (Buda) or extended
//!   at the end nearest the crossing, once for each end (Uzat);
//! - Sil (R, Buda only; AutoCAD's eRase): clicks erase objects until R or
//!   Esc goes back;
//! - Ctrl+Z inside the tool takes the last trim back (AutoCAD's Undo), or a
//!   fence's last point while one is drawn;
//! - the preview follows the pointer: the whole object dashed in the danger
//!   colour and the pieces that stay over it (Buda), the extended object
//!   dashed (Uzat); the window shows what it would do the same way.
//!
//! What a click, a window and a fence do is the semantic commands'
//! (docs/adr/0250, 0294): `cad.entities.trim` and `cad.entities.extend` with
//! the picks, the fence or the window, the boundaries (those picked, or the
//! view's rectangle), `edgeMode` and `eraseUncut`, run with the tool's store;
//! the preview is the command's plan. Sil is `cad.entities.delete`.

use kentos_contracts::{
    CommandResult, EntitiesDelete, EntitiesTrim, EntitiesTrimPlan, EntitiesTrimmed, TrimBoundaries,
    TrimEdgeMode, TrimPick, TrimWindow,
};
use kentos_domain::Slot;
use kentos_native_application::geometry::shape;
use kentos_native_application::{
    ExecutionContext, codes, delete, entities_extend, entities_trim, trim,
};

use crate::edge::{self, Hover, Outline};
use crate::fence::Fence;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tone, Tool};
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

/// How the tool's picks reach the command.
enum Picks {
    /// Clicks: an object and where.
    At(Vec<(Slot, Vec2)>),
    /// Çit: the fence's points.
    Fence(Vec<Vec2>),
    /// Kesişen: the window's corners.
    Window(Vec2, Vec2),
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
    /// Uzantılar: the boundaries along their extensions (the session's, `Memory::trim_edge_mode`).
    edges: bool,
    /// Kesişen: waiting for the window's first corner.
    window_on: bool,
    /// Kesişen: the window's first corner once given.
    corner: Option<Vec2>,
    /// Sil: clicks erase.
    erasing: bool,
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
            edges: false,
            window_on: false,
            corner: None,
            erasing: false,
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

    /// Kesişen is being drawn: its first corner awaited or given.
    fn windowing(&self) -> bool {
        self.window_on || self.corner.is_some()
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

    /// What `cad.entities.trim` (`.extend`) is asked for the picks: quick mode erases what nothing cuts.
    fn request(&self, picks: Picks, cx: &Context<'_>) -> EntitiesTrim {
        let mut input = EntitiesTrim {
            picks: Vec::new(),
            fence: None,
            crossing: None,
            edge_mode: self.edges.then_some(TrimEdgeMode::Extend),
            erase_uncut: self.bounds.is_none().then_some(true),
            boundaries: Some(self.boundaries(cx)),
            expected_revision: None,
        };
        match picks {
            Picks::At(at) => {
                input.picks = at
                    .into_iter()
                    .map(|(slot, at)| TrimPick {
                        uid: edge::uid(cx.doc, slot),
                        at: points::wire(at),
                    })
                    .collect();
            }
            Picks::Fence(pts) => input.fence = Some(pts.into_iter().map(points::wire).collect()),
            Picks::Window(a, b) => {
                input.crossing = Some(TrimWindow {
                    min: points::wire(Vec2::new(a.x.min(b.x), a.y.min(b.y))),
                    max: points::wire(Vec2::new(a.x.max(b.x), a.y.max(b.y))),
                });
            }
        }
        input
    }

    /// The command for the picks, with the tool's store: the answer as it is.
    fn run(&self, act: Act, picks: Picks, cx: &mut Context<'_>) -> CommandResult<EntitiesTrimmed> {
        let input = self.request(picks, cx);
        let mut ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
        // Through the catalog's commands, so the record names them (docs/adr/0253).
        match act {
            Act::Trim => entities_trim::execute(&mut ecx, input),
            Act::Extend => entities_extend::execute(&mut ecx, input),
        }
    }

    /// The command's plan for the picks: what the preview shows.
    fn plan(&self, act: Act, picks: Picks, cx: &Context<'_>) -> Option<EntitiesTrimPlan> {
        let input = self.request(picks, cx);
        match trim::plan_in(cx.doc, Some(cx.spatial.store()), &input, act.core()) {
            CommandResult::Completed { output, .. } => Some(output),
            _ => None,
        }
    }

    fn trim_at(&mut self, slot: Slot, at: Vec2, cx: &mut Context<'_>) {
        let result = self.run(Act::Trim, Picks::At(vec![(slot, at)]), cx);
        if let Some(out) = points::written(result, cx) {
            let left = out.changed.len() + out.created.len();
            if let Some(n) = crate::edge::line_parts(cx.doc, slot) {
                cx.say(Level::Success, format!("Budandı: çizgi {n} parçalı."));
            } else if left == 0 {
                cx.say(
                    Level::Success,
                    "Silindi: onu kesen bir sınır yoktu (hızlı kip).",
                );
            } else {
                cx.say(Level::Success, format!("Budandı: {left} parça kaldı."));
            }
            self.forget_gone(cx);
        }
    }

    fn extend_at(&mut self, slot: Slot, at: Vec2, cx: &mut Context<'_>) {
        let result = self.run(Act::Extend, Picks::At(vec![(slot, at)]), cx);
        if points::written(result, cx).is_some() {
            cx.say(Level::Success, "Uzatıldı.");
        }
    }

    /// The selection and the hover lose what went.
    fn forget_gone(&mut self, cx: &mut Context<'_>) {
        let doc = &*cx.doc;
        cx.selection.retain(|s| doc.get(s).is_some());
        cx.selection.set_hover(None);
        self.hover = None;
    }

    /// Çit or Kesişen: what the fence crosses, or the window meets, trimmed or extended, all as one
    /// command. What could not be done is said with the first reason; with nothing done, nothing is
    /// written. Whether something was.
    fn apply_many(&mut self, picks: Picks, what: &str, cx: &mut Context<'_>) -> bool {
        let (verb, done) = match self.act {
            Act::Trim => ("budayamadı", "budandı"),
            Act::Extend => ("uzatamadı", "uzatıldı"),
        };
        let result = self.run(self.act, picks, cx);
        match result {
            CommandResult::Completed { output, warnings } => {
                self.forget_gone(cx);
                let objects = match self.act {
                    Act::Trim => output.changed.len() + output.removed.len(),
                    Act::Extend => output.changed.len(),
                };
                let mut text = format!("{what}: {objects} nesne {done}.");
                let skipped: Vec<_> = warnings
                    .iter()
                    .filter(|w| w.code == codes::TRIM_FAILED || w.code == codes::EXTEND_FAILED)
                    .collect();
                if let Some(first) = skipped.first() {
                    text.push_str(&format!(
                        " {} yer işlenemedi: {}",
                        skipped.len(),
                        first.message
                    ));
                }
                if let Some(locked) = warnings.iter().find(|w| w.code == codes::LAYER_LOCKED) {
                    text.push(' ');
                    text.push_str(&locked.message);
                }
                cx.say(Level::Success, text);
                true
            }
            CommandResult::Failed { error }
                if error.code == codes::TRIM_FAILED
                    || error.code == codes::EXTEND_FAILED
                    || error.code == codes::LAYER_LOCKED =>
            {
                cx.say(
                    Level::Warn,
                    format!(
                        "{what} hiçbir nesneyi {verb}; nesne değişmedi. {}",
                        error.message
                    ),
                );
                false
            }
            other => points::written(other, cx).is_some(),
        }
    }

    /// Çit: the fence applied, then cleared; a fence that did nothing stays, to amend.
    fn apply_fence(&mut self, cx: &mut Context<'_>) {
        let pts = self.fence.points().to_vec();
        if self.apply_many(Picks::Fence(pts), "Çit", cx) {
            self.fence.clear();
        }
    }

    /// Kesişen's second corner: the window applied; clicking goes on.
    fn apply_window(&mut self, b: Vec2, cx: &mut Context<'_>) {
        let Some(a) = self.corner.take() else { return };
        self.window_on = false;
        self.drawn = Preview::default();
        if (a.x - b.x).abs() < points::SAME || (a.y - b.y).abs() < points::SAME {
            cx.say(
                Level::Warn,
                "Pencerenin iki köşesi aynı doğru üstünde; karşı köşeyi gösterin.",
            );
            return;
        }
        let _ = self.apply_many(Picks::Window(a, b), "Kesişen", cx);
    }

    /// What a plan would do, drawn: the objects that change dashed in the danger colour with what stays
    /// over them (Buda), or what they grow into dashed (Uzat).
    fn draw_plan(&self, act: Act, plan: &EntitiesTrimPlan, cx: &Context<'_>) -> Preview {
        let mut out = Outline::default();
        let originals = plan.changed.iter().map(|e| Slot(e.base().id)).chain(
            plan.removed
                .iter()
                .filter_map(|u| cx.doc.slot_of(kentos_domain::Uuid::parse_str(u).ok()?)),
        );
        match act {
            Act::Trim => {
                for slot in originals {
                    if let Some(e) = cx.doc.get(slot) {
                        out = out.and(Outline::of(&shape(e), Some([5.0, 3.0]), 2.0, Tone::Danger));
                    }
                }
                for p in plan.changed.iter().chain(&plan.created) {
                    out = out.and(Outline::of(&shape(p), None, 1.5, Tone::Accent));
                }
            }
            Act::Extend => {
                for p in &plan.changed {
                    out = out.and(Outline::of(&shape(p), Some([4.0, 3.0]), 1.5, Tone::Accent));
                }
            }
        }
        Preview {
            strokes: out.strokes,
            marks: out.marks,
            ..Preview::default()
        }
    }

    /// The preview under the pointer, from the command's plan (the web's `preview`).
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        if self.picking || self.erasing {
            return;
        }
        let Some(h) = self.hover else { return };
        let act = self.act_for(self.shift);
        if let Some(plan) = self.plan(act, Picks::At(vec![(h.slot, h.at)]), cx) {
            self.drawn = self.draw_plan(act, &plan, cx);
        }
    }

    /// Kesişen's preview: the window dashed, and what it would do.
    fn redraw_window(&mut self, to: Vec2, cx: &Context<'_>) {
        self.drawn = Preview::default();
        let Some(a) = self.corner else { return };
        if (a.x - to.x).abs() >= points::SAME
            && (a.y - to.y).abs() >= points::SAME
            && let Some(plan) = self.plan(self.act, Picks::Window(a, to), cx)
        {
            self.drawn = self.draw_plan(self.act, &plan, cx);
        }
        self.drawn.strokes.push(Stroke {
            pts: vec![a, Vec2::new(to.x, a.y), to, Vec2::new(a.x, to.y)],
            closed: true,
            dash: Some([6.0, 4.0]),
            width: 1.0,
            tone: Tone::Snap,
        });
    }

    /// Sil: the object under the pointer erased through `cad.entities.delete`.
    fn erase_at(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick_said(p, cx, edge::unlocked) else {
            cx.say(Level::Warn, "Silinecek bir nesneye tıklayın.");
            return;
        };
        let input = EntitiesDelete {
            uids: vec![edge::uid(cx.doc, slot)],
            expected_revision: None,
        };
        let result = delete::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            cx.say(Level::Success, "1 nesne silindi.");
            self.forget_gone(cx);
        }
    }

    /// Whether nothing at all is under the pointer: a click there starts a crossing window.
    fn empty_at(p: &Pointer, cx: &Context<'_>) -> bool {
        let tol = cx.pick_tolerance();
        cx.spatial.pick_edge(p.raw, tol, |_| true).is_none()
            && cx.spatial.pick(p.raw, tol).is_none()
    }
}

impl Tool for Boundary {
    /// An object is picked (the web's `cursor = 'pick'`).
    fn cursor(&self) -> Cursor {
        if self.fence.on || self.windowing() {
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

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.edges = cx.memory.trim_edge_mode;
        Flow::Stay
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
        if self.windowing() {
            let step = if self.corner.is_none() {
                "pencerenin ilk köşesini belirtin"
            } else {
                "pencerenin karşı köşesini belirtin"
            };
            return Prompt::new(label, step).then().option("Tıklama", "K");
        }
        if self.erasing {
            return Prompt::new(label, "silinecek nesnelere tıklayın")
                .then()
                .option("Budamaya dön", "R");
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
        let p = p
            .option("Kesişen", "K")
            .toggle("Uzantılar", "U", self.edges);
        let p = match self.act {
            Act::Trim => p.option("Sil", "R"),
            Act::Extend => p,
        };
        p.then().note(format!("Shift+tık: {}", self.other_label()))
    }

    fn point_count(&self) -> usize {
        0
    }

    /// The edge is picked by its edge, so no snaps; a fence and a window are drawn with them.
    fn snaps(&self) -> bool {
        self.fence.on || self.windowing()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.fence.last().or(self.corner)
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.fence.on {
            return self.fence.moved(p, cx);
        }
        if self.windowing() {
            return self.redraw_window(p.world, cx);
        }
        self.shift = p.shift;
        if self.picking || self.erasing {
            let hit = if self.erasing {
                edge::pick(p, cx, edge::unlocked)
            } else {
                cx.spatial.pick(p.raw, cx.pick_tolerance())
            };
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
        if self.windowing() {
            match self.corner {
                None => {
                    self.window_on = false;
                    self.corner = Some(p.world);
                }
                Some(_) => self.apply_window(p.world, cx),
            }
            return;
        }
        if self.picking {
            if let Some(hit) = cx.spatial.pick(p.raw, cx.pick_tolerance()) {
                cx.selection.toggle(hit);
            }
            self.selected = cx.selection.len();
            return;
        }
        if self.erasing {
            return self.erase_at(p, cx);
        }
        // Nothing there: the click is a crossing window's first corner (AutoCAD's implicit window).
        if Self::empty_at(p, cx) {
            self.corner = Some(p.raw);
            self.hover = None;
            self.drawn = Preview::default();
            cx.selection.set_hover(None);
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

    /// S picks the boundaries, T goes back to every visible edge, K draws a crossing window, U turns the
    /// boundaries' extensions on and off, R erases (Buda), C draws a fence (K goes back to clicking);
    /// while fencing or windowing, a typed point is a point of it.
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
        if self.windowing() {
            if key == "K" {
                self.window_on = false;
                self.corner = None;
                self.drawn = Preview::default();
                return true;
            }
            let Some(p) = cx.typed_point(text, self.corner, self.corner) else {
                return false;
            };
            match self.corner {
                None => {
                    self.window_on = false;
                    self.corner = Some(p);
                }
                Some(_) => self.apply_window(p, cx),
            }
            return true;
        }
        if self.erasing {
            if key == "R" {
                self.erasing = false;
                cx.selection.set_hover(None);
                return true;
            }
            return false;
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
            "K" if !self.picking => {
                self.window_on = true;
                self.hover = None;
                cx.selection.set_hover(None);
            }
            "U" if !self.picking => {
                self.edges = !self.edges;
                cx.memory.trim_edge_mode = self.edges;
                cx.say(
                    Level::Info,
                    if self.edges {
                        "Uzantılar açık: sınırlar uçlarının ötesinde de keser."
                    } else {
                        "Uzantılar kapalı: sınırlar yalnız oldukları yerde keser."
                    },
                );
            }
            "R" if !self.picking && self.act == Act::Trim => {
                self.erasing = true;
                self.hover = None;
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
        if self.windowing() {
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

    /// Esc: a fence being drawn is dropped, then the method is left; a window
    /// is dropped; Sil goes back to trimming; picking boundaries goes back to
    /// those taken before.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.fence.on {
            if self.fence.points().is_empty() {
                self.fence.stop();
            } else {
                self.fence.clear();
            }
            return true;
        }
        if self.windowing() {
            self.window_on = false;
            self.corner = None;
            self.drawn = Preview::default();
            return true;
        }
        if self.erasing {
            self.erasing = false;
            cx.selection.set_hover(None);
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
