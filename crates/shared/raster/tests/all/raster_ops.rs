//! Raster işlemleri's runs end to end (docs/adr/0233): small rasters
//! written as tiled GeoTIFFs in memory, each tool's job run over them as a
//! host runs it (blocks asked for and handed over), the result read back.
//! The independent reference's cases are `ops_cases`; these are the
//! rules' plain checks.

use kentos_contracts::RasterSample;
use kentos_formats::raster::TILE;
use kentos_formats::raster::samples::Samples;
use kentos_formats::raster::source::{Reader, open_bytes};
use kentos_formats::raster::write::Geo;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::vec2::Vec2;
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::ops::{OpsFinished, OpsJob, OpsSpec};
use kentos_raster::out::{Out, OutSpec, Rows};
use serde_json::{Value, json};

/// A raster to write: its place, size, samples (bands interleaved), nodata and alpha.
pub struct Raster {
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub sample: RasterSample,
    pub nodata: Option<f64>,
    pub alpha: bool,
    pub values: Vec<f64>,
}

impl Raster {
    pub fn f32(affine: [f64; 6], width: u32, height: u32, values: Vec<f64>) -> Raster {
        Raster {
            affine,
            width,
            height,
            bands: 1,
            sample: RasterSample::F32,
            nodata: Some(f64::NAN),
            alpha: false,
            values,
        }
    }
}

/// The raster as a tiled GeoTIFF's bytes.
pub fn tiff(r: &Raster) -> Vec<u8> {
    let (mut out, header) = Out::new(
        OutSpec {
            width: r.width,
            height: r.height,
            bands: r.bands,
            sample: r.sample,
            alpha: r.alpha,
            nodata: r.nodata,
            geo: Geo {
                affine: r.affine,
                epsg: None,
                geographic: false,
            },
        },
        2,
    )
    .expect("a writer");
    let mut file = header;
    let mut j0 = 0;
    let row = (r.width * r.bands) as usize;
    while j0 < r.height {
        let n = TILE.min(r.height - j0);
        let mut s = Samples::filled(r.sample, n as usize * row, 0.0);
        for k in 0..n as usize * row {
            s.set(k, r.values[j0 as usize * row + k]);
        }
        file.extend(out.push(Rows::Any(&s), n).expect("a strip"));
        j0 += n;
    }
    let (tail, header) = out.finish().expect("finished");
    file.extend(tail);
    file[..header.len()].copy_from_slice(&header);
    file
}

fn opened(bytes: &[u8]) -> Reader {
    open_bytes(bytes, None, READER_BUDGET).expect("the raster opens")
}

/// What a run gives.
pub enum Got {
    /// The result's width, height, bands and level 0, bands interleaved.
    Raster(u32, u32, u32, Vec<f64>, [f64; 6]),
    Done(OpsFinished),
}

/// `tool` run over the rasters (in the run's order) and the shapes on `threads`.
pub fn run(
    rasters: &[Raster],
    tool: Value,
    shapes: Vec<Shape>,
    threads: usize,
) -> Result<Got, String> {
    let names: Vec<String> = (0..rasters.len())
        .map(|k| NAMES[k.min(5)].to_owned())
        .collect();
    run_named(rasters, &names, tool, shapes, threads)
}

