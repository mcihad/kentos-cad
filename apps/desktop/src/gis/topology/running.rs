//! A topology check on its own thread (docs/adr/0756): the drawing's reading copy (its objects shared, not copied) is
//! gathered and checked there, and what the thread says comes back as messages. The tab's bar follows it each frame,
//! easing towards the share said and, through a stretch the check cannot measure (Boşluk olmamalı's overlay of a whole
//! layer), moving on by the time that went by against the time it is expected to take, never to its end. Durdur ends
//! the check at once: the thread is asked to stop, and what it says afterwards is dropped. A check of a drawing with
//! few objects in the rules' layers is made at once, as before (`compute`).

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iced::Task;
use iced::futures::channel::mpsc;
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::topology_rules::{self as core, Watch, incremental};
use kentos_native_application::topology_check::{self, Gathered};

use super::{Checked, Then};

/// Objects in the rules' layers from which a check goes to its own thread: fewer are checked sooner than the thread
/// and the bar's first frame (the processing runner's `WORKER_THRESHOLD`).
pub(crate) const THREAD_FROM: usize = 2000;

/// Least time between two progress messages.
const PROGRESS_EVERY: Duration = Duration::from_millis(50);

/// The share of the bar the gathering takes, before the rules run.
const GATHER: f64 = 0.04;

/// What is checked.
#[derive(Clone)]
pub(crate) enum Work {
    /// Every rule over every object.
    Check,
    /// What changed since the last check (docs/adr/0524); every object when its rules, its tolerance or its
    /// exceptions changed.
    Recheck(Box<Checked>),
}

/// What a check found.
pub(crate) enum Ran {
    Checked {
        g: Gathered,
        found: core::Checked,
    },
    /// Checked again over what changed: how many objects did.
    Rechecked {
        g: Gathered,
        found: core::Checked,
        changed: usize,
    },
}

/// What the thread says.
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    /// The share done, 0..1.
    Progress(f64),
    /// A stretch not measured, from a share to another, expected to take about this many seconds.
    Stretch(f64, f64, f64),
    /// The answer is ready ([`Running::answer`]).
    Done,
    /// The check broke off: why.
    Failed(String),
}

/// The check `work` asks for on `doc` (the drawing, or its reading copy on the thread), watched; none when stopped.
pub(crate) fn compute(work: &Work, doc: &Document, watch: &dyn Watch) -> Option<Ran> {
    let g = topology_check::gather(doc, None);
    let old = match work {
        Work::Recheck(old) if old.g.stamp.rules == g.stamp.rules && old.g.lapsed == g.lapsed => old,
        _ => {
            let found = topology_check::run_watched(&g, watch)?;
            return Some(Ran::Checked { g, found });
        }
    };
    // What changed: an object added, removed, moved or put on another layer; where it was and is.
    let was: HashMap<&str, (&Shape, &str)> = old
        .g
        .uids
        .iter()
        .zip(old.g.shapes.iter().zip(&old.g.layers))
        .map(|(u, (s, l))| (u.as_str(), (s, l.as_str())))
        .collect();
    let mut changed: HashSet<&str> = HashSet::new();
    let mut touched = Vec::new();
    for (i, u) in g.uids.iter().enumerate() {
        match was.get(u.as_str()) {
            Some((s, l)) if **s == g.shapes[i] && *l == g.layers[i] => {}
            Some((s, _)) => {
                changed.insert(u);
                touched.push(incremental::shape_bounds(s));
                touched.push(incremental::shape_bounds(&g.shapes[i]));
            }
            None => {
                changed.insert(u);
                touched.push(incremental::shape_bounds(&g.shapes[i]));
            }
        }
    }
    let now: HashSet<&str> = g.uids.iter().map(String::as_str).collect();
    for (u, (s, _)) in &was {
        if !now.contains(u) {
            changed.insert(u);
            touched.push(incremental::shape_bounds(s));
        }
    }
    let n_changed = changed.len();
    let mut found = incremental::recheck_watched(
        &g.objects(),
        &g.core_rules,
        g.tolerance,
        &g.exceptions,
        &incremental::Previous {
            findings: &old.findings,
            uids: &old.g.uids,
        },
        &changed,
        &touched,
        watch,
    )?;
    // A part's rows alone are not its findings (docs/adr/0589 §1).
    found
        .findings
        .retain(|f| f.objects.is_empty() || f.objects.iter().any(|&k| !g.outside(k)));
    Some(Ran::Rechecked {
        g,
        found,
        changed: n_changed,
    })
}

/// A check going on its own thread.
pub(crate) struct Running {
    pub id: u64,
    /// The drawing (its session) and the state of it the check saw.
    pub session: u64,
    pub generation: u64,
    pub revision: u64,
    /// What follows the answer.
    pub then: Then,
    stop: Arc<AtomicBool>,
    answer: Arc<Mutex<Option<Ran>>>,
    /// What the thread said last, and the stretch it said it is in: its end, when it began, how long it is expected
    /// to take.
    said: f64,
    stretch: Option<(f64, Instant, f64)>,
    /// What the bar shows, and when it last moved.
    pub shown: f64,
    moved: Instant,
}

