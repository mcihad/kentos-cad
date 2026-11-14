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
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// The extensions' expression functions, by their whole names (`ext.<eklenti>.<işlev>`).
    expressions: HashMap<String, Plan>,
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
    /// The labels' answers to the extensions' functions: never waiting (drawing slice).
    pub(crate) labels: Arc<LabelAnswers>,
    /// The functions offered and Python's state the labels were last worked out with.
    label_readiness: String,
    /// The descriptors' refusals already written to a log this session.
    said: BTreeSet<String>,
    /// The process's lists (İşlemler' tools and functions, what runs and answers them) are this window's: given once
    /// it offered a tool or a function, not before.
    registered: bool,
}

impl Default for ExtTools {
    fn default() -> Self {
        let shared: Arc<RwLock<Shared>> = Arc::default();
        let pool = Arc::new(Pool::default());
        let notes: Arc<Mutex<Vec<Note>>> = Arc::default();
        let answers = Arc::new(Answers {
            shared: Arc::clone(&shared),
            pool: Arc::clone(&pool),
            notes: Arc::clone(&notes),
            cache: Mutex::default(),
            told: Mutex::default(),
        });
        let labels = Arc::new(LabelAnswers {
            answers,
            queue: Mutex::default(),
            busy: AtomicBool::new(false),
            fresh: AtomicBool::new(false),
        });
        ExtTools {
            shared,
            pool,
            notes,
            labels,
            label_readiness: String::new(),
            said: BTreeSet::new(),
            registered: false,
        }
    }
}

/// Tests run many windows in one process, and the lists a window gives the process (İşlemler' tools and functions,
/// what runs and answers them, the expression engine's host) are the process's: a test whose window gives them keeps
/// them until it ends, so another test's window does not replace them under it (it failed now and then with
/// "Bilinmeyen işlev: ext.tr.ornek.imar-denetim.ada-parsel()"). A window that offers none, most tests', is not held.
#[cfg(test)]
fn hold_process_lists() {
    use std::sync::{MutexGuard, PoisonError};
    static LISTS: Mutex<()> = Mutex::new(());
    thread_local! {
        static HELD: RefCell<Option<MutexGuard<'static, ()>>> = const { RefCell::new(None) };
    }
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        if held.is_none() {
            *held = Some(LISTS.lock().unwrap_or_else(PoisonError::into_inner));
        }
    });
}

/// How long a batch of an expression function's calls may take.
const EXPRESSION_TIMEOUT: Duration = Duration::from_secs(60);

/// The most answers kept; past it the store starts again.
const CACHED_ANSWERS: usize = 100_000;

/// Answers the extensions' expression functions for İşlemler (`ext.<eklenti>.<işlev>(…)`): a batch's distinct rows
/// asked once of the extension version's process, answers kept by their arguments (an expression run again, or the
/// same values on many objects, asks nothing); a failure said once in the extension's log, its rows empty.
struct Answers {
    shared: Arc<RwLock<Shared>>,
    pool: Arc<Pool>,
    notes: Arc<Mutex<Vec<Note>>>,
    cache: Mutex<HashMap<String, kentos_expression::external::Arg>>,
    /// The failures already said, by function and words.
    told: Mutex<BTreeSet<String>>,
}

fn arg_json(a: &kentos_expression::external::Arg) -> Value {
    use kentos_expression::external::Arg;
    match a {
        Arg::Null => Value::Null,
        Arg::Num(x) => json!(x),
        Arg::Text(t) => json!(t),
        Arg::Bool(b) => json!(b),
    }
}

fn json_arg(v: &Value) -> kentos_expression::external::Arg {
    use kentos_expression::external::Arg;
    match v {
        Value::Null => Arg::Null,
        Value::Bool(b) => Arg::Bool(*b),
        Value::Number(n) => n.as_f64().map_or(Arg::Null, Arg::Num),
        Value::String(s) => Arg::Text(s.clone()),
        other => Arg::Text(other.to_string()),
    }
}

impl Answers {
    fn tell(&self, plan: &Plan, name: &str, code: &'static str, message: String) {
        let first = self
            .told
            .lock()
            .is_ok_and(|mut told| told.insert(format!("{name}/{code}/{message}")));
        if first && let Ok(mut notes) = self.notes.lock() {
            notes.push(Note::Failed {
                ext: plan.ext.clone(),
                part: format!("expression:{}", plan.local),
                title: name.to_owned(),
                code,
                message,
                detail: None,
                crash: false,
            });
        }
    }
}

impl Answers {
    fn key(name: &str, row: &[kentos_expression::external::Arg]) -> String {
        format!(
            "{name}\u{1}{}",
            Value::Array(row.iter().map(arg_json).collect())
        )
    }

