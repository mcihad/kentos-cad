//! The same look on a layer bound to its table and on a copy of it (docs/adr/0635 §11): the classes made from the
//! values (Değerlerden sınıfla) and the legend's rows are the same. A layer bound to PostGIS reads the same objects as
//! one bound to a GeoPackage (`crates/native/postgis/tests/all.rs`, `a_postgis_layer_reads_what_a_file_layer_reads`),
//! so the same holds there.

use std::path::PathBuf;

use kentos_commands::CallOp;
use kentos_contracts::{Entity, LayerStyle};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use kentos_native_style::classify::{ExprScope, Present, categories_of, unique_values, values_of};
use kentos_native_style::legend::{LegendLayer, LegendSources, legend_of};
use kentos_style_core::expr::Unit;
use serde_json::{Value, json};

fn run(doc: &mut Document, command: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        command,
        CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    answer["output"].clone()
}

struct Src<'a>(&'a Document);

impl LegendSources for Src<'_> {
    fn entities(&self, layer: &str) -> Vec<&Entity> {
        self.0.by_layer(layer).collect()
    }
    fn symbol(&self, _: &str) -> Option<Value> {
        None
    }
    fn item_name(&self, _: &str) -> Option<String> {
        None
    }
}

#[test]
fn a_bound_layer_and_its_copy_make_the_same_classes_and_legend() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/geopackage/v1");
    let tmp = std::env::temp_dir().join(format!("kentos-bound-legend-{}.gpkg", std::process::id()));
    std::fs::copy(dir.join("qgis.gpkg"), &tmp).expect("copied");
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Lejant",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                     "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let new = || {
        let snapshot =
            kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
        Document::from_snapshot(snapshot).expect("setup")
    };
    // The same records cannot be in one drawing twice: the bound layer and the copy are in two.
    let (mut db, mut dc) = (new(), new());
    let bound = run(
        &mut db,
        "cad.sources.edit",
        json!({ "edit": { "kind": "link", "path": tmp.display().to_string(), "table": "parsel" } }),
    )["link"]["layer"]
        .as_str()
        .expect("layer")
        .to_owned();
    let copied = run(
        &mut dc,
        "file.import.gpkg",
        json!({ "path": tmp.display().to_string(), "tables": ["parsel"] }),
    )["layers"][0]["id"]
        .as_str()
        .expect("layer")
        .to_owned();
    let name = |id: &str| id.to_owned();
    let measures = |_: &[&Entity]| Vec::new();
    let scope = ExprScope {
        layer_name: &name,
        measures: &measures,
        unit: Unit::METRE,
    };
    let renderer_of = |d: &Document, layer: &str| -> Value {
        let list: Vec<&Entity> = d.by_layer(layer).collect();
        let found = unique_values(&values_of(&list, "[ada]", &scope).values);
        let present = Present {
            fill: list.len(),
            line: 0,
            marker: 0,
        };
        json!({ "type": "categorized", "expr": "[ada]",
            "categories": serde_json::to_value(categories_of(&found, &present, &[])).expect("json") })
    };
    let (a, b) = (renderer_of(&db, &bound), renderer_of(&dc, &copied));
    assert_eq!(a, b, "the same classes");
    assert!(
        a["categories"].as_array().is_some_and(|c| c.len() >= 2),
        "{a}"
    );
    let mut style: LayerStyle = db.layers().get(&bound).expect("layer").style.clone();
    style.renderer = Some(a);
    let rows = |d: &Document, layer: &str| {
        let groups = legend_of(
            &[LegendLayer {
                id: layer,
                name: "parsel",
                style: &style,
            }],
            &Src(d),
        );
        serde_json::to_value(&groups[0].entries).expect("json")
    };
    assert_eq!(rows(&db, &bound), rows(&dc, &copied), "the same legend");
    let _ = std::fs::remove_file(&tmp);
}
