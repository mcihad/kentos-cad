//! Hidroloji's runs end to end (docs/adr/0235): every case of
//! `scripts/fixtures/hydrology_cases.py` (KentOS code not used there; its
//! fills, directions and basins cross-checked with GRASS) run as a host runs
//! it, on one thread and on three: the DEM written as a tiled GeoTIFF, its
//! blocks handed over, the steps taken (the share never going down), the
//! raster read back or the objects compared, and the notes.

use kentos_contracts::RasterSample;
use kentos_formats::raster::source::open_bytes;
use kentos_geometry_core::entity::Shape;
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::ops::{Notes, OpsFinished, OpsJob, OpsSpec};
use kentos_raster::vector::{FeatureKind, Features};
use serde_json::{Value, json};

use crate::raster_ops::{Raster, number, sample_of, tiff};

/// What a run gave.
enum Ran {
    Raster(u32, u32, Vec<f64>),
    Features(Features),
}

pub(crate) fn raster_of(r: &Value) -> Raster {
    let a: Vec<f64> = r["affine"]
        .as_array()
        .expect("affine")
        .iter()
        .map(number)
        .collect();
    Raster {
        affine: [a[0], a[1], a[2], a[3], a[4], a[5]],
        width: r["width"].as_u64().expect("width") as u32,
        height: r["height"].as_u64().expect("height") as u32,
        bands: r["bands"].as_u64().expect("bands") as u32,
        sample: sample_of(r["sample"].as_str().unwrap_or("f32")),
        nodata: match &r["nodata"] {
            Value::String(s) if s == "nan" => Some(f64::NAN),
            Value::Number(n) => n.as_f64(),
            _ => None,
        },
        alpha: r["alpha"].as_bool().unwrap_or(false),
        values: r["values"]
            .as_array()
            .expect("values")
            .iter()
            .map(number)
            .collect(),
    }
}

pub(crate) fn shapes_of(c: &Value) -> Vec<Shape> {
    c["shapes"]
        .as_array()
        .expect("shapes")
        .iter()
        .map(|s| {
            let j = kentos_geometry_core::api::json::Json::parse(&s.to_string()).expect("a shape");
            <kentos_geometry_core::entity::Entity as kentos_geometry_core::api::json::FromJson>::from_json(&j)
                .expect("an entity")
                .shape
        })
        .collect()
}

