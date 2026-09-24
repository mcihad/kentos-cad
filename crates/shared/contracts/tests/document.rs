//! The drawing snapshot the TypeScript app writes (recorded as
//! `fixtures/document/v1/sample.json` from `apps/web/src/model/snapshotSample.ts`)
//! reads into the Rust contracts and writes back to the same JSON, every
//! float64 kept; unknown formats and versions are refused.

use kentos_contracts::{DocumentSnapshotV1, Entity};
use serde_json::Value;

const FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/document/v1/sample.json"
);

#[test]
fn the_typescript_snapshot_round_trips_through_the_contracts() {
    let text = std::fs::read_to_string(FILE).expect("fixture");
    let snap = DocumentSnapshotV1::from_json(&text).expect("readable snapshot");
    assert_eq!(snap.entities.len(), 13);
    // Every kind the app has is present and typed.
    let kinds: Vec<&str> = snap
        .entities
        .iter()
        .map(|e| match e {
            Entity::Point(_) => "point",
            Entity::Line(_) => "line",
            Entity::Polyline(_) => "polyline",
            Entity::Polygon(_) => "polygon",
            Entity::Circle(_) => "circle",
            Entity::Arc(_) => "arc",
            Entity::Ellipse(_) => "ellipse",
            Entity::Spline(_) => "spline",
            Entity::Xline(_) => "xline",
            Entity::Ray(_) => "ray",
            Entity::Text(_) => "text",
            Entity::Dimension(_) => "dimension",
            Entity::Hatch(_) => "hatch",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "point",
            "line",
            "polyline",
            "polygon",
            "circle",
            "arc",
            "ellipse",
            "spline",
            "xline",
            "ray",
            "text",
            "dimension",
            "hatch"
        ]
    );
    // Written back, the JSON is the same value: every float64 exact, optional fields kept or left out
    // as they were. JSON writes 0 and 0.0 alike, so numbers are compared as float64.
    let original: Value = serde_json::from_str(&text).unwrap();
    let again = serde_json::to_value(&snap).unwrap();
    assert_eq!(as_f64(again), as_f64(original));
}

fn as_f64(v: Value) -> Value {
    match v {
        Value::Number(n) => Value::from(n.as_f64().expect("finite number")),
        Value::Array(a) => Value::Array(a.into_iter().map(as_f64).collect()),
        Value::Object(o) => Value::Object(o.into_iter().map(|(k, v)| (k, as_f64(v))).collect()),
        other => other,
    }
}

#[test]
fn other_formats_and_versions_are_refused() {
    let text = std::fs::read_to_string(FILE).expect("fixture");
    let mut v: Value = serde_json::from_str(&text).unwrap();
    v["version"] = Value::from(2);
    let err = DocumentSnapshotV1::from_json(&v.to_string()).unwrap_err();
    assert!(err.contains("sürümü 2"), "{err}");
    v["version"] = Value::from(1);
    v["format"] = Value::from("kentos-style");
    assert!(
        DocumentSnapshotV1::from_json(&v.to_string())
            .unwrap_err()
            .contains("KentOS çizim dosyası değil")
    );
    assert!(
        DocumentSnapshotV1::from_json("{")
            .unwrap_err()
            .contains("JSON değil")
    );
}
