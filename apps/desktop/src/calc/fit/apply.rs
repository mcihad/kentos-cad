//! What Vektör oturtma does to the app (the web's `FitDialog.ts`): its
//! controls, Adla eşle said in the log, a row's pick, and Uygula through
//! `cad.entities.transform`.

use kentos_contracts::{CommandResult, EntitiesTransform};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, transform};

use super::words::kind_name;
use super::{Event, Side, TITLE, transform_of};
use crate::app::App;
use crate::calc::traverse::mm_text;

impl App {
    /// Vektör oturtma's own controls.
    pub(crate) fn fit_event(&mut self, e: Event) {
        let form = &mut self.calc.fit;
        match e {
            Event::Kind(kind) => form.kind = kind,
            Event::Source(id) => form.source = (!id.is_empty()).then_some(id),
            Event::Target(id) => form.target = (!id.is_empty()).then_some(id),
            Event::Scope(scope) => form.scope = scope,
            Event::Layer(id) => form.layer = (!id.is_empty()).then_some(id),
            Event::Copy(on) => form.copy = on,
            Event::Match => self.fit_match(),
            Event::Apply => self.fit_apply(),
            Event::Pick(row, side) => {
                let label = format!(
                    "{}. çiftin {}",
                    row + 1,
                    match side {
                        Side::Source => "kaynağı",
                        Side::Target => "hedefi",
                    }
                );
                self.calc.fit.picking = Some((row, side));
                self.calc_pick(TITLE, label);
            }
        }
    }

    /// Adla eşle, said in the log as the web says it.
    fn fit_match(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let (n, twice) = self.calc.fit.match_by_name(&doc.model);
        if n == 0 {
            let tail = if twice > 0 {
                format!("; {twice} ad bir katmanda birden çok noktada")
            } else {
                String::new()
            };
            self.warn(format!("{TITLE}: iki katmanda aynı adlı nokta yok{tail}."));
            return;
        }
        let tail = if twice > 0 {
            format!("; {twice} ad bir katmanda birden çok noktada olduğu için alınmadı")
        } else {
            String::new()
        };
        self.say(
            Level::Info,
            format!("{TITLE}: {n} çift adla eşlendi{tail}."),
        );
    }

    /// Uygula (the web's `write`): the transform through
    /// `cad.entities.transform` as one undo step (Oturt); copies are
    /// selected; the window closes. A refusal stays in the window.
    fn fit_apply(&mut self) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let form = &self.calc.fit;
        let (Some(fit), true) = (
            form.fit(),
            form.has_targets(&doc.model, self.selection.len()),
        ) else {
            return;
        };
        let Some(transform) = transform_of(fit) else {
            return;
        };
        let m0 = fit
            .m0
            .map(|m0| format!(" (m0 ±{})", mm_text(m0)))
            .unwrap_or_default();
        let copy = form.copy;
        let kind = form.kind;
        let input = EntitiesTransform {
            uids: form.targets(&doc.model, self.selection.ids()),
            transform,
            copy: copy.then_some(true),
            expected_revision: None,
        };
        let model = &mut doc.model;
        let (output, warnings) = match transform::execute(&mut ExecutionContext::new(model), input)
        {
            CommandResult::Completed { output, warnings } => (output, warnings),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.calc.fit.status = Some(error.message);
                return;
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {
                self.calc.fit.status = Some("Yazılamadı.".to_owned());
                return;
            }
        };
        let created: Vec<Slot> = output
            .created
            .iter()
            .filter_map(|uid| uid.parse().ok())
            .filter_map(|uid| model.slot_of(uid))
            .collect();
        let n = if copy {
            output.created.len()
        } else {
            output.changed.len()
        };
        let copies = if copy { "nin kopyası" } else { "" };
        self.say(
            Level::Success,
            format!(
                "{TITLE}: {n} nesne{copies} {} dönüşümle oturtuldu{m0}. Ctrl+Z geri alır.",
                kind_name(kind)
            ),
        );
        for w in warnings {
            self.warn(w.message);
        }
        if copy {
            self.selection.set(created);
        }
        self.calc.open = None;
        self.dialog = None;
    }
}
