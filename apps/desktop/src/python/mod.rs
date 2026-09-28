//! The Python console (docs/adr/0132): the bottom panel's Python tab.
//!
//! Code runs in a Python of its own (`python -m kentos.host`, the `kentos`
//! package of docs/adr/0131), started when code first runs and stopped with
//! Durdur; the desktop never links a Python. The script's `kentos.cad`
//! requests come here and are answered on the open drawing by the headless
//! host's `rpc`, with the desktop's own command handlers, as its tools
//! write:
//!
//! - a run is one undo step (“Python”, or the name the script's first
//!   `doc.group` gives), opened at its first write;
//! - a run that raises, is stopped, or whose Python ends takes back what it
//!   wrote; so does one in which a `doc.group` failed, even when the script
//!   went on;
//! - while code runs the drawing takes no other edit (as while a drawing
//!   opens, opening.rs): commands, clicks, the panels' edits wait for it.

mod assist;
mod code;
mod host;
pub(crate) mod script;
mod view;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use iced::Task;
use iced::widget::text_editor;
use kentos_headless::HeadlessError;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::viewport;

pub use host::Said;

/// The undo step of a run that names none.
pub const STEP: &str = "Python";
/// The most lines the console keeps; the oldest go.
const MOST_LINES: usize = 4000;
/// The most runs the history keeps.
const HISTORY: usize = 200;

/// What a line of the console is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The code run, as it was typed.
    Input,
    /// What the code printed.
    Out,
    /// What it wrote to its errors (warnings, a C library).
    Err,
    /// A traceback, or why the console could not run.
    Error,
    /// The console's own word (stopped, taken back).
    Note,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub kind: Kind,
    pub text: String,
}

/// Code running now.
pub struct Run {
    pub id: u64,
    /// The drawing it runs on (`Document::session`), 0 without one.
    session: u64,
    /// Its undo step, open from its first write.
    group: Option<kentos_domain::Group>,
    /// The step's name, from the script's first `doc.group`.
    label: Option<String>,
    depth: usize,
    /// A `doc.group` failed: the run is taken back at its end.
    poisoned: bool,
    /// The drawing's refusal said once while it runs.
    told: bool,
    /// For the command history: the file, or the first line typed.
    title: String,
}

/// The Python tab's two sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// Code typed and run line by line.
    #[default]
    Console,
    /// A script written, kept in a file, run whole or in part (script.rs).
    Script,
}

/// The console's state.
pub struct Console {
    pub mode: Mode,
    /// The Betik side's script (script.rs).
    pub script: script::Script,
    pub lines: Vec<Line>,
    pub input: text_editor::Content,
    history: Vec<String>,
    /// Where ↑ and ↓ are in the history, and what was typed before.
    browsing: Option<(usize, String)>,
    host: Option<host::Host>,
    /// The process messages belong to: a stopped one's late ones are dropped.
    generation: u64,
    /// “Python 3.14.4 · kentos 0.1.0” once the process said it is ready.
    pub ready: Option<String>,
    pub running: Option<Run>,
    runs: u64,
    python: PathBuf,
    /// The names that can end the word at the cursor, while the list is open (assist.rs).
    pub completion: Option<assist::List>,
    /// The call the cursor is in.
    pub signature: Option<assist::Signature>,
    /// The last questions asked of the process: older answers are dropped.
    asked_completion: u64,
    asked_signature: u64,
}

impl Default for Console {
    fn default() -> Self {
        Self {
            mode: Mode::Console,
            script: script::Script::default(),
            lines: Vec::new(),
            input: text_editor::Content::new(),
            history: Vec::new(),
            browsing: None,
            host: None,
            generation: 0,
            ready: None,
            running: None,
            runs: 0,
            python: PathBuf::new(),
            completion: None,
            signature: None,
            asked_completion: 0,
            asked_signature: 0,
        }
    }
}

impl Console {
    fn push(&mut self, kind: Kind, text: impl Into<String>) {
        self.lines.push(Line {
            kind,
            text: text.into(),
        });
        if self.lines.len() > MOST_LINES {
            let over = self.lines.len() - MOST_LINES;
            self.lines.drain(..over);
        }
    }

    /// Text in lines (a chunk of output may hold several, or end without one).
    fn push_text(&mut self, kind: Kind, text: &str) {
        let text = text.strip_suffix('\n').unwrap_or(text);
        for line in text.split('\n') {
            self.push(kind, line.trim_end_matches('\r'));
        }
    }

