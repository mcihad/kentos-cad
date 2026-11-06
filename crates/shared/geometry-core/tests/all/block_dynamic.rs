//! A dynamic block's variants (docs/adr/0318 §2): the door of `fixtures/kcad/v2/dynamic-blocks.json` drawn at its
//! defaults, wider (its leaf and swing scaled about the hinge, its far jamb moved), flipped about its middle and
//! without its swing; nested in another block, its variant follows the nested insert's values.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::block::{Blocks, shows_nothing};
use kentos_geometry_core::entity::{Entity, Shape};
use kentos_geometry_core::jsmath::js_round;
use kentos_geometry_core::vec2::Vec2;
use serde_json::{Value, json};

const SAMPLE: &str = include_str!("../../../../../fixtures/kcad/v2/dynamic-blocks.json");

fn parse(v: &Value) -> Json {
    Json::parse(&v.to_string()).expect("JSON")
}

fn blocks(extra: Value) -> Blocks {
    let doc: Value = serde_json::from_str(SAMPLE).expect("the sample");
    let mut list = doc["blocks"].as_array().expect("blocks").clone();
    if let Value::Array(more) = extra {
        list.extend(more);
    }
    Blocks::from_json(&parse(&Value::Array(list))).expect("the definitions")
}

fn insert(params: Value) -> Shape {
    let mut e = json!({
        "kind": "insert", "id": 1, "layerId": "kapilar", "attrs": {},
        "block": "0192f5a1-2900-7000-8000-0000000000c1", "p": { "x": 10.0, "y": 0.0 }, "scale": 1.0, "rotation": 0.0
    });
    if !params.is_null() {
        e["parameters"] = params;
    }
    Entity::from_json(&parse(&e)).expect("an insert").shape
}

