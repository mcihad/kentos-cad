//! The vector tile reader (docs/adr/0208 §9) against GDAL's in
//! `fixtures/services/v1/mvt.json` (`scripts/fixtures/service_mvt_cases.py`:
//! a tile GDAL's MVT driver wrote, read by GDAL's own reader, no KentOS
//! code): the same layers, extents, ids, kinds and values, and the same
//! vertices in tile coordinates, a ring's closing vertex aside.

use kentos_services::mvt::{self, GeomType, Geometry, Value};
use serde_json::Value as Json;

fn fixture() -> (Json, Vec<u8>) {
    let root = format!(
        "{}/../../../fixtures/services/v1",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Json = serde_json::from_str(
        &std::fs::read_to_string(format!("{root}/mvt.json")).expect("fixture file"),
    )
    .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.service-mvt");
    assert_eq!(file["version"], 1);
    let tile = std::fs::read(format!("{root}/mvt/kizilay.pbf")).expect("the tile");
    (file, tile)
}

/// A ring or line as GDAL gives it, its closing vertex dropped when `closed`.
fn path(v: &Json, closed: bool) -> Vec<[i32; 2]> {
    let mut out: Vec<[i32; 2]> = v
        .as_array()
        .expect("vertices")
        .iter()
        .map(|p| {
            let x = p[0].as_f64().expect("x");
            let y = p[1].as_f64().expect("y");
            assert!(
                x.fract() == 0.0 && y.fract() == 0.0,
                "a tile's vertices are whole numbers: {x}, {y}"
            );
            [x as i32, y as i32]
        })
        .collect();
    if closed && out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

fn open(r: &[[i32; 2]]) -> Vec<[i32; 2]> {
    let mut out = r.to_vec();
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

#[test]
fn a_tile_reads_as_gdal_reads_it() {
    let (file, bytes) = fixture();
    let mut layers = mvt::read(&bytes).expect("the tile reads");
    layers.sort_by(|a, b| a.name.cmp(&b.name));
    let want = file["layers"].as_array().expect("layers");
    assert_eq!(
        layers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
        want.iter()
            .map(|l| l["name"].as_str().expect("a name"))
            .collect::<Vec<_>>()
    );
    for (layer, w) in layers.iter().zip(want) {
        assert_eq!(
            layer.extent,
            file["extent"].as_u64().expect("an extent") as u32,
            "{}",
            layer.name
        );
        let wf = w["features"].as_array().expect("features");
        assert_eq!(layer.features.len(), wf.len(), "{}: features", layer.name);
        for (f, w) in layer.features.iter().zip(wf) {
            let name = format!("{} #{:?}", layer.name, f.id);
            assert_eq!(f.id, w["id"].as_u64(), "{name}: id");
            // Its values, by key: text, numbers (as decimals) and yes or no.
            let props = w["properties"].as_object().expect("values");
            assert_eq!(f.tags.len(), props.len(), "{name}: values");
            for (k, v) in props {
                let got = layer
                    .get(f, k)
                    .unwrap_or_else(|| panic!("{name}: {k} missing"));
                match (got, v) {
                    (Value::Text(a), Json::String(b)) => assert_eq!(a, b, "{name}: {k}"),
                    (Value::Number(a), Json::Number(b)) => {
                        assert_eq!(*a, b.as_f64().expect("a number"), "{name}: {k}")
                    }
                    (Value::Bool(a), Json::Bool(b)) => assert_eq!(a, b, "{name}: {k}"),
                    other => panic!("{name}: {k}: {other:?}"),
                }
            }
            let g = &w["geometry"];
            match (w["kind"].as_str().expect("a kind"), &f.geometry) {
                ("point", Geometry::Points(points)) => {
                    assert_eq!(f.kind, GeomType::Point);
                    assert_eq!(points, &path(g, false), "{name}: points");
                }
                ("line", Geometry::Lines(lines)) => {
                    assert_eq!(f.kind, GeomType::Line);
                    let want: Vec<Vec<[i32; 2]>> = g
                        .as_array()
                        .expect("lines")
                        .iter()
                        .map(|l| path(l, false))
                        .collect();
                    assert_eq!(lines, &want, "{name}: lines");
                }
                ("polygon", Geometry::Polygons(polygons)) => {
                    assert_eq!(f.kind, GeomType::Polygon);
                    let want: Vec<Vec<Vec<[i32; 2]>>> = g
                        .as_array()
                        .expect("polygons")
                        .iter()
                        .map(|p| {
                            p.as_array()
                                .expect("rings")
                                .iter()
                                .map(|r| path(r, true))
                                .collect()
                        })
                        .collect();
                    let got: Vec<Vec<Vec<[i32; 2]>>> = polygons
                        .iter()
                        .map(|p| p.iter().map(|r| open(r)).collect())
                        .collect();
                    assert_eq!(got, want, "{name}: polygons");
                }
                (kind, other) => panic!("{name}: {kind} read as {other:?}"),
            }
        }
    }
}
