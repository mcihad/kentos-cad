//! The journal written out and replayed (docs/adr/0253 §3).
//!
//! [`journal_of`] writes the document's journal as `kentos.replay/1`: the
//! drawing's unit, system and type, and its rows. [`replay`] runs a
//! journal's command rows again, in order, on another document, through the
//! protocol (`call::call`): an id an earlier row made is mapped to the id
//! the replay made in its place (a row's `created`, in order), so a later
//! row names the right object. Each row is `same` (it completed with as many
//! objects made, changed and removed, and the same warnings' codes),
//! `differs` (it completed, something of that is not the same), `failed`
//! (refused, with its error) or `skipped` (an edit made without a command,
//! a row undone since, a row kept without its input). A row that wrote a
//! file is not written again: the host that runs the file commands plans it
//! and compares ([`replay_with`]; the headless host's
//! `file_commands::replay_row`); without one it is `skipped`.

use std::collections::BTreeMap;

use kentos_contracts::{
    CallEnvelope, CallKind, CallOp, CallRecord, CallStatus, CommandResult, REPLAY_FORMAT,
    REPLAY_VERSION, ReplayJournal, ReplayOutcome, ReplayReport, ReplayStep,
};
use kentos_domain::Document;
use kentos_geometry_core::store::Store;

use crate::call::{call, context_of, rename_texts};

/// The document's journal, to be replayed elsewhere.
pub fn journal_of(doc: &Document) -> ReplayJournal {
    ReplayJournal {
        format: REPLAY_FORMAT.into(),
        version: REPLAY_VERSION,
        context: context_of(doc),
        calls: doc.journal().cloned().collect(),
    }
}

fn skipped(row: &CallRecord, note: &str) -> ReplayStep {
    ReplayStep {
        seq: row.seq,
        command: row.command.clone(),
        outcome: ReplayOutcome::Skipped,
        note: Some(note.to_owned()),
        error: None,
        created: Vec::new(),
    }
}

/// The codes of a list of warnings, sorted.
fn codes_of(warnings: &[kentos_contracts::CommandWarning]) -> Vec<String> {
    let mut codes: Vec<String> = warnings.iter().map(|w| w.code.clone()).collect();
    codes.sort();
    codes
}

/// Runs `journal`'s command rows on `doc` (see the module); a row that
/// wrote a file is `skipped`.
pub fn replay(doc: &mut Document, store: Option<&Store>, journal: &ReplayJournal) -> ReplayReport {
    replay_with(doc, store, journal, &mut |_, _| None)
}

/// [`replay`], a row that wrote a file given to `files`: the host plans the
/// file again on `doc` as the replay has it, writes nothing and says whether
/// it is the same file (none: `skipped`). Its ids are mapped as the rows' are.
pub fn replay_with(
    doc: &mut Document,
    store: Option<&Store>,
    journal: &ReplayJournal,
    files: &mut dyn FnMut(&mut Document, &CallRecord) -> Option<ReplayStep>,
) -> ReplayReport {
    let mut report = ReplayReport::default();
    for row in &journal.calls {
        let step = if row.kind != CallKind::Command {
            skipped(row, "Komutla yazılmamış bir düzenleme; yeniden oynatılmaz.")
        } else if row.status == CallStatus::Undone {
            skipped(row, "Geri alınmış; yeniden oynatılmaz.")
        } else if !row.effects.is_empty() {
            let mut row = row.clone();
            if let Some(input) = row.input.as_mut() {
                rename_texts(input, &report.mapped);
            }
            files(doc, &row).unwrap_or_else(|| {
                skipped(
                    &row,
                    "Dosyaya yazdı; tekrar oynatma dosyayı yeniden yazmaz.",
                )
            })
        } else {
            match (&row.command, &row.input) {
                (Some(command), Some(input)) => {
                    let mut input = input.clone();
                    rename_texts(&mut input, &report.mapped);
                    run_row(doc, store, row, command, input, &mut report.mapped)
                }
                _ => skipped(
                    row,
                    "Girdisi kayıtta yok (1 MiB'tan büyüktü); yeniden oynatılamaz.",
                ),
            }
        };
        match step.outcome {
            ReplayOutcome::Same => report.same += 1,
            ReplayOutcome::Differs => report.differs += 1,
            ReplayOutcome::Failed => report.failed += 1,
            ReplayOutcome::Skipped => report.skipped += 1,
        }
        report.steps.push(step);
    }
    report
}

fn run_row(
    doc: &mut Document,
    store: Option<&Store>,
    row: &CallRecord,
    command: &str,
    input: serde_json::Value,
    mapped: &mut BTreeMap<String, String>,
) -> ReplayStep {
    let answer = call(
        doc,
        store,
        CallEnvelope {
            relax_constraints: row.relax_constraints,
            command: command.to_owned(),
            version: row.version,
            op: CallOp::Execute,
            input,
            request_id: None,
            expected: None,
        },
    );
    let mut step = ReplayStep {
        seq: row.seq,
        command: Some(command.to_owned()),
        outcome: ReplayOutcome::Same,
        note: None,
        error: None,
        created: Vec::new(),
    };
    match answer.result {
        CommandResult::Completed { warnings, .. } => {
            let summary = answer.summary.unwrap_or_default();
            step.created.clone_from(&summary.added);
            for (old, new) in row.created.iter().zip(&summary.added) {
                mapped.insert(old.clone(), new.clone());
            }
            let mut differences = Vec::new();
            for (what, then, now) in [
                ("yapılan", row.created.len(), summary.added.len()),
                ("değişen", row.changed.len(), summary.changed.len()),
                ("silinen", row.removed.len(), summary.removed.len()),
            ] {
                if then != now && !row.truncated {
                    differences.push(format!("{what} nesne {then} yerine {now}"));
                }
            }
            if codes_of(&row.warnings) != codes_of(&warnings) {
                differences.push(format!(
                    "uyarılar {:?} yerine {:?}",
                    codes_of(&row.warnings),
                    codes_of(&warnings)
                ));
            }
            if !differences.is_empty() {
                step.outcome = ReplayOutcome::Differs;
                step.note = Some(differences.join("; "));
            }
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            step.outcome = ReplayOutcome::Failed;
            step.error = Some(error);
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => {
            step.outcome = ReplayOutcome::Failed;
            step.note = Some("Komut yazmadı.".into());
        }
    }
    step
}