/// `run` with the inputs' names.
pub fn run_named(
    rasters: &[Raster],
    names: &[String],
    tool: Value,
    shapes: Vec<Shape>,
    threads: usize,
) -> Result<Got, String> {
    let files: Vec<Vec<u8>> = rasters.iter().map(tiff).collect();
    let spec: OpsSpec = serde_json::from_value(json!({
        "tool": tool,
        "inputs": rasters.iter().zip(names).map(|(r, name)| json!({
            "affine": r.affine,
            "name": name,
        })).collect::<Vec<_>>(),
    }))
    .map_err(|e| e.to_string())?;
    let inputs = rasters
        .iter()
        .zip(&files)
        .map(|(r, f)| Input::new(opened(f), r.affine, None))
        .collect::<Result<Vec<_>, _>>()?;
    let (mut job, header) = OpsJob::new(inputs, &spec, shapes, threads)?;
    let raster = job.result().is_some();
    let grid = job.grid();
    let mut file = header;
    let mut last = -1.0;
    while !job.done() {
        let blocks = job
            .needs()
            .into_iter()
            .map(|(k, n)| {
                let f = &files[k as usize];
                (
                    k,
                    n,
                    f[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
                )
            })
            .collect();
        if !job.put_all(blocks)?.is_empty() {
            return Err("a JPEG block in a test".into());
        }
        file.extend(job.step()?);
        let share = job.share();
        assert!(share >= last, "the share goes up ({last} → {share})");
        last = share;
    }
    match job.finish()? {
        OpsFinished::Raster { tail, header } => {
            assert!(raster);
            file.extend(tail);
            file[..header.len()].copy_from_slice(&header);
            let mut reader = opened(&file);
            let (w, h) = (reader.levels[0].width, reader.levels[0].height);
            for need in reader.needs(0, 0, 0, w, h) {
                let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
                reader.put_block(&need, &file[a..b]).expect("a block");
            }
            let r = reader.region(0, 0, 0, w, h).expect("the level");
            let n = (w * h * r.bands) as usize;
            Ok(Got::Raster(
                w,
                h,
                r.bands,
                (0..n).map(|k| r.samples.get(k)).collect(),
                grid.affine,
            ))
        }
        other => Ok(Got::Done(other)),
    }
}

fn raster_of(g: Got) -> (u32, u32, u32, Vec<f64>, [f64; 6]) {
    match g {
        Got::Raster(w, h, b, v, a) => (w, h, b, v, a),
        Got::Done(_) => panic!("a raster was wanted"),
    }
}

/// The inputs' names in a run, in order.
const NAMES: [&str; 6] = ["A", "B", "C", "D", "E", "F"];

const PLACE: [f64; 6] = [1000.0, 2.0, 0.0, 2000.0, 0.0, -2.0];

fn ramp(w: u32, h: u32) -> Vec<f64> {
    (0..w * h)
        .map(|k| f64::from(k % w) + 10.0 * f64::from(k / w))
        .collect()
}

fn same(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| (x.is_nan() && y.is_nan()) || x == y)
}

#[test]
fn the_calculator_reads_two_rasters_cell_by_cell() {
    let a = Raster::f32(PLACE, 5, 4, ramp(5, 4));
    let mut bv = vec![1.0; 20];
    bv[7] = f64::NAN;
    // B two cells east of A: A's cell (i, j) is B's (i + 2, j).
    let b = Raster::f32([996.0, 2.0, 0.0, 2000.0, 0.0, -2.0], 5, 4, bv);
    let tool = json!({ "kind": "calculator", "expression": "[A] * 2 + [B]", "empty": "propagate", "sample": "f32" });
    for threads in [1, 3] {
        let (w, h, bands, v, affine) =
            raster_of(run(&[a_copy(&a), a_copy(&b)], tool.clone(), vec![], threads).unwrap());
        assert_eq!((w, h, bands, affine), (5, 4, 1, PLACE));
        for j in 0..4 {
            for i in 0..5 {
                let k = (j * 5 + i) as usize;
                let want = if i + 2 >= 5 || (j * 5 + i + 2) == 7 {
                    f64::NAN
                } else {
                    2.0 * (f64::from(i) + 10.0 * f64::from(j)) + 1.0
                };
                assert!(same(&[v[k]], &[want]), "({i}, {j}): {} vs {want}", v[k]);
            }
        }
    }
}

fn a_copy(r: &Raster) -> Raster {
    Raster {
        affine: r.affine,
        width: r.width,
        height: r.height,
        bands: r.bands,
        sample: r.sample,
        nodata: r.nodata,
        alpha: r.alpha,
        values: r.values.clone(),
    }
}

