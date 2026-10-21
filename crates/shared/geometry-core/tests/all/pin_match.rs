//! Matching label pins without a key (docs/adr/0578 §2) against the independent reference in
//! `fixtures/labels/v1/pin-match.json` (`scripts/fixtures/pin_match_cases.py`, no KentOS code).

use kentos_geometry_core::ops::pin_match::{Feature, Found, match_pins};
use serde_json::Value;

fn feature(v: &Value) -> Feature {
    let coords: Vec<f64> = v["coords"]
        .as_array()
        .expect("coordinates")
        .iter()
        .map(|c| c.as_f64().expect("a number"))
        .collect();
    let middle = v["middle"].as_array().expect("a middle");
    Feature {
        kind: v["kind"].as_str().expect("a kind").to_owned(),
        bits: coords.iter().map(|c| c.to_bits()).collect(),
        middle: [
            middle[0].as_f64().expect("east"),
            middle[1].as_f64().expect("north"),
        ],
    }
}

#[test]
fn every_case_is_what_the_reference_finds() {
    let path = format!(
        "{}/../../../fixtures/labels/v1/pin-match.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 40, "{} cases", cases.len());
    let mut kinds = std::collections::BTreeSet::new();
    for c in cases {
        let name = c["name"].as_str().expect("a name");
        let old: Vec<Feature> = c["old"]
            .as_array()
            .expect("old")
            .iter()
            .map(feature)
            .collect();
        let new: Vec<Feature> = c["new"]
            .as_array()
            .expect("new")
            .iter()
            .map(feature)
            .collect();
        let got = match_pins(&old, &new, c["tolerance"].as_f64().expect("a tolerance"));
        let want = c["expect"].as_array().expect("expectations");
        assert_eq!(got.len(), want.len(), "{name}");
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            let at = format!("{name}, object {i}");
            let found = w["found"].as_str().expect("found");
            kinds.insert(found.to_owned());
            match (g, found) {
                (Found::Geometry(j), "geometry") => {
                    assert_eq!(*j as u64, w["to"].as_u64().expect("to"), "{at}");
                }
                (Found::Nearest(j, d), "nearest") => {
                    assert_eq!(*j as u64, w["to"].as_u64().expect("to"), "{at}");
                    assert!(
                        (d - w["distance"].as_f64().expect("distance")).abs() < 1e-12,
                        "{at}"
                    );
                }
                (Found::Ambiguous, "ambiguous") | (Found::None, "none") => {}
                (g, want) => panic!("{at}: {g:?} ≠ {want}"),
            }
        }
    }
    // The reference's cases reach every answer.
    for k in ["geometry", "nearest", "ambiguous", "none"] {
        assert!(kinds.contains(k), "no case finds {k}");
    }
}
