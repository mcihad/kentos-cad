//! `file.export.parquet` and `file.import.parquet` end to end (docs/adr/0628): the GeoPackage's layers written as
//! GeoParquets (Snappy; one without compression and without the bbox column) and read back into a new drawing, the
//! same objects; the whole drawing in one file with its `katman` column; a file taken again into the layer it came
//! from by a merge on its lasting ids adds nothing. With KENTOS_WRITE_PARQUET=1 the files go to
//! fixtures/geoparquet/v1/out for `scripts/fixtures/geoparquet_cases.py --written`, which opens them with pyarrow,
//! geopandas, shapely and pyproj.

use serde_json::{Value, json};

use super::{new_drawing, scratch};

#[test]
fn geoparquets_go_out_and_come_back() {
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let id = |name: &str| -> String {
        took["output"]["layers"]
            .as_array()
            .and_then(|ls| ls.iter().find(|l| l["name"] == name))
            .and_then(|l| l["id"].as_str())
            .unwrap_or_else(|| panic!("{name}"))
            .to_owned()
    };
    let dir = scratch("parquet");
    let mut written = Vec::new();
    for (name, layers, compression, covering) in [
        ("parsel", vec![id("parsel")], "snappy", true),
        ("yol", vec![id("yol")], "snappy", true),
        ("nokta", vec![id("nokta")], "snappy", true),
        ("egri", vec![id("egri")], "snappy", true),
        ("egri_yol", vec![id("egri_yol")], "none", false),
        ("hepsi", Vec::new(), "snappy", true),
    ] {
        let path = dir.join(format!("kentos-{name}.parquet"));
        let input = json!({ "path": path.display().to_string(), "layers": layers,
            "compression": compression, "covering": covering });
        let planned = s
            .call(json!({ "command": "file.export.parquet", "op": "plan", "input": input }))
            .expect("answers");
        assert_eq!(planned["status"], "completed", "{planned}");
        assert!(!path.exists(), "plan writes nothing");
        let out = s
            .call(json!({ "command": "file.export.parquet", "input": input }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{out}");
        assert_eq!(
            out["output"], planned["output"],
            "plan says what execute writes"
        );
        // The covering column is a group of four leaves: bbox.xmin, bbox.ymin, bbox.xmax, bbox.ymax.
        assert_eq!(
            out["output"]["columns"].as_array().is_some_and(|c| {
                c.iter()
                    .any(|c| c.as_str().is_some_and(|c| c.starts_with("bbox.")))
            }),
            covering,
            "{name}: {out}"
        );
        let mut back = new_drawing();
        let got = back
            .call(json!({ "command": "file.import.parquet", "input": { "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(got["status"], "completed", "{got}");
        assert_eq!(
            got["output"]["objects"], out["output"]["rows"],
            "{name}: {got}"
        );
        written.push((name, path));
    }
    if std::env::var_os("KENTOS_WRITE_PARQUET").is_some() {
        let out_dir = format!(
            "{}/../../../fixtures/geoparquet/v1/out",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::create_dir_all(&out_dir).expect("a folder");
        for (name, path) in &written {
            std::fs::copy(path, format!("{out_dir}/kentos-{name}.parquet")).expect("kept");
        }
    }
    // The parcels' file back into the layer they came from, met by their lasting ids: nothing added, nothing doubled.
    let merged = s
        .call(json!({ "command": "file.import.parquet",
            "input": { "path": written[0].1.display().to_string(), "layer": id("parsel"), "merge": {} } }))
        .expect("answers");
    assert_eq!(merged["status"], "completed", "{merged}");
    let report: &Value = &merged["output"]["merged"][0];
    assert_eq!(report["added"], 0, "{merged}");
    assert_eq!(report["rejected"], 0, "{merged}");
    assert_eq!(merged["output"]["objects"], 0, "{merged}");
    let again = s
        .call(json!({ "command": "file.export.parquet",
            "input": { "path": written[0].1.display().to_string(), "layers": [id("parsel")] } }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists", "{again}");
}
