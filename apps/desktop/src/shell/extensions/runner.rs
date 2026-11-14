//! Bir eklentinin Python'u çalışırken (plan/eklenti.md ADIM 19; docs/adr/8057 §6): an extension's commands run in a
//! process of its own, one for each extension version, in the extension's own environment (environment.rs):
//! `<ortam>/python -I -m kentos.ext_host`, started when one of its contributions is first called (PY-RUN-05).
//!
//! - **The broker:** the process asks for the drawing as the console does (docs/adr/0132), and KentOS answers on its
//!   own drawing within what the user granted: reading with `document.read`, writing with `document.write`
//!   (PY-RUN-06…08); never another drawing than the one open when the run began (PY-RUN-16). A run is one undo step,
//!   named after the command, taken back when it raises or is stopped. An extension is a third party's code, not the
//!   user's own: it reads the drawing as an agent does, the values of the fields the drawing hides cut and a call
//!   naming them refused (docs/adr/0738, 0743); the console's code is its owner's and reads it whole.
//! - **One writer at a time:** while a run that may write goes, the drawing takes no other edit, as while the
//!   console's code runs (an edit of the user's would join its step, or be taken back with it); looking, panning and
//!   Durdur go on (EKL-04). Nor does the console, an agent or another extension write meanwhile.
//! - **Durdur:** first asked of the code (`ctx.check()`); a process that has not answered in [`KILL_AFTER`] is ended
//!   and what the run wrote is taken back (PY-RUN-12). The next call starts a new process.
//! - **Told plainly:** a failure is written to the extension's log with its traceback and said by a notice naming the
//!   extension; a process that ends by itself is a crash for the watchdog (PY-RUN-13, D6).
//! - **A separate process is not a sandbox** (PY-RUN-19, 20): an extension runs with the user's rights; the broker
//!   only bounds what it asks of KentOS.
//! - **Its package stays as installed:** Python writes the bytecode of what it imports beside the source
//!   (`__pycache__`), and the package's integrity check rightly refuses files added to it (a `.pyc` there runs in the
//!   source's place): the extension stopped at the next start after its first run (measured in the real window, 14
//!   November 2026). The bytecode goes to the environment's own folder (`-X pycache_prefix`, which `-I` keeps), and
//!   is removed with it.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::Task;
use kentos_extension_host::Record;
use kentos_extensions::manifest::Capability;
use kentos_headless::HeadlessError;
use kentos_ui::widget::Toast;
use serde_json::{Value, json};

use super::{Event, msg};
use crate::app::{App, Message};
use crate::python::host::{Host, Said};

/// How long a stopped run is given before its process is ended.
pub(crate) const KILL_AFTER: Duration = Duration::from_secs(5);

/// What a run keeps of what it printed.
const KEPT_OUTPUT: usize = 64 * 1024;

/// A contribution called, as KentOS runs it.
#[derive(Debug)]
pub(crate) struct ExtRun {
    pub(crate) id: u64,
    /// `command` …, as the process is told.
    pub(crate) kind: &'static str,
    pub(crate) local: String,
    pub(crate) title: String,
    pub(crate) started: Instant,
    /// The drawing it runs on (`Document::session`); none without one.
    pub(crate) session: Option<u64>,
    /// Its undo step, open from its first write.
    pub(crate) group: Option<kentos_domain::Group>,
    pub(crate) label: Option<String>,
    /// A group of its code failed: taken back at its end.
    pub(crate) poisoned: bool,
    /// Durdur pressed.
    pub(crate) stopping: Option<Instant>,
    /// The user was told the drawing waits for it.
    pub(crate) told: bool,
    /// What it printed and wrote to its errors, the newest [`KEPT_OUTPUT`] bytes.
    pub(crate) output: String,
    /// How far it said it is: done, total, words.
    pub(crate) progress: Option<(f64, Option<f64>, Option<String>)>,
    /// What it was granted of the drawing.
    pub(crate) read: bool,
    pub(crate) write: bool,
    /// The notice saying it runs, with its Durdur.
    pub(crate) notice: Option<kentos_ui::widget::toast::Id>,
}

