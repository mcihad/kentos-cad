//! A run's new layer in a GIS project (docs/adr/1070): Tampon's output of an area layer is an area layer that takes
//! no point, and it keeps the source layer's fields (type, shown name, range) with Uzaklık a number.

use kentos_commands::fields::{LayerField, LayerFieldKind, check_value};
use kentos_commands::{DocumentSnapshotV1, LayerGeometry};
use kentos_domain::{Document, Slot};
use kentos_processing::runner::{Outcome, Runner};
use kentos_processing::{Bounds, Host, Registry, Scene};
use serde_json::{Value, json};

struct Bare(Document, Vec<Slot>);

impl Scene for Bare {
    fn doc(&self) -> &Document {
        &self.0
    }
    fn selected(&self) -> Vec<Slot> {
        Vec::new()
    }
    fn visible_bounds(&self) -> Option<Bounds> {
        None
    }
}

impl Host for Bare {
    fn doc_mut(&mut self) -> &mut Document {
        &mut self.0
    }
    fn select(&mut self, ids: &[Slot]) {
        self.1 = ids.to_vec();
    }
}

/// The parcels fixture as a GIS project: Parsel an area layer, its Parsel number a whole number 0–20 shown as
/// “Parsel no”, unique.
fn drawing(workspace: &str) -> Document {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/processing/v1/parcels.kcad");
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    v["settings"]["workspace"] = json!(workspace);
    for layer in v["layers"].as_array_mut().unwrap() {
        if layer["id"] == "parsel" {
            layer["geometry"] = json!("area");
            layer["fields"] = json!([
                { "name": "Ada", "kind": "text" },
                { "name": "Parsel", "alias": "Parsel no", "kind": "integer", "min": "0", "max": "20", "unique": true },
            ]);
        }
    }
    Document::from_snapshot(DocumentSnapshotV1::from_json(&v.to_string()).unwrap()).unwrap()
}

fn buffered(workspace: &str, rings: u32) -> Document {
    let mut host = Bare(drawing(workspace), Vec::new());
    let tool = Registry::current().get("geometry.buffer").unwrap();
    let values = json!({
        "input": { "scope": "layer", "layerId": "parsel" },
        "distance": 10,
        "rings": rings,
        "dissolve": false,
        "side": "both",
        "layer": { "newName": "Parsel tamponu" },
    });
    let outcome = Runner::new().run(
        &mut host,
        &tool,
        values.as_object().unwrap(),
        false,
        &mut Vec::new(),
    );
    assert!(matches!(outcome, Outcome::Ok { .. }), "ran: {outcome:?}");
    host.0
}

fn output(doc: &Document) -> kentos_commands::LayerNode {
    doc.layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == "Parsel tamponu")
        .cloned()
        .expect("the output layer")
}

fn field<'a>(fields: &'a [LayerField], name: &str) -> &'a LayerField {
    fields
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} in {fields:?}"))
}

#[test]
fn a_gis_buffers_new_layer_is_an_area_layer_that_refuses_a_point() {
    let mut doc = buffered("gis", 1);
    let layer = output(&doc);
    assert_eq!(layer.geometry, Some(LayerGeometry::Area));
    // The drawing refuses a point on it (its integrity, docs/adr/0622 §5), so the point tools are off there.
    assert!(!LayerGeometry::Area.admits("point"));
    // Saved and opened again it stays an area layer.
    let again = Document::from_snapshot(doc.to_snapshot()).unwrap();
    assert_eq!(output(&again).geometry, Some(LayerGeometry::Area));
    // Undo takes the layer away with its objects.
    doc.undo();
    assert!(
        doc.layers()
            .leaves()
            .iter()
            .all(|l| l.name != "Parsel tamponu")
    );
}

#[test]
fn a_gis_buffers_new_layer_keeps_the_source_fields_and_their_range() {
    let doc = buffered("gis", 1);
    let fields = output(&doc).fields;
    let parsel = field(&fields, "Parsel");
    assert_eq!(parsel.alias.as_deref(), Some("Parsel no"));
    assert_eq!(parsel.kind, LayerFieldKind::Integer);
    assert!(
        parsel.unique,
        "one buffer per parcel: the numbers do not repeat"
    );
    for ok in ["0", "20"] {
        assert!(check_value(parsel, ok).is_ok(), "{ok}");
    }
    for bad in ["21", "on"] {
        assert!(check_value(parsel, bad).is_err(), "{bad}");
    }
    assert_eq!(field(&fields, "Ada").kind, LayerFieldKind::Text);
    assert_eq!(field(&fields, "Uzaklık").kind, LayerFieldKind::Decimal);
    assert_eq!(field(&fields, "Uzaklık").unit.as_deref(), Some("m"));
    // A value no source layer defines stays a plain value; one ring writes no Halka.
    assert!(
        fields
            .iter()
            .all(|f| f.name != "Nitelik" && f.name != "Halka")
    );
    let again = Document::from_snapshot(doc.to_snapshot()).unwrap();
    assert_eq!(
        output(&again).fields,
        fields,
        "the fields stay after opening again"
    );
}

#[test]
fn rings_repeat_the_source_number_so_it_is_not_unique_there() {
    let doc = buffered("gis", 2);
    let fields = output(&doc).fields;
    assert!(!field(&fields, "Parsel").unique);
    assert_eq!(field(&fields, "Halka").kind, LayerFieldKind::Integer);
}

#[test]
fn a_cad_projects_new_layer_stays_untyped() {
    let doc = buffered("hybrid", 1);
    let layer = output(&doc);
    assert_eq!(layer.geometry, None);
    assert!(layer.fields.is_empty());
}

#[test]
fn a_buffer_into_an_existing_point_layer_is_refused_before_writing() {
    let mut doc = drawing("gis");
    let mut snap: Value = serde_json::to_value(doc.to_snapshot()).unwrap();
    for layer in snap["layers"].as_array_mut().unwrap() {
        if layer["id"] == "cizim" {
            layer["geometry"] = json!("point");
        }
    }
    doc = Document::from_snapshot(serde_json::from_value(snap).unwrap()).unwrap();
    let before = doc.entities().count();
    let mut host = Bare(doc, Vec::new());
    let tool = Registry::current().get("geometry.buffer").unwrap();
    let values = json!({
        "input": { "scope": "layer", "layerId": "parsel" },
        "distance": 10,
        "side": "both",
        "rings": 1,
        "dissolve": false,
        "layer": { "layerId": "cizim" },
    });
    let outcome = Runner::new().run(
        &mut host,
        &tool,
        values.as_object().unwrap(),
        false,
        &mut Vec::new(),
    );
    let Outcome::Stopped { message, .. } = outcome else {
        panic!("wrote onto a point layer: {outcome:?}");
    };
    assert!(
        message.contains("nokta katmanı") && message.contains("hiçbir şey yazılmadı"),
        "{message}"
    );
    assert_eq!(host.0.entities().count(), before);
}
