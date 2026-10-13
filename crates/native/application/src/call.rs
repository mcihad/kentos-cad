//! The command protocol (docs/adr/0253): one path for every write.
//!
//! - [`recorded`] wraps every command's `execute`: the write runs as one
//!   step of the document (`Document::run_step`), all or nothing, parts
//!   outside the undo history too; the step keeps the document's integrity
//!   (§8) and leaves a journal row (`kentos.calls/1`, §3). A command run by
//!   another (an edit inside Ötele, a change set's step) is that one's write.
//! - [`call`] is the protocol itself (§4–§7): a [`CallEnvelope`] in, a
//!   [`CallAnswer`] out. It checks what the call expects (the revision, the
//!   stamps), answers a request sent again from the ledger, plans by
//!   executing on a reading copy, writes with the plan it must equal, and
//!   says what was touched ([`CommandSummary`]).

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::canonical::{canonical_json, id_of};
use kentos_contracts::{
    AlgorithmInfo, Bounds, CallAnswer, CallContext, CallEnvelope, CallExpectations, CallKind,
    CallOp, CallRecord, CallStatus, CommandError, CommandResult, CommandSummary, CommandWarning,
    FileEffect, JOURNAL_IDS, JOURNAL_INPUT_LIMIT, NUMERIC_POLICY, NodeChanges, ProjectPart,
    RequestRecord, StateStamps, SummaryCounts, UndoInfo, codes, request_id_valid,
};
use kentos_domain::etag::{block_stamp, entity_stamp, entity_value};
use kentos_domain::{Document, Sealed, StepReport, Uuid};
use kentos_geometry_core::entity::entity_bounds;
use kentos_geometry_core::store::Store;
use serde::Serialize;
use serde_json::{Value, json};

use crate::ExecutionContext;
use crate::geometry::shape;
use crate::jobs::Progress;
use crate::registry;

/// The build that computed a result: this crate and its version.
pub const CORE: &str = concat!("kentos-native-application/", env!("CARGO_PKG_VERSION"));

/// What the protocol asks of the command it calls, on the context
/// (`ExecutionContext::call`); the write's outcome comes back in it.
#[derive(Clone, Debug, Default)]
pub struct CallState {
    /// The request's id and the call's fingerprint.
    pub(crate) request: Option<(String, String)>,
    /// The plan the write must equal.
    pub(crate) plan: Option<String>,
    /// Whether the caller wants the summary and the plan's id.
    pub(crate) summary: bool,
    pub(crate) outcome: Option<Outcome>,
}

impl CallState {
    /// The request's id and the call's fingerprint, when the call named a request.
    pub fn request(&self) -> Option<&(String, String)> {
        self.request.as_ref()
    }
}

/// What a write gave the protocol.
#[derive(Clone, Debug)]
pub(crate) struct Outcome {
    pub(crate) summary: CommandSummary,
    pub(crate) plan_id: String,
    pub(crate) call: Option<u64>,
}

/// The stamps of the project's parts, before a write.
#[derive(Clone, Debug)]
struct PartStamps {
    layers: String,
    settings: String,
    styles: String,
    sheets: String,
}

impl PartStamps {
    fn of(doc: &Document) -> Self {
        Self {
            layers: doc.layers_stamp(),
            settings: doc.settings_stamp(),
            styles: doc.styles_stamp(),
            sheets: doc.sheets_stamp(),
        }
    }
}

