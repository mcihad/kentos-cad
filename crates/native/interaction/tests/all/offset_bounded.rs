//! A curve approximation needs an explicit choice; preview, script and click
//! use the same bounded semantic command. No preview writes the source.
use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, Entity};
use kentos_domain::Slot;
use kentos_interaction::{Format, Level};
use serde_json::{Value, json};
fn drawing() -> String {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .unwrap();
    raw["entities"] = json!([
        {"kind":"nurbs","id":1,"layerId":"cizim","attrs":{"QA":"source"},"degree":2,
         "ctrl":[{"x":E+1.,"y":N},{"x":E+1.,"y":N+1.},{"x":E,"y":N+1.}],"knots":[0.,0.,0.,1.,1.,1.],
         "weights":[1.,std::f64::consts::FRAC_1_SQRT_2,1.],"closed":false}
    ]);
    raw.to_string()
}
fn bench() -> Bench {
    let mut b = Bench::on(&drawing());
    b.draft.snap = false;
    b.start("offset");
    b
}
#[test]
fn a_curve_preview_requires_consent_then_matches_the_script_and_single_undo() {
    let mut b = bench();
    b.type_text("30cm");
    b.click(1., 0.);
    b.move_to(2., 2.);
    let before = b.doc.to_snapshot_v2();
    let preview = b.session.preview(&Format::default()).unwrap();
    assert!(
        preview
            .tag
            .unwrap()
            .lines
            .join(" ")
            .contains("Yaklaşımı açıp")
    );
    b.click(2., 2.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(b.last_level(), Some(Level::Warn));
    // A refused click keeps the picked curve available for fixing settings.
    assert!(b.run(|s, cx| s.configure("Y", "yes", cx)));
    assert!(b.run(|s, cx| s.configure("T", "0.1mm", cx)));
    b.move_to(2., 2.);
    let preview = b.session.preview(&Format::default()).unwrap();
    assert!(preview.tag.unwrap().lines.join(" ").contains("hata ≤"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    let uid = b.doc.uid(Slot(1)).unwrap().to_string();
    let input = json!({"uids":[uid],"by":{"kind":"distance","distance":0.3,"side":{"x":E+2.,"y":N+2.}},"approximation":{"tolerance":0.0001}});
    let mut scripted = bench();
    let result = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut scripted.doc),
        "cad.entities.offset",
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(result["status"], "completed");
    b.click(2., 2.);
    assert_eq!(b.last_level(), Some(Level::Success));
    let Some(Entity::Polyline(got)) = b.doc.get(Slot(2)) else {
        panic!()
    };
    let Some(Entity::Polyline(want)) = scripted.doc.get(Slot(2)) else {
        panic!()
    };
    assert_eq!(got.pts, want.pts);
    assert_eq!(b.doc.get(Slot(1)), before.entities.first());
    b.doc.undo();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
#[test]
fn invalid_tolerance_preserves_the_previous_setting_and_tolerance_prompt_cancels() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("Y", "yes", cx)));
    let old = b.memory.offset_tolerance;
    assert!(!b.run(|s, cx| s.configure("T", "0", cx)));
    assert_eq!(b.memory.offset_tolerance, old);
    assert!(b.type_text("T"));
    b.move_to(1., 0.);
    assert!(b.session.prompt().step.contains("tolerans"));
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.session.tool_id(), "offset");
    assert!(b.type_text("T"));
    assert!(b.type_text("0.01mm"));
    assert!((b.memory.offset_tolerance - 0.00001).abs() < 1e-15);
}
