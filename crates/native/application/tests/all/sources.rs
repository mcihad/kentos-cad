// A bound layer written back to its file and read again (docs/adr/0503): the writes the shared cases leave out, on a
// copy of QGIS's GeoPackage. With KENTOS_WRITE_GPKG the written copy is kept as fixtures/geopackage/v1/kentos-synced.gpkg,
// which scripts/fixtures/geopackage_cases.py reads with GDAL's OGR.

use std::path::PathBuf;

use kentos_commands::{CallOp, LayerField, LayerFieldKind, SourceLink};
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
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
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
            kind: kentos_commands::SourceKind::Geopackage,
            path: format!("/veri/{n}.gpkg"),
            relative: None,
            table: "t".into(),
            layer: format!("katman-{n}"),
            revision: "0".repeat(64),
            digest: "0".repeat(64),
            srid: None,
            key: None,
            extent: None,
            parked: Vec::new(),
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

// docs/adr/0505: where the host registered no PostGIS provider (as in these tests), a database is not reached: the
// listing, the link and the sync say so; the layer's state says the database is not there; unlinking needs nothing.
#[test]
fn without_a_provider_a_database_is_not_reached() {
    let pg = "postgresql://cbs@veri.belediye.local:5432/kent";
    let fake = "0".repeat(64);
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "PostGIS",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis",
                      "sources": [{ "id": "kaynak-1", "kind": "postgis", "path": pg, "table": "kadastro.parsel",
                                    "layer": "bagli", "revision": fake, "digest": fake, "srid": 5256, "writable": true }] },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "bagli", "name": "Bağlı", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "bagli", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    let mut d = Document::from_snapshot(snapshot).expect("setup");
    let unreachable = |answer: Value, at: &str| {
        assert_eq!(answer["error"]["code"], "source_unreachable", "{answer}");
        assert_eq!(answer["error"]["path"], at, "{answer}");
        assert!(
            answer["error"]["message"]
                .as_str()
                .is_some_and(|m| m.contains("PostGIS sağlayıcısı"))
        );
    };
    unreachable(
        run(
            &mut d,
            "cad.sources.describe",
            CallOp::Execute,
            json!({ "path": pg }),
        ),
        "path",
    );
    unreachable(
        run(
            &mut d,
            "cad.sources.edit",
            CallOp::Plan,
            json!({ "edit": { "kind": "link", "path": pg, "table": "kadastro.yol" } }),
        ),
        "edit.path",
    );
    unreachable(
        run(
            &mut d,
            "cad.sources.sync",
            CallOp::Plan,
            json!({ "layer": "bagli", "direction": "read" }),
        ),
        "layer",
    );
    let s = state(&mut d);
    assert_eq!(
        (s["found"].clone(), s["changed"].clone()),
        (json!(false), json!(false)),
        "{s}"
    );
    ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "unlink", "id": "kaynak-1" } }),
    ));
    assert!(d.settings().sources.is_empty());
}

