//! Constraint clicks use exact NURBS endpoints and never mistake a control polygon for the curve.
use crate::common::{Bench, E, N};
use kentos_commands::Entity;
use kentos_domain::Slot;
use serde_json::{Value, json};
fn drawing(closed: bool) -> Bench {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ))
    .unwrap();
    raw["settings"]["workspace"] = json!("cad");
    raw["entities"] = json!([{"kind":"nurbs","id":1,"layerId":"cizim","attrs":{},"degree":3,"ctrl":[{"x":E,"y":N},{"x":E+2.,"y":N+3.},{"x":E+4.,"y":N+3.},{"x":E+6.,"y":N}],"knots":[0,0,0,0,1,1,1,1],"weights":[1,2,3,1],"closed":closed}]);
    Bench::on(&raw.to_string())
}
#[test]
fn exact_nurbs_endpoint_clicks_share_the_constraint_command_and_single_confirmation() {
    let mut b = drawing(false);
    b.start("constraint");
    b.type_text("SB");
    b.click(0., 0.);
    assert_eq!(b.doc.constraints().constraints.len(), 1, "{:?}", b.log);
    let before = b.doc.to_snapshot_v2();
    b.type_text("U");
    b.click(0., 0.);
    b.click(6., 0.);
    assert!(
        b.session
            .prompt()
            .notes
            .iter()
            .any(|s| s.contains("iç noktalar korunur"))
    );
    assert!(b.type_text("8"));
    assert_eq!(b.doc.constraints().constraints.len(), 2, "{:?}", b.log);
    let Some(Entity::Nurbs(n)) = b.doc.get(Slot(1)) else {
        panic!()
    };
    assert!((n.ctrl[3].x - E - 8.).abs() < 1e-8);
    assert_eq!(n.ctrl[1].x, E + 2.);
    assert_eq!(n.ctrl[2].x, E + 4.);
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
#[test]
fn unsupported_nurbs_endpoint_is_explained_without_creating_a_bond() {
    let mut b = drawing(true);
    let before = b.doc.to_snapshot_v2();
    b.start("constraint");
    b.type_text("SB");
    b.click(0., 0.);
    assert!(b.last_text().unwrap().contains("NURBS"), "{:?}", b.log);
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn backing_out_of_a_nurbs_point_removes_its_endpoint_scope_hint_without_writing() {
    let mut b = drawing(false);
    let before = b.doc.to_snapshot_v2();
    b.start("constraint");
    b.type_text("U");
    b.click(0., 0.);
    assert!(!b.session.prompt().notes.is_empty());
    assert!(b.undo_step());
    assert!(b.session.prompt().notes.is_empty());
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
