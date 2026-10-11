//! KML and KMZ (docs/adr/0222) against fixtures/formats/v1/kml/
//! (scripts/fixtures/kml_cases.py, from OGC KML 2.2 and the ADR's rules with
//! Python's expat and PROJ, no KentOS code): each sample read as the
//! reference reads it, float for float; each reading moved into a project's
//! system as PROJ moves it; and the writer's files as committed
//! (`KENTOS_WRITE_KML=1` rewrites them; the reference's --check holds them to
//! their inputs). The web reads the same files through WASM
//! (`apps/web/src/io/kml.wasm.test.ts`).

use std::path::{Path, PathBuf};

use kentos_contracts::{Entity, ImportResult, KmlWriteInput, RingGeometry, Vec2};
use kentos_formats::kml;
use kentos_formats::reproject::reproject;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::crs::{Choice, System};
use serde_json::{Map, Value, json};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/kml")
}

fn cases() -> Value {
    let text = std::fs::read_to_string(dir().join("cases.json")).expect("cases.json");
    serde_json::from_str(&text).expect("the cases read")
}

fn xy(p: Vec2) -> Value {
    json!([p.x, p.y])
}

fn pts(p: &[Vec2]) -> Value {
    Value::Array(p.iter().map(|q| xy(*q)).collect())
}

fn zs(z: &[Option<f64>]) -> Value {
    Value::Array(
        z.iter()
            .map(|z| z.map_or(Value::Null, |z| json!(z)))
            .collect(),
    )
}

fn area(
    o: &mut Map<String, Value>,
    p: &[Vec2],
    z: &Option<Vec<Option<f64>>>,
    holes: &Option<Vec<RingGeometry>>,
) {
    o.insert("pts".into(), pts(p));
    if let Some(z) = z {
        o.insert("zs".into(), zs(z));
    }
    if let Some(h) = holes.as_ref().filter(|h| !h.is_empty()) {
        o.insert(
            "holes".into(),
            Value::Array(h.iter().map(|r| pts(&r.pts)).collect()),
        );
        if h.iter().any(|r| r.zs.is_some()) {
            o.insert(
                "holeZs".into(),
                Value::Array(
                    h.iter()
                        .map(|r| r.zs.as_deref().map_or(Value::Null, zs))
                        .collect(),
                ),
            );
        }
    }
}

/// An object in the reference's form: its geometry, layer, attributes, colour and line weight.
fn object(e: &Entity) -> Value {
    let mut o = Map::new();
    match e {
        Entity::Point(p) => {
            o.insert("kind".into(), json!("point"));
            o.insert("p".into(), xy(p.p));
            if let Some(z) = p.z {
                o.insert("z".into(), json!(z));
            }
            if let Some(parts) = p.parts.as_ref().filter(|p| !p.is_empty()) {
                let parts = parts.iter().map(|q| {
                    let mut m = Map::new();
                    m.insert("p".into(), xy(q.p));
                    if let Some(z) = q.z {
                        m.insert("z".into(), json!(z));
                    }
                    Value::Object(m)
                });
                o.insert("parts".into(), Value::Array(parts.collect()));
            }
        }
        Entity::Line(l) => {
            o.insert("kind".into(), json!("line"));
            o.insert("a".into(), xy(l.a));
            o.insert("b".into(), xy(l.b));
            for (key, z) in [("za", l.za), ("zb", l.zb)] {
                if let Some(z) = z {
                    o.insert(key.into(), json!(z));
                }
            }
        }
        Entity::Polyline(p) => {
            o.insert("kind".into(), json!("polyline"));
            o.insert("pts".into(), pts(&p.pts));
            if let Some(z) = &p.zs {
                o.insert("zs".into(), zs(z));
            }
            if let Some(parts) = p.parts.as_ref().filter(|p| !p.is_empty()) {
                let parts = parts.iter().map(|part| {
                    let mut m = Map::new();
                    m.insert("pts".into(), pts(&part.pts));
                    if let Some(z) = &part.zs {
                        m.insert("zs".into(), zs(z));
                    }
                    Value::Object(m)
                });
                o.insert("parts".into(), Value::Array(parts.collect()));
            }
        }
        Entity::Polygon(p) => {
            o.insert("kind".into(), json!("polygon"));
            area(&mut o, &p.pts, &p.zs, &p.holes);
            if let Some(parts) = p.parts.as_ref().filter(|p| !p.is_empty()) {
                let parts = parts.iter().map(|part| {
                    let mut m = Map::new();
                    area(&mut m, &part.pts, &part.zs, &part.holes);
                    Value::Object(m)
                });
                o.insert("parts".into(), Value::Array(parts.collect()));
            }
        }
        other => panic!("the KML reader made {other:?}"),
    }
    let b = e.base();
    o.insert("layer".into(), json!(b.layer_id));
    o.insert("attrs".into(), json!(b.attrs));
    if let Some(c) = &b.color {
        o.insert("color".into(), json!(c));
    }
    if let Some(w) = b.line_weight {
        o.insert("lineWeight".into(), json!(w));
    }
    Value::Object(o)
}

