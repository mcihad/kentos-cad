//! An extension version's process for the work that does not wait on the window (plan/eklenti.md ADIM 19; docs/adr/8057
//! §7): İşlemler' tools, run from any thread (the window's run thread, a job's, a command's), each extension version in
//! a process of its own, kept between runs and closed when idle.
//!
//! - **Started as a command's:** `<env>/python -I -X pycache_prefix=<env>/pyc -m kentos.ext_host` (the caller gives the
//!   interpreter and its arguments), its lines JSON (protocol 1, `kentos.ext_host`); no console window on Windows.
//! - **One run at a time in a process:** a second run of the same version waits for its turn, its stop heard as it
//!   waits.
//! - **Durdur:** a `cancel` first; a process that has not answered in [`KILL_AFTER`] is ended (PY-RUN-12). A run that
//!   overruns its time is stopped the same way. A process that ends by itself is a crash, said with what it wrote to
//!   its errors last.
//! - **Idle:** a process not used for [`IDLE`] is asked to quit, and ended if it does not (D6, Ö·9); the next run starts
//!   a new one.
//! - **Memory:** what a process holds (its resident set), read where the system says it (Linux's `/proc`); measured,
//!   not promised.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// How long a stopped run is given before its process is ended.
pub const KILL_AFTER: Duration = Duration::from_secs(5);

/// How long a process may wait unused before it is closed.
pub const IDLE: Duration = Duration::from_secs(10 * 60);

/// How long a process may take to say it is ready (an environment's first import is the slowest).
pub const READY_WITHIN: Duration = Duration::from_secs(120);

/// How much of what a process wrote to its errors is kept, the newest.
const KEPT_ERRORS: usize = 16 * 1024;

/// The protocol this side speaks (`kentos.ext_host.PROTOCOL`).
pub const PROTOCOL: u64 = 1;

/// How a process is started: its interpreter, its arguments and its hello.
#[derive(Clone, Debug)]
pub struct Start {
    pub python: PathBuf,
    pub args: Vec<OsString>,
    /// The first line it is told (`{"type": "hello", …}`).
    pub hello: Value,
}

/// What a run says as it goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    Progress {
        done: f64,
        total: Option<f64>,
        text: Option<String>,
    },
    Log {
        level: String,
        text: String,
    },
    Out {
        stream: String,
        text: String,
    },
}

/// How a run ended.
#[derive(Clone, Debug, PartialEq)]
pub enum Ended {
    /// The code's answer (its `done`): a result, or its own failure, refusal or stop; a stopped run's with
    /// `stoppedAfterMs`, how long after the stop was asked it answered.
    Answered(Value),
    /// Stopped before it ran: while it waited for its turn or for its process.
    StoppedBefore,
    /// It did not answer the stop (`timed_out`: nor its time) within [`KILL_AFTER`]: its process was ended.
    Killed { timed_out: bool },
    /// Its process could not start or say it is ready: why.
    NotStarted(String),
    /// Its process ended by itself: its exit and what it wrote to its errors last.
    Crashed(String),
}

/// A process as the manager shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Running {
    /// The extension version's key (`<extension>@<version>`).
    pub key: String,
    pub pid: u32,
    /// Its resident memory, bytes, where the system says it.
    pub memory: Option<u64>,
    pub runs: u64,
    /// Since its last run ended; none while one runs.
    pub idle: Option<Duration>,
}

struct Process {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<Value>,
    errors: Arc<Mutex<String>>,
    last: Instant,
    runs: u64,
    next: u64,
}