/// An extension version's process.
pub(crate) struct Runner {
    pub(crate) version: String,
    pub(crate) generation: u64,
    host: Host,
    pub(crate) ready: bool,
    pub(crate) run: Option<ExtRun>,
}

impl std::fmt::Debug for Runner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runner")
            .field("version", &self.version)
            .field("generation", &self.generation)
            .field("ready", &self.ready)
            .field("run", &self.run)
            .finish_non_exhaustive()
    }
}

/// What a run of a contribution needs, found from the manifest, the grants and the environment, ready to start.
#[derive(Clone, Debug)]
pub(crate) struct Launch {
    kind: &'static str,
    handler: String,
    title: String,
    /// The environment's Python.
    python: PathBuf,
    /// The package's folder, its code's root.
    source: PathBuf,
    /// Where its bytecode goes: its environment's folder, never the package's.
    pycache: PathBuf,
    /// The content digest of the version it runs, as its files were checked against it.
    digest: String,
    version: String,
    read: bool,
    write: bool,
}

#[cfg(test)]
impl Launch {
    /// A run of a command as the background step would hand it over: reading, and writing when `write`.
    pub(crate) fn for_test(
        python: PathBuf,
        source: PathBuf,
        handler: &str,
        title: &str,
        write: bool,
    ) -> Launch {
        Launch {
            kind: "command",
            handler: handler.to_owned(),
            title: title.to_owned(),
            pycache: source.join("pyc-test"),
            digest: "sha256:0".to_owned(),
            python,
            source,
            version: "1.0.0".to_owned(),
            read: true,
            write,
        }
    }
}

/// The interpreter's arguments for an extension's process: isolated (`-I`: no user site, no environment variables,
/// not the working folder on the path), its bytecode in `pycache`.
pub(crate) fn interpreter_args(pycache: &Path) -> Vec<OsString> {
    let mut prefix = OsString::from("pycache_prefix=");
    prefix.push(pycache);
    vec![
        "-I".into(),
        "-X".into(),
        prefix,
        "-m".into(),
        "kentos.ext_host".into(),
    ]
}

/// What finding the environment needs, gathered without touching the disk.
struct Plan {
    launch: Launch,
    uv: PathBuf,
    root: PathBuf,
    interpreter: kentos_python_env::python::Interpreter,
    lock_file: String,
}

fn refused(code: &'static str, message: &str) -> HeadlessError {
    HeadlessError::new(code, message)
}

/// Whether a method of the broker writes the drawing: a command's execute (by `run`, or by `call`, whose form is
/// execute when not given) and a replay.
fn writes(method: &str, params: &Value) -> bool {
    let op = params.get("op").and_then(Value::as_str);
    match method {
        "run" => op == Some("execute"),
        "call" => op.is_none_or(|o| o == "execute"),
        "replay" => true,
        _ => false,
    }
}

/// What the broker answers at all; anything else (undo, redo, files) is the user's, never an extension's.
fn answered(method: &str) -> bool {
    matches!(
        method,
        "summary"
            | "layers"
            | "blocks"
            | "arrays"
            | "constraints"
            | "entities"
            | "entity"
            | "measure"
            | "measureMany"
            | "select"
            | "selection"
            | "run"
            | "call"
            | "replay"
            | "relax"
    )
}

