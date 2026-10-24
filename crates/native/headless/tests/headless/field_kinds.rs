//! The field kinds of docs/adr/0629 out and back: `fixtures/kcad/v2/field-kinds.kcad` (a big integer past 2⁵³ and the
//! smallest 64-bit one, a time stamp with its zone and a fraction, a UUID, JSON with a number a double cannot hold,
//! binary) written as a GeoPackage, a GeoParquet, a FlatGeobuf, a GML and a CSV and read into a new drawing. GeoPackage
//! and GeoParquet keep every kind (KentOS's document, the file's types); the others keep every value, a time stamp the
//! same moment in UTC. With KENTOS_WRITE_KINDS=1 the files go to fixtures/field-kinds/v1/out for
//! `scripts/fixtures/field_kinds_cases.py --written`, which opens them with GDAL and pyarrow.

use std::collections::BTreeMap;

use kentos_headless::Session;
use serde_json::{Value, json};

use super::{new_drawing, scratch};

fn source() -> Session {
    let path = format!(
        "{}/../../../fixtures/kcad/v2/field-kinds.kcad",
        env!("CARGO_MANIFEST_DIR")
    );
    Session::open(std::path::Path::new(&path)).expect("opens")
}

/// Every object's attributes, by its Kimlik.
fn by_id(s: &Session) -> BTreeMap<String, Value> {
    let page = s
        .entities(None, None, None, None, Some(10_000))
        .expect("a page");
    let page = serde_json::to_value(&page).expect("JSON");
    page["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|e| {
            let attrs = e["entity"]["attrs"].clone();
            (
                attrs["Kimlik"].as_str().unwrap_or_default().to_owned(),
                attrs,
            )
        })
        .collect()
}

/// A layer's fields' kinds by name, the one layer the reading made.
fn kinds(s: &Session) -> BTreeMap<String, String> {
    fn walk(nodes: &[kentos_commands::LayerNode], out: &mut BTreeMap<String, String>) {
        for n in nodes {
            for f in &n.fields {
                out.insert(f.name.clone(), f.kind.name().to_owned());
            }
            walk(&n.children, out);
        }
    }
    let mut out = BTreeMap::new();
    walk(s.layers(), &mut out);
    out
}

/// The attributes as the files hold them: a time stamp in UTC.
fn in_files() -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            "9007199254740993".to_owned(),
            json!({ "Kimlik": "9007199254740993", "Ölçüm anı": "2024-03-01T07:20:30.5Z",
                    "Cihaz": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001",
                    "Ek bilgi": "{\"kot\":[1.25,12345678901234567890],\"not\":\"ç\"}", "İmza": "QUJD" }),
        ),
        (
            "-9223372036854775808".to_owned(),
            json!({ "Kimlik": "-9223372036854775808", "Ölçüm anı": "2024-03-01T07:20:30Z", "Ek bilgi": "[]" }),
        ),
    ])
}