/// The values whose meaning a round trip keeps (plan/gis.md CBS-06, docs/adr/0512): OGR's `genis.gpkg` holds an integer
/// past 2⁵³ and the smallest 64-bit one, 0 and false, a double that has no short decimal, an empty text beside NULL, a
/// time with its zone and one in UTC. Linked, one row's text changed and written back: every other cell is as it was,
/// its value and its type. With KENTOS_WRITE_GPKG the written copy is kept as fixtures/geopackage/v1/kentos-genis.gpkg,
/// which scripts/fixtures/wide_values_cases.py compares with the source cell by cell (sqlite3).
#[test]
fn wide_values_keep_their_meaning_through_a_bound_layer() {
    let tmp = std::env::temp_dir().join(format!("kentos-genis-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("genis.gpkg"), &tmp).expect("copied");
    let path = tmp.display().to_string();
    let mut d = doc();
    let linked = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": path, "table": "deger" } }),
    );
    assert_eq!(ok(&linked)["objects"], 3, "{linked}");
    // Nothing lost (docs/adr/0629 §4): the integer past 2⁵³ a big integer, the zoned time a time stamp.
    assert_eq!(linked["warnings"], json!([]), "{linked}");
    let layer = link_of(&d).layer;
    let kind = |name: &str| {
        d.layers()
            .get(&layer)
            .and_then(|n| n.fields.iter().find(|f| f.name == name).map(|f| f.kind))
    };
    assert_eq!(kind("buyuk"), Some(LayerFieldKind::BigInt));
    assert_eq!(kind("an"), Some(LayerFieldKind::Instant));
    assert_eq!(kind("sayi"), Some(LayerFieldKind::Integer));
    let (uid, attrs) = d
        .by_layer_with_uids(&layer)
        .find(|(_, e)| {
            e.base()
                .attrs
                .get("metin")
                .is_some_and(|m| m.starts_with('ş'))
        })
        .map(|(u, e)| (u, e.base().attrs.clone()))
        .expect("the third row");
    assert!(
        !attrs.contains_key("buyuk"),
        "NULL is the attribute's absence: {attrs:?}"
    );
    // Right after linking nothing differs: a zone the drawing does not keep is not a change.
    let fresh = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(ok(&fresh)["modified"], 0, "{fresh}");
    let first = d
        .by_layer_with_uids(&layer)
        .find(|(_, e)| e.base().attrs.get("sayi").map(String::as_str) == Some("0"))
        .map(|(_, e)| e.base().attrs.clone())
        .expect("the first row");
    assert_eq!(
        first.get("buyuk").map(String::as_str),
        Some("9007199254740993"),
        "{first:?}"
    );
    assert_eq!(
        first.get("metin").map(String::as_str),
        Some(""),
        "an empty text is one: {first:?}"
    );
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uid.to_string()], "attrs": { "metin": "değişti" }, "operation": "attributes" }),
    ));
    let synced = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(ok(&synced)["modified"], 1, "{synced}");
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&tmp, dir().join("kentos-genis.gpkg")).expect("kept");
    }
    // One past 2⁵³ to one below it: the same double, a different integer, a change.
    let big = d
        .by_layer_with_uids(&layer)
        .find(|(_, e)| e.base().attrs.get("buyuk").map(String::as_str) == Some("9007199254740993"))
        .map(|(u, _)| u)
        .expect("the first row");
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [big.to_string()], "attrs": { "buyuk": "9007199254740992" }, "operation": "attributes" }),
    ));
    let planned = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(ok(&planned)["modified"], 1, "{planned}");
    let _ = std::fs::remove_file(&tmp);
}

// ── Label pins when the table is written again by another program (docs/adr/0533 §1) ──

/// Another program writes the parcels again: the first two rows are deleted and inserted anew (other row ids, other
/// lasting ids, the same values), and `extra` more rows repeat the first's values.
fn rewrite(file: &std::path::Path, extra: usize) {
    rewrite_nudged(file, extra, 0.0);
}

/// `rewrite`, the two rows' polygons written `dx` metres east of where they were (the extra rows' as they were).
fn rewrite_nudged(file: &std::path::Path, extra: usize, dx: f64) {
    use kentos_geopackage::{Coord, Curve, Edits, GeoPackage, Geometry, NewRow, Query};
    let nudge = |g: &Geometry| -> Geometry {
        let shift = |c: &Curve| match c {
            Curve::Line(p) => Curve::Line(
                p.iter()
                    .map(|q| Coord {
                        x: q.x + dx,
                        ..*q
                    })
                    .collect(),
            ),
            other => other.clone(),
        };
        match g {
            Geometry::Polygon(rings) => Geometry::Polygon(rings.iter().map(shift).collect()),
            Geometry::MultiPolygon(parts) => Geometry::MultiPolygon(
                parts
                    .iter()
                    .map(|rings| rings.iter().map(shift).collect())
                    .collect(),
            ),
            other => other.clone(),
        }
    };
    let mut gp = GeoPackage::open(file, true).expect("opens");
    let layer = gp
        .layers()
        .expect("layers")
        .into_iter()
        .find(|l| l.name == "parsel")
        .expect("the parcels");
    let rows = gp.rows(&layer, &Query::default()).expect("rows");
    let copy = |r: &kentos_geopackage::Row| NewRow {
        fid: None,
        geometry: match &r.geometry {
            Some(Ok(g)) => Some(g.clone()),
            _ => None,
        },
        values: r.values.clone(),
    };
    let mut edits = Edits::default();
    for (i, r) in rows.iter().take(2).enumerate() {
        edits.delete.push(r.fid);
        let mut row = copy(r);
        if let Some(g) = &row.geometry {
            row.geometry = Some(nudge(g));
        }
        edits.insert.push((
            format!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d77{i:02}"),
            row,
        ));
    }
    for k in 0..extra {
        edits.insert.push((
            format!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d78{k:02}"),
            copy(&rows[0]),
        ));
    }
    gp.edit(&layer, &edits).expect("written again");
}

