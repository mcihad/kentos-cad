//! Uzaklık ve maliyet's runs end to end (docs/adr/0236): every case of
//! `scripts/fixtures/distance_cases.py` (KentOS code not used there; its sums
//! cross-checked with GRASS's r.cost, its distances with r.grow.distance and
//! GDAL's Proximity) run as a host runs it, on one thread and on three: the
//! cost raster (and the surface) written as tiled GeoTIFFs, their blocks
//! handed over, the steps taken (the share never going down), the raster
//! read back or the paths compared, and the notes. Uzaklık yüzeyi from
//! objects runs as the point job.

use kentos_formats::raster::source::open_bytes;
use kentos_geometry_core::entity::Shape;
use kentos_raster::from_points::{PointInput, PointJob, PointSpec};
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::ops::{Notes, OpsFinished, OpsJob, OpsSpec};
use kentos_raster::vector::Features;
use serde_json::{Value, json};

use crate::hydrology::{check_features, numbers, raster_of, shapes_of};
use crate::raster_ops::{Raster, tiff};

enum Ran {
    Raster(u32, u32, Vec<f64>),
    Features(Features),
}

/// A GeoTIFF's level 0 read back.
fn read_back(file: &[u8]) -> Result<(u32, u32, Vec<f64>), String> {
    let mut reader = open_bytes(file, None, READER_BUDGET).map_err(|e| e.0)?;
    let (w, h) = (reader.levels[0].width, reader.levels[0].height);
    for need in reader.needs(0, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        reader.put_block(&need, &file[a..b]).map_err(|e| e.0)?;
    }
    let reg = reader.region(0, 0, 0, w, h).ok_or("the level")?;
    Ok((
        w,
        h,
        (0..(w * h) as usize).map(|k| reg.samples.get(k)).collect(),
    ))
}

