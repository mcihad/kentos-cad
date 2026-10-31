//! The design change at 1, 20 and 100 apartments without a window (docs/adr/0782): the 2+1 apartment of the
//! acceptance flow (fixtures/acceptance/v1/flat.json) drawn by its commands, copied into rows of ten, and a
//! partition moved with Mesafe and Bu ölçüyü koru, an attribute written on it, each undone. Not run by default:
//!
//! ```text
//! cargo test --release -p kentos-headless --test headless flat_timing -- --ignored --nocapture
//! ```
//!
//! `KENTOS_PERF_FLATS` (comma separated) changes the counts, `KENTOS_PERF_REPEAT` the repetitions;
//! `KENTOS_PERF_SCHEDULES=each` copies the flat's two schedules with it (200 at 100 flats, as `perf::flat` did in ADR 0776)
//! instead of keeping the building's two.

use std::collections::BTreeMap;
use std::time::Instant;

use kentos_headless::{Op, Session};
use serde_json::{Value, json};

const SCENARIO: &str = include_str!("../../../../../fixtures/acceptance/v1/flat.json");
const STEP: [f64; 2] = [14.0, 12.0];

fn bind(value: &Value, names: &BTreeMap<String, String>) -> Value {
    match value {
        Value::String(s) if s.starts_with('$') => json!(names[&s[1..]]),
        Value::Object(fields) => Value::Object(fields.iter().map(|(k, v)| (k.clone(), bind(v, names))).collect()),
        Value::Array(items) => Value::Array(items.iter().map(|v| bind(v, names)).collect()),
        other => other.clone(),
    }
}

fn bound(output: &Value, key: &Value, names: &mut BTreeMap<String, String>) {
    let text = |v: &Value| v.as_str().expect("an id").to_owned();
    match key {
        Value::String(name) => {
            let uid = [&output["uid"], &output["set"], &output["uids"][0]]
                .into_iter()
                .find(|v| v.is_string())
                .expect("an id");
            names.insert(name.clone(), text(uid));
        }
        Value::Array(keys) => {
            let ids = output["created"].as_array().or(output["uids"].as_array()).expect("ids");
            for (k, u) in keys.iter().zip(ids) {
                names.insert(text(k), text(u));
            }
        }
        _ => {}
    }
}

fn run(s: &mut Session, command: &str, op: Op, input: Value) -> Value {
    let answer = s.run(command, None, op, input).unwrap_or_else(|e| panic!("{command}: {e:?}"));
    assert_eq!(answer["status"], "completed", "{command}: {answer}");
    answer
}

/// The apartment as the scenario draws it up to the design change; the names it binds.
pub(crate) fn drawn() -> (Session, BTreeMap<String, String>) {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let choices = kentos_headless::new_drawing(&scenario["project"]).expect("a project");
    let mut s = Session::new(&choices).expect("a drawing");
    let mut names = BTreeMap::new();
    names.insert("layer".to_owned(), s.summary()["activeLayer"].as_str().expect("a layer").to_owned());
    for step in scenario["steps"].as_array().expect("steps") {
        let Some(command) = step["command"].as_str() else { continue };
        if step["label"] == "Duvarı düzenle" {
            break;
        }
        if step["refuse"].is_string() {
            continue;
        }
        let answer = run(&mut s, command, Op::Execute, bind(&step["input"], &names));
        bound(&answer["output"], &step["as"], &mut names);
    }
    (s, names)
}

fn uids(s: &Session, tables: bool) -> Vec<String> {
    s.document()
        .entities_with_uids()
        .filter(|(_, e)| tables || !matches!(e, kentos_commands::Entity::Table(_)))
        .map(|(u, _)| u.to_string())
        .collect()
}

