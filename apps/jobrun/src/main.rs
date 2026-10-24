//! `kentos-jobrun <folder>`: the process that runs one server job's work (docs/adr/0559 §3, 0561 §3), started by the
//! server's worker (`kentosd`'s processor) with an empty environment. It carries the headless host, the tools and the
//! file commands so the server does not; it has no database and no network of its own, and reads and writes one folder:
//!
//! - in: `request.json` (a `RunRequest`: an İşlemler tool, an import of the file `import.<ext>` beside it, or an atlas)
//!   and `snapshot.kcad` (the project at the revision the worker read);
//! - out: `answer.json` (what was done: a summary, notes, the delta to commit or the rows of the file, what the command
//!   answered), `sink.gpkg` (a tool's file) or `atlas.pdf`; or `failure.json` (why it did not finish) with exit code 2;
//! - while it works it prints its progress on standard output, one line at a time (`KENTOS-PROGRESS 0.4000 words`).
//!
//! A tool is run as `cad.processing.run` would run it, no local file readable (`forbid_local_files`) and no path of the
//! request's own for the result file; an import and an atlas are run by the same commands as the desktop's
//! (`file.import.*`, `file.export.atlas`), the file's path the folder's. When its parent is gone (standard input
//! closes) it ends itself with 3.

#![forbid(unsafe_code)]

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use kentos_contracts::processing_job::{
    ImportRun, ProcessFailure, Processed, RunRequest, check_atlas_input, check_input, delta,
};
use kentos_contracts::{
    ProcessingJobInput, ProcessingJobMode, ProcessingJobNote, ProcessingUsage, ProjectAtlasInput,
};
use serde_json::{Value, json};

/// What a progress line starts with.
const PROGRESS_PREFIX: &str = "KENTOS-PROGRESS ";
const SNAPSHOT: &str = "snapshot.kcad";
const REQUEST: &str = "request.json";
const ANSWER: &str = "answer.json";
const FAILURE: &str = "failure.json";
const SINK: &str = "sink.gpkg";
const ATLAS: &str = "atlas.pdf";

fn say(fraction: f32, words: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{PROGRESS_PREFIX}{fraction:.4} {words}");
    let _ = out.flush();
}

