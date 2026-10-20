//! Commands as jobs (docs/adr/0253 §9): a command computed on a reading
//! copy of the document on another thread, so the document is never held
//! while it computes; its result is staged, then written by the host's own
//! thread in a short commit.
//!
//! The phases: queued → computing → staged → committing → done (or failed,
//! conflict, cancelled). A stop asked while computing stops a command that
//! asks (`ExecutionContext::stopped`) and drops the result of one that does
//! not; asked once staged, the result is dropped; asked once committing or
//! done, it changes nothing (`already_committed`). The commit writes the
//! copy's step as it is when the document did not change since the job
//! began; when it did, by the objects' persistent ids if every object the
//! step changes or removes is still as the job saw it, new objects in new
//! slots; otherwise `document_changed`, nothing written.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use kentos_commands::{
    CallAnswer, CallEnvelope, CallOp, CommandError, CommandResult, JobPhase, JobStatus, codes,
};
use kentos_domain::Document;

/// A job's progress and its stop, shared with the thread that computes it.
#[derive(Debug, Default)]
pub struct Progress {
    stop: AtomicBool,
    done: AtomicU64,
    total: AtomicU64,
}

impl Progress {
    /// Whether the job was asked to stop.
    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    /// Asks the job to stop.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// `done` of `total`.
    pub fn set(&self, done: u64, total: u64) {
        self.total.store(total, Ordering::Relaxed);
        self.done.store(done.min(total), Ordering::Relaxed);
    }

    /// How far, 0 to 1; none before the command said.
    pub fn fraction(&self) -> Option<f64> {
        let total = self.total.load(Ordering::Relaxed);
        (total > 0).then(|| self.done.load(Ordering::Relaxed) as f64 / total as f64)
    }
}

/// What a job's thread hands back.
pub struct JobOutcome {
    pub answer: CallAnswer,
    /// The copy after a document command's write, to be written to the
    /// document; none for a command that writes no document (a file's).
    pub copy: Option<Document>,
    /// A command that wrote a file: its journal row to record on the
    /// document (`call::record_effect`).
    pub effect: Option<Effect>,
}

/// A file command's write, for the journal.
pub struct Effect {
    pub command: String,
    pub version: u32,
    pub input: serde_json::Value,
    pub effects: Vec<kentos_commands::FileEffect>,
    pub state: crate::call::CallState,
}

/// A command running as a job.
pub struct Job {
    id: String,
    command: String,
    progress: Arc<Progress>,
    phase: Arc<Mutex<JobPhase>>,
    thread: Option<JoinHandle<JobOutcome>>,
    answer: Option<CallAnswer>,
    /// The document's revision and generation when the job began.
    base: (u64, u64),
    request_id: Option<String>,
}

fn phase_of(phase: &Mutex<JobPhase>) -> JobPhase {
    phase.lock().map_or(JobPhase::Failed, |p| *p)
}

fn set_phase(phase: &Mutex<JobPhase>, to: JobPhase) {
    if let Ok(mut p) = phase.lock() {
        *p = to;
    }
}

fn refused(code: &str, message: impl Into<String>) -> CallAnswer {
    CallAnswer::of(CommandResult::Failed {
        error: CommandError {
            code: code.to_owned(),
            message: message.into(),
            ..CommandError::default()
        },
    })
}

impl Job {
    /// Starts `envelope` (its `execute`) as job `id` on a reading copy of
    /// `doc`; the document is free again at once.
    pub fn start(id: String, doc: &Document, envelope: CallEnvelope) -> Job {
        let command = envelope.command.clone();
        let request_id = envelope.request_id.clone();
        Job::start_with(id, command, request_id, doc, move |mut copy, progress| {
            let envelope = CallEnvelope {
                op: CallOp::Execute,
                ..envelope
            };
            let answer = crate::call::call_in_job(&mut copy, envelope, progress);
            JobOutcome {
                answer,
                copy: Some(copy),
                effect: None,
            }
        })
    }

