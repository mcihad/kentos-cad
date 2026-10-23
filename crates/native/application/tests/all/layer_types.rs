//! A layer's type (docs/adr/0622 §5): a typed layer takes only its kind. Every write is held to it by the drawing's
//! own check, so a command, Python and the MCP server are refused alike (`layer_geometry_mismatch`, nothing written);
//! a closed curve fits a line and an area layer; an untyped layer takes anything.

use kentos_commands::DocumentSnapshotV1;
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn layer(id: &str, name: &str, geometry: Option<&str>) -> Value {
    let mut n = json!({"id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.18}, "children": []});
    if let Some(g) = geometry {
        n["geometry"] = json!(g);
    }
    n
}

fn drawing() -> Document {
    let snapshot = json!({"format": "kentos.document", "version": 1, "name": "Türler", "settings": {"srid": 5256,
        "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis"},
        "origin": {"x": 487000.0, "y": 4420000.0},
        "layers": [layer("yol", "Yol", Some("line")), layer("parsel", "Parsel", Some("area")),
                   layer("nokta", "Nokta", Some("point")), layer("serbest", "Serbest", None)],
        "activeLayer": "yol",
        "entities": [{"kind": "line", "id": 1, "layerId": "yol", "attrs": {},
                      "a": {"x": 487000.0, "y": 4420000.0}, "b": {"x": 487050.0, "y": 4420000.0}}],
        "styles": {"items": [], "categories": []}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn run(doc: &mut Document, command: &str, input: Value) -> Value {
    registry::run(
        &mut ExecutionContext::new(doc),
        command,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("a known command")
}

fn square(layer: &str) -> Value {
    json!({"layerId": layer, "pts": [{"x": 487000.0, "y": 4420010.0}, {"x": 487010.0, "y": 4420010.0},
                                     {"x": 487010.0, "y": 4420020.0}, {"x": 487000.0, "y": 4420020.0}]})
}

#[test]
fn a_typed_layer_takes_only_its_kind() {
    let mut doc = drawing();
    let revision = doc.revision();
    // An area on the line layer: refused whole, with the layer and the kind said.
    let refused = run(&mut doc, "cad.polygon.create", square("yol"));
    assert_eq!(refused["status"], "failed", "{refused}");
    assert_eq!(refused["error"]["code"], "layer_geometry_mismatch");
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("“Yol” bir çizgi katmanı; alan bu katmana yazılmaz"), "{message}");
    assert_eq!(doc.revision(), revision, "nothing written");
    // The kinds that fit: an area on the area layer, a line on the line layer, a point on the point layer, a circle on
    // both the line and the area layer, anything on the untyped one.
    for (command, input) in [
        ("cad.polygon.create", square("parsel")),
        ("cad.line.create", json!({"layerId": "yol", "a": {"x": 487000.0, "y": 4420030.0}, "b": {"x": 487020.0, "y": 4420030.0}})),
        ("cad.point.create", json!({"layerId": "nokta", "p": {"x": 487001.0, "y": 4420001.0}})),
        ("cad.circle.create", json!({"layerId": "yol", "c": {"x": 487040.0, "y": 4420040.0}, "r": 2.0})),
        ("cad.circle.create", json!({"layerId": "parsel", "c": {"x": 487040.0, "y": 4420040.0}, "r": 2.0})),
        ("cad.point.create", json!({"layerId": "serbest", "p": {"x": 487002.0, "y": 4420002.0}})),
        ("cad.polygon.create", square("serbest")),
    ] {
        let done = run(&mut doc, command, input);
        assert_eq!(done["status"], "completed", "{command}: {done}");
    }
    // A point on the area layer is refused.
    let point = run(&mut doc, "cad.point.create", json!({"layerId": "parsel", "p": {"x": 487003.0, "y": 4420003.0}}));
    assert_eq!(point["error"]["code"], "layer_geometry_mismatch", "{point}");
    // Moving the line onto the area layer is refused too, and the line stays where it was.
    let uid = doc.uid(Slot(1)).expect("the line").to_string();
    let moved = run(&mut doc, "cad.entities.set",
        json!({"uids": [uid], "operation": "layer", "layerId": "parsel"}));
    assert_eq!(moved["error"]["code"], "layer_geometry_mismatch", "{moved}");
    assert_eq!(doc.get(Slot(1)).map(|e| e.base().layer_id.clone()).as_deref(), Some("yol"));
}

#[test]
fn a_layers_type_is_set_only_when_its_objects_fit() {
    let mut doc = drawing();
    // The untyped layer holds a polygon (drawn there first): it may become an area layer, not a line layer.
    let made = run(&mut doc, "cad.polygon.create", square("serbest"));
    assert_eq!(made["status"], "completed", "{made}");
    let to_line = run(&mut doc, "cad.layers.edit", json!({"operation": "geometry", "layer": "serbest", "geometry": "line"}));
    assert_eq!(to_line["error"]["code"], "layer_geometry_mismatch", "{to_line}");
    let to_area = run(&mut doc, "cad.layers.edit", json!({"operation": "geometry", "layer": "serbest", "geometry": "area"}));
    assert_eq!(to_area["status"], "completed", "{to_area}");
    assert_eq!(
        doc.layers().get("serbest").and_then(|n| n.geometry),
        Some(kentos_commands::LayerGeometry::Area)
    );
    // Undone: untyped again.
    assert_eq!(doc.undo().as_deref(), Some("Katman türü"));
    assert_eq!(doc.layers().get("serbest").and_then(|n| n.geometry), None);
}
