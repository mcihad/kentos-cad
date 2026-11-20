//! Km yaz by measure on a route continued past its last measured corner (UR2-043): the stretch without M is said.

use crate::common::Bench;
use kentos_interaction::Level;

/// The measured route (M 1005, 1050, 1062 at its three corners) with two corners added past its end and no M, as
/// Sürdür leaves them: 7.619 m without measures.
fn continued() -> Bench {
    let mut d: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/interaction/v1/stationing-measure.kcad")).expect("reads");
    let route = d["entities"].as_array_mut().expect("entities").iter_mut().find(|e| e["kind"] == "polyline").expect("the route");
    route["pts"].as_array_mut().expect("pts").extend([
        serde_json::json!({"x": 487010.0, "y": 4420014.0}),
        serde_json::json!({"x": 487013.619, "y": 4420014.0}),
    ]);
    route["ms"].as_array_mut().expect("ms").extend([serde_json::Value::Null, serde_json::Value::Null]);
    let mut b = Bench::on(&d.to_string());
    b.draft.snap = false;
    b
}

#[test]
fn the_unmeasured_end_of_a_continued_route_is_said() {
    let mut b = continued();
    b.start("stationLabels");
    b.click(-20.0, -10.0);
    let before = b.log.len();
    b.confirm();
    let said = b.said(before);
    assert!(said.iter().any(|(l, w)| *l == Level::Success && w.contains("istasyon yazıldı")), "{said:?}");
    let warn = said.iter().find(|(l, _)| *l == Level::Warn).map(|(_, w)| (*w).to_owned()).unwrap_or_default();
    assert!(warn.contains("güzergâhın sonundaki 7.619 m ölçüsüz"), "{warn}");
}
