//! Mesh and multidimensional data (docs/adr/0243) against
//! fixtures/multidim/v1/cases.json (scripts/fixtures/multidim_cases.py: the
//! files written by the reference's own NetCDF writer and by GDAL, the values
//! worked out without KentOS code, cross-checked against GDAL and QGIS's MDAL).

use std::path::PathBuf;

use kentos_formats::multidim::cf;
use kentos_formats::multidim::cube::{Cube, Part, Want};
use kentos_formats::multidim::netcdf;
use kentos_formats::multidim::sms;
use kentos_formats::multidim::write;
use kentos_formats::raster::source::Reader;
use kentos_formats::raster::{ByteStore, Step};
use serde_json::Value;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/multidim/v1")
}

fn file(name: &str) -> Vec<u8> {
    std::fs::read(dir().join("files").join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn cases() -> Vec<Value> {
    let text = std::fs::read_to_string(dir().join("cases.json")).expect("cases.json");
    let v: Value = serde_json::from_str(&text).expect("JSON");
    v["cases"].as_array().expect("cases").clone()
}

/// JSON values alike, numbers by their float value.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

fn part_of(c: &Value) -> Part {
    let p = &c["part"];
    let affine = p["affine"]
        .as_array()
        .map(|a| {
            let v: Vec<f64> = a.iter().map(|x| x.as_f64().unwrap_or(0.0)).collect();
            [v[0], v[1], v[2], v[3], v[4], v[5]]
        })
        .unwrap_or([0.0, 1.0, 0.0, 0.0, 0.0, -1.0]);
    Part {
        variable: p["variable"].as_str().expect("variable").to_owned(),
        vector: p["vector"].as_str().map(str::to_owned),
        mesh: p["mesh"].as_str().map(str::to_owned),
        slice: p["slice"]
            .as_array()
            .map(|a| a.iter().map(|x| x.as_u64().unwrap_or(0) as u32).collect())
            .unwrap_or_default(),
        affine,
        width: p["width"].as_u64().unwrap_or(1) as u32,
        height: p["height"].as_u64().unwrap_or(1) as u32,
    }
}

/// Every block a region wants handed over from the file's bytes.
fn fill(reader: &mut Reader, bytes: &[u8], level: usize, x: i64, y: i64, w: u32, h: u32) {
    for _ in 0..4 {
        let needs = reader.needs(level, x, y, w, h);
        if needs.is_empty() {
            return;
        }
        for n in needs {
            let (a, b) = (n.offset as usize, (n.offset + n.len) as usize);
            reader.put_block(&n, &bytes[a..b]).expect("block");
        }
    }
}

/// Within one unit in the last place of a 32-bit float (the engine works in
/// 64 bits, the reference exactly; both rounded once to 32).
fn near_f32(actual: f64, expected: f64) -> bool {
    if actual == expected {
        return true;
    }
    let e = expected as f32;
    let ulp = (f32::from_bits(e.to_bits() + 1) - e).abs() as f64;
    (actual - expected).abs() <= ulp
}

fn value_matches(actual: f64, expected: &Value, f32s: bool) -> bool {
    match expected.as_f64() {
        None => actual.is_nan(),
        Some(e) if f32s => near_f32(actual, e),
        Some(e) => (actual - e).abs() <= 1e-12 * e.abs().max(1.0),
    }
}

#[test]
fn every_case_is_read_as_the_reference_reads_it() {
    let mut played = 0;
    for c in cases() {
        let name = c["name"].as_str().unwrap_or("?").to_owned();
        match c["kind"].as_str() {
            Some("time") => {
                let t = cf::cf_time(c["units"].as_str().unwrap(), c["calendar"].as_str());
                let got: Option<Vec<f64>> = t.map(|t| {
                    c["values"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| t.moment(v.as_f64().unwrap()).unwrap())
                        .collect()
                });
                let want: Option<Vec<f64>> = c["expect"]
                    .as_array()
                    .map(|a| a.iter().map(|v| v.as_f64().unwrap()).collect());
                assert_eq!(got, want, "{name}");
            }
            Some("info") => {
                let mut cube = Cube::from_bytes(&file(c["file"].as_str().unwrap())).expect(&name);
                let info = serde_json::to_value(cube.info().expect(&name)).unwrap();
                assert!(
                    same(&info, &c["expect"]),
                    "{name}\n got  {info}\n want {}",
                    c["expect"]
                );
            }
            Some("region") => {
                let bytes = file(c["file"].as_str().unwrap());
                let mut cube = Cube::from_bytes(&bytes).expect(&name);
                let mut reader = cube.open(&part_of(&c), 64 << 20).expect(&name);
                let r: Vec<i64> = c["region"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap())
                    .collect();
                let (level, x, y, w, h) = (r[0] as usize, r[1], r[2], r[3] as u32, r[4] as u32);
                fill(&mut reader, &bytes, level, x, y, w, h);
                let region = reader.region(level, x, y, w, h).expect(&name);
                assert_eq!(
                    region.samples.kind().name(),
                    c["expect"]["sample"].as_str().unwrap(),
                    "{name}"
                );
                let want = c["expect"]["values"].as_array().unwrap();
                assert_eq!(region.samples.len(), want.len(), "{name}");
                for (k, e) in want.iter().enumerate() {
                    let a = region.samples.get(k);
                    assert!(
                        value_matches(a, e, false),
                        "{name}: value {k} is {a}, wants {e}"
                    );
                }
            }
            Some("mesh") => {
                let bytes = file(c["file"].as_str().unwrap());
                let mut cube = Cube::from_bytes(&bytes).expect(&name);
                let mut reader = cube.open(&part_of(&c), 64 << 20).expect(&name);
                for t in c["tiles"].as_array().unwrap() {
                    let (level, tx, ty) = (
                        t["level"].as_u64().unwrap() as usize,
                        t["tx"].as_i64().unwrap(),
                        t["ty"].as_i64().unwrap(),
                    );
                    let (x, y) = (tx * 256, ty * 256);
                    fill(&mut reader, &bytes, level, x, y, 256, 256);
                    let region = reader.region(level, x, y, 256, 256).expect(&name);
                    for (p, e) in t["pixels"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .zip(t["values"].as_array().unwrap())
                    {
                        let (i, j) = (p[0].as_u64().unwrap() as u32, p[1].as_u64().unwrap() as u32);
                        let a = region.at(i, j, 0);
                        assert!(
                            value_matches(a, e, true),
                            "{name}: level {level} tile ({tx}, {ty}) pixel ({i}, {j}) is {a}, wants {e}"
                        );
                    }
                }
                fill(&mut reader, &bytes, 0, 0, 0, 1, 1);
                for (p, e) in c["points"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .zip(c["values"].as_array().unwrap())
                {
                    let (x, y) = (p[0].as_f64().unwrap(), p[1].as_f64().unwrap());
                    let a = reader.mesh_value(x, y).expect(&name);
                    assert!(
                        value_matches(a, e, false),
                        "{name}: point ({x}, {y}) is {a}, wants {e}"
                    );
                }
            }
            Some("large") => {
                // A file of gigabytes whose header and coordinates alone are kept, its size as the file would have it.
                let bytes = file(c["file"].as_str().unwrap());
                let size = c["size"].as_u64().unwrap();
                let mut store = ByteStore::new();
                store.put(0, bytes.clone());
                let mut cube = match Cube::parse(&store, size).expect(&name) {
                    Step::Done(c) => c,
                    Step::Need(n) => panic!(
                        "{name}: {} bytes at {} asked for past the header",
                        n.len, n.offset
                    ),
                };
                for n in cube.needs(Want::Inspect) {
                    cube.put(
                        n.offset,
                        bytes[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
                    );
                }
                let info = serde_json::to_value(cube.info().expect(&name)).unwrap();
                assert!(
                    same(&info, &c["expect"]),
                    "{name}\n got  {info}\n want {}",
                    c["expect"]
                );
            }
            Some("error") => {
                let got = Cube::from_bytes(&file(c["file"].as_str().unwrap()));
                let err = got.err().map(|e| e.0);
                assert_eq!(err.as_deref(), c["expect"].as_str(), "{name}");
            }
            Some("parse") => {
                let bytes = file(c["file"].as_str().unwrap());
                let mut store = ByteStore::new();
                let mut asked = Vec::new();
                loop {
                    match netcdf::parse(&store, bytes.len() as u64).expect(&name) {
                        Step::Done(_) => break,
                        Step::Need(n) => {
                            asked.push(n.len);
                            store.put(0, bytes[..n.len as usize].to_vec());
                        }
                    }
                }
                let want: Vec<u64> = c["expect"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect();
                assert_eq!(asked, want, "{name}");
            }
            Some("read2dm") => {
                let m = sms::read_2dm(&file(c["file"].as_str().unwrap())).expect(&name);
                let e = &c["expect"];
                let floats = |k: &str| -> Vec<f64> {
                    e[k].as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_f64().unwrap())
                        .collect()
                };
                assert_eq!(m.x, floats("x"), "{name}");
                assert_eq!(m.y, floats("y"), "{name}");
                assert_eq!(m.z, floats("z"), "{name}");
                let faces: Vec<Vec<u32>> = e["faces"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|f| {
                        f.as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_u64().unwrap() as u32)
                            .collect()
                    })
                    .collect();
                assert_eq!(m.faces, faces, "{name}");
                assert_eq!(m.skipped as u64, e["skipped"].as_u64().unwrap(), "{name}");
            }
            Some("sms") => {
                let mesh = sms::read_2dm(&file(c["mesh"].as_str().unwrap())).expect(&name);
                let dats: Vec<(String, Vec<sms::DatDataset>)> = c["dats"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|d| {
                        let f = d.as_str().unwrap();
                        (f.to_owned(), sms::read_dat(&file(f)).expect(f))
                    })
                    .collect();
                let mut out = Vec::new();
                let mut sink = |b: &[u8]| -> Result<(), String> {
                    out.extend_from_slice(b);
                    Ok(())
                };
                let report = write::write_sms(
                    &mesh,
                    &dats,
                    c["start"].as_f64(),
                    c["epsg"].as_u64().map(|e| e as u32),
                    false,
                    &mut sink,
                )
                .expect(&name);
                let want = file(c["expect"]["file"].as_str().unwrap());
                if out != want {
                    let at = out
                        .iter()
                        .zip(&want)
                        .position(|(a, b)| a != b)
                        .unwrap_or(out.len().min(want.len()));
                    panic!(
                        "{name}: the written file differs at byte {at} (written {}, wanted {} bytes)",
                        out.len(),
                        want.len()
                    );
                }
                let e = &c["expect"]["report"];
                assert_eq!(report.nodes as u64, e["nodes"].as_u64().unwrap(), "{name}");
                assert_eq!(report.faces as u64, e["faces"].as_u64().unwrap(), "{name}");
                let strs = |k: &str| -> Vec<String> {
                    e[k].as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap().to_owned())
                        .collect()
                };
                assert_eq!(report.datasets, strs("datasets"), "{name}");
                assert_eq!(report.notes, strs("notes"), "{name}");
                // The file the engine wrote reads back as a mesh with its datasets.
                let mut cube = Cube::from_bytes(&out).expect(&name);
                let info = cube.info().expect(&name);
                assert_eq!(info.meshes.len(), 1, "{name}");
            }
            Some("series") => {
                let bytes = file(c["file"].as_str().unwrap());
                let mut cube = Cube::from_bytes(&bytes).expect(&name);
                let part = part_of(&c);
                // A grid raster placed by the grid's own place; a mesh's points are the mesh's.
                let affine = cube
                    .info()
                    .expect(&name)
                    .grids
                    .iter()
                    .find(|g| g.variable == part.variable)
                    .and_then(|g| g.affine)
                    .unwrap_or(part.affine);
                let points: Vec<[f64; 2]> = c["points"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| [p[1].as_f64().unwrap(), p[2].as_f64().unwrap()])
                    .collect();
                let mut series = cube.series(&part, affine, None, &points).expect(&name);
                for r in series.runs.clone() {
                    series.put(
                        r.offset,
                        bytes[r.offset as usize..(r.offset + r.len) as usize].to_vec(),
                    );
                }
                let got = series.values().expect(&name);
                let want = c["expect"]["values"].as_array().unwrap();
                assert_eq!(got.len(), want.len(), "{name}: steps");
                for (st, (g, w)) in got.iter().zip(want).enumerate() {
                    for (k, (a, e)) in g.iter().zip(w.as_array().unwrap()).enumerate() {
                        assert!(
                            value_matches(*a, e, false),
                            "{name}: step {st} point {k} is {a}, wants {e}"
                        );
                    }
                }
                let times: Vec<f64> = c["expect"]["times"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect();
                assert_eq!(series.times, times, "{name}: times");
            }
            // Kesit and Mesh hesaplayıcı are the raster core's (crates/shared/raster/tests/all/multidim.rs).
            Some("profile" | "calc") => continue,
            other => panic!("{name}: unknown kind {other:?}"),
        }
        played += 1;
    }
    assert!(played >= 50, "only {played} cases");
}

#[test]
fn a_reader_of_every_slice_shares_the_mesh() {
    let bytes = file("ugrid.nc");
    let mut cube = Cube::from_bytes(&bytes).expect("cube");
    let base = Part {
        variable: "depth".into(),
        vector: None,
        mesh: Some("mesh".into()),
        slice: vec![0],
        affine: [500000.0, 0.125, 0.0, 4420040.0, 0.0, -0.125],
        width: 320,
        height: 320,
    };
    let a = cube.open(&base, 1 << 20).expect("slice 0");
    let b = cube
        .open(
            &Part {
                slice: vec![2],
                ..base.clone()
            },
            1 << 20,
        )
        .expect("slice 2");
    let (ma, mb) = (a.mesh_levels().unwrap(), b.mesh_levels().unwrap());
    assert!(
        std::sync::Arc::ptr_eq(&ma.mesh, &mb.mesh),
        "the mesh is read once"
    );
    // A slice past the dimension is refused with its reason.
    let err = cube
        .open(
            &Part {
                slice: vec![3],
                ..base.clone()
            },
            1 << 20,
        )
        .expect_err("refused");
    assert_eq!(err.0, "“time” boyutunun 3 değeri var; 4. değer yok.");
    let err = cube
        .open(
            &Part {
                slice: vec![],
                ..base
            },
            1 << 20,
        )
        .expect_err("refused");
    assert_eq!(
        err.0,
        "“depth” değişkeninin 1 dilim boyutu var; 0 değer verildi."
    );
}
