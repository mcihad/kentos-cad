//! `cad.fields.write` (docs/adr/0507, plan/gis.md CBS-20): Alan hesapla and Yapıştır of the attribute table. The
//! values, an expression's per object or the rows pasted, are checked as `cad.entities.set` checks a value (the
//! field's rules and canonical text, required, subtypes, combinations, record rules, unique fields) object by
//! object; whole or nothing by default, the problems said by object and field. A label that showed a value written
//! over shows the new one, as Öznitelik hesapla does. One undo step: “Alan hesapla: alan” or “Yapıştır”.

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{
    CommandResult, CommandWarning, EmptyResult, Entity, FIELDS_WRITE_LISTED, FieldChange,
    FieldsWrite, FieldsWriteOp, FieldsWritten, LayerNodeType, ValueProblem, WritePolicy, codes,
};
use kentos_domain::Uuid;
use kentos_domain::{Document, Slot};
use kentos_expression::rows::{As, RowsInput, TEXT, evaluate_rows_on};
use kentos_expression::{Schema, compile_with};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::layer_filter::{kind_label, vertex_count};

fn refuse(code: &str, message: impl Into<String>, at: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(at.to_owned())))
}

/// The step a write is named by.
pub fn step_name(input: &FieldsWrite) -> String {
    match &input.write {
        FieldsWriteOp::Calculate { field, .. } => format!("Alan hesapla: {field}"),
        FieldsWriteOp::Values { .. } => "Yapıştır".to_owned(),
    }
}

/// One object's wanted values: its slot, its id, field → new value (none: taken away).
struct Want {
    slot: Slot,
    uid: String,
    values: BTreeMap<String, Option<String>>,
}

/// The object `text` names, on `layer` (`name` in messages).
fn object(
    doc: &Document,
    layer: &str,
    name: &str,
    text: &str,
    at: &str,
) -> Result<(Slot, String), Stop> {
    let Ok(uid) = Uuid::parse_str(text) else {
        return Err(refuse(
            codes::INVALID_UID,
            format!("“{text}” bir nesne kimliği değil."),
            at,
        ));
    };
    let slot = doc
        .slot_of(uid)
        .filter(|s| doc.get(*s).is_some_and(|e| e.base().layer_id == layer));
    let Some(slot) = slot else {
        return Err(refuse(
            codes::ENTITY_NOT_FOUND,
            format!("“{text}” kimlikli nesne “{name}” katmanında yok."),
            at,
        ));
    };
    Ok((slot, uid.to_string()))
}

/// The objects of `layer` named by `uids` (each on the layer, once), or all of it when empty.
fn objects(
    doc: &Document,
    layer: &str,
    name: &str,
    uids: &[String],
    at: &str,
) -> Result<Vec<(Slot, String)>, Stop> {
    if uids.is_empty() {
        return Ok(doc
            .by_layer_with_uids(layer)
            .map(|(u, e)| (Slot(e.base().id), u.to_string()))
            .collect());
    }
    let mut out = Vec::with_capacity(uids.len());
    let mut seen = BTreeSet::new();
    for (i, text) in uids.iter().enumerate() {
        let (slot, uid) = object(doc, layer, name, text, &format!("{at}[{i}]"))?;
        if seen.insert(slot.0) {
            out.push((slot, uid));
        }
    }
    Ok(out)
}

