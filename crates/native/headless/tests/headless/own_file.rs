//! The open drawing's own file given to the commands that take one (docs/adr/0548 §3): a drawing opened from a file
//! runs a raster tool whose raster is named relative to it, whatever the working folder, and its result is written
//! beside it; a file the caller gives stays; a command that takes no file is left as it was; a drawing with no file
//! gives none.

use super::*;

/// A folder holding the COG cases' drawing with its rasters beside it, named relative to it.
fn drawing_beside_its_rasters(name: &str) -> PathBuf {
    let from = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/cog/v1");
    let dir = scratch(name);
    let text = std::fs::read_to_string(from.join("cog.kcad")).expect("reads");
    for e in serde_json::from_str::<Value>(&text).expect("json")["entities"]
        .as_array()
        .expect("entities")
    {
        let file = e["file"].as_str().expect("a file");
        std::fs::copy(from.join("src").join(file), dir.join(file)).expect("a raster copies");
    }
    let path = dir.join("cizim.kcad");
    std::fs::write(&path, text).expect("the drawing writes");
    path
}

fn cog_of(layer: &str) -> Value {
    json!({ "command": "cad.processing.run", "op": "execute", "input": {
        "tool": "raster.cog",
        "parameters": { "input": { "scope": "layer", "layerId": layer }, "add": true } } })
}

#[test]
fn a_drawing_from_a_file_reads_its_rasters_beside_it() {
    let path = drawing_beside_its_rasters("own-file");
    let mut s = Session::open(&path).expect("opens");
    let answer = s.call(cog_of("kot")).expect("answers");
    assert_eq!(answer["status"], "completed", "{answer}");
    let written = path.with_file_name("kot-cog.tif");
    assert!(written.is_file(), "the result beside the drawing: {answer}");
    // The run's step is in the drawing: the raster written beside it.
    assert_eq!(s.document().entities().count(), 5, "{answer}");

    // The same drawing with no file: the relative raster is not found from here, and it says where it looked.
    let bytes = std::fs::read(&path).expect("reads");
    let mut unfiled = Session::from_bytes(&bytes).expect("opens");
    let refused = unfiled.call(cog_of("sinif")).expect("answers");
    assert_eq!(refused["status"], "failed", "{refused}");
}

#[test]
fn a_given_file_stays_and_other_commands_are_left_as_they_were() {
    let path = std::path::Path::new("/tmp/a/cizim.kcad");
    let mut given = json!({ "tool": "raster.cog", "drawing": "/elsewhere/b.kcad" });
    kentos_headless::with_own_file("cad.processing.run", &mut given, Some(path));
    assert_eq!(given["drawing"], "/elsewhere/b.kcad");
    let mut empty = json!({ "tool": "raster.cog", "drawing": null });
    kentos_headless::with_own_file("cad.processing.batch", &mut empty, Some(path));
    assert_eq!(empty["drawing"], "/tmp/a/cizim.kcad");
    for command in [
        "cad.xrefs.list",
        "cad.pointclouds.describe",
        "cad.assets.check",
        "cad.rasters.mosaic",
    ] {
        let mut input = json!({});
        kentos_headless::with_own_file(command, &mut input, Some(path));
        assert_eq!(input["drawing"], "/tmp/a/cizim.kcad", "{command}");
    }
    let mut other = json!({ "uids": [] });
    kentos_headless::with_own_file("cad.entities.delete", &mut other, Some(path));
    assert_eq!(other, json!({ "uids": [] }));
    let mut none = json!({});
    kentos_headless::with_own_file("cad.processing.run", &mut none, None);
    assert_eq!(none, json!({}));
}
