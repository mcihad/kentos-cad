//! `cad.arch.exceptions` (docs/adr/0732 §3): Mimari denetim's findings marked as left on purpose, the marks taken away,
//! or those whose finding is gone dropped. A finding is named by its lasting key as `cad.arch.audit` gives it for the
//! drawing as it is now; the exception keeps its code, objects and the digest of the finding (its code, objects, words
//! and place), so it lapses once the finding changes. The exceptions are the project's settings, written as one undo
//! step. The shared cases are `fixtures/commands/v1/cad.arch.exceptions.json` (`scripts/fixtures/arch_audit_cases.py`).

use kentos_commands::{
    ArchAudit, ArchAuditException, ArchExceptionOperation, ArchExceptions, ArchExceptionsEdited, CommandResult,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::arch_audit::{every_finding, finding_digest};
use crate::checks::{self, Stop, error};
use crate::codes;

/// The longest note, in characters.
const NOTE_MAX: usize = 1000;

/// The project's exceptions after the operation, and the keys it changed.
struct Checked {
    exceptions: Vec<ArchAuditException>,
    changed: Vec<String>,
}

fn now(doc: &Document) -> Vec<ArchAuditException> {
    doc.settings()
        .arch
        .as_ref()
        .map(|a| a.audit_exceptions.clone())
        .unwrap_or_default()
}

/// A failure of the audit's input, its path under `audit.`.
fn under_audit(stop: Stop) -> Stop {
    match stop {
        Stop::Failed(mut e) => {
            e.path = Some(match e.path.take() {
                Some(p) => format!("audit.{p}"),
                None => "audit".into(),
            });
            Stop::Failed(e)
        }
        other => other,
    }
}

fn check(doc: &Document, input: &ArchExceptions) -> Result<Checked, Stop> {
    let keyed = matches!(input.operation, ArchExceptionOperation::Mark | ArchExceptionOperation::Unmark);
    if keyed && input.keys.is_empty() {
        return Err(Stop::failed(error(
            codes::INVALID_INPUT,
            "Bulgu anahtarı verilmedi; cad.arch.audit'in verdiği anahtarlardan en az birini verin.".into(),
            Some("keys".into()),
        )));
    }
    for (i, k) in input.keys.iter().enumerate() {
        if input.keys[..i].contains(k) {
            return Err(Stop::failed(error(
                codes::INVALID_INPUT,
                format!("“{k}” anahtarı birden çok kez verildi."),
                Some(format!("keys[{i}]")),
            )));
        }
    }
    if let Some(note) = &input.note
        && (note.chars().count() > NOTE_MAX || note.chars().any(|c| c.is_control() && c != '\n'))
    {
        return Err(Stop::failed(error(
            codes::INVALID_INPUT,
            format!("Not en çok {NOTE_MAX} karakterdir, satır sonu dışında denetim karakteri olmaz."),
            Some("note".into()),
        )));
    }
    // The findings of the audit asked, every one of them, whatever part or exceptions it would list.
    let audit = ArchAudit {
        within: None,
        exceptions: None,
        ..input.audit.clone().unwrap_or_default()
    };
    let mut exceptions = now(doc);
    let mut changed = Vec::new();
    match input.operation {
        ArchExceptionOperation::Mark => {
            let (all, _) = every_finding(doc, &audit).map_err(under_audit)?;
            let mut chosen = Vec::with_capacity(input.keys.len());
            for (i, k) in input.keys.iter().enumerate() {
                let Some(f) = all.iter().find(|f| f.key == *k) else {
                    return Err(Stop::failed(error(
                        codes::FINDING_NOT_FOUND,
                        format!("Çizimin bugünkü denetiminde “{k}” anahtarlı bulgu yok; önce cad.arch.audit ile denetleyin."),
                        Some(format!("keys[{i}]")),
                    )));
                };
                chosen.push(f);
            }
            checks::revision(doc, input.expected_revision.as_deref())?;
            for f in chosen {
                let digest = finding_digest(f);
                match exceptions.iter_mut().find(|x| x.key == f.key) {
                    Some(x) => {
                        let note = input.note.clone().or_else(|| x.note.clone());
                        if x.digest == digest && x.note == note {
                            continue;
                        }
                        // A lapsed exception is marked again, on the finding as it is now.
                        x.digest = digest;
                        x.code = f.code.clone();
                        x.uids = f.uids.clone();
                        x.note = note;
                    }
                    None => exceptions.push(ArchAuditException {
                        key: f.key.clone(),
                        code: f.code.clone(),
                        uids: f.uids.clone(),
                        digest,
                        note: input.note.clone(),
                    }),
                }
                changed.push(f.key.clone());
            }
        }
        ArchExceptionOperation::Unmark => {
            for (i, k) in input.keys.iter().enumerate() {
                if !exceptions.iter().any(|x| x.key == *k) {
                    return Err(Stop::failed(error(
                        codes::FINDING_NOT_FOUND,
                        format!("“{k}” anahtarlı istisna yok."),
                        Some(format!("keys[{i}]")),
                    )));
                }
            }
            checks::revision(doc, input.expected_revision.as_deref())?;
            exceptions.retain(|x| !input.keys.contains(&x.key));
            changed = input.keys.clone();
        }
        ArchExceptionOperation::Prune => {
            let (all, run) = every_finding(doc, &audit).map_err(under_audit)?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            exceptions.retain(|x| {
                let gone = run.contains(x.code.as_str()) && !all.iter().any(|f| f.key == x.key);
                if gone {
                    changed.push(x.key.clone());
                }
                !gone
            });
        }
    }
    Ok(Checked { exceptions, changed })
}

fn edited(doc: &Document, checked: &Checked) -> ArchExceptionsEdited {
    ArchExceptionsEdited {
        changed: checked.changed.clone(),
        exceptions: checked.exceptions.len().min(u32::MAX as usize) as u32,
        revision: doc.revision().to_string(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ArchExceptions) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ArchExceptions) -> CommandResult<ArchExceptionsEdited> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: edited(cx.doc, &c),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ArchExceptions) -> CommandResult<ArchExceptionsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ARCH_EXCEPTIONS,
        kentos_commands::CAD_ARCH_EXCEPTIONS_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the document (docs/adr/0253 §1, §3).
fn write_execute(cx: &mut ExecutionContext<'_>, input: ArchExceptions) -> CommandResult<ArchExceptionsEdited> {
    let checked = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let label = match input.operation {
        ArchExceptionOperation::Mark => "İstisna",
        ArchExceptionOperation::Unmark => "İstisnayı kaldır",
        ArchExceptionOperation::Prune => "İstisnaları temizle",
    };
    if !checked.changed.is_empty()
        && let Err(e) = cx.doc.set_audit_exceptions(checked.exceptions.clone(), label)
    {
        return CommandResult::Failed {
            error: error(codes::INVALID_INPUT, e.0, Some("keys".into())),
        };
    }
    CommandResult::Completed {
        output: edited(cx.doc, &checked),
        warnings: Vec::new(),
    }
}