    fn remember(&mut self, code: &str) {
        if self.history.last().map(String::as_str) != Some(code) {
            self.history.push(code.to_owned());
            if self.history.len() > HISTORY {
                self.history.remove(0);
            }
        }
        self.browsing = None;
    }

    /// Whether the process is running (not whether code is).
    pub fn started(&self) -> bool {
        self.host.is_some()
    }
}

/// What the Python tab asked for, or what its process said.
#[derive(Debug, Clone)]
pub enum Event {
    Edit(text_editor::Action),
    /// Çalıştır (Enter when the code is whole, Ctrl+Enter).
    Run,
    Stop,
    Restart,
    Clear,
    /// Betik aç…; the file picked, or none when the dialog was cancelled.
    Open,
    Picked(Option<PathBuf>),
    /// ↑ (-1) and ↓ (+1) through the runs.
    History(i32),
    /// Ctrl+Space, or Tab after a name or a dot: the names that can end it.
    Complete,
    /// ↑ (-1) and ↓ (+1) in the open list.
    Step(i32),
    /// An entry of the list taken: the one shown active, or the one clicked.
    Accept(Option<usize>),
    CloseList,
    /// Tab where nothing is to complete: four spaces.
    Indent,
    /// Konsol or Betik.
    Mode(Mode),
    /// The Betik side (script.rs).
    Script(script::Event),
    /// A message of the process of this generation.
    Host(u64, Said),
}

fn no_document() -> HeadlessError {
    HeadlessError::new(
        "no_document",
        "Masaüstünde açık çizim yok: önce bir çizim açın ya da yeni bir proje başlatın.",
    )
}

