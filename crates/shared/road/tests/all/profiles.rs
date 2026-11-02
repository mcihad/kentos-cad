//! The terrain profile against the shared cases (fixtures/road/v1/profiles.json, docs/adr/0902 §1): written by walking
//! the alignment on Triangle's surface and refining each change of triangle at 40 digits
//! (scripts/fixtures/road_profile_cases.py), no KentOS code.

use kentos_road::alignment::{PiInput, build as align};
use kentos_road::profile::{TOLERANCE, terrain};
use kentos_road::surface::{SurfaceInput, build};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/profiles.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

#[test]
fn every_profile_crosses_where_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("name");
        let input: SurfaceInput = serde_json::from_value(c["input"].clone()).expect("input");
        let surface = build(&input).expect("a surface");
        let pis: Vec<PiInput> = c["pis"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| PiInput {
                id: p["id"].as_str().unwrap().to_owned(),
                x: n(&p["x"]),
                y: n(&p["y"]),
                radius: n(&p["radius"]),
                spiral_in: n(&p["spiralIn"]),
                spiral_out: n(&p["spiralOut"]),
            })
            .collect();
        let al = align(&pis).expect("an alignment");
        let got = terrain(&surface, &al);
        let crossings: Vec<_> = got.iter().filter(|p| p.crossing).collect();
        let want = c["crossings"].as_array().unwrap();
        assert_eq!(crossings.len(), want.len(), "{name}: crossings");
        for (g, w) in crossings.iter().zip(want) {
            assert!((g.station - n(&w["station"])).abs() < 1e-7, "{name}: {} vs {}", g.station, w["station"]);
            let z = g.z.expect("a crossing has a height");
            assert!((z - n(&w["z"])).abs() < 1e-7, "{name}: {z} vs {}", w["z"]);
        }
        // The stretches off the surface: each mark's `gap` (the stretch before it).
        let off: Vec<bool> = c["off"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap()).collect();
        let marks: Vec<_> = got.iter().filter(|p| p.crossing || p.station == 0.0 || p.station == al.length).collect();
        assert_eq!(marks.len(), off.len() + 1, "{name}: marks");
        for (m, o) in marks[1..].iter().zip(&off) {
            assert_eq!(m.gap, *o, "{name}: the stretch before {}", m.station);
        }
        assert!(got.windows(2).all(|w| w[0].station < w[1].station), "{name}: in order");
        // A linear reading is within the tolerance of the true height.
        for w in c["checks"].as_array().unwrap() {
            let s = n(&w["station"]);
            let i = got.partition_point(|p| p.station <= s).clamp(1, got.len() - 1);
            let (a, b) = (got[i - 1], got[i]);
            match (w["z"].as_f64(), a.z, b.z, b.gap) {
                (Some(z), Some(za), Some(zb), false) => {
                    let lin = za + (zb - za) * (s - a.station) / (b.station - a.station);
                    assert!((lin - z).abs() <= TOLERANCE + 1e-9, "{name}: at {s}: {lin} vs {z}");
                }
                (None, _, _, gap) => assert!(gap || a.z.is_none() || b.z.is_none(), "{name}: {s} off the surface"),
                other => panic!("{name}: at {s}: {other:?}"),
            }
        }
    }
}
