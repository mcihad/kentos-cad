//! A layer's description as a dataset out and back (docs/adr/0633 §5): `fixtures/kcad/v2/datasets.kcad` (a parcel
//! layer with its whole description, an owner layer with a short one and a derivation record) written as a
//! GeoPackage and read into a new drawing: the descriptions come back with their tables, the derivation record stays
//! in the file (its inputs are the other drawing's layers). With KENTOS_WRITE_DATASETS=1 the file goes to
//! fixtures/datasets/v1/out for `scripts/fixtures/dataset_cases.py --written`, which reads it with sqlite3 and GDAL.

use kentos_headless::Session;
use serde_json::json;

use super::{new_drawing, scratch};

fn source() -> Session {
    let path = format!(
        "{}/../../../fixtures/kcad/v2/datasets.kcad",
        env!("CARGO_MANIFEST_DIR")
    );
    Session::open(std::path::Path::new(&path)).expect("opens")
}

fn layer<'a>(s: &'a Session, name: &str) -> &'a kentos_commands::LayerNode {
    fn walk<'a>(
        nodes: &'a [kentos_commands::LayerNode],
        name: &str,
    ) -> Option<&'a kentos_commands::LayerNode> {
        nodes.iter().find_map(|n| {
            (n.name == name)
                .then_some(n)
                .or_else(|| walk(&n.children, name))
        })
    }
    walk(s.layers(), name).expect("the layer")
}

#[test]
fn a_description_goes_out_with_its_table_and_comes_back() {
    let dir = scratch("datasets");
    let s = source();
    let parsel = layer(&s, "Parsel").dataset.clone().expect("described");
    let malik = layer(&s, "Malik").clone();
    assert!(malik.derivation.is_some());
    let path = dir.join("veri-kumeleri.gpkg");
    let mut s = s;
    let out = s
        .call(json!({ "command": "file.export.gpkg", "input": { "path": path.display().to_string() } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    let mut back = new_drawing();
    let got = back
        .call(json!({ "command": "file.import.gpkg", "input": { "path": path.display().to_string() } }))
        .expect("answers");
    assert_eq!(got["status"], "completed", "{got}");
    assert_eq!(got["output"]["datasets"], 2, "{got}");
    assert_eq!(layer(&back, "Parsel").dataset.as_ref(), Some(&parsel));
    let owners = layer(&back, "Malik");
    assert_eq!(owners.dataset, malik.dataset);
    assert!(
        owners.derivation.is_none(),
        "another drawing's inputs are not taken"
    );
    let described = back
        .call(json!({ "command": "cad.datasets.describe", "input": { "records": true } }))
        .expect("answers");
    assert_eq!(described["status"], "completed", "{described}");
    let first = described["output"]["datasets"]
        .as_array()
        .expect("datasets")
        .iter()
        .find(|d| d["name"] == "Parsel")
        .expect("the parcels")
        .clone();
    assert_eq!(first["title"], "Kadastro parselleri", "{described}");
    assert_eq!(first["record"]["properties"]["license"], "CC-BY-4.0");
    if std::env::var_os("KENTOS_WRITE_DATASETS").is_some() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/datasets/v1/out");
        std::fs::create_dir_all(&out).expect("a folder");
        std::fs::copy(&path, out.join("veri-kumeleri.gpkg")).expect("copied");
    }
}
