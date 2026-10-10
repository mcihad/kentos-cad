//! Uygunluk analizi's runs end to end (docs/adr/0237): every case of
//! `scripts/fixtures/suitability_cases.py` (KentOS code not used there; the
//! memberships' exp and pow from mpmath, the weights from its 50-digit
//! eigenvector, AUC from every pair) run as a host runs it, on one thread and
//! on three: the inputs written as tiled GeoTIFFs, their blocks handed over,
//! the steps taken (the share never going down), the raster read back, the
//! notes, the weights and the curve compared.

use kentos_formats::raster::source::open_bytes;
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::ops::{Notes, OpsFinished, OpsJob, OpsSpec};
use kentos_raster::suitability::roc::Roc;
use serde_json::{Value, json};

use crate::hydrology::{numbers, raster_of, shapes_of};
use crate::raster_ops::tiff;

enum Ran {
    Raster(u32, u32, Vec<f64>),
    Roc(Roc),
    Weights,
}

/// A GeoTIFF's level 0 read back; its nodata as NaN.
fn read_back(file: &[u8]) -> Result<(u32, u32, Vec<f64>), String> {
    let mut reader = open_bytes(file, None, READER_BUDGET).map_err(|e| e.0)?;
    let (w, h) = (reader.levels[0].width, reader.levels[0].height);
    for need in reader.needs(0, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        reader.put_block(&need, &file[a..b]).map_err(|e| e.0)?;
    }
    let nodata = reader.nodata;
    let reg = reader.region(0, 0, 0, w, h).ok_or("the level")?;
    Ok((
        w,
        h,
        (0..(w * h) as usize)
            .map(|k| {
                let v = reg.samples.get(k);
                if Some(v) == nodata { f64::NAN } else { v }
            })
            .collect(),
    ))
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
        OpsFinished::Roc(r) => Ok((Ran::Roc(r), notes)),
        OpsFinished::Weights => Ok((Ran::Weights, notes)),
        _ => Err("neither a raster, a curve nor weights".into()),
    }
}

/// A number against the reference's (null: NaN) by the case's rule: bit for
/// bit, or within one unit in the last place of float32 (`f32ulp`) or of
/// float64 (`f64ulp`): libm's exp and pow against the correctly rounded ones.
fn same(got: f64, want: &Value, rule: &str) -> bool {
    match want.as_f64() {
        Some(w) if rule == "f32ulp" => ulps32(got, w) <= 1,
        Some(w) if rule == "f64ulp" => ulps64(got, w) <= 1,
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

fn ulps64(a: f64, b: f64) -> u64 {
    let key = |v: f64| {
        let i = v.to_bits() as i64;
        if i < 0 { i64::MIN - i } else { i }
    };
    key(a).wrapping_sub(key(b)).unsigned_abs()
}

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

fn check_notes(name: &str, notes: &Notes, want: &Value) -> Result<(), String> {
    let s = &notes.suit;
    for (k, got) in [
        ("unmatched", s.unmatched),
        ("outside", s.outside),
        ("restricted", s.restricted),
        ("invalid", s.invalid),
    ] {
        if let Some(w) = want.get(k)
            && w.as_u64() != Some(got)
        {
            return Err(format!("{name}: notes: {k} {got} ≠ {w}"));
        }
    }
    Ok(())
}

/// The weights within 10⁻¹² of the 50-digit eigenvector's, λ, CI and CR within 10⁻¹⁰.
fn check_pairwise(name: &str, notes: &Notes, want: &Value) -> Result<(), String> {
    let p = notes
        .suit
        .pairwise
        .as_ref()
        .ok_or(format!("{name}: no weights"))?;
    let w = numbers(&want["weights"]);
    if w.len() != p.weights.len() {
        return Err(format!("{name}: {} weights", p.weights.len()));
    }
    for (k, (got, want)) in p.weights.iter().zip(&w).enumerate() {
        let want = want.as_f64().unwrap_or(f64::NAN);
        if (got - want).abs() > 1e-12 {
            return Err(format!("{name}: weight {k}: {got} for {want}"));
        }
    }
    for (k, got) in [
        ("lambda", p.lambda),
        ("ci", p.ci),
        ("ri", p.ri),
        ("cr", p.cr),
    ] {
        let want = want[k].as_f64().unwrap_or(f64::NAN);
        if (got - want).abs() > 1e-10 {
            return Err(format!("{name}: {k} {got} for {want}"));
        }
    }
    Ok(())
}

fn check_roc(name: &str, r: &Roc, want: &Value) -> Result<(), String> {
    let mut bad = Vec::new();
    for (k, got) in [
        ("presence", r.presence),
        ("background", r.background),
        ("skipped", r.skipped),
        ("outside", r.outside),
        ("both", r.both),
    ] {
        if want[k].as_u64() != Some(got) {
            bad.push(format!("{k} {got} ≠ {}", want[k]));
        }
    }
    if want["allCells"].as_bool() != Some(r.all_cells) {
        bad.push("allCells".into());
    }
    if want["auc"].as_f64() != Some(r.auc) {
        bad.push(format!("auc {} ≠ {}", r.auc, want["auc"]));
    }
    if want["best"].as_u64() != r.best.map(|b| b as u64) {
        bad.push(format!("best {:?} ≠ {}", r.best, want["best"]));
    }
    let rows: Vec<(f64, u64, u64)> = numbers(&want["rows"])
        .iter()
        .map(|row| {
            let t = numbers(row);
            (
                t[0].as_f64().unwrap_or(f64::NAN),
                t[1].as_u64().unwrap_or(0),
                t[2].as_u64().unwrap_or(0),
            )
        })
        .collect();
    let got: Vec<(f64, u64, u64)> = r.rows.iter().map(|x| (x.threshold, x.tp, x.fp)).collect();
    if rows != got {
        bad.push(format!("rows {got:?} ≠ {rows:?}"));
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("{name}: {}", bad.join(", ")))
    }
}

/// Every case of the reference, on one thread and on three.
#[test]
fn suitability_cases() {
    let doc: Value =
        serde_json::from_slice(&crate::host::read("suitability/v1/cases.json")).expect("the cases");
    let mut failures = Vec::new();
    let mut checked = 0;
    for c in doc["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap_or("?");
        let expect = &c["expect"];
        for threads in [1, 3] {
            let outcome = run(c, threads).and_then(|(ran, notes)| {
                match ran {
                    Ran::Raster(w, h, v) => check_raster(name, (w, h, &v), &expect["raster"])?,
                    Ran::Roc(r) => check_roc(name, &r, &expect["roc"])?,
                    Ran::Weights => {
                        if expect.get("raster").is_some() || expect.get("roc").is_some() {
                            return Err(format!("{name}: no result"));
                        }
                    }
                }
                if let Some(n) = expect.get("notes") {
                    check_notes(name, &notes, n)?;
                }
                if let Some(p) = expect.get("pairwise") {
                    check_pairwise(name, &notes, p)?;
                }
                Ok(())
            });
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
