//! An extension's dialog (plan/eklenti.md ADIM 22's "Python'dan iletişim kutusu", docs/adr/8057 §29): its running code
//! asks the user (`ctx.dialog`, `ctx.confirm`), a KentOS window over the drawing with a panel's fields; the run waits
//! for the answer.
//!
//! - **A panel's fields, drawn by the same code** (panel_view.rs's `panel_form`): text, number, date, time, moment,
//!   choice, multiple choice, check, layer and field pickers, and words. No buttons, rows or commands of its own: the
//!   dialog answers the code that asked. Tamam gives the values by id, checked as a panel's (a wrong number is said by
//!   its label and the window stays); İptal, Esc and the window's ✕ give none.
//! - **Checked when asked:** its fields go through Kapı 1's panel check; a wrong one is the call's error, nothing shown
//!   (the extension's mistake, said to its code and its log, not put to the user).
//! - **One at a time, the run's own:** the run's Durdur and its end close it; the code is answered none and hears
//!   Durdur after. A run without a window to ask in (no drawing, another dialog open) is answered at once.

use iced::Element;
use iced::widget::column;
use kentos_extensions::ui_panel;
use kentos_headless::HeadlessError;
use kentos_ui::label;
use kentos_ui::widget::Dialog as Frame;
use serde_json::{Value, json};

use super::panels::{OpenPanel, PanelEvent};
use super::{Event, msg};
use crate::app::{App, Message};
use crate::exchange::words;

/// What a dialog may hold: a panel's fields and words.
const FIELDS: [&str; 12] = [
    "text",
    "section",
    "textField",
    "numberField",
    "dateField",
    "timeField",
    "dateTimeField",
    "select",
    "multiSelect",
    "checkbox",
    "layerPicker",
    "fieldPicker",
];

/// An extension's dialog open over the window.
pub(crate) struct ExtDialog {
    pub(crate) ext: String,
    /// The call it answers (its id in the extension's process).
    pub(crate) call: Value,
    pub(crate) ok: String,
    pub(crate) cancel: String,
    /// Its fields and what the user wrote in them, as a panel's.
    pub(crate) panel: OpenPanel,
}

fn refused(code: &'static str, message: impl Into<String>) -> HeadlessError {
    HeadlessError::new(code, message)
}

/// A button's words from the call, at most 40 characters; `default` when none is given.
fn words_of(params: &Value, key: &str, default: &str) -> String {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map_or_else(|| default.to_owned(), |w| w.chars().take(40).collect())
}