fn items(list: &[kentos_contracts::ReportItem]) -> Value {
    Value::Array(
        list.iter()
            .map(
                |i| json!({"what": i.what, "reason": i.reason, "count": i.count, "lines": i.lines}),
            )
            .collect(),
    )
}

/// A reading in the reference's form.
fn canonical(r: &ImportResult) -> Value {
    json!({
        "objects": r.entities.iter().map(object).collect::<Vec<_>>(),
        "layers": r.layers.iter().map(|l| json!({"name": l.name, "color": l.color, "visible": l.visible, "count": l.count})).collect::<Vec<_>>(),
        "counts": r.report.counts,
        "skipped": items(&r.report.skipped),
        "notes": items(&r.report.notes),
        "source": r.report.source.iter().map(|f| json!([f.label, f.value])).collect::<Vec<_>>(),
        "declared": r.declared_crs,
    })
}

/// Equal as JSON, numbers as float64 bit for bit (or within `tol`); the first difference's path.
fn diff(got: &Value, want: &Value, path: &str, tol: f64) -> Option<String> {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            let (x, y) = (a.as_f64()?, b.as_f64()?);
            let same = if tol == 0.0 {
                x.to_bits() == y.to_bits()
            } else {
                (x - y).abs() <= tol
            };
            (!same).then(|| format!("{path}: {x:?} ≠ {y:?}"))
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{path}: {} öğe ≠ {}", a.len(), b.len()));
            }
            a.iter()
                .zip(b)
                .enumerate()
                .find_map(|(i, (x, y))| diff(x, y, &format!("{path}[{i}]"), tol))
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            keys.into_iter().find_map(|k| match (a.get(k), b.get(k)) {
                (Some(x), Some(y)) => diff(x, y, &format!("{path}.{k}"), tol),
                (x, y) => Some(format!("{path}.{k}: {x:?} ≠ {y:?}")),
            })
        }
        (a, b) => (a != b).then(|| format!("{path}: {a} ≠ {b}")),
    }
}