impl App {
    /// The contribution's handler and what finding its environment needs; why not, in a sentence, when it cannot run.
    /// Nothing here reads the disk: the window does not wait.
    fn ext_plan(&self, ext: &str, kind: &str, local: &str) -> Result<Plan, String> {
        let registry = self
            .extensions
            .registry
            .as_ref()
            .ok_or("Eklenti kayıt defteri açık değil.")?;
        let manifest = registry.manifest(ext).ok_or("Eklenti kurulu değil.")?;
        let (handler, title) = match kind {
            "command" => manifest
                .entrypoints
                .commands
                .iter()
                .find(|c| c.id == local)
                .map(|c| (c.handler.clone(), c.title.clone())),
            _ => None,
        }
        .ok_or_else(|| format!("“{local}” bu eklentinin bir komutu değil."))?;
        let grant = registry.grant(ext).ok_or("Eklenti kurulu değil.")?;
        if !grant.python {
            return Err("Python işlevleri kurulumda seçilmedi.".to_owned());
        }
        let project = manifest
            .python
            .as_ref()
            .ok_or("Eklentinin Python işlevi yok.")?;
        let source = registry.package_dir(ext).ok_or("Eklentinin paketi yok.")?;
        let p = &self.python_setup;
        let (Some(crate::python_setup::OwnPython::Ready(interpreter, _)), Some(root), Some(uv)) =
            (&p.python, &p.root, p.ready_uv())
        else {
            return Err(p.why_not().to_owned());
        };
        Ok(Plan {
            launch: Launch {
                kind: "command",
                handler,
                title,
                python: PathBuf::new(),
                pycache: PathBuf::new(),
                digest: registry.digest(ext).unwrap_or_default(),
                source,
                version: manifest.version.to_string(),
                read: grant.capabilities.contains(&Capability::DocumentRead)
                    || grant.capabilities.contains(&Capability::DocumentWrite),
                write: grant.capabilities.contains(&Capability::DocumentWrite),
            },
            uv,
            root: root.clone(),
            interpreter: interpreter.clone(),
            lock_file: project.lock_file.clone(),
        })
    }

    /// Runs an extension's contribution: its environment found in the background (and this KentOS's library put in it
    /// when it holds another), then its process started when it has none and the call sent.
    pub(crate) fn ext_run(&mut self, ext: &str, kind: &str, local: &str) -> Task<Message> {
        // An İşlemler tool opens its window; it runs from there, as the built-in tools do (tools.rs).
        if kind == "processing" {
            return self.processing_command(&format!("processing.run.ext.{ext}.{local}"));
        }
        let name = self.extension_name(ext);
        if let Some(run) = self
            .extensions
            .runners
            .get(ext)
            .and_then(|r| r.run.as_ref())
        {
            self.toast(
                Toast::info(format!("{name}: “{}” sürüyor", run.title))
                    .body("Bitmesini bekleyin ya da Durdur'a basın."),
            );
            return Task::none();
        }
        let plan = match self.ext_plan(ext, kind, local) {
            Ok(plan) => plan,
            Err(why) => {
                self.toast(Toast::warning(format!("{name}: çalıştırılamadı")).body(why));
                return Task::none();
            }
        };
        let (owner, kind, local) = (ext.to_owned(), kind.to_owned(), local.to_owned());
        let asked = owner.clone();
        crate::python_setup::in_background(
            move || {
                let lock = kentos_python_env::extension::lock_digest(
                    &plan.launch.source,
                    &plan.lock_file,
                )?;
                let ready = kentos_python_env::extension::find(
                    &plan.root,
                    &asked,
                    &plan.launch.version,
                    &lock,
                    &plan.interpreter.version,
                )
                .ok_or_else(|| {
                    "Bu eklentinin Python ortamı hazırlanmadı: Eklentiler › Python ortamı."
                        .to_owned()
                })?;
                let wheel = kentos_python_env::sdk::find(
                    &kentos_python_env::sdk::dirs(),
                    env!("CARGO_PKG_VERSION"),
                )?;
                let ready = kentos_python_env::extension::refresh_library(
                    &plan.uv, &plan.root, &ready, &wheel,
                )?;
                Ok(Launch {
                    pycache: kentos_python_env::extension::place(&plan.root, &ready.key)
                        .join(kentos_python_env::extension::BYTECODE),
                    python: ready.python,
                    ..plan.launch
                })
            },
            Err("hazırlık yarıda kaldı".to_owned()),
            move |launch| {
                msg(Event::Launch(
                    owner.clone(),
                    kind.clone(),
                    local.clone(),
                    launch,
                ))
            },
        )
    }