#[test]
fn the_new_kinds_go_out_and_come_back() {
    let dir = scratch("kinds");
    let mut s = source();
    let all = BTreeMap::from([
        ("Kimlik", "bigint"),
        ("Ölçüm anı", "instant"),
        ("Cihaz", "uuid"),
        ("Ek bilgi", "json"),
        ("İmza", "binary"),
    ]);
    let everything: BTreeMap<String, String> = all
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    assert_eq!(kinds(&s), everything);
    let mut kept = Vec::new();
    // (file, export, import, the kinds that come back)
    let plain = |uuid: &str, json: &str, binary: &str| -> BTreeMap<String, String> {
        BTreeMap::from([
            ("Kimlik".to_owned(), "bigint".to_owned()),
            ("Ölçüm anı".to_owned(), "instant".to_owned()),
            ("Cihaz".to_owned(), uuid.to_owned()),
            ("Ek bilgi".to_owned(), json.to_owned()),
            ("İmza".to_owned(), binary.to_owned()),
        ])
    };
    for (file, export, import, want) in [
        (
            "turler.gpkg",
            "file.export.gpkg",
            "file.import.gpkg",
            everything.clone(),
        ),
        (
            "turler.parquet",
            "file.export.parquet",
            "file.import.parquet",
            everything.clone(),
        ),
        (
            "turler.fgb",
            "file.export.fgb",
            "file.import.fgb",
            plain("text", "text", "binary"),
        ),
        (
            "turler.gml",
            "file.export.gml",
            "file.import.gml",
            plain("text", "text", "binary"),
        ),
        (
            "turler.csv",
            "file.export.csv",
            "file.import.csv",
            plain("text", "text", "text"),
        ),
    ] {
        let path = dir.join(file).display().to_string();
        let out = s
            .call(json!({ "command": export, "input": { "path": path } }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{file}: {out}");
        let mut back = new_drawing();
        let got = back
            .call(json!({ "command": import, "input": { "path": path } }))
            .expect("answers");
        assert_eq!(got["status"], "completed", "{file}: {got}");
        // GML's element names have no spaces: “Ek bilgi” comes back as “Ek_bilgi”.
        let named = |name: &str| {
            if import == "file.import.gml" {
                name.replace(' ', "_")
            } else {
                name.to_owned()
            }
        };
        let want: BTreeMap<String, String> =
            want.into_iter().map(|(k, v)| (named(&k), v)).collect();
        let values: BTreeMap<String, Value> = in_files()
            .into_iter()
            .map(|(id, attrs)| {
                let attrs: serde_json::Map<String, Value> = attrs
                    .as_object()
                    .expect("attributes")
                    .iter()
                    .map(|(k, v)| (named(k), v.clone()))
                    .collect();
                (id, Value::Object(attrs))
            })
            .collect();
        assert_eq!(kinds(&back), want, "{file}");
        assert_eq!(by_id(&back), values, "{file}");
        kept.push(path);
    }
    if std::env::var_os("KENTOS_WRITE_KINDS").is_some() {
        let out = format!(
            "{}/../../../fixtures/field-kinds/v1/out",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::create_dir_all(&out).expect("a folder");
        for p in &kept {
            let name = std::path::Path::new(p).file_name().expect("a name");
            std::fs::copy(p, std::path::Path::new(&out).join(name)).expect("kept");
            // GML's schema beside it.
            let xsd = std::path::Path::new(p).with_extension("xsd");
            if xsd.exists() {
                std::fs::copy(
                    &xsd,
                    std::path::Path::new(&out).join(xsd.file_name().expect("a name")),
                )
                .expect("kept");
            }
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// docs/adr/0629 §4: a GeoParquet's time stamp holds microseconds; a finer fraction is said before anything is written.
#[test]
fn a_finer_fraction_is_said_before_a_geoparquet_is_written() {
    let mut s = source();
    let page = serde_json::to_value(
        s.entities(None, None, None, None, Some(10))
            .expect("a page"),
    )
    .expect("JSON");
    let uid = page["items"][0]["uid"].as_str().expect("a uid").to_owned();
    let set = s
        .call(json!({ "command": "cad.entities.set", "input": { "uids": [uid], "operation": "attributes",
            "attrs": { "Ölçüm anı": "2024-03-01T10:20:30.123456789+03:00" } } }))
        .expect("answers");
    assert_eq!(set["status"], "completed", "{set}");
    let dir = scratch("kinds-fine");
    let path = dir.join("ince.parquet").display().to_string();
    let planned = s
        .call(json!({ "command": "file.export.parquet", "op": "plan", "input": { "path": path } }))
        .expect("answers");
    assert_eq!(planned["status"], "completed", "{planned}");
    assert!(
        planned["warnings"].to_string().contains(
            "“Ölçüm anı” alanının 1 zaman damgasının mikrosaniyeden ince kesri yazılmadı"
        ),
        "{planned}"
    );
    let _ = std::fs::remove_dir_all(dir);
}
