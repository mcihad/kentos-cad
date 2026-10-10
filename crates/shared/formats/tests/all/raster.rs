//! Raster layers' files (docs/adr/0204) against fixtures/raster/v1/cases.json
//! (scripts/fixtures/raster_cases.py: the files written and read by GDAL,
//! the worked-out levels, statistics and tiles' colours by the ADR's rules in
//! numpy, the look's shaded relief and ramp checked against gdaldem): every
//! file's facts, every level's samples (an FNV-1a hash of their
//! little-endian bytes), the statistics and the tiles' colours. JPEG blocks
//! are decoded here with zune-jpeg (the desktop's decoder): their samples
//! are within a few steps of GDAL's libjpeg.

use std::path::PathBuf;

use kentos_contracts::RasterStyle;
use kentos_formats::raster::source::{Put, Reader, open_bytes};
use serde_json::Value;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/raster/v1")
}

fn fnv(bytes: &[u8]) -> String {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("{h:016x}")
}

/// zune-jpeg's pixels of a stream, and their components.
fn jpeg(stream: &[u8]) -> (Vec<u8>, u32) {
    use zune_jpeg::JpegDecoder;
    use zune_jpeg::zune_core::bytestream::ZCursor;
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut d = JpegDecoder::new_with_options(ZCursor::new(stream), options);
    let pixels = d.decode().expect("the block decodes");
    (pixels, 3)
}

/// Every block a region of `level` wants, from the whole file's bytes.
fn fill(reader: &mut Reader, bytes: &[u8], level: usize, x: i64, y: i64, w: u32, h: u32) {
    for need in reader.needs(level, x, y, w, h) {
        let a = need.offset as usize;
        let b = a + need.len as usize;
        match reader
            .put_block(&need, &bytes[a..b])
            .expect("the block reads")
        {
            Put::Done => {}
            Put::Jpeg(stream) => {
                let (px, comps) = jpeg(&stream);
                reader.put_pixels(&need, px, comps).expect("the pixels fit");
            }
        }
    }
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::String(s) if s == "nan" => Some(f64::NAN),
        _ => v.as_f64(),
    }
}

