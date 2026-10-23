//! The GeoPackage crate against fixtures/geopackage/v1 (docs/adr/0502): `qgis.gpkg`, written by QGIS 4.2.3, must read
//! as GDAL's OGR reads it (`expected.json`, scripts/fixtures/geopackage_cases.py): layers, geometry types with Z and M,
//! fid columns, columns, fids, values and geometry (as ISO WKT). Every layer written again must read back the same;
//! with KENTOS_WRITE_GPKG=1 it is written to `kentos.gpkg`, which the script reads with OGR and GDAL's validator.

use std::path::{Path, PathBuf};

use kentos_geopackage::{
    ColumnKind, Coord, Curve, GeoPackage, Geometry, Layer, NewRow, Query, TableDef, Value,
};
use serde_json::{Value as J, json};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/geopackage/v1")
}

fn num(v: f64) -> String {
    format!("{v}")
}

fn coord(c: &Coord) -> String {
    let mut s = format!("{} {}", num(c.x), num(c.y));
    if let Some(z) = c.z {
        s.push_str(&format!(" {}", num(z)));
    }
    if let Some(m) = c.m {
        s.push_str(&format!(" {}", num(m)));
    }
    s
}

fn list(p: &[Coord]) -> String {
    format!("({})", p.iter().map(coord).collect::<Vec<_>>().join(","))
}

