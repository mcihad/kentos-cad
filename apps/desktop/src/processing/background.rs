//! A processing run on another thread (the web's worker executor,
//! docs/adr/0124, 0125). The drawing's reading copy goes there (the objects
//! are shared, not copied):
//!
//! - a tool's job, prepared on the drawing, is computed on the copy; its
//!   answer is applied on the drawing in one undo step;
//! - a model runs whole on the copy, each step prepared, computed and
//!   applied there and recorded; the recorded steps are applied on the
//!   drawing as they went on the copy, in one undo step.
//!
//! The thread says how far it is and what the tool reports as it goes, as
//! the web's worker posts them. Durdur ends the run at once, as the web's
//! terminates its worker: the tool is asked to stop (those that look stop
//! early), and whatever the thread still says afterwards is dropped.

use std::collections::BTreeMap;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iced::Task;
use iced::futures::channel::mpsc;
use kentos_domain::{Document, Slot};
use kentos_expression::Variable;
use kentos_processing::files::Files;
use kentos_processing::model_runner::{RecordedStep, record_model};
use kentos_processing::{
    Bounds, Feedback, Host, Job, Level, LogLine, Model, RunResult, Runner, Scene, Tool, Values,
};

/// Least time between two progress messages (the web's worker's `PROGRESS_MS`).
const PROGRESS_EVERY: Duration = Duration::from_millis(50);

/// What the thread says (the web's worker replies).
#[derive(Clone, Debug)]
pub enum Reply {
    /// The share done, 0..1, and the tool's step label (empty: none).
    Progress(f64, String),
    /// A message of the tool's.
    Line(LogLine),
    /// The answer is ready ([`Running::answer`]).
    Done,
    /// The computation broke off: why.
    Failed(String),
}

/// What runs.
pub(crate) enum Work {
    /// A tool's job, prepared on the drawing; both boxed, a job and a run are of very different sizes.
    Tool(Box<Job>),
    Model(Box<ModelRun>),
}

impl Work {
    pub fn label(&self) -> String {
        match self {
            Work::Tool(job) => job.tool.label.clone(),
            Work::Model(run) => run.model.label.clone(),
        }
    }
}

/// A model run apart: what the replay on the drawing needs too.
#[derive(Clone)]
pub(crate) struct ModelRun {
    pub model: Model,
    pub inputs: Values,
    /// The steps' tools, by id.
    pub tools: BTreeMap<String, Tool>,
    /// The drawing's generation when it was copied: the recorded steps
    /// apply on it only while it is unchanged.
    pub generation: u64,
}

impl ModelRun {
    /// A step's tool.
    pub fn tool(&self, id: &str) -> Option<Tool> {
        self.tools.get(id).cloned()
    }
}

/// What the thread computed.
pub(crate) enum Answer {
    /// The tool's result, to apply on the drawing.
    Tool(RunResult),
    /// The model's steps as they went on the copy, and their messages.
    Model {
        steps: Vec<RecordedStep>,
        log: Vec<LogLine>,
    },
}

/// A run going on another thread.
pub(crate) struct Running {
    pub id: u64,
    pub work: Work,
    /// The drawing it was started on (the open drawing's `session`): the
    /// answer applies to no other.
    pub session: u64,
    /// Asks the tool to stop.
    stop: Arc<AtomicBool>,
    answer: Arc<Mutex<Option<Answer>>>,
}

impl Running {
    /// Durdur: the tool is asked to stop; its later words are not waited for.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// The answer, once the thread said it is done.
    pub fn answer(&self) -> Option<Answer> {
        self.answer.lock().ok()?.take()
    }
}

/// Starts computing a tool's `job` on `copy` (the drawing's reading copy);
/// what the thread says comes back as messages made by `to`.
pub(crate) fn start_tool<M: Send + 'static>(
    id: u64,
    job: Job,
    copy: Document,
    session: u64,
    files: Option<Arc<dyn Files>>,
    to: impl Fn(u64, Reply) -> M + Send + 'static,
) -> (Running, Task<M>) {
    let work = job.clone();
    start(id, Work::Tool(Box::new(job)), session, files, to, move |feedback| {
        Answer::Tool(Runner::compute(&work, &copy, feedback))
    })
}

