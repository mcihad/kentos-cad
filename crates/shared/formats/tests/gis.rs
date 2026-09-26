//! GeoJSON and Shapefile from `fixtures/formats/v1/gis/` (the same files the
//! WASM test reads, `apps/web/src/io/formats.wasm.test.ts`), compared with
//! what the independent reader (`tools/formats/gis.py`, written from the
//! rules in docs/adr/0046 without this crate) wrote for them: every object,
//! layer, label, attribute and coordinate, float for float. Then the
//! exports in `export/`: the writer's bytes as committed (which the same
//! reader checks against their input), and what this crate reads back.

use std::path::{Path, PathBuf};

use kentos_contracts::{Entity, GeoJsonReadOptions, ImportResult, ShapefileReadOptions, Vec2};
use kentos_formats::{geojson, shp};
use serde_json::{Map, Value, json};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/gis")
}

fn read_file(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn optional(path: PathBuf) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

fn xy(p: Vec2) -> Value {
    json!([p.x, p.y])
}

fn pts(p: &[Vec2]) -> Value {
    Value::Array(p.iter().map(|q| xy(*q)).collect())
}

/// The reading in the rules' canonical form (docs/adr/0046).
fn canonical(r: &ImportResult, encoding: Option<&str>) -> Value {
    let objects: Vec<Value> = r
        .entities
        .iter()
        .map(|e| {
            let mut o = Map::new();
            let base = match e {
                Entity::Point(p) => {
                    o.insert("kind".into(), json!("point"));
                    o.insert("p".into(), xy(p.p));
                    if let Some(z) = p.z {
                        o.insert("z".into(), json!(z));
                    }
                    &p.base
                }
                Entity::Line(l) => {
                    o.insert("kind".into(), json!("line"));
                    o.insert("a".into(), xy(l.a));
                    o.insert("b".into(), xy(l.b));
                    &l.base
                }
                Entity::Polyline(p) => {
                    o.insert("kind".into(), json!("polyline"));
                    o.insert("pts".into(), pts(&p.pts));
                    &p.base
                }
                Entity::Polygon(p) => {
                    o.insert("kind".into(), json!("polygon"));
                    o.insert("pts".into(), pts(&p.pts));
                    if let Some(h) = p.holes.as_ref().filter(|h| !h.is_empty()) {
                        o.insert(
                            "holes".into(),
                            Value::Array(h.iter().map(|r| pts(&r.pts)).collect()),
                        );
                    }
                    &p.base
                }
                other => panic!("a GIS reader made {other:?}"),
            };
            o.insert("layer".into(), json!(base.layer_id));
            if let Some(l) = &base.label {
                o.insert("label".into(), json!(l));
            }
            o.insert("attrs".into(), json!(base.attrs));
            Value::Object(o)
        })
        .collect();
    let mut doc = Map::new();
    doc.insert(
        "declaredSrid".into(),
        json!(r.declared_crs.as_ref().and_then(|d| d.srid)),
    );
    if let Some(e) = encoding {
        doc.insert("encoding".into(), json!(e));
    }
    doc.insert("objects".into(), Value::Array(objects));
    Value::Object(doc)
}

/// Equal as JSON, numbers compared as float64 bit for bit; the first difference's path.
fn diff(got: &Value, want: &Value, path: &str) -> Option<String> {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            let (x, y) = (a.as_f64()?, b.as_f64()?);
            (x.to_bits() != y.to_bits()).then(|| format!("{path}: {x:?} ≠ {y:?}"))
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{path}: {} öğe ≠ {}", a.len(), b.len()));
            }
            a.iter()
                .zip(b)
                .enumerate()
                .find_map(|(i, (x, y))| diff(x, y, &format!("{path}[{i}]")))
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            keys.into_iter().find_map(|k| match (a.get(k), b.get(k)) {
                (Some(x), Some(y)) => diff(x, y, &format!("{path}.{k}")),
                (x, y) => Some(format!("{path}.{k}: {x:?} ≠ {y:?}")),
            })
        }
        (a, b) => (a != b).then(|| format!("{path}: {a} ≠ {b}")),
    }
}

