//! An attribute's options as the drawing shows them and Patlat writes them (docs/adr/0317 §1): an invisible one is
//! not drawn, a constant one shows the definition's value whatever the insert holds, a movable one stands where the
//! insert put it (a locked one does not move), a single-line one shows its breaks as spaces and a multiline one its
//! lines in its box.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::block::{Blocks, shows_nothing};
use kentos_geometry_core::entity::{Entity, Shape};
use kentos_geometry_core::ops::curve_cuts::Cut;
use kentos_geometry_core::vec2::Vec2;
use serde_json::json;

fn parse(v: &serde_json::Value) -> Json {
    Json::parse(&v.to_string()).expect("JSON")
}

fn attribute(tag: &str, value: &str, y: f64, options: serde_json::Value) -> serde_json::Value {
    let mut a = json!({ "tag": tag, "value": value, "p": { "x": 0.0, "y": y }, "height": 0.2, "rotation": 0.0 });
    for (k, v) in options.as_object().expect("options") {
        a[k] = v.clone();
    }
    a
}

fn blocks() -> Blocks {
    Blocks::from_json(&parse(&json!([{
        "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5301", "name": "Kapı", "base": { "x": 0.0, "y": 0.0 },
        "entities": [{ "kind": "line", "id": 1, "layerId": "", "attrs": {},
            "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 0.0, "y": 1.0 } }],
        "attributes": [
            attribute("NO", "", -1.0, json!({ "movable": true })),
            attribute("KILITLI", "", -2.0, json!({})),
            attribute("TUR", "İç\nkapı", -3.0, json!({})),
            attribute("NOT", "Ahşap\nsöve", -4.0, json!({ "multiline": true, "boxWidth": 1.5 })),
            attribute("GIZLI", "x", -5.0, json!({ "invisible": true })),
            attribute("SABIT", "KentOS", -6.0, json!({ "constant": true })),
        ]
    }])))
    .expect("the definitions")
}

fn insert() -> Entity {
    Entity::from_json(&parse(&json!({
        "kind": "insert", "id": 9, "layerId": "kapilar", "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5301",
        "p": { "x": 10.0, "y": 20.0 }, "scale": 1.0, "rotation": 0.0,
        "attrs": { "NO": "K-01", "KILITLI": "L", "SABIT": "başka" },
        "attributePlaces": { "NO": { "x": 0.5, "y": 0.5 }, "KILITLI": { "x": 3.0, "y": 3.0 } }
    })))
    .expect("an insert")
}

/// The texts shown: their words, points and box widths, in the definition's order.
fn texts(shapes: impl Iterator<Item = Shape>) -> Vec<(String, Vec2, Option<f64>)> {
    shapes
        .filter(|s| !shows_nothing(s))
        .filter_map(|s| match s {
            Shape::Text {
                p, text, box_width, ..
            } => Some((text, p, box_width)),
            _ => None,
        })
        .collect()
}

fn want() -> Vec<(String, Vec2, Option<f64>)> {
    vec![
        ("K-01".into(), Vec2::new(10.5, 20.5), None),
        ("L".into(), Vec2::new(10.0, 18.0), None),
        ("İç kapı".into(), Vec2::new(10.0, 17.0), None),
        ("Ahşap\nsöve".into(), Vec2::new(10.0, 16.0), Some(1.5)),
        ("KentOS".into(), Vec2::new(10.0, 14.0), None),
    ]
}

#[test]
fn an_insert_shows_its_attributes_by_their_options() {
    let pieces = blocks().expand(&insert().shape);
    assert_eq!(texts(pieces.into_iter().map(|p| p.shape)), want());
}

#[test]
fn patlat_writes_the_attributes_that_show_where_they_stand() {
    let Cut::Pieces(pieces) = blocks().explode(&insert()) else {
        panic!("patlatılmalıydı")
    };
    assert_eq!(texts(pieces.into_iter().map(|e| e.shape)), want());
}