fn uid_of(d: &Document, ada: &str) -> String {
    let layer = link_of(d).layer;
    d.by_layer_with_uids(&layer)
        .find(|(_, e)| e.base().attrs.get("ada").map(String::as_str) == Some(ada))
        .map(|(u, _)| u.to_string())
        .expect("the object")
}

/// A copy of QGIS's GeoPackage linked (with `key`), its two polygon parcels pinned by hand.
fn pinned_parcels(tag: &str, key: Option<&str>) -> (Document, PathBuf) {
    let tmp = std::env::temp_dir().join(format!("kentos-pins-{tag}-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("qgis.gpkg"), &tmp).expect("copied");
    let mut d = doc();
    let mut edit = json!({ "kind": "link", "path": tmp.display().to_string(), "table": "parsel" });
    if let Some(k) = key {
        edit["key"] = json!(k);
    }
    ok(&run(&mut d, "cad.sources.edit", CallOp::Execute, json!({ "edit": edit })));
    for (ada, x) in [("Çağlayan", 1.0), ("Işık-Ğüşöç", 2.0)] {
        let uid = uid_of(&d, ada);
        ok(&run(
            &mut d,
            "cad.labels.pin",
            CallOp::Execute,
            json!({ "pins": [{ "uid": uid, "pin": { "at": { "x": x, "y": 2.0 } } }] }),
        ));
    }
    (d, tmp)
}

fn pins_of(d: &Document, ada: &str) -> Value {
    let uid = uid_of(d, ada);
    let slot = d.slot_of(uid.parse().expect("uid")).expect("slot");
    serde_json::to_value(&d.get(slot).expect("object").base().label_pins).expect("pins")
}

fn read_again(d: &mut Document) -> Value {
    let layer = link_of(d).layer;
    run(
        d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "read" }),
    )
}

fn warning_codes(answer: &Value) -> Vec<String> {
    answer["warnings"]
        .as_array()
        .map(|w| w.iter().map(|w| w["code"].as_str().unwrap_or("").to_string()).collect())
        .unwrap_or_default()
}

/// The table written again with other row ids: with a key the pins follow their records to the new rows, and say so.
#[test]
fn pins_follow_their_key_when_the_table_is_written_again() {
    let (mut d, tmp) = pinned_parcels("key", Some("ada"));
    rewrite(&tmp, 0);
    let read = read_again(&mut d);
    let out = ok(&read);
    assert_eq!(out["pins"]["moved"].as_array().map(Vec::len), Some(2), "{read}");
    assert!(out["pins"].get("lost").is_none(), "{read}");
    assert_eq!(warning_codes(&read), ["label_pins_moved"], "{read}");
    assert_eq!(pins_of(&d, "Çağlayan"), json!([{ "at": { "x": 1.0, "y": 2.0 } }]));
    assert_eq!(pins_of(&d, "Işık-Ğüşöç"), json!([{ "at": { "x": 2.0, "y": 2.0 } }]));
    let _ = std::fs::remove_file(&tmp);
}

/// Without a key, the rows written again with the very same geometry take the pins (docs/adr/0578 §2): moved by
/// `geometry`, said once.
#[test]
fn pins_follow_the_same_geometry_of_a_rewritten_table_without_a_key() {
    let (mut d, tmp) = pinned_parcels("nokey", None);
    rewrite(&tmp, 0);
    let read = read_again(&mut d);
    let out = ok(&read);
    assert_eq!(out["pins"]["moved"].as_array().map(Vec::len), Some(2), "{read}");
    assert!(out["pins"]["moved"].as_array().expect("moved").iter().all(|m| m["by"] == "geometry"
        && m.get("key").is_none()), "{read}");
    assert!(out["pins"].get("lost").is_none(), "{read}");
    assert_eq!(warning_codes(&read), ["label_pins_moved"], "{read}");
    assert_eq!(pins_of(&d, "Çağlayan"), json!([{ "at": { "x": 1.0, "y": 2.0 } }]));
    assert_eq!(pins_of(&d, "Işık-Ğüşöç"), json!([{ "at": { "x": 2.0, "y": 2.0 } }]));
    let _ = std::fs::remove_file(&tmp);
}

