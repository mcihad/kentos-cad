//! The page codecs (docs/adr/0628 §1) against files reference libraries wrote: Arrow's Snappy, GZIP, LZ4_RAW and
//! Brotli, libzstd, LZ4's frames and Hadoop's framing (fixtures/compress/v1, scripts/fixtures/compress_cases.py). Each
//! opens to the length and CRC-32 its case gives; broken ones are refused. Snappy written here goes out for Arrow to
//! open: `KENTOS_WRITE_COMPRESS=1` writes fixtures/compress/v1/out, `compress_cases.py --written` opens it.

use std::path::PathBuf;
use std::time::Instant;

use kentos_formats::compress::{self, Codec, snappy};
use kentos_formats::zip::crc32;
use serde_json::{Value, json};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/compress/v1")
}

fn codec(name: &str) -> Codec {
    match name {
        "SNAPPY" => Codec::Snappy,
        "GZIP" => Codec::Gzip,
        "LZ4" => Codec::Lz4,
        "LZ4_RAW" => Codec::Lz4Raw,
        "ZSTD" => Codec::Zstd,
        "BROTLI" => Codec::Brotli,
        _ => panic!("tanınmayan sıkıştırma {name}"),
    }
}

fn cases() -> Vec<Value> {
    let text = std::fs::read_to_string(dir().join("cases.json")).unwrap();
    let all: Value = serde_json::from_str(&text).unwrap();
    all["cases"].as_array().unwrap().clone()
}

/// The bytes a good case opens to.
fn opened(case: &Value) -> Vec<u8> {
    let raw = std::fs::read(dir().join(case["file"].as_str().unwrap())).unwrap();
    let size = case["size"].as_u64().unwrap() as usize;
    compress::decompress(codec(case["codec"].as_str().unwrap()), &raw, size).unwrap()
}

