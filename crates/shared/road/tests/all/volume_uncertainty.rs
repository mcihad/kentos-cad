//! An earthwork volume's sensitivity to the ground's heights against the shared cases
//! (fixtures/road/v1/volume_uncertainty.json, docs/adr/0941): written by scripts/fixtures/road_volume_uncertainty_cases.py
//! in exact fractions and checked there by finite differences and 4000 Monte Carlo trials, no KentOS code.

use kentos_road::volume::sensitivity;
use kentos_road::volume_uncertainty::{bias, spread};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/volume_uncertainty.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

#[test]
fn the_weights_and_spreads_are_the_references() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let vertices: Vec<[f64; 3]> = c["vertices"].as_array().unwrap().iter().map(|v| [n(&v[0]), n(&v[1]), n(&v[2])]).collect();
        let triangles: Vec<[u32; 3]> = c["triangles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| [t[0].as_u64().unwrap() as u32, t[1].as_u64().unwrap() as u32, t[2].as_u64().unwrap() as u32])
            .collect();
        let design: Vec<[[f64; 3]; 3]> = c["design"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                let p = |k: usize| [n(&t[k][0]), n(&t[k][1]), n(&t[k][2])];
                [p(0), p(1), p(2)]
            })
            .collect();
        let (sigma, beta) = (n(&c["sigma"]), n(&c["beta"]));
        let got = sensitivity(&design, &vertices, &triangles, 2.0 * sigma);
        let e = &c["expected"];
        for (key, w) in [("cut", &got.cut), ("fill", &got.fill)] {
            let want: Vec<f64> = e[key].as_array().unwrap().iter().map(n).collect();
            for (j, (a, b)) in w.iter().zip(&want).enumerate() {
                assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{name}: {key}[{j}] {a} ≠ {b}");
            }
        }
        assert!(close(got.cut_area, n(&e["cutArea"])) && close(got.fill_area, n(&e["fillArea"])), "{name}: areas");
        assert!(close(got.near_area, n(&e["nearArea"])), "{name}: near {} ≠ {}", got.near_area, e["nearArea"]);
        let net: Vec<f64> = got.cut.iter().zip(&got.fill).map(|(a, b)| a - b).collect();
        assert!(close(spread(&got.cut, sigma), n(&e["cutSigma"])), "{name}: σ cut");
        assert!(close(spread(&got.fill, sigma), n(&e["fillSigma"])), "{name}: σ fill");
        assert!(close(spread(&net, sigma), n(&e["netSigma"])), "{name}: σ net");
        assert!(close(bias(&got.cut, beta), n(&e["cutBias"])) && close(bias(&got.fill, beta), n(&e["fillBias"])), "{name}: β");
    }
}
