//! The raster codec profile (docs/adr/0642 §1) against fixtures/raster/v1/codecs.json
//! (scripts/fixtures/raster_codec_cases.py: GeoTIFFs written and read by GDAL): the code's table of TIFF compressions
//! is the reference's, every file read gives GDAL's level 0 samples (lossy WebP within a few steps), and every
//! compression not read is refused by its name and number with the way to convert the file.

use std::path::PathBuf;

use kentos_formats::raster::layout::{CONVERT, TIFF_CODECS};
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

fn fill(reader: &mut Reader, bytes: &[u8], w: u32, h: u32) {
    for need in reader.needs(0, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        match reader
            .put_block(&need, &bytes[a..b])
            .expect("the block reads")
        {
            Put::Done => {}
            Put::Jpeg(_) => panic!("no JPEG here"),
        }
    }
}

#[test]
fn the_codec_profile_is_the_references_and_every_file_reads_or_is_refused_as_it_says() {
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(dir().join("codecs.json")).expect("codecs.json"),
    )
    .expect("JSON");
    assert_eq!(file["format"], "kentos.raster-codecs");
    assert_eq!(file["convert"], CONVERT);
    let profile: Vec<(u64, String, bool)> = file["profile"]
        .as_array()
        .expect("profile")
        .iter()
        .map(|p| {
            (
                p["code"].as_u64().expect("code"),
                p["name"].as_str().expect("name").to_owned(),
                p["read"].as_bool().expect("read"),
            )
        })
        .collect();
    let ours: Vec<(u64, String, bool)> = TIFF_CODECS
        .iter()
        .map(|(c, n, r)| (*c, (*n).to_owned(), *r))
        .collect();
    assert_eq!(ours, profile, "the code's table is the profile");
    let mut off = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let name = c["file"].as_str().expect("file");
        let bytes = std::fs::read(dir().join("codecs").join(name)).expect("the file");
        let opened = open_bytes(&bytes, None, 256 << 20);
        if let Some(words) = c.get("refused") {
            match opened {
                Ok(_) => off.push(format!("{name}: okunmamalıydı")),
                Err(e) => {
                    for w in words.as_array().expect("words") {
                        let w = w.as_str().expect("word");
                        if !e.0.contains(w) {
                            off.push(format!("{name}: ret “{}” “{w}” demiyor", e.0));
                        }
                    }
                }
            }
            continue;
        }
        let mut reader = match opened {
            Ok(r) => r,
            Err(e) => {
                off.push(format!("{name}: {}", e.0));
                continue;
            }
        };
        let info = &c["info"];
        let (w, h) = (reader.info.width, reader.info.height);
        let facts = (
            w,
            h,
            reader.info.bands,
            serde_json::to_value(reader.info.sample).expect("sample"),
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
        fill(&mut reader, &bytes, w, h);
        let Some(region) = reader.region(0, 0, 0, w, h) else {
            off.push(format!("{name}: okunamadı"));
            continue;
        };
        if let Some(want) = c["fnv"].as_str() {
            let got = fnv(&region.samples.to_bytes(true));
            if got != want {
                off.push(format!("{name}: örnekler farklı ({got} ≠ {want})"));
            }
            continue;
        }
        // Lossy WebP: band means within a step, sampled pixels within a few.
        let b = region.bands as usize;
        let n = w as usize * h as usize;
        for (band, m) in c["means"].as_array().expect("means").iter().enumerate() {
            let mean = (0..n)
                .map(|i| region.samples.get(i * b + band))
                .sum::<f64>()
                / n as f64;
            if (mean - m.as_f64().unwrap_or(0.0)).abs() > 1.0 {
                off.push(format!("{name}: {band}. bant ortalaması {mean} ≠ {m}"));
            }
        }
        for p in c["points"].as_array().expect("points") {
            let (x, y) = (
                p[0].as_u64().unwrap_or(0) as u32,
                p[1].as_u64().unwrap_or(0) as u32,
            );
            for (band, v) in p[2].as_array().expect("values").iter().enumerate() {
                let got = region.at(x, y, band as u32);
                if (got - v.as_f64().unwrap_or(0.0)).abs() > 6.0 {
                    off.push(format!("{name}: ({x}, {y}) {band}. bant {got} ≠ {v}"));
                }
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}
