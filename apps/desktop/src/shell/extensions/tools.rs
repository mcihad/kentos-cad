//! Eklentilerin İşlemler araçları (plan/eklenti.md ADIM 19; docs/adr/8057 §7): the installed extensions' tools in the
//! toolbox, the window, the command line, models, batches, Python and MCP alike (`kentos_processing::external`), their
//! work done by the extension's Python in its own environment.
//!
//! - **Listed from the registry:** an extension that is on, granted Python and `processing.provider` gives its tools,
//!   each read from its descriptor (`tools/<id>.tool.json`) without its code; a descriptor that does not fit is written
//!   to the extension's log once a session and its tool left out.
//! - **Run off the window's thread:** a tool runs where İşlemler runs it (its run thread, a job's, a command's); the
//!   bridge finds the extension's environment there and runs the code in the extension version's process
//!   (`kentos_python_env::process::Pool`), kept between runs and closed when idle. The window waits for nothing.
//! - **Told plainly:** what happens is noted by the bridge and written by the window's thread (the extension's log, its
//!   failure notice with Ayrıntı, a crash for the watchdog), as a command's run is (runner.rs).
//! - **A separate process is not a sandbox** (PY-RUN-19, 20): the tool's help says so.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use kentos_extension_host::{ContributionKind, Record};
use kentos_processing::Feedback;
use kentos_processing::external::{self, ExternalTool, Failure, Provider};
use kentos_python_env::process::{Ended, Heard, Pool, Start};
use serde_json::{Value, json};

use crate::app::App;

/// What a run prints that goes to İşlemler' log, lines at most: the rest is in the extension's own output.
const PRINTED_LINES: usize = 200;

/// What running a tool needs, found when the registry or Python changes: no disk is read for it at the run.
#[derive(Clone, Debug)]
struct Plan {
    ext: String,
    local: String,
    handler: String,
    version: String,
    source: PathBuf,
    digest: String,
    lock_file: String,
}

/// KentOS's own Python, when ready: what finds an extension's environment.
#[derive(Clone, Debug)]
struct Python {
    interpreter: kentos_python_env::python::Interpreter,
    root: PathBuf,
    uv: PathBuf,
}

#[derive(Default)]
struct Shared {
    plans: HashMap<String, Plan>,
    python: Option<Python>,
    /// Why there is no Python, in a sentence.
    why_not: String,
}

/// What a run tells the window's thread.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Note {
    Started {
        ext: String,
        part: String,
    },
    Worked {
        ext: String,
        part: String,
        code: &'static str,
        message: String,
    },
    Failed {
        ext: String,
        part: String,
        title: String,
        code: &'static str,
        message: String,
        detail: Option<String>,
        /// Its process ended by itself: a crash the watchdog counts.
        crash: bool,
    },
}

/// The extensions' tools: what the bridge reads, its processes and its notes.
pub(crate) struct ExtTools {
    shared: Arc<RwLock<Shared>>,
    pub(crate) pool: Arc<Pool>,
    notes: Arc<Mutex<Vec<Note>>>,
    /// The descriptors' refusals already written to a log this session.
    said: BTreeSet<String>,
}

impl Default for ExtTools {
    fn default() -> Self {
        let tools = ExtTools {
            shared: Arc::default(),
            pool: Arc::new(Pool::default()),
            notes: Arc::default(),
            said: BTreeSet::new(),
        };
        external::register_provider(Arc::new(Bridge {
            shared: Arc::clone(&tools.shared),
            pool: Arc::clone(&tools.pool),
            notes: Arc::clone(&tools.notes),
        }));
        tools
    }
}

impl std::fmt::Debug for ExtTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtTools").finish_non_exhaustive()
    }
}

/// Runs the extensions' tools for İşlemler.
struct Bridge {
    shared: Arc<RwLock<Shared>>,
    pool: Arc<Pool>,
    notes: Arc<Mutex<Vec<Note>>>,
}

impl Bridge {
    fn note(&self, note: Note) {
        if let Ok(mut notes) = self.notes.lock() {
            notes.push(note);
        }
    }
}

