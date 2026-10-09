//! Yüzey analizi (docs/adr/0231 §3–§8): every case of
//! `scripts/fixtures/terrain_cases.py` (KentOS code not used there; its
//! definitions held to gdaldem's) run through a whole job. A 32-bit result
//! may be one unit in the last place off the reference (the targets'
//! transcendental functions round apart now and then); a byte result is
//! exact. The result's reduced levels are the 2 × 2 means of the level above,
//! its place and system the raster's.

use kentos_formats::raster::samples::stored;
use kentos_formats::raster::source::Reader;
use kentos_raster::job::Spec;
use serde_json::Value;

use crate::host::{Ran, opened, read, run};

fn cases() -> Value {
    serde_json::from_slice(&read("terrain/v1/cases.json")).expect("the cases read")
}

/// The units in the last place between two 32-bit values (NaN only with NaN).
fn ulps(a: f32, b: f32) -> u32 {
    if a.is_nan() || b.is_nan() {
        return if a.is_nan() && b.is_nan() {
            0
        } else {
            u32::MAX
        };
    }
    let key = |v: f32| {
        let i = v.to_bits() as i32;
        if i < 0 { i32::MIN - i } else { i }
    };
    key(a).abs_diff(key(b))
}

/// Level `level` of the result whole: its samples, bands interleaved.
fn level(reader: &mut Reader, level: usize, bytes: &[u8]) -> (u32, u32, Vec<f64>) {
    let (w, h) = (reader.levels[level].width, reader.levels[level].height);
    for need in reader.needs(level, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        reader
            .put_block(&need, &bytes[a..b])
            .expect("a block of the result");
    }
    let r = reader.region(level, 0, 0, w, h).expect("the level whole");
    let n = (w * h * r.bands) as usize;
    (w, h, (0..n).map(|k| r.samples.get(k)).collect())
}

