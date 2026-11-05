//! A flood's closure of the road over a culvert against the shared cases (fixtures/road/v1/culvert_closure.json,
//! docs/adr/0948): written by scripts/fixtures/road_culvert_closure_cases.py without KentOS code and by another way —
//! each criterion a limit on the flow per metre on the crest, its total flow in one root of the culvert's headwater —
//! where KentOS bisects the total flow on the criterion itself.

use kentos_road::culvert_closure::{Span, VEHICLES, flood};
use kentos_road::culvert_hydraulics::{Culvert, Section, Weir, inlet};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/culvert_closure.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn same(name: &str, what: &str, got: &Span, want: &Value) {
    match (got.flow, want["flow"].as_f64()) {
        (None, None) => {}
        (Some(a), Some(b)) => assert!((a - b).abs() <= 1e-7 * b.max(1.0), "{name} {what}: threshold {a} vs {b}"),
        other => panic!("{name} {what}: threshold {other:?}"),
    }
    assert!((got.hours - n(&want["hours"])).abs() <= 1e-7, "{name} {what}: hours {} vs {}", got.hours, want["hours"]);
    let spans = want["spans"].as_array().expect("spans");
    assert_eq!(got.spans.len(), spans.len(), "{name} {what}: spans");
    for (g, w) in got.spans.iter().zip(spans) {
        assert!((g.0 - n(&w[0])).abs() <= 1e-7 && (g.1 - n(&w[1])).abs() <= 1e-7, "{name} {what}: {g:?} vs {w}");
    }
}

#[test]
fn the_closures_are_the_references() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    let vehicles = doc["vehicles"].as_array().expect("vehicles");
    assert_eq!(vehicles.len(), VEHICLES.len());
    for (v, (name, limits)) in vehicles.iter().zip(VEHICLES) {
        assert_eq!(v["name"].as_str(), Some(name));
        assert_eq!((n(&v["depth"]), n(&v["speed"]), n(&v["product"])), (limits.depth, limits.speed, limits.product));
    }
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 12);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let k = &c["culvert"];
        let s = &k["section"];
        let section = if s["kind"] == "circle" {
            Section::Circle { diameter: n(&s["diameter"]) }
        } else {
            Section::Box { width: n(&s["width"]), height: n(&s["height"]) }
        };
        let culvert = Culvert {
            section,
            cells: k["cells"].as_u64().unwrap() as u32,
            length: n(&k["length"]),
            slope: n(&k["slope"]),
            n: n(&k["n"]),
            inlet: inlet(k["inlet"].as_str().unwrap()).expect("named"),
            ke: n(&k["ke"]),
        };
        let w = &c["weir"];
        let weir = Weir { crest: n(&w["crest"]), length: n(&w["length"]), cd: n(&w["cd"]) };
        let points: Vec<(f64, f64)> = c["hydrograph"].as_array().unwrap().iter().map(|p| (n(&p[0]), n(&p[1]))).collect();
        for f in c["floods"].as_array().unwrap() {
            let got = flood(&culvert, &weir, n(&c["tailwater"]), &points, n(&f["factor"]), n(&f["cd"]));
            assert!((got.peak_flow - n(&f["peakFlow"])).abs() < 1e-12, "{name}: peak");
            assert!((got.peak.headwater - n(&f["peakHeadwater"])).abs() < 1e-9, "{name}: peak headwater");
            assert_eq!(u64::from(got.peak.hazard), f["peakHazard"].as_u64().unwrap(), "{name}: peak hazard");
            same(name, "submerged", &got.submerged, &f["submerged"]);
            for (k, (g, w)) in got.vehicles.iter().zip(f["vehicles"].as_array().unwrap()).enumerate() {
                same(name, VEHICLES[k].0, g, w);
            }
            for (k, (g, w)) in got.hazards.iter().zip(f["hazards"].as_array().unwrap()).enumerate() {
                same(name, &format!("H{}", k + 2), g, w);
            }
        }
    }
}