impl Process {
    fn send(&mut self, message: &Value) -> bool {
        let mut line = message.to_string();
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|()| self.stdin.flush())
            .is_ok()
    }

    fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Its exit and its errors' last lines, after it ended.
    fn crash_words(&mut self) -> String {
        // Its error reader gets what it wrote last before its pipe closes.
        std::thread::sleep(Duration::from_millis(50));
        let status = self
            .child
            .try_wait()
            .ok()
            .flatten()
            .map_or_else(|| "sonlandı".to_owned(), |s| format!("çıkış: {s}"));
        let errors = self
            .errors
            .lock()
            .map(|e| e.trim().to_owned())
            .unwrap_or_default();
        let tail: String = errors
            .lines()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        if tail.is_empty() {
            status
        } else {
            format!("{status}\n{tail}")
        }
    }

    fn end(mut self) {
        let _ = self.send(&json!({ "type": "quit" }));
        let until = Instant::now() + Duration::from_secs(2);
        while Instant::now() < until {
            if !self.alive() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// An extension version's place: its process when it has one, and what the manager reads without waiting for a run.
#[derive(Default)]
struct Place {
    process: Mutex<Option<Process>>,
    pid: AtomicU32,
    runs: AtomicU64,
    /// When its last run ended, ms since the pool began; 0 while one runs or before the first.
    ended: AtomicU64,
}

/// The extension versions' processes.
pub struct Pool {
    places: Mutex<HashMap<String, Arc<Place>>>,
    began: Instant,
    idle: Duration,
    kill_after: Duration,
    reaper: OnceLock<()>,
}

impl Default for Pool {
    fn default() -> Self {
        Self::new(IDLE)
    }
}

fn spawn(start: &Start) -> Result<Process, String> {
    let mut command = Command::new(&start.python);
    command
        .args(&start.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("{} başlatılamadı: {e}", start.python.display()))?;
    let (Some(stdin), Some(stdout), Some(stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        let _ = child.kill();
        return Err("Sürecin kanalları açılamadı.".to_owned());
    };
    let (sent, lines) = channel();
    std::thread::Builder::new()
        .name("kentos-ext-oku".into())
        .spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else {
                    break;
                };
                if let Ok(message @ Value::Object(_)) = serde_json::from_str::<Value>(&line)
                    && sent.send(message).is_err()
                {
                    break;
                }
            }
        })
        .map_err(|e| e.to_string())?;
    let errors = Arc::new(Mutex::new(String::new()));
    let kept = Arc::clone(&errors);
    std::thread::Builder::new()
        .name("kentos-ext-hata".into())
        .spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut chunk = [0u8; 4096];
            while let Ok(n) = reader.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                if let Ok(mut all) = kept.lock() {
                    all.push_str(&String::from_utf8_lossy(&chunk[..n]));
                    if all.len() > KEPT_ERRORS {
                        let mut cut = all.len() - KEPT_ERRORS;
                        while !all.is_char_boundary(cut) {
                            cut += 1;
                        }
                        all.drain(..cut);
                    }
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(Process {
        child,
        stdin,
        lines,
        errors,
        last: Instant::now(),
        runs: 0,
        next: 0,
    })
}

/// Its resident memory, bytes: Linux's `VmRSS`; none elsewhere.
pub fn memory_of(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

impl Pool {
    /// A pool whose processes close after `idle` unused.
    pub fn new(idle: Duration) -> Self {
        Self {
            places: Mutex::new(HashMap::new()),
            began: Instant::now(),
            idle,
            kill_after: KILL_AFTER,
            reaper: OnceLock::new(),
        }
    }

    /// The same, a stopped run's process ended after `kill_after` (tests).
    pub fn with_kill_after(mut self, kill_after: Duration) -> Self {
        self.kill_after = kill_after;
        self
    }

    fn place(&self, key: &str) -> Arc<Place> {
        let mut places = self
            .places
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(places.entry(key.to_owned()).or_default())
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.began.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Runs `invoke` (an `invoke` message without its id) in `key`'s process, started by `start` when it has none, and
    /// waits for its answer: what it says as it goes to `heard`, `stop` asked every 20 ms, `timeout` its whole time.
    pub fn run(
        self: &Arc<Self>,
        key: &str,
        start: &dyn Fn() -> Result<Start, String>,
        mut invoke: Value,
        timeout: Duration,
        stop: &dyn Fn() -> bool,
        heard: &mut dyn FnMut(Heard),
    ) -> Ended {
        self.keep_reaping();
        let place = self.place(key);
        // Its turn: one run at a time in a process.
        let mut slot = loop {
            if let Ok(slot) = place.process.try_lock() {
                break slot;
            }
            if stop() {
                return Ended::StoppedBefore;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        place.ended.store(0, Ordering::Relaxed);
        let ended = self.run_in(&place, &mut slot, start, &mut invoke, timeout, stop, heard);
        if let Some(process) = slot.as_mut() {
            process.last = Instant::now();
        }
        place.ended.store(self.now_ms().max(1), Ordering::Relaxed);
        ended
    }

    #[allow(clippy::too_many_arguments)]
    fn run_in(
        &self,
        place: &Place,
        slot: &mut Option<Process>,
        start: &dyn Fn() -> Result<Start, String>,
        invoke: &mut Value,
        timeout: Duration,
        stop: &dyn Fn() -> bool,
        heard: &mut dyn FnMut(Heard),
    ) -> Ended {
        if slot.as_mut().is_none_or(|p| !p.alive()) {
            *slot = None;
            place.pid.store(0, Ordering::Relaxed);
            let begun = match start() {
                Ok(begun) => begun,
                Err(why) => return Ended::NotStarted(why),
            };
            let mut process = match spawn(&begun) {
                Ok(process) => process,
                Err(why) => return Ended::NotStarted(why),
            };
            if !process.send(&begun.hello) {
                return Ended::NotStarted(process.crash_words());
            }
            let until = Instant::now() + READY_WITHIN;
            loop {
                if stop() {
                    return Ended::StoppedBefore;
                }
                if Instant::now() > until {
                    return Ended::NotStarted(format!(
                        "Süreç {} sn içinde hazır olmadı.",
                        READY_WITHIN.as_secs()
                    ));
                }
                match process.lines.recv_timeout(Duration::from_millis(20)) {
                    Ok(m) if m["type"] == "ready" => {
                        if m["protocol"].as_u64() != Some(PROTOCOL) {
                            return Ended::NotStarted(format!(
                                "Eklenti süreci protokolün {} sürümünü konuşuyor; bu KentOS {PROTOCOL}.",
                                m["protocol"]
                            ));
                        }
                        break;
                    }
                    Ok(m) if m["type"] == "refused" => {
                        return Ended::NotStarted(
                            m["message"]
                                .as_str()
                                .unwrap_or("Süreç reddetti.")
                                .to_owned(),
                        );
                    }
                    Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => {
                        return Ended::NotStarted(process.crash_words());
                    }
                }
            }
            place.pid.store(process.child.id(), Ordering::Relaxed);
            *slot = Some(process);
        }
        let Some(process) = slot.as_mut() else {
            return Ended::NotStarted("Süreç yok.".to_owned());
        };
        process.next += 1;
        let id = process.next;
        process.runs += 1;
        place.runs.store(process.runs, Ordering::Relaxed);
        invoke["type"] = json!("invoke");
        invoke["id"] = json!(id);
        if !process.send(invoke) {
            let words = process.crash_words();
            *slot = None;
            return Ended::Crashed(words);
        }
        let began = Instant::now();
        // When the stop was asked, and whether by its time.
        let mut asked: Option<(Instant, bool)> = None;
        loop {
            match process.lines.recv_timeout(Duration::from_millis(20)) {
                Ok(m) => {
                    let mine = m["id"].as_u64() == Some(id);
                    match m["type"].as_str().unwrap_or_default() {
                        "done" if mine => {
                            let mut m = m;
                            // How long after the stop the code answered: what Durdur's promise is measured by.
                            if let Some((when, _)) = asked {
                                m["stoppedAfterMs"] = json!(
                                    u64::try_from(when.elapsed().as_millis()).unwrap_or(u64::MAX)
                                );
                            }
                            return Ended::Answered(m);
                        }
                        "progress" if mine => heard(Heard::Progress {
                            done: m["done"].as_f64().unwrap_or(0.0),
                            total: m["total"].as_f64(),
                            text: m["text"].as_str().map(str::to_owned),
                        }),
                        "log" if mine => heard(Heard::Log {
                            level: m["level"].as_str().unwrap_or("info").to_owned(),
                            text: m["text"].as_str().unwrap_or_default().to_owned(),
                        }),
                        "out" => heard(Heard::Out {
                            stream: m["stream"].as_str().unwrap_or("out").to_owned(),
                            text: m["text"].as_str().unwrap_or_default().to_owned(),
                        }),
                        // A tool has no drawing: its inputs came as data (PY-TOOL-09).
                        "call" => {
                            let _ = process.send(&json!({
                                "type": "reply",
                                "id": m["id"],
                                "ok": false,
                                "code": "capability",
                                "message": "Bu çalıştırma çizime erişemez: girdileri veri olarak verildi.",
                            }));
                        }
                        _ => {}
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    let words = process.crash_words();
                    *slot = None;
                    place.pid.store(0, Ordering::Relaxed);
                    return match asked {
                        // Ended while being stopped: it was asked to.
                        Some((_, timed_out)) => Ended::Killed { timed_out },
                        None => Ended::Crashed(words),
                    };
                }
            }
            match asked {
                None => {
                    let overdue = began.elapsed() > timeout;
                    if stop() || overdue {
                        let _ = process.send(&json!({ "type": "cancel", "id": id }));
                        asked = Some((Instant::now(), overdue));
                    }
                }
                Some((when, timed_out)) if when.elapsed() > self.kill_after => {
                    let _ = process.child.kill();
                    let _ = process.child.wait();
                    *slot = None;
                    place.pid.store(0, Ordering::Relaxed);
                    return Ended::Killed { timed_out };
                }
                Some(_) => {}
            }
        }
    }

    /// The processes now, for the manager: their memory read as it is.
    pub fn running(&self) -> Vec<Running> {
        let places = self
            .places
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = self.now_ms();
        let mut out: Vec<Running> = places
            .iter()
            .filter_map(|(key, place)| {
                let pid = place.pid.load(Ordering::Relaxed);
                (pid != 0).then(|| {
                    let ended = place.ended.load(Ordering::Relaxed);
                    Running {
                        key: key.clone(),
                        pid,
                        memory: memory_of(pid),
                        runs: place.runs.load(Ordering::Relaxed),
                        idle: (ended != 0)
                            .then(|| Duration::from_millis(now.saturating_sub(ended))),
                    }
                })
            })
            .collect();
        out.sort_by(|a, b| a.key.cmp(&b.key));
        out
    }

    /// Closes the processes not used for the pool's idle time; those that run are left.
    pub fn close_idle(&self) {
        self.close_where(|p| p.last.elapsed() >= self.idle);
    }

    /// Closes `key`'s process (its extension removed, turned off or updated); one that runs is left to end.
    pub fn close(&self, key: &str) {
        let place = self
            .places
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(key)
            .cloned();
        if let Some(place) = place
            && let Ok(mut slot) = place.process.try_lock()
            && let Some(process) = slot.take()
        {
            place.pid.store(0, Ordering::Relaxed);
            process.end();
        }
    }

    fn close_where(&self, when: impl Fn(&Process) -> bool) {
        let places: Vec<Arc<Place>> = self
            .places
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .cloned()
            .collect();
        for place in places {
            let Ok(mut slot) = place.process.try_lock() else {
                continue;
            };
            if slot.as_ref().is_some_and(&when)
                && let Some(process) = slot.take()
            {
                place.pid.store(0, Ordering::Relaxed);
                process.end();
            }
        }
    }

    /// Closes every process (the program ends).
    pub fn close_all(&self) {
        self.close_where(|_| true);
    }

    /// A thread that closes idle processes, started with the first run.
    fn keep_reaping(self: &Arc<Self>) {
        let pool = Arc::downgrade(self);
        self.reaper.get_or_init(|| {
            let every = (self.idle / 4).clamp(Duration::from_millis(50), Duration::from_secs(30));
            let _ = std::thread::Builder::new()
                .name("kentos-ext-bosta".into())
                .spawn(move || {
                    loop {
                        std::thread::sleep(every);
                        let Some(pool) = pool.upgrade() else {
                            return;
                        };
                        pool.close_idle();
                    }
                });
        });
    }
}

#[cfg(all(test, unix))]
mod tests;
