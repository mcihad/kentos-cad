//! The server's processor for `project.processing`, `project.import` and `project.atlas` jobs (docs/adr/0559 §3,
//! 0561 §3) against the real child process: `kentos-jobrun` opens a project's `.kcad`, runs an İşlemler tool, an import
//! command or the atlas export in the headless host and answers with a delta or a file; the parent says what it said,
//! measures it, kills it at its budget or at a stop, and says what a child that ended with no answer was. No database:
//! the processor takes a `.kcad` and gives what the work did.

#![allow(dead_code)]

#[path = "../src/processing_job.rs"]
mod processing_job;
mod support;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use kentos_application::processing_job::{
    ImportRun, JobProgress, ProcessFailure, ProcessHooks, Processed, ProcessingBudget,
    ProcessingTask, Processor, RunRequest, StagedFile,
};
use kentos_contracts::{
    BlockChange, FeatureChange, ImportFormat, ProcessingJobInput, ProcessingJobMode,
    ProjectAtlasInput,
};
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
        request: RunRequest::Processing(input),
        file: None,
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
    // What it took is said whatever its length: the parent samples it while it runs, and the child tells its own at its
    // end (docs/adr/0561 §5), so a run over between two samples is not told as nothing.
    assert!(
        processed.usage.peak_memory_mb > 1.0,
        "{:?}",
        processed.usage
    );
    assert!(processed.usage.cpu_seconds > 0.0, "{:?}", processed.usage);
    assert!(processed.file.is_none());
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
    let file = processed.file.expect("a GeoPackage");
    assert!(file.starts_with(b"SQLite format 3\0"), "a SQLite file");
    assert_eq!(processed.rows, 6);
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
    // A child that stays alive (the real one can be over before its first sample: docs/adr/0561 §5) and a budget of a
    // few kilobytes, which any process is past.
    let waiter = support::shell_program("sleep 30");
    let started = Instant::now();
    let hooks = hooks();
    let tiny = ProcessingBudget {
        cpu_seconds: 600,
        memory_bytes: 64 << 10,
    };
    let failure = ChildProcessor::with(waiter, work_dir())
        .process(&task(numbering(ProcessingJobMode::Commit), tiny), &hooks)
        .expect_err("past the memory budget");
    assert_eq!(failure.code, "memory_limit", "{failure:?}");
    assert!(
        failure.message.contains("bellek bütçesi"),
        "{}",
        failure.message
    );
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

// ── Imports and the atlas (docs/adr/0561) ───────────────────────────────────

const ATLAS: &[u8] = include_bytes!("../../../fixtures/server-jobs/v1/atlas.kcad");
const ATLAS_PIN: &str = include_str!("../../../fixtures/server-jobs/v1/atlas-pin.json");
const GPKG: &[u8] = include_bytes!("../../../fixtures/geopackage/v1/kentos-export.gpkg");
const GPKG_WGS84: &[u8] =
    include_bytes!("../../../fixtures/geopackage/v1/kentos-synced-wgs84.gpkg");
const DXF_BLOCKS: &[u8] = include_bytes!("../../../fixtures/formats/v1/blocks.dxf");

/// A file staged for an import: the bytes in a file of their own, as the object store keeps them.
fn staged(name: &str, bytes: &[u8]) -> StagedFile {
    let dir = work_dir();
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("the file");
    StagedFile {
        path,
        name: name.to_owned(),
        size: bytes.len() as u64,
    }
}

fn import_task(
    name: &str,
    bytes: &[u8],
    format: ImportFormat,
    options: serde_json::Value,
) -> ProcessingTask {
    let options = options
        .as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    ProcessingTask {
        job: Uuid::now_v7(),
        snapshot: parcels(),
        request: RunRequest::Import(ImportRun {
            format,
            name: name.to_owned(),
            options,
        }),
        file: Some(staged(name, bytes)),
        budget: budget(),
    }
}

fn atlas_task(input: ProjectAtlasInput) -> ProcessingTask {
    ProcessingTask {
        job: Uuid::now_v7(),
        snapshot: ATLAS.to_vec(),
        request: RunRequest::Atlas(input),
        file: None,
        budget: budget(),
    }
}

fn atlas_pin() -> kentos_contracts::AtlasPin {
    serde_json::from_str(ATLAS_PIN).expect("the pin the plan gave")
}