    /// The environment found: the process started when it has none, the call sent, a notice with Durdur.
    pub(crate) fn ext_launch(
        &mut self,
        ext: &str,
        kind: &str,
        local: &str,
        launch: Result<Launch, String>,
    ) -> Task<Message> {
        let name = self.extension_name(ext);
        let call = match launch {
            Ok(call) => call,
            Err(why) => {
                self.toast(Toast::warning(format!("{name}: çalıştırılamadı")).body(why));
                return Task::none();
            }
        };
        if self
            .extensions
            .runners
            .get(ext)
            .is_some_and(|r| r.run.is_some())
        {
            return Task::none();
        }
        // One code writes the drawing at a time (the console's, an agent's, another extension's).
        if call.write && self.document.is_some() {
            let busy = if self.python.running.is_some() {
                Some("Konsolda kod çalışıyor".to_owned())
            } else {
                self.extension_writing()
                    .map(|writer| format!("{writer} çizime yazıyor"))
            };
            if let Some(busy) = busy {
                self.toast(
                    Toast::info(format!("{name}: “{}” bekletildi", call.title))
                        .body(format!("{busy}; bitince yeniden çalıştırın.")),
                );
                return Task::none();
            }
        }
        let mut task = Task::none();
        // A process of an older version is let go of; one of this version is kept between runs.
        if self
            .extensions
            .runners
            .get(ext)
            .is_some_and(|r| r.version != call.version)
        {
            self.extensions.runners.remove(ext);
        }
        if !self.extensions.runners.contains_key(ext) {
            self.extensions.runner_generation += 1;
            let generation = self.extensions.runner_generation;
            let owner = ext.to_owned();
            let args = interpreter_args(&call.pycache);
            let args: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
            match Host::start_with(&call.python, &args, generation, move |g, said| {
                msg(Event::Runner(owner.clone(), g, said))
            }) {
                Ok((host, heard)) => {
                    host.send(&json!({
                        "type": "hello",
                        "protocol": 1,
                        "extension": ext,
                        "version": call.version,
                        "digest": call.digest,
                        "source": call.source.display().to_string(),
                        "paths": ["src"],
                    }));
                    self.extensions.runners.insert(
                        ext.to_owned(),
                        Runner {
                            version: call.version.clone(),
                            generation,
                            host,
                            ready: false,
                            run: None,
                        },
                    );
                    task = heard;
                }
                Err(why) => {
                    self.extension_failed(
                        ext,
                        &format!("{kind}:{local}"),
                        "runner_not_started",
                        &why,
                        None,
                    );
                    return Task::none();
                }
            }
        }
        self.extensions.run_counter += 1;
        let id = self.extensions.run_counter;
        let session = self.document.as_ref().map(|d| d.session);
        let notice = self.toasts.push(
            Toast::info(format!("{name}: “{}” çalışıyor", call.title))
                .body("Ayrı süreçte, kendi Python ortamında.")
                .action("Durdur", msg(Event::StopRun(ext.to_owned())))
                .sticky(),
        );
        let Some(runner) = self.extensions.runners.get_mut(ext) else {
            return task;
        };
        runner.host.send(&json!({
            "type": "invoke",
            "id": id,
            "kind": call.kind,
            "local": local,
            "handler": call.handler,
            "params": {},
            "settings": {},
            "document": session.is_some() && call.read,
        }));
        runner.run = Some(ExtRun {
            id,
            kind: call.kind,
            local: local.to_owned(),
            title: call.title,
            started: Instant::now(),
            session,
            group: None,
            label: None,
            poisoned: false,
            stopping: None,
            told: false,
            output: String::new(),
            progress: None,
            read: call.read,
            write: call.write,
            notice: Some(notice),
        });
        if let Some(registry) = self.extensions.registry.as_mut() {
            registry.write_log(
                Record::info("run_started", "Çalıştırıldı.")
                    .of(ext, None)
                    .part(format!("{kind}:{local}")),
            );
        }
        task
    }