    /// Starts `work` as job `id` for `command`, on a reading copy of `doc`
    /// on another thread: a host's own command (a file's) runs this way.
    pub fn start_with(
        id: String,
        command: String,
        request_id: Option<String>,
        doc: &Document,
        work: impl FnOnce(Document, &Progress) -> JobOutcome + Send + 'static,
    ) -> Job {
        let progress = Arc::new(Progress::default());
        let phase = Arc::new(Mutex::new(JobPhase::Queued));
        let base = (doc.revision(), doc.generation());
        let copy = doc.reading_copy();
        let (p, ph) = (Arc::clone(&progress), Arc::clone(&phase));
        let thread = std::thread::Builder::new()
            .name(format!("kentos-job-{id}"))
            .spawn(move || {
                set_phase(&ph, JobPhase::Computing);
                let out = work(copy, &p);
                // Ready, not written: the host writes it when it looks.
                if matches!(out.answer.result, CommandResult::Completed { .. }) {
                    set_phase(&ph, JobPhase::Staged);
                }
                out
            })
            .ok();
        let answer = thread.is_none().then(|| {
            set_phase(&phase, JobPhase::Failed);
            refused(
                codes::INVALID_CALL,
                "İş için iş parçacığı açılamadı; komutu iş olarak değil, doğrudan çağırın.",
            )
        });
        Job {
            id,
            command,
            progress,
            phase,
            thread,
            answer,
            base,
            request_id,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// Asks the job to stop. Nothing is written by a stopped job; one already
    /// written stays (`already_committed`, its answer says so).
    pub fn cancel(&mut self) {
        self.progress.stop();
    }

    /// The job as it is now, without looking at its document (its result is
    /// written when `status` or `wait` is asked).
    pub fn peek(&self) -> JobStatus {
        JobStatus {
            id: self.id.clone(),
            command: self.command.clone(),
            phase: phase_of(&self.phase),
            progress: self.progress.fraction(),
            answer: self.answer.clone(),
        }
    }

    /// The job as it is now; when its computation finished, its result is
    /// written to `doc` (the document it was started from) first.
    pub fn status(&mut self, doc: &mut Document) -> JobStatus {
        self.advance(doc);
        JobStatus {
            id: self.id.clone(),
            command: self.command.clone(),
            phase: phase_of(&self.phase),
            progress: self.progress.fraction(),
            answer: self.answer.clone(),
        }
    }

    /// Waits for the computation, writes it to `doc` and answers.
    pub fn wait(&mut self, doc: &mut Document) -> JobStatus {
        if let Some(thread) = self.thread.take() {
            let computed = thread.join();
            self.stage(doc, computed.ok());
        }
        self.status(doc)
    }

    fn advance(&mut self, doc: &mut Document) {
        if self
            .thread
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
            && let Some(thread) = self.thread.take()
        {
            let computed = thread.join();
            self.stage(doc, computed.ok());
        }
    }

    /// The computation is in: drop it when stopped or refused, otherwise commit it.
    fn stage(&mut self, doc: &mut Document, computed: Option<JobOutcome>) {
        let Some(JobOutcome {
            answer,
            copy,
            effect,
        }) = computed
        else {
            set_phase(&self.phase, JobPhase::Failed);
            self.answer = Some(refused(
                codes::INVALID_CALL,
                "İşin hesabı yarıda kaldı; hiçbir şey yazılmadı.",
            ));
            return;
        };
        if !matches!(answer.result, CommandResult::Completed { .. }) {
            let phase = match answer.result {
                CommandResult::Conflict { .. } => JobPhase::Conflict,
                CommandResult::Cancelled => JobPhase::Cancelled,
                _ => JobPhase::Failed,
            };
            set_phase(&self.phase, phase);
            self.answer = Some(answer);
            return;
        }
        set_phase(&self.phase, JobPhase::Staged);
        // A file is written by the time it is staged: a stop no longer takes it back.
        if self.progress.stopped() && effect.is_none() {
            set_phase(&self.phase, JobPhase::Cancelled);
            self.answer = Some(CallAnswer::of(CommandResult::Cancelled));
            return;
        }
        set_phase(&self.phase, JobPhase::Committing);
        let committed = match (copy, effect) {
            (Some(copy), _) => {
                crate::call::commit_job(doc, copy, answer, self.base, self.request_id.as_deref())
            }
            (None, Some(effect)) => {
                let mut answer = answer;
                let call = crate::call::record_effect(
                    doc,
                    &effect.command,
                    effect.version,
                    &effect.input,
                    effect.effects,
                    &effect.state,
                );
                answer.call = Some(call);
                if self.request_id.is_some()
                    && let Ok(text) = serde_json::to_string(&answer)
                {
                    doc.set_request_answer(call, text);
                }
                answer
            }
            (None, None) => answer,
        };
        let phase = match committed.result {
            CommandResult::Completed { .. } => JobPhase::Done,
            CommandResult::Conflict { .. } => JobPhase::Conflict,
            CommandResult::Cancelled => JobPhase::Cancelled,
            _ => JobPhase::Failed,
        };
        set_phase(&self.phase, phase);
        self.answer = Some(committed);
    }

    /// The answer a stop gets: none to wait for while it runs; once written, that it was.
    pub fn stop_answer(&self) -> Option<CallAnswer> {
        matches!(phase_of(&self.phase), JobPhase::Done).then(|| {
            refused(
                codes::ALREADY_COMMITTED,
                "İş yazıldıktan sonra durdurulmak istendi; yazılan kalır. Geri almak için Geri al'ı kullanın.",
            )
        })
    }
}
