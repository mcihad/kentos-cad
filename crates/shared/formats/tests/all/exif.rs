//! A photo's EXIF (docs/adr/0225 §3) against fixtures/exif/v1, whose photos
//! and expected values scripts/fixtures/exif_cases.py writes by hand from the
//! rules (Pillow reads them back): every value to the bit, every problem in
//! its words. And the reader's time on a thousand photos' headers.

use kentos_formats::exif::{needs_more, read};
use serde_json::Value;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/exif/v1/");

#[test]
fn every_photo_reads_as_the_reference_says() {
    let text = std::fs::read_to_string(format!("{DIR}cases.json")).expect("the cases");
    let cases: Value = serde_json::from_str(&text).expect("JSON");
    let list = cases["cases"].as_array().expect("cases");
    assert!(list.len() >= 14);
    for c in list {
        let file = c["file"].as_str().expect("file");
        let bytes = std::fs::read(format!("{DIR}{file}")).expect("the photo");
        assert!(!needs_more(&bytes), "{file}");
        let got = serde_json::to_value(read(&bytes)).expect("JSON");
        let mut want = c.clone();
        want.as_object_mut().expect("a case").remove("file");
        // Floats to the bit: JSON numbers written by Python's repr and read back exactly.
        for key in ["latitude", "longitude", "altitude", "direction"] {
            let (g, w) = (
                got.get(key).and_then(Value::as_f64),
                want.get(key).and_then(Value::as_f64),
            );
            assert_eq!(
                g.map(f64::to_bits),
                w.map(f64::to_bits),
                "{file}: {key} {g:?} ≠ {w:?}"
            );
        }
        assert_eq!(got, want, "{file}");
    }
}

#[test]
fn the_first_bytes_of_a_photo_say_when_more_are_needed() {
    let bytes = std::fs::read(format!("{DIR}konum.jpg")).expect("the photo");
    // Cut inside its APP1: more is needed; the whole file is read.
    assert!(needs_more(&bytes[..40]));
    assert!(!needs_more(&bytes));
    assert!(read(&bytes).latitude.is_some());
}

#[test]
#[ignore = "timing; run with --ignored --nocapture in release"]
fn timing() {
    // A thousand photos' headers (docs/adr/0225 §7: ≤ 50 ms).
    let bytes = std::fs::read(format!("{DIR}konum.jpg")).expect("the photo");
    let start = std::time::Instant::now();
    let mut n = 0;
    for _ in 0..1000 {
        n += usize::from(read(std::hint::black_box(&bytes)).latitude.is_some());
    }
    let ms = start.elapsed().as_secs_f64() * 1e3;
    println!("EXIF: 1 000 fotoğrafın başlığı {ms:.2} ms ({n} konum)");
    assert!(ms <= 50.0, "{ms} ms");
}