/// The expression's value as text for each of `list` (none: empty), in the project's reading unit.
fn calculate(
    doc: &Document,
    expression: &str,
    list: &[(Slot, String)],
) -> Result<Vec<Option<String>>, Stop> {
    let schema = Schema {
        unit: crate::expression_unit::reading_unit(doc.settings()),
        ..Schema::default()
    };
    let e = compile_with(expression, &schema).map_err(|e| {
        refuse(
            codes::INVALID_EXPRESSION,
            format!("İfade derlenmiyor: {}", e.text()),
            "write.expression",
        )
    })?;
    if e.needs.scale {
        return Err(refuse(
            codes::INVALID_EXPRESSION,
            "Alan hesabında $ölçek kullanılamaz: değer çizimin ölçeğine göre değişmemeli.",
            "write.expression",
        ));
    }
    let entities: Vec<&Entity> = list.iter().filter_map(|(s, _)| doc.get(*s)).collect();
    if entities.is_empty() {
        return Ok(Vec::new());
    }
    let needs = e.needs;
    let mut texts = String::new();
    let mut lens: Vec<i32> = Vec::new();
    let mut numbers: Vec<f64> = Vec::new();
    let put = |texts: &mut String, lens: &mut Vec<i32>, v: Option<&str>| match v {
        None => lens.push(-1),
        Some(v) => {
            texts.push_str(v);
            lens.push(v.encode_utf16().count() as i32);
        }
    };
    let layer_name = |id: &str| {
        doc.layers()
            .get(id)
            .map_or_else(|| id.to_owned(), |n| n.name.clone())
    };
    for o in &entities {
        let base = o.base();
        for f in &e.fields {
            put(&mut texts, &mut lens, base.attrs.get(f).map(String::as_str));
        }
        if needs.label {
            put(&mut texts, &mut lens, base.label.as_deref());
        }
        if needs.layer {
            let name = layer_name(&base.layer_id);
            put(&mut texts, &mut lens, Some(&name));
        }
        if needs.kind {
            put(&mut texts, &mut lens, Some(kind_label(o)));
        }
        if needs.id {
            numbers.push(f64::from(base.id));
        }
        if needs.vertices {
            numbers.push(vertex_count(o).unwrap_or(f64::NAN));
        }
    }
    let reads = needs.measured || needs.centroid || needs.bounds;
    let shapes: Vec<_> = if reads {
        entities.iter().map(|o| crate::geometry::shape(o)).collect()
    } else {
        Vec::new()
    };
    let input = RowsInput {
        n: entities.len(),
        texts: &texts,
        text_lens: &lens,
        numbers: &numbers,
        measures: &[],
        scale: f64::NAN,
    };
    let column = evaluate_rows_on(&e, &input, As::Text, |i| shapes.get(i)).map_err(|why| {
        refuse(
            codes::INVALID_EXPRESSION,
            format!("İfade değerlendirilemedi: {why}"),
            "write.expression",
        )
    })?;
    let mut out = Vec::with_capacity(entities.len());
    let mut rest = column.texts.as_str();
    let mut lens = column.text_lens.iter();
    for &k in &column.kinds {
        if k == TEXT {
            let units = lens.next().copied().unwrap_or(0) as usize;
            let mut taken = 0usize;
            let mut end = 0usize;
            for (i, c) in rest.char_indices() {
                if taken >= units {
                    end = i;
                    break;
                }
                taken += c.len_utf16();
                end = i + c.len_utf8();
            }
            if units == 0 {
                end = 0;
            }
            let (head, tail) = rest.split_at(end);
            out.push(Some(head.to_owned()));
            rest = tail;
        } else {
            out.push(None);
        }
    }
    Ok(out)
}

/// An object's attributes.
type Attrs = BTreeMap<String, String>;

/// An object written: its slot, its new attributes, the label that follows (none: the label stays).
type Write = (Slot, Attrs, Option<Option<String>>);

/// An object that changes: its slot, id, attributes before and after.
type Changed = (Slot, String, Attrs, Attrs);

/// What a write would do: the objects' new attributes (only those that change), the counts, the problems.
struct Planned {
    writes: Vec<Write>,
    out: FieldsWritten,
}