#[test]
fn every_case_works_out_as_the_reference_and_its_levels_halve() {
    let doc = cases();
    let dems = doc["dems"].as_array().expect("dems");
    let mut checked = 0;
    for case in doc["cases"].as_array().expect("cases") {
        let name = format!(
            "{} / {}",
            case["dem"].as_str().unwrap(),
            case["name"].as_str().unwrap()
        );
        let dem = dems
            .iter()
            .find(|d| d["name"] == case["dem"])
            .expect("the case's DEM");
        let tif = read(&format!("terrain/v1/{}", dem["file"].as_str().unwrap()));
        let spec: Spec =
            serde_json::from_value(case["spec"].clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let threads = if checked % 2 == 0 { 1 } else { 4 };
        let Ran::Raster(out) = run(&tif, &spec, threads).unwrap_or_else(|e| panic!("{name}: {e}"))
        else {
            panic!("{name}: a raster result");
        };
        let mut reader = opened(&out);
        // Its place and system are the raster's.
        let info = reader.info.clone();
        assert_eq!(info.affine, Some(spec.affine), "{name}: the place");
        assert_eq!(info.epsg, spec.epsg, "{name}: the system");
        let bands = case["bands"].as_u64().unwrap() as u32;
        assert_eq!(info.bands, bands, "{name}: bands");
        let (w, h, got) = level(&mut reader, 0, &out);
        assert_eq!(
            (w, h),
            (
                dem["width"].as_u64().unwrap() as u32,
                dem["height"].as_u64().unwrap() as u32
            ),
            "{name}"
        );
        let byte = case["sample"] == "u8";
        let compare = |k: usize, want: &Value| {
            let g = got[k];
            if byte {
                let want = want.as_f64().unwrap();
                assert_eq!(g, want, "{name}: sample {k}");
            } else {
                let want = want.as_f64().map_or(f32::NAN, |v| v as f32);
                assert!(
                    ulps(g as f32, want) <= 1,
                    "{name}: sample {k}: {g} for {want}"
                );
            }
        };
        if let Some(values) = case["values"].as_array() {
            assert_eq!(values.len(), got.len(), "{name}: every sample");
            for (k, v) in values.iter().enumerate() {
                compare(k, v);
            }
        } else {
            for p in case["probe"].as_array().expect("a probe") {
                let (i, j) = (
                    p[0].as_u64().unwrap() as usize,
                    p[1].as_u64().unwrap() as usize,
                );
                let at = (j * w as usize + i) * bands as usize;
                match &p[2] {
                    Value::Array(px) => {
                        for (c, v) in px.iter().enumerate() {
                            compare(at + c, v);
                        }
                    }
                    v => compare(at, v),
                }
            }
        }
        // Every reduced level is the means of 2 × 2 of the one above.
        let sample = info.sample;
        let nodata = info.nodata;
        let mut above = (w, h, got);
        for k in 1..reader.levels.len() {
            let (lw, lh, lv) = level(&mut reader, k, &out);
            assert_eq!(
                (lw, lh),
                (above.0.div_ceil(2), above.1.div_ceil(2)),
                "{name}: level {k}"
            );
            for j in 0..lh as usize {
                for i in 0..lw as usize {
                    for c in 0..bands as usize {
                        let (mut sum, mut n) = (0.0, 0u32);
                        for (x, y) in [
                            (2 * i, 2 * j),
                            (2 * i + 1, 2 * j),
                            (2 * i, 2 * j + 1),
                            (2 * i + 1, 2 * j + 1),
                        ] {
                            if x >= above.0 as usize || y >= above.1 as usize {
                                continue;
                            }
                            let v = above.2[(y * above.0 as usize + x) * bands as usize + c];
                            if v.is_nan() || nodata.is_some_and(|d| v == d) {
                                continue;
                            }
                            sum += v;
                            n += 1;
                        }
                        let want = if n > 0 {
                            stored(sample, sum / f64::from(n))
                        } else {
                            nodata.unwrap_or(f64::NAN)
                        };
                        let g = lv[(j * lw as usize + i) * bands as usize + c];
                        assert!(
                            g == want || (g.is_nan() && want.is_nan()),
                            "{name}: level {k} ({i}, {j}) band {c}: {g} for {want}"
                        );
                    }
                }
            }
            above = (lw, lh, lv);
        }
        checked += 1;
    }
    assert_eq!(checked, 43, "every case");
}

#[test]
fn a_band_or_a_setting_out_of_bounds_is_refused_saying_why() {
    let tif = read("terrain/v1/tepe.tif");
    let doc = cases();
    let base = doc["cases"][0]["spec"].clone();
    let with = |patch: Value| -> String {
        let mut spec = base.clone();
        for (k, v) in patch.as_object().unwrap() {
            spec[k] = v.clone();
        }
        let spec: Spec = serde_json::from_value(spec).expect("a spec");
        match run(&tif, &spec, 1) {
            Err(e) => e,
            Ok(_) => panic!("{patch} runs"),
        }
    };
    assert!(with(serde_json::json!({"band": 2})).contains("1 bandı var"));
    assert!(with(serde_json::json!({"tool": {"kind": "hillshade", "azimuth": 315.0, "altitude": 95.0, "zFactor": 1.0}})).contains("0 ile 90"));
    assert!(with(serde_json::json!({"tool": {"kind": "colorRelief", "table": "100 #00FF00\n", "interp": "linear"}})).contains("en az iki satır"));
    assert!(with(serde_json::json!({"tool": {"kind": "colorRelief", "table": "100 green\n200 #FF0000\n", "interp": "linear"}})).contains("#RRGGBB"));
    assert!(with(serde_json::json!({"tool": {"kind": "insolation", "firstDay": 200, "lastDay": 100, "dayStep": 7, "hourStep": 1.0, "transmissivity": 0.5, "zFactor": 1.0}})).contains("başlangıcı"));
    assert!(with(serde_json::json!({"tool": {"kind": "insolation", "firstDay": 1, "lastDay": 365, "dayStep": 7, "hourStep": 0.7, "transmissivity": 0.5, "zFactor": 1.0}})).contains("Saat aralığı"));
    assert!(with(serde_json::json!({"system": null, "tool": {"kind": "insolation", "firstDay": 1, "lastDay": 365, "dayStep": 7, "hourStep": 1.0, "transmissivity": 0.5, "zFactor": 1.0}})).contains("Enlem"));
    assert!(
        with(serde_json::json!({"affine": [0.0, 1.0, 2.0, 0.0, 2.0, 4.0]})).contains("tersinmiyor")
    );
    assert!(with(serde_json::json!({"affine": [0.0, 1.0, 0.5, 0.0, 0.0, -1.0], "tool": {"kind": "curvature", "curvature": "total", "zFactor": 1.0}})).contains("dik pikselli"));
}

#[test]
fn a_local_project_s_insolation_takes_the_latitude_written() {
    // tepe's rows lie near 39.9°; written as one latitude for the whole raster the
    // result is close to (not the same as) the rows' own.
    let tif = read("terrain/v1/tepe.tif");
    let doc = cases();
    let case = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["dem"] == "tepe" && c["name"].as_str().unwrap().starts_with("Güneşlenme (yıl"))
        .unwrap();
    let mut spec = case["spec"].clone();
    spec["system"] = Value::Null;
    spec["tool"]["latitude"] = serde_json::json!(39.92);
    let spec: Spec = serde_json::from_value(spec).unwrap();
    let Ran::Raster(out) = run(&tif, &spec, 2).unwrap() else {
        panic!()
    };
    let mut reader = opened(&out);
    let (_, _, got) = level(&mut reader, 0, &out);
    let want: Vec<f64> = case["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap_or(f64::NAN))
        .collect();
    let mut close = 0;
    for (g, w) in got.iter().zip(&want) {
        if w.is_nan() {
            assert!(g.is_nan());
            continue;
        }
        assert!((g - w).abs() / w < 2e-3, "{g} for about {w}");
        close += 1;
    }
    assert!(close > 1000);
}

#[test]
fn each_result_is_drawn_as_its_tool_says() {
    // docs/adr/0231 §10: a band of numbers in the tool's ramp over its least to its most (Eğrilik's over
    // its 2nd to 98th percentile); Gölgeli kabartma grey as its bytes are, Renkli kabartma's colours as
    // they are; lines have no look.
    use kentos_contracts::{RasterRender, RasterSample, RasterStretch};
    use kentos_raster::job::Job;
    let tif = read("terrain/v1/tepe.tif");
    let base = cases()["cases"][0]["spec"].clone();
    let look = |tool: Value| {
        let mut spec = base.clone();
        spec["tool"] = tool;
        let spec: Spec = serde_json::from_value(spec).unwrap();
        let (job, _) = Job::new(opened(&tif), &spec, 1).unwrap();
        (job.result(), job.style())
    };
    for (tool, ramp, stretch) in [
        (
            serde_json::json!({"kind": "slope", "method": "horn", "unit": "degrees", "zFactor": 1.0}),
            "Spektral",
            RasterStretch::MinMax,
        ),
        (
            serde_json::json!({"kind": "aspect", "method": "horn", "zFactor": 1.0}),
            "Spektral",
            RasterStretch::MinMax,
        ),
        (
            serde_json::json!({"kind": "curvature", "curvature": "total", "zFactor": 1.0}),
            "Mavi-kırmızı",
            RasterStretch::Percent,
        ),
        (
            serde_json::json!({"kind": "ruggedness", "index": "tpi"}),
            "Viridis",
            RasterStretch::MinMax,
        ),
        (
            serde_json::json!({"kind": "insolation", "firstDay": 1, "lastDay": 365, "dayStep": 14, "hourStep": 0.5, "transmissivity": 0.5, "zFactor": 1.0}),
            "Sıcaklık",
            RasterStretch::MinMax,
        ),
    ] {
        let (result, style) = look(tool.clone());
        assert_eq!(result, Some((1, RasterSample::F32)), "{tool}");
        let s = style.unwrap();
        assert_eq!(
            (s.render, s.ramp.as_deref(), s.stretch, s.invert, &s.bands),
            (RasterRender::Ramp, Some(ramp), stretch, false, &vec![1]),
            "{tool}"
        );
    }
    let (result, style) = look(
        serde_json::json!({"kind": "hillshade", "azimuth": 315.0, "altitude": 45.0, "zFactor": 1.0}),
    );
    assert_eq!(result, Some((1, RasterSample::U8)));
    let s = style.unwrap();
    assert_eq!(
        (s.render, s.stretch, &s.bands),
        (RasterRender::Gray, RasterStretch::None, &vec![1])
    );
    let (result, style) =
        look(serde_json::json!({"kind": "colorRelief", "ramp": "Arazi", "interp": "linear"}));
    assert_eq!(result, Some((4, RasterSample::U8)));
    let s = style.unwrap();
    assert_eq!(
        (s.render, s.stretch, &s.bands),
        (RasterRender::Rgb, RasterStretch::None, &vec![1, 2, 3, 4])
    );
    let (result, style) = look(
        serde_json::json!({"kind": "contours", "interval": 5.0, "base": 0.0, "indexEvery": 5}),
    );
    assert_eq!((result, style), (None, None));
}
