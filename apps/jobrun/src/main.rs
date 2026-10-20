//! `kentos-jobrun <folder>`: the process that runs one İşlemler tool for a server job (docs/adr/0559 §3), started by the
//! server's worker (`kentosd`'s processor) with an empty environment. It carries the headless host and the tools so the
//! server does not; it has no database and no network of its own, and reads and writes one folder:
//!
//! - in: `request.json` (a `ProcessingJobInput`) and `snapshot.kcad` (the project at the revision the worker read);
//! - out: `answer.json` (what the tool did: a summary, notes, the delta to commit or the rows of the file) and
//!   `sink.gpkg` (the GeoPackage of a `sink` run), or `failure.json` (why it did not finish) with exit code 2;
//! - while it works it prints its progress on standard output, one line at a time (`KENTOS-PROGRESS 0.4000 words`).
//!
//! The tool is run as `cad.processing.run` would run it, no local file readable (`forbid_local_files`) and no path of
//! the request's own for the result file. When its parent is gone (standard input closes) it ends itself with 3.

#![forbid(unsafe_code)]

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use kentos_contracts::processing_job::{ProcessFailure, Processed, check_input, delta};
use kentos_contracts::{ProcessingJobInput, ProcessingJobMode, ProcessingJobNote};
use serde_json::{Value, json};

/// What a progress line starts with.
const PROGRESS_PREFIX: &str = "KENTOS-PROGRESS ";
const SNAPSHOT: &str = "snapshot.kcad";
const REQUEST: &str = "request.json";
const ANSWER: &str = "answer.json";
const FAILURE: &str = "failure.json";
const SINK: &str = "sink.gpkg";

fn say(fraction: f32, words: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{PROGRESS_PREFIX}{fraction:.4} {words}");
    let _ = out.flush();
}

/// How a refusal of `cad.processing.run` reads as a job's failure.
fn failure_of_answer(answer: &Value) -> ProcessFailure {
    let error = &answer["error"];
    let code = error["code"].as_str().unwrap_or("tool_failed");
    let message = error["message"]
        .as_str()
        .unwrap_or("Araç çalışmadı.")
        .to_owned();
    let code = match code {
        "tools_unavailable" => "tool_unavailable",
        "unknown_tool" | "invalid_uid" | "entity_not_found" | "invalid_parameters"
        | "invalid_input" => "invalid",
        _ => "tool_failed",
    };
    ProcessFailure::new(code, message)
}

/// The tool run in this process; what it did.
fn run_task(dir: &Path) -> Result<Processed, ProcessFailure> {
    let invalid = |m: String| ProcessFailure::new("invalid", m);
    let request = std::fs::read(dir.join(REQUEST))
        .map_err(|e| invalid(format!("İşin isteği okunamadı: {e}")))?;
    let input: ProcessingJobInput = serde_json::from_slice(&request)
        .map_err(|e| invalid(format!("İşin isteği okunamadı: {e}")))?;
    if let Some((path, message)) = check_input(&input) {
        return Err(invalid(format!("{path}: {message}")));
    }
    kentos_processing::command::register_as("server");
    kentos_processing::command::forbid_local_files();
    say(0.05, "Proje belleğe alınıyor");
    let bytes = std::fs::read(dir.join(SNAPSHOT))
        .map_err(|e| invalid(format!("Projenin dosyası okunamadı: {e}")))?;
    let mut session = kentos_headless::Session::from_bytes(&bytes)
        .map_err(|e| invalid(format!("Proje açılamadı: {e}")))?;
    drop(bytes);
    let before =
        (input.mode == ProcessingJobMode::Commit).then(|| session.document().to_snapshot_v2());
    let mut run = json!({ "tool": input.tool });
    if !input.parameters.is_empty() {
        run["parameters"] = json!(input.parameters);
    }
    if !input.selection.is_empty() {
        run["selection"] = json!(input.selection);
    }
    if let Some(view) = input.view {
        run["view"] = json!(view);
    }
    if input.mode == ProcessingJobMode::Sink {
        run["sink"] = json!({ "path": dir.join(SINK).display().to_string() });
    }
    let name = input.label.clone().unwrap_or_else(|| input.tool.clone());
    say(0.10, &format!("Araç çalışıyor: {name}"));
    let started = session
        .start(json!({ "command": "cad.processing.run", "op": "execute", "input": run }))
        .map_err(|e| ProcessFailure::new("tool_failed", e.to_string()))?;
    let id = started["id"].as_str().unwrap_or_default().to_owned();
    let status = loop {
        let status = session
            .job(&id)
            .map_err(|e| ProcessFailure::new("tool_failed", e.to_string()))?;
        if let Some(p) = status["progress"].as_f64() {
            say(
                (0.10 + 0.75 * p.clamp(0.0, 1.0)) as f32,
                &format!("Araç çalışıyor: {name}"),
            );
        }
        if matches!(
            status["phase"].as_str(),
            Some("done" | "failed" | "conflict" | "cancelled")
        ) {
            break status;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let answer = &status["answer"];
    if status["phase"].as_str() != Some("done") || answer["status"].as_str() != Some("completed") {
        return Err(failure_of_answer(answer));
    }
    let output = &answer["output"];
    let notes = output["notes"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|n| {
                    Some(ProcessingJobNote {
                        level: n["level"].as_str()?.to_owned(),
                        text: n["text"].as_str()?.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let mut processed = Processed {
        summary: output["summary"].as_str().unwrap_or_default().to_owned(),
        notes,
        ..Processed::default()
    };
    match before {
        Some(before) => {
            say(0.88, "Değişiklikler bulunuyor");
            let after = session.document().to_snapshot_v2();
            processed.delta = Some(delta(&before, &after)?);
        }
        None => {
            processed.sink_rows = output["sink"]["tables"]
                .as_array()
                .map(|tables| {
                    tables
                        .iter()
                        .filter_map(|t| t["rows"].as_u64())
                        .sum::<u64>()
                })
                .unwrap_or(0);
            if !dir.join(SINK).is_file() {
                return Err(ProcessFailure::new(
                    "tool_failed",
                    "Araç dosyaya yazılacak bir nesne üretmedi.",
                ));
            }
        }
    }
    say(0.92, "Sonuç hazırlanıyor");
    Ok(processed)
}

/// The child's whole life; the exit code (0: answered, 1: the answer could not be written, 2: failed with a reason in
/// the folder, 3: the parent went away).
fn main() {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next() else {
        eprintln!("kentos-jobrun <klasör>: klasör verilmedi");
        std::process::exit(64);
    };
    let dir = Path::new(&dir);
    // The parent holds this process's standard input open; when the parent is gone it closes and the job is nobody's.
    std::thread::spawn(|| {
        let mut sink = Vec::new();
        let _ = std::io::stdin().lock().read_to_end(&mut sink);
        std::process::exit(3);
    });
    let code = match run_task(dir) {
        Ok(processed) => match serde_json::to_vec(&processed)
            .map_err(|e| e.to_string())
            .and_then(|b| std::fs::write(dir.join(ANSWER), b).map_err(|e| e.to_string()))
        {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Cevap yazılamadı: {e}");
                1
            }
        },
        Err(failure) => {
            let _ = std::fs::write(
                dir.join(FAILURE),
                serde_json::to_vec(&failure).unwrap_or_default(),
            );
            2
        }
    };
    std::process::exit(code);
}
