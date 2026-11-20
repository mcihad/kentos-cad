//! A run that ends but gives nothing of use is a warning, not a success (docs/adr/1084): GIS-019's M değerlerini çıkar
//! on routes with no measure.

use kentos_commands::DocumentSnapshotV1;
use kentos_domain::{Document, Slot};
use kentos_processing::parameters::default_values;
use kentos_processing::runner::{Outcome, Runner};
use kentos_processing::types::Defaults;
use kentos_processing::{Bounds, Host, Registry, Scene};
use serde_json::json;

struct Bare(Document);

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
    fn select(&mut self, _: &[Slot]) {}
}

/// Three lines and two polylines without measures, as the report's Yol parçaları.
fn roads(measured: bool) -> Document {
    let mut entities = Vec::new();
    for i in 0..3 {
        let y = 4422000.0 + 10.0 * f64::from(i);
        entities.push(json!({ "kind": "line", "id": i + 1, "layerId": "yol", "attrs": {},
                              "a": { "x": 487000.0, "y": y }, "b": { "x": 487100.0, "y": y } }));
    }
    for i in 0..2 {
        let y = 4422100.0 + 10.0 * f64::from(i);
        let mut e = json!({ "kind": "polyline", "id": i + 4, "layerId": "yol", "attrs": {}, "closed": false,
                            "pts": [{ "x": 487000.0, "y": y }, { "x": 487050.0, "y": y }, { "x": 487100.0, "y": y + 5.0 }] });
        if measured {
            e["ms"] = json!([0.0, 50.0, 100.12]);
        }
        entities.push(e);
    }
    let v = json!({
        "format": "kentos.document", "version": 1, "name": "Yollar",
        "settings": { "srid": 5255, "lengthDecimals": 3, "areaDecimals": 2, "workspace": "gis", "areaUnit": "m2",
                      "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow" },
        "origin": { "x": 487000, "y": 4422000 },
        "layers": [{ "id": "yol", "name": "Yol parçaları", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "children": [],
                     "style": { "color": "#E5484D", "lineType": "continuous", "lineWeight": 0.5 } }],
        "activeLayer": "yol",
        "entities": entities,
        "styles": { "items": [], "categories": [] },
    });
    Document::from_snapshot(DocumentSnapshotV1::from_json(&v.to_string()).unwrap()).unwrap()
}

fn extract(doc: Document) -> kentos_processing::types::RunResult {
    let mut host = Bare(doc);
    let tool = Registry::current().get("linearRef.extractMeasures").unwrap();
    let mut values = default_values(&tool, &Defaults::of(&host.0));
    values.insert("input".into(), json!({ "scope": "layer", "layerId": "yol" }));
    let outcome = Runner::new().run(&mut host, &tool, &values, false, &mut Vec::new());
    let Outcome::Ok { result, .. } = outcome else {
        panic!("{outcome:?}");
    };
    result
}

#[test]
fn routes_without_a_measure_are_a_warning_naming_the_way_to_measures() {
    let r = extract(roads(false));
    assert!(r.warning);
    let said = r.summary.unwrap_or_default();
    assert!(said.contains("hiçbirinde M değeri yok") && said.contains("Rota oluştur"), "{said}");
}

#[test]
fn measured_routes_are_a_success() {
    let r = extract(roads(true));
    assert!(!r.warning);
    assert!(r.summary.unwrap_or_default().contains("rotanın ölçüleri yazıldı"));
}
