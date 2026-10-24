//! Independent Decimal XYZ placements through nested OCS, nonuniform scales and MINSERT.
use kentos_contracts::{DrawingUnit, DxfReadOptions};
use kentos_formats::dxf;
use serde_json::Value;

fn cases() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(
        "../../../../../fixtures/formats/v1/cad-spatial/expected.json"
    ))
    .expect("independent oracle")["cases"]
        .as_array()
        .expect("cases")
        .clone()
}

fn source(case: &Value) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures")
            .join(case["source"].as_str().expect("path")),
    )
    .expect("DXF")
}

fn compare(actual: &Value, expected: &Value, at: &str, failures: &mut Vec<String>) {
    match expected {
        Value::Object(fields) => {
            for (key, value) in fields {
                compare(&actual[key], value, &format!("{at}.{key}"), failures);
            }
        }
        Value::Array(list) => {
            if actual.as_array().map(Vec::len) != Some(list.len()) {
                failures.push(format!("{at}: {actual} != {expected}"));
            } else {
                for (i, value) in list.iter().enumerate() {
                    compare(&actual[i], value, &format!("{at}[{i}]"), failures);
                }
            }
        }
        Value::Number(n) => {
            let value = n.as_f64().expect("number");
            // 32 relative float64 roundings, plus 1 pm for small SI values.
            let tolerance = value.abs() * 32.0 * f64::EPSILON + 1e-12;
            if actual
                .as_f64()
                .is_none_or(|a| !a.is_finite() || (a - value).abs() > tolerance)
            {
                failures.push(format!("{at}: {actual} != {expected}"));
            }
        }
        _ => {
            if actual != expected {
                failures.push(format!("{at}: {actual} != {expected}"));
            }
        }
    }
}

#[test]
fn every_xyz_is_preserved_even_when_an_approximate_import_was_explicitly_allowed() {
    let mut failures = Vec::new();
    for case in cases() {
        let name = case["name"].as_str().expect("name");
        let read = dxf::read_checked(
            &source(&case),
            &DxfReadOptions {
                unit: Some(DrawingUnit::M),
                explode_blocks: true,
                allow_polyline_loss: true,
                ..Default::default()
            },
        )
        .expect("read");
        compare(
            &serde_json::to_value(&read.entities).expect("entities"),
            &case["expected"],
            name,
            &mut failures,
        );
        if let Some(directory) = std::env::var_os("KENTOS_CAD_SPATIAL_EXPORTS") {
            let input = dxf::input_from_json(
                &serde_json::json!({
                    "entities":read.entities,"layers":[],"unit":"m","scale":1000.,
                    "lengthDecimals":3,"grads":false,"annotations":{"metadata":false}
                })
                .to_string(),
            )
            .expect("export input");
            let (bytes, _) = dxf::write_checked(&input).expect("lossless standard export");
            std::fs::create_dir_all(&directory).expect("export folder");
            std::fs::write(
                std::path::Path::new(&directory).join(format!("{name}.dxf")),
                bytes,
            )
            .expect("export");
        }
    }
    assert!(
        failures.is_empty(),
        "{} XYZ failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn representable_straight_geometry_needs_no_loss_acceptance() {
    for case in cases() {
        let read = dxf::read_checked(
            &source(&case),
            &DxfReadOptions {
                unit: Some(DrawingUnit::M),
                explode_blocks: true,
                ..Default::default()
            },
        );
        assert!(read.is_ok(), "{}: {read:?}", case["name"]);
    }
}
