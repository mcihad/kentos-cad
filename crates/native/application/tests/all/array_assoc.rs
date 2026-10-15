//! Associative arrays over a large drawing (docs/adr/0300 §6): what making a
//! 100 × 100 grid array of a small symbol takes, what a path array's 2 000
//! items following their path takes when the path changes, and the undo of
//! that step. The shared cases are `fixtures/commands/v1/cad.entities.array.assoc.json`.

use kentos_domain::contracts::{
    ArrayLayout, CommandResult, DocumentSnapshotV1, EntitiesArray, Entity, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, array};
use serde_json::json;

/// A 10 000 m zigzag (1) and a symbol of three lines (2–4) at its start.
fn drawing() -> Document {
    let pts: Vec<_> = (0..=100)
        .map(|i| json!({"x": f64::from(i) * 100.0, "y": if i % 2 == 0 { 0.0 } else { 5.0 }}))
        .collect();
    let line = |id: u32, a: [f64; 2], b: [f64; 2]| json!({"kind":"line","id":id,"layerId":"a","attrs":{},"a":{"x":a[0],"y":a[1]},"b":{"x":b[0],"y":b[1]}});
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Dizi","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"polyline","id":1,"layerId":"a","attrs":{},"pts":pts},
        line(2, [-0.5, 0.0], [0.5, 0.0]), line(3, [0.0, 0.0], [0.0, 2.0]), line(4, [-0.5, 2.0], [0.5, 2.0])],
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn input(doc: &Document, layout: ArrayLayout) -> EntitiesArray {
    EntitiesArray {
        uids: (2..=4)
            .map(|s| doc.uid(Slot(s)).expect("known").to_string())
            .collect(),
        layout,
        associative: true,
        expected_revision: None,
    }
}

/// `cargo test --release -p kentos-native-application --test all array_assoc::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let mut doc = drawing();
    let grid = input(
        &doc,
        ArrayLayout::Grid {
            rows: 100,
            cols: 100,
            dx: 3.0,
            dy: 4.0,
        },
    );
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        array::execute(&mut ExecutionContext::new(&mut doc), grid)
    else {
        panic!("not completed")
    };
    let make = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(output.created.len(), 10_000);
    doc.undo();

    let path = input(
        &doc,
        ArrayLayout::Path {
            path: doc.uid(Slot(1)).expect("known").to_string(),
            count: 2_000,
            spacing: None,
            align: true,
        },
    );
    let CommandResult::Completed { output, .. } =
        array::execute(&mut ExecutionContext::new(&mut doc), path)
    else {
        panic!("not completed")
    };
    let first = output.created[1].parse().expect("an id");
    let before = doc.slot_of(first).and_then(|s| doc.get(s).cloned());
    let Some(Entity::Polyline(mut road)) = doc.get(Slot(1)).cloned() else {
        panic!("the path")
    };
    for p in &mut road.pts {
        *p = Vec2 {
            x: p.x,
            y: p.y * 3.0,
        };
    }
    let t = Instant::now();
    assert!(doc.update(Slot(1), Entity::Polyline(road)));
    let follow = t.elapsed().as_secs_f64() * 1e3;
    let after = doc.slot_of(first).and_then(|s| doc.get(s).cloned());
    assert_ne!(before, after, "the item followed");
    let t = Instant::now();
    doc.undo();
    let undo = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(doc.slot_of(first).and_then(|s| doc.get(s).cloned()), before);
    println!(
        "10 000 öğelik ızgara dizisinin kurulması {make:.2} ms; 2 000 öğelik yol dizisinin yolu değişince \
         izlemesi {follow:.2} ms, geri alınması {undo:.2} ms"
    );
    assert!(
        make < 300.0 && follow < 100.0 && undo < 100.0,
        "within budget"
    );
}

