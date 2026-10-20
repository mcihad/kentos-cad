//! `cad.processing.run` writing its result to a GeoPackage (docs/adr/0508 §4): G1's parcels × zoning, a buffer of
//! the roads and a dissolve by Ada go to new files, the drawing unchanged; a file that is there is replaced only with
//! `overwrite`. With `KENTOS_WRITE_GPKG=1` the files are kept in fixtures/geopackage/v1 (`kentos-run-*.gpkg`) for
//! `scripts/fixtures/processing_sink_cases.py`, which reads them with OGR and checks them against the source.

use std::path::PathBuf;

use kentos_commands::{CallOp, DocumentSnapshotV1};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures")
}

fn drawing() -> Document {
    let text =
        std::fs::read_to_string(fixtures().join("processing/v1/geometry.kcad")).expect("reads");
    Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("a drawing"))
        .expect("opens")
}

fn run(doc: &mut Document, input: Value) -> Value {
    kentos_processing::command::register();
    registry::run(
        &mut ExecutionContext::new(doc),
        "cad.processing.run",
        CallOp::Execute,
        input,
    )
    .expect("runs")
}

#[test]
fn g1_results_go_to_geopackages_and_the_drawing_stays() {
    let dir = std::env::temp_dir().join(format!("kentos-sink-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let runs = [
        (
            "kesisim",
            json!({ "tool": "geometry.intersection", "parameters": {
                "input": { "scope": "layer", "layerId": "parsel" },
                "overlay": { "scope": "layer", "layerId": "imar" },
                "prefix": "", "apportion": "Değer" } }),
        ),
        (
            "tampon",
            json!({ "tool": "geometry.buffer", "parameters": {
                "input": { "scope": "layer", "layerId": "yol" }, "distance": 5 } }),
        ),
        (
            "birlestirilen",
            json!({ "tool": "geometry.dissolve", "parameters": {
                "input": { "scope": "layer", "layerId": "parsel" }, "group": "Ada", "sums": "Değer" } }),
        ),
    ];
    for (name, mut input) in runs {
        let mut doc = drawing();
        let before = doc.revision();
        let path = dir.join(format!("kentos-run-{name}.gpkg"));
        input["sink"] = json!({ "path": path.display().to_string() });
        let got = run(&mut doc, input.clone());
        assert_eq!(got["status"], "completed", "{name}: {got}");
        let sink = &got["output"]["sink"];
        assert!(path.exists(), "{name}: the file is written");
        let bytes = std::fs::read(&path).expect("reads");
        assert_eq!(sink["bytes"], json!(bytes.len()), "{name}");
        assert_eq!(
            sink["sha256"],
            json!(kentos_commands::encoding::sha256_hex(&bytes)),
            "{name}"
        );
        assert_eq!(
            doc.revision(),
            before,
            "{name}: the drawing does not change"
        );
        assert!(!doc.can_undo(), "{name}: no undo step");
        assert!(got["output"].get("step").is_none(), "{name}");
        // No passing file is left beside it.
        let stray: Vec<_> = std::fs::read_dir(&dir)
            .expect("lists")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".kentos-"))
            .collect();
        assert!(stray.is_empty(), "{name}: {stray:?}");
        // There now: refused without overwrite, the file as it was; replaced with it.
        let again = run(&mut doc, input.clone());
        assert_eq!(again["error"]["code"], "file_exists", "{name}: {again}");
        assert_eq!(std::fs::read(&path).expect("reads"), bytes);
        input["sink"]["overwrite"] = json!(true);
        let replaced = run(&mut doc, input);
        assert_eq!(replaced["status"], "completed", "{name}: {replaced}");
        if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
            std::fs::copy(
                &path,
                fixtures().join(format!("geopackage/v1/kentos-run-{name}.gpkg")),
            )
            .expect("keeps the file");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
