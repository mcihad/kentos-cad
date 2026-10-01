//! Editing and REPL state. No interpreter, IO or timers run in this module.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use iced::widget::text_editor::{self, Action, Cursor, Edit, Motion, Position};

use super::syntax;
use super::{CompletionEvent, CompletionRequest, CompletionState};

const UNDO_LIMIT: usize = 128;
const HISTORY_LIMIT: usize = 200;
const TRANSCRIPT_LIMIT: usize = 400;
const OUTPUT_LIMIT: usize = 128 * 1024;

struct Revision {
    text: String,
    cursor: Cursor,
}

/// Optional editor controller: smart newline, four space indentation,
/// grouped typing undo and redo. An application can also keep using its
/// own controller with `PythonEditor::new(&content, on_action)`.
pub struct EditorState {
    pub content: text_editor::Content,
    pub completion: CompletionState,
    saved: String,
    undo: VecDeque<Revision>,
    redo: Vec<Revision>,
    typing: bool,
    revision: u64,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::with_text("")
    }
}

impl EditorState {
    pub fn with_text(source: &str) -> Self {
        Self {
            content: text_editor::Content::with_text(source),
            completion: CompletionState::default(),
            saved: source.to_owned(),
            undo: VecDeque::new(),
            redo: Vec::new(),
            typing: false,
            revision: 0,
        }
    }

