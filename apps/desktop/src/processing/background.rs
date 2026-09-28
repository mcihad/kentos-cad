//! A processing job computed on another thread (the web's worker executor,
//! docs/adr/0124). The job is prepared on the drawing, computed on a reading
//! copy of it there (the objects are shared, not copied), and finished on the
//! drawing again in one undo step. The thread says how far it is and what
//! the tool reports as it goes, as the web's worker posts them.
//!
//! Durdur ends the run at once, as the web's terminates its worker: the
//! tool is asked to stop (those that look stop early), and whatever the
//! thread still says afterwards is dropped.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use iced::Task;
use iced::futures::channel::mpsc;
use kentos_domain::Document;
use kentos_processing::{Feedback, Job, Level, LogLine, RunResult, Runner};

/// Least time between two progress messages (the web's worker's `PROGRESS_MS`).
const PROGRESS_EVERY: Duration = Duration::from_millis(50);

/// What the thread says (the web's worker replies).
#[derive(Clone, Debug)]
pub enum Reply {
    /// The share done, 0..1, and the tool's step label (empty: none).
    Progress(f64, String),
    /// A message of the tool's.
    Line(LogLine),
    /// The tool's answer, to apply on the drawing.
    Done(Arc<RunResult>),
    /// The computation broke off: why.
    Failed(String),
}

/// A run going on another thread.
pub(crate) struct Running {
    pub id: u64,
    pub job: Job,
    /// The drawing it was prepared on (the open drawing's `session`): the
    /// answer applies to no other.
    pub session: u64,
    /// Asks the tool to stop.
    stop: Arc<AtomicBool>,
}

impl Running {
    /// Durdur: the tool is asked to stop; its later words are not waited for.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Starts computing `job` on `copy` (the drawing's reading copy) on a
/// thread of its own; what it says comes back as messages made by `to`.
pub(crate) fn start<M: Send + 'static>(
    id: u64,
    job: Job,
    copy: Document,
    session: u64,
    to: impl Fn(u64, Reply) -> M + Send + 'static,
) -> (Running, Task<M>) {
    let stop = Arc::new(AtomicBool::new(false));
    let (out, replies) = mpsc::unbounded();
    let work = job.clone();
    let mut feedback = Apart {
        out: out.clone(),
        stop: stop.clone(),
        last: None,
    };
    let spawned = std::thread::Builder::new()
        .name("kentos-islem".into())
        .spawn(move || {
            let answer = std::panic::catch_unwind(AssertUnwindSafe(|| {
                Runner::compute(&work, &copy, &mut feedback)
            }));
            let reply = match answer {
                Ok(result) => Reply::Done(Arc::new(result)),
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
        job,
        session,
        stop,
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

/// The tool's feedback on the thread: progress at most every
/// [`PROGRESS_EVERY`] (the last step always), messages as they come.
struct Apart {
    out: mpsc::UnboundedSender<Reply>,
    stop: Arc<AtomicBool>,
    last: Option<Instant>,
}

impl Feedback for Apart {
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
