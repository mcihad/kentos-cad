//! GIS-014 (docs/adr/1080): every input that cannot run is said in one go, not the first alone.

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

/// A park's buffer (active) and three trees.
fn park() -> Document {
    let layer = |id: &str, name: &str, g: &str| {
        json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
                "style": { "color": "#3E63DD", "lineType": "continuous", "lineWeight": 0.35 }, "geometry": g,
                "children": [] })
    };
    let v = json!({
        "format": "kentos.document", "version": 1, "name": "Park",
        "settings": { "srid": 5255, "lengthDecimals": 3, "areaDecimals": 2, "workspace": "gis", "areaUnit": "m2",
                      "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow" },
        "origin": { "x": 487000, "y": 4422000 },
        "layers": [layer("tampon", "Park koruma 10 m", "area"), layer("agac", "Ağaçlar", "point"),
                   layer("bos", "Boş", "area")],
        "activeLayer": "tampon",
        "entities": [
            { "kind": "polygon", "id": 1, "layerId": "tampon", "attrs": {},
              "pts": [{ "x": 487000, "y": 4422000 }, { "x": 487040, "y": 4422000 }, { "x": 487040, "y": 4422030 }] },
            { "kind": "point", "id": 2, "layerId": "agac", "attrs": {}, "p": { "x": 487010, "y": 4422010 } },
            { "kind": "point", "id": 3, "layerId": "agac", "attrs": {}, "p": { "x": 487020, "y": 4422010 } },
            { "kind": "point", "id": 4, "layerId": "agac", "attrs": {}, "p": { "x": 487030, "y": 4422010 } },
        ],
        "styles": { "items": [], "categories": [] },
    });
    Document::from_snapshot(DocumentSnapshotV1::from_json(&v.to_string()).unwrap()).unwrap()
}

#[test]
fn every_input_that_cannot_run_is_said_together() {
    let mut host = Bare(park());
    // Kırp on an empty layer for both its inputs: both are said, not the first alone.
    let tool = Registry::current().get("geometry.clip").unwrap();
    let mut values = default_values(&tool, &Defaults::of(&host.0));
    for name in ["input", "cut"] {
        values.insert(name.into(), json!({ "scope": "layer", "layerId": "bos" }));
    }
    let outcome = Runner::new().run(&mut host, &tool, &values, false, &mut Vec::new());
    let Outcome::Invalid { issues } = outcome else {
        panic!("{outcome:?}");
    };
    let params: Vec<&str> = issues.iter().filter_map(|i| i.param.as_deref()).collect();
    assert!(
        params.contains(&"input") && params.contains(&"cut"),
        "{issues:?}"
    );
}