#[test]
fn every_file_reads_and_draws_as_the_reference_says() {
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(dir().join("cases.json")).expect("cases.json"),
    )
    .expect("cases.json is JSON");
    assert_eq!(file["format"], "kentos.raster-cases");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20, "{} cases", cases.len());
    let mut off: Vec<String> = Vec::new();
    for c in cases {
        let name = c["file"].as_str().expect("file");
        let bytes = std::fs::read(dir().join("files").join(name)).expect("the file");
        let world = c
            .get("world")
            .and_then(Value::as_str)
            .map(|w| std::fs::read_to_string(dir().join("files").join(w)).expect("the world file"));
        let mut reader = match open_bytes(&bytes, world.as_deref(), 256 << 20) {
            Ok(r) => r,
            Err(e) => {
                off.push(format!("{name}: {}", e.0));
                continue;
            }
        };
        let info = &c["info"];
        let got = &reader.info;
        let facts = (
            got.width,
            got.height,
            got.bands,
            serde_json::to_value(got.sample).expect("sample"),
        );
        let want = (
            info["width"].as_u64().unwrap_or(0) as u32,
            info["height"].as_u64().unwrap_or(0) as u32,
            info["bands"].as_u64().unwrap_or(0) as u32,
            info["sample"].clone(),
        );
        if facts != want {
            off.push(format!("{name}: {facts:?} ≠ {want:?}"));
            continue;
        }
        let affine: Option<Vec<f64>> = info["affine"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_f64).collect());
        if got.affine.map(|a| a.to_vec()) != affine {
            off.push(format!("{name}: dönüşüm {:?} ≠ {affine:?}", got.affine));
        }
        if info.get("epsg").is_some() && got.epsg.map(u64::from) != info["epsg"].as_u64() {
            off.push(format!("{name}: EPSG {:?} ≠ {}", got.epsg, info["epsg"]));
        }
        let nodata = num(&info["nodata"]);
        let same_nodata = match (got.nodata, nodata) {
            (Some(a), Some(b)) => a == b || (a.is_nan() && b.is_nan()),
            (None, None) => true,
            _ => false,
        };
        if !same_nodata {
            off.push(format!("{name}: nodata {:?} ≠ {nodata:?}", got.nodata));
        }
        // GDAL's reading of the last band (the TIFFs'): a mask or data (docs/adr/0242's fix).
        if let Some(alpha) = info["alpha"].as_bool()
            && got.alpha != alpha
        {
            off.push(format!("{name}: alfa {} ≠ {alpha}", got.alpha));
        }
        if u64::from(got.overviews) != info["overviews"].as_u64().unwrap_or(0) {
            off.push(format!(
                "{name}: önizleme {} ≠ {}",
                got.overviews, info["overviews"]
            ));
        }
        let levels = c["levels"].as_array().expect("levels");
        if levels.len() != reader.levels.len() {
            off.push(format!(
                "{name}: {} kat ≠ {}",
                reader.levels.len(),
                levels.len()
            ));
            continue;
        }
        for (k, lv) in levels.iter().enumerate() {
            let (w, h) = (reader.levels[k].width, reader.levels[k].height);
            if (u64::from(w), u64::from(h))
                != (
                    lv["width"].as_u64().unwrap_or(0),
                    lv["height"].as_u64().unwrap_or(0),
                )
            {
                off.push(format!("{name}: {k}. kat {w} × {h}"));
                continue;
            }
            fill(&mut reader, &bytes, k, 0, 0, w, h);
            let Some(region) = reader.region(k, 0, 0, w, h) else {
                off.push(format!("{name}: {k}. kat okunamadı"));
                continue;
            };
            if let Some(want) = lv.get("fnv").and_then(Value::as_str) {
                let got = fnv(&region.samples.to_bytes(true));
                if got != want {
                    off.push(format!(
                        "{name}: {k}. katın örnekleri farklı ({got} ≠ {want})"
                    ));
                }
            } else {
                // JPEG: band means within a step, sampled pixels within a few.
                let b = region.bands as usize;
                let n = w as usize * h as usize;
                for (band, m) in lv["means"].as_array().expect("means").iter().enumerate() {
                    let sum: f64 = (0..n).map(|i| region.samples.get(i * b + band)).sum();
                    let mean = sum / n as f64;
                    if (mean - m.as_f64().unwrap_or(0.0)).abs() > 1.0 {
                        off.push(format!(
                            "{name}: {k}. katın {band}. bant ortalaması {mean} ≠ {m}"
                        ));
                    }
                }
                for p in lv["points"].as_array().expect("points") {
                    let (x, y) = (
                        p[0].as_u64().unwrap_or(0) as u32,
                        p[1].as_u64().unwrap_or(0) as u32,
                    );
                    for (band, v) in p[2].as_array().expect("values").iter().enumerate() {
                        let got = region.at(x, y, band as u32);
                        if (got - v.as_f64().unwrap_or(0.0)).abs() > 6.0 {
                            off.push(format!(
                                "{name}: {k}. katın ({x}, {y}) pikseli {band}. bantta {got} ≠ {v}"
                            ));
                        }
                    }
                }
            }
        }
        let stats = reader.stats();
        if let Some(want) = c.get("stats") {
            let got = serde_json::to_value(stats.as_ref().map(|s| &s.bands)).expect("stats");
            let want = &want["bands"];
            let same = got.as_array().zip(want.as_array()).is_some_and(|(g, w)| {
                g.len() == w.len()
                    && g.iter().zip(w).all(|(a, b)| {
                        ["count", "min", "max", "low", "high"]
                            .iter()
                            .all(|k| a[*k].as_f64() == b[*k].as_f64())
                    })
            });
            if !same {
                off.push(format!("{name}: istatistikler {got} ≠ {want}"));
            }
        }
        let affine = got_affine(&reader);
        for t in c
            .get("tiles")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let style: RasterStyle = serde_json::from_value(t["style"].clone()).expect("a style");
            let level = t["level"].as_u64().unwrap_or(0) as usize;
            let (tx, ty) = (
                t["tx"].as_u64().unwrap_or(0) as u32,
                t["ty"].as_u64().unwrap_or(0) as u32,
            );
            let (x, y, w, h) = Reader::tile_region(tx, ty);
            fill(&mut reader, &bytes, level, x, y, w, h);
            let mut rgba = Vec::new();
            if !reader.render_tile(level, tx, ty, &style, stats.as_ref(), &affine, &mut rgba) {
                off.push(format!("{name}: karo ({level}, {tx}, {ty}) çizilemedi"));
                continue;
            }
            let got = fnv(&rgba);
            if got != t["fnv"].as_str().unwrap_or("") {
                let opaque = rgba.chunks_exact(4).filter(|p| p[3] != 0).count();
                off.push(format!(
                    "{name}: karo ({level}, {tx}, {ty}) {} farklı: {got} ({opaque} dolu) ≠ {} ({} dolu)",
                    t["style"], t["fnv"], t["opaque"]
                ));
            }
        }
    }
    assert!(off.is_empty(), "{} fark:\n{}", off.len(), off.join("\n"));
}

/// The reader's own place, else one metre a pixel (the reference's default).
fn got_affine(reader: &Reader) -> [f64; 6] {
    reader
        .info
        .affine
        .unwrap_or([0.0, 1.0, 0.0, 0.0, 0.0, -1.0])
}