#[test]
fn reclassify_by_its_table() {
    let a = Raster::f32(PLACE, 5, 4, ramp(5, 4));
    let tool = json!({ "kind": "reclassify", "band": 1, "table": "* 5 1; 5 20 2; 30 boş", "bounds": "upperClosed", "unmatched": "keep", "sample": "i32" });
    let (_, _, _, v, _) = raster_of(run(&[a], tool, vec![], 2).unwrap());
    let want: Vec<f64> = ramp(5, 4)
        .into_iter()
        .map(|x| {
            if x <= 5.0 {
                1.0
            } else if x <= 20.0 {
                2.0
            } else if x == 30.0 {
                -2_147_483_648.0
            } else {
                x
            }
        })
        .collect();
    assert!(same(&v, &want), "{v:?}");
}

#[test]
fn a_mask_keeps_the_cells_inside_it_and_crops() {
    let a = Raster::f32(PLACE, 10, 8, ramp(10, 8));
    // x 1004..1010, y 1990..1996: cells i 2..5, rows 2..5 (centres x 1005, 1007, 1009; y 1995, 1993, 1991).
    let mask = Shape::Polygon {
        pts: vec![
            Vec2::new(1004.0, 1990.0),
            Vec2::new(1010.0, 1990.0),
            Vec2::new(1010.0, 1996.0),
            Vec2::new(1004.0, 1996.0),
        ],
        bulges: None,
        holes: None,
        parts: None,
    };
    let tool = json!({ "kind": "clipByMask", "crop": true });
    let (w, h, _, v, affine) = raster_of(run(&[a], tool, vec![mask], 2).unwrap());
    assert_eq!((w, h), (3, 3));
    assert_eq!(affine, [1004.0, 2.0, 0.0, 1996.0, 0.0, -2.0]);
    let want: Vec<f64> = (2..5)
        .flat_map(|j| (2..5).map(move |i| f64::from(i) + 10.0 * f64::from(j)))
        .collect();
    assert_eq!(v, want);
}

#[test]
fn a_mosaic_takes_the_top_raster_first() {
    let top = Raster::f32(PLACE, 4, 4, vec![1.0; 16]);
    // Two cells east and one south, overlapping the top's lower right.
    let low = Raster::f32([1004.0, 2.0, 0.0, 1998.0, 0.0, -2.0], 4, 4, vec![2.0; 16]);
    let tool = json!({ "kind": "mosaic", "overlap": "top", "sampling": "nearest" });
    let (w, h, _, v, affine) = raster_of(run(&[top, low], tool, vec![], 2).unwrap());
    assert_eq!((w, h), (6, 5));
    assert_eq!(affine, PLACE);
    for j in 0..5usize {
        for i in 0..6usize {
            let want = if i < 4 && j < 4 {
                1.0
            } else if i >= 2 && j >= 1 {
                2.0
            } else {
                f64::NAN
            };
            assert!(same(&[v[j * 6 + i]], &[want]), "({i}, {j})");
        }
    }
}

#[test]
fn resampling_by_the_mean_of_the_overlapped_cells() {
    let a = Raster::f32(PLACE, 4, 4, ramp(4, 4));
    let tool = json!({ "kind": "resample", "cell": 4.0, "method": "mean" });
    let (w, h, _, v, affine) = raster_of(run(&[a], tool, vec![], 2).unwrap());
    assert_eq!(
        (w, h, affine),
        (2, 2, [1000.0, 4.0, 0.0, 2000.0, 0.0, -4.0])
    );
    // Each new cell is a 2 × 2 block: mean of x + 10y over it.
    assert_eq!(v, vec![5.5, 7.5, 25.5, 27.5]);
}

#[test]
fn zonal_figures_by_area() {
    let a = Raster::f32(PLACE, 10, 8, ramp(10, 8));
    let zone = |x0: f64, y0: f64, x1: f64, y1: f64| Shape::Polygon {
        pts: vec![
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        ],
        bulges: None,
        holes: None,
        parts: None,
    };
    let zones = vec![
        zone(1000.0, 1996.0, 1004.0, 2000.0),
        zone(1100.0, 1100.0, 1102.0, 1102.0),
    ];
    let tool = json!({ "kind": "zonalStatistics", "band": 1, "stat": "mean" });
    let Got::Done(OpsFinished::Zones(z)) = run(&[a], tool, zones, 2).unwrap() else {
        panic!("zones");
    };
    // The first: cells (0, 0), (1, 0), (0, 1), (1, 1): 0, 1, 10, 11.
    assert_eq!(z[0].moments.n, 4);
    assert_eq!(z[0].value, Some(5.5));
    assert_eq!(z[1].moments.n, 0);
    assert_eq!(z[1].value, None);
}