/// Runs `write` (a command's execute) as one recorded step: see the module.
/// A command run inside another's step is that one's write and runs as it is.
pub(crate) fn recorded<I, O>(
    cx: &mut ExecutionContext<'_>,
    command: &'static str,
    version: u32,
    input: I,
    write: impl FnOnce(&mut ExecutionContext<'_>, I) -> CommandResult<O>,
) -> CommandResult<O>
where
    I: Serialize,
{
    if cx.doc.in_step() {
        return write(cx, input);
    }
    let mut state = cx.call.take().unwrap_or_default();
    let normalized = serde_json::to_value(&input).unwrap_or(Value::Null);
    let base = cx.doc.revision();
    let parts = (state.summary || state.plan.is_some()).then(|| PartStamps::of(cx.doc));
    let (store, job) = (cx.store, cx.job);
    let mut outcome: Option<Outcome> = None;
    let result = cx.doc.run_step(
        |doc| {
            let mut inner = ExecutionContext {
                doc,
                store,
                call: None,
                job,
            };
            match write(&mut inner, input) {
                CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                other => Err(Box::new(other)),
            }
        },
        |doc, report, (_, warnings)| {
            if report.outermost {
                let faults = doc.faults(report);
                if !faults.is_empty() {
                    return Err(Box::new(CommandResult::Failed {
                        error: CommandError {
                            code: codes::INTEGRITY_FAILED.into(),
                            message: format!(
                                "Yazma çizimi tutarsız bırakırdı; hiçbir şey yazılmadı: {}",
                                faults
                                    .iter()
                                    .map(|f| f.message.as_str())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            ),
                            uids: faults.iter().map(|f| f.uid.to_string()).collect(),
                            ..CommandError::default()
                        },
                    }));
                }
            }
            if let Some(parts) = &parts {
                let (summary, plan_id) =
                    summarize(doc, report, warnings, command, version, &normalized, base, parts);
                if let Some(expected) = &state.plan
                    && *expected != plan_id
                {
                    return Err(Box::new(CommandResult::Conflict {
                        error: CommandError {
                            code: codes::PLAN_CHANGED.into(),
                            message: "Yazılacak olan onaylanan plana eşit değil: kaynakları plandan sonra değişti; hiçbir şey yazılmadı. Yeniden planlayın.".into(),
                            path: Some("expected.plan".into()),
                            revision: Some(base.to_string()),
                            retryable: true,
                            ..CommandError::default()
                        },
                    }));
                }
                outcome = Some(Outcome {
                    summary,
                    plan_id,
                    call: None,
                });
            }
            let row = row_of(
                doc,
                report,
                command,
                version,
                &normalized,
                warnings,
                state.request.as_ref(),
                base,
            );
            let request = state.request.as_ref().map(|(id, fingerprint)| RequestRecord {
                request_id: id.clone(),
                fingerprint: fingerprint.clone(),
                command: command.to_owned(),
                version,
                revision: String::new(),
                status: CallStatus::Done,
                answer: None,
            });
            Ok(Some(Sealed { row, request }))
        },
    );
    let answer = match result {
        Ok(((output, warnings), report)) => {
            if let Some(o) = &mut outcome {
                o.call = report.call;
                o.summary.stamps.revision = Some(cx.doc.revision().to_string());
            }
            CommandResult::Completed { output, warnings }
        }
        Err(refused) => *refused,
    };
    state.outcome = outcome;
    cx.call = Some(state);
    answer
}

/// The persistent ids written in a value: every text that is one.
pub fn uids_in(value: &Value) -> BTreeSet<String> {
    fn walk(v: &Value, out: &mut BTreeSet<String>) {
        match v {
            Value::String(s) if kentos_contracts::uuid_from_text(s).is_some() => {
                out.insert(s.clone());
            }
            Value::Array(items) => items.iter().for_each(|i| walk(i, out)),
            Value::Object(map) => map.values().for_each(|i| walk(i, out)),
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    walk(value, &mut out);
    out
}

/// Where the drawing computes: its unit, system, type, plane.
pub fn context_of(doc: &Document) -> CallContext {
    let settings = doc.settings();
    let name = |v: Value| v.as_str().map(str::to_owned);
    CallContext {
        unit: name(serde_json::to_value(settings.unit()).unwrap_or(Value::Null))
            .unwrap_or_else(|| "m".into()),
        srid: settings.srid,
        workspace: settings
            .workspace
            .and_then(|w| name(serde_json::to_value(w).unwrap_or(Value::Null)))
            .unwrap_or_else(|| "unset".into()),
        plane: "world".into(),
    }
}

fn algorithm(command: &str, version: u32) -> AlgorithmInfo {
    AlgorithmInfo {
        command: command.to_owned(),
        version,
        policy: NUMERIC_POLICY.into(),
        core: CORE.into(),
    }
}

/// The ids of a step's objects added, changed and removed, each cut at
/// `JOURNAL_IDS`; whether one was.
fn id_lists(report: &StepReport) -> (Vec<String>, Vec<String>, Vec<String>, bool) {
    let (mut added, mut changed, mut removed) = (Vec::new(), Vec::new(), Vec::new());
    let mut truncated = false;
    for o in &report.objects {
        let list = if o.added() {
            &mut added
        } else if o.removed() {
            &mut removed
        } else if o.changed() {
            &mut changed
        } else {
            continue;
        };
        if list.len() < JOURNAL_IDS {
            list.push(o.uid.to_string());
        } else {
            truncated = true;
        }
    }
    (added, changed, removed, truncated)
}

#[allow(clippy::too_many_arguments)]
fn row_of(
    doc: &Document,
    report: &StepReport,
    command: &str,
    version: u32,
    input: &Value,
    warnings: &[CommandWarning],
    request: Option<&(String, String)>,
    base: u64,
) -> CallRecord {
    let text = canonical_json(input);
    let digest = kentos_contracts::encoding::sha256_hex(text.as_bytes())[..32].to_owned();
    let (created, changed, removed, truncated) = id_lists(report);
    CallRecord {
        seq: 0,
        kind: CallKind::Command,
        label: report.label.clone(),
        command: Some(command.to_owned()),
        version: Some(version),
        input: (text.len() <= JOURNAL_INPUT_LIMIT).then(|| input.clone()),
        input_digest: Some(digest),
        request_id: request.map(|(id, _)| id.clone()),
        fingerprint: request.map(|(_, f)| f.clone()),
        base: base.to_string(),
        revision: String::new(),
        context: Some(context_of(doc)),
        algorithm: Some(algorithm(command, version)),
        seed: None,
        created,
        changed,
        removed,
        truncated,
        warnings: warnings.to_vec(),
        parts: report.parts.clone(),
        effects: Vec::new(),
        step: None,
        status: CallStatus::Done,
    }
}

/// The box of an object, metres.
fn bounds_of(entity: &kentos_contracts::Entity) -> Option<Bounds> {
    let b = entity_bounds(&shape(entity));
    (b.min_x.is_finite() && b.max_x >= b.min_x).then_some(Bounds {
        min_x: b.min_x,
        min_y: b.min_y,
        max_x: b.max_x,
        max_y: b.max_y,
    })
}

fn union(a: Option<Bounds>, b: Option<Bounds>) -> Option<Bounds> {
    match (a, b) {
        (Some(a), Some(b)) => Some(Bounds {
            min_x: a.min_x.min(b.min_x),
            min_y: a.min_y.min(b.min_y),
            max_x: a.max_x.max(b.max_x),
            max_y: a.max_y.max(b.max_y),
        }),
        (a, None) => a,
        (None, b) => b,
    }
}

/// The blocks an object's inserts name, nested definitions too.
fn blocks_named(doc: &Document, entity: &kentos_contracts::Entity, out: &mut BTreeSet<String>) {
    if let kentos_contracts::Entity::Insert(insert) = entity
        && out.insert(insert.block.to_text())
        && let Some(block) = doc.block(insert.block)
    {
        for inner in &block.entities {
            blocks_named(doc, inner, out);
        }
    }
}

/// The parts' stamps after a write, of the parts it changed.
fn part_stamps(doc: &Document, parts: &[ProjectPart], into: &mut StateStamps) {
    for part in parts {
        match part {
            ProjectPart::Name | ProjectPart::Settings => into.settings = Some(doc.settings_stamp()),
            ProjectPart::Styles => into.styles = Some(doc.styles_stamp()),
            ProjectPart::Sheets => into.sheets = Some(doc.sheets_stamp()),
            ProjectPart::LayerFlags => into.layers = Some(doc.layers_stamp()),
        }
    }
}

/// The write's summary and the plan's id (docs/adr/0253 §5).
#[allow(clippy::too_many_arguments)]
fn summarize(
    doc: &Document,
    report: &StepReport,
    warnings: &[CommandWarning],
    command: &str,
    version: u32,
    input: &Value,
    base: u64,
    parts: &PartStamps,
) -> (CommandSummary, String) {
    let named = uids_in(input);
    let mut summary = CommandSummary {
        algorithm: algorithm(command, version),
        warnings: warnings.to_vec(),
        ..CommandSummary::default()
    };
    summary.sources.revision = Some(base.to_string());
    let mut referenced = BTreeSet::new();
    let mut new_uids: Vec<String> = Vec::new();
    let (mut added, mut changed, mut removed) = (Vec::new(), BTreeMap::new(), Vec::new());
    for o in &report.objects {
        let uid = o.uid.to_string();
        for e in [o.before.as_deref(), o.after.as_deref()]
            .into_iter()
            .flatten()
        {
            summary.bounds = union(summary.bounds, bounds_of(e));
            blocks_named(doc, e, &mut referenced);
        }
        if o.added() {
            if let Some(after) = &o.after {
                summary
                    .stamps
                    .objects
                    .insert(uid.clone(), entity_stamp(o.uid, after));
                added.push(entity_value(after));
            }
            summary.added.push(uid.clone());
            new_uids.push(uid);
        } else if o.removed() {
            if let Some(before) = &o.before {
                summary
                    .sources
                    .objects
                    .insert(uid.clone(), entity_stamp(o.uid, before));
            }
            if !named.contains(&uid) {
                summary.dependents.push(uid.clone());
            }
            removed.push(Value::String(uid.clone()));
            summary.removed.push(uid);
        } else if o.changed()
            && let (Some(before), Some(after)) = (&o.before, &o.after)
        {
            summary
                .sources
                .objects
                .insert(uid.clone(), entity_stamp(o.uid, before));
            summary
                .stamps
                .objects
                .insert(uid.clone(), entity_stamp(o.uid, after));
            if !named.contains(&uid) {
                summary.dependents.push(uid.clone());
            }
            changed.insert(uid.clone(), entity_value(after));
            summary.changed.push(uid);
        }
    }
    // The definitions the touched inserts rest on, as they were, and the ones the step changed.
    let mut new_blocks: Vec<String> = Vec::new();
    let mut blocks_result = json!({ "added": [], "changed": {}, "removed": [] });
    for b in &report.block_states {
        let id = b.id.to_text();
        match (&b.before, &b.after) {
            (None, Some(after)) => {
                summary.stamps.blocks.insert(id.clone(), block_stamp(after));
                let mut content = serde_json::to_value(&**after).unwrap_or(Value::Null);
                if let Some(map) = content.as_object_mut() {
                    map.remove("id");
                }
                if let Some(list) = blocks_result["added"].as_array_mut() {
                    list.push(content);
                }
                new_blocks.push(id);
            }
            (Some(before), after) => {
                summary
                    .sources
                    .blocks
                    .insert(id.clone(), block_stamp(before));
                match after {
                    Some(after) => {
                        let stamp = block_stamp(after);
                        summary.stamps.blocks.insert(id.clone(), stamp.clone());
                        blocks_result["changed"][&id] = Value::String(stamp);
                    }
                    None => {
                        if let Some(list) = blocks_result["removed"].as_array_mut() {
                            list.push(Value::String(id));
                        }
                    }
                }
            }
            (None, None) => {}
        }
    }
    for id in referenced {
        if summary.sources.blocks.contains_key(&id) || new_blocks.contains(&id) {
            continue;
        }
        if let Some(stamp) = kentos_contracts::uuid_from_text(&id)
            .map(kentos_contracts::BlockId)
            .and_then(|b| doc.block_stamp(b))
        {
            summary.sources.blocks.insert(id, stamp);
        }
    }
    summary.counts = SummaryCounts {
        added: u32::try_from(summary.added.len()).unwrap_or(u32::MAX),
        changed: u32::try_from(summary.changed.len()).unwrap_or(u32::MAX),
        removed: u32::try_from(summary.removed.len()).unwrap_or(u32::MAX),
        dependents: u32::try_from(summary.dependents.len()).unwrap_or(u32::MAX),
    };
    summary.layers = NodeChanges {
        added: report.layers.added.clone(),
        changed: report.layers.changed.clone(),
        removed: report.layers.removed.clone(),
    };
    summary.blocks = NodeChanges {
        added: report.blocks.added.iter().map(|b| b.to_text()).collect(),
        changed: report.blocks.changed.iter().map(|b| b.to_text()).collect(),
        removed: report.blocks.removed.iter().map(|b| b.to_text()).collect(),
    };
    summary.project = report.parts.clone();
    // The write's own undo step: an outermost one, not inside a group, that changed what undo takes back.
    let undoable =
        !report.objects.is_empty() || !report.layers.is_empty() || !report.blocks.is_empty();
    summary.undo = UndoInfo {
        label: (report.outermost && !doc.in_group() && undoable)
            .then(|| report.label.clone())
            .flatten(),
        outside: report.parts.clone(),
    };
    // The parts' stamps: before, of the parts the write changed (and the
    // layer tree when its ops changed it), and after.
    let mut changed_parts = report.parts.clone();
    if !report.layers.is_empty() && !changed_parts.contains(&ProjectPart::LayerFlags) {
        changed_parts.push(ProjectPart::LayerFlags);
    }
    for part in &changed_parts {
        match part {
            ProjectPart::Name | ProjectPart::Settings => {
                summary.sources.settings = Some(parts.settings.clone())
            }
            ProjectPart::Styles => summary.sources.styles = Some(parts.styles.clone()),
            ProjectPart::Sheets => summary.sources.sheets = Some(parts.sheets.clone()),
            ProjectPart::LayerFlags => summary.sources.layers = Some(parts.layers.clone()),
        }
    }
    part_stamps(doc, &changed_parts, &mut summary.stamps);
    // The plan's id: what the write rests on and what it makes, new ids by their order.
    let mut plain_input = input.clone();
    if let Some(map) = plain_input.as_object_mut() {
        map.remove("expectedRevision");
    }
    let mut essence = json!({
        "command": command,
        "version": version,
        "input": plain_input,
        "sources": {
            "objects": summary.sources.objects,
            "blocks": summary.sources.blocks,
            "layers": summary.sources.layers,
            "settings": summary.sources.settings,
            "styles": summary.sources.styles,
            "sheets": summary.sources.sheets,
        },
        "result": {
            "added": added,
            "changed": changed,
            "removed": removed,
            "blocks": blocks_result,
            "layers": summary.stamps.layers,
            "settings": summary.stamps.settings,
            "styles": summary.stamps.styles,
            "sheets": summary.stamps.sheets,
        },
    });
    let fresh: BTreeMap<String, String> = new_uids
        .iter()
        .enumerate()
        .map(|(i, u)| (u.clone(), format!("#a{i}")))
        .chain(
            new_blocks
                .iter()
                .enumerate()
                .map(|(i, b)| (b.clone(), format!("#b{i}"))),
        )
        .collect();
    if !fresh.is_empty() {
        rename(&mut essence, &fresh);
    }
    (summary, id_of(&essence))
}

/// Every text of `value` that `names` maps, mapped (a new object's id by
/// its order; a replayed row's ids, replay.rs).
pub(crate) fn rename_texts(value: &mut Value, names: &BTreeMap<String, String>) {
    rename(value, names);
}

fn rename(value: &mut Value, names: &BTreeMap<String, String>) {
    match value {
        Value::String(s) => {
            if let Some(n) = names.get(s.as_str()) {
                n.clone_into(s);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|i| rename(i, names)),
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                if let Some(mut v) = map.remove(&k) {
                    rename(&mut v, names);
                    map.insert(names.get(&k).cloned().unwrap_or(k), v);
                }
            }
        }
        _ => {}
    }
}

// ── The protocol (docs/adr/0253 §4–§7) ─────────────────────────────────

fn failed(code: &str, message: impl Into<String>, path: Option<&str>) -> CallAnswer {
    CallAnswer::of(CommandResult::Failed {
        error: CommandError {
            code: code.to_owned(),
            message: message.into(),
            path: path.map(str::to_owned),
            ..CommandError::default()
        },
    })
}

fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The call's own faults: a request or a plan with `validate` or `plan`, a
/// request id out of its rule, stamps that are not stamps.
fn call_fault(envelope: &CallEnvelope) -> Option<CallAnswer> {
    let invalid =
        |message: &str, path: &str| Some(failed(codes::INVALID_CALL, message, Some(path)));
    if let Some(id) = &envelope.request_id {
        if envelope.op != CallOp::Execute {
            return invalid(
                "İstek kimliği yalnız execute ile verilir: validate ve plan bir şey yazmaz.",
                "requestId",
            );
        }
        if !request_id_valid(id) {
            return invalid(
                "İstek kimliği 1–128 karakter olmalı ve yalnız harf, rakam ve . _ : - içermeli.",
                "requestId",
            );
        }
    }
    let Some(e) = &envelope.expected else {
        return None;
    };
    if e.plan.is_some() && envelope.op != CallOp::Execute {
        return invalid("Beklenen plan yalnız execute ile verilir.", "expected.plan");
    }
    if let Some(plan) = &e.plan
        && !is_hex(plan, 32)
    {
        return invalid(
            "Planın kimliği 32 küçük onaltılık hane olmalı.",
            "expected.plan",
        );
    }
    if let Some(r) = &e.revision
        && (r.is_empty() || !r.bytes().all(|b| b.is_ascii_digit()))
    {
        return Some(failed(
            codes::INVALID_REVISION,
            "Beklenen sürüm ondalık bir sayı metni olmalı.",
            Some("expected.revision"),
        ));
    }
    for (k, v) in &e.objects {
        if kentos_contracts::uuid_from_text(k).is_none() {
            return invalid(
                "Beklenen nesnelerin anahtarı kalıcı kimlik (küçük harfli, tireli UUID) olmalı.",
                &format!("expected.objects.{k}"),
            );
        }
        if !is_hex(v, 16) {
            return invalid(
                "Durum damgası 16 küçük onaltılık hane olmalı.",
                &format!("expected.objects.{k}"),
            );
        }
    }
    for (k, v) in &e.blocks {
        if kentos_contracts::uuid_from_text(k).is_none() || !is_hex(v, 16) {
            return invalid(
                "Beklenen blokların anahtarı tanımın kimliği, değeri 16 haneli damga olmalı.",
                &format!("expected.blocks.{k}"),
            );
        }
    }
    for (name, v) in [
        ("layers", &e.layers),
        ("settings", &e.settings),
        ("styles", &e.styles),
        ("sheets", &e.sheets),
    ] {
        if let Some(v) = v
            && !is_hex(v, 16)
        {
            return invalid(
                "Durum damgası 16 küçük onaltılık hane olmalı.",
                &format!("expected.{name}"),
            );
        }
    }
    None
}

/// What no longer holds of `e`: the revision (`revision_conflict`) or the stamps (`stale_state`).
fn expectation_fault(doc: &Document, e: &CallExpectations) -> Option<CallAnswer> {
    let now = doc.revision().to_string();
    if let Some(r) = &e.revision
        && *r != now
    {
        return Some(CallAnswer::of(CommandResult::Conflict {
            error: CommandError {
                code: codes::REVISION_CONFLICT.into(),
                message: "Çizim girdi hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Girdiyi çizimin şimdiki hâline göre yeniden hazırlayın.".into(),
                path: Some("expected.revision".into()),
                revision: Some(now),
                retryable: true,
                ..CommandError::default()
            },
        }));
    }
    let mut stale: Vec<String> = Vec::new();
    let mut first: Option<String> = None;
    for (uid, stamp) in &e.objects {
        let now = Uuid::parse_str(uid).ok().and_then(|u| doc.stamp_of_uid(u));
        if now.as_deref() != Some(stamp.as_str()) {
            first.get_or_insert_with(|| format!("expected.objects.{uid}"));
            stale.push(uid.clone());
        }
    }
    for (id, stamp) in &e.blocks {
        let now = kentos_contracts::uuid_from_text(id)
            .map(kentos_contracts::BlockId)
            .and_then(|b| doc.block_stamp(b));
        if now.as_deref() != Some(stamp.as_str()) {
            first.get_or_insert_with(|| format!("expected.blocks.{id}"));
        }
    }
    for (name, want, have) in [
        ("layers", &e.layers, doc.layers_stamp()),
        ("settings", &e.settings, doc.settings_stamp()),
        ("styles", &e.styles, doc.styles_stamp()),
        ("sheets", &e.sheets, doc.sheets_stamp()),
    ] {
        if let Some(want) = want
            && *want != have
        {
            first.get_or_insert_with(|| format!("expected.{name}"));
        }
    }
    let path = first?;
    Some(CallAnswer::of(CommandResult::Conflict {
        error: CommandError {
            code: codes::STALE_STATE.into(),
            message: "Beklenen nesne, tanım ya da proje parçası değişmiş ya da artık yok; hiçbir şey yazılmadı. Damgaları yeniden okuyup girdiyi yeniden hazırlayın.".into(),
            path: Some(path),
            revision: Some(now),
            uids: stale,
            retryable: true,
        },
    }))
}

/// A value at a path of the input (`uids[2]`, `changes[0].uid`, `ops[1]`).
fn at_path<'v>(input: &'v Value, path: &str) -> Option<&'v Value> {
    let mut v = input;
    for part in path.split('.') {
        let (name, rest) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        if !name.is_empty() {
            v = v.get(name)?;
        }
        for index in rest.split('[').skip(1) {
            let i: usize = index.trim_end_matches(']').parse().ok()?;
            v = v.get(i)?;
        }
    }
    Some(v)
}

