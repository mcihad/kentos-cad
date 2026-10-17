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
