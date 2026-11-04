//! A corridor's adaptive sampling against the shared cases (fixtures/road/v1/sampling.json, docs/adr/0939): written by
//! scripts/fixtures/road_sampling_cases.py from the ADR's rules on sections given by closed forms, no KentOS code.

use kentos_road::profile::vertical::Piece;
use kentos_road::sampling::{Coded, Left, Measured, Stray, Tolerance, refine, stray, vertical_divisions, vertical_step};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/sampling.json");
const WITHIN: f64 = 1e-9;

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn opt(v: &Value) -> Option<f64> {
    v.as_f64()
}

/// The case's section at a station, as the reference writes it.
fn coded(family: &Value, s: f64) -> Coded {
    if let Some(none) = family["none"].as_array()
        && none.iter().any(|r| n(&r[0]) <= s && s < n(&r[1]))
    {
        return None;
    }
    let ((ax, ay), (nx, ny)) = match family["radius"].as_f64() {
        None => ((s, 0.0), (0.0, -1.0)),
        Some(r) => {
            let th = s / r;
            ((r * th.sin(), r * (1.0 - th.cos())), (th.sin(), -th.cos()))
        }
    };
    let get = |k: &str, d: f64| family[k].as_f64().unwrap_or(d);
    let (z0, g, c, a, lam) = (get("z0", 100.0), get("grade", 0.0), get("bend", 0.0), get("amplitude", 0.0), get("wave", 1.0));
    let mut out = Vec::new();
    for p in family["points"].as_array().expect("points") {
        let from = p["from"].as_f64().unwrap_or(f64::NEG_INFINITY);
        let to = p["to"].as_f64().unwrap_or(f64::INFINITY);
        if from <= s && s < to {
            let o = n(&p["offset"]);
            let z = z0 + g * s + c * s * s / 2.0 + a * (2.0 * std::f64::consts::PI * s / lam).sin() + p["dz"].as_f64().unwrap_or(0.0);
            out.push((p["code"].as_str().expect("code").to_owned(), [ax + o * nx, ay + o * ny, z]));
        }
    }
    Some(out)
}

fn same_stray(got: &Stray, want: &Value, what: &str) {
    assert!((got.plan - n(&want["plan"])).abs() < WITHIN, "{what}: plan {} ≠ {}", got.plan, want["plan"]);
    assert!((got.height - n(&want["height"])).abs() < WITHIN, "{what}: height {} ≠ {}", got.height, want["height"]);
    assert_eq!(got.plan_code.as_deref(), want["planCode"].as_str(), "{what}: plan code");
    assert_eq!(got.height_code.as_deref(), want["heightCode"].as_str(), "{what}: height code");
    assert_eq!(got.changed, want["changed"].as_bool().expect("changed"), "{what}: changed");
}

fn same_measured(got: &Measured, want: &Value, what: &str) {
    assert!((got.from - n(&want["from"])).abs() < WITHIN && (got.to - n(&want["to"])).abs() < WITHIN, "{what}: {got:?} ≠ {want}");
    same_stray(&got.stray, want, what);
    assert_eq!(got.within, want["within"].as_bool().expect("within"), "{what}: within");
}

#[test]
fn the_vertical_step_keeps_the_chord_within_its_height() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    for c in doc["verticals"].as_array().expect("verticals") {
        let got = vertical_step(n(&c["tolerance"]), n(&c["bend"]));
        match c["expected"].as_f64() {
            None => assert!(got.is_infinite(), "{c}"),
            Some(want) => assert!((got - want).abs() < WITHIN, "{c}: {got}"),
        }
    }
}

#[test]
fn a_middle_strays_as_the_reference_measures_it() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    for c in doc["strays"].as_array().expect("strays") {
        let f = &c["family"];
        let [a, m, b] = [n(&c["at"][0]), n(&c["at"][1]), n(&c["at"][2])];
        same_stray(&stray(&coded(f, a), &coded(f, m), &coded(f, b)), &c["expected"], c["name"].as_str().unwrap());
    }
}

#[test]
fn refinement_adds_the_stations_the_reference_adds() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20, "at least 20 cases: {}", cases.len());
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let f = &c["family"];
        let t = &c["tolerance"];
        let tol = Tolerance { plan: opt(&t["plan"]), height: opt(&t["height"]), least: n(&t["least"]) };
        let stations: Vec<(f64, f64)> = c["stations"].as_array().unwrap().iter().map(|s| (n(s), n(s))).collect();
        let got = refine(&stations, &tol, c["budget"].as_u64().unwrap() as usize, |s| s, |s| coded(f, *s));
        let e = &c["expected"];
        let added: Vec<f64> = got.added.iter().map(|a| a.0).collect();
        let want: Vec<f64> = e["added"].as_array().unwrap().iter().map(n).collect();
        assert_eq!(added.len(), want.len(), "{name}: added {added:?} ≠ {want:?}");
        for (g, w) in added.iter().zip(&want) {
            assert!((g - w).abs() < WITHIN, "{name}: added {added:?} ≠ {want:?}");
        }
        // What was built there is what make gave for that station.
        assert!(got.added.iter().all(|(s, t)| s == t), "{name}");
        for (key, list) in [("before", &got.before), ("after", &got.after)] {
            let want = e[key].as_array().unwrap();
            assert_eq!(list.len(), want.len(), "{name}: {key}");
            for (g, w) in list.iter().zip(want) {
                same_measured(g, w, &format!("{name}: {key}"));
            }
        }
        let want = e["left"].as_array().unwrap();
        assert_eq!(got.left.len(), want.len(), "{name}: left {:?}", got.left);
        for ((m, why), w) in got.left.iter().zip(want) {
            let word = match why {
                Left::Least => "least",
                Left::Budget => "budget",
            };
            assert_eq!(word, w["why"].as_str().unwrap(), "{name}: left");
            assert!((m.from - n(&w["from"])).abs() < WITHIN && (m.to - n(&w["to"])).abs() < WITHIN, "{name}: left {m:?}");
            if *why == Left::Least {
                same_measured(m, w, &format!("{name}: left"));
            }
        }
    }
}

#[test]
fn vertical_curves_are_divided_by_their_chord_height() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    for c in doc["divisions"].as_array().expect("divisions") {
        let pieces: Vec<Piece> = c["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let (s0, s1, z0) = (n(&p["s0"]), n(&p["s1"]), p["z0"].as_f64().unwrap_or(0.0));
                match p["kind"].as_str().unwrap() {
                    "grade" => Piece::Grade { s0, s1, z0, g: n(&p["g"]) },
                    "parabola" => Piece::Parabola { s0, s1, z0, g0: n(&p["g0"]), r: n(&p["r"]) },
                    _ => Piece::Circle {
                        s0,
                        s1,
                        sc: n(&p["sc"]),
                        zc: n(&p["zc"]),
                        radius: n(&p["radius"]),
                        sag: p["sag"].as_bool().unwrap(),
                    },
                }
            })
            .collect();
        let got = vertical_divisions(&pieces, n(&c["tolerance"]), [n(&c["region"][0]), n(&c["region"][1])]);
        let want: Vec<f64> = c["expected"].as_array().unwrap().iter().map(n).collect();
        assert_eq!(got.len(), want.len(), "{}: {got:?}", c["name"]);
        assert!(got.iter().zip(&want).all(|(g, w)| (g - w).abs() < WITHIN), "{}: {got:?}", c["name"]);
    }
}