/// A table written again with the polygons 3 cm east: the nearest feature of the same kind within the tolerance
/// takes the pins (`nearest`, with the distance).
#[test]
fn pins_follow_the_nearest_feature_within_the_tolerance_without_a_key() {
    let (mut d, tmp) = pinned_parcels("near", None);
    rewrite_nudged(&tmp, 0, 0.03125);
    let read = read_again(&mut d);
    let out = ok(&read);
    let moved = out["pins"]["moved"].as_array().expect("moved");
    assert_eq!(moved.len(), 2, "{read}");
    assert!(moved.iter().all(|m| m["by"] == "nearest" && m["distance"] == 0.03125), "{read}");
    assert_eq!(warning_codes(&read), ["label_pins_moved"], "{read}");
    assert_eq!(pins_of(&d, "Çağlayan"), json!([{ "at": { "x": 1.0, "y": 2.0 } }]));
    let _ = std::fs::remove_file(&tmp);
}

/// Half a metre away is no match: the pins are lost, and said (deleted).
#[test]
fn pins_of_rows_a_rewritten_table_lacks_are_said_without_a_key() {
    let (mut d, tmp) = pinned_parcels("far", None);
    rewrite_nudged(&tmp, 0, 0.5);
    let read = read_again(&mut d);
    let out = ok(&read);
    assert_eq!(out["pins"]["lost"].as_array().map(Vec::len), Some(2), "{read}");
    assert_eq!(out["pins"]["lost"][0]["reason"], "deleted", "{read}");
    assert_eq!(warning_codes(&read), ["label_pins_lost"], "{read}");
    assert_eq!(pins_of(&d, "Çağlayan"), json!([]));
    let _ = std::fs::remove_file(&tmp);
}

/// A row written twice has two rows of the very geometry: the pins are never given on a guess (ambiguous); the other
/// parcel's follow.
#[test]
fn pins_are_not_given_to_one_of_two_equal_rows() {
    let (mut d, tmp) = pinned_parcels("twice", None);
    rewrite(&tmp, 1);
    let read = read_again(&mut d);
    let out = ok(&read);
    assert_eq!(out["pins"]["lost"].as_array().map(Vec::len), Some(1), "{read}");
    assert_eq!(out["pins"]["lost"][0]["reason"], "ambiguous", "{read}");
    assert_eq!(out["pins"]["moved"].as_array().map(Vec::len), Some(1), "{read}");
    assert_eq!(pins_of(&d, "Işık-Ğüşöç"), json!([{ "at": { "x": 2.0, "y": 2.0 } }]));
    assert_eq!(pins_of(&d, "Çağlayan"), json!([]));
    let _ = std::fs::remove_file(&tmp);
}

/// A table that repeats the key's value is said; a repeated key value follows no pin.
#[test]
fn a_table_that_repeats_the_key_is_said() {
    let (mut d, tmp) = pinned_parcels("dup", Some("ada"));
    rewrite(&tmp, 1);
    let read = read_again(&mut d);
    let codes = warning_codes(&read);
    assert!(codes.contains(&"key_not_unique".to_string()), "{read}");
    let _ = std::fs::remove_file(&tmp);
}

