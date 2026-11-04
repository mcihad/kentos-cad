//! A culvert's headwater and the road's overtopping against the shared cases (fixtures/road/v1/culvert_hydraulics.json,
//! docs/adr/0947): written by scripts/fixtures/road_culvert_hydraulics_cases.py in mpmath at 40 digits, no KentOS code.

use kentos_road::culvert_hydraulics::{Control, Culvert, INLETS, Inlet, Section, Weir, inlet, perform};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/culvert_hydraulics.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

#[test]
fn the_headwaters_and_overtopping_are_the_references() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    let inlets = &doc["inlets"];
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 15);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let k = &c["culvert"];
        let s = &k["section"];
        let section = if s["kind"] == "circle" {
            Section::Circle { diameter: n(&s["diameter"]) }
        } else {
            Section::Box { width: n(&s["width"]), height: n(&s["height"]) }
        };
        let i = &inlets[k["inlet"].as_str().unwrap()];
        let culvert = Culvert {
            section,
            cells: k["cells"].as_u64().unwrap() as u32,
            length: n(&k["length"]),
            slope: n(&k["slope"]),
            n: n(&k["n"]),
            inlet: Inlet { form: i["form"].as_u64().unwrap() as u8, k: n(&i["k"]), m: n(&i["m"]), c: n(&i["c"]), y: n(&i["y"]), ks: n(&i["ks"]) },
            ke: n(&k["ke"]),
        };
        let w = &c["weir"];
        let weir = Weir { crest: n(&w["crest"]), length: n(&w["length"]), cd: n(&w["cd"]) };
        let got = perform(&culvert, &weir, n(&c["flow"]), n(&c["tailwater"]));
        let e = &c["expected"];
        let close = |a: f64, key: &str| {
            let b = n(&e[key]);
            assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{name}: {key} {a} vs {b}");
        };
        close(got.headwater, "headwater");
        close(got.inlet, "inlet");
        close(got.outlet, "outlet");
        close(got.culvert, "culvert");
        close(got.overtopping, "overtopping");
        close(got.head, "head");
        close(got.depth, "depth");
        close(got.speed, "speed");
        close(got.unit, "unit");
        close(section.critical(n(&c["flow"]) / f64::from(culvert.cells)), "critical");
        let control = if e["control"] == "inlet" { Control::Inlet } else { Control::Outlet };
        assert_eq!(got.control, control, "{name}: control");
        assert_eq!(u64::from(got.hazard), e["hazard"].as_u64().unwrap(), "{name}: hazard");
    }
}

#[test]
fn the_named_inlets_are_the_references_rows() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    let rows = doc["inlets"].as_object().expect("inlets");
    assert_eq!(rows.len(), INLETS.len());
    for (name, i) in rows {
        let got = inlet(name).unwrap_or_else(|| panic!("{name} named"));
        let want = Inlet { form: i["form"].as_u64().unwrap() as u8, k: n(&i["k"]), m: n(&i["m"]), c: n(&i["c"]), y: n(&i["y"]), ks: n(&i["ks"]) };
        assert_eq!(got, want, "{name}");
    }
    assert!(inlet("pipe-mitered").is_none());
}
