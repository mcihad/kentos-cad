//! Çok boyutlu veri's tools (docs/adr/0243 §8–§10) against
//! fixtures/multidim/v1/cases.json (scripts/fixtures/multidim_cases.py): Kesit's
//! points and values, Zaman serisi's table, Mesh hesaplayıcı's file written
//! byte for byte as the reference's own UGRID writer writes it, and its refusals.

use kentos_formats::multidim::cube::{Cube, Part, Want};
use kentos_formats::raster::source::Reader;
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::vec2::Vec2;
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::multidim::calc::{CalcSpec, MeshCalc};
use kentos_raster::multidim::series::{NamedPoint, TimeJob};
use kentos_raster::multidim::{Finished, MultidimJob, profile};
use serde_json::Value;

use crate::host::read;

fn cases() -> Vec<Value> {
    let text = String::from_utf8(read("multidim/v1/cases.json")).expect("UTF-8");
    let v: Value = serde_json::from_str(&text).expect("JSON");
    v["cases"].as_array().expect("cases").clone()
}

fn bytes_of(c: &Value) -> Vec<u8> {
    read(&format!(
        "multidim/v1/files/{}",
        c["file"].as_str().unwrap()
    ))
}

fn part_of(c: &Value) -> Part {
    let text = serde_json::to_string(&{
        let p = &c["part"];
        let mut o = p.clone();
        if let (Some(w), Some(h)) = (p["width"].as_u64(), p["height"].as_u64()) {
            o["size"] = serde_json::json!([w, h]);
        }
        o
    })
    .unwrap();
    Part::from_json(&text).expect("part")
}

