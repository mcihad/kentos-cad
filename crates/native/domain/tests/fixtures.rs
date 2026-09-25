//! The shared document fixtures (fixtures/document-ops/v1, TODOS.md DOM-02)
//! run against the native document. They describe the web's `CadDocument`,
//! which runs the same files (apps/web/src/model/documentOps.test.ts); the
//! format is in fixtures/document-ops/README.md.
//!
//! The web takes field patches (`update(id, { p })`); the native document takes
//! whole objects. This runner writes a patch over the object as JSON (a null
//! removes the field) and hands the result to the document, the one
//! translation between the two APIs.

use std::collections::HashMap;
use std::path::PathBuf;

use kentos_domain::contracts::{DocumentSnapshotV1, Entity, LayerStyle};
use kentos_domain::{Document, Group, Slot};
use serde_json::{Map, Value, json};

/// Why a step stopped: the fixture's own `throw`, or a failed expectation.
enum Stop {
    Thrown(String),
    Failed(String),
}

type Outcome<T> = Result<T, Stop>;

fn fail<T>(message: String) -> Outcome<T> {
    Err(Stop::Failed(message))
}

#[derive(Default)]
struct State {
    groups: Vec<Group>,
    revisions: HashMap<String, u64>,
}

fn text<'a>(step: &'a Value, field: &str, at: &str) -> Outcome<&'a str> {
    match step.get(field).and_then(Value::as_str) {
        Some(value) => Ok(value),
        None => fail(format!("{at}: “{field}” metni yok")),
    }
}

fn number(step: &Value, field: &str, at: &str) -> Outcome<u64> {
    match step.get(field).and_then(Value::as_u64) {
        Some(value) => Ok(value),
        None => fail(format!("{at}: “{field}” sayısı yok")),
    }
}

fn slot(value: &Value, at: &str) -> Outcome<Slot> {
    match value.as_u64().and_then(|n| u32::try_from(n).ok()) {
        Some(n) => Ok(Slot(n)),
        None => fail(format!("{at}: nesne kimliği değil: {value}")),
    }
}

fn list<'a>(step: &'a Value, field: &str, at: &str) -> Outcome<&'a Vec<Value>> {
    match step.get(field).and_then(Value::as_array) {
        Some(values) => Ok(values),
        None => fail(format!("{at}: “{field}” listesi yok")),
    }
}

/// A new object as the fixture writes it (no id): the document numbers it.
fn new_entity(value: &Value, at: &str) -> Outcome<Entity> {
    let mut value = value.clone();
    if let Some(object) = value.as_object_mut() {
        object.insert("id".into(), json!(0));
    }
    serde_json::from_value(value).or_else(|e| fail(format!("{at}: nesne okunamadı: {e}")))
}

/// `patch` over `base`, as the web's `{ ...before, ...patch }`: a null removes the field.
fn patched(mut base: Value, patch: &Value, at: &str) -> Outcome<Value> {
    let (Some(object), Some(patch)) = (base.as_object_mut(), patch.as_object()) else {
        return fail(format!("{at}: yama bir nesne olmalı"));
    };
    for (key, value) in patch {
        if value.is_null() {
            object.remove(key);
        } else {
            object.insert(key.clone(), value.clone());
        }
    }
    Ok(base)
}

fn entity_json(doc: &Document, slot: Slot, at: &str) -> Outcome<Option<Value>> {
    doc.get(slot)
        .map(serde_json::to_value)
        .transpose()
        .or_else(|e| fail(format!("{at}: {e}")))
}

fn entity_of(value: Value, slot: Slot, at: &str) -> Outcome<Entity> {
    let mut value = value;
    if let Some(object) = value.as_object_mut() {
        object.insert("id".into(), json!(slot.0));
    }
    serde_json::from_value(value).or_else(|e| fail(format!("{at}: yamalı nesne okunamadı: {e}")))
}

/// JSON equality with numbers compared as numbers (the file's `1` is the document's `1.0`).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