fn read_sample(file: &str) -> Result<ImportResult, String> {
    let bytes = std::fs::read(dir().join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
    kml::read(&bytes, 0)
}

fn system_of(v: &Value) -> System {
    System::from_json(&Json::parse(&v.to_string()).expect("JSON")).expect("a system")
}

#[test]
fn every_sample_reads_as_the_reference_reads_it() {
    let cases = cases();
    let reads = cases["reads"].as_array().expect("reads");
    assert!(reads.len() >= 5);
    for case in reads {
        let file = case["file"].as_str().expect("a file");
        let expect = &case["expect"];
        match read_sample(file) {
            Ok(r) => {
                assert!(
                    expect.get("error").is_none(),
                    "{file}: read, the reference says {}",
                    expect["error"]
                );
                let got = canonical(&r);
                if let Some(d) = diff(&got, expect, file, 0.0) {
                    panic!(
                        "{d}\n{}",
                        serde_json::to_string(&got["source"]).unwrap_or_default()
                    );
                }
            }
            Err(e) => assert_eq!(Some(e.as_str()), expect["error"].as_str(), "{file}"),
        }
    }
}

#[test]
fn readings_move_into_a_projects_system_as_proj_moves_them() {
    let cases = cases();
    let metres = cases["tolerance"]["metres"].as_f64().expect("a tolerance");
    let moves = cases["moves"].as_array().expect("moves");
    assert!(moves.len() >= 2);
    for case in moves {
        let file = case["file"].as_str().expect("a file");
        let mut r = read_sample(file).expect("the sample reads");
        let to = system_of(&case["system"]);
        let from = System::Geographic {
            datum: kentos_geometry_core::crs::Datum::Wgs84,
        };
        let moved = reproject(&mut r, &from, &to, &[] as &[Choice]);
        let want = &case["moved"];
        assert_eq!(
            u64::from(moved.objects),
            want["objects"].as_u64().expect("objects"),
            "{file}"
        );
        assert_eq!(
            moved.vertices,
            want["vertices"].as_u64().expect("vertices"),
            "{file}"
        );
        assert_eq!(moved.accuracy, want["accuracy"].as_f64(), "{file}");
        assert_eq!(moved.via, want["via"].as_str().expect("via"), "{file}");
        assert_eq!(
            moved.unofficial,
            want["unofficial"].as_bool().expect("unofficial"),
            "{file}"
        );
        let got = Value::Array(r.entities.iter().map(object).collect());
        if let Some(d) = diff(
            &got,
            &case["objects"],
            &format!("{file} → {}", case["srid"]),
            metres,
        ) {
            panic!("{d}");
        }
        assert!(r.bounds.is_some() && r.view.is_some(), "{file}");
    }
}

/// The writer's cases (write/<name>.case.json): its bytes as committed.
#[test]
fn the_writer_writes_as_committed() {
    let write = dir().join("write");
    let mut names: Vec<String> = std::fs::read_dir(&write)
        .expect("write/")
        .filter_map(|e| {
            let p = e.ok()?.path();
            Some(
                p.file_name()?
                    .to_str()?
                    .strip_suffix(".case.json")?
                    .to_owned(),
            )
        })
        .collect();
    names.sort();
    assert!(names.len() >= 2);
    let update = std::env::var_os("KENTOS_WRITE_KML").is_some();
    for name in names {
        let case: Value = serde_json::from_str(
            &std::fs::read_to_string(write.join(format!("{name}.case.json"))).expect("the case"),
        )
        .expect("JSON");
        let input: KmlWriteInput =
            serde_json::from_value::<kml::write::WriteInput>(case["input"].clone())
                .map(|w| w.0)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
        let from = system_of(&case["system"]);
        let choices: Vec<Choice> =
            Vec::<Choice>::from_json(&Json::parse(&case["choices"].to_string()).expect("JSON"))
                .expect("choices");
        let (bytes, report) = kml::write(&input, &from, &choices);
        let path = write.join(format!("{name}.{}", if input.kmz { "kmz" } else { "kml" }));
        if update {
            std::fs::write(&path, &bytes).expect("write");
        }
        assert_eq!(
            bytes,
            std::fs::read(&path).unwrap_or_default(),
            "{name}: the writer's bytes differ from the committed file (KENTOS_WRITE_KML=1 rewrites it; read the difference first)"
        );
        // What it left out and noted.
        assert_eq!(report.counts.get("text"), Some(&1), "{name}");
        assert!(
            report.notes.iter().any(|n| n.what == "Daire"),
            "{name}: {:?}",
            report.notes
        );
        assert!(
            report.notes.iter().any(|n| n.what == "Kotsuz köşe"),
            "{name}: {:?}",
            report.notes
        );
        // It reads back: the same objects, a layer a folder path.
        let back = kml::read(&bytes, 0).unwrap_or_else(|e| panic!("{name}: {e}"));
        let written = input.entities.len();
        assert_eq!(back.entities.len(), written, "{name}");
        assert!(
            back.layers
                .iter()
                .any(|l| l.name == "Kadastro / Ada 101 / Parseller"),
            "{name}"
        );
    }
}

/// A vertex the move does not reach leaves its object out, said (a latitude past the pole).
#[test]
fn an_object_the_move_does_not_reach_is_left_out_and_said() {
    let input: KmlWriteInput = serde_json::from_value::<kml::write::WriteInput>(json!({
        "name": "Uçta",
        "layers": [{"id": "a", "name": "A", "color": "#ff0000"}],
        "entities": [
            {"kind": "point", "id": 1, "layerId": "a", "attrs": {}, "p": {"x": 32.0, "y": 39.0}},
            {"kind": "point", "id": 2, "layerId": "a", "attrs": {}, "p": {"x": 32.0, "y": 95.0}}
        ]
    }))
    .expect("the input")
    .0;
    let from = System::Geographic {
        datum: kentos_geometry_core::crs::Datum::Turef,
    };
    let (bytes, report) = kml::write(&input, &from, &[]);
    assert_eq!(report.counts.get("point"), Some(&1));
    assert!(
        report.skipped.iter().any(|s| s.what == "Çevrilemeyen nesne"
            && s.count == 1
            && s.reason.contains("ulaştığı yerin dışında")),
        "{:?}",
        report.skipped
    );
    let text = String::from_utf8(bytes).expect("UTF-8");
    assert_eq!(text.matches("<Placemark>").count(), 1);
}