/// Starts running a model on `copy` with the selection and the view it
/// was started with; what the thread says comes back as messages made by `to`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn start_model<M: Send + 'static>(
    id: u64,
    run: ModelRun,
    copy: Document,
    selection: Vec<Slot>,
    view: Option<Bounds>,
    session: u64,
    files: Option<Arc<dyn Files>>,
    variables: Vec<Variable>,
    to: impl Fn(u64, Reply) -> M + Send + 'static,
) -> (Running, Task<M>) {
    let there = run.clone();
    start(id, Work::Model(Box::new(run)), session, files, to, move |feedback| {
        let mut host = Copy {
            doc: copy,
            selection,
            view,
            variables,
        };
        let lookup = |id: &str| there.tool(id);
        let mut log = Vec::new();
        let steps = record_model(
            &there.model,
            &there.inputs,
            &mut host,
            &lookup,
            feedback,
            &mut log,
        );
        Answer::Model { steps, log }
    })
}

/// Runs `compute` on a thread of its own, the answer put aside for the
/// message that says it is done.
fn start<M: Send + 'static>(
    id: u64,
    work: Work,
    session: u64,
    files: Option<Arc<dyn Files>>,
    to: impl Fn(u64, Reply) -> M + Send + 'static,
    compute: impl FnOnce(&mut dyn Feedback) -> Answer + Send + 'static,
) -> (Running, Task<M>) {
    let stop = Arc::new(AtomicBool::new(false));
    let answer = Arc::new(Mutex::new(None));
    let (out, replies) = mpsc::unbounded();
    let mut feedback = Apart {
        out: out.clone(),
        stop: stop.clone(),
        last: None,
        files,
    };
    let slot = answer.clone();
    let spawned = std::thread::Builder::new()
        .name("kentos-islem".into())
        .spawn(move || {
            let computed = std::panic::catch_unwind(AssertUnwindSafe(|| compute(&mut feedback)));
            let reply = match computed {
                Ok(done) => {
                    if let Ok(mut slot) = slot.lock() {
                        *slot = Some(done);
                    }
                    Reply::Done
                }
                Err(panic) => Reply::Failed(panic_text(panic.as_ref())),
            };
            let _ = out.unbounded_send(reply);
        });
    let task = match spawned {
        Ok(_) => Task::run(replies, move |reply| to(id, reply)),
        // The web's words when its worker cannot take the job.
        Err(e) => Task::done(to(
            id,
            Reply::Failed(format!("Arka plana gönderilemedi: {e}")),
        )),
    };
    let running = Running {
        id,
        work,
        session,
        stop,
        answer,
    };
    (running, task)
}

/// What a panic said, for the run's error line.
fn panic_text(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "beklenmeyen bir hata oluştu".to_owned())
}

/// The drawing's reading copy as a model's steps change it on the thread.
struct Copy {
    doc: Document,
    selection: Vec<Slot>,
    view: Option<Bounds>,
    /// The `@` values the steps' expressions read (docs/adr/0214 §2.3), taken when the model started.
    variables: Vec<Variable>,
}

impl Scene for Copy {
    fn doc(&self) -> &Document {
        &self.doc
    }

    fn selected(&self) -> Vec<Slot> {
        self.selection.clone()
    }

    fn visible_bounds(&self) -> Option<Bounds> {
        self.view
    }

    fn variables(&self) -> Vec<Variable> {
        self.variables.clone()
    }
}

impl Host for Copy {
    fn doc_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    fn select(&mut self, ids: &[Slot]) {
        self.selection = ids.to_vec();
    }
}

/// The tool's feedback on the thread: progress at most every
/// [`PROGRESS_EVERY`] (the last step always), messages as they come.
struct Apart {
    out: mpsc::UnboundedSender<Reply>,
    stop: Arc<AtomicBool>,
    last: Option<Instant>,
    /// The host's files for the tools that read and write them (docs/adr/0207 §7).
    files: Option<Arc<dyn Files>>,
}

impl Feedback for Apart {
    fn files(&self) -> Option<Arc<dyn Files>> {
        self.files.clone()
    }

    fn progress(&mut self, fraction: f64, label: &str) {
        let now = Instant::now();
        if fraction < 1.0 && self.last.is_some_and(|t| now - t < PROGRESS_EVERY) {
            return;
        }
        self.last = Some(now);
        let _ = self
            .out
            .unbounded_send(Reply::Progress(fraction, label.to_owned()));
    }

    fn info(&mut self, message: String) {
        let _ = self.out.unbounded_send(Reply::Line(LogLine {
            level: Level::Info,
            text: message,
        }));
    }

    fn warn(&mut self, message: String) {
        let _ = self.out.unbounded_send(Reply::Line(LogLine::warn(message)));
    }

    fn canceled(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}
