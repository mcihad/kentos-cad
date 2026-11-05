//! The corridor volume's sampling convergence corpus (fixtures/road/v1/volume-convergence.json, docs/adr/0971,
//! ROAD-QA-03): written in exact fractions and mpmath by scripts/fixtures/road_volume_convergence_cases.py, no KentOS
//! code. KentOS's red line (`Vertical`), sections (`template::build` on flat ground) and sums (`volume::end_areas`,
//! `volume::prismoidal`) at each station interval give the reference's sums; the reference checks their convergence
//! to the exact volume (h² for average end areas, h⁴ for the prismoidal formula where the area is smooth).

use kentos_road::profile::vertical::{Pvi, VCurve, build};
use kentos_road::section::sample::GroundPoint;
use kentos_road::section::template::{self, Component, Side, Template};
use kentos_road::volume::{self, SectionAreas};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/volume-convergence.json");

#[test]
fn the_sums_at_every_interval_are_the_reference_s() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["cases"].as_array().expect("cases") {
        let n = |v: &Value| v.as_f64().expect("a number");
        let name = c["name"].as_str().unwrap();
        let pvis: Vec<Pvi> = c["pvis"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Pvi {
                id: p["id"].as_str().unwrap().into(),
                station: n(&p["station"]),
                elevation: n(&p["elevation"]),
                curve: p.get("length").map_or(VCurve::None, |l| VCurve::Parabola { length: n(l) }),
            })
            .collect();
        let v = build(&pvis).expect("builds");
        let side = Side {
            components: vec![Component::Lane { width: n(&c["width"]) / 2.0, slope: 0.0 }],
            cut: n(&c["cut"]),
            fill: n(&c["fill"]),
            ditch: None,
            bench: None,
            fill_bench: None,
        };
        let t = Template { left: side.clone(), right: side };
        let g = n(&c["ground"]);
        let ground = [-400.0, 400.0].map(|o| GroundPoint { offset: o, z: Some(g), gap: false, crossing: false });
        let areas = |s: f64| {
            let sec = template::build(&t, v.at(s).expect("on the profile").0, &ground).expect("a section");
            (sec.cut, sec.fill)
        };
        let close = |got: f64, want: f64, what: &str| {
            assert!((got - want).abs() <= 1e-9 * want.abs().max(1.0), "{name} {what}: {got} vs {want}");
        };
        for row in c["sums"].as_array().unwrap() {
            let h = n(&row["step"]);
            let k = ((v.end - v.start) / h).round() as usize;
            let st: Vec<f64> = (0..=k).map(|i| v.start + h * i as f64).collect();
            let sections: Vec<SectionAreas> = st.iter().map(|s| SectionAreas { station: *s, areas: Some(areas(*s)) }).collect();
            let (mut cut, mut fill) = (0.0, 0.0);
            for b in volume::end_areas(&sections) {
                let (c, f) = b.volumes.expect("solved");
                cut += c;
                fill += f;
            }
            close(cut, n(&row["endAreas"][0]), &format!("h {h} kazı, ortalama alan"));
            close(fill, n(&row["endAreas"][1]), &format!("h {h} dolgu, ortalama alan"));
            let (mut cut, mut fill) = (0.0, 0.0);
            for w in st.windows(2) {
                let (a0, am, a1) = (areas(w[0]), areas(0.5 * (w[0] + w[1])), areas(w[1]));
                cut += volume::prismoidal(a0.0, am.0, a1.0, w[1] - w[0]);
                fill += volume::prismoidal(a0.1, am.1, a1.1, w[1] - w[0]);
            }
            close(cut, n(&row["prismoidal"][0]), &format!("h {h} kazı, prizmoidal"));
            close(fill, n(&row["prismoidal"][1]), &format!("h {h} dolgu, prizmoidal"));
        }
    }
}