/// An answer's error made whole (docs/adr/0253 §7): whether calling again
/// may help, by its code; the objects it is about, from its path in the input.
fn enrich(answer: &mut CallAnswer, input: &Value) {
    let error = match &mut answer.result {
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => error,
        _ => return,
    };
    error.retryable |= codes::retryable(&error.code);
    if error.uids.is_empty()
        && let Some(path) = &error.path
        && let Some(v) = at_path(input, path)
    {
        error.uids = uids_in(v).into_iter().collect();
    }
}

/// Reads the command's own answer (its wire JSON) as the protocol's.
fn answer_of(value: Value) -> CallAnswer {
    match serde_json::from_value::<CommandResult<Value>>(value) {
        Ok(result) => CallAnswer::of(result),
        Err(e) => failed(
            codes::INVALID_INPUT,
            format!("Komutun cevabı okunamadı: {e}"),
            None,
        ),
    }
}

/// The protocol's call (docs/adr/0253 §4): validates, plans or executes the
/// envelope's command on `doc`, with the host's geometry store when it has
/// one, and answers with its result and summary. Never panics; every
/// refusal is an answer.
pub fn call(doc: &mut Document, store: Option<&Store>, envelope: CallEnvelope) -> CallAnswer {
    call_with(doc, store, envelope, None)
}

