//! The server's processor for `project.processing` jobs (docs/adr/0559 §3) against the real child process:
//! `kentos-jobrun` opens a project's `.kcad`, runs an İşlemler tool in the headless host and answers with a delta or a
//! GeoPackage; the parent says what the tool said, measures it, kills it at its budget or at a stop, and says what a
//! child that ended with no answer was. No database: the processor takes a `.kcad` and gives what the tool did.

#![allow(dead_code)]

#[path = "../src/processing_job.rs"]
mod processing_job;
mod support;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use kentos_application::processing_job::{
    JobProgress, ProcessFailure, ProcessHooks, Processed, ProcessingBudget, ProcessingTask,
    Processor,
};
use kentos_contracts::{FeatureChange, ProcessingJobInput, ProcessingJobMode};
use processing_job::ChildProcessor;
use uuid::Uuid;

/// The parcels as the server hands a project over: one KCAD v2 file.
fn parcels() -> Vec<u8> {
    support::PARCELS.to_vec()
}

fn work_dir() -> PathBuf {
    std::env::temp_dir().join(format!("kentos-job-run-test-{}", Uuid::now_v7()))
}

fn processor() -> ChildProcessor {
    ChildProcessor::with(support::child_exe(), work_dir())
}

fn budget() -> ProcessingBudget {
    ProcessingBudget {
        cpu_seconds: 600,
        memory_bytes: 8 << 30,
    }
}

fn task(input: ProcessingJobInput, budget: ProcessingBudget) -> ProcessingTask {
    ProcessingTask {
        job: Uuid::now_v7(),
        snapshot: parcels(),
        input,
        budget,
    }
}

fn hooks() -> ProcessHooks {
    ProcessHooks {
        progress: Arc::new(JobProgress::default()),
        stop: Arc::new(AtomicBool::new(false)),
    }
}

/// The tool numbering the corners of the two parcels (the first two objects, by their persistent ids).
fn numbering(mode: ProcessingJobMode) -> ProcessingJobInput {
    let doc = kentos_kcad::decode(&parcels()).expect("the parcels read");
    ProcessingJobInput {
        tool: "points.numberVertices".into(),
        label: Some("Köşe noktalarını numarala".into()),
        selection: doc
            .uids
            .iter()
            .take(2)
            .map(|u| Uuid::from_bytes(u.0).to_string())
            .collect(),
        mode,
        ..ProcessingJobInput::default()
    }
}

fn run(input: ProcessingJobInput) -> (Result<Processed, ProcessFailure>, ProcessHooks) {
    let hooks = hooks();
    let out = processor().process(&task(input, budget()), &hooks);
    (out, hooks)
}

#[test]
fn a_tool_runs_in_a_child_and_what_it_changed_comes_back_as_a_delta() {
    let root = work_dir();
    let hooks = hooks();
    let out = ChildProcessor::with(support::child_exe(), root.clone()).process(
        &task(numbering(ProcessingJobMode::Commit), budget()),
        &hooks,
    );
    let processed = out.expect("the tool ran");
    assert!(
        processed.summary.contains("6 köşe numaralandı"),
        "{}",
        processed.summary
    );
    let delta = processed.delta.expect("a delta");
    // Six corner points made on a layer the tool made; nothing of the parcels changed.
    assert_eq!((delta.added, delta.changed, delta.removed), (6, 0, 0));
    assert!(
        delta
            .features
            .iter()
            .all(|c| matches!(c, FeatureChange::Create { .. }))
    );
    let layers = delta
        .project
        .as_ref()
        .and_then(|p| p.layers.as_ref())
        .expect("the layer tree with the new layer");
    assert!(layers.iter().any(|l| l.id == "islem-kose-noktalari"));
    // It said where it was, and what it took was measured.
    let (fraction, words) = hooks.progress.get().expect("the child said something");
    assert!(fraction > 0.0 && !words.is_empty(), "{fraction} {words}");
    assert!(
        processed.usage.peak_memory_mb > 1.0,
        "{:?}",
        processed.usage
    );
    assert!(processed.sink.is_none());
    // The work folder is gone.
    assert_eq!(
        std::fs::read_dir(&root).map(Iterator::count).unwrap_or(0),
        0
    );
}

