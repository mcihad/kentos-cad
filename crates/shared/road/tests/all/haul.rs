//! A road's mass haul diagram against the shared cases (fixtures/road/v1/haul.json, docs/adr/0907): worked out in
//! exact fractions by scripts/fixtures/road_haul_cases.py, no KentOS code.

use kentos_road::haul::diagram;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/haul.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn near(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() <= 1e-9 * want.abs().max(1.0), "{what}: {got} ≠ {want}");
}

#[test]
fn mass_haul_diagrams_are_the_references() {
    let file: Value = serde_json::from_str(CASES).expect("cases");
    for c in file["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap_or_default();
        let points: Vec<[f64; 2]> =
            c["points"].as_array().expect("points").iter().map(|p| [n(&p[0]), n(&p[1])]).collect();
        let d = diagram(&points, n(&c["balance"]), n(&c["free"]));
        let e = &c["expect"];
        let balance: Vec<f64> = e["balance"].as_array().expect("balance").iter().map(n).collect();
        assert_eq!(d.balance.len(), balance.len(), "{name}: balance points {:?} ≠ {balance:?}", d.balance);
        for (g, w) in d.balance.iter().zip(&balance) {
            near(*g, *w, &format!("{name}: balance point"));
        }
        let loops = e["loops"].as_array().expect("loops");
        assert_eq!(d.loops.len(), loops.len(), "{name}: loops {:?}", d.loops);
        for (g, w) in d.loops.iter().zip(loops) {
            assert_eq!(g.forward, w["forward"].as_bool().unwrap_or_default(), "{name}: direction");
            for (got, key) in [
                (g.from, "from"),
                (g.to, "to"),
                (g.volume, "volume"),
                (g.moment, "moment"),
                (g.over_volume, "overVolume"),
                (g.over_moment, "overMoment"),
                (g.unbalanced, "unbalanced"),
            ] {
                near(got, n(&w[key]), &format!("{name}: loop {key}"));
            }
        }
        for (got, key) in [
            (d.volume, "volume"),
            (d.moment, "moment"),
            (d.over_volume, "overVolume"),
            (d.over_moment, "overMoment"),
            (d.waste, "waste"),
            (d.borrow, "borrow"),
        ] {
            near(got, n(&e[key]), &format!("{name}: {key}"));
        }
    }
}