/// The environment of `plan`'s version found, this KentOS's library in it, and how its process starts.
fn start_of(plan: &Plan, python: &Python) -> Result<Start, String> {
    let lock = kentos_python_env::extension::lock_digest(&plan.source, &plan.lock_file)?;
    let ready = kentos_python_env::extension::find(
        &python.root,
        &plan.ext,
        &plan.version,
        &lock,
        &python.interpreter.version,
    )
    .ok_or_else(|| {
        "Bu eklentinin Python ortamı hazırlanmadı: Eklentiler › Python ortamı.".to_owned()
    })?;
    let wheel =
        kentos_python_env::sdk::find(&kentos_python_env::sdk::dirs(), env!("CARGO_PKG_VERSION"))?;
    let ready =
        kentos_python_env::extension::refresh_library(&python.uv, &python.root, &ready, &wheel)?;
    let pycache = kentos_python_env::extension::place(&python.root, &ready.key)
        .join(kentos_python_env::extension::BYTECODE);
    Ok(Start {
        python: ready.python,
        args: super::runner::interpreter_args(&pycache),
        hello: json!({
            "type": "hello",
            "protocol": 1,
            "extension": plan.ext,
            "version": plan.version,
            "digest": plan.digest,
            "source": plan.source.display().to_string(),
            "paths": ["src"],
        }),
    })
}

impl Provider for Bridge {
    fn run(
        &self,
        tool: &ExternalTool,
        input: Value,
        feedback: &mut dyn Feedback,
    ) -> Result<Value, Failure> {
        let (plan, python, why_not) = {
            let shared = self
                .shared
                .read()
                .map_err(|_| Failure::Failed("Eklenti araçları okunamadı.".into()))?;
            (
                shared.plans.get(&tool.tool.id).cloned(),
                shared.python.clone(),
                shared.why_not.clone(),
            )
        };
        let label = &tool.tool.label;
        let plan = plan.ok_or_else(|| {
            Failure::Failed(format!(
                "“{label}” artık kurulu ve açık bir eklentinin aracı değil."
            ))
        })?;
        let python = python
            .ok_or_else(|| Failure::Failed(format!("“{label}” çalıştırılamadı: {why_not}")))?;
        let part = format!("processing:{}", plan.local);
        let key = format!("{}@{}", plan.ext, plan.version);
        let invoke = json!({
            "kind": "processing",
            "local": plan.local,
            "handler": plan.handler,
            "params": input.get("params").cloned().unwrap_or_else(|| json!({})),
            "inputs": input.get("inputs").cloned().unwrap_or_else(|| json!({})),
            "settings": {},
            "document": false,
        });
        self.note(Note::Started {
            ext: plan.ext.clone(),
            part: part.clone(),
        });
        let began = Instant::now();
        let feedback = RefCell::new(feedback);
        let mut printed = 0usize;
        let mut output = String::new();
        let ended = self.pool.run(
            &key,
            &|| start_of(&plan, &python),
            invoke,
            Duration::from_secs(tool.timeout),
            &|| feedback.borrow().canceled(),
            &mut |heard| {
                let mut f = feedback.borrow_mut();
                match heard {
                    Heard::Progress { done, total, text } => {
                        let share = total
                            .filter(|t| *t > 0.0)
                            .map_or(0.0, |t| (done / t).clamp(0.0, 1.0));
                        f.progress(share, text.as_deref().unwrap_or_default());
                    }
                    Heard::Log { level, text } if level == "info" => f.info(text),
                    Heard::Log { text, .. } => f.warn(text),
                    Heard::Out { text, .. } => {
                        if output.len() < 64 * 1024 {
                            output.push_str(&text);
                        }
                        for line in text.lines() {
                            if printed < PRINTED_LINES {
                                f.info(line.to_owned());
                            }
                            printed += 1;
                        }
                    }
                }
            },
        );
        let canceled = feedback.borrow().canceled();
        // What its process holds now (measured where the system says it): its log line says it.
        let memory = self
            .pool
            .running()
            .into_iter()
            .find(|r| r.key == key)
            .and_then(|r| r.memory);
        let run = Concluded {
            ext: &plan.ext,
            part: &part,
            label,
            timeout: tool.timeout,
            canceled,
            took: began.elapsed().as_secs_f64(),
            output: &output,
            memory,
        };
        let (answer, note) = run.conclude(ended);
        if let Some(note) = note {
            self.note(note);
        }
        answer
    }
}

/// How a run ended, read: what İşlemler gets and what the extension's log is told.
struct Concluded<'a> {
    ext: &'a str,
    part: &'a str,
    label: &'a str,
    timeout: u64,
    /// The user stopped it (Durdur).
    canceled: bool,
    took: f64,
    /// What it printed.
    output: &'a str,
    /// Its process's resident memory after the run, bytes, where the system says it.
    memory: Option<u64>,
}

/// A number as Turkish writes it: a decimal comma.
fn turkish(value: f64, decimals: usize) -> String {
    format!("{value:.decimals$}").replace('.', ",")
}