// docs/adr/0531: a part of a table written with its neighbours outside the area. The bump of parcel A moves; B (its
// shared edges) and C (a triangle on the bump) lie outside the area and follow in the same transaction. With
// KENTOS_WRITE_GPKG the written copy is kept as fixtures/geopackage/v1/kentos-komsu.gpkg, which
// scripts/fixtures/bound_sources_cases.py reads with GDAL's OGR.
#[test]
fn a_part_writes_its_neighbours_outside_the_area() {
    let tmp =
        std::env::temp_dir().join(format!("kentos-sources-komsu-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("komsu.gpkg"), &tmp).expect("copied");
    let path = tmp.display().to_string();
    let mut d = doc();
    let linked = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": path, "table": "parsel",
                          "extent": [486990.0, 4419990.0, 487009.0, 4420030.0] } }),
    );
    assert_eq!(ok(&linked)["objects"], 1);
    let layer = link_of(&d).layer;
    let a = d
        .by_layer_with_uids(&layer)
        .next()
        .map(|(u, _)| u.to_string())
        .expect("A");
    ok(&run(
        &mut d,
        "cad.entities.stretch",
        CallOp::Execute,
        json!({ "window": { "min": { "x": 487011.5, "y": 4420004.5 }, "max": { "x": 487012.5, "y": 4420005.5 } },
                "dx": 1.0, "dy": 0.0, "uids": [a] }),
    ));
    let input = json!({ "layer": layer, "direction": "write", "topology": "strict" });
    let untouched = std::fs::read(&tmp).expect("read");
    let plan = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        input.clone(),
    ))
    .clone();
    assert_eq!(
        (plan["modified"].clone(), plan["followed"].clone()),
        (json!(1), json!(2)),
        "{plan}"
    );
    assert_eq!(
        std::fs::read(&tmp).expect("read"),
        untouched,
        "a plan writes nothing"
    );
    let written = ok(&run(&mut d, "cad.sources.sync", CallOp::Execute, input)).clone();
    assert_eq!(
        (written["modified"].clone(), written["followed"].clone()),
        (json!(1), json!(2)),
        "{written}"
    );
    let after = state(&mut d);
    assert_eq!(
        (after["edited"].clone(), after["found"].clone()),
        (json!(false), json!(true)),
        "{after}"
    );
    // The whole table read back: every parcel has the bump where A put it, none where it was.
    let mut whole = doc();
    ok(&run(
        &mut whole,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": path, "table": "parsel" } }),
    ));
    let wl = link_of(&whole).layer;
    let mut seen = 0;
    for (_, e) in whole.by_layer_with_uids(&wl) {
        let v = serde_json::to_value(e).expect("entity");
        let pts = v["pts"].as_array().expect("ring");
        let has = |x: f64| {
            pts.iter()
                .any(|p| p["x"] == json!(x) && p["y"] == json!(4420005.0))
        };
        assert!(has(487013.0) && !has(487012.0), "{v}");
        seen += 1;
    }
    assert_eq!(seen, 3);
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&tmp, dir().join("kentos-komsu.gpkg")).expect("copied");
    }
    let _ = std::fs::remove_file(&tmp);
}

// docs/adr/0578 §3: an object that leaves the part (moved out by hand and written) has its row outside the area on the
// next read; its pins are parked in the binding, listed by `cad.labels.report`, and given back to the row when the area
// takes it again.
#[test]
fn pins_of_an_object_that_left_the_part_are_parked_and_given_back() {
    let tmp = std::env::temp_dir().join(format!("kentos-park-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("qgis.gpkg"), &tmp).expect("copied");
    let mut d = doc();
    ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": tmp.display().to_string(), "table": "parsel",
                          "extent": [486990.0, 4419990.0, 487050.0, 4420030.0] } }),
    ));
    let link = link_of(&d);
    let uid = d
        .by_layer_with_uids(&link.layer)
        .next()
        .map(|(u, _)| u.to_string())
        .expect("the parcel in the area");
    ok(&run(
        &mut d,
        "cad.labels.pin",
        CallOp::Execute,
        json!({ "pins": [{ "uid": uid, "pin": { "at": { "x": 1.0, "y": 2.0 } } }] }),
    ));
    ok(&run(
        &mut d,
        "cad.entities.transform",
        CallOp::Execute,
        json!({ "uids": [uid], "transform": { "kind": "move", "dx": 500.0, "dy": 0.0 } }),
    ));
    ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": link.layer, "direction": "write" }),
    ));
    // The table's row is outside the area now: the read takes the object out and parks its pins.
    let read = read_again(&mut d);
    let out = ok(&read);
    assert_eq!(out["pins"]["parked"], json!([uid]), "{read}");
    assert!(out["pins"].get("lost").is_none(), "{read}");
    assert_eq!(warning_codes(&read), ["label_pins_parked"], "{read}");
    let parked = &link_of(&d).parked;
    assert_eq!(parked.len(), 1);
    assert_eq!((parked[0].uid.as_str(), parked[0].pins.len()), (uid.as_str(), 1));
    // The report lists them.
    let report = run(
        &mut d,
        "cad.labels.report",
        CallOp::Execute,
        json!({ "pxPerMetre": 4.0, "window": [486000.0, 4419000.0, 488000.0, 4421000.0] }),
    );
    let layer = &ok(&report)["layers"][0];
    assert_eq!(layer["parked"], json!([{ "uid": uid, "pins": 1 }]), "{report}");
    assert_eq!(layer["neighbours"], 0, "the band holds no other row: {report}");
    // The area takes every row: the object comes back with its pins, the binding parks nothing.
    let widened = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "extent", "id": link.id } }),
    );
    let out = ok(&widened);
    assert_eq!(out["pins"]["restored"], json!([uid]), "{widened}");
    assert_eq!(warning_codes(&widened), ["label_pins_restored"], "{widened}");
    assert!(link_of(&d).parked.is_empty());
    assert_eq!(pins_of(&d, "Çağlayan"), json!([{ "at": { "x": 1.0, "y": 2.0 } }]));
    let _ = std::fs::remove_file(&tmp);
}

