//! COG olarak kaydet through `cad.processing.run` where there is no window (docs/adr/0515): the drawing's four rasters
//! (a stripped float GeoTIFF with a hole, a tiled 16-bit one with GDAL's overviews and a turned grid, a PNG, an RGBA
//! TIFF; fixtures/cog/v1, written by GDAL) are read through the headless server's files and written as COGs; each
//! result laid out again is itself, the drawing gets its raster in one step. With `KENTOS_WRITE_COG=1` the files go
//! to fixtures/cog/v1/out for `scripts/fixtures/cog_cases.py`, which checks them with GDAL's COG validator and numpy.

use std::path::PathBuf;

use kentos_commands::{CallOp, DocumentSnapshotV1};
use kentos_domain::Document;
use kentos_formats::raster::cog;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/cog/v1")
}

/// The drawing, its rasters' files named where they are.
fn drawing() -> Document {
    let text = std::fs::read_to_string(folder().join("cog.kcad")).expect("reads");
    let mut v: Value = serde_json::from_str(&text).expect("json");
    for e in v["entities"].as_array_mut().expect("entities") {
        let name = e["file"].as_str().expect("a file").to_owned();
        e["file"] = json!(folder().join("src").join(name).display().to_string());
    }
    Document::from_snapshot(DocumentSnapshotV1::from_json(&v.to_string()).expect("a drawing"))
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
fn rasters_go_out_as_cogs_through_the_headless_files() {
    let dir = std::env::temp_dir().join(format!("kentos-cog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let layers = [
        ("kot", "kot"),
        ("sinif", "sinif"),
        ("orto", "orto"),
        ("renkli", "saydam"),
    ];
    for (layer, stem) in layers {
        let mut doc = drawing();
        let rasters = doc.entities().count();
        let path = dir.join(format!("{stem}-cog.tif"));
        let got = run(
            &mut doc,
            json!({ "tool": "raster.cog", "parameters": {
                "input": { "scope": "layer", "layerId": layer },
                "output": path.display().to_string(), "add": true } }),
        );
        assert_eq!(got["status"], "completed", "{layer}: {got}");
        let bytes = std::fs::read(&path).expect("written");
        assert!(
            cog::declared(&bytes[..256]),
            "{layer}: GDAL's structure first"
        );
        assert_eq!(
            cog::of_bytes(&bytes).as_ref(),
            Ok(&bytes),
            "{layer}: laid out again, a COG is itself"
        );
        // The raster in the drawing, in the run's one step; nothing left beside the file.
        assert_eq!(doc.entities().count(), rasters + 1, "{layer}");
        assert!(doc.can_undo(), "{layer}");
        let stray: Vec<_> = std::fs::read_dir(&dir)
            .expect("lists")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("yaziliyor"))
            .collect();
        assert!(stray.is_empty(), "{layer}: {stray:?}");
        // The same raster gives the same bytes.
        let again = dir.join(format!("{stem}-cog-2.tif"));
        let twice = run(
            &mut drawing(),
            json!({ "tool": "raster.cog", "parameters": {
                "input": { "scope": "layer", "layerId": layer },
                "output": again.display().to_string(), "add": false } }),
        );
        assert_eq!(twice["status"], "completed", "{layer}: {twice}");
        assert_eq!(std::fs::read(&again).expect("written"), bytes, "{layer}");
        if std::env::var_os("KENTOS_WRITE_COG").is_some() {
            let out = folder().join("out");
            std::fs::create_dir_all(&out).expect("a folder");
            std::fs::copy(&path, out.join(format!("{stem}-cog.tif"))).expect("kept");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn what_the_headless_files_cannot_read_is_said() {
    // A raster at an address: read where the host has a network (docs/adr/0645 §5); this test's host has none.
    let text = std::fs::read_to_string(folder().join("cog.kcad")).expect("reads");
    let mut v: Value = serde_json::from_str(&text).expect("json");
    v["entities"][0]["file"] = Value::Null;
    v["entities"][0]["url"] = json!("https://example.org/kot.tif");
    let mut remote =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&v.to_string()).expect("a drawing"))
            .expect("opens");
    let got = run(
        &mut remote,
        json!({ "tool": "raster.cog", "parameters": {
            "input": { "scope": "layer", "layerId": "kot" }, "output": "/tmp/kentos-uzak.tif" } }),
    );
    assert_ne!(got["status"], "completed", "{got}");
    assert!(got.to_string().contains("ağa çıkmıyor"), "{got}");
    // A file that is not there names itself.
    v["entities"][0]["url"] = Value::Null;
    v["entities"][0]["file"] = json!("/yok/kot.tif");
    let mut gone =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&v.to_string()).expect("a drawing"))
            .expect("opens");
    let got = run(
        &mut gone,
        json!({ "tool": "raster.cog", "parameters": {
            "input": { "scope": "layer", "layerId": "kot" }, "output": "/tmp/kentos-yok.tif" } }),
    );
    assert!(got.to_string().contains("/yok/kot.tif"), "{got}");
}
