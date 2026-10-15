//! Eğriyi düzenle by its meaning over a large drawing (docs/adr/0303 §6): what a preview (the plan) and a write
//! take when the drawing holds 100 000 other objects, for a curve through 2 000 points and one of 10 000 control
//! points. The shared cases are `fixtures/commands/v1/cad.curves.edit.json`.

use kentos_domain::contracts::{
    CommandResult, CurveEdit, CurvesEdit, DocumentSnapshotV1, Entity, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, curves_edit};
use serde_json::json;

/// A 2 000-vertex polyline (1) and a 10 000-vertex one (2), wavy.
fn drawing() -> Document {
    let wave = |n: u32| -> Vec<serde_json::Value> {
        (0..n)
            .map(|i| json!({"x": f64::from(i) * 2.0, "y": (f64::from(i) * 0.37).sin() * 3.0}))
            .collect()
    };
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Eğriyi düzenle","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"polyline","id":1,"layerId":"a","attrs":{},"pts":wave(2_000)},
        {"kind":"polyline","id":2,"layerId":"a","attrs":{},"pts":wave(10_000)}],
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn input(doc: &Document, slot: u32, edit: CurveEdit) -> CurvesEdit {
    CurvesEdit {
        uid: doc.uid(Slot(slot)).expect("known").to_string(),
        edit,
        expected_revision: None,
    }
}

fn written(doc: &mut Document, slot: u32, edit: CurveEdit) -> f64 {
    let t = std::time::Instant::now();
    let input = input(doc, slot, edit);
    let result = curves_edit::execute(&mut ExecutionContext::new(doc), input);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    t.elapsed().as_secs_f64() * 1e3
}

#[test]
fn a_polyline_made_a_curve_and_back_is_one_undo_step_each() {
    let mut doc = drawing();
    let before = doc.get(Slot(1)).cloned();
    written(
        &mut doc,
        1,
        CurveEdit::ToSpline {
            degree: None,
            through: true,
        },
    );
    let Some(Entity::Nurbs(n)) = doc.get(Slot(1)) else {
        panic!("a NURBS curve")
    };
    assert_eq!(n.fit.as_ref().map(Vec::len), Some(2_000));
    written(&mut doc, 1, CurveEdit::ToPolyline { tolerance: 0.01 });
    assert!(matches!(doc.get(Slot(1)), Some(Entity::Polyline(_))));
    assert_eq!(doc.undo().as_deref(), Some("Eğriyi düzenle"));
    assert_eq!(doc.undo().as_deref(), Some("Eğriyi düzenle"));
    assert_eq!(doc.get(Slot(1)).cloned(), before);
}

/// `cargo test --release -p kentos-native-application --test all curves_edit::timing -- --ignored --nocapture`.
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
    let through = written(
        &mut doc,
        1,
        CurveEdit::ToSpline {
            degree: None,
            through: true,
        },
    );
    let control = written(
        &mut doc,
        2,
        CurveEdit::ToSpline {
            degree: None,
            through: false,
        },
    );
    let runs = 20;
    let plan = |doc: &Document, input: &CurvesEdit| -> f64 {
        let t = Instant::now();
        for _ in 0..runs {
            let CommandResult::Completed { .. } = curves_edit::plan_in(doc, input) else {
                panic!("not completed")
            };
        }
        t.elapsed().as_secs_f64() * 1e3 / f64::from(runs)
    };
    let to = Vec2 { x: 1000.0, y: 9.0 };
    let move_fit = plan(
        &doc,
        &input(&doc, 1, CurveEdit::MoveFit { index: 500, at: to }),
    );
    let move_control = plan(
        &doc,
        &input(
            &doc,
            2,
            CurveEdit::MoveControl {
                index: 5_000,
                at: to,
            },
        ),
    );
    let knot = plan(&doc, &input(&doc, 2, CurveEdit::InsertKnot { at: 4_321.5 }));
    let remove = plan(
        &doc,
        &input(&doc, 2, CurveEdit::RemoveControl { index: 5_000 }),
    );
    let polyline = plan(
        &doc,
        &input(&doc, 2, CurveEdit::ToPolyline { tolerance: 0.01 }),
    );
    println!(
        "Eğriye çevir: 2 000 köşeden geçen {through:.2} ms, 10 000 denetim noktalı {control:.2} ms; \
         plan: geçiş noktası taşı {move_fit:.2} ms, denetim noktası taşı {move_control:.3} ms, düğüm ekle \
         {knot:.3} ms, denetim noktası sil {remove:.3} ms, çoklu çizgiye çevir (1 cm) {polyline:.2} ms"
    );
}