fn expect_same(got: &Value, want: &Value, what: &str, at: &str) -> Outcome<()> {
    if same(got, want) {
        Ok(())
    } else {
        fail(format!("{at}: {what}: beklenen {want}, bulunan {got}"))
    }
}

fn run_steps(doc: &mut Document, state: &mut State, steps: &Value, at: &str) -> Outcome<()> {
    let Some(steps) = steps.as_array() else {
        return fail(format!("{at}: adım listesi yok"));
    };
    for (i, step) in steps.iter().enumerate() {
        let op = step.get("op").and_then(Value::as_str).unwrap_or("?");
        run_step(doc, state, step, &format!("{at} › {} {op}", i + 1))?;
    }
    Ok(())
}

fn run_step(doc: &mut Document, state: &mut State, step: &Value, at: &str) -> Outcome<()> {
    let before = doc.revision();
    let caught = step.get("catch").and_then(Value::as_str);
    match (apply(doc, state, step, at), caught) {
        (Err(Stop::Thrown(message)), Some(want)) => {
            if message != want {
                return fail(format!(
                    "{at}: yakalanan hata “{message}”, beklenen “{want}”"
                ));
            }
        }
        (Err(stop), _) => return Err(stop),
        (Ok(_), Some(want)) => {
            return fail(format!(
                "{at}: “{want}” hatası bekleniyordu, adım hatasız bitti"
            ));
        }
        (Ok(result), None) => {
            if let Some(want) = step.get("returns") {
                expect_same(&result, want, "dönen değer", at)?;
            }
        }
    }
    match step.get("expect") {
        Some(expect) => check(doc, expect, before, at),
        None => Ok(()),
    }
}

