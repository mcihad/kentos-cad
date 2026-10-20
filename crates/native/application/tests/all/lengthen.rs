//! Uzat-kısalt by its meaning over a large drawing (docs/adr/0298 §6): what
//! a preview (the plan) and a write take when the drawing holds 100 000
//! other objects, for a line, a long polyline cut back and an elliptical arc
//! found on its ellipse. The shared cases are
//! `fixtures/commands/v1/cad.entities.lengthen.json`.

use kentos_commands::{
    CommandResult, DocumentSnapshotV1, EntitiesLengthen, Entity, LengthenChange, LengthenEnd, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, entities_lengthen};
use serde_json::json;

/// A line (1), a 10 000-vertex polyline (2) and an elliptical arc (3).
fn drawing() -> Document {
    let pts: Vec<_> = (0..10_000)
        .map(|i| json!({"x": f64::from(i), "y": if i % 2 == 0 { 0.0 } else { 1.0 }}))
        .collect();
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Uzat-kısalt","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":0,"y":-10},"b":{"x":10,"y":-10},"za":1,"zb":3},
        {"kind":"polyline","id":2,"layerId":"a","attrs":{},"pts":pts},
        {"kind":"ellipse","id":3,"layerId":"a","attrs":{},"c":{"x":0,"y":-30},"major":{"x":8,"y":0},"ratio":0.3,
         "t0":0.0,"t1":2.0}],
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn input(doc: &Document, slot: u32, change: LengthenChange) -> EntitiesLengthen {
    EntitiesLengthen {
        uid: doc.uid(Slot(slot)).expect("known").to_string(),
        end: LengthenEnd::End,
        change,
        expected_revision: None,
    }
}

#[test]
fn a_lengthened_line_runs_its_elevation_on_and_one_undo_takes_it_back() {
    let mut doc = drawing();
    let before = doc.get(Slot(1)).cloned();
    let lengthen = input(
        &doc,
        1,
        LengthenChange::Dynamic {
            to: Vec2 { x: 15.0, y: -9.0 },
        },
    );
    let result = entities_lengthen::execute(&mut ExecutionContext::new(&mut doc), lengthen);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    let Some(Entity::Line(l)) = doc.get(Slot(1)) else {
        panic!("a line")
    };
    assert_eq!((l.b.x, l.b.y, l.zb), (15.0, -10.0, Some(4.0)));
    assert_eq!(doc.undo().as_deref(), Some("Uzat-kısalt"));
    assert_eq!(doc.get(Slot(1)).cloned(), before);
}

/// `cargo test --release -p kentos-native-application --test all lengthen::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let mut doc = drawing();
    let mut lines = Vec::with_capacity(100_000);
    for i in 0..100_000u32 {
        let (x, y) = (
            f64::from(i % 400) * 3.0 + 100.0,
            f64::from(i / 400) * 3.0 + 100.0,
        );
        let line = json!({"kind":"line","id":0,"layerId":"a","attrs":{},"a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}});
        lines.push(serde_json::from_value::<Entity>(line).expect("a line"));
    }
    doc.add_many(lines, "Çizgiler").expect("added");
    let runs = 50;
    let plan = |doc: &Document, input: &EntitiesLengthen| -> f64 {
        let t = Instant::now();
        for _ in 0..runs {
            let CommandResult::Completed { .. } = entities_lengthen::plan_in(doc, input) else {
                panic!("not completed")
            };
        }
        t.elapsed().as_secs_f64() * 1e3 / f64::from(runs)
    };
    let line = plan(
        &doc,
        &input(
            &doc,
            1,
            LengthenChange::Dynamic {
                to: Vec2 { x: 14.0, y: -9.0 },
            },
        ),
    );
    let cut = plan(
        &doc,
        &input(
            &doc,
            2,
            LengthenChange::Total {
                length: Some(7000.5),
                angle: None,
            },
        ),
    );
    let oval = plan(
        &doc,
        &input(
            &doc,
            3,
            LengthenChange::Delta {
                length: Some(3.0),
                angle: None,
            },
        ),
    );
    let t = Instant::now();
    for _ in 0..runs {
        let lengthen = input(
            &doc,
            1,
            LengthenChange::Delta {
                length: Some(1.0),
                angle: None,
            },
        );
        let CommandResult::Completed { .. } =
            entities_lengthen::execute(&mut ExecutionContext::new(&mut doc), lengthen)
        else {
            panic!("not completed")
        };
    }
    let write = t.elapsed().as_secs_f64() * 1e3 / f64::from(runs);
    println!(
        "100 000 çizgide önizleme: çizgi {line:.3} ms, 10 000 köşeli çoklu çizgi kısalır {cut:.3} ms, \
         elips yayı {oval:.3} ms; yazma {write:.3} ms"
    );
    assert!(
        line < 1.0 && oval < 2.0 && cut < 16.0,
        "a preview within a frame"
    );
}