/// The apartment copied into `n` (rows of ten), as `perf::flat` copies it: with its two schedules each time
/// (`tables`), or the building's two schedules once, reading every flat.
pub(crate) fn copied_with(s: &mut Session, n: usize, tables: bool) {
    let one = uids(s, tables);
    for c in 1..n.min(10) {
        run(s, "cad.entities.transform", Op::Execute, json!({ "uids": one,
            "transform": { "kind": "move", "dx": STEP[0] * c as f64, "dy": 0.0 }, "copy": true }));
    }
    let row = uids(s, tables);
    for r in 1..n.div_ceil(10) {
        run(s, "cad.entities.transform", Op::Execute, json!({ "uids": row,
            "transform": { "kind": "move", "dx": 0.0, "dy": STEP[1] * r as f64 }, "copy": true }));
    }
}

/// The apartment copied into `n` with its schedules each time.
pub(crate) fn copied(s: &mut Session, n: usize) {
    copied_with(s, n, true);
}

/// The process's resident memory, MB (`ps`).
fn resident() -> f64 {
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|t| t.trim().parse::<f64>().ok())
        .map_or(f64::NAN, |kb| kb / 1024.0)
}

/// The resident memory's peak while `work` runs, MB: `ps` read every 5 ms on a thread.
fn peak<T>(work: impl FnOnce() -> T) -> (T, f64) {
    use std::sync::atomic::{AtomicBool, Ordering};
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let watch = {
        let stop = std::sync::Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut most = resident();
            while !stop.load(Ordering::Relaxed) {
                most = most.max(resident());
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            most.max(resident())
        })
    };
    let out = work();
    stop.store(true, Ordering::Relaxed);
    (out, watch.join().unwrap_or(f64::NAN))
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn flat_timing() {
    let counts: Vec<usize> = std::env::var("KENTOS_PERF_FLATS")
        .unwrap_or_else(|_| "1,20,100".into())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let repeat: usize = std::env::var("KENTOS_PERF_REPEAT").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
    for n in counts {
        let (mut s, names) = drawn();
        let each = std::env::var("KENTOS_PERF_SCHEDULES").is_ok_and(|v| v == "each");
        copied_with(&mut s, n, each);
        let p3 = names["p3"].clone();
        let objects = s.document().entities().count();
        let mut rows = serde_json::Map::new();
        rows.insert("residentMb".into(), json!(resident()));
        let runs: [(&str, &str, Op, Value); 4] = [
            ("distanceMs", "cad.walls.edit", Op::Execute,
                json!({ "uids": [p3], "operation": { "kind": "distance", "side": "right", "distance": 3.7 } })),
            ("planMs", "cad.walls.edit", Op::Plan,
                json!({ "uids": [p3], "operation": { "kind": "distance", "side": "right", "distance": 3.7 } })),
            ("shiftMs", "cad.walls.edit", Op::Execute, json!({ "uids": [p3], "operation": { "kind": "shift", "by": 0.05 } })),
            ("attributeMs", "cad.entities.set", Op::Execute,
                json!({ "uids": [p3], "operation": "attributes", "attrs": { "Not": "x" } })),
        ];
        for (name, command, op, input) in runs {
            let mut took = Vec::new();
            let mut undo = Vec::new();
            let ((), most) = peak(|| for _ in 0..repeat {
                let t = Instant::now();
                run(&mut s, command, op, input.clone());
                took.push(t.elapsed().as_secs_f64() * 1e3);
                if op == Op::Execute {
                    let t = Instant::now();
                    s.undo();
                    undo.push(t.elapsed().as_secs_f64() * 1e3);
                }
            });
            rows.insert(name.into(), json!(median(took)));
            rows.insert(format!("{name}PeakMb"), json!(most));
            if !undo.is_empty() {
                rows.insert(format!("{name}Undo"), json!(median(undo)));
            }
        }
        let tables = s.document().entities().filter(|e| matches!(e, kentos_commands::Entity::Table(_))).count();
        println!("FLAT {n} ({objects} objects, {tables} tables): {}", Value::Object(rows));
    }
}
