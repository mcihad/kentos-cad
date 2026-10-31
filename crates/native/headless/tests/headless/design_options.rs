//! Design options' acceptance (docs/adr/0788): a flat's kitchen defined as option A, copied as B, B's partition moved a
//! metre; the options do not see each other (B's partition joins nothing while A is primary, B's kitchen finds its
//! boundary among B's walls, the main living room among A's; two copies of a door on the main wall do not clash);
//! B made primary, the living room grows and the store shows B; two options compared; KCAD keeps the sets; DXF writes
//! only what is drawn and says the rest; the sheet's look hides what a view's recipe does not choose; accepting B
//! deletes A's objects in one undo step. And the timings of a set of many options (ignored, release).

use std::time::Instant;

use kentos_commands::{
    CommandResult, DocumentSnapshotV2, Entity, OptionChoice, OptionsEdit, OptionsEdited, OptionsQuery,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_headless::export;
use kentos_native_application::{ExecutionContext, options_edit, options_query};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/commands/v1/cad.options.edit.json");

fn uid(n: u32) -> String {
    format!("0192f5a1-7880-7000-8000-{n:012x}")
}

/// The shared cases' flat with its two spaces seeded: the kitchen right of the partition, the living room left of it.
fn drawing() -> Document {
    let file: Value = serde_json::from_str(CASES).unwrap();
    let mut setup = file["setup"].clone();
    for e in setup["entities"].as_array_mut().unwrap() {
        match e["id"].as_u64() {
            Some(7) => e["seed"] = json!({"x": 4.5, "y": 2.0}),
            Some(11) => e["seed"] = json!({"x": 1.5, "y": 2.0}),
            _ => {}
        }
    }
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    Document::from_snapshot_v2(snapshot).unwrap()
}

fn kitchen() -> Vec<String> {
    [5, 6, 7, 8, 9].map(uid).to_vec()
}

fn edit(doc: &mut Document, op: Value) -> OptionsEdited {
    let input: OptionsEdit = serde_json::from_value(json!({ "operation": op })).unwrap();
    match options_edit::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn slot(doc: &Document, uid: &str) -> Slot {
    doc.slot_of(Uuid::parse_str(uid).unwrap()).unwrap()
}

fn area(doc: &Document, uid: &str) -> f64 {
    let Some(Entity::Space(s)) = doc.get(slot(doc, uid)) else {
        panic!("not a space")
    };
    s.space
        .parts
        .iter()
        .map(|p| {
            let pts = &p.outer.pts;
            let n = pts.len();
            (0..n)
                .map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y)
                .sum::<f64>()
                .abs()
                / 2.0
        })
        .sum()
}

fn moved(doc: &mut Document, uid: &str, dx: f64) {
    let input: kentos_commands::EntitiesTransform = serde_json::from_value(
        json!({"uids": [uid], "transform": {"kind": "move", "dx": dx, "dy": 0.0}}),
    )
    .unwrap();
    assert!(matches!(
        kentos_native_application::transform::execute(&mut ExecutionContext::new(doc), input),
        CommandResult::Completed { .. }
    ));
}

#[test]
fn a_kitchen_with_two_options_draws_counts_and_accepts_one() {
    let mut doc = drawing();
    let salon = uid(11);
    let before = area(&doc, &salon);
    assert!((before - 2.9 * 3.9).abs() < 1e-9);
    edit(
        &mut doc,
        json!({"kind": "define", "name": "Mutfak", "uids": kitchen()}),
    );
    let b = edit(&mut doc, json!({"kind": "copy", "set": "mutfak", "option": "a"})).created;
    assert_eq!(b.len(), 5);
    // B's partition a metre right: B's kitchen finds its boundary among B's walls, the living room among A's.
    moved(&mut doc, &b[0], 1.0);
    assert!((area(&doc, &b[2]) - 1.9 * 3.9).abs() < 1e-9, "B's kitchen");
    assert!((area(&doc, &uid(7)) - 2.9 * 3.9).abs() < 1e-9, "A's kitchen");
    assert!((area(&doc, &salon) - before).abs() < 1e-9, "the living room sees A");
    // The store: B left out, its partition joining nothing; A shown.
    let store = kentos_native_application::geometry_store::store_of(&doc);
    assert!(!store.view_shown(f64::from(slot(&doc, &b[0]).0)));
    assert!(store.view_shown(f64::from(slot(&doc, &uid(5)).0)));
    let issues = store.wall_issues().len();
    // B primary: the living room grows, the store shows B and hides A.
    edit(&mut doc, json!({"kind": "primary", "set": "mutfak", "option": "b"}));
    assert!((area(&doc, &salon) - 3.9 * 3.9).abs() < 1e-9, "the living room sees B");
    let store = kentos_native_application::geometry_store::store_of(&doc);
    assert!(store.view_shown(f64::from(slot(&doc, &b[0]).0)));
    assert!(!store.view_shown(f64::from(slot(&doc, &uid(5)).0)));
    assert_eq!(store.wall_issues().len(), issues, "the two options' walls never meet");
    // The comparison: the partition and its door moved, the kitchens' areas differ.
    let q: OptionsQuery = serde_json::from_value(json!({"compare": {"set": "mutfak", "a": "a", "b": "b", "search": 1.5}})).unwrap();
    let CommandResult::Completed { output, .. } = options_query::query(&doc, &q) else {
        panic!()
    };
    let c = output.compared.unwrap();
    assert!(c.changed.len() >= 2, "{c:?}");
    assert!((c.space_area - (1.9 - 2.9) * 3.9).abs() < 1e-9);
    // Two copies of a door on the main wall, in two options: neither is broken as overlapping.
    edit(&mut doc, json!({"kind": "define", "name": "Giriş", "uids": [uid(10)]}));
    let door = edit(&mut doc, json!({"kind": "copy", "set": "giris", "option": "a"})).created;
    for u in [uid(10), door[0].clone()] {
        let Some(Entity::Opening(o)) = doc.get(slot(&doc, &u)) else {
            panic!()
        };
        assert!(o.opening.broken.is_none(), "{u}");
    }
    // The paper's look: the project's hides A (B primary), a recipe choosing A hides B.
    let look = kentos_plot::layers::phasing_of(&doc);
    assert_eq!(look.look(slot(&doc, &uid(5)).0), kentos_commands::PhaseLook::Hidden);
    assert_eq!(look.look(slot(&doc, &b[0]).0), kentos_commands::PhaseLook::Shown);
    let a = [OptionChoice {
        set: "mutfak".into(),
        option: "a".into(),
    }];
    let map = kentos_plot::layers::map_phasing_of(&doc, None, &[], &a);
    assert_eq!(map.look(slot(&doc, &uid(5)).0), kentos_commands::PhaseLook::Shown);
    assert_eq!(map.look(slot(&doc, &b[0]).0), kentos_commands::PhaseLook::Hidden);
    // KCAD keeps the sets.
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
    let again = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert_eq!(again.arch_options(), doc.arch_options());
    // DXF: only what is drawn, the rest said.
    let input = export::dxf_input(&doc, &export::by_layer(&doc, doc.entities()));
    assert!(!input.entities.iter().any(|e| e.base().id == slot(&doc, &uid(5)).0));
    let note = input
        .notes
        .iter()
        .find(|n| n.what == "Birincil olmayan tasarım seçeneği")
        .expect("the DXF report says it");
    assert_eq!(note.count, 6, "A's five objects and the door's other copy");
    // Accepting B: A's objects go, the set goes, the living room keeps B's size; one undo gives all back.
    let kept = doc.to_snapshot_v2();
    let done = edit(&mut doc, json!({"kind": "accept", "set": "mutfak", "option": "b"}));
    assert_eq!(done.deleted.len(), 5);
    assert!(doc.slot_of(Uuid::parse_str(&uid(5)).unwrap()).is_none());
    assert_eq!(doc.arch_options().len(), 1);
    assert!((area(&doc, &salon) - 3.9 * 3.9).abs() < 1e-9);
    doc.undo();
    let back = doc.to_snapshot_v2();
    assert_eq!(back.entities, kept.entities);
    assert_eq!(back.settings, kept.settings);
}

#[test]
fn a_drawing_without_options_writes_no_option_note() {
    let doc = drawing();
    let input = export::dxf_input(&doc, &export::by_layer(&doc, doc.entities()));
    assert!(!input.notes.iter().any(|n| n.what == "Birincil olmayan tasarım seçeneği"));
    assert!(doc.option_out().is_empty());
}

#[test]
#[ignore]
fn design_options_timing() {
    let mut doc = drawing();
    edit(
        &mut doc,
        json!({"kind": "define", "name": "Mutfak", "uids": kitchen()}),
    );
    let t = Instant::now();
    for _ in 0..11 {
        edit(&mut doc, json!({"kind": "copy", "set": "mutfak", "option": "a"}));
    }
    let copy_ms = t.elapsed().as_secs_f64() * 1000.0 / 11.0;
    let t = Instant::now();
    edit(&mut doc, json!({"kind": "primary", "set": "mutfak", "option": "l"}));
    let primary_ms = t.elapsed().as_secs_f64() * 1000.0;
    let q: OptionsQuery = serde_json::from_value(json!({"compare": {"set": "mutfak", "a": "a", "b": "l"}})).unwrap();
    let t = Instant::now();
    let _ = options_query::query(&doc, &q);
    let compare_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let _ = kentos_native_application::geometry_store::store_of(&doc);
    let store_ms = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "12 seçenek: kopya {copy_ms:.2} ms, birincil {primary_ms:.2} ms, karşılaştırma {compare_ms:.2} ms, depo {store_ms:.2} ms"
    );
    assert!(copy_ms < 100.0 && primary_ms < 200.0 && compare_ms < 100.0);
}
