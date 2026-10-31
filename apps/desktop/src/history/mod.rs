//! The command history's calls (docs/adr/0284, CAD-INPUT-20): in Komut
//! geçmişi, beside the log's lines, every product command the drawing ran
//! (its journal's rows, `kentos.calls/1`, docs/adr/0253 §3) and every call
//! that wrote nothing (`Document::refused_calls`): the command's title, its
//! input in words, what it wrote, its warnings and its error with their
//! codes; the calls of one undo step together. Each gives its Python
//! (`kentos_native_application::python_call`: the SDK's call with the same
//! input), shown under it or copied, and its replay input (`kentos.replay/1`
//! with that row, replayed by `Document.replay`, MCP and the headless host),
//! copied or saved.
//!
//! The calls are read from the drawing's journal after every message, not
//! made again from what the tools did; a row's state (undone, done again) is
//! read when it is drawn.

pub(crate) mod text;
mod view;

use std::path::PathBuf;

use iced::Task;
use kentos_commands::{
    CallKind, CallRecord, CallStatus, REPLAY_FORMAT, REPLAY_VERSION, ReplayJournal,
};
use kentos_domain::RefusedCall;
use kentos_native_application::python_call::python_call_in;

use crate::app::{App, Message, Picker};

/// The history keeps this many calls, as the log its lines.
const LIMIT: usize = crate::log_plan::LIMIT;

/// What a call of the history is.
#[derive(Debug, Clone)]
pub(crate) enum Call {
    /// A journal row: a command's write, or an edit made without one.
    Row(Box<CallRecord>),
    /// A command's call that wrote nothing.
    Refused(Box<RefusedCall>),
}

/// A call in Komut geçmişi.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub id: u64,
    /// The log's newest line when it was seen: it is listed after that line.
    pub after: u64,
    /// When it was seen (milliseconds since 1970).
    pub at: i64,
    /// The drawing it was run in (`Document::session`).
    pub doc: u64,
    pub call: Call,
}

/// The calls listed and what was seen of the open drawing's journal.
#[derive(Debug, Default)]
pub(crate) struct History {
    pub(crate) entries: Vec<Entry>,
    given: u64,
    /// The drawing, its newest row and its newest refused call listed.
    seen: Option<(u64, u64, u64)>,
    /// The call whose Python is shown under it.
    pub(crate) open: Option<u64>,
}

impl History {
    /// Geçmişi temizle: the calls go with the lines.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.open = None;
    }

    fn push(&mut self, after: u64, at: i64, doc: u64, call: Call) {
        self.given += 1;
        self.entries.push(Entry {
            id: self.given,
            after,
            at,
            doc,
            call,
        });
        if self.entries.len() > LIMIT {
            self.entries.remove(0);
        }
    }

    /// How many calls were ever listed: it grows with each.
    pub(crate) fn given(&self) -> u64 {
        self.given
    }

    fn get(&self, id: u64) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

/// What a call's buttons ask.
#[derive(Debug, Clone)]
pub enum Event {
    /// Its Python shown under it, or hidden.
    Toggle(u64),
    CopyPython(u64),
    CopyReplay(u64),
    SaveReplay(u64),
    /// Where its replay input goes (none: not chosen).
    SaveTo(u64, Option<PathBuf>),
    /// Every call as one Python script, to the clipboard (docs/adr/0443 §2).
    CopyScript,
}

impl App {
    /// After every message: the open drawing's new journal rows and refused
    /// calls join the history, after the log's newest line.
    pub(crate) fn follow_history(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let session = doc.session;
        let (mut row_seen, mut refused_seen) = match self.history.seen {
            Some((s, rows, refused)) if s == session => (rows, refused),
            _ => (0, 0),
        };
        let after = self.log.last_id();
        let at = crate::cloud::now_ms();
        let rows: Vec<CallRecord> = doc
            .model
            .journal()
            .filter(|r| r.seq > row_seen)
            .cloned()
            .collect();
        let refused: Vec<RefusedCall> = doc
            .model
            .refused_calls()
            .filter(|r| r.seq > refused_seen)
            .cloned()
            .collect();
        for row in rows {
            row_seen = row.seq;
            self.history
                .push(after, at, session, Call::Row(Box::new(row)));
        }
        for r in refused {
            refused_seen = r.seq;
            self.history.push(after, at, session, Call::Refused(Box::new(r)));
        }
        self.history.seen = Some((session, row_seen, refused_seen));
    }

    /// A row as the open drawing has it now (undone, done again), or as it was seen.
    pub(crate) fn row_now<'a>(&'a self, entry: &'a Entry) -> Option<&'a CallRecord> {
        let Call::Row(row) = &entry.call else {
            return None;
        };
        Some(
            self.document
                .as_ref()
                .filter(|d| d.session == entry.doc)
                .and_then(|d| d.model.journal_row(row.seq))
                .unwrap_or(row.as_ref()),
        )
    }

    /// The Python that runs the call again; none for an edit made without a command.
    pub(crate) fn python_of(&self, entry: &Entry) -> Option<String> {
        let (command, version, input, relaxed) = match &entry.call {
            Call::Row(row) if row.kind == CallKind::Command => (
                row.command.as_deref()?,
                row.version.unwrap_or(1),
                row.input.as_ref()?,
                row.relax_constraints,
            ),
            Call::Row(_) => return None,
            Call::Refused(r) => (r.command.as_str(), r.version, &r.input, false),
        };
        // A form's hidden fields are written hidden (docs/adr/0443 §2).
        let names = self
            .document
            .as_ref()
            .map(|d| kentos_native_application::python_call::redacted_names(&d.model))
            .unwrap_or_default();
        Some(python_call_in(
            command,
            version,
            Some(text::title_of(command)),
            &kentos_native_application::python_call::redacted(input, &names),
            relaxed,
        ))
    }