/// [`call`] on a job's reading copy, its progress and stop on the context
/// (jobs.rs).
pub(crate) fn call_in_job(
    doc: &mut Document,
    envelope: CallEnvelope,
    job: &Progress,
) -> CallAnswer {
    call_with(doc, None, envelope, Some(job))
}

fn call_with(
    doc: &mut Document,
    store: Option<&Store>,
    envelope: CallEnvelope,
    job: Option<&Progress>,
) -> CallAnswer {
    let command = envelope.command.clone();
    let Some(version) = registry::version_of(&command) else {
        return failed(
            codes::UNKNOWN_COMMAND,
            format!("“{command}” bu çizimde çalışan bir komut değil."),
            Some("command"),
        );
    };
    let input = match registry::normalize(&command, envelope.input.clone()) {
        Ok(v) => v,
        Err(unrun) => return failed(unrun.code, unrun.message, Some("input")),
    };
    let state = match preflight(doc, &envelope, version, &input) {
        Ok(state) => state,
        Err(answer) => return *answer,
    };
    let mut answer = match envelope.op {
        CallOp::Validate => {
            let mut cx = ExecutionContext::with_optional_store(doc, store);
            match registry::run(&mut cx, &command, CallOp::Validate, input.clone()) {
                Ok(v) => answer_of(v),
                Err(unrun) => failed(unrun.code, unrun.message, None),
            }
        }
        CallOp::Plan => plan(doc, store, &command, &input, state),
        CallOp::Execute => {
            let mut cx = ExecutionContext::with_optional_store(doc, store);
            cx.call = Some(state);
            cx.job = job;
            let result = registry::run(&mut cx, &command, CallOp::Execute, input.clone());
            let outcome = cx.call.take().and_then(|s| s.outcome);
            let mut answer = match result {
                Ok(v) => answer_of(v),
                Err(unrun) => failed(unrun.code, unrun.message, None),
            };
            if let (CommandResult::Completed { .. }, Some(o)) = (&answer.result, outcome) {
                answer.summary = Some(o.summary);
                answer.plan_id = Some(o.plan_id);
                answer.call = o.call;
                if envelope.request_id.is_some()
                    && let Some(call) = o.call
                    && let Ok(text) = serde_json::to_string(&answer)
                {
                    doc.set_request_answer(call, text);
                }
            }
            answer
        }
    };
    enrich(&mut answer, &input);
    answer
}

