//! The server's processor for `project.processing`, `project.import` and `project.atlas` jobs (docs/adr/0559 §3, 0561 §3):
//! an İşlemler tool, an import command or the atlas export runs in a process of its own, so the server can measure what
//! it takes, stop it at its budget and survive what it does.
//!
//! The processor ([`ChildProcessor`]) writes the task to a private work folder (the project's `.kcad`, the request and,
//! for an import, a link to the staged file), starts `kentos-jobrun <folder>` (the headless host and the tools, a program of its own so the server does
//! not carry them) with an empty environment (the server's secrets stay in the server), reads the progress the child
//! prints, and watches it: every sample it reads the CPU time and resident memory of the child and of every process
//! under it (an audit rule's Python, docs/adr/8057 §58; `/proc` where there is one, `ps` elsewhere: [`tree`]); past the
//! task's budget it ends the child's process group (`cpu_limit`, `memory_limit`); when the worker says stop it ends the
//! group; when the child ends it reads what it wrote. The child has no database and no network of its own; it reads one
//! folder and writes to it. An audit's extension package goes into the folder beside the request (`extension.kek`);
//! where its rules come from is the processor's audit source (docs/adr/8057 §42).

use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use kentos_application::audit_catalog::AuditSource;
use kentos_application::processing_job::RunRequest;
use kentos_application::processing_job::{
    ProcessFailure, ProcessHooks, Processed, ProcessingBudget, ProcessingTask, Processor,
};
use kentos_contracts::{ProcessingJobMode, ProcessingUsage};
use uuid::Uuid;

#[path = "processing_job/tree.rs"]
mod tree;

/// The child's program name, beside the server's own, and the variable that names another place for it.
pub const CHILD_PROGRAM: &str = "kentos-jobrun";
pub const CHILD_ENV: &str = "KENTOS_JOBRUN";
/// The server's setting for the cloud audit box where it cannot close (docs/adr/8057 §61): `require` (the default)
/// or `report`; given to the child, whose environment is otherwise empty.
pub const AUDIT_CONFINE_ENV: &str = "KENTOS_AUDIT_CONFINE";
/// What a child's progress line starts with: `KENTOS-PROGRESS 0.4000 words`.
const PROGRESS_PREFIX: &str = "KENTOS-PROGRESS ";
/// How often the parent looks at whether the child ended or the worker said stop.
const LOOK_EVERY: Duration = Duration::from_millis(50);

/// How often it measures what the child takes: a read of `/proc` is cheap, `ps` is a program to start.
fn measure_every() -> Duration {
    if std::path::Path::new("/proc/self/stat").exists() {
        Duration::from_millis(100)
    } else {
        Duration::from_millis(200)
    }
}
/// How long after it starts a child is measured more often.
const EARLY: Duration = Duration::from_secs(2);
/// How much of the child's standard error is kept for a failure's message.
const STDERR_KEEP: usize = 4096;
/// The files of the work folder.
const SNAPSHOT: &str = "snapshot.kcad";
const REQUEST: &str = "request.json";
const ANSWER: &str = "answer.json";
const FAILURE: &str = "failure.json";
const SINK: &str = "sink.gpkg";
const ATLAS: &str = "atlas.pdf";
const PACKAGE: &str = "extension.kek";

// ── What a process takes ────────────────────────────────────────────────────

/// A process as measured: CPU seconds so far and resident memory now, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Taken {
    pub cpu_seconds: f64,
    pub rss_bytes: u64,
}