fn invalid(message: String) -> ProcessFailure {
    ProcessFailure::new("invalid", message)
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

/// How a refusal of an import or atlas command reads as a job's failure: the command's own code and words.
fn failure_of_command(answer: &Value) -> ProcessFailure {
    let error = &answer["error"];
    let code = error["code"].as_str().unwrap_or("failed");
    let message = error["message"].as_str().unwrap_or("Komut çalışmadı.");
    let at = error["path"]
        .as_str()
        .map(|p| format!(" ({p})"))
        .unwrap_or_default();
    ProcessFailure::new(code, format!("{message}{at}"))
}

/// A command's warnings as notes: the code in front of the words.
fn warnings_of(answer: &Value) -> Vec<ProcessingJobNote> {
    answer["warnings"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|w| {
                    Some(ProcessingJobNote {
                        level: "warn".to_owned(),
                        text: format!("{}: {}", w["code"].as_str()?, w["message"].as_str()?),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn open_project(dir: &Path) -> Result<kentos_headless::Session, ProcessFailure> {
    say(0.05, "Proje belleğe alınıyor");
    let bytes = std::fs::read(dir.join(SNAPSHOT))
        .map_err(|e| invalid(format!("Projenin dosyası okunamadı: {e}")))?;
    let session = kentos_headless::Session::from_bytes(&bytes)
        .map_err(|e| invalid(format!("Proje açılamadı: {e}")))?;
    drop(bytes);
    Ok(session)
}

/// An İşlemler tool run in this process; what it did.
fn run_tool(dir: &Path, input: ProcessingJobInput) -> Result<Processed, ProcessFailure> {
    if let Some((path, message)) = check_input(&input) {
        return Err(invalid(format!("{path}: {message}")));
    }
    kentos_processing::command::register_as("server");
    kentos_processing::command::forbid_local_files();
    let mut session = open_project(dir)?;
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
            processed.rows = output["sink"]["tables"]
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

/// A staged file taken in by the format's import command; what came in.
fn run_import(dir: &Path, run: ImportRun) -> Result<Processed, ProcessFailure> {
    let ImportRun {
        format,
        name,
        options,
    } = run;
    if options.contains_key("path") {
        return Err(invalid(
            "input.options.path: dosyanın yolunu sunucu verir.".into(),
        ));
    }
    // A file of the server's is never named by a request: the rows refused are counted in the job's result.
    if options.contains_key("rejects") {
        return Err(invalid(
            "input.options.rejects: reddedilen satırların dosyası sunucuda yazılmaz; reddedilenler işin sonucunda sayılır."
                .into(),
        ));
    }
    let file = dir.join(format!("import.{}", format.extension()));
    if !file.is_file() {
        return Err(invalid(format!("“{name}” dosyası işin klasöründe yok.")));
    }
    let mut session = open_project(dir)?;
    let before = session.document().to_snapshot_v2();
    let mut input = serde_json::Map::new();
    for (key, value) in options {
        input.insert(key, value);
    }
    input.insert("path".into(), json!(file.display().to_string()));
    say(0.10, &format!("Dosya okunuyor: {name}"));
    let answer = session
        .call(json!({ "command": format.command(), "op": "execute", "input": input }))
        .map_err(|e| ProcessFailure::new("failed", e.to_string()))?;
    if answer["status"].as_str() != Some("completed") {
        return Err(failure_of_command(&answer));
    }
    say(0.80, "Değişiklikler bulunuyor");
    let after = session.document().to_snapshot_v2();
    let delta = delta(&before, &after)?;
    let layers = delta
        .project
        .as_ref()
        .and_then(|p| p.layers.as_ref())
        .map_or(0, |after_layers| {
            let count = |tree: &[kentos_contracts::LayerNode]| {
                fn walk(nodes: &[kentos_contracts::LayerNode]) -> usize {
                    nodes.iter().map(|n| 1 + walk(&n.children)).sum()
                }
                walk(tree)
            };
            count(after_layers).saturating_sub(count(&before.layers))
        });
    let summary = format!(
        "“{name}” içe aktarıldı: {} nesne eklendi{}{}.",
        delta.added,
        if layers > 0 {
            format!(", {layers} katman açıldı")
        } else {
            String::new()
        },
        if delta.blocks.is_empty() {
            String::new()
        } else {
            format!(", {} blok tanımı geldi", delta.blocks.len())
        },
    );
    let notes = warnings_of(&answer);
    say(0.92, "Sonuç hazırlanıyor");
    Ok(Processed {
        summary,
        notes,
        delta: Some(delta),
        details: answer["output"].clone(),
        ..Processed::default()
    })
}

/// The atlas of a sheet written as `atlas.pdf`, by the same command as the desktop's.
fn run_atlas(dir: &Path, input: ProjectAtlasInput) -> Result<Processed, ProcessFailure> {
    if let Some((path, message)) = check_atlas_input(&input) {
        return Err(invalid(format!("{path}: {message}")));
    }
    let mut session = open_project(dir)?;
    let mut command = serde_json::Map::new();
    command.insert("path".into(), json!(dir.join(ATLAS).display().to_string()));
    command.insert("sheet".into(), json!(input.sheet));
    command.insert("overwrite".into(), json!(true));
    if let Some(fields) = &input.fields {
        command.insert("fields".into(), json!(fields));
    }
    if let Some(theme) = &input.theme {
        command.insert("theme".into(), json!(theme));
    }
    if let Some(time) = &input.time {
        command.insert("time".into(), json!(time));
    }
    if let Some(crs) = input.crs {
        command.insert("crs".into(), json!(crs));
    }
    if let Some(pin) = &input.pin {
        command.insert("pin".into(), json!(pin));
    }
    say(0.10, "Atlasın sayfaları çiziliyor");
    let answer = session
        .call(json!({ "command": "file.export.atlas", "op": "execute", "input": command }))
        .map_err(|e| ProcessFailure::new("failed", e.to_string()))?;
    if answer["status"].as_str() != Some("completed") {
        return Err(failure_of_command(&answer));
    }
    let output = answer["output"].clone();
    let pages = output["pages"].as_array().map_or(0, Vec::len);
    let bytes = output["bytes"].as_u64().unwrap_or(0);
    let mut notes = warnings_of(&answer);
    if let Some(a) = output["notes"].as_array() {
        notes.extend(a.iter().filter_map(|n| {
            Some(ProcessingJobNote {
                level: "info".to_owned(),
                text: n.as_str()?.to_owned(),
            })
        }));
    }
    if let Some(a) = output["findings"].as_array() {
        notes.extend(a.iter().map(|f| {
            ProcessingJobNote {
                level: "info".to_owned(),
                text: f["message"]
                    .as_str()
                    .or_else(|| f["text"].as_str())
                    .map_or_else(|| f.to_string(), str::to_owned),
            }
        }));
    }
    if !dir.join(ATLAS).is_file() {
        return Err(ProcessFailure::new("failed", "Atlas PDF'i yazılmadı."));
    }
    say(0.92, "Sonuç hazırlanıyor");
    Ok(Processed {
        summary: format!("{pages} sayfalık atlas yazıldı ({bytes} bayt)."),
        notes,
        details: output,
        ..Processed::default()
    })
}

/// What this process has taken so far (docs/adr/0561 §5): where the system keeps it, the peak resident memory
/// (`VmHWM` of `/proc/self/status`) and the CPU time of `/proc/self/stat`; elsewhere what `ps` says of this process
/// (its resident memory now, which a run that does not give memory back has at its peak). The parent samples the child
/// while it runs; a run over between two samples is still told here, by the child itself, at its end.
fn own_usage() -> ProcessingUsage {
    let pid = std::process::id();
    if let (Ok(stat), Ok(status)) = (
        std::fs::read_to_string("/proc/self/stat"),
        std::fs::read_to_string("/proc/self/status"),
    ) {
        let cpu = stat.rfind(')').and_then(|i| {
            let fields: Vec<&str> = stat.get(i + 1..)?.split_whitespace().collect();
            let user: f64 = fields.get(11)?.parse().ok()?;
            let system: f64 = fields.get(12)?.parse().ok()?;
            Some((user + system) / 100.0)
        });
        let peak = status
            .lines()
            .find(|l| l.starts_with("VmHWM:"))
            .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok());
        return ProcessingUsage {
            cpu_seconds: cpu.unwrap_or(0.0),
            peak_memory_mb: peak.map_or(0.0, |kb| kb / 1024.0),
        };
    }
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=,time=", "-p", &pid.to_string()])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    let Ok(out) = out else {
        return ProcessingUsage::default();
    };
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut words = text.split_whitespace();
    let rss: f64 = words.next().and_then(|w| w.parse().ok()).unwrap_or(0.0);
    // `[dd-][hh:]mm:ss[.cc]`.
    let time = words.next().unwrap_or("");
    let (days, clock) = match time.split_once('-') {
        Some((d, rest)) => (d.parse::<f64>().unwrap_or(0.0), rest),
        None => (0.0, time),
    };
    let mut cpu = days * 86_400.0;
    for (i, part) in clock.rsplit(':').enumerate().take(3) {
        cpu += part.parse::<f64>().unwrap_or(0.0) * [1.0, 60.0, 3600.0][i];
    }
    ProcessingUsage {
        cpu_seconds: cpu,
        peak_memory_mb: rss / 1024.0,
    }
}

/// The work of the folder's request; what it did.
fn run_task(dir: &Path) -> Result<Processed, ProcessFailure> {
    let request = std::fs::read(dir.join(REQUEST))
        .map_err(|e| invalid(format!("İşin isteği okunamadı: {e}")))?;
    let request: RunRequest = serde_json::from_slice(&request)
        .map_err(|e| invalid(format!("İşin isteği okunamadı: {e}")))?;
    let mut processed = match request {
        RunRequest::Processing(input) => run_tool(dir, input),
        RunRequest::Import(run) => run_import(dir, run),
        RunRequest::Atlas(input) => run_atlas(dir, input),
    }?;
    processed.usage = own_usage();
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
    let dir = std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.into());
    let dir = dir.as_path();
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