impl App {
    /// `dialog` asked by `ext`'s running code (`params`: `title`, `body`, `ok`, `cancel`): opened, answered later; or
    /// why not now, the call's error.
    pub(crate) fn ext_dialog_open(
        &mut self,
        ext: &str,
        call: Value,
        params: &Value,
    ) -> Result<(), HeadlessError> {
        if self
            .extensions
            .runners
            .get(ext)
            .and_then(|r| r.run.as_ref())
            .is_none()
        {
            return Err(refused("busy", "Eklentinin çalışan bir işlevi yok."));
        }
        if self.extensions.dialog.is_some() {
            return Err(refused(
                "busy",
                "Bir eklenti penceresi zaten açık: kullanıcı onu yanıtlayınca sorun.",
            ));
        }
        let title = params
            .get("title")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| refused("invalid_input", "Pencerenin başlığı (title) gerekli."))?;
        let body = params
            .get("body")
            .and_then(Value::as_array)
            .ok_or_else(|| refused("invalid_input", "Pencerenin alanları (body) bir liste olmalı."))?;
        for (i, c) in body.iter().enumerate() {
            let kind = c.get("type").and_then(Value::as_str).unwrap_or_default();
            if !FIELDS.contains(&kind) {
                return Err(refused(
                    "invalid_input",
                    format!(
                        "body[{i}]: “{kind}” pencerede olamaz; olanlar: {}.",
                        FIELDS.join(", ")
                    ),
                ));
            }
            if c.get("onChange").is_some() {
                return Err(refused(
                    "invalid_input",
                    format!("body[{i}]: pencerenin alanı komut çalıştırmaz (onChange yok); değerler Tamam'la döner."),
                ));
            }
        }
        // The fields as a panel's, through Kapı 1's check (its words, its places).
        let panel = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": title,
                            "body": body });
        if let Some(problem) = ui_panel::check("dialog", &panel, &[]).into_iter().next() {
            let at = problem.at.as_deref().unwrap_or("");
            return Err(refused("invalid_input", format!("{at}: {}", problem.message)));
        }
        let definition = ui_panel::read(&panel)
            .ok_or_else(|| refused("invalid_input", "Pencerenin alanları okunamadı."))?;
        self.extensions.dialog = Some(ExtDialog {
            ext: ext.to_owned(),
            call,
            ok: words_of(params, "ok", "Tamam"),
            cancel: words_of(params, "cancel", "İptal"),
            panel: OpenPanel::new(ext, "dialog", definition),
        });
        Ok(())
    }

    /// What the user does in the dialog's fields, as in a panel's (no command runs).
    pub(crate) fn ext_dialog_event(&mut self, event: PanelEvent) {
        let Some(panel) = self.extensions.dialog.as_mut().map(|d| &mut d.panel) else {
            return;
        };
        panel.forget_invalid(&event);
        match event {
            PanelEvent::Text(id, text) | PanelEvent::Picked(id, text) => {
                panel.texts.insert(id, text);
            }
            PanelEvent::Check(id, on) => {
                panel.checks.insert(id, on);
            }
            PanelEvent::Choose(id, value) => panel.choose(id, value),
            PanelEvent::Toggle(id, value) => {
                let chosen = panel.multi.entry(id).or_default();
                match chosen.iter().position(|v| *v == value) {
                    Some(i) => {
                        chosen.remove(i);
                    }
                    None => chosen.push(value),
                }
            }
            PanelEvent::Section(at) => panel.toggle_section(at),
            _ => {}
        }
        // Its calculations follow the values, as a panel's (panel_values.rs).
        panel.recalculate();
    }

    /// Tamam (`ok`): the values by id when they are right, else why under the fields, the window open; İptal, Esc, ✕:
    /// none. Either way the code that asked goes on.
    pub(crate) fn ext_dialog_answer(&mut self, ok: bool) {
        let Some(dialog) = self.extensions.dialog.as_mut() else {
            return;
        };
        let answer = if ok {
            match dialog.panel.check_params() {
                Ok(values) => values,
                Err((input, why)) => {
                    dialog.panel.said = Some((words::Kind::Error, why.clone()));
                    dialog.panel.reveal(&input);
                    dialog.panel.invalid = Some((input, why));
                    return;
                }
            }
        } else {
            Value::Null
        };
        if let Some(dialog) = self.extensions.dialog.take() {
            self.ext_dialog_reply(&dialog, answer);
        }
    }

    /// The run of `ext` ended or was stopped: its dialog closed, its code answered none (it hears Durdur after).
    pub(crate) fn ext_dialog_close_for(&mut self, ext: &str) {
        if self.extensions.dialog.as_ref().is_some_and(|d| d.ext == ext)
            && let Some(dialog) = self.extensions.dialog.take()
        {
            self.ext_dialog_reply(&dialog, Value::Null);
        }
    }

    fn ext_dialog_reply(&self, dialog: &ExtDialog, answer: Value) {
        if let Some(runner) = self.extensions.runners.get(&dialog.ext) {
            runner.send(&json!({ "type": "reply", "id": dialog.call, "ok": true, "result": answer }));
        }
    }

    /// The dialog over the window: whose it is, its fields as a panel's, what was wrong with Tamam, its two buttons.
    pub(crate) fn ext_dialog_view(&self) -> Option<Element<'_, Message>> {
        let dialog = self.extensions.dialog.as_ref()?;
        let p = &dialog.panel;
        let (form, _) = self.panel_form(
            p,
            |e| msg(Event::Dialog(e)),
            Some(msg(Event::DialogAnswer(true))),
            None,
        );
        let mut body = column![
            label::caption(format!("{} soruyor", self.extension_name(&dialog.ext))),
            form
        ]
        .spacing(12);
        // A wrong field's words are under it (the window is short); others under the fields.
        if let Some((kind, said)) = p.said.as_ref().filter(|_| p.invalid.is_none()) {
            body = body.push(words::line(*kind, label::caption(said.clone())));
        }
        Some(
            Frame::new(p.definition.title.clone())
                .key("eklenti-penceresi")
                .scroll(body)
                .action(words::secondary(
                    dialog.cancel.clone(),
                    Some(msg(Event::DialogAnswer(false))),
                ))
                .action(words::primary(
                    dialog.ok.clone(),
                    Some(msg(Event::DialogAnswer(true))),
                ))
                .width(480.0)
                .max_height(600.0)
                .modal(msg(Event::DialogAnswer(false))),
        )
    }
}
