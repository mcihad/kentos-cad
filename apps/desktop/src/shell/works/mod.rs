//! Çalışan işler (plan/eklenti.md ADIM 21, EKL-06: "çalışan işler … kullanıcıya anlaşılır gösterilir"; docs/adr/8057
//! §102): the work going on in the background, in one list whoever started it (the user, Asistan, an agent on the
//! link): what it is, who asked for it, since when, how far it is, and its Durdur. The status bar's cell says it while
//! there is some; the bottom panel's İşler tab lists it (view.rs).
//!
//! - **Read where it runs, kept nowhere else:** each work is read from its own state every time (sources.rs): a
//!   processing run, an extension's command, its audit and its trial, an agent's job, Asistan's turn, the console's
//!   code, Python's install and environments. Nothing registers, so nothing stays listed after its work ended.
//! - **Who:** an agent's job, an audit and a trial say who asked for them; the rest only the user starts.
//! - **Its Durdur is its own:** the stop its own window, notice or tab has, with the same outcome (what it wrote taken
//!   back, its log, its notice). An agent's job is stopped without being written first; the agent hears `cancelled`
//!   when it next asks, Asistan's call too.
//! - **Not here:** work held by a window of its own while it runs (Topoloji, Dizi, CAD denetimi …: the window shows it
//!   and has its Durdur), and the catalog's jobs, which run on the server and are listed in its pane.

mod sources;
#[cfg(test)]
mod tests;
mod view;

use std::time::Instant;

pub(crate) use view::GLYPH;

use iced::Task;

use crate::app::{App, Message};
use crate::bottom::BottomTab;

/// Who started a work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum By {
    #[default]
    You,
    Assistant,
    /// A program on the agents' link (an MCP agent's).
    Agent,
}

impl By {
    /// Who an agent request is from, as the link and Asistan name themselves (`agent_request`'s `who`).
    pub(crate) fn of(who: &str) -> By {
        if who == "Asistan" {
            By::Assistant
        } else {
            By::Agent
        }
    }

    pub(crate) fn words(self) -> &'static str {
        match self {
            By::You => "Siz",
            By::Assistant => "Asistan",
            By::Agent => "Bağlı ajan",
        }
    }
}

/// Which work, for its buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Key {
    /// A processing run, by its id.
    Processing(u64),
    /// An extension's command, by the extension (one runs at a time in its process).
    Command(String),
    Audit,
    Trial,
    /// An agent's job, by its id.
    Job(String),
    Assistant,
    Console,
    /// KentOS's own Python being installed.
    Install,
    /// The personal environment's change.
    Personal,
    /// An extension's environment, by the extension.
    Environment(String),
}

/// One work as the list shows it.
#[derive(Clone, Debug)]
pub(crate) struct Work {
    pub(crate) key: Key,
    /// What kind of work: “İşlemler aracı”, “Eklenti komutu” …
    pub(crate) what: &'static str,
    pub(crate) title: String,
    pub(crate) by: By,
    /// When it began, where it is known.
    pub(crate) since: Option<Instant>,
    /// How far, 0 to 1, when it says.
    pub(crate) share: Option<f64>,
    /// Its step, or what it waits for.
    pub(crate) state: Option<String>,
    pub(crate) can_stop: bool,
    /// Durdur was pressed and it has not ended yet.
    pub(crate) stopping: bool,
    /// Where it shows more: its tab, its window.
    pub(crate) can_show: bool,
}

impl Work {
    fn new(key: Key, what: &'static str, title: impl Into<String>) -> Work {
        Work {
            key,
            what,
            title: title.into(),
            by: By::You,
            since: None,
            share: None,
            state: None,
            can_stop: true,
            stopping: false,
            can_show: false,
        }
    }
}

/// An agent's job as the desktop keeps it beside the jobs' table (agents.rs): who started it, on which drawing, when,
/// whether it was a plan, and what its request names (its command, the tool it runs).
#[derive(Clone, Debug)]
pub(crate) struct JobStart {
    pub(crate) by: By,
    pub(crate) session: u64,
    pub(crate) at: Instant,
    pub(crate) plan: bool,
    pub(crate) command: String,
    pub(crate) tool: Option<String>,
}

