//! docs/adr/0776 §2: Metraj's sums are +0 when nothing is summed. Rust's f64 `Sum` starts from -0, so a drawing
//! whose walls had no height said `"wallNetArea": -0.0` (and each wall `"openingsArea": -0.0`) in the API's JSON, as
//! the 2+1 acceptance flow found.
use super::wall_contours::{completed, drawing};
use serde_json::json;

#[test]
fn metrajs_empty_sums_are_plus_zero() {
    let walls = drawing(
        json!([{ "kind": "wall", "id": 1, "layerId": "cizim", "attrs": {}, "width": 0.25, "justify": "left",
                 "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 4.0, "y": 0.0 }] }]),
        "m",
    );
    let empty = drawing(json!([]), "m");
    for doc in [walls, empty] {
        let q = completed(kentos_native_application::spaces_quantities::plan_in(&doc, &Default::default()));
        let totals = serde_json::to_value(&q.totals).expect("json");
        for (name, value) in totals.as_object().expect("totals") {
            let v = value.as_f64().expect("a number");
            assert!(v.is_sign_positive(), "{name}: {v}");
        }
        for w in &q.walls {
            assert!(w.openings_area.is_sign_positive(), "a wall's openings: {}", w.openings_area);
        }
    }
}