    /// The answers already kept, by row; none where a row was never asked.
    fn kept(
        &self,
        name: &str,
        rows: &[Vec<kentos_expression::external::Arg>],
    ) -> Vec<Option<kentos_expression::external::Arg>> {
        let cache = self.cache.lock().ok();
        rows.iter()
            .map(|r| {
                cache
                    .as_ref()
                    .and_then(|c| c.get(&Self::key(name, r)).cloned())
            })
            .collect()
    }
}

/// A label's answers (docs/adr/8057 §8): what `Answers` keeps, at once; a row not answered yet is empty and goes to a
/// queue a background thread asks of the extension's process, after which the labels calling it are worked out again
/// (`label_texts::answers_changed`). A failed row is kept empty: it is not asked again and again.
pub(crate) struct LabelAnswers {
    answers: Arc<Answers>,
    queue: Mutex<Vec<(String, Vec<kentos_expression::external::Arg>)>>,
    /// A thread is asking.
    busy: AtomicBool,
    /// Answers came since the window last looked.
    fresh: AtomicBool,
}

impl LabelAnswers {
    /// Whether the window should look again soon: a thread asks, or answers came.
    pub(crate) fn waiting(&self) -> bool {
        self.busy.load(Ordering::Relaxed) || self.fresh.load(Ordering::Relaxed)
    }

    /// Answers came since the last look (and it is now looked at).
    pub(crate) fn take_fresh(&self) -> bool {
        self.fresh.swap(false, Ordering::Relaxed)
    }

    fn ask_in_background(self: &Arc<Self>) {
        if self.busy.swap(true, Ordering::Relaxed) {
            return;
        }
        let me = Arc::clone(self);
        let _ = std::thread::Builder::new()
            .name("kentos-ext-etiket".into())
            .spawn(move || {
                loop {
                    let batch: Vec<(String, Vec<kentos_expression::external::Arg>)> = me
                        .queue
                        .lock()
                        .map(|mut q| std::mem::take(&mut *q))
                        .unwrap_or_default();
                    if batch.is_empty() {
                        me.busy.store(false, Ordering::Relaxed);
                        // One more look: a row queued while the flag was going down is not left.
                        if me.queue.lock().is_ok_and(|q| q.is_empty())
                            || me.busy.swap(true, Ordering::Relaxed)
                        {
                            return;
                        }
                        continue;
                    }
                    let mut names: Vec<String> = batch.iter().map(|(n, _)| n.clone()).collect();
                    names.sort();
                    names.dedup();
                    let mut kept = 0usize;
                    for name in names {
                        let rows: Vec<Vec<kentos_expression::external::Arg>> = batch
                            .iter()
                            .filter(|(n, _)| *n == name)
                            .map(|(_, r)| r.clone())
                            .collect();
                        kept += me.answers.answer(&name, &rows).1;
                    }
                    // Only answers that were kept change a label: rows not kept (not started yet) are asked again when
                    // the extension is ready, never in a loop.
                    if kept > 0 {
                        kentos_native_application::label_texts::answers_changed();
                        me.fresh.store(true, Ordering::Relaxed);
                    }
                }
            });
    }
}

/// The labels' calls: never waiting. `Arc<LabelAnswers>` answers them (its background thread needs the `Arc`).
pub(crate) struct LabelCalls(pub(crate) Arc<LabelAnswers>);

impl kentos_expression::external::ExternalCalls for LabelCalls {
    fn call(
        &self,
        name: &str,
        rows: &[Vec<kentos_expression::external::Arg>],
    ) -> Vec<kentos_expression::external::Arg> {
        let kept = self.0.answers.kept(name, rows);
        // Not askable yet (its extension off, no Python): empty, not queued; the labels are worked out again when it
        // becomes ready (ext_tools_changed).
        if !self.0.answers.ready_for(name) {
            return kept
                .into_iter()
                .map(|a| a.unwrap_or(kentos_expression::external::Arg::Null))
                .collect();
        }
        let missing: Vec<(String, Vec<kentos_expression::external::Arg>)> = kept
            .iter()
            .zip(rows)
            .filter(|(k, _)| k.is_none())
            .map(|(_, r)| (name.to_owned(), r.clone()))
            .collect();
        if !missing.is_empty() {
            if let Ok(mut queue) = self.0.queue.lock() {
                for m in missing {
                    if !queue.contains(&m) {
                        queue.push(m);
                    }
                }
            }
            self.0.ask_in_background();
        }
        kept.into_iter()
            .map(|a| a.unwrap_or(kentos_expression::external::Arg::Null))
            .collect()
    }
}

