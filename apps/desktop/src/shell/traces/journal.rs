//! A trace's journal (docs/adr/0253 §3): the drawing's record of what was
//! written, as the trace left it. `edits` is how many rows are edits made
//! without a product command (none: every write of the trace went through a
//! command, CAD-CMD-06); with `replays`, the record runs again on the
//! trace's drawing opened anew, every command row `same`, and gives the same
//! drawing: every object as the app has it, the ids the replay made in
//! place of the app's.

use std::collections::BTreeMap;

use kentos_commands::{CallKind, CallStatus, DocumentSnapshotV1, ReplayOutcome};
use kentos_domain::{Document, Slot};
use kentos_native_application::replay::{journal_of, replay};
use serde_json::Value;

use super::folder;
use super::format::{JournalExpect, Trace};
use crate::app::App;

/// What of `want` does not hold of the app's drawing.
pub(super) fn problems(app: &App, trace: &Trace, want: &JournalExpect) -> Vec<String> {
    let Some(doc) = app.document.as_ref().map(|d| &d.model) else {
        return vec!["journal: çizim yok".to_owned()];
    };
    let mut bad = Vec::new();
    let edits: Vec<String> = doc
        .journal()
        .filter(|r| r.kind == CallKind::Edit)
        .map(|r| r.label.clone().unwrap_or_default())
        .collect();
    if let Some(n) = want.edits
        && edits.len() != n
    {
        bad.push(format!(
            "journal.edits: {} ({edits:?}), beklenen {n}",
            edits.len()
        ));
    }
    let commands = doc
        .journal()
        .filter(|r| r.kind == CallKind::Command && r.status == CallStatus::Done)
        .count();
    if let Some(n) = want.commands
        && commands != n
    {
        bad.push(format!("journal.commands: {commands}, beklenen {n}"));
    }
    if want.replays {
        bad.extend(replayed(doc, trace));
    }
    bad
}

/// The record replayed on the trace's drawing opened anew: what differs.
fn replayed(doc: &Document, trace: &Trace) -> Vec<String> {
    let opened = std::fs::read_to_string(folder().join(&trace.document))
        .map_err(|e| e.to_string())
        .and_then(|text| DocumentSnapshotV1::from_json(&text))
        .and_then(Document::from_snapshot);
    let mut copy = match opened {
        Ok(d) => d,
        Err(e) => return vec![format!("journal: {}: {e}", trace.document)],
    };
    let journal = journal_of(doc);
    let report = replay(&mut copy, None, &journal);
    let mut bad: Vec<String> = report
        .steps
        .iter()
        .filter(|s| matches!(s.outcome, ReplayOutcome::Differs | ReplayOutcome::Failed))
        .map(|s| {
            format!(
                "journal: satır {} ({}) {:?}: {}",
                s.seq,
                s.command.as_deref().unwrap_or("?"),
                s.outcome,
                s.error
                    .as_ref()
                    .map_or_else(|| s.note.clone().unwrap_or_default(), |e| e.message.clone())
            )
        })
        .collect();
    // The same drawing: every object of the app, by the id the replay made in its place.
    let objects = |d: &Document| -> BTreeMap<String, Value> {
        d.entities()
            .filter_map(|e| {
                let uid = d.uid(Slot(e.base().id))?;
                Some((uid.to_string(), kentos_domain::etag::entity_value(e)))
            })
            .collect()
    };
    let mine = objects(doc);
    let theirs = objects(&copy);
    if mine.len() != theirs.len() {
        bad.push(format!(
            "journal: tekrar oynatılan çizimde {} nesne, uygulamada {}",
            theirs.len(),
            mine.len()
        ));
    }
    for (uid, entity) in &mine {
        let there = report.mapped.get(uid).unwrap_or(uid);
        let mut entity = entity.clone();
        rename(&mut entity, &report.mapped);
        match theirs.get(there) {
            Some(e) if *e == entity => {}
            Some(e) => bad.push(format!("journal: {uid} farklı: {e} ≠ {entity}")),
            None => bad.push(format!("journal: {uid} tekrar oynatılan çizimde yok")),
        }
    }
    bad
}

/// `v` with every id the replay made anew written as the replay's.
fn rename(v: &mut Value, names: &BTreeMap<String, String>) {
    match v {
        Value::String(s) => {
            if let Some(n) = names.get(s.as_str()) {
                s.clone_from(n);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|i| rename(i, names)),
        Value::Object(map) => map.values_mut().for_each(|i| rename(i, names)),
        _ => {}
    }
}
