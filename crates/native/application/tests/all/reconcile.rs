// A bound layer reconciled with its table after both changed (docs/adr/0538): the writes the shared cases check by
// their answers, on a copy of uzlas.gpkg changed into uzlas-qgis.gpkg between the link and the reconcile. With
// KENTOS_WRITE_GPKG the written copy is kept as fixtures/geopackage/v1/kentos-uzlasti.gpkg, which
// scripts/fixtures/reconcile_cases.py --written reads with GDAL's OGR.

use std::path::PathBuf;

use kentos_commands::CallOp;
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/geopackage/v1")
}

fn run(doc: &mut Document, command: &str, op: CallOp, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), command, op, input).expect("runs")
}

fn ok(answer: &Value) -> &Value {
    assert_eq!(answer["status"], "completed", "{answer}");
    &answer["output"]
}

fn doc() -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Uzlaştırma",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

/// The lasting id of the parcel numbered `fid` (uzlas.gpkg numbers its rows by their fid).
fn uid_of(doc: &Document, layer: &str, fid: u32) -> String {
    doc.by_layer_with_uids(layer)
        .find(|(_, e)| e.base().attrs.get("parsel_no") == Some(&fid.to_string()))
        .map(|(u, _)| u.to_string())
        .unwrap_or_else(|| panic!("{fid}: no parcel"))
}

/// The temporary copy bound, the layer edited as the shared cases edit it, the file changed as QGIS changed it.
fn bound(name: &str) -> (Document, PathBuf, String) {
    let tmp = std::env::temp_dir().join(format!("kentos-uzlas-{name}-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("uzlas.gpkg"), &tmp).expect("copied");
    let mut d = doc();
    ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": tmp.display().to_string(), "table": "parsel" } }),
    ));
    let layer = "import-parsel".to_owned();
    let set = |d: &mut Document, fid: u32, attrs: Value| {
        let u = uid_of(d, &layer, fid);
        ok(&run(
            d,
            "cad.entities.set",
            CallOp::Execute,
            json!({ "uids": [u], "attrs": attrs, "operation": "attributes" }),
        ));
    };
    set(&mut d, 4, json!({ "malik": "Veli" }));
    set(&mut d, 5, json!({ "parsel_no": "50" }));
    set(&mut d, 6, json!({ "malik": "Mehmet" }));
    for fid in [7, 9] {
        let u = uid_of(&d, &layer, fid);
        ok(&run(&mut d, "cad.entities.delete", CallOp::Execute, json!({ "uids": [u] })));
    }
    set(&mut d, 8, json!({ "malik": "Selin" }));
    set(&mut d, 10, json!({ "malik": "Ortak" }));
    ok(&run(
        &mut d,
        "cad.polygon.create",
        CallOp::Execute,
        json!({ "layerId": layer, "pts": [{ "x": 487400.0, "y": 4420000.0 }, { "x": 487420.0, "y": 4420000.0 },
                                          { "x": 487420.0, "y": 4420020.0 }, { "x": 487400.0, "y": 4420020.0 }] }),
    ));
    std::fs::copy(dir().join("uzlas-qgis.gpkg"), &tmp).expect("copied");
    (d, tmp, layer)
}

#[test]
fn a_reconcile_writes_both_sides_and_takes_the_base_again() {
    let (mut d, tmp, layer) = bound("katman");
    // A write or a reload alone would lose one side's work.
    let w = run(&mut d, "cad.sources.sync", CallOp::Plan, json!({ "layer": layer, "direction": "write" }));
    assert_eq!(w["error"]["code"], "source_changed", "{w}");
    let r = run(&mut d, "cad.sources.sync", CallOp::Plan, json!({ "layer": layer, "direction": "read" }));
    assert_eq!(r["error"]["code"], "pending_changes", "{r}");
    let untouched = std::fs::read(&tmp).expect("read");
    let input = json!({ "layer": layer, "rest": "layer" });
    let plan = ok(&run(&mut d, "cad.sources.reconcile", CallOp::Plan, input.clone())).clone();
    assert_eq!(std::fs::read(&tmp).expect("read"), untouched, "a plan writes nothing");
    let done = ok(&run(&mut d, "cad.sources.reconcile", CallOp::Execute, input)).clone();
    assert_eq!(
        (&done["fromTable"], &done["toTable"], &done["unresolved"]),
        (&plan["fromTable"], &plan["toTable"], &plan["unresolved"]),
        "{done}"
    );
    assert_ne!(std::fs::read(&tmp).expect("read"), untouched, "written");
    // The two match now: nothing to write, nothing to reload, nothing to reconcile.
    let w = ok(&run(&mut d, "cad.sources.sync", CallOp::Plan, json!({ "layer": layer, "direction": "write" }))).clone();
    assert_eq!((&w["added"], &w["modified"], &w["removed"]), (&json!(0), &json!(0), &json!(0)), "{w}");
    let again = ok(&run(&mut d, "cad.sources.reconcile", CallOp::Plan, json!({ "layer": layer }))).clone();
    assert_eq!(again["toTable"], json!({ "added": 0, "modified": 0, "removed": 0 }), "{again}");
    assert_eq!(again["fromTable"], json!({ "added": 0, "modified": 0, "removed": 0 }), "{again}");
    // The base survives a save and an open.
    let bytes = kentos_kcad::encode(&d.to_snapshot_v2()).expect("saved");
    let mut opened = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).expect("opened")).expect("opened");
    let again = ok(&run(&mut opened, "cad.sources.reconcile", CallOp::Plan, json!({ "layer": layer }))).clone();
    assert_eq!(again["agreed"], json!(0), "{again}");
    assert_eq!(again["toTable"], json!({ "added": 0, "modified": 0, "removed": 0 }), "{again}");
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&tmp, dir().join("kentos-uzlasti.gpkg")).expect("copied");
    }
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn a_file_that_cannot_be_written_takes_nothing_into_the_layer() {
    let (mut d, tmp, layer) = bound("salt");
    let before = d.revision();
    let mut perms = std::fs::metadata(&tmp).expect("file").permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&tmp, perms.clone()).expect("read-only");
    let answer = run(&mut d, "cad.sources.reconcile", CallOp::Execute, json!({ "layer": layer, "rest": "table" }));
    assert_eq!(answer["status"], "failed", "{answer}");
    assert_eq!(d.revision(), before, "the layer's side is undone with the file's refusal");
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    let _ = std::fs::set_permissions(&tmp, perms);
    let _ = std::fs::remove_file(&tmp);
}