impl App {
    pub(crate) fn python_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Edit(action) => {
                let moved = !matches!(action, text_editor::Action::Scroll { .. });
                self.python.input.perform(action);
                if moved {
                    self.python_refilter();
                    return self.python_ask_signature();
                }
                Task::none()
            }
            Event::Complete => self.python_ask_completion(),
            Event::Mode(mode) => {
                self.python.mode = mode;
                self.python.completion = None;
                Task::none()
            }
            Event::Script(event) => self.script_event(event),
            Event::Step(step) => {
                if let Some(list) = &mut self.python.completion {
                    list.step(step);
                }
                Task::none()
            }
            Event::Accept(which) => {
                self.python_accept(which);
                self.python_ask_signature()
            }
            Event::CloseList => {
                self.python.completion = None;
                Task::none()
            }
            Event::Indent => {
                self.python
                    .input
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        std::sync::Arc::new("    ".to_owned()),
                    )));
                Task::none()
            }
            Event::Run => {
                let code = self.python.input.text();
                self.python_run(code, None)
            }
            Event::Stop => {
                self.python_stop();
                Task::none()
            }
            Event::Restart => {
                self.python_stop();
                self.python.push(
                    Kind::Note,
                    "Yeni oturum: tanımlanan adlar silindi; sonraki çalıştırma yeni bir Python'da.",
                );
                Task::none()
            }
            Event::Clear => {
                self.python.lines.clear();
                Task::none()
            }
            Event::Open => Task::perform(
                async {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title("Python betiği aç")
                        .add_filter("Python betiği (.py)", &["py"])
                        .pick_file()
                        .await?;
                    Some(file.path().to_path_buf())
                },
                |path| Message::Python(Event::Picked(path)),
            ),
            Event::Picked(None) => Task::none(),
            Event::Picked(Some(path)) => match std::fs::read_to_string(&path) {
                Ok(code) => self.python_run(code, Some(path.display().to_string())),
                Err(e) => {
                    self.python
                        .push(Kind::Error, format!("{} okunamadı: {e}", path.display()));
                    Task::none()
                }
            },
            Event::History(step) => {
                self.python_history(step);
                Task::none()
            }
            Event::Host(generation, said) => {
                if generation != self.python.generation {
                    return Task::none();
                }
                self.python_said(said)
            }
        }
    }

    /// Runs code typed in, or a file's (`file` names it in the tracebacks).
    pub(crate) fn python_run(&mut self, code: String, file: Option<String>) -> Task<Message> {
        let code = code.trim_end().to_owned();
        if code.trim().is_empty() || self.python.running.is_some() {
            return Task::none();
        }
        self.python.completion = None;
        self.python.signature = None;
        let title = match &file {
            Some(name) => {
                self.python
                    .push(Kind::Note, format!("{name} çalıştırılıyor…"));
                name.clone()
            }
            None => {
                for (i, line) in code.lines().enumerate() {
                    let prompt = if i == 0 { ">>>" } else { "..." };
                    self.python.push(Kind::Input, format!("{prompt} {line}"));
                }
                self.python.remember(&code);
                self.python.input = text_editor::Content::new();
                code.lines().next().unwrap_or_default().trim().to_owned()
            }
        };
        let mut started = Task::none();
        if self.python.host.is_none() {
            match self.python_start() {
                Ok(task) => started = task,
                Err(e) => {
                    self.python.push(Kind::Error, e);
                    return view::follow();
                }
            }
        }
        self.python.runs += 1;
        let id = self.python.runs;
        let mut exec = json!({"type": "exec", "id": id, "code": code});
        if let Some(name) = &file {
            exec["name"] = json!(name);
        }
        let sent = self.python.host.as_ref().is_some_and(|h| h.send(&exec));
        if !sent {
            self.python.host = None;
            self.python.push(
                Kind::Error,
                "Python'a ulaşılamadı: yeniden başlatıldı, kodu yeniden çalıştırın.",
            );
            return Task::batch([started, view::follow()]);
        }
        self.python.running = Some(Run {
            id,
            session: self.document.as_ref().map_or(0, |d| d.session),
            group: None,
            label: None,
            depth: 0,
            poisoned: false,
            told: false,
            title,
        });
        Task::batch([started, view::follow()])
    }

    fn python_start(&mut self) -> Result<Task<Message>, String> {
        self.python.generation += 1;
        let python = host::interpreter();
        let (process, task) = host::Host::start(&python, self.python.generation, |g, said| {
            Message::Python(Event::Host(g, said))
        })?;
        self.python.host = Some(process);
        self.python.ready = None;
        self.python.python = python;
        Ok(task)
    }

    fn python_said(&mut self, said: Said) -> Task<Message> {
        match said {
            Said::Message(m) => match m.get("type").and_then(Value::as_str) {
                Some("ready") => {
                    let python = m.get("python").and_then(Value::as_str).unwrap_or("?");
                    let kentos = m.get("kentos").and_then(Value::as_str).unwrap_or("?");
                    self.python.ready = Some(format!("Python {python} · kentos {kentos}"));
                    Task::none()
                }
                Some("out") => {
                    let kind = match m.get("stream").and_then(Value::as_str) {
                        Some("err") => Kind::Err,
                        _ => Kind::Out,
                    };
                    let text = m.get("text").and_then(Value::as_str).unwrap_or_default();
                    self.python.push_text(kind, text);
                    view::follow()
                }
                Some("call") => {
                    self.python_call(&m);
                    Task::none()
                }
                Some("done") => {
                    self.python_done(&m);
                    view::follow()
                }
                Some("completions") => {
                    if m.get("id").and_then(Value::as_u64) == Some(self.python.asked_completion) {
                        self.python.completion = assist::List::of(&m);
                        self.python_refilter();
                    }
                    Task::none()
                }
                Some("signature") => {
                    if m.get("id").and_then(Value::as_u64) == Some(self.python.asked_signature) {
                        self.python.signature = assist::Signature::of(&m);
                    }
                    Task::none()
                }
                _ => Task::none(),
            },
            Said::Garbled(line) => {
                self.python.push(Kind::Err, line);
                view::follow()
            }
            Said::Stderr(text) => {
                self.python.push_text(Kind::Err, &text);
                view::follow()
            }
            Said::Closed => {
                self.python_closed();
                view::follow()
            }
        }
    }

    /// A request of the running code, answered on the drawing.
    fn python_call(&mut self, m: &Value) {
        let id = m.get("id").cloned().unwrap_or(Value::Null);
        let method = m.get("method").and_then(Value::as_str).unwrap_or_default();
        let params = m.get("params").cloned().unwrap_or(Value::Null);
        let reply = match self.python_answer(method, &params) {
            Ok(result) => json!({"type": "reply", "id": id, "ok": true, "result": result}),
            Err(e) => {
                json!({"type": "reply", "id": id, "ok": false, "code": e.code, "message": e.message})
            }
        };
        if let Some(h) = &self.python.host {
            h.send(&reply);
        }
    }

    fn python_answer(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        let Some(run) = self.python.running.as_mut() else {
            return Err(HeadlessError::new("busy", "Konsolda çalışan bir kod yok."));
        };
        // A run is one step already: a group names it, a failed one takes it back.
        match method {
            "begin_group" => {
                if run.group.is_none() && run.label.is_none() {
                    run.label = params
                        .get("label")
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                }
                run.depth += 1;
                return Ok(Value::Null);
            }
            "end_group" => {
                run.depth = run.depth.saturating_sub(1);
                return Ok(json!(true));
            }
            "cancel_group" => {
                run.depth = run.depth.saturating_sub(1);
                run.poisoned = true;
                return Ok(json!(true));
            }
            _ => {}
        }
        let Some(doc) = self.document.as_mut() else {
            return Err(no_document());
        };
        if run.session != doc.session {
            return Err(no_document());
        }
        let writes = method == "run" && params.get("op").and_then(Value::as_str) == Some("execute");
        if writes && run.group.is_none() {
            run.group = Some(doc.model.begin_group(run.label.as_deref().unwrap_or(STEP)));
        }
        let mut answer = kentos_headless::rpc::call(&mut doc.model, method, params)?;
        if method == "summary"
            && let Some(fields) = answer.as_object_mut()
        {
            if let Some(path) = &doc.path {
                fields.insert("path".into(), json!(path.display().to_string()));
            }
            fields.insert("legacy".into(), json!(doc.legacy));
        }
        Ok(answer)
    }

    /// The code ended: its step is kept, or taken back when it raised or a group failed.
    fn python_done(&mut self, m: &Value) {
        let id = m.get("id").and_then(Value::as_u64);
        let Some(run) = self.python.running.take_if(|r| Some(r.id) == id) else {
            return;
        };
        let ok = m.get("ok").and_then(Value::as_bool) == Some(true);
        if !ok {
            let error = m.get("error").and_then(Value::as_str).unwrap_or_default();
            self.python.push_text(Kind::Error, error);
            // The script run whole: the cursor goes to the line it stopped on.
            if run.title == self.python.script.run_name()
                && let Some(line) = script::failed_line(error, &run.title)
            {
                self.python.script.show_line(line);
            }
        }
        let poisoned = run.poisoned;
        let title = run.title.clone();
        let exception = m
            .get("exception")
            .and_then(Value::as_str)
            .unwrap_or("hata")
            .to_owned();
        let wrote = self.python_close(run, ok && !poisoned);
        match (ok, poisoned, wrote) {
            (true, false, true) => self.output(format!("Python: {title}")),
            (true, true, true) => {
                self.python.push(
                    Kind::Note,
                    "Betikte bir grup başarısız oldu: bu çalıştırmanın çizime yazdıkları geri alındı.",
                );
            }
            (false, _, true) => {
                self.python.push(
                    Kind::Note,
                    "Bu çalıştırmanın çizime yazdıkları geri alındı.",
                );
                self.warn(format!(
                    "Python: {exception}; çalıştırmanın çizime yazdıkları geri alındı."
                ));
            }
            _ => {}
        }
    }

    /// Ends a run's step, kept or taken back; whether it had written.
    fn python_close(&mut self, run: Run, keep: bool) -> bool {
        let Some(group) = run.group else {
            return false;
        };
        let Some(doc) = self.document.as_mut().filter(|d| d.session == run.session) else {
            return false;
        };
        // Inside the group nothing is an edit yet: what it gathered says whether it wrote.
        let wrote = doc.model.group_changes() > 0;
        if keep {
            doc.model.end_group(group);
        } else {
            doc.model.cancel_group(group);
        }
        wrote
    }

    /// Durdur: the process goes (a new one for the next run) and what the
    /// running code wrote is taken back.
    pub(crate) fn python_stop(&mut self) {
        let run = self.python.running.take();
        self.python.host = None;
        self.python.generation += 1;
        self.python.ready = None;
        if let Some(run) = run {
            let wrote = self.python_close(run, false);
            self.python.push(
                Kind::Note,
                if wrote {
                    "Durduruldu: bu çalıştırmanın çizime yazdıkları geri alındı."
                } else {
                    "Durduruldu."
                },
            );
        }
    }

    /// The process ended by itself.
    fn python_closed(&mut self) {
        let was_ready = self.python.ready.is_some();
        self.python.host = None;
        self.python.ready = None;
        if let Some(run) = self.python.running.take() {
            let wrote = self.python_close(run, false);
            self.python.push(
                Kind::Error,
                if wrote {
                    "Python beklenmedik biçimde kapandı; bu çalıştırmanın çizime yazdıkları geri alındı."
                } else {
                    "Python beklenmedik biçimde kapandı."
                },
            );
        }
        if !was_ready {
            self.python.push(
                Kind::Error,
                format!(
                    "Python konsolu başlamadı ({}): kentos paketi bu Python'da kurulu olmayabilir. Geliştirme ortamını `pnpm py:test` kurar; başka bir Python için KENTOS_PYTHON.",
                    self.python.python.display()
                ),
            );
        }
    }

    /// Asks the process what can end the word at the cursor (it starts the
    /// process when there is none yet); not while code runs.
    fn python_ask_completion(&mut self) -> Task<Message> {
        if self.python.running.is_some() {
            return Task::none();
        }
        let mut started = Task::none();
        if self.python.host.is_none() {
            match self.python_start() {
                Ok(task) => started = task,
                Err(e) => {
                    self.python.push(Kind::Error, e);
                    return view::follow();
                }
            }
        }
        let (code, cursor) = assist::caret(&self.python.input);
        self.python.asked_completion += 1;
        let ask = json!({"type": "complete", "id": self.python.asked_completion, "code": code, "cursor": cursor});
        if let Some(h) = &self.python.host {
            h.send(&ask);
        }
        started
    }

    /// Asks for the call the cursor is in, when it is in one and the process runs idle.
    fn python_ask_signature(&mut self) -> Task<Message> {
        let (code, cursor) = assist::caret(&self.python.input);
        let before: String = code.chars().take(cursor).collect();
        if !before.contains('(') || self.python.running.is_some() {
            self.python.signature = None;
            return Task::none();
        }
        let Some(h) = &self.python.host else {
            return Task::none();
        };
        self.python.asked_signature += 1;
        h.send(&json!({"type": "signature", "id": self.python.asked_signature, "code": code, "cursor": cursor}));
        Task::none()
    }

    /// The open list follows what is typed; it closes when the word ends.
    fn python_refilter(&mut self) {
        let Some(list) = &mut self.python.completion else {
            return;
        };
        let (code, cursor) = assist::caret(&self.python.input);
        if !list.follow(&code, cursor) {
            self.python.completion = None;
        }
    }

    /// Puts the entry in place of the word it ends.
    fn python_accept(&mut self, which: Option<usize>) {
        let Some(list) = self.python.completion.take() else {
            return;
        };
        let Some(choice) = list.chosen(which) else {
            return;
        };
        let (code, cursor) = assist::caret(&self.python.input);
        assist::replace(
            &mut self.python.input,
            &code,
            list.start,
            cursor,
            &choice.text,
        );
    }

    fn python_history(&mut self, step: i32) {
        let c = &mut self.python;
        if c.history.is_empty() {
            return;
        }
        let last = c.history.len() - 1;
        let (at, draft) = match c.browsing.take() {
            Some((at, draft)) => (Some(at), draft),
            None => (None, c.input.text()),
        };
        let next = match (at, step < 0) {
            (None, true) => Some(last),
            (None, false) => None,
            (Some(0), true) => Some(0),
            (Some(i), true) => Some(i - 1),
            (Some(i), false) if i < last => Some(i + 1),
            (Some(_), false) => None,
        };
        match next {
            Some(i) => {
                c.input = text_editor::Content::with_text(&c.history[i]);
                c.browsing = Some((i, draft));
            }
            None => {
                c.input = text_editor::Content::with_text(draft.trim_end());
            }
        }
    }

    /// While code runs the drawing takes no other edit; the first refusal is
    /// said once (the web's words for a busy document).
    pub(crate) fn while_scripting(&mut self, message: &Message) -> Option<Task<Message>> {
        let run = self.python.running.as_mut()?;
        let quiet = match message {
            // Shortcuts wait silently; the console's own keys are its editor's.
            Message::Key(_) => true,
            Message::Run(_)
            | Message::RunMethod { .. }
            | Message::SplitChosen { .. }
            | Message::CommandSubmitted
            | Message::CommandRun(_)
            | Message::PromptOption(_)
            | Message::PointCalc(_)
            | Message::LayerVisible(_)
            | Message::LayerLocked(_)
            | Message::Layer(_)
            | Message::Properties(_)
            | Message::TextField(_)
            | Message::DrawingMenu(_)
            | Message::Settings(_)
            | Message::Viewport(viewport::Event::Pressed(_) | viewport::Event::RightClick(_)) => {
                false
            }
            _ => return None,
        };
        if !quiet && !run.told {
            run.told = true;
            self.output(
                "Python kodu çalışıyor: çizim, çalıştırma bitene ya da Durdur'a basılana kadar düzenlenemez.",
            );
        }
        Some(Task::none())
    }
}
