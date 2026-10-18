//! View marks drawn (docs/adr/0345 §1) against the independent reference's cases (fixtures/arch/v1/viewmarks.json,
//! scripts/fixtures/viewmark_cases.py): the cut line and its heavy ends, the bubbles, their words and look arrows, a
//! detail's dashed circle and leader, piece by piece in order.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/arch/v1/viewmarks.json");
const TOL: f64 = 1e-9;

fn close(a: f64, b: &Value, what: &str) {
    let b = b.as_f64().unwrap();
    assert!((a - b).abs() <= TOL, "{what}: {a} ≠ {b}");
}

#[test]
fn viewmarks_draw_as_the_reference_says() {
    let file: Value = serde_json::from_str(CASES).unwrap();
    for c in file["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let shape = Shape::from_json(&Json::parse(&c["shape"].to_string()).unwrap()).unwrap();
        let pieces = arch::pieces(&shape, c["scale"].as_f64().unwrap());
        let want = c["pieces"].as_array().unwrap();
        assert_eq!(pieces.len(), want.len(), "{name}: parça sayısı");
        for (k, (p, w)) in pieces.iter().zip(want).enumerate() {
            let at = format!("{name} › {k}");
            match (&p.shape, w["kind"].as_str().unwrap()) {
                (Shape::Line { a, b }, "line") => {
                    close(a.x, &w["a"][0], &at);
                    close(a.y, &w["a"][1], &at);
                    close(b.x, &w["b"][0], &at);
                    close(b.y, &w["b"][1], &at);
                }
                (Shape::Circle { c: m, r }, "circle") => {
                    close(m.x, &w["c"][0], &at);
                    close(m.y, &w["c"][1], &at);
                    close(*r, &w["r"], &at);
                    let dashed = p.stroke.line_type.as_deref() == Some("dashed");
                    assert_eq!(dashed, w["dashed"] == true, "{at}: kesikli");
                }
                (Shape::Text { p: q, text, height, .. }, "text") => {
                    assert_eq!(text, w["text"].as_str().unwrap(), "{at}");
                    close(q.x, &w["p"][0], &at);
                    close(q.y, &w["p"][1], &at);
                    close(*height, &w["height"], &at);
                }
                (Shape::Polygon { pts, .. }, "polygon") => {
                    let ws = w["pts"].as_array().unwrap();
                    assert_eq!(pts.len(), ws.len(), "{at}: köşe sayısı");
                    for (q, v) in pts.iter().zip(ws) {
                        close(q.x, &v[0], &at);
                        close(q.y, &v[1], &at);
                    }
                }
                (got, kind) => panic!("{at}: {kind} beklenirken {got:?}"),
            }
        }
    }
}