fn run(doc: &Document, input: &FieldsWrite) -> Result<Planned, Stop> {
    let layer = input.layer.as_str();
    let node = doc.layers().get(layer).ok_or_else(|| {
        refuse(
            codes::LAYER_NOT_FOUND,
            format!("“{layer}” kimlikli katman yok."),
            "layer",
        )
    })?;
    if node.kind != LayerNodeType::Layer {
        return Err(refuse(
            codes::NOT_A_LAYER,
            format!("“{}” bir grup; nesneleri katmanlardadır.", node.name),
            "layer",
        ));
    }
    if doc.layers().is_locked(layer) {
        return Err(refuse(
            codes::LAYER_LOCKED,
            format!("“{}” katmanı kilitli; önce kilidini açın.", node.name),
            "layer",
        ));
    }
    let typed = !node.fields.is_empty();
    let known = |name: &str| !typed || node.fields.iter().any(|f| f.name == name);
    let mut kept = 0u32;
    let wants: Vec<Want> = match &input.write {
        FieldsWriteOp::Calculate {
            field,
            expression,
            uids,
            empty,
        } => {
            if checks::is_blank(field) {
                return Err(refuse(
                    codes::INVALID_ATTRIBUTE,
                    "Alanın adı boş.",
                    "write.field",
                ));
            }
            let list = objects(doc, layer, &node.name, uids, "write.uids")?;
            if !known(field) {
                return Err(refuse(
                    codes::FIELD_NOT_FOUND,
                    format!(
                        "“{}” katmanında “{field}” alanı yok; Alanlar penceresinden ekleyin.",
                        node.name
                    ),
                    "write.field",
                ));
            }
            let values = calculate(doc, expression, &list)?;
            let mut wants = Vec::with_capacity(list.len());
            for ((slot, uid), v) in list.into_iter().zip(values) {
                let value = match (v, empty) {
                    (Some(t), _) if !t.is_empty() => Some(t),
                    (_, EmptyResult::Keep) => {
                        kept += 1;
                        continue;
                    }
                    (_, EmptyResult::Clear) => None,
                };
                wants.push(Want {
                    slot,
                    uid,
                    values: BTreeMap::from([(field.clone(), value)]),
                });
            }
            wants
        }
        FieldsWriteOp::Values { rows } => {
            if rows.is_empty() {
                return Err(refuse(
                    codes::NOTHING_TO_SET,
                    "Yazılacak satır yok.",
                    "write.rows",
                ));
            }
            let mut wants = Vec::with_capacity(rows.len());
            for (i, row) in rows.iter().enumerate() {
                let at = format!("write.rows[{i}]");
                if row.values.is_empty() {
                    return Err(refuse(
                        codes::NOTHING_TO_SET,
                        format!("{}. satırda değer yok.", i + 1),
                        &format!("{at}.values"),
                    ));
                }
                if row.values.keys().any(|n| checks::is_blank(n)) {
                    return Err(refuse(
                        codes::INVALID_ATTRIBUTE,
                        "Alanın adı boş.",
                        &format!("{at}.values"),
                    ));
                }
                let (slot, uid) = object(doc, layer, &node.name, &row.uid, &format!("{at}.uid"))?;
                if let Some(name) = row.values.keys().find(|n| !known(n)) {
                    return Err(refuse(
                        codes::FIELD_NOT_FOUND,
                        format!(
                            "“{}” katmanında “{name}” alanı yok; Alanlar penceresinden ekleyin.",
                            node.name
                        ),
                        &format!("{at}.values.{name}"),
                    ));
                }
                wants.push(Want {
                    slot,
                    uid,
                    values: row.values.clone(),
                });
            }
            wants
        }
    };
    // Each object's values against the rules, as `cad.entities.set` asks them.
    let mut rules = checks::LayerRules::new(doc);
    let mut problems: Vec<ValueProblem> = Vec::new();
    let mut invalid_slots: BTreeSet<u32> = BTreeSet::new();
    let mut changed: Vec<Changed> = Vec::new();
    let mut unchanged = 0u32;
    let problem = |problems: &mut Vec<ValueProblem>,
                   uid: &str,
                   field: &str,
                   value: Option<&String>,
                   stop: Stop| {
        let e = stop.into_error();
        let field = e
            .path
            .as_deref()
            .and_then(|p| p.rsplit('.').next())
            .filter(|f| !f.is_empty())
            .unwrap_or(field);
        problems.push(ValueProblem {
            uid: uid.to_owned(),
            field: field.to_owned(),
            value: value.cloned(),
            code: e.code,
            message: e.message,
        });
    };
    for w in &wants {
        let Some(entity) = doc.get(w.slot) else {
            continue;
        };
        let before = &entity.base().attrs;
        let first = w.values.keys().next().cloned().unwrap_or_default();
        let canonical = match checks::field_values(doc, layer, &w.values, "attrs") {
            Ok(c) => c,
            Err(stop) => {
                let value = stop_value(&stop, &w.values);
                problem(&mut problems, &w.uid, &first, value.as_ref(), stop);
                invalid_slots.insert(w.slot.0);
                continue;
            }
        };
        let mut after = before.clone();
        for (k, v) in &canonical {
            match v {
                Some(v) if !v.is_empty() => {
                    after.insert(k.clone(), v.clone());
                }
                _ => {
                    after.remove(k);
                }
            }
        }
        if after == *before {
            unchanged += 1;
            continue;
        }
        let fit = checks::subtype_fit(doc, layer, Some(before), &after, "attrs")
            .and_then(|_| checks::combination_fit(doc, layer, Some(before), &after, "attrs"))
            .and_then(|_| {
                rules
                    .fit(layer, Some(before), &after, None, "attrs")
                    .map(|_| ())
            });
        if let Err(stop) = fit {
            problem(&mut problems, &w.uid, &first, None, stop);
            invalid_slots.insert(w.slot.0);
            continue;
        }
        changed.push((w.slot, w.uid.clone(), before.clone(), after));
    }
    // Unique fields over the writes together: a clash takes the objects holding the clashing value out, again
    // until the rest fit (each round takes at least one out).
    loop {
        let uniques: Vec<checks::UniqueWrite<'_>> = changed
            .iter()
            .map(|(slot, _, before, after)| checks::UniqueWrite {
                slot: Some(*slot),
                layer,
                before: Some(before),
                after,
                at: "attrs".to_owned(),
            })
            .collect();
        let Err(stop) = checks::unique_fit(doc, &uniques) else {
            break;
        };
        let e = stop.into_error();
        let field = e
            .path
            .as_deref()
            .and_then(|p| p.rsplit('.').next())
            .unwrap_or_default()
            .to_owned();
        let written: BTreeSet<u32> = changed.iter().map(|c| c.0.0).collect();
        let others: BTreeSet<&String> = doc
            .by_layer_with_uids(layer)
            .filter(|(_, o)| !written.contains(&o.base().id))
            .filter_map(|(_, o)| o.base().attrs.get(&field))
            .collect();
        let clashes = |i: usize| {
            let Some(v) = changed[i].3.get(&field) else {
                return false;
            };
            others.contains(v)
                || changed
                    .iter()
                    .enumerate()
                    .any(|(j, c)| j != i && c.3.get(&field) == Some(v))
        };
        let mut hit: Vec<usize> = (0..changed.len()).filter(|&i| clashes(i)).collect();
        if hit.is_empty() {
            hit = (0..changed.len()).collect();
        }
        for &i in hit.iter().rev() {
            let (slot, uid, _, after) = changed.remove(i);
            problems.push(ValueProblem {
                uid,
                field: field.clone(),
                value: after.get(&field).cloned(),
                code: e.code.clone(),
                message: e.message.clone(),
            });
            invalid_slots.insert(slot.0);
        }
        if changed.is_empty() {
            break;
        }
    }
    let position: BTreeMap<&str, usize> = wants
        .iter()
        .enumerate()
        .map(|(i, w)| (w.uid.as_str(), i))
        .collect();
    problems.sort_by_key(|p| position.get(p.uid.as_str()).copied().unwrap_or(usize::MAX));
    let invalid = invalid_slots.len() as u32;
    let mut changes = Vec::new();
    let mut writes = Vec::with_capacity(changed.len());
    for (slot, uid, before, after) in &changed {
        let mut keys: BTreeSet<&String> = before.keys().collect();
        keys.extend(after.keys());
        for k in keys {
            if before.get(k) != after.get(k) && changes.len() < FIELDS_WRITE_LISTED {
                changes.push(FieldChange {
                    uid: uid.clone(),
                    field: k.clone(),
                    before: before.get(k).cloned(),
                    after: after.get(k).cloned(),
                });
            }
        }
        // A label that showed a value written over follows it.
        let label = doc.get(*slot).and_then(|e| e.base().label.clone());
        let follows = label.as_ref().and_then(|l| {
            before
                .iter()
                .find(|(k, v)| *v == l && after.get(*k) != Some(*v))
                .map(|(k, _)| after.get(k).cloned())
        });
        writes.push((*slot, after.clone(), follows));
    }
    let listed: Vec<ValueProblem> = problems.iter().take(FIELDS_WRITE_LISTED).cloned().collect();
    Ok(Planned {
        writes,
        out: FieldsWritten {
            changed: changed.len() as u32,
            unchanged,
            kept,
            invalid,
            changes,
            problems: listed,
            revision: doc.revision().to_string(),
        },
    })
}

