//! The job's budget covers the rule's Python (docs/adr/8057 §58): the processor measures `kentos-jobrun` and every
//! process under it and ends them together. A rule that burns the processor and one that takes memory are stopped at
//! the job's budget with the built-in tools' own outcomes (`cpu_limit`, `memory_limit`), and nothing of theirs is left
//! running; a normal rule fits the same budget and its usage counts its Python. Without a Python (`KENTOS_TEST_PYTHON`,
//! else the repository's `.run/py`) the tests say so and pass.

use std::path::Path;
use std::time::{Duration, Instant};

use kentos_application::processing_job::{ProcessingBudget, Processor};

use super::audit::{audit_task_with, python, release, release_with};
use super::{hooks, processing_job::ChildProcessor, support, work_dir};

/// A rule that keeps the processor busy until it is stopped.
const BURNER: &str = r#"@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """İşlemciyi durdurulana dek kullanır."""
    n = 0
    while True:
        n += 1
"#;

/// A rule that fills memory, 16 MB at a time up to 1 GB, until it is stopped.
const HOG: &str = r#"@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Belleği durdurulana dek doldurur."""
    parcalar = []
    for _ in range(64):
        parcalar.append(b"x" * (16 << 20))
    return []
"#;

/// The processes whose command line names `dir`: the job runner (its folder) and the rule's Python (its bytecode's
/// place, `-X pycache_prefix=`, is in the folder).
fn left_under(dir: &Path) -> Vec<u32> {
    let mark = dir.display().to_string();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let pid: u32 = e.file_name().to_str()?.parse().ok()?;
            let line = std::fs::read(e.path().join("cmdline")).ok()?;
            String::from_utf8_lossy(&line)
                .contains(&mark)
                .then_some(pid)
        })
        .collect()
}

/// What is left of the job a moment after it was ended (a killed process is gone within milliseconds).
fn left_after(dir: &Path) -> Vec<u32> {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        let left = left_under(dir);
        if left.is_empty() || Instant::now() > until {
            return left;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_rule_that_burns_the_processor_is_stopped_at_the_job_s_cpu_budget() {
    let Some(python) = python() else {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        return;
    };
    let (bytes, digest, manifest) = release_with(BURNER);
    let three = ProcessingBudget {
        cpu_seconds: 3,
        memory_bytes: 8 << 30,
    };
    // The job runner built (once) before the clock starts.
    let exe = support::child_exe();
    let work = work_dir();
    let started = Instant::now();
    let failure = ChildProcessor::with(exe, work.clone())
        .process(
            &audit_task_with(&digest, &manifest, Some(python), Some(bytes), three),
            &hooks(),
        )
        .expect_err("past the CPU budget");
    let took = started.elapsed();
    assert_eq!(failure.code, "cpu_limit", "{failure:?}");
    assert!(
        failure.message.contains("bütçesi 3 sn"),
        "{}",
        failure.message
    );
    // Long before the rule's own time (120 s) would end it.
    assert!(took < Duration::from_secs(30), "{took:?}");
    assert_eq!(left_after(&work), Vec::<u32>::new(), "nothing left running");
    eprintln!(
        "işlemciyi yakan kural 3 sn'lik bütçede {took:.2?} sonra durduruldu: {}",
        failure.message
    );
}

/// A normal rule's usage is measured first (it counts its Python: the SDK's native module alone is large in a debug
/// build); the budget is that and 300 MB. The normal rule fits it again; the rule that fills memory (up to 1 GB more) is
/// stopped past it, its processes gone.
#[test]
fn a_rule_that_takes_memory_past_the_budget_is_stopped_and_a_normal_one_fits() {
    let Some(python) = python() else {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        return;
    };
    let exe = support::child_exe();
    let normal = |budget: ProcessingBudget| {
        let (bytes, digest, manifest) = release();
        let started = Instant::now();
        let done = ChildProcessor::with(exe.clone(), work_dir())
            .process(
                &audit_task_with(
                    &digest,
                    &manifest,
                    Some(python.clone()),
                    Some(bytes),
                    budget,
                ),
                &hooks(),
            )
            .expect("a normal rule runs");
        (done.usage, started.elapsed())
    };
    let (usage, took) = normal(ProcessingBudget {
        cpu_seconds: 600,
        memory_bytes: 8 << 30,
    });
    assert!(
        usage.cpu_seconds > 0.0 && usage.peak_memory_mb > 0.0,
        "{usage:?}"
    );
    eprintln!(
        "olağan kural (iş çalıştırıcısı ve Python'u): {:.2} sn işlemci, en çok {:.1} MB, {took:.2?}",
        usage.cpu_seconds, usage.peak_memory_mb
    );
    let megabytes = usage.peak_memory_mb.ceil() as u64 + 300;
    let budget = ProcessingBudget {
        cpu_seconds: 600,
        memory_bytes: megabytes << 20,
    };
    let (again, _) = normal(budget);
    assert!(again.peak_memory_mb > 0.0);
    let (bytes, digest, manifest) = release_with(HOG);
    let work = work_dir();
    let started = Instant::now();
    let failure = ChildProcessor::with(exe, work.clone())
        .process(
            &audit_task_with(&digest, &manifest, Some(python), Some(bytes), budget),
            &hooks(),
        )
        .expect_err("past the memory budget");
    let took = started.elapsed();
    assert_eq!(failure.code, "memory_limit", "{failure:?}");
    assert!(
        failure
            .message
            .contains(&format!("bellek bütçesi {megabytes} MB")),
        "{}",
        failure.message
    );
    assert_eq!(left_after(&work), Vec::<u32>::new(), "nothing left running");
    eprintln!(
        "belleği dolduran kural {megabytes} MB'lık bütçede {took:.2?} sonra durduruldu: {}",
        failure.message
    );
}