/// Every fixture with its expected reading.
fn fixtures() -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = std::fs::read_dir(dir())
        .expect("fixtures/formats/v1/gis")
        .filter_map(|e| {
            let p = e.ok()?.path();
            let name = p
                .file_name()?
                .to_str()?
                .strip_suffix(".expected.json")?
                .to_string();
            Some((name, p))
        })
        .collect();
    out.sort();
    out
}

/// Reads fixture `name` (a .geojson, or a .shp with the files beside it).
fn read_fixture(name: &str) -> (ImportResult, Option<&'static str>) {
    let d = dir();
    let geo = d.join(format!("{name}.geojson"));
    if geo.exists() {
        let r = geojson::read(
            &read_file(&geo),
            &GeoJsonReadOptions {
                layer: name.into(),
                max_entities: 0,
            },
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        return (r, None);
    }
    let shp_bytes = read_file(&d.join(format!("{name}.shp")));
    let (shx, dbf, prj, cpg) = (
        optional(d.join(format!("{name}.shx"))),
        optional(d.join(format!("{name}.dbf"))),
        optional(d.join(format!("{name}.prj"))),
        optional(d.join(format!("{name}.cpg"))),
    );
    let files = shp::Files {
        shp: &shp_bytes,
        shx: shx.as_deref(),
        dbf: dbf.as_deref(),
        prj: prj.as_deref(),
        cpg: cpg.as_deref(),
    };
    let r = shp::read(
        &files,
        &ShapefileReadOptions {
            layer: name.into(),
            max_entities: 0,
        },
    )
    .unwrap_or_else(|e| panic!("{name}: {e}"));
    (r, Some(shp::encoding_name(&files)))
}

#[test]
fn every_fixture_reads_as_the_independent_reader_reads_it() {
    let all = fixtures();
    assert!(all.len() >= 12, "fixtures: {all:?}");
    let mut failed = Vec::new();
    for (name, expected) in &all {
        let want: Value = serde_json::from_slice(&read_file(expected)).expect("expected json");
        let (r, encoding) = read_fixture(name);
        if let Some(d) = diff(&canonical(&r, encoding), &want, name) {
            failed.push(d);
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn the_reports_say_what_was_left_out_and_why() {
    let (r, _) = read_fixture("nonfinite");
    let skipped: Vec<&str> = r.report.skipped.iter().map(|i| i.what.as_str()).collect();
    assert!(skipped.contains(&"Sonlu olmayan koordinat"), "{skipped:?}");
    assert!(skipped.contains(&"Geçersiz koordinat"), "{skipped:?}");
    let (r, enc) = read_fixture("parseller");
    assert_eq!(enc, Some("CP857"));
    let skipped: Vec<&str> = r.report.skipped.iter().map(|i| i.what.as_str()).collect();
    assert!(skipped.contains(&"Silinmiş kayıt"), "{skipped:?}");
    let facts: Vec<(&str, &str)> = r
        .report
        .source
        .iter()
        .map(|f| (f.label.as_str(), f.value.as_str()))
        .collect();
    assert!(
        facts
            .iter()
            .any(|(l, v)| *l == "Kodlama" && v.starts_with("CP857")),
        "{facts:?}"
    );
    let (r, _) = read_fixture("alanlarz");
    let noted: Vec<&str> = r.report.notes.iter().map(|i| i.what.as_str()).collect();
    assert!(noted.contains(&"Z (yükseklik)"), "{noted:?}");
}

/// Exports: `<name>.input.json` written as `<name>.geojson` (committed; the
/// independent reader checks it against its input), and read back here.
#[test]
fn exports_are_written_as_committed_and_read_back() {
    let export = dir().join("export");
    let Ok(entries) = std::fs::read_dir(&export) else {
        panic!("{} yok", export.display());
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| {
            let p = e.ok()?.path();
            Some(
                p.file_name()?
                    .to_str()?
                    .strip_suffix(".input.json")?
                    .to_string(),
            )
        })
        .collect();
    names.sort();
    assert!(!names.is_empty());
    let update = std::env::var_os("KENTOS_WRITE_GIS_EXPORTS").is_some();
    for name in names {
        let input = geojson::input_from_json(
            std::str::from_utf8(&read_file(&export.join(format!("{name}.input.json"))))
                .expect("utf8"),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let (bytes, _) = geojson::write(&input);
        let path = export.join(format!("{name}.geojson"));
        if update {
            std::fs::write(&path, &bytes).expect("write");
        }
        assert_eq!(
            String::from_utf8(bytes.clone()).expect("utf8"),
            String::from_utf8(read_file(&path)).expect("utf8"),
            "{name}: the writer's bytes differ from the committed file (KENTOS_WRITE_GIS_EXPORTS=1 rewrites it; read the difference first)"
        );
        let back = geojson::read(
            &bytes,
            &GeoJsonReadOptions {
                layer: name.clone(),
                max_entities: 0,
            },
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            back.declared_crs.and_then(|d| d.srid),
            Some(input.srid),
            "{name}"
        );
    }
}

/// A small deterministic generator (xorshift) for damaged bytes.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Every cut of every fixture file and thousands of damaged copies read to an
/// answer (objects or a reason), never a panic, whatever the bytes say.
#[test]
fn cut_or_damaged_files_never_panic() {
    let d = dir();
    let geo = GeoJsonReadOptions {
        layer: "x".into(),
        max_entities: 0,
    };
    let shp_opts = ShapefileReadOptions {
        layer: "x".into(),
        max_entities: 0,
    };
    let mut noise = Noise(0x9E37_79B9_7F4A_7C15);
    for (name, _) in fixtures() {
        let path = d.join(format!("{name}.geojson"));
        if path.exists() {
            let bytes = read_file(&path);
            for n in 0..=bytes.len() {
                let _ = geojson::read(&bytes[..n], &geo);
            }
            for _ in 0..2000 {
                let mut b = bytes.clone();
                for _ in 0..1 + noise.next() % 4 {
                    let i = (noise.next() % b.len() as u64) as usize;
                    b[i] = noise.next() as u8;
                }
                let _ = geojson::read(&b, &geo);
            }
            continue;
        }
        let part = |ext: &str| optional(d.join(format!("{name}.{ext}")));
        let (shp_bytes, shx, dbf, prj, cpg) = (
            read_file(&d.join(format!("{name}.shp"))),
            part("shx"),
            part("dbf"),
            part("prj"),
            part("cpg"),
        );
        let files = |shp: &[u8], dbf: Option<&[u8]>, prj: Option<&[u8]>| {
            let f = shp::Files {
                shp,
                shx: shx.as_deref(),
                dbf,
                prj,
                cpg: cpg.as_deref(),
            };
            let _ = shp::read(&f, &shp_opts);
        };
        for n in 0..=shp_bytes.len() {
            files(&shp_bytes[..n], dbf.as_deref(), prj.as_deref());
        }
        if let Some(dbf) = &dbf {
            for n in 0..=dbf.len() {
                files(&shp_bytes, Some(&dbf[..n]), prj.as_deref());
            }
        }
        if let Some(prj) = &prj {
            for n in 0..=prj.len() {
                files(&shp_bytes, dbf.as_deref(), Some(&prj[..n]));
            }
        }
        for _ in 0..2000 {
            let mut s = shp_bytes.clone();
            let mut t = dbf.clone();
            for _ in 0..1 + noise.next() % 4 {
                // Damage past the header too: lengths, counts, part indices, doubles.
                let i = (noise.next() % s.len() as u64) as usize;
                s[i] = noise.next() as u8;
                if let Some(t) = t.as_mut() {
                    let j = (noise.next() % t.len() as u64) as usize;
                    t[j] = noise.next() as u8;
                }
            }
            files(&s, t.as_deref(), prj.as_deref());
        }
    }
}