#[test]
fn a_histogram_counts_each_interval() {
    let a = Raster::f32(PLACE, 10, 8, ramp(10, 8));
    let tool = json!({ "kind": "histogram", "band": 1, "bins": 8 });
    let Got::Done(OpsFinished::Histogram(h)) = run(&[a], tool, vec![], 2).unwrap() else {
        panic!("a histogram");
    };
    assert_eq!((h.lo, h.hi, h.valid, h.empty), (0.0, 79.0, 80, 0));
    assert_eq!(h.counts.iter().sum::<u64>(), 80);
}

#[test]
fn focal_and_cell_statistics() {
    let a = Raster::f32(PLACE, 6, 5, ramp(6, 5));
    let tool = json!({ "kind": "focalStatistics", "band": 1, "shape": "rect", "width": 3, "height": 3, "stat": "mean", "ignore": true });
    let (_, _, _, v, _) = raster_of(run(&[a_copy(&a)], tool, vec![], 2).unwrap());
    // An inner cell's 3 × 3 mean is its own value (the ramp is linear).
    assert_eq!(v[6 + 1], 1.0 + 10.0);
    // A corner's window holds four cells: 0, 1, 10, 11.
    assert_eq!(v[0], 5.5);
    let b = Raster::f32(PLACE, 6, 5, vec![100.0; 30]);
    let tool = json!({ "kind": "cellStatistics", "band": 1, "stat": "max", "ignore": true });
    let (_, _, _, v, _) = raster_of(run(&[a, b], tool, vec![], 2).unwrap());
    assert!(v.iter().all(|&x| x == 100.0));
}

// ── The independent reference's cases (scripts/fixtures/raster_ops_cases.py) ──

/// Raster hesaplayıcı reads only the rasters it names, in the order it names
/// them (§3), told before any is opened; a name that is no raster's is
/// refused with every input's name; every other tool reads all.
#[test]
fn a_calculator_reads_only_the_rasters_it_names() {
    let spec = |tool: Value| -> OpsSpec {
        let inputs: Vec<Value> = ["DEM", "Ortofoto", "DEM (2)"]
            .iter()
            .map(|n| json!({ "affine": [0.0, 1.0, 0.0, 0.0, 0.0, -1.0], "name": n }))
            .collect();
        serde_json::from_value(json!({ "tool": tool, "inputs": inputs })).expect("a spec")
    };
    let calc = |e: &str| {
        spec(
            json!({ "kind": "calculator", "expression": e, "empty": "propagate", "sample": "f32" }),
        )
    };
    assert_eq!(calc("[DEM (2)] - [DEM]").reads(), Ok(vec![2, 0]));
    assert_eq!(
        calc("([Ortofoto@4] - [Ortofoto@1]) / ([Ortofoto@4] + [Ortofoto@1])").reads(),
        Ok(vec![1])
    );
    assert_eq!(
        calc("durum eğer [DEM] > 100 ise [DEM] yoksa [DEM@1] son").reads(),
        Ok(vec![0])
    );
    assert_eq!(
        calc("[Yok] + [DEM]").reads(),
        Err("“Yok” adında raster bandı yok. Rasterler: [DEM], [Ortofoto], [DEM (2)]; bant @ ile: [Ad@2].".to_owned())
    );
    assert!(calc("[Ortofoto@x]").reads().is_err(), "not a band number");
    assert_eq!(
        calc("1 + 2").reads(),
        Err("İfade bir raster anmalı (örnek: [DEM] * 2).".to_owned())
    );
    let mosaic = spec(json!({ "kind": "mosaic", "overlap": "top", "sampling": "nearest" }));
    assert_eq!(mosaic.reads(), Ok(vec![0, 1, 2]));
    // The settings over those inputs only, in that order.
    let only = calc("[DEM (2)] - [DEM]").reading(&[2, 0]);
    let names: Vec<&str> = only.inputs.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["DEM (2)", "DEM"]);
}

