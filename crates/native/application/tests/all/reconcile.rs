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
        ok(&run(
            &mut d,
            "cad.entities.delete",
            CallOp::Execute,
            json!({ "uids": [u] }),
        ));
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
    let w = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(w["error"]["code"], "source_changed", "{w}");
    let r = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        json!({ "layer": layer, "direction": "read" }),
    );
    assert_eq!(r["error"]["code"], "pending_changes", "{r}");
    let untouched = std::fs::read(&tmp).expect("read");
    let input = json!({ "layer": layer, "rest": "layer" });
    let plan = ok(&run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Plan,
        input.clone(),
    ))
    .clone();
    assert_eq!(
        std::fs::read(&tmp).expect("read"),
        untouched,
        "a plan writes nothing"
    );
    let done = ok(&run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Execute,
        input,
    ))
    .clone();
    assert_eq!(
        (&done["fromTable"], &done["toTable"], &done["unresolved"]),
        (&plan["fromTable"], &plan["toTable"], &plan["unresolved"]),
        "{done}"
    );
    assert_ne!(std::fs::read(&tmp).expect("read"), untouched, "written");
    // The two match now: nothing to write, nothing to reload, nothing to reconcile.
    let w = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        json!({ "layer": layer, "direction": "write" }),
    ))
    .clone();
    assert_eq!(
        (&w["added"], &w["modified"], &w["removed"]),
        (&json!(0), &json!(0), &json!(0)),
        "{w}"
    );
    let again = ok(&run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Plan,
        json!({ "layer": layer }),
    ))
    .clone();
    assert_eq!(
        again["toTable"],
        json!({ "added": 0, "modified": 0, "removed": 0 }),
        "{again}"
    );
    assert_eq!(
        again["fromTable"],
        json!({ "added": 0, "modified": 0, "removed": 0 }),
        "{again}"
    );
    // The base survives a save and an open.
    let bytes = kentos_kcad::encode(&d.to_snapshot_v2()).expect("saved");
    let mut opened =
        Document::from_snapshot_v2(kentos_kcad::decode(&bytes).expect("opened")).expect("opened");
    let again = ok(&run(
        &mut opened,
        "cad.sources.reconcile",
        CallOp::Plan,
        json!({ "layer": layer }),
    ))
    .clone();
    assert_eq!(again["agreed"], json!(0), "{again}");
    assert_eq!(
        again["toTable"],
        json!({ "added": 0, "modified": 0, "removed": 0 }),
        "{again}"
    );
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
    let answer = run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Execute,
        json!({ "layer": layer, "rest": "table" }),
    );
    assert_eq!(answer["status"], "failed", "{answer}");
    assert_eq!(
        d.revision(),
        before,
        "the layer's side is undone with the file's refusal"
    );
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    let _ = std::fs::set_permissions(&tmp, perms);
    let _ = std::fs::remove_file(&tmp);
}

/// The lasting id of the object of `layer` whose attribute `name` is `value`.
fn uid_by(doc: &Document, layer: &str, name: &str, value: &str) -> String {
    doc.by_layer_with_uids(layer)
        .find(|(_, e)| e.base().attrs.get(name).map(String::as_str) == Some(value))
        .map(|(u, _)| u.to_string())
        .unwrap_or_else(|| panic!("{layer}.{name} = {value}: no object"))
}

