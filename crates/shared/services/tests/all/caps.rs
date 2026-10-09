//! The capabilities readers (docs/adr/0208 §5, §6, §10) against OWSLib in
//! `fixtures/services/v1/caps.json` (`scripts/fixtures/service_caps_cases.py`:
//! OWSLib and PROJ's axis orders, no KentOS code): the same addresses,
//! formats, layers, styles and systems; a WMTS matrix's pixel within 1e-12
//! of itself, corners and boxes within 1e-9.

use kentos_services::caps::{wfs, wms, wmts};
use serde_json::Value;

fn root() -> String {
    format!(
        "{}/../../../fixtures/services/v1",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn fixture() -> Value {
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{}/caps.json", root())).expect("fixture file"),
    )
    .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.service-caps");
    assert_eq!(file["version"], 1);
    file
}

fn doc(name: &str) -> String {
    std::fs::read_to_string(format!("{}/caps/{name}", root())).expect("a document")
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("a list")
        .iter()
        .map(|s| s.as_str().expect("text").to_owned())
        .collect()
}

fn near(a: [f64; 4], b: &Value, what: &str) {
    for (i, w) in b.as_array().expect("a box").iter().enumerate() {
        let w = w.as_f64().expect("a number");
        assert!((a[i] - w).abs() <= 1e-9, "{what}[{i}]: {} ≠ {w}", a[i]);
    }
}

#[test]
fn a_wms_reads_as_owslib_reads_it() {
    let file = fixture();
    for w in file["wms"].as_array().expect("documents") {
        let name = w["file"].as_str().expect("a file");
        let caps = wms::read(&doc(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            caps.version,
            w["version"].as_str().expect("a version"),
            "{name}"
        );
        assert_eq!(caps.title, w["title"].as_str().expect("a title"), "{name}");
        assert_eq!(
            caps.get_map.as_deref(),
            w["getMap"].as_str(),
            "{name}: GetMap"
        );
        assert_eq!(caps.formats, strings(&w["formats"]), "{name}: formats");
        let named: Vec<&wms::WmsLayer> = caps.layers.iter().filter(|l| l.name.is_some()).collect();
        let want = w["layers"].as_array().expect("layers");
        assert_eq!(named.len(), want.len(), "{name}: named layers");
        for (l, x) in named.iter().zip(want) {
            let id = x["name"].as_str().expect("a name");
            assert_eq!(l.name.as_deref(), Some(id), "{name}");
            assert_eq!(
                l.title,
                x["title"].as_str().expect("a title"),
                "{name}: {id}"
            );
            assert_eq!(
                l.queryable,
                x["queryable"].as_bool().expect("yes or no"),
                "{name}: {id}: queryable"
            );
            let mut styles: Vec<String> = l.styles.iter().map(|s| s.name.clone()).collect();
            styles.sort();
            assert_eq!(styles, strings(&x["styles"]), "{name}: {id}: styles");
            let mut srids = l.srids.clone();
            srids.sort_unstable();
            srids.dedup();
            let want: Vec<u32> = x["srids"]
                .as_array()
                .expect("systems")
                .iter()
                .map(|s| s.as_u64().expect("a code") as u32)
                .collect();
            assert_eq!(srids, want, "{name}: {id}: systems");
            match (&l.wgs84, &x["wgs84"]) {
                (Some(b), v) if v.is_array() => near(*b, v, &format!("{name}: {id}: WGS 84")),
                (None, Value::Null) => {}
                other => panic!("{name}: {id}: WGS 84 {other:?}"),
            }
        }
    }
}

#[test]
fn a_wmts_reads_as_owslib_reads_it_its_corners_east_and_north() {
    let file = fixture();
    let w = &file["wmts"];
    let caps = wmts::read(&doc("wmts.xml")).expect("reads");
    assert_eq!(caps.title, w["title"].as_str().expect("a title"));
    let want = w["layers"].as_array().expect("layers");
    assert_eq!(caps.layers.len(), want.len());
    for (l, x) in caps.layers.iter().zip(want) {
        let id = x["id"].as_str().expect("an id");
        assert_eq!(l.id, id);
        assert_eq!(l.title, x["title"].as_str().expect("a title"), "{id}");
        assert_eq!(l.formats, strings(&x["formats"]), "{id}: formats");
        let mut styles: Vec<String> = l.styles.iter().map(|s| s.id.clone()).collect();
        styles.sort();
        assert_eq!(styles, strings(&x["styles"]), "{id}: styles");
        let mut sets: Vec<String> = l.links.iter().map(|k| k.set.clone()).collect();
        sets.sort();
        assert_eq!(sets, strings(&x["sets"]), "{id}: matrix sets");
        let templates: Vec<(String, String)> = x["templates"]
            .as_array()
            .expect("templates")
            .iter()
            .map(|t| {
                (
                    t[0].as_str().expect("a format").to_owned(),
                    t[1].as_str().expect("a template").to_owned(),
                )
            })
            .collect();
        assert_eq!(l.templates, templates, "{id}: templates");
        match (&l.wgs84, &x["wgs84"]) {
            (Some(b), v) if v.is_array() => near(*b, v, &format!("{id}: WGS 84")),
            (None, Value::Null) => {}
            other => panic!("{id}: WGS 84 {other:?}"),
        }
    }
    let sets = w["sets"].as_array().expect("matrix sets");
    assert_eq!(caps.matrix_sets.len(), sets.len());
    for (s, x) in caps.matrix_sets.iter().zip(sets) {
        let id = x["id"].as_str().expect("an id");
        assert_eq!(s.id, id);
        assert_eq!(s.srid.map(u64::from), x["srid"].as_u64(), "{id}: system");
        let matrices = x["matrices"].as_array().expect("matrices");
        assert_eq!(s.matrices.len(), matrices.len(), "{id}: matrices");
        for (m, y) in s.matrices.iter().zip(matrices) {
            let mid = format!("{id}/{}", y["id"].as_str().expect("an id"));
            assert_eq!(m.id, y["id"].as_str().expect("an id"), "{mid}");
            let res = y["resolution"].as_f64().expect("a pixel");
            assert!(
                (m.resolution - res).abs() <= res * 1e-12,
                "{mid}: pixel {} ≠ {res}",
                m.resolution
            );
            for (got, key) in [(m.x0, "x0"), (m.y0, "y0")] {
                let w = y[key].as_f64().expect("a corner");
                assert!((got - w).abs() <= 1e-9, "{mid}: {key} {got} ≠ {w}");
            }
            assert_eq!(
                u64::from(m.tile_width),
                y["tileWidth"].as_u64().expect("a side"),
                "{mid}"
            );
            assert_eq!(
                u64::from(m.tile_height),
                y["tileHeight"].as_u64().expect("a side"),
                "{mid}"
            );
            assert_eq!(
                m.matrix_width,
                y["matrixWidth"].as_u64().expect("a count"),
                "{mid}"
            );
            assert_eq!(
                m.matrix_height,
                y["matrixHeight"].as_u64().expect("a count"),
                "{mid}"
            );
        }
    }
}

#[test]
fn a_wfs_reads_as_owslib_reads_it() {
    let file = fixture();
    let w = &file["wfs"];
    let caps = wfs::read(&doc("wfs-200.xml")).expect("reads");
    assert_eq!(caps.title, w["title"].as_str().expect("a title"));
    assert_eq!(caps.get_feature.as_deref(), w["getFeature"].as_str());
    assert_eq!(caps.formats, strings(&w["formats"]));
    let types = w["types"].as_array().expect("types");
    assert_eq!(caps.types.len(), types.len());
    for (t, x) in caps.types.iter().zip(types) {
        let name = x["name"].as_str().expect("a name");
        assert_eq!(t.name, name);
        assert_eq!(t.title, x["title"].as_str().expect("a title"), "{name}");
        let want: Vec<u32> = x["srids"]
            .as_array()
            .expect("systems")
            .iter()
            .map(|s| s.as_u64().expect("a code") as u32)
            .collect();
        assert_eq!(t.srids, want, "{name}: systems");
    }
    assert!(caps.paging, "the document says it pages");
}