/// The cube of a file with what `part` needs read.
fn cube_for(bytes: &[u8], part: &Part) -> Cube {
    let mut cube = Cube::from_bytes(bytes).expect("cube");
    for n in cube.needs(Want::Part(part)) {
        cube.put(
            n.offset,
            bytes[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
        );
    }
    cube
}

/// A raster's place: a grid's own, a mesh's virtual grid.
fn affine_of(cube: &mut Cube, part: &Part) -> [f64; 6] {
    cube.info()
        .expect("info")
        .grids
        .iter()
        .find(|g| g.variable == part.variable)
        .and_then(|g| g.affine)
        .unwrap_or(part.affine)
}

/// A job run to its end over the file's bytes.
fn run(mut job: MultidimJob, bytes: &[u8]) -> (Finished, Vec<u8>) {
    let mut guard = 0;
    while !job.done() {
        for (k, (at, len)) in job.needs().expect("needs").into_iter().enumerate() {
            let piece = bytes[at as usize..(at + len) as usize].to_vec();
            assert!(job.put(k, piece).expect("put").is_none(), "no JPEG here");
        }
        job.step().expect("step");
        guard += 1;
        assert!(guard < 100_000, "the job never ends");
    }
    let mut out = Vec::new();
    let finished = job
        .finish(&mut |b: &[u8]| {
            out.extend_from_slice(b);
            Ok(())
        })
        .expect("finish");
    (finished, out)
}

fn matches(actual: f64, expected: &Value) -> bool {
    match expected.as_f64() {
        None => actual.is_nan(),
        Some(e) => (actual - e).abs() <= 1e-12 * e.abs().max(1.0),
    }
}

#[test]
fn kesit_reads_every_point_as_the_reference() {
    let mut played = 0;
    for c in cases().iter().filter(|c| c["kind"] == "profile") {
        let name = c["name"].as_str().unwrap();
        let bytes = bytes_of(c);
        let part = part_of(c);
        let mut cube = cube_for(&bytes, &part);
        let affine = affine_of(&mut cube, &part);
        let reader: Reader = cube.open(&part, READER_BUDGET).expect(name);
        let input = Input::new(reader, affine, None).expect(name);
        let lines: Vec<Shape> = c["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| Shape::Polyline {
                pts: l
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| Vec2::new(p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
                    .collect(),
                bulges: None,
                holes: None,
                parts: None,
            })
            .collect();
        let step = c["step"].as_f64().unwrap();
        let mut job = profile::job(input, &lines, step, 0, ["Y".into(), "X".into()]).expect(name);
        let e = &c["expect"];
        let want_st = e["stations"].as_array().unwrap();
        // The points first, then the values (read by the job's own batches).
        while !job.done() {
            let needs = job.needs();
            for (k, (at, len)) in needs.into_iter().enumerate() {
                assert!(
                    job.put(k, &bytes[at as usize..(at + len) as usize])
                        .expect(name)
                        .is_none()
                );
            }
            job.step().expect(name);
        }
        let values = job.values.values().to_vec();
        let want = e["values"].as_array().unwrap();
        assert_eq!(values.len(), want.len(), "{name}: points");
        for (k, (a, w)) in values.iter().zip(want).enumerate() {
            assert!(matches(*a, w), "{name}: point {k} is {a}, wants {w}");
        }
        let finished = job.finish();
        let table = finished.table.as_ref().expect("table");
        assert_eq!(
            table.columns,
            ["Çizgi", "Uzaklık (m)", "Y", "X", "Değer"],
            "{name}"
        );
        assert_eq!(table.rows.len(), want_st.len(), "{name}: rows");
        for (k, (row, st)) in table.rows.iter().zip(want_st).enumerate() {
            let st: Vec<f64> = st
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            assert_eq!(row[0], (st[0] as u32).to_string(), "{name}: row {k}");
            for (cell, want) in row[1..4].iter().zip(&st[1..4]) {
                let got: f64 = cell.parse().unwrap();
                assert!(
                    (got - want).abs() <= 0.0005 + 1e-9,
                    "{name}: row {k} has {cell}, wants {want}"
                );
            }
            let v = values[k];
            assert_eq!(
                row[4],
                if v.is_finite() {
                    fixed(v, 3)
                } else {
                    String::new()
                },
                "{name}: row {k}"
            );
        }
        assert_eq!(finished.summary, e["summary"].as_str().unwrap(), "{name}");
        // The lines drawn with values break where a point has none.
        for piece in &finished.pieces {
            assert!(
                piece.points.len() >= 2 && piece.points.iter().all(|p| p[2].is_finite()),
                "{name}"
            );
        }
        played += 1;
    }
    assert_eq!(played, 5);
}

#[test]
fn zaman_serisi_gives_the_reference_table() {
    let mut played = 0;
    for c in cases().iter().filter(|c| c["kind"] == "series") {
        let name = c["name"].as_str().unwrap();
        let bytes = bytes_of(c);
        let part = part_of(c);
        let mut cube = cube_for(&bytes, &part);
        let affine = affine_of(&mut cube, &part);
        let points: Vec<NamedPoint> = c["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| NamedPoint {
                name: p[0].as_str().map(str::to_owned),
                x: p[1].as_f64().unwrap(),
                y: p[2].as_f64().unwrap(),
            })
            .collect();
        let xy: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
        let series = cube.series(&part, affine, None, &xy).expect(name);
        let job = MultidimJob::Series(Box::new(TimeJob::new(series, &points).expect(name)));
        let (finished, _) = run(job, &bytes);
        let e = &c["expect"];
        let table = finished.table.expect("table");
        let names: Vec<String> = e["names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        let mut columns = vec!["Adım".to_owned(), "Zaman".to_owned()];
        columns.extend(names);
        assert_eq!(table.columns, columns, "{name}");
        let labels = e["labels"].as_array().unwrap();
        for (st, (row, want)) in table
            .rows
            .iter()
            .zip(e["values"].as_array().unwrap())
            .enumerate()
        {
            assert_eq!(row[0], (st + 1).to_string(), "{name}");
            assert_eq!(row[1], labels[st].as_str().unwrap(), "{name}");
            for (cell, w) in row[2..].iter().zip(want.as_array().unwrap()) {
                let text = w.as_f64().map(|v| fixed(v, 3)).unwrap_or_default();
                assert_eq!(cell, &text, "{name}: step {st}");
            }
        }
        assert_eq!(finished.summary, e["summary"].as_str().unwrap(), "{name}");
        played += 1;
    }
    assert_eq!(played, 6);
}

#[test]
fn mesh_hesaplayici_writes_the_reference_file() {
    let mut played = 0;
    for c in cases().iter().filter(|c| c["kind"] == "calc") {
        let name = c["name"].as_str().unwrap();
        let bytes = bytes_of(c);
        let part = part_of(c);
        let mut cube = cube_for(&bytes, &part);
        let spec: CalcSpec = serde_json::from_value(c["spec"].clone()).expect(name);
        let e = &c["expect"];
        match MeshCalc::new(&mut cube, &part, &spec) {
            Err(why) => assert_eq!(Some(why.as_str()), e["error"].as_str(), "{name}"),
            Ok(calc) => {
                assert!(e["error"].is_null(), "{name}: not refused");
                assert_eq!(calc.output().0, e["variable"].as_str().unwrap(), "{name}");
                let (finished, out) = run(MultidimJob::Calc(Box::new(calc)), &bytes);
                let want = read(&format!(
                    "multidim/v1/files/{}",
                    e["file"].as_str().unwrap()
                ));
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
                assert_eq!(finished.summary, e["summary"].as_str().unwrap(), "{name}");
                // The file reads back as a mesh with the new dataset.
                let mut back = Cube::from_bytes(&out).expect(name);
                let info = back.info().expect(name);
                assert_eq!(
                    info.meshes[0].datasets[0].variable,
                    e["variable"].as_str().unwrap(),
                    "{name}"
                );
            }
        }
        played += 1;
    }
    assert_eq!(played, 9);
}
