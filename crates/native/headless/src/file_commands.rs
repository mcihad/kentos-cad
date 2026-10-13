//! The commands that write files (docs/adr/0253 §10): `file.export.dxf`
//! and `file.export.pdf`, in every command's shape (`validate`, `plan`,
//! `execute`). They write no document: no undo step, and undo takes no file
//! back. `plan` makes the file's bytes and says what they would be, writing
//! nothing; `execute` writes them whole or not at all (`files::write_bytes`:
//! a temporary file beside the target, made durable, read back, renamed).
//! A write leaves a journal row with the file it wrote (its path, bytes and
//! SHA-256), and a request id is remembered as any command's.

use std::path::{Path, PathBuf};

use kentos_contracts::encoding::sha256_hex;
use kentos_contracts::{
    CallAnswer, CallEnvelope, CallOp, CallRecord, CommandError, CommandResult, DxfExported,
    ExportDxf, ExportPdf, FILE_COMMANDS, FILE_EXPORT_DXF, FILE_EXPORT_DXF_VERSION, FILE_EXPORT_PDF,
    FILE_EXPORT_PDF_VERSION, FileEffect, LayerNodeType, PdfExported, ReplayOutcome, ReplayStep,
    codes,
};
use kentos_domain::Document;
use kentos_native_application::call::{self, CallState};
use serde_json::Value;

use crate::files;

/// Whether `command` is one of these.
pub fn is_file_command(command: &str) -> bool {
    FILE_COMMANDS.iter().any(|(c, _)| *c == command)
}

/// Why a file command writes nothing: its input or the file refused
/// (`failed`). Boxed: a call is refused at most once.
type Refused = Box<CommandError>;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Refused {
    Box::new(CommandError {
        code: code.to_owned(),
        message: message.into(),
        path: Some(path.to_owned()),
        ..CommandError::default()
    })
}

/// The file's path: absolute, not a folder, in a folder that is there;
/// a file there only with `overwrite`. Whether one is there.
fn target(path: &str, overwrite: bool) -> Result<(PathBuf, bool), Refused> {
    let p = Path::new(path);
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{path}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.dxf)."
            ),
            "path",
        ));
    }
    if p.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("{path} bir klasör; yazılacak dosyanın adını da verin."),
            "path",
        ));
    }
    if !p.parent().is_some_and(Path::is_dir) {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("{path} dosyasının klasörü yok; önce klasörü açın ya da başka bir yol verin."),
            "path",
        ));
    }
    let exists = p.exists();
    if exists && !overwrite {
        return Err(refuse(
            codes::FILE_EXISTS,
            format!("{path} var; üzerine yazmak için overwrite: true verin."),
            "path",
        ));
    }
    Ok((p.to_path_buf(), exists))
}