impl Running {
    /// Durdur: the thread is asked to stop; its later words are not waited for.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// The answer, once the thread said it is done.
    pub fn answer(&self) -> Option<Ran> {
        self.answer.lock().ok()?.take()
    }

    /// What the thread said of its progress.
    pub fn heard(&mut self, reply: &Reply) {
        match *reply {
            Reply::Progress(done) => {
                self.said = js_max(self.said, done);
                self.stretch = None;
            }
            Reply::Stretch(from, until, seconds) => {
                self.said = js_max(self.said, from);
                self.stretch = Some((until, Instant::now(), seconds));
            }
            Reply::Done | Reply::Failed(_) => {}
        }
    }

    /// The bar's next frame at `now`: towards what was said, or within a stretch as far on as the time gone by takes
    /// it (63 % of the way at the time expected, 95 % at most); eased, never back.
    pub fn frame(&mut self, now: Instant) {
        let mut target = self.said;
        if let Some((until, began, seconds)) = self.stretch {
            let gone = now.saturating_duration_since(began).as_secs_f64();
            let k = 1.0 - (-gone / seconds.max(0.05)).exp();
            target = self.said + (until - self.said) * 0.95 * k;
        }
        let dt = now
            .saturating_duration_since(self.moved)
            .as_secs_f64()
            .min(0.25);
        self.moved = now;
        let a = 1.0 - (-dt / 0.12).exp();
        self.shown = js_max(self.shown, self.shown + (target - self.shown) * a).min(1.0);
    }
}

fn js_max(a: f64, b: f64) -> f64 {
    if a >= b { a } else { b }
}

/// Starts `work` on `copy` (the drawing's reading copy) on a thread of its own; what the thread says comes back as
/// messages made by `to`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn start<M: Send + 'static>(
    id: u64,
    work: Work,
    copy: Document,
    objects: usize,
    then: Then,
    (session, generation, revision): (u64, u64, u64),
    to: impl Fn(u64, Reply) -> M + Send + 'static,
) -> (Running, Task<M>) {
    let stop = Arc::new(AtomicBool::new(false));
    let answer = Arc::new(Mutex::new(None));
    let (out, replies) = mpsc::unbounded();
    let watch = Apart {
        out: out.clone(),
        stop: stop.clone(),
        last: Cell::new(None),
    };
    let slot = answer.clone();
    let spawned = std::thread::Builder::new()
        .name("kentos-topoloji".into())
        .spawn(move || {
            let computed = std::panic::catch_unwind(AssertUnwindSafe(|| {
                // Gathering copies the rules' layers' objects (about 0.3 µs each) and reads the rows round bound parts.
                watch.stretch(0.0, GATHER, objects as f64 * 3e-7);
                compute(&work, &copy, &After(&watch))
            }));
            let reply = match computed {
                Ok(done) => {
                    if let Ok(mut slot) = slot.lock() {
                        *slot = done;
                    }
                    Reply::Done
                }
                Err(panic) => Reply::Failed(panic_text(panic.as_ref())),
            };
            let _ = out.unbounded_send(reply);
        });
    let task = match spawned {
        Ok(_) => Task::run(replies, move |reply| to(id, reply)),
        Err(e) => Task::done(to(
            id,
            Reply::Failed(format!("Denetim arka plana gönderilemedi: {e}")),
        )),
    };
    let now = Instant::now();
    let running = Running {
        id,
        session,
        generation,
        revision,
        then,
        stop,
        answer,
        said: 0.0,
        stretch: None,
        shown: 0.0,
        moved: now,
    };
    (running, task)
}

/// What a panic said.
fn panic_text(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "beklenmeyen bir hata oluştu".to_owned())
}

/// The check's watch on the thread: its progress at most every [`PROGRESS_EVERY`] (the end always), its stretches as
/// they come, the stop Durdur sets.
struct Apart {
    out: mpsc::UnboundedSender<Reply>,
    stop: Arc<AtomicBool>,
    last: Cell<Option<Instant>>,
}

impl Watch for Apart {
    fn progress(&self, done: f64) {
        let now = Instant::now();
        if done < 1.0 && self.last.get().is_some_and(|t| now - t < PROGRESS_EVERY) {
            return;
        }
        self.last.set(Some(now));
        let _ = self.out.unbounded_send(Reply::Progress(done));
    }

    fn stretch(&self, from: f64, until: f64, seconds: f64) {
        self.last.set(Some(Instant::now()));
        let _ = self
            .out
            .unbounded_send(Reply::Stretch(from, until, seconds));
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}

/// The rules' share of the bar, after the gathering's.
struct After<'a>(&'a Apart);

impl After<'_> {
    fn scale(done: f64) -> f64 {
        GATHER + (1.0 - GATHER) * done
    }
}

impl Watch for After<'_> {
    fn progress(&self, done: f64) {
        self.0.progress(Self::scale(done));
    }

    fn stretch(&self, from: f64, until: f64, seconds: f64) {
        self.0
            .stretch(Self::scale(from), Self::scale(until), seconds);
    }

    fn stopped(&self) -> bool {
        self.0.stopped()
    }
}
