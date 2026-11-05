//! A side's daylight diagnosis against the shared cases (fixtures/road/v1/daylight.json, docs/adr/0951): written by
//! scripts/fixtures/road_daylight_cases.py in exact fractions, cutting the slope's ray against every terrain stretch on
//! its own where KentOS walks outward; no KentOS code.

use kentos_road::section::GroundPoint;
use kentos_road::section::template::{Component, Daylight, Ditch, Side, Template, build, daylights};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/daylight.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn side(v: &Value) -> Side {
    Side {
        components: v["components"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| Component::Lane { width: n(&c["width"]), slope: n(&c["slope"]) })
            .collect(),
        cut: n(&v["cut"]),
        fill: n(&v["fill"]),
        ditch: v.get("ditch").map(|d| Ditch { fore: n(&d["fore"]), depth: n(&d["depth"]), bottom: n(&d["bottom"]) }),
        bench: v.get("bench").map(|b| kentos_road::section::template::CutBench { height: n(&b["height"]), width: n(&b["width"]), fall: b.get("fall").map_or(0.0, n) }),
    }
}

#[test]
fn the_daylights_are_the_references() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 10);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let template = Template { left: side(&c["template"]["left"]), right: side(&c["template"]["right"]) };
        let ground: Vec<GroundPoint> = c["ground"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| GroundPoint { offset: n(&g["offset"]), z: g["z"].as_f64(), gap: g["gap"].as_bool().unwrap(), crossing: false })
            .collect();
        let axis = n(&c["axis"]);
        let got = daylights(&template, axis, &ground);
        let built = build(&template, axis, &ground).expect("builds");
        for (k, (d, want)) in got.iter().zip(c["expected"].as_array().unwrap()).enumerate() {
            let close = |a: f64, b: f64, what: &str| assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{name} {k} {what}: {a} vs {b}");
            match (d, want["kind"].as_str().unwrap()) {
                (Daylight::Hit { offset, crossings, next, sensitivity, cut }, "hit") => {
                    close(*offset, n(&want["offset"]), "offset");
                    assert_eq!(u64::from(*crossings), want["crossings"].as_u64().unwrap(), "{name} {k} crossings");
                    assert_eq!(*cut, want["cut"].as_bool().unwrap(), "{name} {k} cut");
                    match (next, want.get("next")) {
                        (None, None) => {}
                        (Some(a), Some(b)) => close(*a, n(b), "next"),
                        other => panic!("{name} {k} next {other:?}"),
                    }
                    close(*sensitivity, n(&want["sensitivity"]), "sensitivity");
                    // The diagnosis finds the daylight the section was built to.
                    let s = if k == 0 { &built.left } else { &built.right };
                    close(s.daylight.expect("built to it")[0].abs(), *offset, "the section's daylight");
                }
                (Daylight::Hinge, "hinge") => {}
                (Daylight::Gap { at }, "gap") => close(*at, n(&want["at"]), "gap"),
                (Daylight::Swath { end }, "swath") => close(*end, n(&want["end"]), "swath"),
                other => panic!("{name} {k}: {other:?} vs {want}"),
            }
            if !matches!(d, Daylight::Hit { .. }) {
                let s = if k == 0 { &built.left } else { &built.right };
                assert!(s.daylight.is_none(), "{name} {k}: open in the section too");
            }
        }
    }
}