/// The DXF's bytes and what they hold, or the refusal.
fn dxf(
    doc: &Document,
    input: &ExportDxf,
) -> Result<(PathBuf, bool, Vec<u8>, DxfExported), Refused> {
    let (path, exists) = target(&input.path, input.overwrite)?;
    let objects: Vec<&kentos_contracts::Entity> = match &input.layers {
        None => doc.entities().collect(),
        Some(ids) => {
            let mut chosen = Vec::new();
            for (i, id) in ids.iter().enumerate() {
                let Some(node) = doc.layers().get(id) else {
                    return Err(refuse(
                        codes::LAYER_NOT_FOUND,
                        format!(
                            "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                        ),
                        &format!("layers[{i}]"),
                    ));
                };
                // A group gives its layers.
                let leaves: Vec<String> = match node.kind {
                    LayerNodeType::Layer => vec![id.clone()],
                    LayerNodeType::Group => doc
                        .layers()
                        .leaves_of(id)
                        .iter()
                        .map(|n| n.id.clone())
                        .collect(),
                };
                for leaf in leaves {
                    chosen.extend(doc.by_layer(&leaf));
                }
            }
            chosen
        }
    };
    let grouped = crate::export::by_layer(doc, objects);
    if grouped.is_empty() {
        return Err(refuse(
            codes::NOTHING_TO_EXPORT,
            "Seçilen katmanlarda nesne yok: DXF'e yazılacak bir şey yok.",
            if input.layers.is_some() {
                "layers"
            } else {
                "path"
            },
        ));
    }
    let dxf_input = crate::export::dxf_input(doc, &grouped);
    let layers = u32::try_from(dxf_input.layers.len()).unwrap_or(u32::MAX);
    let (bytes, report) = kentos_formats::dxf::write(&dxf_input);
    let output = DxfExported {
        path: path.display().to_string(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        objects: report.counts.values().sum::<u32>(),
        layers,
        replaced: exists,
        report: serde_json::to_value(&report).unwrap_or(Value::Null),
    };
    Ok((path, exists, bytes, output))
}

/// The PDF's bytes and what they hold, or the refusal.
fn pdf(
    doc: &Document,
    input: &ExportPdf,
) -> Result<(PathBuf, bool, Vec<u8>, PdfExported), Refused> {
    let (path, exists) = target(&input.path, input.overwrite)?;
    if doc.sheets().is_none() {
        return Err(refuse(
            codes::NO_SHEETS,
            "Projede pafta yok: önce bir pafta ekleyin (cad.sheets.edit ya da Pafta kipi).",
            "path",
        ));
    }
    let Some(sheets) = kentos_native_application::sheets_edit::sheets_in(doc) else {
        return Err(refuse(
            codes::BOOK_UNREADABLE,
            "Projenin pafta kitabı pafta çekirdeğinin okuyamadığı bir kitap; olduğu gibi kalır. Kitabı masaüstünün Pafta kipinde açıp denetleyin.",
            "path",
        ));
    };
    if sheets.is_empty() {
        return Err(refuse(
            codes::NO_SHEETS,
            "Pafta kitabında pafta yok: önce bir pafta ekleyin.",
            "path",
        ));
    }
    let wanted = input.sheets.clone().unwrap_or_default();
    for (i, id) in wanted.iter().enumerate() {
        if !sheets.iter().any(|s| s.id == *id) {
            return Err(refuse(
                codes::UNKNOWN_SHEET,
                format!("“{id}” paftası kitapta yok."),
                &format!("sheets[{i}]"),
            ));
        }
    }
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let made = kentos_plot::book::book_pdf(doc, &wanted, &user, &kentos_plot::book::today())
        .map_err(|e| refuse(codes::PDF_UNWRITABLE, e.message, "path"))?;
    let output = PdfExported {
        path: path.display().to_string(),
        bytes: made.bytes.len() as u64,
        sha256: sha256_hex(&made.bytes),
        sheets: made.sheets,
        replaced: exists,
        findings: serde_json::to_value(&made.findings).unwrap_or(Value::Null),
        notes: made.notes,
    };
    Ok((path, exists, made.bytes, output))
}

/// Writes `bytes` to `path` whole (`files::write_bytes`), or the refusal.
fn write(path: &Path, bytes: &[u8]) -> Result<(), Refused> {
    files::write_bytes(bytes, path)
        .map(|_| ())
        .map_err(|e| refuse(codes::FILE_UNWRITABLE, e.message, "path"))
}

/// What a file command answers with, written or planned: its output as JSON.
struct Done {
    output: Value,
    effect: Option<FileEffect>,
}

/// Runs `op` of the file command `command`; its answer, and the file it
/// wrote (execute).
fn run_typed(
    doc: &Document,
    command: &str,
    op: CallOp,
    input: Value,
) -> Result<(CommandResult<Value>, Option<FileEffect>), String> {
    macro_rules! go {
        ($input:ty, $make:ident) => {{
            let typed: $input = serde_json::from_value(input).map_err(|e| {
                format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
            })?;
            let made = $make(doc, &typed);
            let done = match made {
                Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
                Ok((path, _, bytes, output)) => match op {
                    CallOp::Validate => Done {
                        output: Value::Null,
                        effect: None,
                    },
                    CallOp::Plan => Done {
                        output: serde_json::to_value(&output).unwrap_or(Value::Null),
                        effect: None,
                    },
                    CallOp::Execute => {
                        if let Err(refused) = write(&path, &bytes) {
                            return Ok((CommandResult::Failed { error: *refused }, None));
                        }
                        Done {
                            effect: Some(FileEffect {
                                path: output.path.clone(),
                                bytes: output.bytes,
                                sha256: output.sha256.clone(),
                            }),
                            output: serde_json::to_value(&output).unwrap_or(Value::Null),
                        }
                    }
                },
            };
            Ok((
                CommandResult::Completed {
                    output: done.output,
                    warnings: Vec::new(),
                },
                done.effect,
            ))
        }};
    }
    match command {
        FILE_EXPORT_DXF => go!(ExportDxf, dxf),
        FILE_EXPORT_PDF => go!(ExportPdf, pdf),
        other => Err(format!("“{other}” bir dosya komutu değil.")),
    }
}

fn version_of(command: &str) -> u32 {
    match command {
        FILE_EXPORT_PDF => FILE_EXPORT_PDF_VERSION,
        _ => FILE_EXPORT_DXF_VERSION,
    }
}

/// The input as its type writes it back, or the refusal.
fn normalize(command: &str, input: &Value) -> Result<Value, String> {
    let text = |e: serde_json::Error| {
        format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
    };
    match command {
        FILE_EXPORT_DXF => serde_json::from_value::<ExportDxf>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        _ => serde_json::from_value::<ExportPdf>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
    }
}

/// A file command by the protocol (docs/adr/0253 §4, §10): its call's
/// faults, its request, its answer; an execute's write in the journal.
pub fn call(doc: &mut Document, envelope: CallEnvelope) -> CallAnswer {
    let command = envelope.command.clone();
    let version = version_of(&command);
    let failed = |code: &str, message: String, path: &str| {
        CallAnswer::of(CommandResult::Failed {
            error: CommandError {
                code: code.to_owned(),
                message,
                path: Some(path.to_owned()),
                ..CommandError::default()
            },
        })
    };
    let input = match normalize(&command, &envelope.input) {
        Ok(v) => v,
        Err(message) => return failed(codes::INVALID_INPUT, message, "input"),
    };
    let state = match call::preflight(doc, &envelope, version, &input) {
        Ok(state) => state,
        Err(answer) => return *answer,
    };
    let mut answer = run_answer(doc, &command, version, envelope.op, &input, &state);
    call::complete(&mut answer, &input);
    answer
}

fn run_answer(
    doc: &mut Document,
    command: &str,
    version: u32,
    op: CallOp,
    input: &Value,
    state: &CallState,
) -> CallAnswer {
    match run_typed(doc, command, op, input.clone()) {
        Ok((result, effect)) => {
            let mut answer = CallAnswer::of(result);
            if let Some(effect) = effect {
                let call = call::record_effect(doc, command, version, input, vec![effect], state);
                answer.call = Some(call);
                if state.request().is_some()
                    && let Ok(text) = serde_json::to_string(&answer)
                {
                    doc.set_request_answer(call, text);
                }
            }
            answer
        }
        Err(message) => CallAnswer::of(CommandResult::Failed {
            error: CommandError {
                code: codes::INVALID_INPUT.into(),
                message,
                path: Some("input".into()),
                ..CommandError::default()
            },
        }),
    }
}

/// A file command run as a job (docs/adr/0253 §9): its file made and
/// written on a reading copy on another thread, its row recorded on the
/// document when the job is looked at.
pub fn job_work(
    envelope: CallEnvelope,
) -> impl FnOnce(
    Document,
    &kentos_native_application::jobs::Progress,
) -> kentos_native_application::jobs::JobOutcome
+ Send
+ 'static {
    move |copy, _progress| {
        use kentos_native_application::jobs::{Effect, JobOutcome};
        let command = envelope.command.clone();
        let version = version_of(&command);
        let input = match normalize(&command, &envelope.input) {
            Ok(v) => v,
            Err(message) => {
                return JobOutcome {
                    answer: CallAnswer::of(CommandResult::Failed {
                        error: CommandError {
                            code: codes::INVALID_INPUT.into(),
                            message,
                            path: Some("input".into()),
                            ..CommandError::default()
                        },
                    }),
                    copy: None,
                    effect: None,
                };
            }
        };
        let state = match call::preflight(&copy, &envelope, version, &input) {
            Ok(state) => state,
            Err(answer) => {
                return JobOutcome {
                    answer: *answer,
                    copy: None,
                    effect: None,
                };
            }
        };
        match run_typed(&copy, &command, CallOp::Execute, input.clone()) {
            Ok((result, effect)) => {
                let mut answer = CallAnswer::of(result);
                call::complete(&mut answer, &input);
                // The copy is not the document: the row is recorded there when the job is staged.
                JobOutcome {
                    answer,
                    copy: None,
                    effect: effect.map(|e| Effect {
                        command,
                        version,
                        input,
                        effects: vec![e],
                        state,
                    }),
                }
            }
            Err(message) => JobOutcome {
                answer: CallAnswer::of(CommandResult::Failed {
                    error: CommandError {
                        code: codes::INVALID_INPUT.into(),
                        message,
                        path: Some("input".into()),
                        ..CommandError::default()
                    },
                }),
                copy: None,
                effect: None,
            },
        }
    }
}

/// A file command's plain answer (`Session::run`, the `run` request): the
/// `CommandResult` as the catalog writes it; an execute's write in the journal.
pub fn run(doc: &mut Document, command: &str, op: CallOp, input: Value) -> Result<Value, String> {
    let normalized = normalize(command, &input)?;
    let state = CallState::default();
    let answer = run_answer(doc, command, version_of(command), op, &normalized, &state);
    serde_json::to_value(&answer.result).map_err(|e| e.to_string())
}

/// A journal row that wrote a file, replayed (docs/adr/0253 §10): the file
/// planned again on `doc` as the replay has it and not written; `same` when
/// its bytes' SHA-256 is the one written then, `differs` when not (a PDF
/// carries the day it is made), `skipped` when it cannot be planned here
/// (its folder is not on this machine, …), with the refusal.
pub fn replay_row(doc: &mut Document, row: &CallRecord) -> Option<ReplayStep> {
    let command = row.command.as_deref()?;
    let mut input = row.input.clone()?;
    // A plan writes nothing: the file written then may be there.
    input["overwrite"] = Value::Bool(true);
    let (answer, _) = run_typed(doc, command, CallOp::Plan, input).ok()?;
    let mut step = ReplayStep {
        seq: row.seq,
        command: Some(command.to_owned()),
        outcome: ReplayOutcome::Same,
        note: None,
        error: None,
        created: Vec::new(),
    };
    match answer {
        CommandResult::Completed { output, .. } => {
            let now = output["sha256"].as_str().unwrap_or_default();
            let then = row.effects.first().map_or("", |e| e.sha256.as_str());
            if now != then {
                step.outcome = ReplayOutcome::Differs;
                step.note = Some(format!(
                    "Dosya yazılmadı; yazılsaydı başka olurdu (SHA-256 {now}, kayıtta {then})."
                ));
            } else {
                step.note = Some("Dosya yazılmadı; yazılsaydı aynısı olurdu.".to_owned());
            }
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            step.outcome = ReplayOutcome::Skipped;
            step.note = Some(format!(
                "Dosya burada planlanamadı ({}): karşılaştırılmadı.",
                error.code
            ));
            step.error = Some(error);
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => return None,
    }
    Some(step)
}