fn sample_of(name: &str) -> RasterSample {
    match name {
        "u8" => RasterSample::U8,
        "i8" => RasterSample::I8,
        "u16" => RasterSample::U16,
        "i16" => RasterSample::I16,
        "u32" => RasterSample::U32,
        "i32" => RasterSample::I32,
        "f64" => RasterSample::F64,
        _ => RasterSample::F32,
    }
}

fn number(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// Whether `got` meets `want` by the case's rule (NaN only with NaN).
fn meets(got: f64, want: f64, rule: &str, sample: RasterSample) -> bool {
    if got.is_nan() || want.is_nan() {
        return got.is_nan() && want.is_nan();
    }
    if rule == "exact" {
        return got == want;
    }
    match sample {
        RasterSample::F32 => {
            let (a, b) = (got as f32, want as f32);
            a.is_sign_negative() == b.is_sign_negative() && a.to_bits().abs_diff(b.to_bits()) <= 1
        }
        RasterSample::F64 => {
            got.is_sign_negative() == want.is_sign_negative()
                && got.to_bits().abs_diff(want.to_bits()) <= 2
        }
        _ => (got - want).abs() <= 1.0,
    }
}

#[test]
fn ops_cases() {
    let doc: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{}raster-ops/v1/cases.json", crate::host::DIR))
            .expect("the cases"),
    )
    .expect("JSON");
    let cases = doc["cases"].as_array().expect("cases");
    let mut failures = Vec::new();
    let mut checked = 0usize;
    for c in cases {
        let name = c["name"].as_str().unwrap_or("?");
        let mut rasters = Vec::new();
        let mut names = Vec::new();
        for r in c["inputs"].as_array().expect("inputs") {
            let a: Vec<f64> = r["affine"]
                .as_array()
                .expect("affine")
                .iter()
                .map(number)
                .collect();
            let nodata = match &r["nodata"] {
                Value::String(s) if s == "nan" => Some(f64::NAN),
                Value::Number(n) => n.as_f64(),
                _ => None,
            };
            rasters.push(Raster {
                affine: [a[0], a[1], a[2], a[3], a[4], a[5]],
                width: r["width"].as_u64().expect("width") as u32,
                height: r["height"].as_u64().expect("height") as u32,
                bands: r["bands"].as_u64().expect("bands") as u32,
                sample: sample_of(r["sample"].as_str().unwrap_or("f32")),
                nodata,
                alpha: r["alpha"].as_bool().unwrap_or(false),
                values: r["values"]
                    .as_array()
                    .expect("values")
                    .iter()
                    .map(number)
                    .collect(),
            });
            names.push(r["name"].as_str().unwrap_or("A").to_owned());
        }
        let shapes: Vec<Shape> = c["shapes"]
            .as_array()
            .expect("shapes")
            .iter()
            .map(|s| {
                let j = kentos_geometry_core::api::json::Json::parse(&s.to_string()).expect("a shape");
                <kentos_geometry_core::entity::Entity as kentos_geometry_core::api::json::FromJson>::from_json(&j)
                    .expect("an entity")
                    .shape
            })
            .collect();
        let expect = &c["expect"];
        let runs: Vec<Result<Got, String>> = [1, 3]
            .iter()
            .map(|&t| run_named(&rasters, &names, c["tool"].clone(), shapes.clone(), t))
            .collect();
        if let Some(why) = expect["refused"].as_str() {
            match &runs[0] {
                Err(e) if e.contains(why) => {}
                Err(e) => failures.push(format!("{name}: refused with “{e}”, not “{why}”")),
                Ok(_) => failures.push(format!("{name}: ran, should be refused ({why})")),
            }
            continue;
        }
        let (first, second) = match (&runs[0], &runs[1]) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        match (first, second) {
            (Got::Raster(w, h, b, v, affine), Got::Raster(_, _, _, v3, _)) => {
                let want = &expect["raster"];
                let rule = want["rule"].as_str().unwrap_or("exact");
                let wa: Vec<f64> = want["affine"]
                    .as_array()
                    .expect("affine")
                    .iter()
                    .map(number)
                    .collect();
                if (*w, *h, *b)
                    != (
                        want["width"].as_u64().unwrap_or(0) as u32,
                        want["height"].as_u64().unwrap_or(0) as u32,
                        want["bands"].as_u64().unwrap_or(0) as u32,
                    )
                    || affine.to_vec() != wa
                {
                    failures.push(format!(
                        "{name}: {w} × {h} × {b} at {affine:?}, want {want}"
                    ));
                    continue;
                }
                let sample = rasters[0].sample;
                let out_sample = match c["tool"]["kind"].as_str() {
                    Some("calculator") => sample_of(c["tool"]["sample"].as_str().unwrap_or("f32")),
                    Some("reclassify") => sample_of(c["tool"]["sample"].as_str().unwrap_or("f32")),
                    Some("focalStatistics" | "cellStatistics") => {
                        if rasters.iter().any(|r| r.sample == RasterSample::F64) {
                            RasterSample::F64
                        } else {
                            RasterSample::F32
                        }
                    }
                    _ => sample,
                };
                let wv: Vec<f64> = want["values"]
                    .as_array()
                    .expect("values")
                    .iter()
                    .map(number)
                    .collect();
                if v.len() != wv.len() {
                    failures.push(format!("{name}: {} samples, want {}", v.len(), wv.len()));
                    continue;
                }
                checked += v.len();
                let wrong: Vec<String> = v
                    .iter()
                    .zip(&wv)
                    .enumerate()
                    .filter(|(_, (g, w))| !meets(**g, **w, rule, out_sample))
                    .take(5)
                    .map(|(k, (g, w))| format!("sample {k}: {g} ≠ {w}"))
                    .collect();
                if !wrong.is_empty() {
                    failures.push(format!("{name}: {}", wrong.join("; ")));
                }
                if !same(v, v3) {
                    failures.push(format!("{name}: one thread and three differ"));
                }
            }
            (Got::Done(OpsFinished::Zones(z)), Got::Done(OpsFinished::Zones(z3))) => {
                let want_zones = expect["zones"].as_array().map_or(0, Vec::len);
                if z.len() != want_zones {
                    failures.push(format!("{name}: {} zones, want {want_zones}", z.len()));
                    continue;
                }
                checked += z.len();
                for (k, (got, want)) in z
                    .iter()
                    .zip(expect["zones"].as_array().expect("zones"))
                    .enumerate()
                {
                    let rule = want["rule"].as_str().unwrap_or("exact");
                    let n = want["n"].as_u64().unwrap_or(0);
                    let wv = number(&want["value"]);
                    let gv = got.value.unwrap_or(f64::NAN);
                    if got.moments.n != n || !meets(gv, wv, rule, RasterSample::F64) {
                        failures.push(format!(
                            "{name}: zone {k}: n {} value {gv}, want n {n} value {wv}",
                            got.moments.n
                        ));
                    }
                }
                if z.iter()
                    .zip(z3.iter())
                    .any(|(a, b)| a.value.map(f64::to_bits) != b.value.map(f64::to_bits))
                {
                    failures.push(format!("{name}: one thread and three differ"));
                }
            }
            (Got::Done(OpsFinished::Histogram(h)), _) => {
                let want = &expect["histogram"];
                let counts: Vec<u64> = want["counts"]
                    .as_array()
                    .expect("counts")
                    .iter()
                    .map(|v| v.as_u64().unwrap_or(0))
                    .collect();
                if h.counts != counts
                    || h.lo != number(&want["lo"])
                    || h.hi != number(&want["hi"])
                    || (h.below, h.above, h.valid, h.empty)
                        != (
                            want["below"].as_u64().unwrap_or(0),
                            want["above"].as_u64().unwrap_or(0),
                            want["valid"].as_u64().unwrap_or(0),
                            want["empty"].as_u64().unwrap_or(0),
                        )
                {
                    failures.push(format!("{name}: {h:?}, want {want}"));
                }
            }
            _ => failures.push(format!("{name}: not the result wanted")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
    // Every case read: the samples and zones compared.
    assert!(
        cases.len() >= 70 && checked > 5_000,
        "{} cases, {checked} values",
        cases.len()
    );
}
