//! Uzaktan algılama's runs end to end (docs/adr/0242): every case of
//! `scripts/fixtures/remote_cases.py` (KentOS code not used there; the
//! classifiers' discriminants from mpmath, Brovey cross-checked against
//! GDAL) run as a host runs it, on one thread and on three: the inputs
//! written as tiled GeoTIFFs, their blocks handed over, the steps taken (the
//! share never going down), the raster read back with its type and nodata,
//! the notes and Doğruluk analizi's table and figures compared; the two runs
//! give the same samples and notes.

use kentos_contracts::RasterSample;
use kentos_formats::raster::source::open_bytes;
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::ops::{Notes, OpsFinished, OpsJob, OpsSpec};
use kentos_raster::remote::Table;
use serde_json::{Value, json};

use crate::hydrology::{raster_of, shapes_of};
use crate::raster_ops::{number, sample_of, tiff};

/// A result raster read back.
struct Back {
    width: u32,
    height: u32,
    bands: u32,
    sample: RasterSample,
    nodata: Option<f64>,
    affine: [f64; 6],
    values: Vec<f64>,
}

enum Ran {
    Raster(Back),
    Report,
}

/// The ops job as a host runs it.
fn run(c: &Value, threads: usize) -> Result<(Ran, Notes), String> {
    let rasters: Vec<_> = c["inputs"]
        .as_array()
        .expect("inputs")
        .iter()
        .map(raster_of)
        .collect();
    let names: Vec<&str> = c["names"]
        .as_array()
        .expect("names")
        .iter()
        .map(|n| n.as_str().unwrap_or(""))
        .collect();
    let files: Vec<Vec<u8>> = rasters.iter().map(tiff).collect();
    let spec: OpsSpec = serde_json::from_value(json!({
        "tool": c["tool"],
        "inputs": rasters.iter().zip(&names).map(|(r, n)| json!({ "affine": r.affine, "name": n })).collect::<Vec<_>>(),
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
    let (mut job, header) = OpsJob::new(inputs, &spec, shapes_of(c), threads)?;
    let affine = job.grid().affine;
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
        job.put_all(blocks)?;
        out.extend(job.step()?);
        let share = job.share();
        assert!(share >= last, "the share goes up ({last} → {share})");
        last = share;
    }
    let notes = job.notes().clone();
    match job.finish()? {
        OpsFinished::Raster { tail, header } => {
            out.extend(tail);
            out[..header.len()].copy_from_slice(&header);
            let mut reader = open_bytes(&out, None, READER_BUDGET).map_err(|e| e.0)?;
            let (w, h) = (reader.levels[0].width, reader.levels[0].height);
            for need in reader.needs(0, 0, 0, w, h) {
                let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
                reader.put_block(&need, &out[a..b]).map_err(|e| e.0)?;
            }
            let (sample, nodata) = (reader.info.sample, reader.nodata);
            let reg = reader.region(0, 0, 0, w, h).ok_or("the level")?;
            let n = (w * h * reg.bands) as usize;
            Ok((
                Ran::Raster(Back {
                    width: w,
                    height: h,
                    bands: reg.bands,
                    sample,
                    nodata,
                    affine,
                    values: (0..n).map(|k| reg.samples.get(k)).collect(),
                }),
                notes,
            ))
        }
        OpsFinished::Report => Ok((Ran::Report, notes)),
        _ => Err("an unexpected result".into()),
    }
}

/// Whether `got` meets `want` by the case's rule (NaN only with NaN).
fn meets(got: f64, want: f64, rule: &str) -> bool {
    if got.is_nan() || want.is_nan() {
        return got.is_nan() && want.is_nan();
    }
    if rule == "exact" {
        return got == want;
    }
    (got - want).abs() <= 1.0
}

fn table_of(v: &Value) -> Option<Table> {
    let strings = |a: &Value| -> Vec<String> {
        a.as_array()
            .map(|a| {
                a.iter()
                    .map(|s| s.as_str().unwrap_or("").to_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    v.as_object().map(|_| Table {
        columns: strings(&v["columns"]),
        rows: v["rows"]
            .as_array()
            .map(|r| r.iter().map(strings).collect())
            .unwrap_or_default(),
    })
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|s| s.as_str().unwrap_or("").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// What is wrong with a raster result against `want`, if anything.
fn raster_wrong(back: &Back, notes: &Notes, want: &Value, said: &Value) -> Option<String> {
    let rule = want["rule"].as_str().unwrap_or("exact");
    let wa: Vec<f64> = want["affine"]
        .as_array()
        .expect("affine")
        .iter()
        .map(number)
        .collect();
    let size = (
        want["width"].as_u64().unwrap_or(0) as u32,
        want["height"].as_u64().unwrap_or(0) as u32,
        want["bands"].as_u64().unwrap_or(0) as u32,
    );
    if (back.width, back.height, back.bands) != size || back.affine.to_vec() != wa {
        return Some(format!(
            "{} × {} × {} at {:?}, want {size:?} at {wa:?}",
            back.width, back.height, back.bands, back.affine
        ));
    }
    let sample = sample_of(want["sample"].as_str().unwrap_or("f32"));
    if back.sample != sample {
        return Some(format!("{:?} samples, want {sample:?}", back.sample));
    }
    let nodata_ok = match &want["nodata"] {
        Value::String(s) if s == "nan" => back.nodata.is_some_and(f64::is_nan),
        Value::Number(n) => back.nodata == n.as_f64(),
        _ => back.nodata.is_none(),
    };
    if !nodata_ok {
        return Some(format!("nodata {:?}, want {}", back.nodata, want["nodata"]));
    }
    let wv: Vec<f64> = want["values"]
        .as_array()
        .expect("values")
        .iter()
        .map(number)
        .collect();
    if let Some(k) = (0..wv.len()).find(|&k| !meets(back.values[k], wv[k], rule)) {
        return Some(format!(
            "sample {k}: {} vs {} ({rule})",
            back.values[k], wv[k]
        ));
    }
    let r = &notes.remote;
    if r.table != table_of(&said["table"]) {
        return Some(format!("table {:?}, want {}", r.table, said["table"]));
    }
    if r.tail != said["tail"].as_str().unwrap_or("") {
        return Some(format!("tail “{}”, want {}", r.tail, said["tail"]));
    }
    if r.warnings != strings(&said["warnings"]) {
        return Some(format!(
            "warnings {:?}, want {}",
            r.warnings, said["warnings"]
        ));
    }
    None
}

#[test]
fn remote_cases() {
    let doc: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{}remote/v1/cases.json", crate::host::DIR))
            .expect("the cases"),
    )
    .expect("JSON");
    let cases = doc["cases"].as_array().expect("cases");
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap_or("?");
        let expect = &c["expect"];
        let runs: Vec<Result<(Ran, Notes), String>> = [1, 3].iter().map(|&t| run(c, t)).collect();
        if let Some(why) = expect["refused"].as_str() {
            for r in &runs {
                match r {
                    Err(e) if e.contains(why) => {}
                    Err(e) => failures.push(format!("{name}: refused with “{e}”, not “{why}”")),
                    Ok(_) => failures.push(format!("{name}: ran, should be refused ({why})")),
                }
            }
            continue;
        }
        let ((first, n1), (second, n3)) = match (&runs[0], &runs[1]) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        if n1 != n3 {
            failures.push(format!("{name}: the notes differ on 1 and 3 threads"));
        }
        match (first, second) {
            (Ran::Raster(a), Ran::Raster(b)) => {
                let same = a.values.len() == b.values.len()
                    && a.values
                        .iter()
                        .zip(&b.values)
                        .all(|(x, y)| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()));
                if !same {
                    failures.push(format!("{name}: the samples differ on 1 and 3 threads"));
                }
                if let Some(why) = raster_wrong(a, n1, &expect["raster"], &expect["notes"]) {
                    failures.push(format!("{name}: {why}"));
                }
            }
            (Ran::Report, Ran::Report) => {
                let want = &expect["accuracy"];
                let r = &n1.remote;
                if r.table != table_of(&want["table"]) {
                    failures.push(format!(
                        "{name}: table {:?}, want {}",
                        r.table, want["table"]
                    ));
                }
                if r.tail != want["summary"].as_str().unwrap_or("") {
                    failures.push(format!("{name}: “{}”, want {}", r.tail, want["summary"]));
                }
                if r.warnings != strings(&want["warnings"]) {
                    failures.push(format!(
                        "{name}: warnings {:?}, want {}",
                        r.warnings, want["warnings"]
                    ));
                }
                if r.overall != want["overall"].as_f64() || r.kappa != want["kappa"].as_f64() {
                    failures.push(format!(
                        "{name}: overall {:?}, kappa {:?}, want {} and {}",
                        r.overall, r.kappa, want["overall"], want["kappa"]
                    ));
                }
            }
            _ => failures.push(format!("{name}: the two runs gave different results")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