fn curve_body(c: &Curve, in_compound: bool) -> String {
    match c {
        Curve::Line(p) if in_compound => list(p),
        Curve::Line(p) => list(p),
        Curve::Arc(p) => format!("CIRCULARSTRING {}", list(p)),
        Curve::Compound(parts) => format!(
            "COMPOUNDCURVE ({})",
            parts
                .iter()
                .map(|c| curve_body(c, true))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn dims(g: &Geometry) -> &'static str {
    match g.dims() {
        (false, false) => "",
        (true, false) => " Z",
        (false, true) => " M",
        (true, true) => " ZM",
    }
}

fn rings(rs: &[Curve]) -> String {
    format!(
        "({})",
        rs.iter()
            .map(|c| curve_body(c, false))
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// ISO WKT as OGR's `ExportToIsoWkt` writes it.
fn wkt(g: &Geometry, declared: &str) -> String {
    let d = dims(g);
    let empty = |name: &str| format!("{name}{d} EMPTY");
    match g {
        Geometry::Point(None) => empty("POINT"),
        Geometry::Point(Some(c)) => format!("POINT{d} ({})", coord(c)),
        Geometry::Curve(Curve::Line(p)) => format!("LINESTRING{d} {}", list(p)),
        Geometry::Curve(c) => {
            let body = curve_body(c, false);
            match body.split_once(' ') {
                Some((name, rest)) => format!("{name}{d} {rest}"),
                None => body,
            }
        }
        Geometry::Polygon(rs) if rs.is_empty() => empty(declared),
        Geometry::Polygon(rs) => format!("{}{d} {}", g.type_name(), rings(rs)),
        Geometry::MultiPoint(p) if p.is_empty() => empty("MULTIPOINT"),
        Geometry::MultiPoint(p) => format!(
            "MULTIPOINT{d} ({})",
            p.iter()
                .map(|c| format!("({})", coord(c)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Geometry::MultiCurve(cs) if cs.is_empty() => empty(declared),
        Geometry::MultiCurve(cs) => format!("{}{d} {}", g.type_name(), rings(cs)),
        Geometry::MultiPolygon(ps) if ps.is_empty() => empty(declared),
        Geometry::MultiPolygon(ps) => format!(
            "{}{d} ({})",
            g.type_name(),
            ps.iter().map(|r| rings(r)).collect::<Vec<_>>().join(",")
        ),
        Geometry::Collection(_) => "GEOMETRYCOLLECTION".into(),
    }
}

fn ogr_type(layer: &Layer) -> Option<String> {
    let g = layer.geometry.as_ref()?;
    let base = match g.type_name.to_ascii_uppercase().as_str() {
        "POINT" => "Point",
        "LINESTRING" => "Line String",
        "POLYGON" => "Polygon",
        "MULTIPOINT" => "Multi Point",
        "MULTILINESTRING" => "Multi Line String",
        "MULTIPOLYGON" => "Multi Polygon",
        "CIRCULARSTRING" => "Circular String",
        "COMPOUNDCURVE" => "Compound Curve",
        "CURVEPOLYGON" => "Curve Polygon",
        "MULTICURVE" => "Multi Curve",
        "MULTISURFACE" => "Multi Surface",
        _ => "Unknown (any)",
    };
    let mut s = String::new();
    if g.z == 1 {
        s.push_str("3D ");
    }
    if g.m == 1 {
        s.push_str("Measured ");
    }
    s.push_str(base);
    Some(s)
}

fn ogr_kind(decl: &str, kind: &ColumnKind) -> &'static str {
    match kind {
        ColumnKind::Integer => match decl.trim().to_ascii_uppercase().as_str() {
            "INTEGER" | "INT" => "Integer64",
            _ => "Integer",
        },
        ColumnKind::Boolean => "Boolean",
        ColumnKind::Real => "Real",
        ColumnKind::Date => "Date",
        ColumnKind::DateTime => "DateTime",
        ColumnKind::Blob(_) => "Binary",
        _ => "String",
    }
}

fn value(v: &Value, ogr: &str) -> J {
    match (v, ogr) {
        (Value::Null, _) => J::Null,
        (Value::Integer(i), "Integer64") => json!(i.to_string()),
        (Value::Integer(i), _) => json!(i),
        (Value::Real(r), _) => json!(r),
        (Value::Text(t), _) => json!(t),
        (Value::Blob(_), _) => json!("<blob>"),
    }
}

/// Every layer of the file as the reference writes it.
fn read_all(path: &Path) -> J {
    let gp = GeoPackage::open(path, false).expect("opens");
    let mut out = Vec::new();
    for layer in gp.layers().expect("layers") {
        let kinds: Vec<&str> = layer
            .columns
            .iter()
            .map(|c| ogr_kind(&c.decl, &c.kind))
            .collect();
        let declared = layer
            .geometry
            .as_ref()
            .map(|g| g.type_name.clone())
            .unwrap_or_default();
        let rows: Vec<J> = gp
            .rows(&layer, &Query::default())
            .expect("rows")
            .iter()
            .map(|r| {
                let mut values = serde_json::Map::new();
                for ((c, v), k) in layer.columns.iter().zip(&r.values).zip(&kinds) {
                    values.insert(c.name.clone(), value(v, k));
                }
                json!({
                    "fid": r.fid,
                    "wkt": r.geometry.as_ref().map(|g| wkt(g.as_ref().expect("a geometry"), &declared)),
                    "values": values,
                })
            })
            .collect();
        out.push(json!({
            "name": layer.name,
            "geometryColumn": layer.geometry.as_ref().map(|g| g.name.clone()),
            "geometryType": ogr_type(&layer),
            "srid": layer.srs_id.filter(|&s| s > 0),
            "fid": layer.fid,
            "columns": layer.columns.iter().zip(&kinds).map(|(c, k)| json!({"name": c.name, "kind": k})).collect::<Vec<_>>(),
            "rows": rows,
        }));
    }
    J::Array(out)
}

fn expected() -> J {
    let text = std::fs::read_to_string(dir().join("expected.json")).expect("expected.json");
    let doc: J = serde_json::from_str(&text).expect("JSON");
    assert_eq!(doc["format"], "kentos.geopackage-cases");
    doc["layers"].clone()
}

#[test]
fn a_geopackage_qgis_wrote_reads_as_gdal_reads_it() {
    let got = read_all(&dir().join("qgis.gpkg"));
    let want = expected();
    for (g, w) in got
        .as_array()
        .expect("layers")
        .iter()
        .zip(want.as_array().expect("layers"))
    {
        assert_eq!(g, w, "{}", w["name"]);
    }
    assert_eq!(got.as_array().map(Vec::len), want.as_array().map(Vec::len));
}

/// Every layer of `from` written to `to`, with the parcels' lasting ids.
fn copy(from: &Path, to: &Path) -> Vec<String> {
    let src = GeoPackage::open(from, false).expect("opens");
    let mut out = GeoPackage::create(to).expect("creates");
    let mut ids = Vec::new();
    for layer in src.layers().expect("layers") {
        if let Some(id) = layer.srs_id
            && let Some(srs) = src.srs(id).expect("srs")
        {
            out.add_srs(&srs).expect("srs written");
        }
        let rows: Vec<NewRow> = src
            .rows(&layer, &Query::default())
            .expect("rows")
            .into_iter()
            .map(|r| NewRow {
                fid: Some(r.fid),
                geometry: r.geometry.map(|g| g.expect("a geometry")),
                values: r.values,
            })
            .collect();
        let uids: Vec<String> = (0..rows.len())
            .map(|i| format!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{:04}", 7000 + i))
            .collect();
        let def = TableDef {
            name: layer.name.clone(),
            data_type: layer.data_type.clone(),
            identifier: layer.identifier.clone(),
            description: layer.description.clone(),
            srs_id: layer.srs_id,
            geometry: layer.geometry.clone(),
            columns: layer.columns.clone(),
        };
        let fids = out
            .write_table(&def, &rows, (layer.name == "parsel").then_some(&uids[..]))
            .expect("written");
        assert_eq!(
            fids,
            rows.iter().map(|r| r.fid.expect("fid")).collect::<Vec<_>>()
        );
        if layer.name == "parsel" {
            ids = uids;
        }
    }
    out.close().expect("closed");
    ids
}

#[test]
fn every_layer_written_again_reads_back_the_same() {
    let tmp = std::env::temp_dir().join(format!("kentos-gpkg-{}.gpkg", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let ids = copy(&dir().join("qgis.gpkg"), &tmp);
    assert_eq!(read_all(&tmp), expected());
    let gp = GeoPackage::open(&tmp, false).expect("opens");
    let uids = gp.uids("parsel").expect("uids");
    assert_eq!(uids.get(&1), ids.first());
    let parsel = gp
        .layers()
        .expect("layers")
        .into_iter()
        .find(|l| l.name == "parsel")
        .expect("parsel");
    assert!(parsel.indexed);
    // The index answers a box: only the second parcel is east of 487050.
    let east = gp
        .rows(
            &parsel,
            &Query {
                bbox: Some([487050.0, 4419990.0, 487200.0, 4420100.0]),
                ..Query::default()
            },
        )
        .expect("rows");
    assert_eq!(east.iter().map(|r| r.fid).collect::<Vec<_>>(), vec![2]);
    // A page after fid 1, at most one row.
    let page = gp
        .rows(
            &parsel,
            &Query {
                after: Some(1),
                limit: Some(1),
                ..Query::default()
            },
        )
        .expect("rows");
    assert_eq!(page.iter().map(|r| r.fid).collect::<Vec<_>>(), vec![2]);
    drop(gp);
    let _ = std::fs::remove_file(&tmp);
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        let out = dir().join("kentos.gpkg");
        let _ = std::fs::remove_file(&out);
        copy(&dir().join("qgis.gpkg"), &out);
    }
}

#[test]
fn a_file_that_is_not_a_geopackage_is_said() {
    let tmp = std::env::temp_dir().join(format!("kentos-not-gpkg-{}.gpkg", std::process::id()));
    std::fs::write(&tmp, b"not a database at all").expect("written");
    let e = GeoPackage::open(&tmp, false).err().expect("refused");
    assert!(e.0.contains("SQLite") || e.0.contains("GeoPackage"), "{e}");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn a_table_is_edited_in_place_and_its_index_follows() {
    use kentos_geopackage::{Column, Edits};
    let tmp = std::env::temp_dir().join(format!("kentos-gpkg-edit-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("qgis.gpkg"), &tmp).expect("copied");
    let mut gp = GeoPackage::open(&tmp, true).expect("opens");
    let parsel = gp
        .layers()
        .expect("layers")
        .into_iter()
        .find(|l| l.name == "parsel")
        .expect("parsel");
    let rows = gp.rows(&parsel, &Query::default()).expect("rows");
    let second = &rows[1];
    let mut values = second.values.clone();
    values[0] = Value::Text("Yeni Mahalle".into());
    values.push(Value::Text("onaylı".into()));
    let square = |x: f64| {
        Geometry::MultiPolygon(vec![vec![Curve::Line(vec![
            Coord::xy(x, 4420000.0),
            Coord::xy(x + 5.0, 4420000.0),
            Coord::xy(x + 5.0, 4420005.0),
            Coord::xy(x, 4420000.0),
        ])]])
    };
    let mut fresh = vec![Value::Null; parsel.columns.len() + 1];
    fresh[0] = Value::Text("eklenen".into());
    let edits = Edits {
        add_columns: vec![Column {
            name: "durum".into(),
            decl: "TEXT".into(),
            kind: ColumnKind::Text(None),
            not_null: false,
            default: None,
        }],
        update: vec![(
            2,
            NewRow {
                fid: None,
                geometry: Some(square(487300.0)),
                values,
            },
        )],
        insert: vec![(
            "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7999".into(),
            NewRow {
                fid: None,
                geometry: Some(square(487500.0)),
                values: fresh,
            },
        )],
        delete: vec![1],
        lineage: Vec::new(),
    };
    let fids = gp.edit(&parsel, &edits).expect("edited");
    assert_eq!(fids, vec![4]);
    drop(gp);
    let gp = GeoPackage::open(&tmp, false).expect("opens");
    let parsel = gp
        .layers()
        .expect("layers")
        .into_iter()
        .find(|l| l.name == "parsel")
        .expect("parsel");
    assert_eq!(
        parsel.columns.last().map(|c| c.name.as_str()),
        Some("durum")
    );
    let rows = gp.rows(&parsel, &Query::default()).expect("rows");
    assert_eq!(
        rows.iter().map(|r| r.fid).collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
    assert_eq!(rows[0].values[0], Value::Text("Yeni Mahalle".into()));
    assert_eq!(
        gp.uids("parsel").expect("uids").get(&4).map(String::as_str),
        Some("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7999")
    );
    // GDAL's own triggers moved the index: the edited parcel is found at its new place, the deleted one nowhere.
    let at = |b: [f64; 4]| {
        gp.rows(
            &parsel,
            &Query {
                bbox: Some(b),
                ..Query::default()
            },
        )
        .expect("rows")
        .iter()
        .map(|r| r.fid)
        .collect::<Vec<_>>()
    };
    assert_eq!(at([487299.0, 4419999.0, 487306.0, 4420006.0]), vec![2]);
    assert_eq!(
        at([486999.0, 4419999.0, 487021.0, 4420021.0]),
        Vec::<i64>::new()
    );
    assert_eq!(at([487499.0, 4419999.0, 487506.0, 4420006.0]), vec![4]);
    drop(gp);
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&tmp, dir().join("kentos-edited.gpkg")).expect("copied");
    }
    let _ = std::fs::remove_file(&tmp);
}

/// A GeoPackage's tables as `layers()` names them.
fn names(path: &Path) -> Vec<String> {
    GeoPackage::open(path, false)
        .expect("opens")
        .layers()
        .expect("layers")
        .into_iter()
        .map(|l| l.name)
        .collect()
}

fn one_square(x: f64) -> NewRow {
    NewRow {
        fid: None,
        geometry: Some(Geometry::Polygon(vec![Curve::Line(vec![
            Coord::xy(x, 4420000.0),
            Coord::xy(x + 5.0, 4420000.0),
            Coord::xy(x + 5.0, 4420005.0),
            Coord::xy(x, 4420000.0),
        ])])),
        values: vec![Value::Text("a".into())],
    }
}

fn square_table(name: &str, srs: Option<i32>) -> TableDef {
    TableDef {
        name: name.into(),
        data_type: kentos_geopackage::DataType::Features,
        identifier: Some(name.into()),
        description: None,
        srs_id: srs,
        geometry: Some(kentos_geopackage::GeometryColumn {
            name: "geom".into(),
            type_name: "POLYGON".into(),
            z: 0,
            m: 0,
        }),
        columns: vec![kentos_geopackage::Column {
            name: "ad".into(),
            decl: String::new(),
            kind: ColumnKind::Text(None),
            not_null: false,
            default: None,
        }],
    }
}

#[test]
fn a_session_adds_and_replaces_tables_in_one_transaction_and_a_dropped_one_leaves_nothing() {
    use kentos_geopackage::Taken;
    let tmp = std::env::temp_dir().join(format!("kentos-gpkg-session-{}.gpkg", std::process::id()));
    std::fs::copy(dir().join("qgis.gpkg"), &tmp).expect("copied");
    let before = names(&tmp);
    assert!(before.contains(&"parsel".to_owned()));
    let mut gp = GeoPackage::open(&tmp, true).expect("opens");
    let srs = gp
        .layers()
        .expect("layers")
        .into_iter()
        .find(|l| l.name == "parsel")
        .and_then(|l| l.srs_id);
    // Rolled back: nothing of it is in the file.
    {
        let s = gp.session().expect("a session");
        assert_eq!(s.taken("PARSEL").expect("taken"), Taken::Data(kentos_geopackage::DataType::Features));
        assert_eq!(s.taken("yeni").expect("taken"), Taken::Free);
        assert_eq!(s.taken("gpkg_contents").expect("taken"), Taken::Other);
        s.drop_table("parsel").expect("dropped");
        let mut w = s.start_table(&square_table("yeni", srs)).expect("started");
        s.append(&mut w, &[one_square(1.0)], None).expect("appended");
        s.finish_table(w).expect("finished");
        s.rollback().expect("rolled back");
    }
    assert_eq!(names(&tmp), before);
    // Committed: the parcel table replaced by a new one of the same name in two batches, another added beside it.
    let s = gp.session().expect("a session");
    s.drop_table("parsel").expect("dropped");
    let mut w = s.start_table(&square_table("parsel", srs)).expect("started");
    let a = s
        .append(&mut w, &[one_square(1.0), one_square(10.0)], Some(&["u1".into(), "u2".into()]))
        .expect("batch one");
    let b = s
        .append(&mut w, &[one_square(20.0)], Some(&["u3".into()]))
        .expect("batch two");
    assert_eq!((a, b), (vec![1, 2], vec![3]));
    assert_eq!(w.rows(), 3);
    s.finish_table(w).expect("finished");
    s.add_metadata("parsel", "https://example.org/x", "text/plain", "x", "2026-10-20T00:00:00.000Z")
        .expect("metadata");
    s.commit().expect("committed");
    drop(gp);
    let gp = GeoPackage::open(&tmp, false).expect("opens");
    let layers = gp.layers().expect("layers");
    let parsel = layers.iter().find(|l| l.name == "parsel").expect("parsel");
    assert_eq!(gp.count(parsel).expect("count"), 3);
    assert_eq!(parsel.columns.len(), 1);
    assert_eq!(
        gp.uids("parsel").expect("uids").values().cloned().collect::<std::collections::BTreeSet<_>>(),
        ["u1", "u2", "u3"].map(String::from).into()
    );
    // The index is the new one's, its three boxes; the other tables are as they were.
    let found = gp
        .rows(parsel, &Query { bbox: Some([0.0, 4419999.0, 30.0, 4420010.0]), ..Query::default() })
        .expect("rows");
    assert_eq!(found.len(), 3);
    let mut again = before.clone();
    again.sort();
    let mut now = names(&tmp);
    now.sort();
    assert_eq!(now, again);
    // The old table's extension rows went with it: exactly one rtree row for the new one.
    let rtree = gp
        .extensions()
        .expect("extensions")
        .into_iter()
        .filter(|e| e.table.as_deref() == Some("parsel") && e.name == "gpkg_rtree_index")
        .count();
    assert_eq!(rtree, 1);
    let _ = std::fs::remove_file(&tmp);
}