/// What the protocol checks before a command runs (docs/adr/0253 §4, §6,
/// §7), its input read as its type (`input`): the call's own faults, a
/// request sent again (`Err`: the answer then, or `request_reused`), what
/// the call expects. `Ok`: what the command runs with. The answer is boxed:
/// it is large, and a call stops here at most once.
pub fn preflight(
    doc: &Document,
    envelope: &CallEnvelope,
    version: u32,
    input: &Value,
) -> Result<CallState, Box<CallAnswer>> {
    if let Some(v) = envelope.version
        && v != version
    {
        return Err(Box::new(failed(
            codes::UNKNOWN_VERSION,
            format!(
                "{} için {v}. sürüm yok; bu sürüm {version}. sürümü çalıştırır.",
                envelope.command
            ),
            Some("version"),
        )));
    }
    if let Some(fault) = call_fault(envelope) {
        return Err(Box::new(fault));
    }
    let mut state = CallState {
        summary: true,
        plan: envelope.expected.as_ref().and_then(|e| e.plan.clone()),
        ..CallState::default()
    };
    if let Some(id) = &envelope.request_id {
        let fingerprint = id_of(&json!({
            "command": envelope.command,
            "version": version,
            "op": envelope.op.name(),
            "input": input,
            "expected": envelope.expected.clone().unwrap_or_default(),
        }));
        if let Some(record) = doc.request(id) {
            if record.fingerprint != fingerprint {
                return Err(Box::new(failed(
                    codes::REQUEST_REUSED,
                    format!(
                        "“{id}” istek kimliği başka bir çağrıyla ({}) kullanıldı; bir kimlik tek bir işi adlar. Yeni bir kimlik verin.",
                        record.command
                    ),
                    Some("requestId"),
                )));
            }
            let mut answer = record
                .answer
                .as_deref()
                .and_then(|a| serde_json::from_str::<CallAnswer>(a).ok())
                .unwrap_or_else(|| {
                    CallAnswer::of(CommandResult::Completed {
                        output: Value::Null,
                        warnings: Vec::new(),
                    })
                });
            answer.replayed = true;
            answer.undone = record.status == CallStatus::Undone;
            return Err(Box::new(answer));
        }
        state.request = Some((id.clone(), fingerprint));
    }
    if let Some(e) = &envelope.expected
        && let Some(fault) = expectation_fault(doc, e)
    {
        return Err(Box::new(fault));
    }
    Ok(state)
}