impl Concluded<'_> {
    fn worked(&self, code: &'static str, message: String) -> Option<Note> {
        Some(Note::Worked {
            ext: self.ext.to_owned(),
            part: self.part.to_owned(),
            code,
            message,
        })
    }

    fn failed(
        &self,
        code: &'static str,
        message: String,
        detail: Option<String>,
        crash: bool,
    ) -> (Result<Value, Failure>, Option<Note>) {
        let note = Note::Failed {
            ext: self.ext.to_owned(),
            part: self.part.to_owned(),
            title: self.label.to_owned(),
            code,
            message: message.clone(),
            detail,
            crash,
        };
        (Err(Failure::Failed(message)), Some(note))
    }

    fn conclude(&self, ended: Ended) -> (Result<Value, Failure>, Option<Note>) {
        let label = self.label;
        match ended {
            Ended::Answered(done) if done["ok"] == true => {
                let memory = self
                    .memory
                    .map(|b| format!("; süreç belleği {} MB", turkish(b as f64 / 1_048_576.0, 0)))
                    .unwrap_or_default();
                (
                    Ok(done["result"].clone()),
                    self.worked(
                        "run_finished",
                        format!("Bitti ({} sn{memory}).", turkish(self.took, 1)),
                    ),
                )
            }
            Ended::Answered(done) if done["stopped"] == true => {
                let words = done["stoppedAfterMs"].as_u64().map_or_else(
                    || "Durduruldu; çizim değişmedi.".to_owned(),
                    |ms| format!("Durdur'dan {ms} ms sonra durdu; çizim değişmedi."),
                );
                let note = self.worked("run_stopped", words);
                if self.canceled {
                    (Err(Failure::Stopped), note)
                } else {
                    (
                        Err(Failure::Failed(format!(
                            "“{label}” kendiliğinden durdu; çizim değişmedi."
                        ))),
                        note,
                    )
                }
            }
            Ended::Answered(done) => {
                let message = done["message"].as_str().unwrap_or_default();
                if done["unavailable"] == true {
                    return self.failed(
                        "run_unavailable",
                        format!("“{label}” bu KentOS sürümünde çalışmaz: {message}"),
                        None,
                        false,
                    );
                }
                if done["refused"] == true {
                    return self.failed(
                        "handler_refused",
                        format!("“{label}” çalıştırılmadı: {message}"),
                        None,
                        false,
                    );
                }
                let exception = done["exception"].as_str().unwrap_or("Hata");
                let mut detail = done["error"].as_str().unwrap_or_default().to_owned();
                if !self.output.is_empty() {
                    detail = format!("{}\n{detail}", self.output);
                }
                self.failed(
                    "run_failed",
                    format!("“{label}” hata verdi: {exception}: {message}"),
                    (!detail.is_empty()).then_some(detail),
                    false,
                )
            }
            Ended::StoppedBefore => (Err(Failure::Stopped), None),
            Ended::Killed { timed_out: false } => (
                Err(Failure::Stopped),
                self.worked(
                    "run_killed",
                    format!(
                        "Durdur'a {} sn içinde yanıt vermedi; süreci sonlandırıldı, çizim değişmedi.",
                        kentos_python_env::process::KILL_AFTER.as_secs()
                    ),
                ),
            ),
            Ended::Killed { timed_out: true } => self.failed(
                "run_timed_out",
                format!(
                    "“{label}” {} sn içinde bitmedi; süreci sonlandırıldı, çizim değişmedi.",
                    self.timeout
                ),
                None,
                false,
            ),
            Ended::NotStarted(why) => self.failed(
                "runner_not_started",
                format!("“{label}” çalıştırılamadı: {why}"),
                None,
                false,
            ),
            Ended::Crashed(words) => self.failed(
                "runner_crashed",
                format!(
                    "“{label}” çalışırken eklentinin süreci kapandı; çizim değişmedi. {}",
                    words.lines().next().unwrap_or_default()
                ),
                Some(words),
                true,
            ),
        }
    }
}