    /// Durdur: asked of the code; its process ended if it has not answered in [`KILL_AFTER`].
    pub(crate) fn ext_stop(&mut self, ext: &str) -> Task<Message> {
        let Some(runner) = self.extensions.runners.get_mut(ext) else {
            return Task::none();
        };
        let Some(run) = runner.run.as_mut() else {
            return Task::none();
        };
        if run.stopping.is_some() {
            return Task::none();
        }
        run.stopping = Some(Instant::now());
        runner.host.send(&json!({"type": "cancel", "id": run.id}));
        let (owner, id) = (ext.to_owned(), run.id);
        Task::perform(
            async move {
                let (sent, got) = iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    std::thread::sleep(KILL_AFTER);
                    let _ = sent.send(());
                });
                let _ = got.await;
            },
            move |()| msg(Event::RunOverdue(owner.clone(), id)),
        )
    }

    /// A stopped run that has not answered: its process is ended, what it wrote taken back.
    pub(crate) fn ext_overdue(&mut self, ext: &str, id: u64) {
        let late = self
            .extensions
            .runners
            .get(ext)
            .and_then(|r| r.run.as_ref())
            .is_some_and(|run| run.id == id);
        if !late {
            return;
        }
        let Some(mut runner) = self.extensions.runners.remove(ext) else {
            return;
        };
        let name = self.extension_name(ext);
        if let Some(mut run) = runner.run.take() {
            self.ext_close(&mut run, false);
            self.dismiss_notice(&run);
            self.extension_worked(
                ext,
                &format!("{}:{}", run.kind, run.local),
                "run_killed",
                &format!(
                    "Durdur'a {} sn içinde yanıt vermedi; süreci sonlandırıldı, yazdıkları geri alındı.",
                    KILL_AFTER.as_secs()
                ),
            );
            self.toast(
                Toast::warning(format!("{name}: “{}” durduruldu", run.title))
                    .body("Kod Durdur'a yanıt vermedi; süreci sonlandırıldı, çizime yazdıkları geri alındı."),
            );
        }
        // Dropped: the process is ended (Host's Drop), its late lines carry an old generation.
        drop(runner);
    }

    /// The run that may write the open drawing now, as the user reads it: `MEBİS: “MEBİS katmanlarını kur”`.
    pub(crate) fn extension_writing(&self) -> Option<String> {
        let session = self.document.as_ref()?.session;
        let (ext, run) = self.extensions.runners.iter().find_map(|(ext, r)| {
            r.run
                .as_ref()
                .filter(|run| run.write && run.session == Some(session))
                .map(|run| (ext, run))
        })?;
        Some(format!("{}: “{}”", self.extension_name(ext), run.title))
    }

    /// While a run that may write goes the drawing takes no other edit; the first refusal is said once.
    pub(crate) fn while_extension_writes(&mut self, message: &Message) -> Option<Task<Message>> {
        let quiet = crate::python::held(message)?;
        let writer = self.extension_writing()?;
        let session = self.document.as_ref().map(|d| d.session);
        let run = self
            .extensions
            .runners
            .values_mut()
            .filter_map(|r| r.run.as_mut())
            .find(|run| run.write && run.session == session)?;
        if !quiet && !run.told {
            run.told = true;
            self.output(format!(
                "{writer} çalışıyor: çizim, bitene ya da Durdur'a basılana kadar düzenlenemez."
            ));
        }
        Some(Task::none())
    }

    fn dismiss_notice(&mut self, run: &ExtRun) {
        if let Some(id) = run.notice {
            self.toasts.dismiss(id);
        }
    }

    /// What an extension's process said.
    pub(crate) fn ext_said(&mut self, ext: &str, generation: u64, said: Said) -> Task<Message> {
        if self
            .extensions
            .runners
            .get(ext)
            .is_none_or(|r| r.generation != generation)
        {
            return Task::none();
        }
        match said {
            Said::Message(m) => self.ext_message(ext, &m),
            Said::Stderr(text) | Said::Garbled(text) => {
                if let Some(run) = self
                    .extensions
                    .runners
                    .get_mut(ext)
                    .and_then(|r| r.run.as_mut())
                {
                    keep(&mut run.output, &text);
                }
            }
            Said::Closed => self.ext_closed(ext),
        }
        Task::none()
    }

    fn ext_message(&mut self, ext: &str, m: &Value) {
        let kind = m.get("type").and_then(Value::as_str).unwrap_or_default();
        match kind {
            "ready" => {
                if let Some(runner) = self.extensions.runners.get_mut(ext) {
                    runner.ready = true;
                }
            }
            "refused" => {
                let said = m
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let mut title = None;
                if let Some(mut runner) = self.extensions.runners.remove(ext)
                    && let Some(run) = runner.run.take()
                {
                    self.dismiss_notice(&run);
                    title = Some(run.title);
                }
                let message = format!(
                    "Eklentinin ortamındaki KentOS kitaplığı bu KentOS'la konuşmuyor: {said}"
                );
                self.extension_failed(ext, "runner", "runner_refused", &message, None);
                if let Some(title) = title {
                    self.failure_notice(ext, &title, message);
                }
            }
            "out" => {
                let text = m.get("text").and_then(Value::as_str).unwrap_or_default();
                if let Some(run) = self
                    .extensions
                    .runners
                    .get_mut(ext)
                    .and_then(|r| r.run.as_mut())
                {
                    keep(&mut run.output, text);
                }
            }
            "progress" => {
                if let Some(run) = self
                    .extensions
                    .runners
                    .get_mut(ext)
                    .and_then(|r| r.run.as_mut())
                {
                    run.progress = Some((
                        m.get("done").and_then(Value::as_f64).unwrap_or(0.0),
                        m.get("total").and_then(Value::as_f64),
                        m.get("text").and_then(Value::as_str).map(str::to_owned),
                    ));
                }
            }
            "log" => {
                let text = m.get("text").and_then(Value::as_str).unwrap_or_default();
                let level = m.get("level").and_then(Value::as_str).unwrap_or("info");
                let part = self
                    .extensions
                    .runners
                    .get(ext)
                    .and_then(|r| r.run.as_ref())
                    .map(|run| format!("{}:{}", run.kind, run.local))
                    .unwrap_or_default();
                if let Some(registry) = self.extensions.registry.as_mut() {
                    let record = match level {
                        "error" => Record::error("run_log", text),
                        "warning" => Record::warning("run_log", text),
                        _ => Record::info("run_log", text),
                    };
                    registry.write_log(record.of(ext, None).part(&part));
                }
            }
            "call" => self.ext_broker(ext, m),
            "done" => self.ext_done(ext, m),
            _ => {}
        }
    }

    /// A request of the running code, answered on the drawing within its grants.
    fn ext_broker(&mut self, ext: &str, m: &Value) {
        let id = m.get("id").cloned().unwrap_or(Value::Null);
        let method = m
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let params = m.get("params").cloned().unwrap_or(Value::Null);
        let reply = match self.ext_answer(ext, &method, &params) {
            Ok(result) => json!({"type": "reply", "id": id, "ok": true, "result": result}),
            Err(e) => {
                json!({"type": "reply", "id": id, "ok": false, "code": e.code, "message": e.message})
            }
        };
        if let Some(runner) = self.extensions.runners.get(ext) {
            runner.host.send(&reply);
        }
    }

    #[cfg(test)]
    pub(crate) fn ext_answer_for_test(
        &mut self,
        ext: &str,
        method: &str,
        params: &Value,
    ) -> Result<Value, HeadlessError> {
        self.ext_answer(ext, method, params)
    }

    fn ext_answer(
        &mut self,
        ext: &str,
        method: &str,
        params: &Value,
    ) -> Result<Value, HeadlessError> {
        let Some(run) = self
            .extensions
            .runners
            .get_mut(ext)
            .and_then(|r| r.run.as_mut())
        else {
            return Err(refused("busy", "Eklentinin çalışan bir işlevi yok."));
        };
        match method {
            "begin_group" => {
                if run.group.is_none() && run.label.is_none() {
                    run.label = params
                        .get("label")
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                }
                return Ok(Value::Null);
            }
            "end_group" => return Ok(json!(true)),
            "cancel_group" => {
                run.poisoned = true;
                return Ok(json!(true));
            }
            _ => {}
        }
        if !answered(method) {
            return Err(refused(
                "not_for_extensions",
                "Eklentiler bunu isteyemez: geri alma, yineleme ve dosya işlemleri kullanıcınındır.",
            ));
        }
        let write = writes(method, params);
        if write && !run.write {
            return Err(refused(
                "capability",
                "Eklenti çizime yazma izni (document.write) istemedi; yazamaz.",
            ));
        }
        if !write && !run.read {
            return Err(refused(
                "capability",
                "Eklenti çizimi okuma izni (document.read) istemedi; okuyamaz.",
            ));
        }
        let Some(doc) = self.document.as_mut() else {
            return Err(refused("no_document", "Masaüstünde açık çizim yok."));
        };
        // Never another drawing than the one open when it began (PY-RUN-16).
        if run.session != Some(doc.session) {
            return Err(refused(
                "no_document",
                "Çalıştırıldığı çizim artık açık değil; başka bir çizime yazılmaz.",
            ));
        }
        if matches!(method, "select" | "selection") {
            return crate::python::selection_answer(
                &doc.model,
                &mut self.selection,
                method,
                params,
            );
        }
        if write && run.group.is_none() {
            // The undo step is named after the command, unless its code names it.
            run.group = Some(
                doc.model
                    .begin_group(run.label.as_deref().unwrap_or(&run.title)),
            );
        }
        kentos_headless::rpc::call_for_agent(
            &mut doc.model,
            None,
            doc.path.as_deref(),
            method,
            params,
        )
    }

    /// Ends a run's step, kept or taken back; whether it had written.
    fn ext_close(&mut self, run: &mut ExtRun, keep: bool) -> bool {
        let Some(group) = run.group.take() else {
            return false;
        };
        let Some(doc) = self
            .document
            .as_mut()
            .filter(|d| Some(d.session) == run.session)
        else {
            return false;
        };
        let wrote = doc.model.group_changes() > 0;
        if keep {
            doc.model.end_group(group);
        } else {
            doc.model.cancel_group(group);
        }
        wrote
    }

    fn ext_done(&mut self, ext: &str, m: &Value) {
        let id = m.get("id").and_then(Value::as_u64);
        let Some(mut run) = self
            .extensions
            .runners
            .get_mut(ext)
            .and_then(|r| r.run.take_if(|run| Some(run.id) == id))
        else {
            return;
        };
        self.dismiss_notice(&run);
        let name = self.extension_name(ext);
        let part = format!("{}:{}", run.kind, run.local);
        let ok = m.get("ok").and_then(Value::as_bool) == Some(true);
        let keep = ok && !run.poisoned;
        let wrote = self.ext_close(&mut run, keep);
        let seconds = run.started.elapsed().as_secs_f64();
        if ok {
            let result = m.get("result").cloned().unwrap_or(Value::Null);
            let rows = result
                .get("rows")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let said = result
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    if rows.is_empty() {
                        "Bitti.".to_owned()
                    } else {
                        format!("{} satır.", rows.len())
                    }
                });
            let title = result
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(&run.title)
                .to_owned();
            // The rows in the messages, as the console's output: the user reads and copies them there.
            if !rows.is_empty() {
                let columns: Vec<&str> = result
                    .get("columns")
                    .and_then(Value::as_array)
                    .map(|c| c.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                self.output(format!("{name} · {title}"));
                for row in rows.iter().take(500) {
                    self.output(format!("  {}", row_words(&columns, row)));
                }
                if rows.len() > 500 {
                    self.output(format!("  … {} satır daha", rows.len() - 500));
                }
            }
            let mut body = said.clone();
            if run.poisoned && wrote {
                body.push_str(" Bir grubu başarısız oldu: çizime yazdıkları geri alındı.");
            }
            self.extension_worked(ext, &part, "run_done", &format!("{said} ({seconds:.1} sn)"));
            self.toast(Toast::success(format!("{name}: {title}")).body(body));
            self.extensions.result = Some((ext.to_owned(), title, result));
        } else if m.get("stopped").and_then(Value::as_bool) == Some(true) {
            self.extension_worked(
                ext,
                &part,
                "run_stopped",
                "Durdur'la durdu; yazdıkları geri alındı.",
            );
            self.toast(
                Toast::info(format!("{name}: “{}” durduruldu", run.title)).body(if wrote {
                    "Çizime yazdıkları geri alındı."
                } else {
                    "Çizimde bir şey değişmedi."
                }),
            );
        } else if m.get("unavailable").and_then(Value::as_bool) == Some(true) {
            // An ability this KentOS lacks, not a fault of the extension's code: said calmly, no traceback.
            let said = m.get("message").and_then(Value::as_str).unwrap_or_default();
            if let Some(registry) = self.extensions.registry.as_mut() {
                registry.write_log(
                    Record::warning("run_unavailable", said)
                        .of(ext, None)
                        .part(&part),
                );
            }
            self.toast(
                Toast::warning(format!("{name}: “{}” bu sürümde çalışmaz", run.title)).body(
                    if wrote {
                        format!("{said} Çizime yazdıkları geri alındı.")
                    } else {
                        said.to_owned()
                    },
                ),
            );
        } else if m.get("refused").and_then(Value::as_bool) == Some(true) {
            let said = m.get("message").and_then(Value::as_str).unwrap_or_default();
            self.extension_failed(ext, &part, "handler_refused", said, None);
            self.failure_notice(ext, &run.title, said.to_owned());
        } else {
            let exception = m.get("exception").and_then(Value::as_str).unwrap_or("hata");
            let said = m.get("message").and_then(Value::as_str).unwrap_or_default();
            let mut detail = m
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if !run.output.is_empty() {
                detail.push_str("\n--- çıktı ---\n");
                detail.push_str(&run.output);
            }
            let mut message = format!("“{}” {exception} ile durdu: {said}", run.title);
            if wrote {
                message.push_str(" Çizime yazdıkları geri alındı.");
            }
            self.extension_failed(ext, &part, "run_failed", &message, Some(&detail));
            let mut body = format!("{exception}: {said}");
            if wrote {
                body.push_str(" Çizime yazdıkları geri alındı.");
            }
            self.failure_notice(ext, &run.title, body);
        }
    }

    /// A run the user started failed: said where its start was said, beside the status bar's standing cell, with the
    /// way to its log and traceback.
    pub(crate) fn failure_notice(&mut self, ext: &str, title: &str, body: String) {
        let name = self.extension_name(ext);
        self.toast(
            Toast::error(format!("{name}: “{title}” hata verdi"))
                .body(body)
                .action("Ayrıntı", msg(Event::Show(ext.to_owned()))),
        );
    }

    /// The process ended: during a run, a crash the watchdog counts; its writes taken back.
    fn ext_closed(&mut self, ext: &str) {
        let Some(mut runner) = self.extensions.runners.remove(ext) else {
            return;
        };
        let Some(mut run) = runner.run.take() else {
            return;
        };
        self.dismiss_notice(&run);
        let wrote = self.ext_close(&mut run, false);
        let part = format!("{}:{}", run.kind, run.local);
        let mut detail = run.output.clone();
        if detail.is_empty() {
            detail = "Süreç bir şey söylemeden kapandı.".to_owned();
        }
        let message = format!(
            "“{}” çalışırken eklentinin süreci kapandı{}",
            run.title,
            if wrote {
                "; çizime yazdıkları geri alındı."
            } else {
                "."
            }
        );
        if let Some(registry) = self.extensions.registry.as_mut()
            && let Some(lease) = registry.lease(ext).cloned()
        {
            let _ = registry.report_crash(&lease, &message);
        }
        self.extension_failed(ext, &part, "runner_crashed", &message, Some(&detail));
        self.failure_notice(ext, &run.title, message);
        self.extensions_changed();
    }
}

/// Keeps the newest [`KEPT_OUTPUT`] bytes of what a run printed.
fn keep(output: &mut String, text: &str) {
    output.push_str(text);
    if output.len() > KEPT_OUTPUT {
        let mut cut = output.len() - KEPT_OUTPUT;
        while !output.is_char_boundary(cut) {
            cut += 1;
        }
        output.drain(..cut);
    }
}

/// A row of a report in a line, its columns in the order the extension gave them: `yapı: B12 · mesafe_m: 3.2`;
/// a column it did not name comes after them.
pub(crate) fn row_words(columns: &[&str], row: &Value) -> String {
    let Some(fields) = row.as_object() else {
        return row.to_string();
    };
    let word = |k: &str, v: &Value| match v {
        Value::String(s) => format!("{k}: {s}"),
        other => format!("{k}: {other}"),
    };
    let named = columns
        .iter()
        .filter_map(|k| fields.get(*k).map(|v| word(k, v)));
    let others = fields
        .iter()
        .filter(|(k, _)| !columns.contains(&k.as_str()))
        .map(|(k, v)| word(k, v));
    named.chain(others).collect::<Vec<_>>().join(" · ")
}
