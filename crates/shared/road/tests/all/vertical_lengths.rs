//! A vertical curve's least length against the shared cases (fixtures/road/v1/vertical-lengths.json, docs/adr/0935
//! §1): written by scripts/fixtures/road_vertical_length_cases.py, which solves each criterion forward (the sight a
//! curve gives) at 30 digits; and a crest of the proposed length driven with the profile's own sight (Y20).

use kentos_road::profile::lengths::{bisector, comfort, crest, headlight, stopping};
use kentos_road::profile::vertical::{Pvi, VCurve, build};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/vertical-lengths.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

#[test]
fn every_length_is_the_references() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for c in all["stopping"].as_array().unwrap() {
        let got = stopping(n(&c["speed"]), n(&c["time"]), n(&c["decel"]), n(&c["grade"]));
        match c["distance"].as_f64() {
            None => assert_eq!(got, None, "{c}"),
            Some(d) => assert!((got.expect("a distance") - d).abs() < 1e-9, "{c}: {got:?}"),
        }
    }
    for c in all["crest"].as_array().unwrap() {
        let got = crest(n(&c["a"]), n(&c["sight"]), n(&c["eye"]), n(&c["object"]));
        assert!((got - n(&c["length"])).abs() < 1e-6, "{c}: {got}");
    }
    for c in all["headlight"].as_array().unwrap() {
        let got = headlight(n(&c["a"]), n(&c["sight"]), n(&c["height"]), n(&c["beam"]));
        assert!((got - n(&c["length"])).abs() < 1e-6, "{c}: {got}");
    }
    for c in all["comfort"].as_array().unwrap() {
        let got = comfort(n(&c["a"]), n(&c["speed"]), n(&c["accel"]));
        assert!((got - n(&c["length"])).abs() < 1e-9, "{c}: {got}");
    }
    for c in all["bisector"].as_array().unwrap() {
        let got = bisector(n(&c["a"]), n(&c["external"]));
        assert!((got - n(&c["length"])).abs() < 1e-9, "{c}: {got}");
    }
}

/// A crest of the proposed length: the least sight the profile's own sight line (Y20) finds over every eye place is
/// the stopping sight distance asked, within the sight's step. The numbers are docs/adr/0935 §1's.
#[test]
fn a_crest_of_the_proposed_length_gives_the_sight() {
    for (speed, a) in [(50.0, 4.0), (80.0, 6.0), (100.0, 3.0), (80.0, 1.0)] {
        let s = stopping(speed, 2.5, 3.4, 0.0).unwrap();
        let l = crest(a, s, 1.08, 0.60);
        let g = a / 200.0;
        let pvi = |id: &str, station: f64, elevation: f64, curve: VCurve| Pvi { id: id.into(), station, elevation, curve };
        let v = build(&[
            pvi("a", 0.0, 100.0 - g * 2000.0, VCurve::None),
            // A length of 0: the grades alone give the sight, no curve is asked.
            pvi("v", 2000.0, 100.0, if l > 0.0 { VCurve::Parabola { length: l } } else { VCurve::None }),
            pvi("b", 4000.0, 100.0 - g * 2000.0, VCurve::None),
        ])
        .expect("builds");
        let worst = (0..=((l + 2.0 * s) / 0.5) as usize)
            .filter_map(|i| v.sight(2000.0 - l / 2.0 - s + 0.5 * i as f64, true, 1.08, 0.60, 3.0 * s, 0.01))
            .map(|seen| seen.distance)
            .fold(f64::INFINITY, f64::min);
        println!("V {speed} km/sa, A %{a}: DGM {s:.2} m, L {l:.2} m, en kısa görüş {worst:.2} m");
        if l > 0.0 {
            assert!((worst - s).abs() < 0.05, "{speed} {a}: {worst} vs {s}");
        } else {
            assert!(worst >= s - 0.05, "{speed} {a}: the grades alone give {worst} ≥ {s}");
        }
    }
}