fn apply(doc: &mut Document, state: &mut State, step: &Value, at: &str) -> Outcome<Value> {
    let op = text(step, "op", at)?;
    let id = || text(step, "id", at);
    let label = step.get("label").and_then(Value::as_str);
    Ok(match op {
        "check" => Value::Null,
        "add" => {
            let entity = new_entity(step.get("entity").unwrap_or(&Value::Null), at)?;
            match doc.add(entity) {
                Ok(slot) => json!(slot.0),
                Err(e) => return fail(format!("{at}: {e}")),
            }
        }
        "addMany" => {
            let entities = list(step, "entities", at)?
                .iter()
                .map(|e| new_entity(e, at))
                .collect::<Outcome<Vec<_>>>()?;
            match doc.add_many(entities, label.unwrap_or(kentos_domain::labels::ADD)) {
                Ok(slots) => json!(slots.iter().map(|s| s.0).collect::<Vec<_>>()),
                Err(e) => return fail(format!("{at}: {e}")),
            }
        }
        "update" => {
            let target = slot(step.get("id").unwrap_or(&Value::Null), at)?;
            if let Some(current) = entity_json(doc, target, at)? {
                let entity = entity_of(patched(current, &step["patch"], at)?, target, at)?;
                doc.update(target, entity);
            }
            Value::Null
        }
        "updateMany" => {
            // A slot given twice: its second patch goes over what the first made.
            let mut latest: HashMap<Slot, Value> = HashMap::new();
            let mut changes = Vec::new();
            for patch in list(step, "patches", at)? {
                let target = slot(patch.get("id").unwrap_or(&Value::Null), at)?;
                let current = match latest.get(&target) {
                    Some(value) => Some(value.clone()),
                    None => entity_json(doc, target, at)?,
                };
                let Some(current) = current else { continue };
                let value = patched(current, patch, at)?;
                latest.insert(target, value.clone());
                changes.push((target, entity_of(value, target, at)?));
            }
            json!(doc.update_many(changes, label.unwrap_or(kentos_domain::labels::CHANGE)))
        }
        "remove" => {
            let slots = list(step, "ids", at)?
                .iter()
                .map(|v| slot(v, at))
                .collect::<Outcome<Vec<_>>>()?;
            doc.remove(&slots);
            Value::Null
        }
        "transact" => {
            let label = text(step, "label", at)?.to_owned();
            let steps = step.get("steps").cloned().unwrap_or(json!([]));
            let throw = step.get("throw").and_then(Value::as_str).map(str::to_owned);
            doc.transact(&label, |doc| {
                run_steps(doc, state, &steps, at)?;
                match throw {
                    Some(message) => Err(Stop::Thrown(message)),
                    None => Ok(()),
                }
            })?;
            Value::Null
        }
        "beginGroup" => {
            let group = doc.begin_group(text(step, "label", at)?);
            state.groups.push(group);
            Value::Null
        }
        "endGroup" | "cancelGroup" => {
            let Some(group) = state.groups.pop() else {
                return fail(format!("{at}: açık grup yok"));
            };
            if op == "endGroup" {
                doc.end_group(group);
            } else {
                doc.cancel_group(group);
            }
            Value::Null
        }
        "undo" => json!(doc.undo()),
        "redo" => json!(doc.redo()),
        "captureRevision" => {
            state
                .revisions
                .insert(text(step, "as", at)?.to_owned(), doc.revision());
            Value::Null
        }
        "markSaved" => {
            let name = text(step, "revision", at)?;
            let Some(revision) = state.revisions.get(name) else {
                return fail(format!("{at}: “{name}” sürümü alınmadı"));
            };
            doc.mark_saved(*revision);
            Value::Null
        }
        "markUnsaved" => {
            doc.mark_unsaved();
            Value::Null
        }
        "repeat" => {
            let steps = step.get("steps").cloned().unwrap_or(json!([]));
            for k in 0..number(step, "times", at)? {
                run_steps(doc, state, &steps, &format!("{at} ({}.)", k + 1))?;
            }
            Value::Null
        }
        "setVisible" => {
            let visible = step.get("visible").and_then(Value::as_bool).unwrap_or(true);
            doc.set_layer_visible(id()?, visible);
            Value::Null
        }
        "toggleVisible" => {
            doc.toggle_layer_visible(id()?);
            Value::Null
        }
        "toggleLocked" => {
            doc.toggle_layer_locked(id()?);
            Value::Null
        }
        "isolate" => {
            doc.isolate_layer(id()?);
            Value::Null
        }
        "showAll" => {
            doc.show_all_layers();
            Value::Null
        }
        "setExpanded" => {
            let expanded = step
                .get("expanded")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            doc.set_layer_expanded(id()?, expanded);
            Value::Null
        }
        "setActive" => {
            doc.set_active_layer(id()?);
            Value::Null
        }
        "rename" => {
            doc.rename_layer(id()?, text(step, "name", at)?);
            Value::Null
        }
        "setLayerStyle" => {
            let layer = id()?;
            // An unknown layer gets the patch over the default look; the document ignores it.
            let current = match doc.layers().get(layer) {
                Some(node) => {
                    serde_json::to_value(&node.style).or_else(|e| fail(format!("{at}: {e}")))?
                }
                None => json!({ "color": "fg", "lineType": "continuous", "lineWeight": 0.18 }),
            };
            let style: LayerStyle = serde_json::from_value(patched(current, &step["patch"], at)?)
                .or_else(|e| fail(format!("{at}: stil okunamadı: {e}")))?;
            doc.set_layer_style(
                layer,
                style,
                label.unwrap_or(kentos_domain::labels::LAYER_STYLE),
            );
            Value::Null
        }
        other => return fail(format!("{at}: bilinmeyen işlem “{other}”")),
    })
}