impl JobStart {
    /// What `who` started with `params` (the request) and `answer` (its status) on drawing `session`.
    pub(crate) fn of(
        who: &str,
        session: u64,
        plan: bool,
        params: &serde_json::Value,
        answer: &serde_json::Value,
    ) -> JobStart {
        JobStart {
            by: By::of(who),
            session,
            at: Instant::now(),
            plan,
            command: params
                .get("command")
                .or_else(|| answer.get("command"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("?")
                .to_owned(),
            tool: params
                .pointer("/input/tool")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        }
    }

    /// Its title: its command's in the catalog (its id until the catalog is read, or when it has none), and the tool
    /// it runs when it names one (`İşlemler aracını çalıştır · alan-ozeti`).
    pub(crate) fn title(&self) -> String {
        let title = kentos_headless::command_title(&self.command).unwrap_or(&self.command);
        match &self.tool {
            Some(tool) => format!("{title} · {tool}"),
            None => title.to_owned(),
        }
    }
}

/// The list's buttons.
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Stop(Key),
    Show(Key),
}

impl App {
    /// The works going on now, in the order they are read: the user's long work first, the agents' after.
    pub(crate) fn works(&self) -> Vec<Work> {
        let mut all = self.processing_works();
        all.extend(self.extension_works());
        all.extend(self.console_work());
        all.extend(self.python_works());
        all.extend(self.assistant_work());
        all.extend(self.agent_job_works());
        all
    }

    pub(crate) fn works_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Stop(key) => self.work_stop(key),
            Event::Show(key) => self.work_show(key),
        }
    }

    /// Durdur: the work's own stop, by its own handler (the message its own button sends).
    fn work_stop(&mut self, key: Key) -> Task<Message> {
        use crate::extensions::Event as Ext;
        use crate::python_setup::PythonEdit;
        match key {
            Key::Processing(id) => self.processing_stop_run(id),
            Key::Command(ext) => return self.extensions_event(Ext::StopRun(ext)),
            Key::Audit => self.ext_audit_stop(),
            Key::Trial => self.ext_trial_stop(),
            Key::Job(id) => self.agent_job_stop(&id),
            Key::Assistant => self.assistant_stop(),
            Key::Console => return self.python_event(crate::python::Event::Stop),
            Key::Install => {
                return self
                    .settings_edit(crate::settings_view::Edit::Python(PythonEdit::StopInstall));
            }
            Key::Personal => {
                return self.settings_edit(crate::settings_view::Edit::Python(
                    PythonEdit::Personal(crate::python_personal::PersonalEdit::Stop),
                ));
            }
            Key::Environment(ext) => {
                return self.extensions_event(Ext::Env(
                    crate::extensions::environment::EnvEvent::Stop(ext),
                ));
            }
        }
        Task::none()
    }

    /// Göster: where the work shows more.
    fn work_show(&mut self, key: Key) -> Task<Message> {
        match key {
            Key::Audit => self.show_bottom(BottomTab::ExtAudit),
            Key::Console | Key::Job(_) => self.show_bottom(BottomTab::Python),
            Key::Assistant => return self.show_assistant(),
            Key::Install | Key::Personal => {
                self.open_settings_at(crate::settings_sections::Section::Python);
            }
            Key::Environment(ext) | Key::Command(ext) => {
                self.open_extensions();
                self.extensions.part = crate::extensions::Part::Installed;
                return self.extensions_event(crate::extensions::Event::Choose(ext));
            }
            Key::Processing(_) | Key::Trial => {}
        }
        Task::none()
    }

    /// An agent's job stopped by the user: asked to stop, then asked how it is on its drawing so a computed result is
    /// dropped there and not written; the agent (or Asistan) hears `cancelled` when it next asks. Its record goes.
    fn agent_job_stop(&mut self, id: &str) {
        let python = &mut self.python;
        if !python.jobs.stop(id) {
            python.job_starts.remove(id);
            return;
        }
        let session = python.job_starts.get(id).map(|s| s.session);
        let doc = match &mut self.document {
            Some(d) if Some(d.session) == session => Some(d),
            _ => self
                .doc_tabs
                .parked
                .iter_mut()
                .map(|p| &mut p.document)
                .find(|d| Some(d.session) == session),
        };
        if let Some(doc) = doc {
            let _ = self.python.jobs.status(&mut doc.model, id);
        }
        let title = self
            .python
            .job_starts
            .remove(id)
            .map_or_else(|| id.to_owned(), |s| s.title());
        self.toast(
            kentos_ui::widget::Toast::info(format!("Ajan işi durduruldu: {title}"))
                .body("Çizime bir şey yazılmadı; işi başlatan ajan durdurulduğunu bir sonraki sorusunda öğrenir."),
        );
    }
}