    /// Every command call of the open drawing's history as one Python script, in their order; the
    /// form's hidden fields written hidden (docs/adr/0443 §2). None without a call.
    pub(crate) fn python_script_of_history(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        let calls: Vec<(String, u32, Option<String>, serde_json::Value, bool)> = self
            .history
            .entries
            .iter()
            .filter(|e| e.doc == doc.session)
            .filter_map(|e| match &e.call {
                Call::Row(row) if row.kind == CallKind::Command => Some((
                    row.command.clone()?,
                    row.version.unwrap_or(1),
                    row.command.as_deref().map(|c| text::title_of(c).to_owned()),
                    row.input.clone()?,
                    row.relax_constraints,
                )),
                _ => None,
            })
            .collect();
        if calls.is_empty() {
            return None;
        }
        let names = kentos_native_application::python_call::redacted_names(&doc.model);
        Some(kentos_native_application::python_call::python_script(
            &calls, &names,
        ))
    }

    /// The call as a replay input (`kentos.replay/1` with its one row);
    /// none for an edit made without a command or a row kept without its input.
    pub(crate) fn replay_of(&self, entry: &Entry) -> Option<String> {
        let doc = self
            .document
            .as_ref()
            .filter(|d| d.session == entry.doc)
            .map(|d| &d.model);
        let row = match &entry.call {
            Call::Row(row) if row.kind == CallKind::Command && row.input.is_some() => self
                .row_now(entry)
                .cloned()
                .unwrap_or_else(|| (**row).clone()),
            Call::Row(_) => return None,
            Call::Refused(r) => {
                let revision = doc.map_or_else(|| "0".to_owned(), |d| d.revision().to_string());
                CallRecord {
                    relax_constraints: false,
                    seq: r.seq,
                    kind: CallKind::Command,
                    label: None,
                    command: Some(r.command.clone()),
                    version: Some(r.version),
                    input: Some(r.input.clone()),
                    input_digest: None,
                    request_id: None,
                    fingerprint: None,
                    base: revision.clone(),
                    revision,
                    context: None,
                    algorithm: None,
                    seed: None,
                    created: Vec::new(),
                    changed: Vec::new(),
                    removed: Vec::new(),
                    dimension_sets: Default::default(),
                    truncated: false,
                    warnings: Vec::new(),
                    parts: Vec::new(),
                    effects: Vec::new(),
                    step: None,
                    status: CallStatus::Done,
                }
            }
        };
        let context = row
            .context
            .clone()
            .or_else(|| doc.map(kentos_native_application::call::context_of))?;
        let journal = ReplayJournal {
            format: REPLAY_FORMAT.into(),
            version: REPLAY_VERSION,
            context,
            calls: vec![row],
        };
        // A form's hidden fields are written hidden here too: replaying asks for their values.
        let names = doc
            .map(kentos_native_application::python_call::redacted_names)
            .unwrap_or_default();
        let value = serde_json::to_value(&journal).ok()?;
        serde_json::to_string_pretty(&kentos_native_application::python_call::redacted(
            &value, &names,
        ))
        .ok()
    }

    pub(crate) fn calls_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Toggle(id) => {
                self.history.open = (self.history.open != Some(id)).then_some(id);
            }
            Event::CopyPython(id) => {
                if let Some(code) = self.history.get(id).and_then(|e| self.python_of(e)) {
                    self.output(
                        "Python kodu panoya kopyalandı; Python konsolunda doc açık çizimdir.",
                    );
                    return iced::clipboard::write(code);
                }
            }
            Event::CopyReplay(id) => {
                if let Some(json) = self.history.get(id).and_then(|e| self.replay_of(e)) {
                    self.output(
                        "Tekrar oynatma girdisi (kentos.replay/1) panoya kopyalandı; Python'da doc.replay(...) ile oynatılır.",
                    );
                    return iced::clipboard::write(json);
                }
            }
            Event::SaveReplay(id) => {
                let Some(entry) = self.history.get(id) else {
                    return Task::none();
                };
                if let Picker::File(path) = &self.picker {
                    let path = path.clone();
                    return self.calls_event(Event::SaveTo(id, Some(path)));
                }
                let name = match &entry.call {
                    Call::Row(row) => format!("komut-{}.json", row.seq),
                    Call::Refused(r) => format!("reddedilen-{}.json", r.seq),
                };
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Tekrar oynatma girdisini kaydet")
                            .add_filter("KentOS tekrar oynatma (.json)", &["json"])
                            .set_file_name(name)
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    move |path| Message::History(Event::SaveTo(id, path)),
                );
            }
            Event::CopyScript => {
                if let Some(script) = self.python_script_of_history() {
                    self.output(
                        "Komut geçmişi Python betiği olarak panoya kopyalandı; formda gizlenen alanların değerleri ‹gizli› yazıldı.",
                    );
                    return iced::clipboard::write(script);
                }
            }
            Event::SaveTo(_, None) => {}
            Event::SaveTo(id, Some(path)) => {
                let Some(json) = self.history.get(id).and_then(|e| self.replay_of(e)) else {
                    return Task::none();
                };
                match std::fs::write(&path, json) {
                    Ok(()) => self.output(format!(
                        "Tekrar oynatma girdisi yazıldı: {}.",
                        path.display()
                    )),
                    Err(e) => self.error(format!(
                        "Tekrar oynatma girdisi yazılamadı ({}): {e}. Başka bir klasör seçin.",
                        path.display()
                    )),
                }
            }
        }
        Task::none()
    }
}

#[cfg(test)]
pub(crate) mod tests;
