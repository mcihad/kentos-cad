//! A haul plan that sorts its material against the shared cases (fixtures/road/v1/haul_sorted.json, docs/adr/0944):
//! written by scripts/fixtures/road_haul_sorted_cases.py as a linear programme (HiGHS), not KentOS's successive
//! shortest paths; and with no unsuitable share and no barrier, the same as the plain plan on the net mass.

use kentos_road::haul_plan::{Barrier, PlanSite, SortedStretch, plan, plan_sorted};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/haul_sorted.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn stretches(v: &Value) -> Vec<SortedStretch> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|r| SortedStretch { from: n(&r["from"]), to: n(&r["to"]), cut: n(&r["cut"]), fill: n(&r["fill"]), unsuitable: n(&r["unsuitable"]) })
        .collect()
}

fn sites(v: &Value) -> Vec<PlanSite> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| PlanSite { station: n(&s["station"]), capacity: n(&s["capacity"]), borrow: s["borrow"].as_bool().unwrap() })
        .collect()
}

#[test]
fn the_sorted_plans_are_the_references() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 15);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let barriers: Vec<Barrier> = c["barriers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| Barrier { station: n(&b["station"]), detour: b["detour"].as_f64() })
            .collect();
        let got = plan_sorted(&stretches(&c["stretches"]), &sites(&c["sites"]), &barriers);
        let e = &c["expected"];
        assert_eq!(got.feasible, e["feasible"].as_bool().unwrap(), "{name}: {got:?}");
        let scale = n(&e["cut"]) + n(&e["fill"]) + 1.0;
        for (key, a) in [("shortage", got.shortage), ("surplus", got.surplus), ("stranded", got.stranded)] {
            assert!((a - n(&e[key])).abs() <= 1e-6 * scale, "{name}: {key} {a} vs {}", e[key]);
        }
        let m = n(&e["moment"]);
        assert!((got.moment - m).abs() <= 1e-6 * m.max(1.0), "{name}: moment {} vs {m}", got.moment);
        // Every place gives or takes what it should: the flows conserve on each line.
        let placed: f64 = got.used.iter().zip(sites(&c["sites"])).map(|(u, s)| if s.borrow { *u } else { 0.0 }).sum();
        assert!(placed >= -1e-9, "{name}");
    }
}

#[test]
fn with_nothing_unsuitable_and_no_barrier_it_is_the_plain_plan() {
    let mut seed = 7u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..20 {
        let rows: Vec<SortedStretch> = (0..30)
            .map(|i| {
                let a = 25.0 * i as f64;
                let cut = if next() < 0.5 { (next() * 800.0).round() } else { 0.0 };
                let fill = if next() < 0.5 { (next() * 800.0).round() } else { 0.0 };
                SortedStretch { from: a, to: a + 25.0, cut, fill, unsuitable: 0.0 }
            })
            .collect();
        let sites = [
            PlanSite { station: -40.0, capacity: 1e5, borrow: false },
            PlanSite { station: 800.0, capacity: 1e5, borrow: true },
        ];
        let mut points = vec![[rows[0].from, 0.0]];
        let mut mass = 0.0;
        for r in &rows {
            mass += r.cut - r.fill;
            points.push([r.to, mass]);
        }
        let plain = plan(&points, &sites);
        let sorted = plan_sorted(&rows, &sites, &[]);
        assert!(plain.feasible && sorted.feasible);
        assert!((plain.moment - sorted.moment).abs() <= 1e-9 * plain.moment.max(1.0), "{} vs {}", plain.moment, sorted.moment);
    }
}
