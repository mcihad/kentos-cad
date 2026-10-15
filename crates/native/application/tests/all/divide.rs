//! Böl and Ölç with blocks over a large drawing (docs/adr/0299 §5): what a
//! preview (the plan) and a write take for 10 000 aligned inserts along a
//! long polyline, and an elliptical arc's places measured on its true length.
//! The shared cases are `fixtures/commands/v1/cad.entities.divide.places.json`.

use kentos_domain::contracts::{
    CommandResult, DivideBlock, DivideEnds, Division, DocumentSnapshotV1, EntitiesDivide, Entity,
    NewLook,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, divide};
use serde_json::json;

/// A 10 000 m zigzag (1), an elliptical arc (2) and the block “Direk”.
fn drawing() -> Document {
    let pts: Vec<_> = (0..=100)
        .map(|i| json!({"x": f64::from(i) * 100.0, "y": if i % 2 == 0 { 0.0 } else { 5.0 }}))
        .collect();
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Böl","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"polyline","id":1,"layerId":"a","attrs":{},"pts":pts},
        {"kind":"ellipse","id":2,"layerId":"a","attrs":{},"c":{"x":0,"y":-50},"major":{"x":40,"y":0},"ratio":0.25,
         "t0":0.0,"t1":3.0}],
        "styles":{"items":[],"categories":[]},
        "blocks":[{"id":"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001","name":"Direk","base":{"x":0,"y":0},
         "entities":[{"kind":"line","id":1,"layerId":"0","attrs":{},"a":{"x":0,"y":0},"b":{"x":1,"y":0}}]}]});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn input(doc: &Document, slot: u32, division: Division, block: bool) -> EntitiesDivide {
    EntitiesDivide {
        uid: doc.uid(Slot(slot)).expect("known").to_string(),
        division,
        from_end: false,
        start: None,
        ends: DivideEnds::Both,
        block: block.then(|| DivideBlock {
            name: "Direk".into(),
            align: true,
            scale: 1.0,
            rotation: 0.0,
        }),
        layer_id: "a".into(),
        look: NewLook::default(),
        expected_revision: None,
    }
}

#[test]
fn an_elliptical_arcs_parts_are_equal_along_its_true_length() {
    let doc = drawing();
    let CommandResult::Completed { output, .. } =
        divide::plan_in(&doc, &input(&doc, 2, Division::Parts { count: 6 }, false))
    else {
        panic!("not completed")
    };
    let Some(Entity::Ellipse(e)) = doc.get(Slot(2)) else {
        panic!("an ellipse")
    };
    // The length along the arc between neighbouring places, by fine sampling of the curve.
    let (a, b) = (e.major.x, e.major.x * e.ratio);
    let length = |t0: f64, t1: f64| {
        let n = 200_000;
        (0..n)
            .map(|k| {
                let t = t0 + (t1 - t0) * (f64::from(k) + 0.5) / f64::from(n);
                (a * t.sin()).hypot(b * t.cos()) * (t1 - t0) / f64::from(n)
            })
            .sum::<f64>()
    };
    let param = |x: f64, y: f64| (y / b).atan2(x / a);
    let ts: Vec<f64> = output
        .entities
        .iter()
        .map(|p| match p {
            Entity::Point(p) => param(p.p.x - e.c.x, p.p.y - e.c.y),
            _ => panic!("points"),
        })
        .collect();
    assert_eq!(ts.len(), 7, "five between and both ends");
    let whole = length(e.t0, e.t1);
    for pair in ts.windows(2) {
        let piece = length(pair[0], pair[1]);
        assert!(
            (piece - whole / 6.0).abs() < 1e-7,
            "{piece} of {}",
            whole / 6.0
        );
    }
}

/// `cargo test --release -p kentos-native-application --test all divide::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let mut doc = drawing();
    let mut lines = Vec::with_capacity(100_000);
    for i in 0..100_000u32 {
        let (x, y) = (f64::from(i % 400) * 3.0, f64::from(i / 400) * 3.0 + 100.0);
        let line = json!({"kind":"line","id":0,"layerId":"a","attrs":{},"a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}});
        lines.push(serde_json::from_value::<Entity>(line).expect("a line"));
    }
    doc.add_many(lines, "Çizgiler").expect("added");
    let blocks = input(&doc, 1, Division::Parts { count: 9_999 }, true);
    let t = Instant::now();
    let CommandResult::Completed { output, .. } = divide::plan_in(&doc, &blocks) else {
        panic!("not completed")
    };
    let plan = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(output.entities.len(), 10_000);
    let oval = input(&doc, 2, Division::Parts { count: 1_000 }, true);
    let t = Instant::now();
    let CommandResult::Completed { .. } = divide::plan_in(&doc, &oval) else {
        panic!("not completed")
    };
    let ellipse = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    let CommandResult::Completed { .. } =
        divide::execute(&mut ExecutionContext::new(&mut doc), blocks)
    else {
        panic!("not completed")
    };
    let write = t.elapsed().as_secs_f64() * 1e3;
    println!(
        "100 000 çizgide: 10 000 hizalı blok planı {plan:.2} ms, elips yayında 1 000 yer {ellipse:.2} ms, \
         10 000 bloğun yazılması {write:.2} ms"
    );
    assert!(
        plan < 50.0 && ellipse < 50.0 && write < 200.0,
        "within budget"
    );
}