/// Two halves of `uid`'s bounding box, as polygons.
fn halves(d: &Document, uid: &str) -> (Value, Value) {
    let (_, e) = d
        .by_layer_with_uids(&link_of(d).layer)
        .find(|(u, _)| u.to_string() == uid)
        .expect("object");
    let v = serde_json::to_value(e).expect("json");
    let pts = v["pts"].as_array().expect("pts");
    let xs: Vec<f64> = pts.iter().map(|p| p["x"].as_f64().expect("x")).collect();
    let ys: Vec<f64> = pts.iter().map(|p| p["y"].as_f64().expect("y")).collect();
    let (x0, x1) = (xs.iter().copied().fold(f64::MAX, f64::min), xs.iter().copied().fold(f64::MIN, f64::max));
    let (y0, y1) = (ys.iter().copied().fold(f64::MAX, f64::min), ys.iter().copied().fold(f64::MIN, f64::max));
    let xm = (x0 + x1) / 2.0;
    let rect = |a: f64, b: f64| {
        json!({ "kind": "polygon", "pts": [{ "x": a, "y": y0 }, { "x": b, "y": y0 }, { "x": b, "y": y1 }, { "x": a, "y": y1 }] })
    };
    (rect(x0, xm), rect(xm, x1))
}

fn lineage_in(path: &std::path::Path) -> Vec<(String, String, String, String)> {
    kentos_geopackage::GeoPackage::open(path, false)
        .expect("open")
        .lineage_of("parsel")
        .expect("lineage")
}