/// Two layers of one file (parcels side by side and the buildings on them), the project's relation, both layers
/// edited and the file changed in QGIS; reconciled together in one call. With KENTOS_WRITE_GPKG the written copy is kept
/// as fixtures/geopackage/v1/kentos-uzlasti-komsu.gpkg, which scripts/fixtures/reconcile_cases.py --written reads.
#[test]
fn two_layers_with_groups_are_reconciled_in_one_transaction() {
    let tmp = std::env::temp_dir().join(format!("kentos-uzlas-komsu-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("uzlas-komsu.gpkg"), &tmp).expect("copied");
    let mut d = doc();
    let path = tmp.display().to_string();
    for table in ["komsu", "bina"] {
        ok(&run(
            &mut d,
            "cad.sources.edit",
            CallOp::Execute,
            json!({ "edit": { "kind": "link", "path": path, "table": table } }),
        ));
    }
    ok(&run(
        &mut d,
        "cad.relations.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "create", "relation": { "id": "bina-komsu", "name": "Parselin binaları",
            "parent": "import-komsu", "child": "import-bina", "field": "komsu" } } }),
    ));
    let (k, b) = ("import-komsu", "import-bina");
    let k_uid = |d: &Document, n: u32| uid_by(d, k, "ad", &format!("K{n}"));
    let grip = |d: &mut Document, n: u32, pts: Value| {
        let u = k_uid(d, n);
        ok(&run(
            d,
            "cad.entities.edit",
            CallOp::Execute,
            json!({ "operation": "grip", "changes": [{ "kind": "update", "uid": u,
                "geometry": { "kind": "polygon", "pts": pts } }] }),
        ));
    };
    let (x0, y0) = (487000.0, 4420000.0);
    let p = |x: f64, y: f64| json!({ "x": x0 + x, "y": y0 + y });
    let u1 = k_uid(&d, 1);
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [u1], "attrs": { "ad": "Z1" }, "operation": "attributes" }),
    ));
    grip(
        &mut d,
        2,
        json!([p(20.0, 0.0), p(40.0, 0.0), p(42.0, 22.0), p(20.0, 20.0)]),
    );
    grip(
        &mut d,
        3,
        json!([p(40.0, 0.0), p(60.0, 0.0), p(60.0, 20.0), p(42.0, 22.0)]),
    );
    let u4 = k_uid(&d, 4);
    ok(&run(
        &mut d,
        "cad.entities.delete",
        CallOp::Execute,
        json!({ "uids": [u4] }),
    ));
    for (n, kat) in [(2, "5"), (3, "7")] {
        let parent = uid_by(&d, k, "ad", &format!("K{n}"));
        let u = uid_by(&d, b, "komsu", &parent);
        ok(&run(
            &mut d,
            "cad.entities.set",
            CallOp::Execute,
            json!({ "uids": [u], "attrs": { "kat": kat }, "operation": "attributes" }),
        ));
    }
    std::fs::copy(dir().join("uzlas-komsu-qgis.gpkg"), &tmp).expect("copied");

    let input = json!({ "layer": k, "with": [b], "rest": "layer" });
    let plan = ok(&run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Plan,
        input.clone(),
    ))
    .clone();
    assert_eq!(plan["groups"].as_array().map(Vec::len), Some(2), "{plan}");
    assert_eq!(plan["layers"].as_array().map(Vec::len), Some(2), "{plan}");
    let before = std::fs::read(&tmp).expect("read");
    let done = ok(&run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Execute,
        input,
    ))
    .clone();
    assert_eq!(done["toTable"], plan["toTable"], "{done}");
    assert_ne!(std::fs::read(&tmp).expect("read"), before, "written");
    // One transaction wrote both tables: the two layers match the file now.
    for layer in [k, b] {
        let again = ok(&run(
            &mut d,
            "cad.sources.reconcile",
            CallOp::Plan,
            json!({ "layer": layer }),
        ))
        .clone();
        assert_eq!(
            again["toTable"],
            json!({ "added": 0, "modified": 0, "removed": 0 }),
            "{again}"
        );
        assert_eq!(
            again["fromTable"],
            json!({ "added": 0, "modified": 0, "removed": 0 }),
            "{again}"
        );
    }
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&tmp, dir().join("kentos-uzlasti-komsu.gpkg")).expect("copied");
    }
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn a_base_without_field_records_conflicts_as_a_whole_and_says_so() {
    let (mut d, tmp, layer) = bound("alansiz");
    // A base taken before the field records (or too big for them): the objects both sides changed cannot be merged.
    let mut settings = d.settings().clone();
    for b in &mut settings.source_bases {
        b.fields = None;
    }
    d.set_settings(settings);
    let answer = run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Plan,
        json!({ "layer": layer }),
    );
    let out = ok(&answer);
    // Parcels 4 and 5 (different fields, 5 would merge), 6 and 7 (a removal): four conflicts, nothing merged.
    assert_eq!(out["conflicts"].as_array().map(Vec::len), Some(4), "{out}");
    assert!(out.get("merged").is_none(), "{out}");
    let warnings = answer["warnings"].as_array().expect("warnings");
    assert!(
        warnings.iter().any(|w| w["code"] == "field_base_missing"),
        "{answer}"
    );
    // With the records it is a merge.
    let (mut d, tmp2, layer) = bound("alanli");
    let answer = run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Plan,
        json!({ "layer": layer }),
    );
    let out = ok(&answer);
    assert_eq!(out["conflicts"].as_array().map(Vec::len), Some(3), "{out}");
    assert_eq!(out["merged"].as_array().map(Vec::len), Some(1), "{out}");
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp2);
}

