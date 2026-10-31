//! Flat modules' acceptance (docs/adr/0779): the shared cases' two-room flat defined as a module, a second flat placed
//! beside it in its own group of the building program, the ground floor repeated on the first in one step (each flat
//! numbered and grouped for itself); a fixture added to the definition reaches all four, a dimension taken out leaves
//! three copies deleted; a fixture moved by hand is said drifted and a source update writes it back; each write is one
//! undo step; the drawing written as KCAD (schema 144) and read back keeps the modules and the flats' groups. And the
//! timings of a ten-storey block (ignored, release).

use std::time::Instant;

use kentos_commands::{
    CommandResult, DocumentSnapshotV2, Entity, ModulesEdit, ModulesEdited, ModulesQuery,
};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, modules_edit, modules_query};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/commands/v1/cad.modules.edit.flats.json");

fn drawing() -> Document {
    let file: Value = serde_json::from_str(CASES).unwrap();
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(file["setup"].clone()).unwrap();
    Document::from_snapshot_v2(snapshot).unwrap()
}

fn uid(n: u32) -> String {
    format!("0192f5a1-7790-7000-8000-{n:012x}")
}

fn edit(doc: &mut Document, op: Value) -> ModulesEdited {
    let input: ModulesEdit = serde_json::from_value(json!({ "operation": op })).unwrap();
    match modules_edit::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn entity<'a>(doc: &'a Document, uid: &str) -> Option<&'a Entity> {
    doc.get(doc.slot_of(Uuid::parse_str(uid).ok()?)?)
}

fn define(doc: &mut Document) {
    edit(
        doc,
        json!({"kind": "define", "name": "Daire", "uids": (1..=13).map(uid).collect::<Vec<_>>(),
               "base": {"x": 0, "y": 0}}),
    );
}

/// The spaces of the module's instances: (instance, level, group, number).
fn spaces(doc: &Document) -> Vec<(String, Option<String>, Option<String>, Option<String>)> {
    let mut out = Vec::new();
    for m in doc.arch_modules() {
        for i in &m.instances {
            for o in &i.members {
                if let Some(Entity::Space(s)) = entity(doc, &o.uid) {
                    out.push((
                        i.id.clone(),
                        s.space.arch.level.clone(),
                        s.space.unit.clone(),
                        s.space.number.clone(),
                    ));
                }
            }
        }
    }
    out
}

