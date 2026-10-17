// A bound layer written back to its file and read again (docs/adr/0503): the writes the shared cases leave out, on a
// copy of QGIS's GeoPackage. With KENTOS_WRITE_GPKG the written copy is kept as fixtures/geopackage/v1/kentos-synced.gpkg,
// which scripts/fixtures/geopackage_cases.py reads with GDAL's OGR.

use std::path::PathBuf;

use kentos_contracts::{CallOp, LayerField, LayerFieldKind, SourceLink};
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
        "format": "kentos.document", "version": 1, "name": "Bağlı katman",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

fn link_of(doc: &Document) -> SourceLink {
    doc.settings().sources[0].clone()
}

fn state(doc: &mut Document) -> Value {
    ok(&run(
        doc,
        "cad.sources.describe",
        CallOp::Execute,
        json!({}),
    ))["links"][0]
        .clone()
}

#[test]
fn a_layer_is_written_to_its_table_and_read_again() {
    let tmp = std::env::temp_dir().join(format!("kentos-sources-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("qgis.gpkg"), &tmp).expect("copied");
    let path = tmp.display().to_string();
    let mut d = doc();
    let linked = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": path, "table": "parsel" } }),
    );
    assert_eq!(ok(&linked)["objects"], 3);
    let layer = link_of(&d).layer;
    let uids: Vec<String> = d
        .by_layer_with_uids(&layer)
        .map(|(u, _)| u.to_string())
        .collect();
    // Edit: one parcel renamed, one deleted, a new one drawn, a field the table lacks.
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uids[0]], "attrs": { "ada": "Yeni Mahalle", "durum": "onaylı" }, "operation": "attributes" }),
    ));
    ok(&run(
        &mut d,
        "cad.entities.delete",
        CallOp::Execute,
        json!({ "uids": [uids[1]] }),
    ));
    let drawn = ok(&run(
        &mut d,
        "cad.polygon.create",
        CallOp::Execute,
        json!({ "layerId": layer, "pts": [{ "x": 487500.0, "y": 4420000.0 }, { "x": 487510.0, "y": 4420000.0 },
                                         { "x": 487510.0, "y": 4420010.0 }] }),
    ))["uid"]
        .as_str()
        .expect("uid")
        .to_owned();
    let mut fields = d.layers().get(&layer).expect("layer").fields.clone();
    fields.push(
        serde_json::from_value::<LayerField>(json!({ "name": "durum", "kind": "text" }))
            .expect("field"),
    );
    assert_eq!(fields.last().map(|f| f.kind), Some(LayerFieldKind::Text));
    d.set_layer_fields(&layer, fields, &[]).expect("fields");
    let before = state(&mut d);
    assert_eq!(
        (
            before["edited"].clone(),
            before["added"].clone(),
            before["modified"].clone(),
            before["removed"].clone()
        ),
        (json!(true), json!(1), json!(1), json!(1)),
        "{before}"
    );
    let input = json!({ "layer": layer, "direction": "write" });
    let untouched = std::fs::read(&tmp).expect("read");
    let plan = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        input.clone(),
    ))
    .clone();
    assert!(
        std::fs::read(&tmp).expect("read") == untouched,
        "a plan writes nothing"
    );
    let written = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        input.clone(),
    ))
    .clone();
    assert_eq!(
        (
            written["added"].clone(),
            written["modified"].clone(),
            written["removed"].clone(),
            written["columns"].clone()
        ),
        (json!(1), json!(1), json!(1), json!(["durum"])),
        "{written}"
    );
    assert_eq!(plan["added"], written["added"]);
    assert_ne!(written["source"], plan["source"], "the file changed");
    assert_eq!(
        link_of(&d).revision,
        written["source"].as_str().unwrap_or_default()
    );
    let after = state(&mut d);
    assert_eq!(
        (
            after["changed"].clone(),
            after["edited"].clone(),
            after["added"].clone(),
            after["modified"].clone(),
            after["removed"].clone()
        ),
        (json!(false), json!(false), json!(0), json!(0), json!(0)),
        "a written layer matches its table: {after}"
    );
    // The file holds what the layer does: read into a fresh drawing, the same objects by id.
    let mut fresh = doc();
    ok(&run(
        &mut fresh,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": path, "table": "parsel" } }),
    ));
    let mut got: Vec<String> = fresh
        .by_layer_with_uids(&link_of(&fresh).layer)
        .map(|(u, _)| u.to_string())
        .collect();
    let mut want: Vec<String> = d
        .by_layer_with_uids(&layer)
        .map(|(u, _)| u.to_string())
        .collect();
    got.sort();
    want.sort();
    assert_eq!(got, want);
    assert!(got.contains(&drawn));
    // Another program changes the file: a write is refused, a read takes it.
    {
        let mut gp = kentos_geopackage::GeoPackage::open(&tmp, true).expect("opens");
        let table = gp
            .layers()
            .expect("layers")
            .into_iter()
            .find(|l| l.name == "parsel")
            .expect("parsel");
        gp.edit(
            &table,
            &kentos_geopackage::Edits {
                delete: vec![3],
                ..Default::default()
            },
        )
        .expect("edited");
    }
    let changed = state(&mut d);
    assert_eq!(
        (
            changed["changed"].clone(),
            changed["removed"].clone(),
            changed["edited"].clone()
        ),
        (json!(true), json!(0), json!(false)),
        "{changed}"
    );
    let refused = run(&mut d, "cad.sources.sync", CallOp::Execute, input);
    assert_eq!(refused["status"], "conflict");
    assert_eq!(refused["error"]["code"], "source_changed");
    let read = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "read" }),
    ))
    .clone();
    assert_eq!(read["removed"], 1, "{read}");
    let last = state(&mut d);
    assert_eq!(
        (last["changed"].clone(), last["edited"].clone()),
        (json!(false), json!(false)),
        "{last}"
    );
    // Undoing the read brings the object back: the layer differs from the file again, as an edit.
    assert!(d.undo().is_some());
    let undone = state(&mut d);
    assert_eq!(
        (undone["edited"].clone(), undone["added"].clone()),
        (json!(true), json!(1)),
        "{undone}"
    );
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        assert!(d.redo().is_some());
        std::fs::copy(&tmp, dir().join("kentos-synced.gpkg")).expect("copied");
    }
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn a_file_moved_is_bound_again_and_another_file_is_not() {
    let tmp =
        std::env::temp_dir().join(format!("kentos-sources-moved-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("qgis.gpkg"), &tmp).expect("copied");
    let mut d = doc();
    ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": tmp.display().to_string(), "table": "nokta" } }),
    ));
    let moved = tmp.with_extension("tasindi.gpkg");
    std::fs::rename(&tmp, &moved).expect("moved");
    let layer = link_of(&d).layer;
    let gone = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        json!({ "layer": layer, "direction": "read" }),
    );
    assert_eq!(gone["error"]["code"], "source_gone");
    assert_eq!(state(&mut d)["found"], false);
    ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "relink", "id": "kaynak-1", "path": moved.display().to_string() } }),
    ));
    assert_eq!(state(&mut d)["found"], true);
    let _ = std::fs::remove_file(&moved);
}