/// The resident memory in `/proc/<pid>/status`, in bytes (`VmRSS:  1234 kB`).
pub fn parse_vmrss(status: &str) -> Option<u64> {
    let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

/// `ps`'s accumulated CPU time (`[dd-][hh:]mm:ss[.cc]`) in seconds.
pub fn parse_ps_time(text: &str) -> Option<f64> {
    let text = text.trim();
    let (days, clock) = match text.split_once('-') {
        Some((d, rest)) => (d.parse::<f64>().ok()?, rest),
        None => (0.0, text),
    };
    let mut seconds = days * 86_400.0;
    let parts: Vec<&str> = clock.split(':').collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    for (i, part) in parts.iter().rev().enumerate() {
        let value: f64 = part.parse().ok()?;
        seconds += value * [1.0, 60.0, 3600.0][i];
    }
    Some(seconds)
}

/// What the process `pid` and every process under it take now; none when it is gone or cannot be read.
pub fn measure(pid: u32) -> Option<Taken> {
    tree::measure(pid)
}

/// Whether `taken` is past `budget`, as the failure that stops the job (docs/adr/0559 §4).
pub fn over_budget(budget: &ProcessingBudget, taken: &Taken) -> Option<ProcessFailure> {
    if taken.cpu_seconds > f64::from(budget.cpu_seconds) {
        return Some(ProcessFailure::new(
            "cpu_limit",
            format!(
                "Araç {:.0} sn işlemci süresi kullandı; bu işin işlemci bütçesi {} sn. İş durduruldu, hiçbir şey yazılmadı. Girdiyi daraltın ya da sunucu yöneticisinden bütçeyi artırmasını isteyin.",
                taken.cpu_seconds, budget.cpu_seconds
            ),
        ));
    }
    if taken.rss_bytes > budget.memory_bytes {
        return Some(ProcessFailure::new(
            "memory_limit",
            format!(
                "Araç {} MB bellek kullandı; bu işin bellek bütçesi {} MB. İş durduruldu, hiçbir şey yazılmadı. Girdiyi daraltın ya da sunucu yöneticisinden bütçeyi artırmasını isteyin.",
                taken.rss_bytes / (1024 * 1024),
                budget.memory_bytes / (1024 * 1024)
            ),
        ));
    }
    None
}

/// What a run took: the larger of what the parent saw while it ran and what the child told at its end (a run over
/// between two samples is told by the child alone, docs/adr/0561 §5).
fn usage_of(peak: &Taken, last: &Taken, own: &ProcessingUsage) -> ProcessingUsage {
    ProcessingUsage {
        cpu_seconds: (last.cpu_seconds.max(own.cpu_seconds) * 100.0).round() / 100.0,
        peak_memory_mb: ((peak.rss_bytes as f64 / (1024.0 * 1024.0)).max(own.peak_memory_mb)
            * 10.0)
            .round()
            / 10.0,
    }
}

// ── The parent ──────────────────────────────────────────────────────────────

/// Runs the tool in a child process of the server's own program.
pub struct ChildProcessor {
    exe: PathBuf,
    work: PathBuf,
    /// Where `project.audit` jobs take their rules from (docs/adr/8057 §42): the catalog and its prepared
    /// environments; none while the server's catalog is off.
    audits: Option<Arc<dyn AuditSource>>,
}

/// A work folder, removed when it goes out of scope.
struct Folder(PathBuf);

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl ChildProcessor {
    /// The child program beside the server's own (or where `KENTOS_JOBRUN` says), work folders in the system's
    /// temporary folder; an error saying where it looked when the program is not there.
    pub fn new() -> Result<Self, String> {
        let exe = match std::env::var_os(CHILD_ENV) {
            Some(path) => PathBuf::from(path),
            None => std::env::current_exe()
                .map_err(|e| format!("kentosd'nin yeri bulunamadı: {e}"))?
                .with_file_name(CHILD_PROGRAM),
        };
        if !exe.is_file() {
            return Err(format!(
                "{} yok: araç işlemi için `cargo build -p kentos-jobrun` ile derleyip kentosd'nin yanına koyun ya da {CHILD_ENV} ile yerini söyleyin.",
                exe.display()
            ));
        }
        Ok(Self::with(exe, std::env::temp_dir().join("kentos-jobs")))
    }

    /// A given program and work root (tests).
    pub fn with(exe: PathBuf, work: PathBuf) -> Self {
        ChildProcessor {
            exe,
            work,
            audits: None,
        }
    }

    /// This processor, its audits' rules taken from `audits` (docs/adr/8057 §42): the catalog the server opened
    /// (docs/adr/8059 §13).
    pub fn with_audits(mut self, audits: Arc<dyn AuditSource>) -> Self {
        self.audits = Some(audits);
        self
    }

    fn folder(&self, job: Uuid) -> Result<Folder, ProcessFailure> {
        let broke = |e: std::io::Error| {
            ProcessFailure::new(
                "failed",
                format!(
                    "İşin çalışma klasörü açılamadı ({}): {e}",
                    self.work.display()
                ),
            )
        };
        let path = self.work.join(job.to_string());
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).map_err(broke)?;
        Ok(Folder(path))
    }
}