fn drifted(doc: &Document) -> Vec<String> {
    match modules_query::query(doc, &ModulesQuery::default()) {
        CommandResult::Completed { output, .. } => output
            .modules
            .iter()
            .flat_map(|m| m.instances.iter().flat_map(|i| i.drifted.clone()))
            .collect(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn four_flats_on_two_floors_each_in_its_own_group_follow_their_source() {
    let mut doc = drawing();
    define(&mut doc);
    let second = edit(
        &mut doc,
        json!({"kind": "place", "module": "Daire", "at": {"x": 10, "y": 0}, "mirror": true, "level": "zemin",
               "numbering": {"prefix": "2-"}, "unit": "d2"}),
    );
    assert_eq!(second.created.len(), 13);
    let before_repeat = doc.to_snapshot_v2();
    let repeated = edit(
        &mut doc,
        json!({"kind": "repeat", "from": "zemin", "to": [{"level": "kat1", "instances": [
            {"module": "daire", "instance": "daire-1", "numbering": {"prefix": "3-"}, "unit": "d3"},
            {"module": "daire", "instance": "daire-2", "numbering": {"prefix": "4-"}, "unit": "d4"}]}]}),
    );
    assert_eq!(repeated.placed.len(), 2);
    assert_eq!(repeated.created.len(), 26);
    // Every flat's two rooms in its own group, on its own floor, numbered for itself.
    let mut rooms = spaces(&doc);
    rooms.sort();
    let want: Vec<(&str, &str, &str, &str)> = vec![
        ("daire-1", "zemin", "d1", "S1"),
        ("daire-1", "zemin", "d1", "Y1"),
        ("daire-2", "zemin", "d2", "2-S1"),
        ("daire-2", "zemin", "d2", "2-Y1"),
        ("daire-3", "kat1", "d3", "3-S1"),
        ("daire-3", "kat1", "d3", "3-Y1"),
        ("daire-4", "kat1", "d4", "4-S1"),
        ("daire-4", "kat1", "d4", "4-Y1"),
    ];
    let got: Vec<(&str, &str, &str, &str)> = rooms
        .iter()
        .map(|(i, l, u, n)| {
            (
                i.as_str(),
                l.as_deref().unwrap_or(""),
                u.as_deref().unwrap_or(""),
                n.as_deref().unwrap_or(""),
            )
        })
        .collect();
    assert_eq!(got, want);
    assert!(drifted(&doc).is_empty(), "nothing changed by hand yet");
    // No link broken: every door on its own wall, every room found from its own walls.
    for e in doc.entities() {
        match e {
            Entity::Opening(o) => assert!(o.opening.broken.is_none(), "{:?}", o.opening),
            Entity::Space(s) => assert!(s.space.broken.is_none(), "{:?}", s.space.broken),
            Entity::Dimension(d) => assert!(
                d.refs
                    .as_ref()
                    .is_none_or(|r| r.a.as_ref().is_none_or(|a| a.broken.is_none())
                        && r.b.as_ref().is_none_or(|b| b.broken.is_none())),
                "{:?}",
                d.refs
            ),
            _ => {}
        }
    }
    // One undo takes the repeat back whole.
    doc.undo();
    assert_eq!(doc.to_snapshot_v2().entities, before_repeat.entities);
    assert_eq!(doc.to_snapshot_v2().settings, before_repeat.settings);
    doc.redo();
    // A second bed added in the first flat reaches the three others, each in its own place.
    let added = edit(
        &mut doc,
        json!({"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [uid(14)]}),
    );
    assert_eq!(added.created.len(), 3);
    assert_eq!(added.report.updated.len(), 4);
    let module = doc.arch_modules()[0].clone();
    assert_eq!(module.version, 2);
    assert!(module.instances.iter().all(|i| i.members.len() == 14));
    // The mirrored flat's bed stands mirrored: x = 10 − 1.
    let mirrored = module.instance("daire-2").unwrap();
    let bed = &mirrored
        .members
        .iter()
        .find(|o| o.key == uid(14))
        .unwrap()
        .uid;
    let Some(Entity::Insert(b)) = entity(&doc, bed) else {
        panic!()
    };
    assert!((b.p.x - 9.0).abs() < 1e-9 && (b.p.y - 3.0).abs() < 1e-9);
    // The dimension taken out: it stays in the first flat, the three copies go.
    let dims_before = doc
        .entities()
        .filter(|e| matches!(e, Entity::Dimension(_)))
        .count();
    let removed = edit(
        &mut doc,
        json!({"kind": "removeMembers", "module": "Daire", "uids": [uid(13)]}),
    );
    assert_eq!(removed.report.deleted.len(), 3);
    assert_eq!(removed.report.released, [uid(13)]);
    assert_eq!(
        doc.entities()
            .filter(|e| matches!(e, Entity::Dimension(_)))
            .count(),
        dims_before - 3
    );
    // A bed moved by hand in the first-floor flat is drifted; the source update after a door change writes it back.
    let upstairs = doc.arch_modules()[0].instance("daire-3").unwrap().clone();
    let moved = upstairs
        .members
        .iter()
        .find(|o| o.key == uid(10))
        .unwrap()
        .uid
        .clone();
    let slot = doc.slot_of(Uuid::parse_str(&moved).unwrap()).unwrap();
    let mut e = doc.get(slot).unwrap().clone();
    if let Entity::Insert(b) = &mut e {
        b.p.x += 0.5;
    }
    doc.update(slot, e);
    assert_eq!(drifted(&doc), [moved.clone()]);
    // The first flat's bed turned: the source changes on that key.
    let bed_slot = doc.slot_of(Uuid::parse_str(&uid(10)).unwrap()).unwrap();
    let mut first_bed = doc.get(bed_slot).unwrap().clone();
    if let Entity::Insert(b) = &mut first_bed {
        b.rotation = 0.1;
    }
    doc.update(bed_slot, first_bed);
    let updated = edit(
        &mut doc,
        json!({"kind": "update", "module": "Daire", "from": "daire-1"}),
    );
    assert!(
        updated
            .report
            .conflicts
            .iter()
            .any(|c| c.uid.as_deref() == Some(moved.as_str())
                && c.why == kentos_commands::ModuleConflictWhy::Drifted)
    );
    assert!(drifted(&doc).is_empty(), "the source update wrote it back");
    // KCAD 144: written and read back, the modules, their groups and the flats' rooms are the same.
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
    let again = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert_eq!(again.arch_modules(), doc.arch_modules());
    assert_eq!(spaces(&again), spaces(&doc));
    assert!(drifted(&again).is_empty());
}

#[test]
#[ignore]
fn flat_modules_timing() {
    let mut doc = drawing();
    define(&mut doc);
    for k in 1..10 {
        edit(
            &mut doc,
            json!({"kind": "place", "module": "Daire", "at": {"x": 7.0 * f64::from(k), "y": 0}, "level": "zemin"}),
        );
    }
    let mut levels = Vec::new();
    {
        let mut settings = doc.settings().clone();
        let arch = settings.arch.as_mut().unwrap();
        for n in 1..10 {
            let id = format!("k{n}");
            arch.levels.push(kentos_commands::ArchLevel {
                id: id.clone(),
                name: format!("{n}. kat"),
                elevation: 3.0 * f64::from(n) + 9.0,
                height: Some(3.0),
            });
            levels.push(id);
        }
        doc.set_settings(settings);
    }
    let to: Vec<Value> = levels.iter().map(|l| json!({"level": l})).collect();
    let input: ModulesEdit =
        serde_json::from_value(json!({"operation": {"kind": "repeat", "from": "zemin", "to": to}}))
            .unwrap();
    let t = Instant::now();
    let _ = modules_edit::plan_in(&doc, &input);
    let plan_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let done = modules_edit::execute(&mut ExecutionContext::new(&mut doc), input);
    let write_ms = t.elapsed().as_secs_f64() * 1000.0;
    let CommandResult::Completed { output, .. } = done else {
        panic!("{done:?}")
    };
    let t = Instant::now();
    let _ = drifted(&doc);
    let drift_ms = t.elapsed().as_secs_f64() * 1000.0;
    let add: ModulesEdit = serde_json::from_value(json!({"operation": {
        "kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [uid(14)]}}))
    .unwrap();
    let t = Instant::now();
    let added = modules_edit::execute(&mut ExecutionContext::new(&mut doc), add);
    let add_ms = t.elapsed().as_secs_f64() * 1000.0;
    assert!(matches!(added, CommandResult::Completed { .. }));
    println!(
        "10 daire × 10 kat: tekrarın planı {plan_ms:.1} ms, yazması {write_ms:.1} ms ({} örnek, {} nesne); \
         elle değişenlerin sorgusu {drift_ms:.1} ms; 100 örneğe üye ekleme {add_ms:.1} ms",
        output.placed.len(),
        output.created.len()
    );
    assert_eq!(output.placed.len(), 90);
    assert!(plan_ms < 500.0 && write_ms < 1000.0 && drift_ms < 500.0 && add_ms < 500.0);
}