/// docs/adr/0648 §4: a parcel of more corners than a base record keeps (here K2 with two hundred) still ties to its
/// neighbour through the corner they share: both sides moved it, the two are one group.
#[test]
fn a_many_cornered_parcel_still_groups_with_its_neighbour() {
    let tmp = std::env::temp_dir().join(format!("kentos-uzlas-cok-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("uzlas-komsu.gpkg"), &tmp).expect("copied");
    let path = tmp.display().to_string();
    let k = "import-komsu";
    let link = |d: &mut Document| {
        ok(&run(
            d,
            "cad.sources.edit",
            CallOp::Execute,
            json!({ "edit": { "kind": "link", "path": path, "table": "komsu" } }),
        ));
    };
    let (x0, y0) = (487000.0, 4420000.0);
    let p = |x: f64, y: f64| json!({ "x": x0 + x, "y": y0 + y });
    // K2 with 200 corners on its bottom edge, the corner (40, 20) shared with K3.
    let dense = |corner: (f64, f64)| {
        let mut pts: Vec<Value> = (0..200)
            .map(|i| p(20.0 + 0.1 * f64::from(i), 0.0))
            .collect();
        pts.push(p(40.0, 0.0));
        pts.push(p(corner.0, corner.1));
        pts.push(p(20.0, 20.0));
        Value::Array(pts)
    };
    let grip = |d: &mut Document, n: u32, pts: Value| {
        let u = uid_by(d, k, "ad", &format!("K{n}"));
        ok(&run(
            d,
            "cad.entities.edit",
            CallOp::Execute,
            json!({ "operation": "grip", "changes": [{ "kind": "update", "uid": u,
                "geometry": { "kind": "polygon", "pts": pts } }] }),
        ));
    };
    let mut d = doc();
    link(&mut d);
    grip(&mut d, 2, dense((40.0, 20.0)));
    ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": k, "direction": "write" }),
    ));
    // The table: someone moves the shared corner of K2 only.
    let mut other = doc();
    link(&mut other);
    grip(&mut other, 2, dense((41.0, 21.0)));
    ok(&run(
        &mut other,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": k, "direction": "write" }),
    ));
    // The layer: the corner moves in both.
    grip(&mut d, 2, dense((42.0, 22.0)));
    grip(
        &mut d,
        3,
        json!([p(40.0, 0.0), p(60.0, 0.0), p(60.0, 20.0), p(42.0, 22.0)]),
    );
    let plan = ok(&run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Plan,
        json!({ "layer": k }),
    ))
    .clone();
    let groups = plan["groups"].as_array().cloned().unwrap_or_default();
    assert_eq!(groups.len(), 1, "{plan}");
    assert_eq!(
        groups[0]["objects"].as_array().map(Vec::len),
        Some(2),
        "{plan}"
    );
    assert_eq!(groups[0]["reasons"], json!(["sharedVertex"]), "{plan}");
    let _ = std::fs::remove_file(&tmp);
}
