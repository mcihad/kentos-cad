//! Control points to and from a file (docs/adr/0518 §4) against fixtures/raster/v1/gcp/cases.json
//! (scripts/fixtures/gcp_file_cases.py: Python's `csv`, no KentOS code): QGIS's `.points` and KentOS's own read, a set
//! of points written.

use std::path::PathBuf;

use kentos_contracts::GeorefPoint;
use kentos_formats::raster::gcp;
use serde_json::Value;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/raster/v1/gcp")
}

fn cases() -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir().join("cases.json")).expect("cases.json")).expect("JSON")
}

#[test]
fn points_files_read_as_csv_reads_them() {
    let cases = cases();
    for case in cases["reads"].as_array().expect("reads") {
        let file = case["file"].as_str().expect("file");
        let read = gcp::read(&std::fs::read_to_string(dir().join(file)).expect("file")).expect(file);
        let want: Vec<GeorefPoint> = serde_json::from_value(case["points"].clone()).expect("points");
        assert_eq!(read.points, want, "{file}");
        assert_eq!(read.crs.as_deref(), case["crs"].as_str(), "{file}");
    }
}

#[test]
fn points_are_written_as_the_reference_writes_them() {
    let cases = cases();
    for case in cases["writes"].as_array().expect("writes") {
        let points: Vec<GeorefPoint> = serde_json::from_value(case["points"].clone()).expect("points");
        let residuals: Vec<Option<[f64; 2]>> = serde_json::from_value(case["residuals"].clone()).expect("residuals");
        let text = gcp::write(&points, &residuals);
        assert_eq!(text, case["text"].as_str().expect("text"));
        // Read back: the same points, a comma dropped from a name.
        let back = gcp::read(&text).expect("reads back");
        assert_eq!(back.points.len(), points.len());
        for (a, b) in back.points.iter().zip(&points) {
            assert_eq!((a.pixel, a.target, a.check, a.off), (b.pixel, b.target, b.check, b.off));
        }
    }
}

#[test]
fn broken_files_say_their_line() {
    assert!(gcp::read("mapX,mapY,sourceX,sourceY\n").unwrap_err().contains("kontrol noktası yok"));
    assert!(gcp::read("mapX,mapY,sourceX,sourceY\n1,2,x,4\n").unwrap_err().starts_with("2. satır"));
    assert!(gcp::read("1,2,3\n").unwrap_err().contains("sourceY yok"));
    assert!(gcp::read("mapX,mapY,sourceX,sourceY,role\n1,2,3,4,kontrol\n").unwrap_err().contains("rolü bilinmiyor"));
}