/// Reads the child's standard output on a thread of its own: its progress lines go to the hooks.
fn read_progress(
    out: impl Read + Send + 'static,
    hooks: ProcessHooks,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix(PROGRESS_PREFIX)
                && let Some((fraction, words)) = rest.split_once(' ')
                && let Ok(fraction) = fraction.parse::<f32>()
            {
                hooks.progress.set(fraction, words.trim());
            }
        }
    })
}

/// Reads the child's standard error, keeping the first [`STDERR_KEEP`] bytes.
fn read_errors(err: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        let mut buf = [0u8; 1024];
        let mut err = err;
        while let Ok(n) = err.read(&mut buf) {
            if n == 0 {
                break;
            }
            if kept.len() < STDERR_KEEP {
                kept.extend_from_slice(&buf[..n.min(STDERR_KEEP - kept.len())]);
            }
        }
        String::from_utf8_lossy(&kept).into_owned()
    })
}

impl Processor for ChildProcessor {
    /// The child program's size and change time (docs/adr/0739 §2): read at each job, so a program put in place while
    /// the server runs is a new build to the result cache.
    fn software(&self) -> Option<String> {
        std::fs::metadata(&self.exe).ok().map(|m| {
            format!(
                "{CHILD_PROGRAM}+{}",
                kentos_application::job_cache::file_mark(&m)
            )
        })
    }

    fn audits(&self) -> Option<Arc<dyn AuditSource>> {
        self.audits.clone()
    }

    fn process(
        &self,
        task: &ProcessingTask,
        hooks: &ProcessHooks,
    ) -> Result<Processed, ProcessFailure> {
        let folder = self.folder(task.job)?;
        let wrote = |name: &str, bytes: &[u8]| {
            std::fs::write(folder.0.join(name), bytes).map_err(|e| {
                ProcessFailure::new("failed", format!("İşin çalışma klasörüne yazılamadı: {e}"))
            })
        };
        wrote(SNAPSHOT, &task.snapshot)?;
        wrote(
            REQUEST,
            &serde_json::to_vec(&task.request).unwrap_or_default(),
        )?;
        // An audit's extension package, beside the request: the job runner checks it against the digest asked.
        if let Some(package) = &task.package {
            wrote(PACKAGE, package)?;
        }
        // An import's file is read where the object store keeps it: linked into the folder, copied where a link cannot
        // be made (a staged file is gigabytes at most, never read into the server's memory).
        if let (RunRequest::Import(run), Some(file)) = (&task.request, &task.file) {
            let to = folder.0.join(format!("import.{}", run.format.extension()));
            link_or_copy(&file.path, &to).map_err(|e| {
                ProcessFailure::new(
                    "failed",
                    format!(
                        "Hazırlanan dosya işin klasörüne konamadı ({}): {e}",
                        file.name
                    ),
                )
            })?;
        }
        let mut command = Command::new(&self.exe);
        command
            .arg(&folder.0)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // A group of its own: ending it at the budget or a stop ends what the child started too (a rule's Python).
        tree::own_group(&mut command);
        // The server's choice where the audit box cannot close (docs/adr/8057 §61): the child has no other environment.
        if let Some(policy) = std::env::var_os(AUDIT_CONFINE_ENV) {
            command.env(AUDIT_CONFINE_ENV, policy);
        }
        let mut child = command.spawn().map_err(|e| {
            ProcessFailure::new(
                "tool_unavailable",
                format!("Araç işlemi başlatılamadı ({}): {e}", self.exe.display()),
            )
        })?;
        let pid = child.id();
        let stdin = child.stdin.take();
        let progress = child
            .stdout
            .take()
            .map(|out| read_progress(out, hooks.clone()));
        let errors = child.stderr.take().map(read_errors);
        let mut peak = Taken::default();
        let mut last = Taken::default();
        let every = measure_every();
        let began = std::time::Instant::now();
        let mut measured = began.checked_sub(every).unwrap_or(began);
        let status = loop {
            if hooks.stop.load(Ordering::SeqCst) {
                tree::end(&mut child);
                return Err(ProcessFailure::new("stopped", "İş durduruldu."));
            }
            // Looked at four times as often while the child starts: a run that is over in a fraction of a second is
            // still seen working (what a sample misses of one that ends between two is told in docs/adr/0561 §5).
            let step = if began.elapsed() < EARLY {
                every / 4
            } else {
                every
            };
            let due = measured.elapsed() >= step;
            if due {
                measured = std::time::Instant::now();
            }
            if let Some(taken) = due.then(|| measure(pid)).flatten() {
                peak.rss_bytes = peak.rss_bytes.max(taken.rss_bytes);
                peak.cpu_seconds = peak.cpu_seconds.max(taken.cpu_seconds);
                last = taken;
                if let Some(over) = over_budget(&task.budget, &taken) {
                    tree::end(&mut child);
                    return Err(over);
                }
            }
            match tree::ended(&mut child) {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(LOOK_EVERY),
                Err(e) => {
                    tree::end(&mut child);
                    return Err(ProcessFailure::new(
                        "crashed",
                        format!("Araç işleminin durumu okunamadı: {e}"),
                    ));
                }
            }
        };
        drop(stdin);
        if let Some(t) = progress {
            let _ = t.join();
        }
        let stderr = errors.and_then(|t| t.join().ok()).unwrap_or_default();
        let read = |name: &str| std::fs::read(folder.0.join(name)).ok();
        if let Some(bytes) = read(FAILURE) {
            let failure: ProcessFailure = serde_json::from_slice(&bytes).map_err(|e| {
                ProcessFailure::new("crashed", format!("Aracın hatası okunamadı: {e}"))
            })?;
            return Err(failure);
        }
        let answer = read(ANSWER);
        let (true, Some(answer)) = (status.success(), answer) else {
            return Err(crashed(&status, &stderr, &task.budget, &peak));
        };
        let mut processed: Processed = serde_json::from_slice(&answer)
            .map_err(|e| ProcessFailure::new("crashed", format!("Aracın cevabı okunamadı: {e}")))?;
        match &task.request {
            RunRequest::Processing(input) if input.mode == ProcessingJobMode::Sink => {
                processed.file = read(SINK);
            }
            RunRequest::Atlas(_) => processed.file = read(ATLAS),
            _ => {}
        }
        processed.usage = usage_of(&peak, &last, &processed.usage);
        Ok(processed)
    }
}