/// `cargo test --release -p kentos-native-application --test all array_assoc::edit_timing -- --ignored --nocapture`:
/// Diziyi düzenle (docs/adr/0301) on a 100 × 100 grid.
#[test]
#[ignore = "a timing, run on purpose"]
fn edit_timing() {
    use kentos_domain::contracts::{ArrayEdit, ArraysEdit};
    use kentos_native_application::arrays_edit;
    use std::time::Instant;
    let mut doc = drawing();
    let grid = |dx: f64| ArrayLayout::Grid {
        rows: 100,
        cols: 100,
        dx,
        dy: 4.0,
    };
    let made = input(&doc, grid(3.0));
    let CommandResult::Completed { output, .. } =
        array::execute(&mut ExecutionContext::new(&mut doc), made)
    else {
        panic!("not completed")
    };
    let id = output.array.expect("an array").to_text();
    let edit = |e: ArrayEdit| ArraysEdit {
        array: id.clone(),
        edit: e,
        expected_revision: None,
    };
    let spacing = edit(ArrayEdit::Layout { layout: grid(5.0) });
    let t = Instant::now();
    let CommandResult::Completed { output: plan, .. } = arrays_edit::plan_in(&doc, &spacing) else {
        panic!("no plan")
    };
    let planned = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(plan.changed.len(), 9_900);
    let t = Instant::now();
    let CommandResult::Completed { .. } =
        arrays_edit::execute(&mut ExecutionContext::new(&mut doc), spacing)
    else {
        panic!("not completed")
    };
    let relaid = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    let CommandResult::Completed { .. } = arrays_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        edit(ArrayEdit::Placement {
            p: Some(Vec2 { x: 50.0, y: 50.0 }),
            scale: None,
            rotation: None,
            mirror: None,
        }),
    ) else {
        panic!("not completed")
    };
    let moved = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    let CommandResult::Completed { .. } = arrays_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        edit(ArrayEdit::Layout {
            layout: ArrayLayout::Grid {
                rows: 50,
                cols: 100,
                dx: 5.0,
                dy: 4.0,
            },
        }),
    ) else {
        panic!("not completed")
    };
    let shrunk = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    let CommandResult::Completed { .. } = arrays_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        edit(ArrayEdit::Layout { layout: grid(5.0) }),
    ) else {
        panic!("not completed")
    };
    let grown = t.elapsed().as_secs_f64() * 1e3;
    println!(
        "100 × 100 ızgarada: aralık değişikliğinin planı {planned:.2} ms, yazılması {relaid:.2} ms, ilk yerin taşınması \
         {moved:.2} ms, 50 satıra inmesi (5 000 silinen) {shrunk:.2} ms, 100 satıra çıkması (5 000 yeni) {grown:.2} ms"
    );
    assert!(
        planned < 100.0 && relaid < 300.0 && moved < 300.0 && shrunk < 300.0 && grown < 300.0,
        "within budget"
    );
}

/// `cargo test --release -p kentos-native-application --test all array_assoc::explode_timing -- --ignored --nocapture`:
/// Patlat (docs/adr/0302) on one item of a 100 × 100 associative array of a three-line symbol: 30 000 pieces.
#[test]
#[ignore = "a timing, run on purpose"]
fn explode_timing() {
    use kentos_domain::contracts::EntitiesExplode;
    use kentos_native_application::entities_explode;
    use std::time::Instant;
    let mut doc = drawing();
    let grid = input(
        &doc,
        ArrayLayout::Grid {
            rows: 100,
            cols: 100,
            dx: 3.0,
            dy: 4.0,
        },
    );
    let CommandResult::Completed { output, .. } =
        array::execute(&mut ExecutionContext::new(&mut doc), grid)
    else {
        panic!("not completed")
    };
    let one = EntitiesExplode {
        uids: vec![output.created[5_000].clone()],
        expected_revision: None,
    };
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        entities_explode::execute(&mut ExecutionContext::new(&mut doc), one)
    else {
        panic!("not completed")
    };
    let exploded = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(output.removed.len(), 10_000);
    assert_eq!(output.created.len(), 30_000);
    assert!(doc.arrays().is_empty());
    let t = Instant::now();
    assert_eq!(doc.undo().as_deref(), Some("Patlat"));
    let undone = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(doc.arrays().len(), 1);
    println!(
        "100 × 100 ilişkili dizinin bir öğesinden: bütün dizinin patlatılması (30 000 parça) {exploded:.2} ms, geri alınması {undone:.2} ms"
    );
    assert!(exploded < 1000.0 && undone < 300.0, "within budget");
}