#[test]
fn a_project_keeps_at_most_a_thousand_bound_layers() {
    let mut d = doc();
    let mut settings = d.settings().clone();
    settings.sources = (1..=1000)
        .map(|n| SourceLink {
            id: format!("kaynak-{n}"),
            kind: kentos_contracts::SourceKind::Geopackage,
            path: format!("/veri/{n}.gpkg"),
            table: "t".into(),
            layer: format!("katman-{n}"),
            revision: "0".repeat(64),
            digest: "0".repeat(64),
            srid: None,
            writable: true,
        })
        .collect();
    d.set_settings(settings);
    let path = dir().join("qgis.gpkg").display().to_string();
    let refused = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Plan,
        json!({ "edit": { "kind": "link", "path": path, "table": "parsel" } }),
    );
    assert_eq!(refused["error"]["code"], "too_many_sources");
}

/// A table in WGS 84 bound to a TM36 drawing (docs/adr/0504 §1): the point moved 10 m east in the drawing is written
/// back in degrees, the untouched round trip writes nothing. With KENTOS_WRITE_GPKG the file is kept as
/// fixtures/geopackage/v1/kentos-synced-wgs84.gpkg; geopackage_cases.py moves it into TM36 with PROJ and compares.
#[test]
fn a_table_in_another_system_is_written_back_in_its_own() {
    let tmp = std::env::temp_dir().join(format!("kentos-sources-wgs-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("wgs84.gpkg"), &tmp).expect("copied");
    let mut d = doc();
    let mut settings = d.settings().clone();
    settings.srid = 5256;
    d.set_settings(settings);
    let path = tmp.display().to_string();
    let refused = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Plan,
        json!({ "edit": { "kind": "link", "path": path, "table": "yer" } }),
    );
    assert_eq!(refused["error"]["code"], "crs_mismatch");
    ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": path, "table": "yer", "reproject": true } }),
    ));
    let layer = link_of(&d).layer;
    let input = json!({ "layer": layer, "direction": "write" });
    let nothing = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        input.clone(),
    ))
    .clone();
    assert_eq!(
        (
            nothing["added"].clone(),
            nothing["modified"].clone(),
            nothing["removed"].clone()
        ),
        (json!(0), json!(0), json!(0))
    );
    let uid = d
        .by_layer_with_uids(&layer)
        .next()
        .map(|(u, _)| u.to_string())
        .expect("the point");
    ok(&run(
        &mut d,
        "cad.entities.transform",
        CallOp::Execute,
        json!({ "uids": [uid], "transform": { "kind": "move", "dx": 10.0, "dy": 0.0 } }),
    ));
    let written = ok(&run(&mut d, "cad.sources.sync", CallOp::Execute, input)).clone();
    assert_eq!(written["modified"], 1, "{written}");
    assert_eq!(link_of(&d).srid, Some(4326));
    let after = state(&mut d);
    assert_eq!(
        (after["edited"].clone(), after["modified"].clone()),
        (json!(false), json!(0)),
        "{after}"
    );
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&tmp, dir().join("kentos-synced-wgs84.gpkg")).expect("copied");
    }
    let _ = std::fs::remove_file(&tmp);
}