/// The ops job as a host runs it: the cost raster first, the surface second.
fn run_ops(
    rasters: &[Raster],
    tool: Value,
    geographic: bool,
    shapes: Vec<Shape>,
    threads: usize,
) -> Result<(Ran, Notes), String> {
    let files: Vec<Vec<u8>> = rasters.iter().map(tiff).collect();
    let names = ["A", "B"];
    let spec: OpsSpec = serde_json::from_value(json!({
        "tool": tool,
        "inputs": rasters.iter().zip(names).map(|(r, n)| json!({ "affine": r.affine, "name": n })).collect::<Vec<_>>(),
        "geographic": geographic,
    }))
    .map_err(|e| e.to_string())?;
    let inputs = rasters
        .iter()
        .zip(&files)
        .map(|(r, f)| {
            let reader = open_bytes(f, None, READER_BUDGET).map_err(|e| e.0)?;
            Input::new(reader, r.affine, None)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let (mut job, header) = OpsJob::new(inputs, &spec, shapes, threads)?;
    let raster = job.result().is_some();
    let mut out = header;
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
        out.extend(job.step()?);
        let share = job.share();
        assert!(share >= last, "the share goes up ({last} → {share})");
        last = share;
    }
    let notes = job.notes().clone();
    match job.finish()? {
        OpsFinished::Raster { tail, header } => {
            assert!(raster);
            out.extend(tail);
            out[..header.len()].copy_from_slice(&header);
            let (w, h, v) = read_back(&out)?;
            Ok((Ran::Raster(w, h, v), notes))
        }
        OpsFinished::Features(f) => Ok((Ran::Features(f), notes)),
        _ => Err("neither a raster nor objects".into()),
    }
}

/// What the point job gave: the grid's size, the samples, the notes and the grid's place.
type PointsRan = (
    u32,
    u32,
    Vec<f64>,
    kentos_raster::from_points::Notes,
    [f64; 6],
);

/// Uzaklık yüzeyi from objects: the point job over the objects.
fn run_points(spec: &Value, shapes: Vec<Shape>, threads: usize) -> Result<PointsRan, String> {
    let spec: PointSpec = serde_json::from_value(spec.clone()).map_err(|e| e.to_string())?;
    let (mut job, header) = PointJob::new(
        PointInput::Lines {
            shapes,
            weights: None,
        },
        &spec,
        threads,
    )?;
    let affine = job.grid().affine;
    let mut file = header;
    let mut last = -1.0;
    while !job.done() {
        file.extend(job.step()?);
        let share = job.share();
        assert!(share >= last, "the share goes up ({last} → {share})");
        last = share;
    }
    let done = job.finish()?;
    file.extend(&done.tail);
    file[..done.header.len()].copy_from_slice(&done.header);
    let (w, h, v) = read_back(&file)?;
    Ok((w, h, v, done.notes, affine))
}

/// A number against the reference's (null: NaN) by the case's rule: bit
/// for bit, or `f32ulp` (a geographic grid's: a degree's metres come from sin
/// and cos, which libraries may round apart in the last bit) within one unit
/// in the last place of float32.
fn same(got: f64, want: &Value, rule: &str) -> bool {
    match want.as_f64() {
        Some(w) if rule == "f32ulp" => ulps32(got, w) <= 1,
        Some(w) => got == w,
        None => got.is_nan(),
    }
}

fn ulps32(a: f64, b: f64) -> u64 {
    let key = |v: f64| {
        let i = (v as f32).to_bits() as i32 as i64;
        if i < 0 { i64::from(i32::MIN) - i } else { i }
    };
    (key(a) - key(b)).unsigned_abs()
}

fn check_notes(name: &str, notes: &Notes, want: &Value, rule: &str) -> Result<(), String> {
    let d = &notes.distance;
    let mut bad = Vec::new();
    for (k, got) in [
        ("sources", d.sources),
        ("outside", d.outside),
        ("cells", d.cells),
    ] {
        if let Some(w) = want.get(k)
            && w.as_u64() != Some(got)
        {
            bad.push(format!("{k} {got} ≠ {w}"));
        }
    }
    for (k, got) in [("least", d.least), ("most", d.most)] {
        if let Some(w) = want.get(k)
            && !same(got, w, rule)
        {
            bad.push(format!("{k} {got} ≠ {w}"));
        }
    }
    if let Some(w) = want.get("unreached") {
        let list: Vec<u32> = numbers(w)
            .iter()
            .map(|v| v.as_u64().unwrap_or(0) as u32)
            .collect();
        if list != d.unreached {
            bad.push(format!("unreached {:?} ≠ {w}", d.unreached));
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("{name}: notes: {}", bad.join(", ")))
    }
}

/// The raster against the reference's values (null: none), bit for bit.
fn check_raster(name: &str, (w, h, v): (u32, u32, &[f64]), want: &Value) -> Result<(), String> {
    let vals = numbers(&want["values"]);
    if (u64::from(w), u64::from(h))
        != (
            want["width"].as_u64().unwrap_or(0),
            want["height"].as_u64().unwrap_or(0),
        )
    {
        return Err(format!("{name}: {w} × {h}"));
    }
    let rule = want["rule"].as_str().unwrap_or("exact");
    match (0..vals.len()).find(|&k| !same(v[k], &vals[k], rule)) {
        Some(k) => Err(format!("{name}: cell {k}: {} for {}", v[k], vals[k])),
        None => Ok(()),
    }
}

/// Every case of the reference, on one thread and on three.
#[test]
fn distance_cases() {
    let doc: Value =
        serde_json::from_slice(&crate::host::read("distance/v1/cases.json")).expect("the cases");
    let mut failures = Vec::new();
    let mut checked = 0;
    for c in doc["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap_or("?");
        let expect = &c["expect"];
        for threads in [1, 3] {
            let outcome = if c["job"] == "points" {
                run_points(&c["spec"], shapes_of(c), threads).and_then(
                    |(w, h, v, notes, affine)| {
                        check_raster(name, (w, h, &v), &expect["raster"])?;
                        let want: Vec<f64> = numbers(&expect["raster"]["affine"])
                            .iter()
                            .map(|x| x.as_f64().unwrap_or(f64::NAN))
                            .collect();
                        if affine.to_vec() != want {
                            return Err(format!("{name}: grid {affine:?}"));
                        }
                        let n = &expect["notes"];
                        let got = [
                            ("taken", notes.taken as u64),
                            ("outside", notes.outside as u64),
                            ("empty", notes.empty),
                        ];
                        match got
                            .iter()
                            .find(|(k, g)| n.get(*k).is_some_and(|w| w.as_u64() != Some(*g)))
                        {
                            Some((k, g)) => Err(format!("{name}: notes: {k} {g} ≠ {}", n[*k])),
                            None => Ok(()),
                        }
                    },
                )
            } else {
                let mut rasters = vec![raster_of(&c["input"])];
                if c["surface"].is_object() {
                    rasters.push(raster_of(&c["surface"]));
                }
                let geographic = c["geographic"].as_bool().unwrap_or(false);
                run_ops(
                    &rasters,
                    c["tool"].clone(),
                    geographic,
                    shapes_of(c),
                    threads,
                )
                .and_then(|(ran, notes)| {
                    match ran {
                        Ran::Raster(w, h, v) => check_raster(name, (w, h, &v), &expect["raster"])?,
                        Ran::Features(f) => check_features(name, &f, &expect["features"], "exact")?,
                    }
                    let rule = expect["raster"]["rule"].as_str().unwrap_or("exact");
                    check_notes(name, &notes, &expect["notes"], rule)
                })
            };
            match (outcome, expect["refused"].as_str()) {
                (Err(e), Some(why)) if e.contains(why) => checked += 1,
                (Err(e), _) => failures.push(format!("{name} ({threads}): {e}")),
                (Ok(()), Some(why)) => {
                    failures.push(format!("{name}: ran, should be refused ({why})"))
                }
                (Ok(()), None) => checked += 1,
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(checked, 2 * doc["cases"].as_array().map_or(0, Vec::len));
}