/// The drawn shapes: lines as their ends, arcs as centre, radius and ends' angles.
fn drawn(b: &Blocks, s: &Shape) -> Vec<String> {
    let r = |x: f64| js_round(x * 1e6) / 1e6 + 0.0;
    let pt = |p: Vec2| format!("({}, {})", r(p.x), r(p.y));
    b.expand(s)
        .into_iter()
        .map(|p| p.shape)
        .filter(|s| !shows_nothing(s))
        .map(|s| match s {
            Shape::Line { a, b } => format!("çizgi {} {}", pt(a), pt(b)),
            Shape::Arc { c, r: rad, a0, a1 } => {
                format!("yay {} {} {} {}", pt(c), r(rad), r(a0), r(a1))
            }
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn a_door_draws_at_its_values() {
    let b = blocks(Value::Null);
    let quarter = std::f64::consts::FRAC_PI_2;
    let q = js_round(quarter * 1e6) / 1e6;
    assert_eq!(
        drawn(&b, &insert(Value::Null)),
        [
            "çizgi (10, 0) (10, 0.9)".to_owned(),
            format!("yay (10, 0) 0.9 0 {q}"),
            "çizgi (10, -0.1) (10, 0)".into(),
            "çizgi (10.9, -0.1) (10.9, 0)".into(),
        ]
    );
    // Wider: the leaf and the swing scale about the hinge, the far jamb moves along the width's line.
    assert_eq!(
        drawn(&b, &insert(json!({ "Genişlik": 1.2 }))),
        [
            "çizgi (10, 0) (10, 1.2)".to_owned(),
            format!("yay (10, 0) 1.2 0 {q}"),
            "çizgi (10, -0.1) (10, 0)".into(),
            "çizgi (11.2, -0.1) (11.2, 0)".into(),
        ]
    );
    // Without its swing.
    assert_eq!(drawn(&b, &insert(json!({ "Görünüş": "Sade" }))).len(), 3);
    // Flipped about x = 0.45: the hinge at 0.9, the jambs swapped.
    let flipped = drawn(&b, &insert(json!({ "Yön": true })));
    assert_eq!(flipped[0], "çizgi (10.9, 0) (10.9, 0.9)");
    assert_eq!(flipped[2], "çizgi (10.9, -0.1) (10.9, 0)");
    assert_eq!(flipped[3], "çizgi (10, -0.1) (10, 0)");
    // The same values twice draw the same variant (from the cache).
    assert_eq!(
        drawn(&b, &insert(json!({ "Genişlik": 1.2 }))),
        drawn(&b, &insert(json!({ "Genişlik": 1.2 })))
    );
}

#[test]
fn a_nested_door_draws_its_own_values() {
    let wall = json!([{
        "id": "0192f5a1-2900-7000-8000-0000000000c2", "name": "Duvar ve kapı", "base": { "x": 0.0, "y": 0.0 },
        "entities": [{
            "kind": "insert", "id": 1, "layerId": "", "attrs": {}, "block": "0192f5a1-2900-7000-8000-0000000000c1",
            "p": { "x": 1.0, "y": 0.0 }, "scale": 1.0, "rotation": 0.0, "parameters": { "Genişlik": 1.2 }
        }]
    }]);
    let b = blocks(wall);
    let outer = Entity::from_json(&parse(&json!({
        "kind": "insert", "id": 2, "layerId": "kapilar", "attrs": {},
        "block": "0192f5a1-2900-7000-8000-0000000000c2", "p": { "x": 0.0, "y": 0.0 }, "scale": 1.0, "rotation": 0.0
    })))
    .expect("an insert")
    .shape;
    let lines = drawn(&b, &outer);
    assert_eq!(lines[0], "çizgi (1, 0) (1, 1.2)");
    assert_eq!(lines[3], "çizgi (2.2, -0.1) (2.2, 0)");
}

#[test]
fn an_array_adds_a_copy_for_each_spacing_past_the_default() {
    let shelf = json!([{
        "id": "0192f5a1-2900-7000-8000-0000000000c3", "name": "Dolap", "base": { "x": 0.0, "y": 0.0 },
        "entities": [{ "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": { "x": 0.6, "y": 0.0 }, "b": { "x": 0.6, "y": 0.05 } }],
        "dynamic": {
            "parameters": [{ "kind": "linear", "name": "Genişlik", "from": { "x": 0.0, "y": 0.0 }, "to": { "x": 1.2, "y": 0.0 } }],
            "actions": [{ "kind": "array", "parameter": "Genişlik", "pieces": [1], "spacing": 0.6 }]
        }
    }]);
    let b = blocks(shelf);
    let at = |w: f64| {
        let e = Entity::from_json(&parse(&json!({
            "kind": "insert", "id": 1, "layerId": "", "attrs": {}, "block": "0192f5a1-2900-7000-8000-0000000000c3",
            "p": { "x": 0.0, "y": 0.0 }, "scale": 1.0, "rotation": 0.0, "parameters": { "Genişlik": w }
        })))
        .expect("an insert");
        drawn(&b, &e.shape)
    };
    assert_eq!(at(1.2).len(), 1);
    assert_eq!(
        at(2.4),
        [
            "çizgi (0.6, 0) (0.6, 0.05)",
            "çizgi (1.2, 0) (1.2, 0.05)",
            "çizgi (1.8, 0) (1.8, 0.05)"
        ]
    );
    assert_eq!(at(0.6).len(), 1);
}

/// A piece the visibility state hides is hidden with the array action's copies of it (docs/adr/1131 §2): the copies
/// take the id of the piece they were made of.
#[test]
fn a_hidden_piece_hides_its_array_copies_too() {
    let shelf = json!([{
        "id": "0192f5a1-2900-7000-8000-0000000000c4", "name": "Raf", "base": { "x": 0.0, "y": 0.0 },
        "entities": [
            { "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 1.2, "y": 0.0 } },
            { "kind": "line", "id": 2, "layerId": "", "attrs": {}, "a": { "x": 0.6, "y": 0.0 }, "b": { "x": 0.6, "y": 0.05 } }
        ],
        "dynamic": {
            "parameters": [
                { "kind": "linear", "name": "Genişlik", "from": { "x": 0.0, "y": 0.0 }, "to": { "x": 1.2, "y": 0.0 } },
                { "kind": "visibility", "name": "Görünüm", "states": [{ "name": "Dolu" }, { "name": "Boş", "hidden": [2] }] }
            ],
            "actions": [{ "kind": "array", "parameter": "Genişlik", "pieces": [2], "spacing": 0.6 }]
        }
    }]);
    let b = blocks(shelf);
    let at = |params: Value| {
        let e = Entity::from_json(&parse(&json!({
            "kind": "insert", "id": 1, "layerId": "", "attrs": {}, "block": "0192f5a1-2900-7000-8000-0000000000c4",
            "p": { "x": 0.0, "y": 0.0 }, "scale": 1.0, "rotation": 0.0, "parameters": params
        })))
        .expect("an insert");
        drawn(&b, &e.shape)
    };
    assert_eq!(at(json!({ "Genişlik": 2.4 })).len(), 4, "the frame and three dividers");
    assert_eq!(at(json!({ "Genişlik": 2.4, "Görünüm": "Boş" })), ["çizgi (0, 0) (1.2, 0)"], "no divider, no copy");
}

#[test]
fn a_default_visibility_state_hides_its_pieces_directly_and_through_a_plain_parent() {
    let mut doc: Value = serde_json::from_str(SAMPLE).unwrap();
    let list = doc["blocks"].as_array_mut().unwrap();
    let dynamic = &mut list[0]["dynamic"];
    let params = dynamic["parameters"].as_array_mut().unwrap();
    let visibility = params
        .iter_mut()
        .find(|p| p["kind"] == "visibility")
        .unwrap();
    visibility["states"].as_array_mut().unwrap().reverse();
    let parent = "0192f5a1-2900-7000-8000-0000000000e1";
    list.push(json!({"id":parent,"name":"Giriş","base":{"x":0.,"y":0.},"entities":[
        {"kind":"insert","id":1,"layerId":"","attrs":{},"block":"0192f5a1-2900-7000-8000-0000000000c1","p":{"x":0.,"y":0.},"scale":1.,"rotation":0.}
    ]}));
    let blocks = Blocks::from_json(&parse(&Value::Array(list.clone()))).unwrap();
    let default = drawn(&blocks, &insert(Value::Null));
    assert_eq!(default.len(), 3);
    assert!(!default.iter().any(|p| p.starts_with("yay")));
    let open = drawn(&blocks, &insert(json!({"Görünüş":"Açılım"})));
    assert_eq!(open.len(), 4);
    assert!(open.iter().any(|p| p.starts_with("yay")));
    let nested=Entity::from_json(&parse(&json!({"kind":"insert","id":1,"layerId":"kapilar","attrs":{},"block":parent,"p":{"x":10.,"y":0.},"scale":1.,"rotation":0.}))).unwrap();
    assert_eq!(drawn(&blocks, &nested.shape), default);
    // Returning to the default after another state cannot reuse the raw four-piece definition.
    assert_eq!(drawn(&blocks, &insert(json!({"Görünüş":"Sade"}))), default);
}

#[test]
fn shared_type_values_render_without_instance_overrides_and_have_no_instance_grips() {
    use kentos_geometry_core::block_dynamic::Dynamic;
    let mut doc: Value = serde_json::from_str(SAMPLE).unwrap();
    let d = &mut doc["blocks"][0]["dynamic"];
    d["typeParameters"] = json!(["Genişlik", "Yön", "Görünüş"]);
    d["typeValues"] = json!({"Genişlik":1.2,"Yön":true,"Görünüş":"Sade"});
    let dynamic = Dynamic::from_json(&parse(d));
    assert!(!dynamic.is_default(None));
    assert!(dynamic.grips(None, 0.2).is_empty());
    assert!(
        dynamic
            .value_at("Genişlik", Vec2::new(1., 0.), None)
            .is_none()
    );
    assert!(dynamic.value_at("Yön", Vec2::new(1., 0.), None).is_none());
    let b = Blocks::from_json(&parse(&doc["blocks"])).unwrap();
    assert_eq!(
        drawn(&b, &insert(Value::Null)),
        [
            "çizgi (10.9, 0) (10.9, 1.2)",
            "çizgi (10.9, -0.1) (10.9, 0)",
            "çizgi (9.7, -0.1) (9.7, 0)"
        ]
    );
    // Common values dominate even malformed overrides at the rendering boundary;
    // the document/command validator refuses storing those overrides.
    assert_eq!(
        drawn(
            &b,
            &insert(json!({"Genişlik":0.7,"Yön":false,"Görünüş":"Açılım"}))
        ),
        drawn(&b, &insert(Value::Null))
    );
}