/// The value a field refusal is about, by the path's last part.
fn stop_value(stop: &Stop, values: &BTreeMap<String, Option<String>>) -> Option<String> {
    let path = match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.path.as_deref()?,
        Stop::Cancelled => return None,
    };
    let key = path.rsplit('.').next()?;
    values.get(key).cloned().flatten()
}

fn decided(doc: &Document, input: &FieldsWrite) -> Result<(Planned, Vec<CommandWarning>), Stop> {
    let planned = run(doc, input)?;
    let mut warnings = Vec::new();
    if planned.out.invalid > 0 {
        let first = planned.out.problems.first();
        let said = first.map_or_else(String::new, |p| {
            format!(" İlki: {} ({}).", p.message, p.field)
        });
        match input.policy {
            WritePolicy::All => {
                return Err(Stop::failed(error(
                    codes::INVALID_VALUES,
                    format!(
                        "{} nesnenin değeri alanların kurallarına uymuyor; hiçbir değer yazılmadı.{said} Sorunların hepsi plan'da; uyanları yazmak için policy valid verin.",
                        planned.out.invalid
                    ),
                    Some("write".into()),
                )));
            }
            WritePolicy::Valid => warnings.push(CommandWarning {
                code: codes::VALUES_SKIPPED.into(),
                message: format!(
                    "{} nesnenin değeri kurallara uymadığı için yazılmadı.{said}",
                    planned.out.invalid
                ),
                path: Some("write".into()),
            }),
        }
    }
    Ok((planned, warnings))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &FieldsWrite) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What would change, problems listed, writing nothing; a whole-or-nothing write's problems are listed, not refused.
pub fn plan(cx: &ExecutionContext<'_>, input: &FieldsWrite) -> CommandResult<FieldsWritten> {
    match run(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: p.out,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: FieldsWrite) -> CommandResult<FieldsWritten> {
    let (planned, warnings) = match decided(cx.doc, &input) {
        Ok(d) => d,
        Err(stop) => return stop.into(),
    };
    let label = step_name(&input);
    if !planned.writes.is_empty() {
        let updates: Vec<(Slot, Entity)> = planned
            .writes
            .iter()
            .filter_map(|(slot, attrs, follows)| {
                let mut e = cx.doc.get(*slot)?.clone();
                let base = e.base_mut();
                base.attrs = attrs.clone();
                if let Some(l) = follows {
                    base.label = l.clone();
                }
                Some((*slot, e))
            })
            .collect();
        cx.doc.update_many(updates, &label);
    }
    let mut out = planned.out;
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed {
        output: out,
        warnings,
    }
}