/// A file command's write in the journal (docs/adr/0253 §10): its row, the
/// files it wrote, and its request; the row's number. Its answer is kept for
/// the request once it is made (`set_request_answer`).
pub fn record_effect(
    doc: &mut Document,
    command: &str,
    version: u32,
    input: &Value,
    effects: Vec<FileEffect>,
    state: &CallState,
) -> u64 {
    let report = StepReport::default();
    let mut row = row_of(
        doc,
        &report,
        command,
        version,
        input,
        &[],
        state.request.as_ref(),
        doc.revision(),
    );
    row.effects = effects;
    let request = state
        .request
        .as_ref()
        .map(|(id, fingerprint)| RequestRecord {
            request_id: id.clone(),
            fingerprint: fingerprint.clone(),
            command: command.to_owned(),
            version,
            revision: String::new(),
            status: CallStatus::Done,
            answer: None,
        });
    doc.record_call(row, request)
}

/// The journal row of a command's write its host made itself, over several
/// frames, as one undo group (the desktop's large DXF import, docs/adr/0257
/// §4, written a slice at a time as docs/adr/0138 asks): the command, its
/// input, the step's name, the objects it made and what it said, as
/// [`recorded`] writes for a write made in one step. `base` is the revision
/// before the write.
#[allow(clippy::too_many_arguments)]
pub fn record_written(
    doc: &mut Document,
    command: &str,
    version: u32,
    input: &Value,
    label: &str,
    created: &[kentos_domain::Slot],
    warnings: &[CommandWarning],
    base: u64,
) -> u64 {
    let report = StepReport {
        label: Some(label.to_owned()),
        ..StepReport::default()
    };
    let mut row = row_of(doc, &report, command, version, input, warnings, None, base);
    row.truncated = created.len() > JOURNAL_IDS;
    row.created = created
        .iter()
        .filter_map(|s| doc.uid(*s))
        .take(JOURNAL_IDS)
        .map(|u| u.to_string())
        .collect();
    doc.record_call(row, None)
}