impl App {
    /// The extensions' tools follow the registry and Python: read from their descriptors, put in İşlemler, the
    /// processes of versions no longer installed closed.
    pub(crate) fn ext_tools_changed(&mut self) {
        let mut tools: Vec<ExternalTool> = Vec::new();
        let mut plans: HashMap<String, Plan> = HashMap::new();
        let mut refusals: Vec<(String, String, String)> = Vec::new();
        if let Some(registry) = self.extensions.registry.as_ref() {
            for c in registry
                .contributions()
                .iter()
                .filter(|c| c.kind == ContributionKind::Processing)
            {
                let ext = c.lease.extension.clone();
                let Some(manifest) = registry.manifest(&ext) else {
                    continue;
                };
                let local =
                    c.id.strip_prefix(&format!("ext.{ext}."))
                        .unwrap_or_default()
                        .to_owned();
                let (Some(entry), Some(source)) = (
                    manifest
                        .entrypoints
                        .processing
                        .iter()
                        .find(|p| p.id == local),
                    registry.package_dir(&ext),
                ) else {
                    continue;
                };
                let read = std::fs::read_to_string(source.join(&entry.descriptor))
                    .map_err(|e| format!("{} okunamadı: {e}", entry.descriptor))
                    .and_then(|text| {
                        serde_json::from_str::<Value>(&text)
                            .map_err(|e| format!("{} JSON değil: {e}", entry.descriptor))
                    })
                    .and_then(|descriptor| {
                        external::from_descriptor(
                            &ext,
                            &manifest.name,
                            &manifest.version.to_string(),
                            &local,
                            &descriptor,
                        )
                    });
                match read {
                    Ok(tool) => {
                        plans.insert(
                            tool.tool.id.clone(),
                            Plan {
                                ext: ext.clone(),
                                local: local.clone(),
                                handler: entry.handler.clone(),

                                version: manifest.version.to_string(),
                                source: source.clone(),
                                digest: registry.digest(&ext).unwrap_or_default(),
                                lock_file: manifest
                                    .python
                                    .as_ref()
                                    .map(|p| p.lock_file.clone())
                                    .unwrap_or_default(),
                            },
                        );
                        tools.push(tool);
                    }
                    Err(why) => refusals.push((ext, local, why)),
                }
            }
        }
        // A descriptor that does not fit: in the extension's log once a session, its tool left out.
        for (ext, local, why) in refusals {
            if self
                .extensions
                .tools
                .said
                .insert(format!("{ext}/{local}/{why}"))
                && let Some(registry) = self.extensions.registry.as_mut()
            {
                registry.write_log(
                    Record::warning(
                        "tool_descriptor",
                        format!("İşlemler aracı “{local}” listelenmedi: {why}"),
                    )
                    .of(&ext, None)
                    .part(format!("processing:{local}")),
                );
            }
        }
        let p = &self.python_setup;
        let python = match (&p.python, &p.root, p.ready_uv()) {
            (Some(crate::python_setup::OwnPython::Ready(interpreter, _)), Some(root), Some(uv)) => {
                Some(Python {
                    interpreter: interpreter.clone(),
                    root: root.clone(),
                    uv,
                })
            }
            _ => None,
        };
        let keys: BTreeSet<String> = plans
            .values()
            .map(|p| format!("{}@{}", p.ext, p.version))
            .collect();
        if let Ok(mut shared) = self.extensions.tools.shared.write() {
            shared.plans = plans;
            shared.python = python;
            shared.why_not = p.why_not().to_owned();
        }
        // A version removed, turned off or replaced: its process closed (one that runs ends with its run).
        for running in self.extensions.tools.pool.running() {
            if !keys.contains(&running.key) {
                self.extensions.tools.pool.close(&running.key);
            }
        }
        let listed: Vec<kentos_processing::Tool> = tools.iter().map(|t| t.tool.clone()).collect();
        external::set_tools(tools);
        self.gis.processing.registry.set_external(listed);
    }

    /// What the extensions' tools' runs noted, written by the window's thread: the log, the notice, the watchdog.
    pub(crate) fn ext_tool_notes(&mut self) {
        let notes = match self.extensions.tools.notes.lock() {
            Ok(mut notes) if !notes.is_empty() => std::mem::take(&mut *notes),
            _ => return,
        };
        for note in notes {
            match note {
                Note::Started { ext, part } => {
                    if let Some(registry) = self.extensions.registry.as_mut() {
                        registry.write_log(
                            Record::info("run_started", "Çalıştırıldı (İşlemler).")
                                .of(&ext, None)
                                .part(part),
                        );
                    }
                }
                Note::Worked {
                    ext,
                    part,
                    code,
                    message,
                } => self.extension_worked(&ext, &part, code, &message),
                Note::Failed {
                    ext,
                    part,
                    title,
                    code,
                    message,
                    detail,
                    crash,
                } => {
                    if crash
                        && let Some(registry) = self.extensions.registry.as_mut()
                        && let Some(lease) = registry.lease(&ext).cloned()
                    {
                        let _ = registry.report_crash(&lease, &message);
                    }
                    self.extension_failed(&ext, &part, code, &message, detail.as_deref());
                    self.failure_notice(&ext, &title, message);
                    if crash {
                        self.extensions_changed();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