/// While a label's answers are asked or came: the window looks again every 300 ms (Toplu çalıştır's way).
pub(crate) fn label_ticks() -> impl iced::futures::Stream<Item = crate::app::Message> {
    use iced::futures::SinkExt;
    let (mut out, ticks) = iced::futures::channel::mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(300));
            let tick = crate::app::Message::Extensions(super::Event::LabelAnswers);
            if iced::futures::executor::block_on(out.send(tick)).is_err() {
                break;
            }
        }
    });
    ticks
}

impl kentos_expression::external::ExternalCalls for Answers {
    fn call(
        &self,
        name: &str,
        rows: &[Vec<kentos_expression::external::Arg>],
    ) -> Vec<kentos_expression::external::Arg> {
        self.answer(name, rows).0
    }
}

impl Answers {
    /// Whether `name` can be asked now: its extension installed and on, Python ready.
    fn ready_for(&self, name: &str) -> bool {
        self.shared
            .read()
            .is_ok_and(|s| s.python.is_some() && s.expressions.contains_key(name))
    }

    /// `name`'s answers for `rows`, and how many answers were newly kept. An answer is kept only when the extension's
    /// process answered (a failed row too, so it is not asked again and again); without a ready Python or plan, or
    /// when the process did not start, nothing is kept: the rows are asked again once it is ready.
    fn answer(
        &self,
        name: &str,
        rows: &[Vec<kentos_expression::external::Arg>],
    ) -> (Vec<kentos_expression::external::Arg>, usize) {
        use kentos_expression::external::Arg;
        let keys: Vec<String> = rows
            .iter()
            .map(|r| {
                format!(
                    "{name}\u{1}{}",
                    Value::Array(r.iter().map(arg_json).collect())
                )
            })
            .collect();
        let mut answers: Vec<Option<Arg>> = {
            let cache = self.cache.lock().ok();
            keys.iter()
                .map(|k| cache.as_ref().and_then(|c| c.get(k).cloned()))
                .collect()
        };
        // The distinct rows not answered yet, each asked once.
        let mut missing: Vec<usize> = Vec::new();
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for (i, k) in keys.iter().enumerate() {
            if answers[i].is_none() && !seen.contains_key(k.as_str()) {
                seen.insert(k.as_str(), missing.len());
                missing.push(i);
            }
        }
        let mut newly = 0usize;
        if !missing.is_empty() {
            let (plan, python) = match self.shared.read() {
                Ok(shared) => (shared.expressions.get(name).cloned(), shared.python.clone()),
                Err(_) => (None, None),
            };
            // Whether the process answered (only then are the answers kept).
            let mut kept_now = false;
            let got: Vec<Arg> = match (plan, python) {
                (Some(plan), Some(python)) => {
                    let invoke = json!({
                        "kind": "expression",
                        "local": plan.local,
                        "handler": plan.handler,
                        "rows": missing.iter().map(|&i| Value::Array(rows[i].iter().map(arg_json).collect())).collect::<Vec<_>>(),
                        "document": false,
                    });
                    let key = format!("{}@{}", plan.ext, plan.version);
                    match self.pool.run(
                        &key,
                        &|| start_of(&plan, &python),
                        invoke,
                        EXPRESSION_TIMEOUT,
                        &|| false,
                        &mut |_| {},
                    ) {
                        Ended::Answered(done) if done["ok"] == true => {
                            kept_now = true;
                            let result = &done["result"];
                            if let Some(failed) = result["failed"].as_u64() {
                                self.tell(
                                    &plan,
                                    name,
                                    "expression_failed",
                                    format!(
                                        "{name}() {failed} satırda hata verdi, o satırlar boş kaldı: {}",
                                        result["error"].as_str().unwrap_or_default()
                                    ),
                                );
                            }
                            result["values"]
                                .as_array()
                                .map(|v| v.iter().map(json_arg).collect())
                                .unwrap_or_default()
                        }
                        Ended::Answered(done) => {
                            kept_now = true;
                            let words = done["message"].as_str().unwrap_or_default().to_owned();
                            self.tell(
                                &plan,
                                name,
                                "expression_failed",
                                format!("{name}() çalışmadı: {words}"),
                            );
                            Vec::new()
                        }
                        // Ended or overdue: kept empty, so a hanging function is not asked again and again; not
                        // started (no environment yet): not kept, asked again once it is ready.
                        Ended::NotStarted(why) => {
                            self.tell(
                                &plan,
                                name,
                                "expression_failed",
                                format!("{name}() çalıştırılamadı: {why}"),
                            );
                            Vec::new()
                        }
                        other => {
                            kept_now = true;
                            self.tell(
                                &plan,
                                name,
                                "expression_failed",
                                format!("{name}() yanıt vermedi: {other:?}"),
                            );
                            Vec::new()
                        }
                    }
                }
                _ => Vec::new(),
            };
            let mut got = got.into_iter();
            let mut asked: Vec<(String, Arg)> = Vec::with_capacity(missing.len());
            for &i in &missing {
                let a = got.next().unwrap_or(Arg::Null);
                asked.push((keys[i].clone(), a));
            }
            for (i, k) in keys.iter().enumerate() {
                if answers[i].is_none()
                    && let Some(&j) = seen.get(k.as_str())
                {
                    answers[i] = Some(asked[j].1.clone());
                }
            }
            if kept_now && let Ok(mut cache) = self.cache.lock() {
                if cache.len() + asked.len() > CACHED_ANSWERS {
                    cache.clear();
                }
                newly = asked.len();
                cache.extend(asked);
            }
        }
        let answers = answers
            .into_iter()
            .map(|a| a.unwrap_or(Arg::Null))
            .collect();
        (answers, newly)
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
                    Ok(mut tool) => {
                        // The package's digest: what a model's step pins (docs/adr/8057 §13).
                        tool.digest = registry.digest(&ext).unwrap_or_default();
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
        // The extensions' expression functions (`ext.<eklenti>.<işlev>`), known by name without their code.
        let mut expressions: HashMap<String, Plan> = HashMap::new();
        let mut defs: Vec<kentos_expression::external::ExternalDef> = Vec::new();
        if let Some(registry) = self.extensions.registry.as_ref() {
            for c in registry
                .contributions()
                .iter()
                .filter(|c| c.kind == ContributionKind::Expression)
            {
                let ext = c.lease.extension.clone();
                let Some(manifest) = registry.manifest(&ext) else {
                    continue;
                };
                let local =
                    c.id.strip_prefix(&format!("ext.{ext}."))
                        .unwrap_or_default();
                let (Some(entry), Some(source)) = (
                    manifest
                        .entrypoints
                        .expressions
                        .iter()
                        .find(|e| e.id == local),
                    registry.package_dir(&ext),
                ) else {
                    continue;
                };
                defs.push(kentos_expression::external::ExternalDef {
                    name: c.id.clone(),
                    description: format!(
                        "{} — {} eklentisinin ifade işlevi; kendi Python ortamında çalışır.",
                        entry.title.as_deref().unwrap_or(local),
                        manifest.name
                    ),
                });
                expressions.insert(
                    c.id.clone(),
                    Plan {
                        ext: ext.clone(),
                        local: local.to_owned(),
                        handler: entry.handler.clone(),
                        version: manifest.version.to_string(),
                        source,
                        digest: registry.digest(&ext).unwrap_or_default(),
                        lock_file: manifest
                            .python
                            .as_ref()
                            .map(|p| p.lock_file.clone())
                            .unwrap_or_default(),
                    },
                );
            }
        }
        // The process's lists are given once a tool or a function is offered, and kept up to date after; a window
        // that never offered one leaves them alone (in tests many windows share the process, `hold_process_lists`).
        let global = self.extensions.tools.registered || !tools.is_empty() || !defs.is_empty();
        if global && !self.extensions.tools.registered {
            #[cfg(test)]
            hold_process_lists();
            let t = &self.extensions.tools;
            external::register_provider(Arc::new(Bridge {
                shared: Arc::clone(&t.shared),
                pool: Arc::clone(&t.pool),
                notes: Arc::clone(&t.notes),
            }));
            external::register_expression_calls(Arc::<Answers>::clone(&t.labels.answers));
            self.extensions.tools.registered = true;
        }
        if global {
            // Labels know them too, answered without waiting (drawing slice); a label compiled before a function
            // was offered, or asked before Python was ready, is worked out again.
            let readiness = format!(
                "{:?}|{}",
                defs.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
                self.python_setup.why_not()
            );
            if self.extensions.tools.label_readiness != readiness {
                self.extensions.tools.label_readiness = readiness;
                kentos_native_application::label_texts::answers_changed();
            }
            // One registry for whatever works an expression out while drawing: labels, styles, filters, forms
            // (`kentos_expression::external::set_host`, docs/adr/8057 §8).
            kentos_native_application::label_texts::set_externals(
                defs.clone(),
                Arc::new(LabelCalls(Arc::clone(&self.extensions.tools.labels))),
            );
            external::set_expressions(defs);
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
            shared.expressions = expressions;
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
        if global {
            external::set_tools(tools);
        }
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