#[test]
fn a_geopackage_is_taken_in_by_the_import_command_in_the_child() {
    let out = processor().process(
        &import_task(
            "kentos-export.gpkg",
            GPKG,
            ImportFormat::Gpkg,
            serde_json::json!({}),
        ),
        &hooks(),
    );
    let processed = out.expect("the import ran");
    let delta = processed.delta.expect("a delta");
    assert_eq!((delta.added, delta.changed, delta.removed), (10, 0, 0));
    assert!(
        delta
            .features
            .iter()
            .all(|c| matches!(c, FeatureChange::Create { .. }))
    );
    assert!(
        processed.summary.contains("10 nesne eklendi"),
        "{}",
        processed.summary
    );
    // What the command answered is kept: the tables it read.
    assert_eq!(processed.details["objects"], 10);
    assert!(
        processed.details["tables"]
            .as_array()
            .is_some_and(|t| !t.is_empty())
    );
}

#[test]
fn a_file_in_another_system_is_refused_with_the_commands_own_code_and_taken_when_asked_to_move() {
    let failure = processor()
        .process(
            &import_task(
                "wgs84.gpkg",
                GPKG_WGS84,
                ImportFormat::Gpkg,
                serde_json::json!({}),
            ),
            &hooks(),
        )
        .expect_err("another system");
    assert_eq!(failure.code, "crs_mismatch", "{failure:?}");
    let moved = processor()
        .process(
            &import_task(
                "wgs84.gpkg",
                GPKG_WGS84,
                ImportFormat::Gpkg,
                serde_json::json!({ "reproject": true }),
            ),
            &hooks(),
        )
        .expect("moved into the project's system");
    assert!(moved.delta.expect("a delta").added > 0);
}

#[test]
fn a_dxf_with_blocks_brings_its_block_definitions_with_it() {
    let processed = processor()
        .process(
            &import_task(
                "bloklar.dxf",
                DXF_BLOCKS,
                ImportFormat::Dxf,
                serde_json::json!({}),
            ),
            &hooks(),
        )
        .expect("the import ran");
    let delta = processed.delta.expect("a delta");
    assert!(
        delta
            .blocks
            .iter()
            .any(|b| matches!(b, BlockChange::Create { .. })),
        "{:?} blocks",
        delta.blocks.len()
    );
    assert!(delta.added > 0, "its objects too");
}

#[test]
fn a_path_among_the_options_or_a_file_that_is_not_there_is_refused_by_the_child_too() {
    let failure = processor()
        .process(
            &import_task(
                "a.gpkg",
                GPKG,
                ImportFormat::Gpkg,
                serde_json::json!({ "path": "/etc/passwd" }),
            ),
            &hooks(),
        )
        .expect_err("no path of the request's own");
    assert_eq!(failure.code, "invalid");
    assert!(
        failure.message.contains("input.options.path"),
        "{}",
        failure.message
    );
    // No file in the folder: the server gave none.
    let mut missing = import_task("a.gpkg", GPKG, ImportFormat::Gpkg, serde_json::json!({}));
    missing.file = None;
    let failure = processor()
        .process(&missing, &hooks())
        .expect_err("no file");
    assert_eq!(failure.code, "invalid");
}

#[test]
fn the_atlas_comes_back_as_a_pdf_of_a_page_for_each_object() {
    let hooks = hooks();
    let processed = processor()
        .process(
            &atlas_task(ProjectAtlasInput {
                sheet: "atlas-paftasi".into(),
                pin: Some(atlas_pin()),
                ..ProjectAtlasInput::default()
            }),
            &hooks,
        )
        .expect("the atlas was made");
    let pdf = processed.file.expect("a PDF");
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(processed.details["pages"].as_array().map(Vec::len), Some(6));
    assert_eq!(
        processed.details["pin"]["digest"].as_str(),
        Some(atlas_pin().digest.as_str())
    );
    assert!(
        processed.summary.contains("6 sayfalık"),
        "{}",
        processed.summary
    );
    assert!(processed.delta.is_none());
    assert!(hooks.progress.get().is_some());
}

#[test]
fn a_pin_the_project_has_moved_on_from_and_a_sheet_that_is_not_there_are_refused_by_code() {
    let mut pin = atlas_pin();
    pin.digest = "0".repeat(64);
    pin.layers = "0".repeat(64);
    let failure = processor()
        .process(
            &atlas_task(ProjectAtlasInput {
                sheet: "atlas-paftasi".into(),
                pin: Some(pin),
                ..ProjectAtlasInput::default()
            }),
            &hooks(),
        )
        .expect_err("not the state the pin names");
    assert_eq!(failure.code, "pin_mismatch", "{failure:?}");
    let failure = processor()
        .process(
            &atlas_task(ProjectAtlasInput {
                sheet: "yok".into(),
                ..ProjectAtlasInput::default()
            }),
            &hooks(),
        )
        .expect_err("no such sheet");
    assert_eq!(failure.code, "unknown_sheet", "{failure:?}");
}
