//! A road's superelevation against the shared cases (fixtures/road/v1/superelevation.json, docs/adr/0904): written in
//! exact fractions of the reference alignment's stations by scripts/fixtures/road_superelevation_cases.py, no KentOS
//! code.

use kentos_road::alignment::{PiInput, build};
use kentos_road::superelevation::{Crossfall, CurveRule, Mark, Row, SuperError, from_curves, from_rows};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/superelevation.json");
const WITHIN: f64 = 1e-9;

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn pis(v: &Value) -> Vec<PiInput> {
    v.as_array()
        .expect("pis")
        .iter()
        .map(|p| PiInput {
            id: p["id"].as_str().expect("id").to_owned(),
            x: n(&p["x"]),
            y: n(&p["y"]),
            radius: n(&p["radius"]),
            spiral_in: n(&p["spiralIn"]),
            spiral_out: n(&p["spiralOut"]),
        })
        .collect()
}

fn kind(e: &SuperError) -> Value {
    match e {
        SuperError::NoCurve { pi } => serde_json::json!({"kind": "NoCurve", "pi": pi}),
        SuperError::Bad { pi } => serde_json::json!({"kind": "Bad", "pi": pi}),
        SuperError::RateBelowNormal { pi } => serde_json::json!({"kind": "RateBelowNormal", "pi": pi}),
        SuperError::TooShort { pi } => serde_json::json!({"kind": "TooShort", "pi": pi}),
        SuperError::Overlap { a, b } => serde_json::json!({"kind": "Overlap", "a": a, "b": b}),
        SuperError::OffAlignment { pi } => serde_json::json!({"kind": "OffAlignment", "pi": pi}),
        SuperError::Twice { pi } => serde_json::json!({"kind": "Twice", "pi": pi}),
        SuperError::BadRow { row } => serde_json::json!({"kind": "BadRow", "row": row}),
    }
}

fn points(got: &[[f64; 2]], want: &Value, what: &str) {
    let want = want.as_array().expect("points");
    assert_eq!(got.len(), want.len(), "{what}: {got:?} vs {want:?}");
    for (a, b) in got.iter().zip(want) {
        assert!((a[0] - n(&b[0])).abs() < WITHIN && (a[1] - n(&b[1])).abs() < WITHIN, "{what}: {a:?} vs {b}");
    }
}

#[test]
fn every_schedule_is_what_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap();
        let normal = Crossfall { left: n(&c["normal"][0]), right: n(&c["normal"][1]) };
        let al = build(&pis(&c["pis"])).expect("the alignment builds");
        let got = if let Some(rows) = c.get("rows") {
            let rows: Vec<Row> = rows
                .as_array()
                .unwrap()
                .iter()
                .map(|r| Row { station: n(&r["station"]), left: n(&r["left"]), right: n(&r["right"]) })
                .collect();
            from_rows(&rows).map(|s| (s, Vec::new()))
        } else {
            let rules: Vec<CurveRule> = c["rules"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| CurveRule {
                    pi: r["pi"].as_str().unwrap().to_owned(),
                    rate: n(&r["rate"]),
                    runoff: n(&r["runoff"]),
                    on_tangent: n(&r["onTangent"]),
                    widening: n(&r["widening"]),
                })
                .collect();
            from_curves(&al, normal, &rules)
        };
        match (got, c.get("error")) {
            (Err(e), Some(want)) => assert_eq!(&kind(&e), want, "{name}"),
            (Err(e), None) => panic!("{name}: refused {e:?}"),
            (Ok(_), Some(want)) => panic!("{name}: built, expected {want}"),
            (Ok((s, critical)), None) => {
                let w = &c["schedule"];
                points(&s.left, &w["left"], &format!("{name}: left"));
                points(&s.right, &w["right"], &format!("{name}: right"));
                points(&s.left_widening, &w["leftWidening"], &format!("{name}: left widening"));
                points(&s.right_widening, &w["rightWidening"], &format!("{name}: right widening"));
                let want = c["critical"].as_array().unwrap();
                assert_eq!(critical.len(), want.len(), "{name}: critical");
                for (a, b) in critical.iter().zip(want) {
                    let mark = match a.mark {
                        Mark::NormalCrown => "normalCrown",
                        Mark::LevelCrown => "levelCrown",
                        Mark::ReverseCrown => "reverseCrown",
                        Mark::Full => "full",
                    };
                    assert_eq!((a.pi.as_str(), mark, a.exit), (b["pi"].as_str().unwrap(), b["mark"].as_str().unwrap(), b["exit"].as_bool().unwrap()), "{name}");
                    assert!((a.station - n(&b["station"])).abs() < WITHIN, "{name}: {a:?} vs {b}");
                }
                for row in c["at"].as_array().unwrap() {
                    let s0 = n(&row["station"]);
                    let x = s.at(s0, normal);
                    let (wl, wr) = s.widening(s0);
                    for (g, key) in [(x.left, "left"), (x.right, "right"), (wl, "leftWidening"), (wr, "rightWidening")] {
                        assert!((g - n(&row[key])).abs() < WITHIN, "{name} at {s0}: {key} {g} vs {}", row[key]);
                    }
                }
            }
        }
    }
}