#[test]
fn every_reference_file_opens_or_is_refused() {
    let cases = cases();
    assert!(cases.len() >= 60, "{} durum", cases.len());
    let mut failures = Vec::new();
    for case in &cases {
        let file = case["file"].as_str().unwrap();
        let raw = std::fs::read(dir().join(file)).unwrap();
        let size = case["size"].as_u64().unwrap() as usize;
        let got = compress::decompress(codec(case["codec"].as_str().unwrap()), &raw, size);
        match (case.get("error"), got) {
            (Some(_), Ok(_)) => failures.push(format!("{file}: açıldı, reddedilmeliydi")),
            (Some(_), Err(_)) => {}
            (None, Err(e)) => failures.push(format!("{file}: {e}")),
            (None, Ok(out)) => {
                let crc = case["crc32"].as_u64().unwrap() as u32;
                if out.len() != size || crc32(&out) != crc {
                    failures.push(format!(
                        "{file}: {} bayt, CRC {:08x}; beklenen {size}, {crc:08x}",
                        out.len(),
                        crc32(&out)
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn refusals_say_why() {
    for case in cases().iter().filter(|c| c.get("error").is_some()) {
        let raw = std::fs::read(dir().join(case["file"].as_str().unwrap())).unwrap();
        let size = case["size"].as_u64().unwrap() as usize;
        let e =
            compress::decompress(codec(case["codec"].as_str().unwrap()), &raw, size).unwrap_err();
        let codec_name = codec(case["codec"].as_str().unwrap()).name().to_lowercase();
        let family = codec_name.split('_').next().unwrap();
        assert!(e.to_lowercase().contains(family), "{}: {e}", case["file"]);
    }
    let brotli = cases()
        .into_iter()
        .find(|c| c["codec"] == "BROTLI")
        .unwrap();
    let raw = std::fs::read(dir().join(brotli["file"].as_str().unwrap())).unwrap();
    let e = compress::decompress(Codec::Brotli, &raw, 10).unwrap_err();
    assert!(e.contains("açılmaz") && e.contains("ZSTD"), "{e}");
}

#[test]
fn snappy_written_here_opens_again_and_elsewhere() {
    let out = dir().join("out");
    let write = std::env::var_os("KENTOS_WRITE_COMPRESS").is_some();
    if write {
        std::fs::create_dir_all(&out).unwrap();
    }
    let mut index = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for case in cases().iter().filter(|c| c.get("error").is_none()) {
        let data = opened(case);
        let packed = snappy::compress(&data);
        assert_eq!(
            snappy::decompress(&packed, data.len()).unwrap(),
            data,
            "{}",
            case["file"]
        );
        let source = case["file"].as_str().unwrap();
        let stem = source.rsplit_once('.').map_or(source, |(stem, _)| stem);
        let name = stem.replace('/', "-") + ".snappy";
        // One block per distinct input goes out.
        if write && seen.insert((data.len(), crc32(&data))) {
            std::fs::write(out.join(&name), &packed).unwrap();
            index.push(json!({"file": name, "source": case["file"], "codec": case["codec"], "size": data.len()}));
        }
    }
    if write {
        let text = serde_json::to_string_pretty(&index).unwrap() + "\n";
        std::fs::write(out.join("index.json"), text).unwrap();
    }
}

#[test]
fn a_page_opened_with_the_wrong_size_is_refused() {
    let case = cases()
        .into_iter()
        .find(|c| c["file"] == "zstd/metin-3.zst")
        .unwrap();
    let raw = std::fs::read(dir().join("zstd/metin-3.zst")).unwrap();
    let size = case["size"].as_u64().unwrap() as usize;
    assert!(compress::decompress(Codec::Zstd, &raw, size - 1).is_err());
    let snappy_case = cases()
        .into_iter()
        .find(|c| c["file"] == "snappy/metin.snappy")
        .unwrap();
    let raw = std::fs::read(dir().join("snappy/metin.snappy")).unwrap();
    let size = snappy_case["size"].as_u64().unwrap() as usize;
    assert!(compress::decompress(Codec::Snappy, &raw, size + 1).is_err());
    assert!(compress::decompress(Codec::Snappy, &raw, size - 1).is_err());
}

/// Decoding speed of every good case, repeated to at least 64 MB opened, and Snappy's encoding speed (release:
/// `cargo test --release -p kentos-formats --test all compress_timing -- --ignored --nocapture`).
#[test]
#[ignore]
fn compress_timing() {
    let mut by_codec: std::collections::BTreeMap<&str, (usize, f64)> = Default::default();
    for case in cases().iter().filter(|c| c.get("error").is_none()) {
        let raw = std::fs::read(dir().join(case["file"].as_str().unwrap())).unwrap();
        let size = case["size"].as_u64().unwrap() as usize;
        if size < 10_000 {
            continue;
        }
        let codec = codec(case["codec"].as_str().unwrap());
        let rounds = (64 << 20) / size + 1;
        let t = Instant::now();
        for _ in 0..rounds {
            std::hint::black_box(compress::decompress(codec, &raw, size).unwrap());
        }
        let s = t.elapsed().as_secs_f64();
        println!(
            "{:28} {:>8} bayt → {:>8}: {:7.0} MB/s",
            case["file"].as_str().unwrap(),
            raw.len(),
            size,
            (rounds * size) as f64 / s / 1e6
        );
        let e = by_codec.entry(codec.name()).or_default();
        e.0 += rounds * size;
        e.1 += s;
    }
    for (name, (bytes, s)) in &by_codec {
        println!("{name:8} ortalama {:7.0} MB/s", *bytes as f64 / s / 1e6);
    }
    let data: Vec<u8> = cases()
        .iter()
        .filter(|c| c.get("error").is_none() && c["size"].as_u64().unwrap() > 10_000)
        .flat_map(opened)
        .collect();
    let t = Instant::now();
    let mut packed = 0;
    for _ in 0..8 {
        packed = snappy::compress(&data).len();
    }
    let s = t.elapsed().as_secs_f64();
    println!(
        "Snappy yazma: {} bayt → {packed}, {:.0} MB/s",
        data.len(),
        (8 * data.len()) as f64 / s / 1e6
    );
}