fn check(doc: &Document, expect: &Value, before: u64, at: &str) -> Outcome<()> {
    let empty = Map::new();
    let fields = expect.as_object().unwrap_or(&empty);
    for (key, want) in fields {
        match key.as_str() {
            "ids" => {
                let ids: Vec<u32> = doc.entities().map(|e| e.base().id).collect();
                expect_same(&json!(ids), want, "nesneler", at)?;
            }
            "count" => expect_same(&json!(doc.len()), want, "nesne sayısı", at)?,
            "entities" => {
                for (id, entity) in want.as_object().unwrap_or(&empty) {
                    let target = slot(&json!(id.parse::<u64>().unwrap_or(0)), at)?;
                    let got = entity_json(doc, target, at)?.unwrap_or(Value::Null);
                    expect_same(&got, entity, &format!("nesne {id}"), at)?;
                }
            }
            "byLayer" => {
                for (layer, ids) in want.as_object().unwrap_or(&empty) {
                    let got: Vec<u32> = doc.by_layer(layer).map(|e| e.base().id).collect();
                    expect_same(
                        &json!(got),
                        ids,
                        &format!("“{layer}” katmanının nesneleri"),
                        at,
                    )?;
                }
            }
            "canUndo" => expect_same(&json!(doc.can_undo()), want, "canUndo", at)?,
            "canRedo" => expect_same(&json!(doc.can_redo()), want, "canRedo", at)?,
            "dirty" => expect_same(&json!(doc.is_dirty()), want, "dirty", at)?,
            "revision" => {
                let got = if doc.revision() == before {
                    "same"
                } else {
                    "changed"
                };
                expect_same(&json!(got), want, "sürüm", at)?;
            }
            "activeLayer" => expect_same(&json!(doc.layers().active()), want, "etkin katman", at)?,
            "layers" => {
                for (id, fields) in want.as_object().unwrap_or(&empty) {
                    let Some(node) = doc.layers().get(id) else {
                        return fail(format!("{at}: “{id}” katmanı yok"));
                    };
                    for (field, value) in fields.as_object().unwrap_or(&empty) {
                        let got = match field.as_str() {
                            "visible" => json!(node.visible),
                            "locked" => json!(node.locked),
                            "expanded" => json!(node.expanded),
                            "name" => json!(node.name),
                            "style" => serde_json::to_value(&node.style).unwrap_or(Value::Null),
                            "isVisible" => json!(doc.layers().is_visible(id)),
                            "isLocked" => json!(doc.layers().is_locked(id)),
                            other => {
                                return fail(format!("{at}: bilinmeyen katman alanı “{other}”"));
                            }
                        };
                        expect_same(&got, value, &format!("“{id}” katmanı › {field}"), at)?;
                    }
                }
            }
            other => return fail(format!("{at}: bilinmeyen beklenti “{other}”")),
        }
    }
    Ok(())
}

fn run_scenario(setup: &Value, scenario: &Value, at: &str) -> Result<(), String> {
    let snapshot = DocumentSnapshotV1::from_json(&setup.to_string())
        .map_err(|e| format!("{at}: kurulum: {e}"))?;
    let mut doc = Document::from_snapshot(snapshot).map_err(|e| format!("{at}: kurulum: {e}"))?;
    if doc.is_dirty() {
        return Err(format!("{at}: kurulumdan sonra belge kirli"));
    }
    let mut state = State::default();
    let steps = scenario.get("steps").cloned().unwrap_or(json!([]));
    run_steps(&mut doc, &mut state, &steps, at).map_err(|stop| match stop {
        Stop::Failed(message) => message,
        Stop::Thrown(message) => format!("{at}: yakalanmayan hata “{message}”"),
    })
}

#[test]
fn every_document_fixture_matches_the_native_document() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/document-ops/v1");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures/document-ops/v1")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(
        files.len() >= 6,
        "fixtures/document-ops/v1: {} files",
        files.len()
    );
    let mut problems = Vec::new();
    let mut scenarios = 0;
    for path in &files {
        let file = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let fixture: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
                .expect("JSON");
        assert_eq!(fixture["format"], "kentos.document-ops", "{file}");
        assert_eq!(fixture["version"], 1, "{file}");
        for scenario in fixture["scenarios"].as_array().expect("scenarios") {
            scenarios += 1;
            let name = scenario["name"].as_str().unwrap_or("?");
            let setup = scenario.get("setup").unwrap_or(&fixture["setup"]);
            if let Err(problem) = run_scenario(setup, scenario, &format!("{file} › {name}")) {
                problems.push(problem);
            }
        }
    }
    assert!(scenarios >= 30, "{scenarios} scenarios");
    assert!(
        problems.is_empty(),
        "{} of {scenarios} scenarios failed:\n{}",
        problems.len(),
        problems.join("\n")
    );
}
