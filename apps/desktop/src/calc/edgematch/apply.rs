//! What Kenar eşleme does to the app (the web's `EdgematchDialog.ts`): its
//! controls, Sınır shown on the drawing, a link looked at (Göster), and
//! Uygula through `cad.entities.edit`.

use kentos_contracts::{CommandResult, EditOperation, EntitiesEdit};
use kentos_domain::Slot;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::edgematch::Meet;
use kentos_interaction::{Level, ViewChange, fixed};
use kentos_native_application::{ExecutionContext, edit};

use super::{Aside, Event, TITLE};
use crate::app::App;
use crate::calc::Window;

impl App {
    /// Kenar eşleme's own controls.
    pub(crate) fn edgematch_event(&mut self, e: Event) {
        let form = &mut self.calc.edgematch;
        match e {
            Event::Scope(scope) => form.scope = scope,
            Event::Source(id) => form.source = (!id.is_empty()).then_some(id),
            Event::Adjacent(id) => form.adjacent = (!id.is_empty()).then_some(id),
            Event::Distance(t) => form.distance = t,
            Event::Angle(t) => form.angle = t,
            Event::Key(k) => form.key = k,
            // Sınırda needs a border; its segment is off without one.
            Event::Meet(m) => {
                if m != Meet::Border || form.border.is_some() {
                    form.meet = m;
                }
            }
            Event::Method(m) => form.method = m,
            Event::ClearBorder => {
                form.border = None;
                if form.meet == Meet::Border {
                    form.meet = Meet::Adjacent;
                }
            }
            Event::PickBorder => self.edgematch_pick_border(),
            Event::Show(row) => self.edgematch_show(row),
            Event::Apply => self.edgematch_apply(),
        }
    }

    /// Sahneden seç for Sınır: the window steps aside, one line, polyline or
    /// area is picked on the drawing, the window comes back (the web's `pickBorder`).
    fn edgematch_pick_border(&mut self) {
        if self.document.is_none() {
            return;
        }
        self.calc.edgematch.aside = Some(Aside::Border(self.selection.ids().to_vec()));
        self.selection.clear();
        self.dialog = None;
        self.field = None;
        self.snap = None;
        self.say(Level::Command, format!("{TITLE}: sınır"));
        self.session.run(Box::new(
            kentos_interaction::pick_objects::PickObjects::one(
                "Sınır",
                Some(vec!["line".into(), "polyline".into(), "polygon".into()]),
            ),
        ));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// Sınır's pick over: the object kept (Enter) becomes the border; the
    /// selection comes back and so does the window. False when Kenar
    /// eşleme was not picking.
    pub(crate) fn edgematch_picked_objects(&mut self, keep: bool) -> bool {
        let Some(Aside::Border(before)) = self.calc.edgematch.aside.take() else {
            return false;
        };
        if keep
            && let (Some(&slot), Some(doc)) = (self.selection.ids().first(), &self.document)
            && let Some(uid) = doc.model.uid(slot)
        {
            self.calc.edgematch.border = Some(uid.to_string());
        }
        self.selection.set(before);
        self.calc_show(Window::Edgematch);
        true
    }

    /// Göster: the window steps aside, the link's two lines are selected and
    /// the view goes to it; a click, Enter or Esc brings the window back (the web's `show`).
    fn edgematch_show(&mut self, row: usize) {
        let form = &self.calc.edgematch;
        let Some(l) = form.found.as_ref().and_then(|f| f.links.get(row)) else {
            return;
        };
        let (Some(&source), Some(&adjacent)) = (
            form.source_slots.get(l.source),
            form.adjacent_slots.get(l.adjacent),
        ) else {
            return;
        };
        let half = (l.gap * 60.0).max(3.0);
        let (mx, my) = ((l.from.x + l.to.x) / 2.0, (l.from.y + l.to.y) / 2.0);
        let what = format!("{}. bağ, aralık {} mm", row + 1, fixed(l.gap * 1000.0, 1));
        self.calc.edgematch.aside = Some(Aside::Look(self.selection.ids().to_vec()));
        self.selection.set(vec![source, adjacent]);
        self.dialog = None;
        self.field = None;
        self.snap = None;
        self.navigating(|app| {
            app.viewport.change(ViewChange::Fit {
                bounds: Bounds {
                    min_x: mx - half,
                    min_y: my - half,
                    max_x: mx + half,
                    max_y: my + half,
                },
                padding: 48.0,
            })
        });
        self.say(Level::Command, format!("{TITLE}: {what}"));
        self.session
            .run(Box::new(kentos_interaction::look::Look::new(TITLE, what)));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// Göster's look over: the selection and the window come back. False
    /// when Kenar eşleme was not looking.
    pub(crate) fn edgematch_looked(&mut self) -> bool {
        match self.calc.edgematch.aside.take() {
            Some(Aside::Look(before)) => {
                self.selection.set(before);
                self.calc_show(Window::Edgematch);
                true
            }
            other => {
                self.calc.edgematch.aside = other;
                false
            }
        }
    }

    /// Uygula (the web's `write`): the used links through
    /// `cad.entities.edit` as one undo step (Kenar eşle); the lines put
    /// right are selected; the window closes. A refusal stays in the window.
    fn edgematch_apply(&mut self) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let form = &self.calc.edgematch;
        if form.changes.is_empty() {
            return;
        }
        let written = form.written;
        let input = EntitiesEdit {
            operation: EditOperation::Edgematch,
            changes: form.changes.clone(),
            expected_revision: None,
        };
        let model = &mut doc.model;
        let (output, warnings) = match edit::execute(&mut ExecutionContext::new(model), input) {
            CommandResult::Completed { output, warnings } => (output, warnings),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.calc.edgematch.status = Some(error.message);
                return;
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {
                self.calc.edgematch.status = Some("Yazılamadı.".to_owned());
                return;
            }
        };
        let changed: Vec<Slot> = output
            .changed
            .iter()
            .filter_map(|uid| uid.parse().ok())
            .filter_map(|uid| model.slot_of(uid))
            .collect();
        self.say(
            Level::Success,
            format!(
                "{TITLE}: {written} bağ yazıldı, {} çizgi düzeltildi. Ctrl+Z geri alır.",
                changed.len()
            ),
        );
        for w in warnings {
            self.warn(w.message);
        }
        self.selection.set(changed);
        self.calc.open = None;
        self.dialog = None;
    }
}