// docs/adr/0598: a parcel split in a bound layer writes its new piece, the piece's lasting id and its parent in the
// one SQLite transaction; undo takes the record back with the piece and the next write takes the rows out; a merge's
// object names the parcels merged into it. With KENTOS_WRITE_GPKG the written copy is kept as
// fixtures/geopackage/v1/kentos-soy.gpkg, which scripts/fixtures/lineage_cases.py reads with sqlite3.
#[test]
fn a_split_parcel_writes_its_parent_beside_its_rows() {
    let tmp = std::env::temp_dir().join(format!("kentos-lineage-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("qgis.gpkg"), &tmp).expect("copied");
    let path = tmp.display().to_string();
    let mut d = doc();
    ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": path, "table": "parsel" } }),
    ));
    let layer = link_of(&d).layer;
    let uids: Vec<String> = d.by_layer_with_uids(&layer).map(|(u, _)| u.to_string()).collect();
    let write = json!({ "layer": layer, "direction": "write" });
    let (a, b) = halves(&d, &uids[0]);
    let split = ok(&run(
        &mut d,
        "cad.entities.edit",
        CallOp::Execute,
        json!({ "operation": "areaSplit", "changes": [
            { "kind": "replace", "uid": uids[0], "geometry": a, "keepData": true },
            { "kind": "add", "from": uids[0], "geometry": b, "keepData": true } ] }),
    ))
    .clone();
    let piece = split["created"][0].as_str().expect("piece").to_owned();
    assert_eq!(
        serde_json::to_value(d.lineage()).expect("json"),
        json!([{ "child": piece, "parents": [uids[0]], "operation": "split", "step": "Alan böl" }])
    );
    ok(&run(&mut d, "cad.sources.sync", CallOp::Execute, write.clone()));
    assert_eq!(
        lineage_in(&tmp),
        vec![(piece.clone(), uids[0].clone(), "split".into(), "Alan böl".into())]
    );
    // Undo is the drawing's: the record goes with the piece; the next write takes the piece's row and its parent out.
    assert_eq!(d.undo().as_deref(), Some("Alan böl"));
    assert!(d.lineage().is_empty());
    ok(&run(&mut d, "cad.sources.sync", CallOp::Execute, write.clone()));
    assert!(lineage_in(&tmp).is_empty(), "{:?}", lineage_in(&tmp));
    assert_eq!(d.redo().as_deref(), Some("Alan böl"));
    ok(&run(&mut d, "cad.sources.sync", CallOp::Execute, write.clone()));
    assert_eq!(lineage_in(&tmp).len(), 1);
    // A merge of the piece and another parcel: the new object names both, in the step's order.
    let (whole, _) = halves(&d, &uids[1]);
    let merged = ok(&run(
        &mut d,
        "cad.entities.edit",
        CallOp::Execute,
        json!({ "operation": "areaUnion", "changes": [
            { "kind": "remove", "uid": piece }, { "kind": "remove", "uid": uids[1] },
            { "kind": "add", "from": piece, "geometry": whole, "keepData": true } ] }),
    ))
    .clone();
    let union = merged["created"][0].as_str().expect("union").to_owned();
    ok(&run(&mut d, "cad.sources.sync", CallOp::Execute, write));
    let mut want = vec![
        (union.clone(), piece.clone(), "merge".to_owned(), "Alan birleştir".to_owned()),
        (union, uids[1].clone(), "merge".into(), "Alan birleştir".into()),
    ];
    // The piece is gone from the table; its own parent stays: it was split, then merged away.
    want.push((piece, uids[0].clone(), "split".into(), "Alan böl".into()));
    want.sort();
    assert_eq!(lineage_in(&tmp), want);
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&tmp, dir().join("kentos-soy.gpkg")).expect("kept");
    }
    let _ = std::fs::remove_file(&tmp);
}

/// A record changed in the table touches only its object when read (docs/adr/0635 §10): the drawing's journal says one
/// object changed, so the style scene builds that object's part again and leaves the rest (docs/adr/0121).
#[test]
fn a_record_changed_in_its_table_touches_only_its_object() {
    let tmp = std::env::temp_dir().join(format!("kentos-one-row-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("qgis.gpkg"), &tmp).expect("copied");
    let mut d = doc();
    let linked = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": tmp.display().to_string(), "table": "parsel" } }),
    );
    assert_eq!(ok(&linked)["objects"], 3);
    let layer = link_of(&d).layer;
    {
        let mut gp = kentos_geopackage::GeoPackage::open(&tmp, true).expect("opens");
        let table = gp
            .layers()
            .expect("layers")
            .into_iter()
            .find(|l| l.name == "parsel")
            .expect("parsel");
        let row = gp
            .rows(&table, &kentos_geopackage::Query::default())
            .expect("rows")
            .into_iter()
            .next()
            .expect("a row");
        let at = table
            .columns
            .iter()
            .position(|c| c.name == "ada")
            .expect("ada");
        let mut values = row.values.clone();
        values[at] = kentos_geopackage::Value::Text("Değişen".into());
        gp.edit(
            &table,
            &kentos_geopackage::Edits {
                update: vec![(
                    row.fid,
                    kentos_geopackage::NewRow {
                        fid: Some(row.fid),
                        geometry: row.geometry.and_then(Result::ok),
                        values,
                    },
                )],
                ..Default::default()
            },
        )
        .expect("edited");
    }
    let mark = d.change_mark();
    let read = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "read" }),
    ))
    .clone();
    assert_eq!(read["modified"], 1, "{read}");
    let touched: std::collections::BTreeSet<_> = match d.changes_since(mark) {
        kentos_domain::Changes::Slots(s) => s.iter().copied().collect(),
        kentos_domain::Changes::All => panic!("the whole drawing read again"),
    };
    assert_eq!(touched.len(), 1, "{touched:?}");
    let _ = std::fs::remove_file(&tmp);
}