/// [`enrich`] for a host's own answer (a file command's).
pub fn complete(answer: &mut CallAnswer, input: &Value) {
    enrich(answer, input);
}

/// The command's own plan, with the summary and the plan's id of executing
/// it on a reading copy (docs/adr/0253 §5): what execute would write, by the
/// same algorithm.
fn plan(
    doc: &mut Document,
    store: Option<&Store>,
    command: &str,
    input: &Value,
    mut state: CallState,
) -> CallAnswer {
    let own = {
        let mut cx = ExecutionContext::with_optional_store(doc, store);
        match registry::run(&mut cx, command, CallOp::Plan, input.clone()) {
            Ok(v) => answer_of(v),
            Err(unrun) => return failed(unrun.code, unrun.message, None),
        }
    };
    if !matches!(own.result, CommandResult::Completed { .. }) {
        return own;
    }
    state.plan = None;
    let mut copy = doc.reading_copy();
    let mut cx = ExecutionContext::with_optional_store(&mut copy, store);
    cx.call = Some(state);
    let written = registry::run(&mut cx, command, CallOp::Execute, input.clone());
    let outcome = cx.call.take().and_then(|s| s.outcome);
    match written.map(answer_of) {
        Ok(answer) if matches!(answer.result, CommandResult::Completed { .. }) => {
            let mut own = own;
            if let Some(o) = outcome {
                let mut summary = o.summary;
                // A plan writes nothing: the revision after is the one it would make.
                summary.stamps.revision = None;
                own.summary = Some(summary);
                own.plan_id = Some(o.plan_id);
            }
            own
        }
        // The plan said yes; the write says no: the write's word is the plan's.
        Ok(answer) => answer,
        Err(unrun) => failed(unrun.code, unrun.message, None),
    }
}

