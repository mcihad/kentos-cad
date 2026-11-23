//! `cad.layers.session` v1 (docs/adr/9103 §3): a data layer's edit session, through the document's session
//! (`Document::start_editing` and the rest) and its table's reads and writes ([`crate::data_write`]). Opening, letting go,
//! closing and taking are not undo steps; `delete` is one, journalled as a command's step; `save` writes each layer's
//! changes in one transaction of its source, and a row written is not unwritten from here. `conflicts` says the clashes
//! a save would meet, field by field; `save` with `choices` settles them as chosen and writes (docs/adr/9193).

use kentos_commands::{
    ClashChoice, ClashDeleted, ClashSide, CommandResult, CommandWarning, FieldConflict, LAYER_SESSION_KEYS_MOST,
    LayersSession, LayersSessionDone, RowConflict, SessionAction, SessionLayer,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::data_write::{self, WriteFailure};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The layers the input acts on, checked against the document in the order its refusals are listed.
fn check(doc: &Document, input: &LayersSession) -> Result<Vec<String>, Stop> {
    let action = input.action;
    let named: Vec<String> = input
        .layers
        .iter()
        .flatten()
        .map(|l| l.trim().to_owned())
        .collect();
    let single = matches!(action, SessionAction::Take | SessionAction::Delete);
    if named.is_empty() && (single || action == SessionAction::Start) {
        return Err(fail(
            codes::NO_LAYER,
            "Düzenleme oturumunun katmanını (layers) verin.".into(),
            "layers",
        ));
    }
    if single && named.len() > 1 {
        return Err(fail(
            codes::ONE_LAYER,
            format!("Satır alma ve silme tek bir katmanla çalışır; {} katman verildi.", named.len()),
            "layers",
        ));
    }
    if single {
        let keys = input.keys.as_deref().unwrap_or_default();
        if keys.is_empty() {
            return Err(fail(codes::NO_KEYS, "Satırların anahtarlarını (keys) verin.".into(), "keys"));
        }
        if keys.len() > LAYER_SESSION_KEYS_MOST {
            return Err(fail(
                codes::INVALID_COUNT,
                format!(
                    "Bir seferde en çok {LAYER_SESSION_KEYS_MOST} satır alınır ya da silinir; {} anahtar verildi.",
                    keys.len()
                ),
                "keys",
            ));
        }
    }
    let layers = if named.is_empty() { doc.edited_layers() } else { named };
    for (i, id) in layers.iter().enumerate() {
        let path = format!("layers[{i}]");
        let Some(node) = doc.layers().get(id) else {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{id}” kimlikli katman çizimde yok."), &path));
        };
        let name = &node.name;
        let Some(data) = &node.data else {
            return Err(fail(
                codes::NOT_A_DATA_LAYER,
                format!("“{name}” bir veri katmanı değil; düzenleme oturumu yalnız bir tablodan gelen katmanın olur."),
                &path,
            ));
        };
        if action == SessionAction::Start {
            if data.read_only {
                return Err(fail(
                    codes::DATA_LAYER_READ_ONLY,
                    format!("“{name}” salt okunur bir veri katmanı; düzenlenemez."),
                    &path,
                ));
            }
            continue;
        }
        if !doc.editing(id) {
            return Err(fail(
                codes::DATA_LAYER_NOT_EDITING,
                format!("“{name}” düzenlemeye açık değil; önce düzenlemesini başlatın (action: start)."),
                &path,
            ));
        }
        if action == SessionAction::Stop && doc.data_modified(id) {
            return Err(fail(
                codes::DATA_EDITS_UNSAVED,
                format!(
                    "“{name}” katmanında kaydedilmemiş değişiklik var; düzenlemeyi bitirmeden önce kaydedin (save) ya da bırakın (rollback, cancel)."
                ),
                &path,
            ));
        }
    }
    // The layers that go together with the named ones (docs/adr/9104 §2): a source's or every one, as the project says.
    let grouping = doc.settings().edit_grouping();
    if single || grouping == kentos_commands::EditGrouping::Layer || named_none(input) {
        return Ok(layers);
    }
    let together = data_write::grouped(doc, &layers, grouping, action != SessionAction::Start);
    if action == SessionAction::Stop
        && let Some(id) = together.iter().find(|l| doc.data_modified(l))
    {
        let name = doc.layers().get(id).map_or(id.as_str(), |n| n.name.as_str());
        return Err(fail(
            codes::DATA_EDITS_UNSAVED,
            format!(
                "“{name}” katmanında kaydedilmemiş değişiklik var ve düzenlemesi bu katmanla birlikte biter; önce kaydedin (save) ya da bırakın (rollback, cancel)."
            ),
            "layers",
        ));
    }
    Ok(together)
}

/// Whether the input names no layer (then every layer being edited is meant already).
fn named_none(input: &LayersSession) -> bool {
    input.layers.as_ref().is_none_or(Vec::is_empty)
}

/// A layer's session as it is now.
fn state(doc: &Document, layer: &str) -> SessionLayer {
    let c = doc.data_changes(layer);
    SessionLayer {
        layer: layer.to_owned(),
        editing: doc.editing(layer),
        added: c.added.len() as u32,
        changed: c.changed.len() as u32,
        deleted: c.deleted.len() as u32,
        keys: Vec::new(),
        failed: None,
    }
}

fn done(doc: &Document, layers: Vec<SessionLayer>, objects: Vec<Option<String>>) -> LayersSessionDone {
    LayersSessionDone {
        layers,
        objects,
        conflicts: Vec::new(),
        revision: doc.revision().to_string(),
    }
}

/// Checks `input` against the document; reads no table, writes nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersSession) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layers' sessions as they are now, after the checks; reads no table, writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersSession) -> CommandResult<LayersSessionDone> {
    match check(cx.doc, input) {
        Ok(layers) => CommandResult::Completed {
            output: done(cx.doc, layers.iter().map(|l| state(cx.doc, l)).collect(), Vec::new()),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersSession) -> CommandResult<LayersSessionDone> {
    let layers = match check(cx.doc, &input) {
        Ok(l) => l,
        Err(stop) => return stop.into(),
    };
    match input.action {
        SessionAction::Delete => crate::call::recorded(
            cx,
            kentos_commands::CAD_LAYERS_SESSION,
            kentos_commands::CAD_LAYERS_SESSION_VERSION,
            input,
            write_delete,
        ),
        SessionAction::Save => save(cx, &layers, &input),
        SessionAction::Conflicts => conflicts(cx, &layers),
        SessionAction::Take => take(cx, &layers[0], input.keys.as_deref().unwrap_or_default()),
        action => {
            let doc = &mut *cx.doc;
            for id in &layers {
                let went = match action {
                    SessionAction::Start => doc.start_editing(id).map(|_| ()).map_err(|r| r.0),
                    SessionAction::Stop => doc.stop_editing(id).map(|_| ()).map_err(|r| r.0),
                    SessionAction::Rollback => doc.discard_data_edits(id).map(|_| ()),
                    SessionAction::Cancel => doc
                        .discard_data_edits(id)
                        .and_then(|_| doc.stop_editing(id).map(|_| ()).map_err(|r| r.0)),
                    _ => Ok(()),
                };
                if let Err(message) = went {
                    return fail(codes::LAYER_REFUSED, message, "layers").into();
                }
            }
            CommandResult::Completed {
                output: done(doc, layers.iter().map(|l| state(doc, l)).collect(), Vec::new()),
                warnings: Vec::new(),
            }
        }
    }
}

/// `delete`'s write: one undo step of the document, journalled.
fn write_delete(cx: &mut ExecutionContext<'_>, input: LayersSession) -> CommandResult<LayersSessionDone> {
    let layers = match check(cx.doc, &input) {
        Ok(l) => l,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    if let Err(refusal) = doc.delete_data_rows(&layers[0], input.keys.as_deref().unwrap_or_default()) {
        return fail(codes::DATA_LAYER_NOT_EDITING, refusal.0, "layers").into();
    }
    CommandResult::Completed {
        output: done(doc, vec![state(doc, &layers[0])], Vec::new()),
        warnings: Vec::new(),
    }
}

/// `take`: the rows read from the table and taken into the drawing; their objects' ids in the keys' order.
fn take(cx: &mut ExecutionContext<'_>, layer: &str, keys: &[i64]) -> CommandResult<LayersSessionDone> {
    let Some(node) = cx.doc.layers().get(layer).cloned() else {
        return fail(codes::LAYER_NOT_FOUND, format!("“{layer}” kimlikli katman çizimde yok."), "layers").into();
    };
    let settings = cx.doc.settings().clone();
    let rows = match data_write::rows_to_take(&node, &settings, keys, &|| cx.stopped()) {
        Ok(rows) => rows,
        Err(message) => return fail(codes::SOURCE_UNREACHABLE, message, "layers").into(),
    };
    let read: Vec<i64> = rows.iter().map(|(k, _)| *k).collect();
    let doc = &mut *cx.doc;
    let uids = match doc.take_data_rows(layer, rows) {
        Ok(u) => u,
        Err(refusal) => return fail(codes::DATA_LAYER_NOT_EDITING, refusal.0, "layers").into(),
    };
    let by_key: std::collections::HashMap<i64, String> = read
        .into_iter()
        .zip(uids)
        .filter_map(|(k, u)| Some((k, u?.to_string())))
        .collect();
    let objects = keys.iter().map(|k| by_key.get(k).cloned()).collect();
    CommandResult::Completed {
        output: done(doc, vec![state(doc, layer)], objects),
        warnings: Vec::new(),
    }
}

/// `save`: the layers' changes written, each alone or a source's together in one transaction (docs/adr/9104 §2),
/// checked against what others wrote since (unless `force`), and taken out of the drawing. One group that fails is the
/// command's refusal (a conflict's own); of several, those that fail are warnings while the others are written.
fn save(cx: &mut ExecutionContext<'_>, layers: &[String], input: &LayersSession) -> CommandResult<LayersSessionDone> {
    let force = input.force == Some(true);
    let choices = input.choices.as_deref();
    let settings = cx.doc.settings().clone();
    let grouping = settings.edit_grouping();
    let groups = data_write::groups_of(cx.doc, layers, grouping);
    let mut out: Vec<SessionLayer> = Vec::with_capacity(layers.len());
    let mut failures: Vec<(String, WriteFailure)> = Vec::new();
    let mut settled: Vec<RowConflict> = Vec::new();
    for group in groups {
        let mut group = group;
        let mut result = data_write::write_group(&group, &settings, force);
        // The clashes met, settled as chosen (docs/adr/9193); the group's writes made again from the settled drawing
        // and written, the table checked once more.
        if let (Err(WriteFailure::Conflict(m, _, rows)), Some(choices)) = (&result, choices) {
            let picks = match picks_of(cx.doc, rows, choices, m) {
                Ok(picks) => picks,
                Err(answer) => return *answer,
            };
            let rows = rows.clone();
            let ids: Vec<String> = group.iter().map(|w| w.layer.clone()).collect();
            // The objects made as chosen: the command's own recorded step (one undo step, “Çakışmayı çöz”). What the
            // session keeps of the rows (their bases, a row let go) is not undone with it, as a take is not.
            let chosen = rows.clone();
            let step = crate::call::recorded(
                cx,
                kentos_commands::CAD_LAYERS_SESSION,
                kentos_commands::CAD_LAYERS_SESSION_VERSION,
                input.clone(),
                move |cx, _| match crate::data_clash::settle(cx.doc, &chosen, &picks, SETTLE) {
                    Ok(done) if done.problems.is_empty() => CommandResult::Completed { output: (), warnings: Vec::new() },
                    Ok(done) => fail(codes::DATA_SAVE_FAILED, done.problems.join("; "), "choices").into(),
                    Err(why) => fail(codes::DATA_SAVE_FAILED, why, "choices").into(),
                },
            );
            match step {
                CommandResult::Completed { .. } => {}
                CommandResult::Failed { error } => return CommandResult::Failed { error },
                CommandResult::Conflict { error } => return CommandResult::Conflict { error },
                CommandResult::NeedsInput { error } => return CommandResult::NeedsInput { error },
                CommandResult::Queued { job_id } => return CommandResult::Queued { job_id },
                CommandResult::Cancelled => return CommandResult::Cancelled,
            }
            settled.extend(rows.iter().map(row_conflict));
            group = ids.iter().filter_map(|id| data_write::writes_of(cx.doc, id)).collect();
            result = if group.is_empty() { Ok(Vec::new()) } else { data_write::write_group(&group, &settings, false) };
        }
        match result {
            Ok(keys) => {
                for (w, keys) in group.iter().zip(keys) {
                    let counts = (
                        w.changes.added.len() as u32,
                        w.changes.changed.len() as u32,
                        w.changes.deleted.len() as u32,
                    );
                    if let Err(message) = cx.doc.data_saved(&w.layer, data_write::written(w, &keys)) {
                        failures.push((w.layer.clone(), WriteFailure::Refused(message, None)));
                        continue;
                    }
                    out.push(SessionLayer {
                        layer: w.layer.clone(),
                        editing: cx.doc.editing(&w.layer),
                        added: counts.0,
                        changed: counts.1,
                        deleted: counts.2,
                        keys,
                        failed: None,
                    });
                }
            }
            Err(failure) => {
                for w in &group {
                    let mut s = state(cx.doc, &w.layer);
                    s.failed = Some(failure.message().to_owned());
                    out.push(s);
                }
                failures.push((group[0].layer.clone(), failure));
            }
        }
    }
    // The layers with nothing to write are said as they are.
    for id in layers {
        if !out.iter().any(|s| &s.layer == id) {
            out.push(state(cx.doc, id));
        }
    }
    let order = |id: &str| layers.iter().position(|l| l == id).unwrap_or(usize::MAX);
    out.sort_by_key(|s| order(&s.layer));
    let doc = &*cx.doc;
    let written = out.iter().any(|s| s.failed.is_none() && (s.added + s.changed + s.deleted) > 0);
    if !failures.is_empty() && !written {
        let (id, failure) = failures.swap_remove(0);
        let path = format!("layers[{}]", order(&id).min(layers.len().saturating_sub(1)));
        return match failure {
            WriteFailure::Conflict(m, uids, _) => {
                let words = format!(
                    "{m} Değişikliği bırakıp satırları yeniden alın (action: rollback, sonra take) ya da üstüne yazın \
                     (force: true)."
                );
                let mut e = error(codes::ROWS_CHANGED, words, Some(path));
                e.uids = uids.iter().map(ToString::to_string).collect();
                e.retryable = true;
                CommandResult::Conflict { error: e }
            }
            WriteFailure::Unreachable(m) => Stop::failed(error(codes::SOURCE_UNREACHABLE, m, Some(path))).into(),
            // The commit's answer lost (docs/adr/9126): not retried by itself, the source may hold the rows already.
            WriteFailure::Uncertain(m, _) => {
                let words = format!(
                    "{m} Değişiklikler katmanda duruyor; yeniden kaydetmeden önce kaynakta satırların yazılıp yazılmadığını \
                     denetleyin: yeniden yazmak yeni satırları ikinci kez ekleyebilir."
                );
                Stop::failed(error(codes::SOURCE_UNREACHABLE, words, Some(path))).into()
            }
            WriteFailure::Refused(m, uid) => {
                let mut e = error(codes::DATA_SAVE_FAILED, m, Some(path));
                e.uids = uid.iter().map(ToString::to_string).collect();
                Stop::failed(e).into()
            }
        };
    }
    let warnings = failures
        .into_iter()
        .map(|(id, failure)| CommandWarning {
            code: codes::DATA_SAVE_FAILED.into(),
            message: failure.message().to_owned(),
            path: Some(format!("layers[{}]", order(&id))),
        })
        .collect();
    let mut output = done(doc, out, Vec::new());
    output.conflicts = settled;
    CommandResult::Completed { output, warnings }
}

/// The undo step a save's settled clashes make (docs/adr/9148 §4).
const SETTLE: &str = "Çakışmayı çöz";

/// A clash as the command says it.
fn row_conflict(r: &crate::data_clash::RowClash) -> RowConflict {
    RowConflict {
        layer: r.layer.clone(),
        key: r.key,
        uid: r.uid.to_string(),
        fields: r
            .fields
            .iter()
            .map(|f| FieldConflict {
                field: f.field.clone(),
                first: f.first.clone(),
                mine: f.mine.clone(),
                theirs: f.theirs.clone(),
            })
            .collect(),
        shape: r.geometry,
        deleted: if r.gone() {
            Some(ClashDeleted::Theirs)
        } else if r.deleted_here() {
            Some(ClashDeleted::Here)
        } else {
            None
        },
        may_add: r.gone() && r.may_add,
    }
}

/// Each clash's picks as `choices` say them (`true` the side here), in the order `settle` reads them; a row with one
/// way only (deleted on both sides, or deleted in a table the connection may not add rows to) is let go unasked. A
/// choice no clash asks for is a `clash_choice`; a clash not chosen a `rows_changed`, nothing settled.
fn picks_of(
    doc: &Document,
    rows: &[crate::data_clash::RowClash],
    choices: &[ClashChoice],
    clash_words: &str,
) -> Result<Vec<Vec<bool>>, Box<CommandResult<LayersSessionDone>>> {
    let name = |layer: &str| doc.layers().get(layer).map_or_else(|| layer.to_owned(), |n| n.name.clone());
    let bad = |i: usize, words: String| {
        Box::new(
            fail(
                codes::CLASH_CHOICE,
                format!("{words}; çakışmaları okuyun (action: conflicts) ve yalnız onları seçin."),
                &format!("choices[{i}]"),
            )
            .into(),
        )
    };
    let side = |s: ClashSide| s == ClashSide::Mine;
    for (i, c) in choices.iter().enumerate() {
        let Some(r) = rows.iter().find(|r| r.layer == c.layer && r.key == c.key) else {
            return Err(bad(i, format!("“{}” katmanının {} anahtarlı satırı çakışmıyor", name(&c.layer), c.key)));
        };
        let at = format!("“{}” katmanının {} anahtarlı satırının", name(&c.layer), c.key);
        if let Some(f) = c.fields.keys().find(|f| !r.fields.iter().any(|x| &x.field == *f)) {
            return Err(bad(i, format!("{at} “{f}” alanı çakışmıyor")));
        }
        if c.shape.is_some() && !r.geometry {
            return Err(bad(i, format!("{at} şekli çakışmıyor")));
        }
        match c.row {
            Some(_) if !(r.gone() || r.deleted_here()) => {
                return Err(bad(i, format!("{at} iki yanı da duruyor; row yalnız bir yanda silinmiş satırda seçilir")));
            }
            Some(ClashSide::Mine) if r.gone() && !(r.may_add && r.mine.is_some()) => {
                return Err(bad(
                    i,
                    format!("{at} tablodaki kaydı silinmiş ve yeni satır olarak yazılamaz; yalnız theirs seçilir"),
                ));
            }
            _ => {}
        }
    }
    let mut out = Vec::with_capacity(rows.len());
    let mut unchosen: Vec<String> = Vec::new();
    for r in rows {
        let c = choices.iter().find(|c| c.layer == r.layer && c.key == r.key);
        let at = format!("satır {}", r.key);
        if r.gone() && (r.mine.is_none() || !r.may_add) {
            out.push(vec![false]);
        } else if r.gone() || r.deleted_here() {
            match c.and_then(|c| c.row) {
                Some(s) => out.push(vec![side(s)]),
                None => {
                    unchosen.push(format!("{at} (silinmiş)"));
                    out.push(vec![false]);
                }
            }
        } else {
            let mut picks = Vec::with_capacity(r.fields.len() + 1);
            for f in &r.fields {
                match c.and_then(|c| c.fields.get(&f.field)) {
                    Some(s) => picks.push(side(*s)),
                    None => {
                        unchosen.push(format!("{at} {}", f.field));
                        picks.push(false);
                    }
                }
            }
            if r.geometry {
                match c.and_then(|c| c.shape) {
                    Some(s) => picks.push(side(s)),
                    None => {
                        unchosen.push(format!("{at} şekli"));
                        picks.push(false);
                    }
                }
            }
            out.push(picks);
        }
    }
    if !unchosen.is_empty() {
        let mut e = error(
            codes::ROWS_CHANGED,
            format!(
                "{clash_words} Seçilmeyen: {}. Çakışmaları okuyun (action: conflicts) ve her birini choices'ta seçin.",
                unchosen.join(", ")
            ),
            Some("choices".to_owned()),
        );
        e.uids = rows.iter().map(|r| r.uid.to_string()).collect();
        e.retryable = true;
        return Err(Box::new(CommandResult::Conflict { error: e }));
    }
    Ok(out)
}

/// `conflicts`: the clashes a save of the layers would meet now, read without writing (docs/adr/9193).
fn conflicts(cx: &mut ExecutionContext<'_>, layers: &[String]) -> CommandResult<LayersSessionDone> {
    let settings = cx.doc.settings().clone();
    let groups = data_write::groups_of(cx.doc, layers, settings.edit_grouping());
    let mut found: Vec<RowConflict> = Vec::new();
    for group in &groups {
        match data_write::clashes_of_group(group, &settings) {
            Ok(rows) => found.extend(rows.iter().map(row_conflict)),
            Err(WriteFailure::Unreachable(m)) => return fail(codes::SOURCE_UNREACHABLE, m, "layers").into(),
            Err(failure) => return fail(codes::DATA_SAVE_FAILED, failure.message().to_owned(), "layers").into(),
        }
    }
    let doc = &*cx.doc;
    let mut output = done(doc, layers.iter().map(|l| state(doc, l)).collect(), Vec::new());
    output.conflicts = found;
    CommandResult::Completed { output, warnings: Vec::new() }
}