#[test]
fn a_result_to_a_file_comes_back_as_the_geopackage() {
    let (out, _) = run(numbering(ProcessingJobMode::Sink));
    let processed = out.expect("the tool ran");
    let file = processed.sink.expect("a GeoPackage");
    assert!(file.starts_with(b"SQLite format 3\0"), "a SQLite file");
    assert_eq!(processed.sink_rows, 6);
    assert!(processed.delta.is_none());
}

#[test]
fn a_tool_that_fails_or_a_request_the_child_refuses_says_why() {
    // The tool is not there.
    let (out, _) = run(ProcessingJobInput {
        tool: "yok.arac".into(),
        ..ProcessingJobInput::default()
    });
    let failure = out.expect_err("no such tool");
    assert_eq!(failure.code, "invalid", "{failure:?}");
    // A local file in a value: the child checks it again whatever the server did.
    let mut input = numbering(ProcessingJobMode::Commit);
    input
        .parameters
        .insert("dosya".into(), serde_json::json!({ "path": "/etc/passwd" }));
    let failure = run(input).0.expect_err("no local file");
    assert_eq!(failure.code, "invalid");
    assert!(
        failure.message.contains("input.parameters.dosya.path"),
        "{}",
        failure.message
    );
    // A tool that stops of its own (an object that is not in the project).
    let mut input = numbering(ProcessingJobMode::Commit);
    input.selection = vec![Uuid::new_v4().to_string()];
    let failure = run(input).0.expect_err("no such object");
    assert!(
        ["invalid", "tool_failed"].contains(&failure.code.as_str()),
        "{failure:?}"
    );
}

#[test]
fn a_memory_budget_is_kept_by_killing_the_child() {
    let started = Instant::now();
    let hooks = hooks();
    let tiny = ProcessingBudget {
        cpu_seconds: 600,
        memory_bytes: 1 << 20,
    };
    let failure = processor()
        .process(&task(numbering(ProcessingJobMode::Commit), tiny), &hooks)
        .expect_err("past the memory budget");
    assert_eq!(failure.code, "memory_limit", "{failure:?}");
    assert!(failure.message.contains("1 MB"), "{}", failure.message);
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn a_cpu_budget_is_kept_by_killing_the_child() {
    // A child that burns the processor until it is stopped, and a budget of one CPU second.
    let burner = support::shell_program("while :; do :; done");
    let started = Instant::now();
    let failure = ChildProcessor::with(burner, work_dir())
        .process(
            &task(
                numbering(ProcessingJobMode::Commit),
                ProcessingBudget {
                    cpu_seconds: 1,
                    memory_bytes: 8 << 30,
                },
            ),
            &hooks(),
        )
        .expect_err("past the CPU budget");
    assert_eq!(failure.code, "cpu_limit", "{failure:?}");
    assert!(
        failure.message.contains("bütçesi 1 sn"),
        "{}",
        failure.message
    );
    // Stopped soon after the second was used, not left running.
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_stop_kills_the_child() {
    // A child that waits (a tool would be done before a stop could come), and a stop after a moment.
    let waiter = support::shell_program("sleep 30");
    let hooks = hooks();
    let stop = hooks.stop.clone();
    let asked = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        stop.store(true, Ordering::SeqCst);
    });
    let started = Instant::now();
    let failure = ChildProcessor::with(waiter, work_dir())
        .process(
            &task(numbering(ProcessingJobMode::Commit), budget()),
            &hooks,
        )
        .expect_err("stopped");
    asked.join().unwrap();
    assert_eq!(failure.code, "stopped");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "killed, not waited for"
    );
}

#[test]
fn a_child_that_ends_with_no_answer_is_said_to_have_crashed() {
    let hooks = hooks();
    let failure = ChildProcessor::with(PathBuf::from("/usr/bin/false"), work_dir())
        .process(
            &task(numbering(ProcessingJobMode::Commit), budget()),
            &hooks,
        )
        .expect_err("no answer");
    assert_eq!(failure.code, "crashed", "{failure:?}");
    assert!(
        failure.message.contains("hiçbir şey yazılmadı"),
        "{}",
        failure.message
    );
    // A program that is not there cannot start.
    let failure = ChildProcessor::with(PathBuf::from("/yok/kentosd"), work_dir())
        .process(
            &task(numbering(ProcessingJobMode::Commit), budget()),
            &hooks,
        )
        .expect_err("not started");
    assert_eq!(failure.code, "tool_unavailable");
}
