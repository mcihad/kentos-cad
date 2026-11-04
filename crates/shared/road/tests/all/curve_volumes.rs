//! A section's cut and fill moments and their volume on a curve against the shared cases
//! (fixtures/road/v1/curve-volumes.json, docs/adr/0934 §1): written by scripts/fixtures/road_curve_volume_cases.py,
//! which assembles the regions as polygons (shoelace) in fractions and sweeps them as solids of revolution.

use kentos_road::section::GroundPoint;
use kentos_road::section::template::{self as tpl, Component, Ditch, Side, Template};
use kentos_road::volume::{CurvedSection, end_areas_curved};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/curve-volumes.json");

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
    }
}

/// The case's section built, its ground, and its moments.
fn built(c: &Value) -> (tpl::Section, Vec<GroundPoint>) {
    let t = Template { left: side(&c["template"]["left"]), right: side(&c["template"]["right"]) };
    let ground: Vec<GroundPoint> = c["ground"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| GroundPoint { offset: n(&g["offset"]), z: g["z"].as_f64(), gap: g["gap"].as_bool().unwrap(), crossing: false })
        .collect();
    (tpl::build(&t, n(&c["axis"]), &ground).expect("builds"), ground)
}

#[test]
fn the_moments_are_the_polygons() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for c in all["sections"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let (section, ground) = built(c);
        let (qc, qf) = tpl::moments(&section, &ground, 0.0);
        assert!((section.cut - n(&c["cut"])).abs() < 1e-9 && (section.fill - n(&c["fill"])).abs() < 1e-9, "{name}");
        assert!((qc - n(&c["cutMoment"])).abs() < 1e-9, "{name}: cut moment {qc} vs {}", c["cutMoment"]);
        assert!((qf - n(&c["fillMoment"])).abs() < 1e-9, "{name}: fill moment {qf} vs {}", c["fillMoment"]);
    }
}

/// A constant section on a circular arc: the effective areas' end areas are the solid of revolution's volume (Pappus).
#[test]
fn a_constant_section_on_a_circle_is_pappus() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    let sections: Vec<&Value> = all["sections"].as_array().unwrap().iter().collect();
    for c in all["swept"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let s = sections.iter().find(|x| x["name"] == c["section"]).expect("its section");
        let (section, ground) = built(s);
        let moments = tpl::moments(&section, &ground, 0.0);
        let k = if c["left"].as_bool().unwrap() { 1.0 } else { -1.0 } / n(&c["radius"]);
        let reach = (n(&s["reach"][0]), n(&s["reach"][1]));
        let at = |station: f64| CurvedSection { station, areas: Some((section.cut, section.fill)), moments, curvature: k, reach };
        let got = end_areas_curved(&[at(0.0), at(n(&c["length"]))]);
        if c.get("past").is_some() {
            assert_eq!(got[0].volumes, None, "{name}: past the centre is left out");
            continue;
        }
        let (cut, fill) = got[0].volumes.expect("a volume");
        assert!((cut - n(&c["cut"])).abs() < 1e-7 && (fill - n(&c["fill"])).abs() < 1e-7, "{name}: {cut} {fill} vs {c}");
    }
}
