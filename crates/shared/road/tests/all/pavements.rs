//! A template's pavement against the shared cases (fixtures/road/v1/pavements.json, docs/adr/0930 §1): written in
//! exact fractions by scripts/fixtures/road_pavement_cases.py, no KentOS code.

use kentos_road::section::GroundPoint;
use kentos_road::section::pavement::{Edge, Pavement, PavementError, build};
use kentos_road::section::template::{self as tpl, Component, Ditch, Side, Template};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/pavements.json");
const WITHIN: f64 = 1e-9;

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn side(v: &Value) -> Side {
    Side {
        components: v["components"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| match c["kind"].as_str().unwrap() {
                "lane" => Component::Lane { width: n(&c["width"]), slope: n(&c["slope"]) },
                _ => Component::Shoulder { width: n(&c["width"]), slope: n(&c["slope"]) },
            })
            .collect(),
        cut: n(&v["cut"]),
        fill: n(&v["fill"]),
        ditch: v.get("ditch").map(|d| Ditch { fore: n(&d["fore"]), depth: n(&d["depth"]), bottom: n(&d["bottom"]) }),
        bench: v.get("bench").map(|b| kentos_road::section::template::CutBench { height: n(&b["height"]), width: n(&b["width"]), fall: b.get("fall").map_or(0.0, n) }),
    }
}

fn ground(v: &Value, lower: f64) -> Vec<GroundPoint> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|g| GroundPoint {
            offset: n(&g["offset"]),
            z: g["z"].as_f64().map(|z| z - lower),
            gap: g["gap"].as_bool().unwrap(),
            crossing: false,
        })
        .collect()
}

fn near(got: f64, want: &Value, what: &str) {
    assert!((got - n(want)).abs() < WITHIN, "{what}: {got} vs {want}");
}

#[test]
fn every_pavement_builds_what_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap();
        let t = Template { left: side(&c["template"]["left"]), right: side(&c["template"]["right"]) };
        let natural = ground(&c["ground"], 0.0);
        let strip = c.get("strip").map_or(0.0, n);
        let stripped = ground(&c["ground"], strip);
        let section = tpl::build(&t, n(&c["axis"]), &natural).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let p = Pavement {
            layers: c["pavement"]["layers"].as_array().unwrap().iter().map(n).collect(),
            edge: c["pavement"]["edge"].as_f64().map_or(Edge::Vertical, Edge::Slope),
        };
        let got = build(&t, &section, &p, &stripped);
        if let Some(e) = c.get("error") {
            let want = if let Some(i) = e.get("layer") {
                PavementError::Layer(i.as_u64().map(|i| i as usize))
            } else if let Some(r) = e.get("edge") {
                PavementError::Edge { right: r.as_bool().unwrap() }
            } else {
                PavementError::NoPlatform { right: e["noPlatform"].as_bool().unwrap() }
            };
            assert_eq!(got, Err(want), "{name}");
            continue;
        }
        let got = got.unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let want = c["layers"].as_array().unwrap();
        assert_eq!(got.layers.len(), want.len(), "{name}");
        for (i, (a, w)) in got.layers.iter().zip(want).enumerate() {
            near(*a, w, &format!("{name}: {}. tabaka", i + 1));
        }
        near(got.below, &c["below"], &format!("{name}: zeminin altında"));
        near(got.above, &c["above"], &format!("{name}: zeminin üstünde"));
        // The earthwork from the formation: the road's against the same ground, plus the pavement under it, less the
        // pavement above it.
        let road = tpl::earthwork(&section, &natural, strip, None);
        near(road.cut, &c["road"]["cut"], &format!("{name}: Yol kazı"));
        near(road.fill, &c["road"]["fill"], &format!("{name}: Yol dolgu"));
        near(road.cut + got.below, &c["formation"]["cut"], &format!("{name}: Formasyon kazı"));
        near(road.fill - got.above, &c["formation"]["fill"], &format!("{name}: Formasyon dolgu"));
        // The formation runs from the left edge to the right through the axis, under the finish grade.
        assert!(got.formation.windows(2).all(|w| w[0][0] <= w[1][0] + 1e-12), "{name}: {:?}", got.formation);
    }
}