/// A symbolic link where the system has them, else a copy. The link names the file by its absolute path: a link to a
/// relative one is read from the link's own folder, not from where the server runs (the object store's root may be
/// relative, `KENTOS_BLOB_DIR=.run/blobs`).
fn link_or_copy(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(std::fs::canonicalize(from)?, to)
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(from, to).map(|_| ())
    }
}

/// A child that ended with no answer: killed by the system (memory, most likely) or broken.
fn crashed(
    status: &std::process::ExitStatus,
    stderr: &str,
    budget: &ProcessingBudget,
    peak: &Taken,
) -> ProcessFailure {
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    // The system's kill (9) of a process near its memory budget is the memory the tool took.
    if signal == Some(9)
        && peak.rss_bytes.saturating_mul(10) >= budget.memory_bytes.saturating_mul(6)
    {
        return ProcessFailure::new(
            "memory_limit",
            format!(
                "Araç işlemi bellek yetmediği için sistem tarafından sonlandırıldı (en çok {} MB, bütçe {} MB). Hiçbir şey yazılmadı.",
                peak.rss_bytes / (1024 * 1024),
                budget.memory_bytes / (1024 * 1024)
            ),
        );
    }
    let how = match (signal, status.code()) {
        (Some(s), _) => format!("sinyal {s} ile"),
        (None, Some(c)) => format!("{c} koduyla"),
        _ => "beklenmedik biçimde".to_owned(),
    };
    let tail = stderr.trim();
    ProcessFailure::new(
        "crashed",
        if tail.is_empty() {
            format!("Araç işlemi {how} sonlandı; hiçbir şey yazılmadı.")
        } else {
            format!("Araç işlemi {how} sonlandı; hiçbir şey yazılmadı. Son sözleri: {tail}")
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cpu_time_comes_out_of_a_proc_stat_line_with_spaces_in_its_command() {
        // utime 250, stime 50, cutime 120 (a child waited for: a rule's Python the job runner ended), cstime 30.
        let stat = "4242 (kentosd job (run)) S 1 4242 4242 0 -1 4194560 1000 0 0 0 250 50 120 30 20 0 1 0 12345 99999 100 18446744073709551615";
        let line = tree::parse_proc_line(stat, 100.0).expect("a stat line");
        assert_eq!(
            (line.state, line.parent, line.own, line.reaped),
            ('S', 1, 3.0, 1.5)
        );
        assert_eq!(tree::parse_proc_line("garbage", 100.0), None);
        assert_eq!(tree::parse_proc_line("1 (a) S 1 2", 100.0), None);
    }

    /// The tree under a process from every process's `ps` row: its own, its children's and theirs, nobody else's.
    #[test]
    fn a_process_tree_is_summed_from_ps_rows() {
        let rows = tree::parse_ps_rows(
            "    1     0  1000   1:00:00\n  500     1  2048     00:02\n  501   500 10240  01:30.50\n  502   501   512     00:01\n  600     1  4096     00:09\n",
        );
        assert_eq!(rows.len(), 5);
        let taken = tree::tree_taken(&rows, 500).expect("the job runner's tree");
        assert_eq!(taken.rss_bytes, (2048 + 10240 + 512) * 1024);
        assert!((taken.cpu_seconds - 93.5).abs() < 1e-9, "{taken:?}");
        assert_eq!(tree::tree_taken(&rows, 999), None);
    }

    #[test]
    fn the_resident_memory_comes_out_of_proc_status() {
        let status = "Name:\tkentosd\nVmPeak:\t  9999 kB\nVmRSS:\t  2048 kB\nThreads:\t3\n";
        assert_eq!(parse_vmrss(status), Some(2048 * 1024));
        assert_eq!(parse_vmrss("Name:\tx\n"), None);
    }

    #[test]
    fn ps_times_read_in_every_form_they_come_in() {
        assert_eq!(parse_ps_time("0:01.50"), Some(1.5));
        assert_eq!(parse_ps_time("00:00:07"), Some(7.0));
        assert_eq!(parse_ps_time("1:02:03"), Some(3723.0));
        assert_eq!(parse_ps_time("2-00:00:01"), Some(172_801.0));
        assert_eq!(parse_ps_time("12.5"), Some(12.5));
        assert_eq!(parse_ps_time("a:b"), None);
        assert_eq!(parse_ps_time("1:2:3:4"), None);
    }

    #[test]
    fn a_file_named_by_a_relative_path_is_linked_by_its_absolute_one() {
        // The object store's root may be relative to where the server runs; the link is read from its own folder.
        let name = format!("kentos-relative-link-{}", std::process::id());
        std::fs::write(&name, b"bytes").expect("a file in the working folder");
        let folder = std::env::temp_dir().join(format!("{name}-folder"));
        std::fs::create_dir_all(&folder).expect("a folder");
        let link = folder.join("import.gpkg");
        link_or_copy(std::path::Path::new(&name), &link).expect("linked");
        assert_eq!(
            std::fs::read(&link).expect("read through the link"),
            b"bytes"
        );
        let _ = std::fs::remove_file(&name);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn this_process_can_be_measured() {
        let taken = measure(std::process::id()).expect("a running process is measurable");
        assert!(taken.rss_bytes > 1024 * 1024, "{taken:?}");
        assert!(taken.cpu_seconds >= 0.0);
    }

    #[test]
    fn a_budget_is_past_by_cpu_or_by_memory_and_the_words_say_which() {
        let budget = ProcessingBudget {
            cpu_seconds: 10,
            memory_bytes: 512 * 1024 * 1024,
        };
        let within = Taken {
            cpu_seconds: 10.0,
            rss_bytes: 512 * 1024 * 1024,
        };
        assert_eq!(over_budget(&budget, &within), None);
        let cpu = over_budget(
            &budget,
            &Taken {
                cpu_seconds: 10.5,
                ..within
            },
        )
        .expect("past the CPU budget");
        assert_eq!(cpu.code, "cpu_limit");
        assert!(cpu.message.contains("10 sn"), "{}", cpu.message);
        let memory = over_budget(
            &budget,
            &Taken {
                rss_bytes: 513 * 1024 * 1024,
                ..within
            },
        )
        .expect("past the memory budget");
        assert_eq!(memory.code, "memory_limit");
        assert!(memory.message.contains("512 MB"), "{}", memory.message);
    }
}