    pub fn modified(&self) -> bool {
        self.content.text() != self.saved
    }
    pub fn mark_saved(&mut self) {
        self.saved = self.content.text();
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn snapshot(&self) -> Revision {
        Revision {
            text: self.content.text(),
            cursor: self.content.cursor(),
        }
    }

    fn restore(&mut self, revision: Revision) {
        self.content = text_editor::Content::with_text(&revision.text);
        self.content.move_to(revision.cursor);
        self.typing = false;
        self.completion.dismiss();
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop_back() {
            self.redo.push(self.snapshot());
            self.restore(previous);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push_back(self.snapshot());
            self.restore(next);
        }
    }

    pub fn perform(&mut self, action: Action) {
        let suggest = matches!(&action, Action::Edit(Edit::Insert(ch)) if ch.is_alphanumeric() || matches!(ch, '_' | '.' | '('))
            || matches!(&action, Action::Edit(Edit::Backspace));
        let editing = action.is_edit();
        let typing = matches!(action, Action::Edit(Edit::Insert(_)))
            && self.content.cursor().selection.is_none();
        if editing {
            if !typing || !self.typing {
                self.undo.push_back(self.snapshot());
                while self.undo.len() > UNDO_LIMIT {
                    self.undo.pop_front();
                }
            }
            self.redo.clear();
            self.revision = self.revision.wrapping_add(1);
        }
        self.typing = typing;
        match action {
            Action::Edit(Edit::Enter) => smart_newline(&mut self.content),
            Action::Edit(Edit::Indent) => indent(&mut self.content, false),
            Action::Edit(Edit::Unindent) => indent(&mut self.content, true),
            other => self.content.perform(other),
        }
        if suggest {
            self.completion.refresh(&self.content, false);
        } else {
            self.completion.dismiss();
        }
    }

    pub fn complete(&mut self, event: CompletionEvent) -> Option<CompletionRequest> {
        match event {
            CompletionEvent::Request => {
                self.completion.refresh(&self.content, true);
                return self.completion.request.clone();
            }
            CompletionEvent::Move(direction) => self.completion.navigate(direction),
            CompletionEvent::Select(index) => self.completion.select(index),
            CompletionEvent::Dismiss => self.completion.dismiss(),
            CompletionEvent::Accept | CompletionEvent::Pick(_) => {
                if let CompletionEvent::Pick(index) = event {
                    self.completion.selected = index;
                }
                let item = self.completion.selected()?.clone();
                let (start, end) = self.completion.range(&self.content)?;
                self.undo.push_back(self.snapshot());
                while self.undo.len() > UNDO_LIMIT {
                    self.undo.pop_front();
                }
                self.redo.clear();
                self.content.move_to(Cursor {
                    position: start,
                    selection: Some(end),
                });
                self.content
                    .perform(Action::Edit(Edit::Paste(Arc::new(item.insert))));
                self.typing = false;
                self.revision = self.revision.wrapping_add(1);
                self.completion.dismiss();
            }
        }
        None
    }
}

pub(crate) fn smart_newline(content: &mut text_editor::Content) {
    let cursor = content.cursor();
    let line = content
        .line(cursor.position.line)
        .map(|l| l.text.into_owned())
        .unwrap_or_default();
    let mut column = cursor.position.column.min(line.len());
    while !line.is_char_boundary(column) {
        column -= 1;
    }
    let before = line[..column].to_owned();
    let prefix: String = before
        .chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .collect();
    let mut padding = prefix.replace('\t', "    ");
    // A blank indented line closes an interactive suite. A colon inside a
    // comment or string is excluded by the shared scanner.
    if before.trim().is_empty() {
        for _ in 0..before.chars().count() {
            content.perform(Action::Edit(Edit::Backspace));
        }
        padding.clear();
    } else {
        let spans = syntax::lines(&before).remove(0);
        let colon = spans.iter().any(|(r, kind)| {
            *kind == syntax::Kind::Punctuation
                && &before[r.clone()] == ":"
                && before[r.end..].trim().is_empty()
        });
        if colon {
            padding.push_str("    ");
        }
    }
    content.perform(Action::Edit(Edit::Enter));
    if !padding.is_empty() {
        content.perform(Action::Edit(Edit::Paste(Arc::new(padding))));
    }
}

pub(crate) fn indent(content: &mut text_editor::Content, remove: bool) {
    let cursor = content.cursor();
    if cursor.selection.is_none() && !remove {
        let spaces = 4 - cursor.position.column % 4;
        content.perform(Action::Edit(Edit::Paste(Arc::new(" ".repeat(spaces)))));
        return;
    }
    let source = content.text();
    let mut lines: Vec<String> = source.split('\n').map(str::to_owned).collect();
    let anchor = cursor.selection.unwrap_or(cursor.position);
    let first = cursor.position.line.min(anchor.line);
    let mut last = cursor.position.line.max(anchor.line);
    // A selection ending at column zero doesn't include that next line.
    let end = if cursor.position.line > anchor.line {
        cursor.position
    } else {
        anchor
    };
    if last > first && end.column == 0 {
        last -= 1;
    }
    let mut changes = Vec::new();
    for line in lines.iter_mut().take(last + 1).skip(first) {
        let delta = if remove {
            let n = if line.starts_with('\t') {
                1
            } else {
                line.chars().take_while(|c| *c == ' ').count().min(4)
            };
            line.drain(..n);
            -(n as isize)
        } else {
            line.insert_str(0, "    ");
            4
        };
        changes.push(delta);
    }
    let adjust = |p: Position| Position {
        column: if (first..=last).contains(&p.line) {
            p.column
                .saturating_add_signed(changes.get(p.line - first).copied().unwrap_or_default())
        } else {
            p.column
        },
        ..p
    };
    *content = text_editor::Content::with_text(&lines.join("\n"));
    content.move_to(Cursor {
        position: adjust(cursor.position),
        selection: cursor.selection.map(adjust),
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunStatus {
    #[default]
    Ready,
    Running,
    Success,
    Error,
}

impl RunStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Hazır",
            Self::Running => "Çalışıyor",
            Self::Success => "Tamamlandı",
            Self::Error => "Hata",
        }
    }
}

impl From<RunStatus> for crate::widget::animated_surface::Activity {
    fn from(status: RunStatus) -> Self {
        match status {
            RunStatus::Ready => Self::Ready,
            RunStatus::Running => Self::Running,
            RunStatus::Success => Self::Success,
            RunStatus::Error => Self::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Input,
    Output,
    Error,
    Note,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub kind: EntryKind,
    pub text: String,
    pub execution: u64,
}

/// A submitted source and its unique execution id. Return this id with all
/// output: results from an interrupted execution will then be ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: u64,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct RunResult {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    /// Set this when the interpreter needs another line (`compile_command`
    /// returned None); the submitted draft is restored for continuation.
    pub incomplete: bool,
    pub elapsed: Duration,
}

#[derive(Debug, Clone)]
pub enum ReplEvent {
    Edit(Action),
    Complete(CompletionEvent),
    Submit,
    History(i8),
    Undo,
    Redo,
    Clear,
    Interrupt,
}

/// Bounded transcript and command history; the interpreter belongs to the host.
pub struct ReplState {
    pub input: EditorState,
    entries: VecDeque<Entry>,
    history: VecDeque<String>,
    browsing: Option<usize>,
    draft: String,
    serial: u64,
    pending: Option<Request>,
    status: RunStatus,
    elapsed: Option<Duration>,
    revision: u64,
}

impl Default for ReplState {
    fn default() -> Self {
        Self {
            input: EditorState::default(),
            entries: VecDeque::new(),
            history: VecDeque::new(),
            browsing: None,
            draft: String::new(),
            serial: 0,
            pending: None,
            status: RunStatus::Ready,
            elapsed: None,
            revision: 0,
        }
    }
}

impl ReplState {
    pub fn entries(&self) -> impl ExactSizeIterator<Item = &Entry> {
        self.entries.iter()
    }
    pub fn status(&self) -> RunStatus {
        self.status
    }
    pub fn elapsed(&self) -> Option<Duration> {
        self.elapsed
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn pending(&self) -> Option<&Request> {
        self.pending.as_ref()
    }

    pub fn note(&mut self, text: impl Into<String>) {
        self.push(EntryKind::Note, text.into(), 0);
    }

    fn push(&mut self, kind: EntryKind, mut text: String, execution: u64) {
        if text.len() > OUTPUT_LIMIT {
            let mut end = OUTPUT_LIMIT;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
            text.push_str("\n… çıktı sınırına ulaşıldı");
        }
        self.entries.push_back(Entry {
            kind,
            text,
            execution,
        });
        while self.entries.len() > TRANSCRIPT_LIMIT {
            self.entries.pop_front();
        }
        self.revision = self.revision.wrapping_add(1);
    }

    /// Append streaming output only if the request is still current.
    pub fn append(&mut self, id: u64, kind: EntryKind, text: impl Into<String>) {
        if self.pending.as_ref().is_some_and(|r| r.id == id) {
            self.push(kind, text.into(), id);
        }
    }

    pub fn update(&mut self, event: ReplEvent) -> Option<Request> {
        match event {
            ReplEvent::Submit => {
                let source = self.input.content.text();
                if self.pending.is_some() || source.trim().is_empty() {
                    return None;
                }
                let request = self.submit(source)?;
                self.input = EditorState::default();
                return Some(request);
            }
            ReplEvent::Edit(action) => {
                let edited = action.is_edit();
                self.input.perform(action);
                if edited {
                    self.browsing = None;
                }
            }
            ReplEvent::Complete(event) => {
                self.input.complete(event);
            }
            ReplEvent::History(direction) => self.browse(direction),
            ReplEvent::Undo => self.input.undo(),
            ReplEvent::Redo => self.input.redo(),
            ReplEvent::Clear => {
                self.entries.clear();
                self.revision = self.revision.wrapping_add(1);
            }
            ReplEvent::Interrupt => {
                if let Some(request) = self.pending.take() {
                    self.push(
                        EntryKind::Error,
                        "Çalıştırma durduruldu.".to_owned(),
                        request.id,
                    );
                    self.status = RunStatus::Error;
                    self.elapsed = None;
                }
            }
        }
        None
    }

    /// Also used by a script editor sharing the REPL's interpreter.
    pub fn submit(&mut self, source: String) -> Option<Request> {
        if self.pending.is_some() || source.trim().is_empty() {
            return None;
        }
        self.serial = self.serial.checked_add(1)?;
        let request = Request {
            id: self.serial,
            source,
        };
        self.push(EntryKind::Input, request.source.clone(), request.id);
        if self.history.back() != Some(&request.source) {
            self.history.push_back(request.source.clone());
            while self.history.len() > HISTORY_LIMIT {
                self.history.pop_front();
            }
        }
        self.browsing = None;
        self.draft.clear();
        self.elapsed = None;
        self.status = RunStatus::Running;
        self.pending = Some(request.clone());
        Some(request)
    }

    pub fn finish(&mut self, id: u64, result: RunResult) -> bool {
        if self.pending.as_ref().is_none_or(|r| r.id != id) {
            return false;
        }
        let Some(request) = self.pending.take() else {
            return false;
        };
        if result.incomplete {
            self.entries.retain(|e| e.execution != id);
            self.input = EditorState::with_text(&request.source);
            self.input
                .content
                .perform(Action::Move(Motion::DocumentEnd));
            smart_newline(&mut self.input.content);
            self.status = RunStatus::Ready;
            self.revision = self.revision.wrapping_add(1);
            return true;
        }
        if !result.stdout.is_empty() {
            self.push(EntryKind::Output, result.stdout, id);
        }
        if !result.stderr.is_empty() {
            self.push(EntryKind::Error, result.stderr, id);
        }
        self.elapsed = Some(result.elapsed);
        self.status = if result.success {
            RunStatus::Success
        } else {
            RunStatus::Error
        };
        self.revision = self.revision.wrapping_add(1);
        true
    }

    fn browse(&mut self, direction: i8) {
        if self.history.is_empty() || direction == 0 {
            return;
        }
        if self.browsing.is_none() {
            if direction > 0 {
                return;
            }
            self.draft = self.input.content.text();
        }
        let current = self.browsing.unwrap_or(self.history.len());
        let next = if direction < 0 {
            current.saturating_sub(1)
        } else {
            (current + 1).min(self.history.len())
        };
        let source = if next == self.history.len() {
            self.browsing = None;
            self.draft.clone()
        } else {
            self.browsing = Some(next);
            self.history[next].clone()
        };
        self.input = EditorState::with_text(&source);
        self.input
            .content
            .perform(Action::Move(Motion::DocumentEnd));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result() -> RunResult {
        RunResult {
            stdout: "42\n".into(),
            stderr: String::new(),
            success: true,
            incomplete: false,
            elapsed: Duration::from_millis(12),
        }
    }

    #[test]
    fn smart_indent_undo_and_redo_preserve_unicode_and_selection() {
        let mut state = EditorState::with_text("def ölç():");
        state.content.perform(Action::Move(Motion::DocumentEnd));
        state.perform(Action::Edit(Edit::Enter));
        assert_eq!(state.content.text(), "def ölç():\n    ");
        state.perform(Action::Edit(Edit::Paste(Arc::new("return 42".into()))));
        state.undo();
        assert_eq!(state.content.text(), "def ölç():\n    ");
        state.redo();
        assert_eq!(state.content.text(), "def ölç():\n    return 42");
        assert!(state.modified());
        state.mark_saved();
        assert!(!state.modified());
        state.content.perform(Action::SelectAll);
        state.perform(Action::Edit(Edit::Indent));
        assert_eq!(state.content.text(), "    def ölç():\n        return 42");
        state.perform(Action::Edit(Edit::Unindent));
        assert_eq!(state.content.text(), "def ölç():\n    return 42");
    }

    #[test]
    fn stale_completion_and_streaming_output_cannot_finish_a_new_run() {
        let mut repl = ReplState::default();
        let first = repl.submit("while True: pass".into()).unwrap();
        assert!(repl.submit("42".into()).is_none());
        repl.update(ReplEvent::Interrupt);
        let second = repl.submit("42".into()).unwrap();
        repl.append(first.id, EntryKind::Output, "old output");
        assert!(!repl.finish(first.id, result()));
        assert_eq!(repl.status(), RunStatus::Running);
        assert!(repl.finish(second.id, result()));
        assert_eq!(repl.status(), RunStatus::Success);
        assert!(!repl.entries().any(|e| e.text == "old output"));
    }

    #[test]
    fn running_a_script_keeps_the_repl_draft_but_submitting_it_clears_it() {
        let mut repl = ReplState {
            input: EditorState::with_text("unsent = 10"),
            ..ReplState::default()
        };
        let request = repl.submit("print(42)".into()).unwrap();
        assert_eq!(repl.input.content.text(), "unsent = 10");
        repl.finish(request.id, result());
        let request = repl.update(ReplEvent::Submit).unwrap();
        assert_eq!(request.source, "unsent = 10");
        assert_eq!(repl.input.content.text(), "");
    }

    #[test]
    fn history_restores_unsent_draft_and_incomplete_input_continues() {
        let mut repl = ReplState::default();
        let request = repl.submit("x = 42".into()).unwrap();
        repl.finish(request.id, result());
        repl.input = EditorState::with_text("draft");
        repl.update(ReplEvent::History(-1));
        assert_eq!(repl.input.content.text(), "x = 42");
        repl.update(ReplEvent::History(1));
        assert_eq!(repl.input.content.text(), "draft");
        let request = repl.submit("for x in range(3):".into()).unwrap();
        repl.finish(
            request.id,
            RunResult {
                incomplete: true,
                ..result()
            },
        );
        assert_eq!(repl.input.content.text(), "for x in range(3):\n    ");
        assert!(!repl.entries().any(|e| e.execution == request.id));
    }

    #[test]
    fn transcript_limits_cut_utf8_at_character_boundaries() {
        let mut repl = ReplState::default();
        repl.note("🦀".repeat(OUTPUT_LIMIT));
        assert!(repl.entries().next().unwrap().text.len() < OUTPUT_LIMIT + 100);
        for i in 0..TRANSCRIPT_LIMIT + 8 {
            repl.note(i.to_string());
        }
        assert_eq!(repl.entries().len(), TRANSCRIPT_LIMIT);
    }
}
