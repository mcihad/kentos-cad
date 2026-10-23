//! İşlemlerin Python kodu konsolda (docs/adr/0622 §7; FreeCAD's “Show script commands in Python console”): with
//! `drafting.pythonEcho` on, every product command the open drawing runs (its journal's rows, `kentos.calls/1`, as
//! Komut geçmişi reads them) and every selection made are written to the console as the code that does the same
//! (`python_echo`, `doc.select`), shown as code and run by nobody. What the console's own runs do is not written:
//! their code is there already. A drawing opened, or the setting turned on, starts from what is there; nothing done
//! before is written.

use iced::Task;
use kentos_commands::CallKind;
use kentos_native_application::python_call::{python_echo, python_literal, redacted, redacted_names};

use crate::app::{App, Message};

/// The setting that turns the echo on.
pub(crate) const SETTING: &str = "drafting.pythonEcho";

/// What the echo has passed of the open drawing.
#[derive(Debug, Default)]
pub(crate) struct Echo {
    /// The drawing and its newest journal row passed (written or not).
    rows: Option<(u64, u64)>,
    /// The drawing and the selection's version passed.
    selection: Option<(u64, u64)>,
}

impl App {
    /// History changes do not append a journal row: echo the successful operation explicitly.
    pub(crate) fn echo_history(&mut self, undo: bool) {
        if self.settings.bool(SETTING) && self.python.running.is_none() {
            self.python.repl.echo(if undo { "doc.undo()" } else { "doc.redo()" }.to_owned());
        }
    }

    /// After every message: the drawing's new commands and a selection made, as code in the console.
    pub(crate) fn follow_python_echo(&mut self) -> Task<Message> {
        let on = self.settings.effective(SETTING) == serde_json::Value::Bool(true);
        // A run's own writes and selections are its code's.
        let console = self.python.running.is_some();
        let Some(doc) = &self.document else {
            self.python.echo = Echo::default();
            return Task::none();
        };
        let session = doc.session;
        let mut said: Vec<String> = Vec::new();
        let newest = doc.model.journal().last().map_or(0, |r| r.seq);
        let from = match self.python.echo.rows {
            Some((s, seen)) if s == session => seen,
            _ => newest,
        };
        if on && !console && newest > from {
            // A form's hidden fields are written hidden (docs/adr/0443 §2).
            let names = redacted_names(&doc.model);
            for row in doc
                .model
                .journal()
                .filter(|r| r.seq > from && r.kind == CallKind::Command)
            {
                let (Some(command), Some(input)) = (row.command.as_deref(), row.input.as_ref()) else {
                    continue;
                };
                said.push(python_echo(
                    command,
                    Some(crate::history::text::title_of(command)),
                    &redacted(input, &names),
                    row.relax_constraints,
                ));
            }
        }
        self.python.echo.rows = Some((session, newest));
        let version = self.selection.version();
        let changed = matches!(self.python.echo.selection, Some((s, v)) if s == session && v != version);
        if on && !console && changed {
            let uids: Vec<String> = self
                .selection
                .ids()
                .iter()
                .filter_map(|slot| doc.model.uid(*slot))
                .map(|u| u.to_string())
                .collect();
            said.push(selection_code(&uids));
        }
        self.python.echo.selection = Some((session, version));
        if said.is_empty() {
            return Task::none();
        }
        for code in said {
            self.python.repl.echo(code);
        }
        super::view::follow()
    }
}

/// A selection as the code that makes it.
fn selection_code(uids: &[String]) -> String {
    match uids.len() {
        0 => "# Seçim kaldırıldı\ndoc.select([])".to_owned(),
        n => format!(
            "# Seçim: {n} nesne\ndoc.select({})",
            python_literal(&serde_json::json!(uids), 0)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{SETTING, selection_code};
    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;
    use kentos_domain::Slot;
    use kentos_ui::widget::python::EntryKind;

    fn echoed(app: &App) -> Vec<String> {
        app.python
            .repl
            .entries()
            .filter(|e| e.kind == EntryKind::Echo)
            .map(|e| e.text.clone())
            .collect()
    }

    #[test]
    fn with_the_setting_on_commands_and_selections_are_written_as_code() {
        let mut app = app_with_drawing();
        // Off: nothing.
        app.selection.set([Slot(4)]);
        let _ = app.update(Message::DialogClosed);
        assert!(echoed(&app).is_empty());
        let _ = app
            .settings
            .choose(&[(SETTING, serde_json::Value::Bool(true))]);
        let _ = app.update(Message::DialogClosed);
        assert!(echoed(&app).is_empty(), "nothing done before is written");
        // A selection made.
        app.selection.set([Slot(4)]);
        app.selection.add([Slot(5)]);
        let _ = app.update(Message::DialogClosed);
        let said = echoed(&app);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].starts_with("# Seçim: 2 nesne\ndoc.select(["), "{}", said[0]);
        // A command run: its call, title first.
        let uid = app
            .document
            .as_ref()
            .and_then(|d| d.model.uid(Slot(4)))
            .expect("the parcel")
            .to_string();
        let _ = app.update(Message::Run("tool.erase"));
        let said = echoed(&app);
        assert!(
            said.iter().any(|s| s.contains("cad.entities.delete.execute_input(doc,") && s.contains(&uid)),
            "{said:?}"
        );
        // Cleared.
        app.selection.clear();
        let _ = app.update(Message::DialogClosed);
        assert!(echoed(&app).last().is_some_and(|s| s.ends_with("doc.select([])")));
    }

    #[test]
    fn history_and_turning_the_echo_off_are_observed() {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[(SETTING, serde_json::Value::Bool(true))]);
        let _ = app.update(Message::DialogClosed);
        app.selection.set([Slot(4)]);
        let _ = app.update(Message::Run("tool.erase"));
        let _ = app.update(Message::Run("edit.undo"));
        assert!(echoed(&app).iter().any(|s| s == "doc.undo()"));
        let _ = app.update(Message::Run("edit.redo"));
        assert!(echoed(&app).iter().any(|s| s == "doc.redo()"));
        let _ = app.settings.choose(&[(SETTING, serde_json::Value::Bool(false))]);
        let _ = app.update(Message::DialogClosed);
        let before = echoed(&app);
        let _ = app.update(Message::Run("edit.undo"));
        app.selection.set([Slot(5)]);
        let _ = app.update(Message::DialogClosed);
        assert_eq!(echoed(&app), before);
    }

    #[test]
    fn a_large_selection_is_complete_executable_code() {
        let many: Vec<String> = (0..6000).map(|i| format!("00000000-0000-7000-8000-{i:012}")).collect();
        let code = selection_code(&many);
        let literal = code.split_once("doc.select(").expect("a Python call").1.strip_suffix(')').expect("closed call");
        let actual: Vec<String> = serde_json::from_str(&literal.replace(",\n]", "\n]")).expect("the UUID list is also JSON");
        assert_eq!(actual, many);
        assert!(code.len() > 128 * 1024);
        let mut repl = kentos_ui::widget::python::ReplState::default();
        repl.echo(&code);
        assert_eq!(repl.entries().next().expect("echoed code").text, code, "the console must not cut executable code");
        assert_eq!(selection_code(&[]), "# Seçim kaldırıldı\ndoc.select([])");
    }
}
