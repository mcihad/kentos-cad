//! A volume's ledger against the shared cases (fixtures/road/v1/volume_ledger.json, docs/adr/0949): written by
//! scripts/fixtures/road_volume_ledger_cases.py in exact fractions, no KentOS code.

use kentos_road::volume_ledger::{Stretch, concentration, group, reasons, rounding, steps, sum};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/volume_ledger.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() <= 1e-12 * b.abs().max(1.0), "{what}: {a} vs {b}");
}

#[test]
fn the_ledgers_are_the_references() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    for c in doc["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap();
        let stretches: Vec<Stretch> = c["stretches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| Stretch {
                from: n(&s["from"]),
                to: n(&s["to"]),
                values: s["values"].as_array().unwrap().iter().map(Value::as_f64).collect(),
            })
            .collect();
        let ranges: Vec<(f64, f64)> = match c["step"].as_f64() {
            Some(step) => steps(stretches[0].from, stretches.last().unwrap().to, step),
            None => c["ranges"].as_array().unwrap().iter().map(|r| (n(&r[0]), n(&r[1]))).collect(),
        };
        let want: Vec<(f64, f64)> = c["ranges"].as_array().unwrap().iter().map(|r| (n(&r[0]), n(&r[1]))).collect();
        assert_eq!(ranges, want, "{name}: ranges");
        let groups = group(&stretches, &ranges);
        let wg = c["groups"].as_array().unwrap();
        assert_eq!(groups.len(), wg.len(), "{name}: groups");
        for (g, w) in groups.iter().zip(wg) {
            assert_eq!((g.from, g.to), (n(&w["from"]), n(&w["to"])), "{name}: group range");
            let members: Vec<usize> = w["members"].as_array().unwrap().iter().map(|m| m.as_u64().unwrap() as usize).collect();
            assert_eq!(g.members, members, "{name}: members");
            for (a, b) in g.values.iter().zip(w["values"].as_array().unwrap()) {
                close(*a, n(b), &format!("{name}: group value"));
            }
        }
        // The groups add up to the total.
        let columns = stretches[0].values.len();
        for k in 0..columns {
            let total = sum(stretches.iter().filter_map(|s| s.values[k]));
            close(sum(groups.iter().map(|g| g.values[k])), total, &format!("{name}: reconciles"));
        }
        let lengths: Vec<f64> = stretches.iter().map(|s| s.to - s.from).collect();
        let decimals = c["decimals"].as_u64().unwrap() as u32;
        for k in 0..columns {
            let vols: Vec<f64> = stretches.iter().map(|s| s.values[k].unwrap_or(0.0)).collect();
            let wc = &c["concentration"][k];
            match (concentration(&lengths, &vols), wc.is_null()) {
                (None, true) => {}
                (Some(got), false) => {
                    close(got.gini, n(&wc["gini"]), &format!("{name}: gini"));
                    close(got.half, n(&wc["half"]), &format!("{name}: half"));
                    close(got.most, n(&wc["most"]), &format!("{name}: most"));
                    let curve = wc["curve"].as_array().unwrap();
                    assert_eq!(got.curve.len(), curve.len(), "{name}: curve");
                    for (p, q) in got.curve.iter().zip(curve) {
                        close(p.0, n(&q[0]), &format!("{name}: curve x"));
                        close(p.1, n(&q[1]), &format!("{name}: curve y"));
                    }
                }
                other => panic!("{name}: concentration {:?}", other.0),
            }
            if let Some(wr) = c["reasons"][k].as_array() {
                for (r, w) in reasons(&lengths, &vols).iter().zip(wr) {
                    match (r, w.is_null()) {
                        (None, true) => {}
                        (Some(r), false) => {
                            close(r.length, n(&w["length"]), &format!("{name}: length factor"));
                            close(r.area, n(&w["area"]), &format!("{name}: area factor"));
                        }
                        other => panic!("{name}: reason {other:?}"),
                    }
                }
            }
            let present: Vec<f64> = stretches.iter().filter_map(|s| s.values[k]).collect();
            let (rows, total, residue) = rounding(&present, decimals);
            let wr = &c["rounding"][k];
            close(rows, n(&wr[0]), &format!("{name}: rounded rows"));
            close(total, n(&wr[1]), &format!("{name}: rounded total"));
            close(residue, n(&wr[2]), &format!("{name}: residue"));
        }
    }
}
