//! The writers that move into WGS 84, GPX and RFC 7946 GeoJSON (docs/adr/0222
//! §5, §7), against fixtures/formats/v1/wgs84/ (scripts/fixtures/
//! wgs84_write_cases.py writes the inputs and, with --check, holds the files
//! written here to them with PROJ, no KentOS code): each writer's bytes as
//! committed (`KENTOS_WRITE_WGS84=1` rewrites them), and what it says.

use std::path::{Path, PathBuf};

use kentos_contracts::{GeoJsonWriteInput, GpxWriteInput};
use kentos_formats::{geojson, gnss::gpx_write};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::crs::{Choice, System};
use serde_json::Value;

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/wgs84")
}

fn parsed<T: FromJson>(v: &Value) -> T {
    T::from_json(&Json::parse(&v.to_string()).expect("JSON")).expect("the value")
}

#[test]
fn the_writers_write_as_committed() {
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .expect("wgs84/")
        .filter_map(|e| {
            let p = e.ok()?.path();
            Some(
                p.file_name()?
                    .to_str()?
                    .strip_suffix(".case.json")?
                    .to_owned(),
            )
        })
        .collect();
    names.sort();
    assert!(names.len() >= 3);
    let update = std::env::var_os("KENTOS_WRITE_WGS84").is_some();
    for name in names {
        let case: Value = serde_json::from_str(
            &std::fs::read_to_string(dir().join(format!("{name}.case.json"))).expect("the case"),
        )
        .expect("JSON");
        let from: System = parsed(&case["system"]);
        let choices: Vec<Choice> = parsed(&case["choices"]);
        let format = case["format"].as_str().expect("a format");
        let (bytes, report) = match format {
            "gpx" => {
                let input: GpxWriteInput =
                    serde_json::from_value::<gpx_write::WriteInput>(case["input"].clone())
                        .map(|w| w.0)
                        .unwrap_or_else(|e| panic!("{name}: {e}"));
                gpx_write::write(&input, &from, &choices)
            }
            _ => {
                let input: GeoJsonWriteInput =
                    serde_json::from_value::<geojson::WriteInput>(case["input"].clone())
                        .map(|w| w.0)
                        .unwrap_or_else(|e| panic!("{name}: {e}"));
                geojson::write_wgs84(&input, &from, &choices)
            }
        };
        let path = dir().join(format!("{name}.{format}"));
        if update {
            std::fs::write(&path, &bytes).expect("write");
        }
        assert_eq!(
            bytes,
            std::fs::read(&path).unwrap_or_default(),
            "{name}: the writer's bytes differ from the committed file (KENTOS_WRITE_WGS84=1 rewrites it; read the difference first)"
        );
        assert!(
            report.notes.iter().any(|n| n.what == "Daire"),
            "{name}: {:?}",
            report.notes
        );
        if format == "gpx" {
            assert!(
                report
                    .notes
                    .iter()
                    .any(|n| n.what == "Alan" && n.count == 3),
                "{name}: {:?}",
                report.notes
            );
            assert!(
                report.notes.iter().any(|n| n.what == "Öznitelik"),
                "{name}: {:?}",
                report.notes
            );
            assert_eq!(report.counts.get("text"), Some(&1), "{name}");
        } else {
            assert!(
                report
                    .notes
                    .iter()
                    .any(|n| n.what == "Koordinat sistemi" && n.reason.contains("WGS 84")),
                "{name}: {:?}",
                report.notes
            );
            assert!(
                report.skipped.iter().any(|s| s.what == "Yazı"),
                "{name}: {:?}",
                report.skipped
            );
        }
    }
}
