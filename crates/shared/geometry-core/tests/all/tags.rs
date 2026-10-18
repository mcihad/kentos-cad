//! Tags drawn (docs/adr/0344 §1) against the independent reference's cases (fixtures/arch/v1/tags.json,
//! scripts/fixtures/tag_cases.py): the frame round the words, the words' lines and the leader.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/arch/v1/tags.json");
const TOL: f64 = 1e-9;

fn close(a: f64, b: &Value, what: &str) {
    let b = b.as_f64().unwrap();
    assert!((a - b).abs() <= TOL, "{what}: {a} ≠ {b}");
}

#[test]
fn tags_draw_as_the_reference_says() {
    let file: Value = serde_json::from_str(CASES).unwrap();
    for c in file["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let shape = Shape::from_json(&Json::parse(&c["shape"].to_string()).unwrap()).unwrap();
        let pieces = arch::pieces(&shape, c["scale"].as_f64().unwrap());
        let mut frames = Vec::new();
        let mut texts = Vec::new();
        let mut lines = Vec::new();
        for p in &pieces {
            match &p.shape {
                Shape::Circle { .. } | Shape::Polygon { .. } => frames.push(p.shape.clone()),
                Shape::Text {
                    p, text, height, ..
                } => texts.push((*p, text.clone(), *height)),
                Shape::Line { a, b } => lines.push((*a, *b)),
                other => panic!("{name}: beklenmeyen parça {other:?}"),
            }
        }
        match (frames.as_slice(), &c["frame"]) {
            ([], Value::Null) => {}
            ([Shape::Circle { c: m, r }], want) => {
                assert_eq!(want["kind"], "circle", "{name}");
                close(m.x, &want["c"][0], name);
                close(m.y, &want["c"][1], name);
                close(*r, &want["r"], name);
            }
            ([Shape::Polygon { pts, bulges, .. }], want) => {
                assert_eq!(want["kind"], "polygon", "{name}");
                let w = want["pts"].as_array().unwrap();
                assert_eq!(pts.len(), w.len(), "{name}: köşe sayısı");
                for (p, q) in pts.iter().zip(w) {
                    close(p.x, &q[0], name);
                    close(p.y, &q[1], name);
                }
                match (bulges, &want["bulges"]) {
                    (None, Value::Null) => {}
                    (Some(b), Value::Array(w)) => {
                        for (x, y) in b.iter().zip(w) {
                            close(*x, y, name);
                        }
                    }
                    _ => panic!("{name}: bükümler"),
                }
            }
            (got, want) => panic!("{name}: çerçeve {got:?}, beklenen {want}"),
        }
        let want = c["texts"].as_array().unwrap();
        assert_eq!(texts.len(), want.len(), "{name}: satır sayısı");
        for ((p, t, h), w) in texts.iter().zip(want) {
            assert_eq!(t, w["text"].as_str().unwrap(), "{name}");
            close(p.x, &w["p"][0], name);
            close(p.y, &w["p"][1], name);
            close(*h, &w["height"], name);
        }
        match (lines.as_slice(), &c["leader"]) {
            ([], Value::Null) => {}
            ([(a, b)], want) => {
                close(a.x, &want["a"][0], name);
                close(a.y, &want["a"][1], name);
                close(b.x, &want["b"][0], name);
                close(b.y, &want["b"][1], name);
            }
            (got, want) => panic!("{name}: kol {got:?}, beklenen {want}"),
        }
    }
}