/// The run as a host runs it.
fn run(
    r: &Raster,
    tool: Value,
    geographic: bool,
    shapes: Vec<Shape>,
    threads: usize,
) -> Result<(Ran, Notes), String> {
    let file = tiff(r);
    let spec: OpsSpec = serde_json::from_value(json!({
        "tool": tool,
        "inputs": [{ "affine": r.affine, "name": "A" }],
        "geographic": geographic,
    }))
    .map_err(|e| e.to_string())?;
    let reader = open_bytes(&file, None, READER_BUDGET).map_err(|e| e.0)?;
    let input = Input::new(reader, r.affine, None)?;
    let (mut job, header) = OpsJob::new(vec![input], &spec, shapes, threads)?;
    let raster = job.result().is_some();
    let mut out = header;
    let mut last = -1.0;
    while !job.done() {
        let blocks = job
            .needs()
            .into_iter()
            .map(|(k, n)| {
                (
                    k,
                    n,
                    file[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
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
            let mut reader = open_bytes(&out, None, READER_BUDGET).map_err(|e| e.0)?;
            let (w, h) = (reader.levels[0].width, reader.levels[0].height);
            for need in reader.needs(0, 0, 0, w, h) {
                let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
                reader.put_block(&need, &out[a..b]).map_err(|e| e.0)?;
            }
            let reg = reader.region(0, 0, 0, w, h).ok_or("the level")?;
            let n = (w * h) as usize;
            Ok((
                Ran::Raster(w, h, (0..n).map(|k| reg.samples.get(k)).collect()),
                notes,
            ))
        }
        OpsFinished::Features(f) => Ok((Ran::Features(f), notes)),
        _ => Err("neither a raster nor objects".into()),
    }
}

/// Units in the last place between two float32 values (as float64s).
fn ulps32(a: f64, b: f64) -> u64 {
    let key = |v: f64| {
        let i = (v as f32).to_bits() as i64;
        if i < 0 { i64::from(i32::MIN) - i } else { i }
    };
    (key(a) - key(b)).unsigned_abs()
}

/// A sample against the reference's (null: none) by the case's rule.
fn meets(got: f64, want: &Value, rule: &str) -> bool {
    match want.as_f64() {
        None => got.is_nan(),
        Some(w) if rule == "f32ulp" => ulps32(got, w) <= 1,
        Some(w) => got.to_bits() == w.to_bits() || got == w,
    }
}

pub(crate) fn numbers(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}

pub(crate) fn check_features(
    name: &str,
    f: &Features,
    want: &Value,
    rule: &str,
) -> Result<(), String> {
    let kind = match f.kind {
        FeatureKind::Polygons => "polygons",
        FeatureKind::Lines => "lines",
        FeatureKind::Points => "points",
    };
    let fields: Vec<String> = numbers(&want["fields"])
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_owned())
        .collect();
    let ints = |k: &str| -> Vec<u64> {
        numbers(&want[k])
            .iter()
            .map(|v| v.as_u64().unwrap_or(u64::MAX))
            .collect()
    };
    let same_floats = |got: &[f64], w: &[Value], tol: &dyn Fn(usize) -> f64| {
        got.len() == w.len()
            && got
                .iter()
                .zip(w)
                .enumerate()
                .all(|(k, (g, w))| match w.as_f64() {
                    Some(x) => *g == x || (g - x).abs() <= tol(k),
                    None => g.is_nan(),
                })
    };
    let stride = fields.len().max(1);
    // Km (the route's reading) is the geometry core's, the reference's straight-line one: 1 nm apart at most.
    let km = fields.iter().position(|f| f == "Km");
    let tol = |k: usize| {
        if rule == "km" && Some(k % stride) == km {
            1e-9
        } else {
            0.0
        }
    };
    let checks = [
        ("kind", kind == want["kind"].as_str().unwrap_or("")),
        (
            "fields",
            f.fields.iter().map(|s| s.to_string()).collect::<Vec<_>>() == fields,
        ),
        (
            "numbers",
            same_floats(&f.numbers, &numbers(&want["numbers"]), &tol),
        ),
        (
            "rings",
            f.rings.iter().map(|&t| u64::from(t)).collect::<Vec<_>>() == ints("rings"),
        ),
        (
            "sizes",
            f.sizes.iter().map(|&t| u64::from(t)).collect::<Vec<_>>() == ints("sizes"),
        ),
        ("xy", same_floats(&f.xy, &numbers(&want["xy"]), &|_| 0.0)),
    ];
    match checks.iter().find(|(_, ok)| !ok) {
        None => Ok(()),
        Some((what, _)) => Err(format!(
            "{name}: {what} differ: got {:?} / {:?}",
            &f.numbers[..f.numbers.len().min(24)],
            &f.sizes[..f.sizes.len().min(12)]
        )),
    }
}

fn check_notes(name: &str, notes: &Notes, want: &Value) -> Result<(), String> {
    let h = &notes.hydro;
    let list = |k: &str| -> Vec<u32> {
        numbers(&want[k])
            .iter()
            .map(|v| v.as_u64().unwrap_or(0) as u32)
            .collect()
    };
    let mut bad = Vec::new();
    if let Some(c) = want.get("cells")
        && c.as_u64() != Some(h.cells)
    {
        bad.push(format!("cells {} ≠ {c}", h.cells));
    }
    if let Some(c) = want.get("empty") {
        if c.is_array() {
            if list("empty") != h.empty_points {
                bad.push(format!("empty points {:?} ≠ {c}", h.empty_points));
            }
        } else if c.as_u64() != Some(h.empty) {
            bad.push(format!("empty {} ≠ {c}", h.empty));
        }
    }
    if want.get("skipped").is_some() && list("skipped") != h.skipped {
        bad.push(format!("skipped {:?}", h.skipped));
    }
    if let Some(d) = want.get("dropped")
        && d.as_u64() != Some(h.dropped)
    {
        bad.push(format!("dropped {} ≠ {d}", h.dropped));
    }
    if let Some(t) = want.get("threshold")
        && t.as_f64() != Some(h.threshold)
    {
        bad.push(format!("threshold {} ≠ {t}", h.threshold));
    }
    if let Some(l) = want.get("links")
        && l.as_u64() != Some(h.links)
    {
        bad.push(format!("links {} ≠ {l}", h.links));
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("{name}: notes: {}", bad.join(", ")))
    }
}

/// Every case of the reference, on one thread and on three.
#[test]
fn hydrology_cases() {
    let doc: Value =
        serde_json::from_slice(&crate::host::read("hydrology/v1/cases.json")).expect("the cases");
    let mut failures = Vec::new();
    let mut checked = 0;
    for c in doc["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap_or("?");
        let r = raster_of(&c["input"]);
        let expect = &c["expect"];
        let geographic = c["geographic"].as_bool().unwrap_or(false);
        for threads in [1, 3] {
            let got = run(&r, c["tool"].clone(), geographic, shapes_of(c), threads);
            match (got, expect["refused"].as_str()) {
                (Err(e), Some(why)) if e.contains(why) => {}
                (Err(e), _) => failures.push(format!("{name} ({threads}): {e}")),
                (Ok(_), Some(why)) => {
                    failures.push(format!("{name}: ran, should be refused ({why})"))
                }
                (Ok((Ran::Raster(w, h, v), notes)), None) => {
                    let want = &expect["raster"];
                    let rule = want["rule"].as_str().unwrap_or("exact");
                    let vals = numbers(&want["values"]);
                    if (u64::from(w), u64::from(h))
                        != (
                            want["width"].as_u64().unwrap_or(0),
                            want["height"].as_u64().unwrap_or(0),
                        )
                    {
                        failures.push(format!("{name}: {w} × {h}"));
                    } else if let Some(off) =
                        (0..vals.len()).find(|&k| !meets(v[k], &vals[k], rule))
                    {
                        failures.push(format!(
                            "{name} ({threads}): cell {off}: {} for {}",
                            v[off], vals[off]
                        ));
                    } else if let Some(n) = expect.get("notes")
                        && let Err(e) = check_notes(name, &notes, n)
                    {
                        failures.push(e);
                    } else {
                        checked += 1;
                    }
                }
                (Ok((Ran::Features(f), notes)), None) => {
                    let rule = expect["rule"].as_str().unwrap_or("exact");
                    match check_features(name, &f, &expect["features"], rule)
                        .and_then(|()| check_notes(name, &notes, &expect["notes"]))
                    {
                        Ok(()) => checked += 1,
                        Err(e) => failures.push(e),
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(checked >= 250, "{checked}");
    let _ = RasterSample::F32;
}
