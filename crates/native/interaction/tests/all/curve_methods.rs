//! Daire's, Yay's and Elips's methods end to end through the tools (docs/adr/0289): every case of
//! `fixtures/curve-methods/v1/tools.json` (written by `scripts/fixtures/curve_method_cases.py` from the methods'
//! definitions, without KentOS code) is played with clicks and typed input on the traces' empty drawing; the
//! object the tool writes, or the warning it says, is the reference's.

use crate::common;

use common::{Bench, rel};
use kentos_commands::Entity;
use kentos_interaction::Level;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/curve-methods/v1/tools.json");

fn number(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn pair(v: &Value) -> [f64; 2] {
    [number(&v[0]), number(&v[1])]
}

/// Whether `got` is within `within` of every number `want` names.
fn check(name: &str, got: &[(&str, f64)], want: &Value, within: f64) {
    for (key, value) in got {
        let w = number(&want[*key]);
        assert!(
            (value - w).abs() <= within,
            "{name}: {key} {value}, the reference {w}"
        );
    }
}

fn play(b: &mut Bench, step: &Value) {
    if let Some(at) = step.get("click") {
        let [x, y] = pair(at);
        b.click(x, y);
    } else if let Some(text) = step.get("type").and_then(Value::as_str) {
        assert!(b.type_text(text), "“{text}” is taken");
    } else {
        panic!("a step: {step}");
    }
}

#[test]
fn every_method_draws_the_reference_s_curve() {
    let file: Value = serde_json::from_str(CASES).expect("the cases read");
    let within = number(&file["within"]);
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 30, "{}", cases.len());
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let mut b = Bench::new(case["tool"].as_str().expect("a tool"));
        for object in case["objects"].as_array().expect("objects") {
            let ends = &object["line"];
            b.add_line("cizim", pair(&ends[0]), pair(&ends[1]));
        }
        let count = b.doc.len();
        let made_before = |b: &Bench| b.doc.len();
        for step in case["steps"].as_array().expect("steps") {
            play(&mut b, step);
        }
        let expect = &case["expect"];
        if let Some(words) = expect.get("warn").and_then(Value::as_str) {
            assert_eq!(b.last_level(), Some(Level::Warn), "{name}");
            assert_eq!(b.last_text(), Some(words), "{name}");
            continue;
        }
        assert!(
            made_before(&b) > count,
            "{name}: nothing written ({:?})",
            b.last_text()
        );
        assert_eq!(
            b.last_level(),
            Some(Level::Success),
            "{name}: {:?}",
            b.last_text()
        );
        match b.newest() {
            Entity::Circle(c) => {
                let want = &expect["circle"];
                let [x, y] = rel(c.c);
                let [wx, wy] = pair(&want["c"]);
                check(name, &[("r", c.r)], want, within);
                assert!(
                    (x - wx).abs() <= within && (y - wy).abs() <= within,
                    "{name}: {x}, {y}"
                );
            }
            Entity::Arc(a) => {
                let want = &expect["arc"];
                let [x, y] = rel(a.c);
                let [wx, wy] = pair(&want["c"]);
                check(
                    name,
                    &[("r", a.r), ("a0", a.a0), ("a1", a.a1)],
                    want,
                    within,
                );
                assert!(
                    (x - wx).abs() <= within && (y - wy).abs() <= within,
                    "{name}: {x}, {y}"
                );
            }
            Entity::Ellipse(e) => {
                let want = &expect["ellipse"];
                let [x, y] = rel(e.c);
                let [wx, wy] = pair(&want["c"]);
                let [mx, my] = pair(&want["major"]);
                check(
                    name,
                    &[("ratio", e.ratio), ("t0", e.t0), ("t1", e.t1)],
                    want,
                    within,
                );
                assert!(
                    (x - wx).abs() <= within && (y - wy).abs() <= within,
                    "{name}: {x}, {y}"
                );
                assert!(
                    (e.major.x - mx).abs() <= within && (e.major.y - my).abs() <= within,
                    "{name}: major {:?}",
                    e.major
                );
            }
            other => panic!("{name}: {other:?}"),
        }
    }
}