/// Writes a job's result, computed on `copy`, to `doc` (jobs.rs, docs/adr/0253
/// §9): the copy's write as it is when `doc` is still at `base` (its revision
/// and generation then), by the objects' persistent ids when it changed and
/// they are still as the job saw them; otherwise `document_changed`.
pub(crate) fn commit_job(
    doc: &mut Document,
    mut copy: Document,
    mut answer: CallAnswer,
    base: (u64, u64),
    request_id: Option<&str>,
) -> CallAnswer {
    let patch = copy.take_patch();
    match doc.apply_patch(patch, base) {
        Ok(call) => {
            let revision = doc.revision().to_string();
            if let CommandResult::Completed { output, .. } = &mut answer.result
                && let Some(r) = output.get_mut("revision")
            {
                *r = Value::String(revision.clone());
            }
            if let Some(summary) = &mut answer.summary {
                summary.stamps.revision = Some(revision);
            }
            answer.call = call;
            if request_id.is_some()
                && let Some(call) = call
                && let Ok(text) = serde_json::to_string(&answer)
            {
                doc.set_request_answer(call, text);
            }
            answer
        }
        Err(refused) => CallAnswer::of(CommandResult::Conflict {
            error: CommandError {
                code: codes::DOCUMENT_CHANGED.into(),
                message: refused.message,
                revision: Some(doc.revision().to_string()),
                uids: refused.uids.iter().map(Uuid::to_string).collect(),
                retryable: true,
                ..CommandError::default()
            },
        }),
    }
}
