//! Zamansal katmanların zamanı (docs/adr/0210 §3–§5) against fixtures/temporal/v1/cases.json
//! (`scripts/fixtures/temporal_cases.py`: the ADR's grammar and calendar in Python, no KentOS code):
//! values read, written and shown, moments rounded down, the slider's positions, its last index and
//! opening step, objects' times, the window's table and a layer's summary. Everything exactly: moments
//! are whole milliseconds.

use kentos_geometry_core::time::{
    Mode, Read, Rule, Step, Time, Unit, Window, auto_step, floor_to, layer_times, object_time,
    position, positions, read, show, shows, write,
};
use serde_json::Value;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/temporal/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON");
    assert_eq!(file["format"], "kentos.temporal-cases");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    match v {
        Value::String(s) if s == "inf" => f64::INFINITY,
        Value::String(s) if s == "-inf" => f64::NEG_INFINITY,
        _ => v.as_f64().expect("a number"),
    }
}

fn unit(v: &Value) -> Unit {
    Unit::from_name(v.as_str().expect("a unit")).expect("a known unit")
}

fn step(c: &Value) -> Step {
    Step::new(c["n"].as_i64().expect("n"), unit(&c["unit"])).expect("a step")
}

fn rule(v: &Value) -> Rule {
    Rule {
        ranged: v["ranged"].as_bool().expect("ranged"),
        cumulative: v["cumulative"].as_bool().expect("cumulative"),
    }
}

fn text(v: &Value) -> Option<&str> {
    v.as_str()
}

fn window(v: &Value) -> Window {
    match v.get("instant") {
        Some(a) => Window::Instant(num(a)),
        None => Window::Range(num(&v["range"][0]), num(&v["range"][1])),
    }
}

#[test]
fn values_read_as_the_reference_reads_them() {
    let file = fixture();
    let list = file["reads"].as_array().expect("reads");
    assert!(list.len() >= 60);
    for c in list {
        let t = c["text"].as_str().expect("text");
        let want = match &c["expect"] {
            Value::String(s) if s == "empty" => Read::Empty,
            Value::String(s) if s == "unreadable" => Read::Unreadable,
            v => Read::Moment(num(v)),
        };
        assert_eq!(read(t), want, "{t:?}");
    }
}

#[test]
fn moments_are_written_shown_and_rounded_as_the_reference_does() {
    let file = fixture();
    for c in file["writes"].as_array().expect("writes") {
        let got = write(num(&c["ms"]), c["dateOnly"].as_bool().expect("dateOnly"));
        assert_eq!(got, c["expect"].as_str().expect("text"), "{c}");
    }
    for c in file["shows"].as_array().expect("shows") {
        assert_eq!(
            show(num(&c["ms"]), unit(&c["unit"])),
            c["expect"].as_str().expect("text"),
            "{c}"
        );
    }
    for c in file["floors"].as_array().expect("floors") {
        assert_eq!(
            floor_to(num(&c["ms"]), unit(&c["unit"])),
            num(&c["expect"]),
            "{c}"
        );
    }
}

#[test]
fn the_slider_steps_and_positions_are_the_references() {
    let file = fixture();
    for c in file["steps"].as_array().expect("steps") {
        let got = position(num(&c["anchor"]), step(c), c["k"].as_i64().expect("k"));
        assert_eq!(got, num(&c["expect"]), "{c}");
    }
    for c in file["positions"].as_array().expect("positions") {
        let extent = (num(&c["extent"][0]), num(&c["extent"][1]));
        let got = positions(extent, step(c));
        match &c["expect"] {
            Value::Null => assert_eq!(got, None, "{c}"),
            e => assert_eq!(
                got,
                Some((num(&e["anchor"]), e["k"].as_i64().expect("k"))),
                "{c}"
            ),
        }
    }
    for c in file["autoSteps"].as_array().expect("autoSteps") {
        let extent = (num(&c["extent"][0]), num(&c["extent"][1]));
        assert_eq!(auto_step(extent), step(&c["expect"]), "{c}");
    }
}

#[test]
fn objects_times_and_the_window_follow_the_table() {
    let file = fixture();
    for c in file["times"].as_array().expect("times") {
        let r = rule(&c["rule"]);
        let start = text(&c["start"]).map_or(Read::Empty, read);
        let end = if r.ranged {
            text(&c["end"]).map_or(Read::Empty, read)
        } else {
            Read::Empty
        };
        let got = object_time(r, start, end);
        match &c["expect"] {
            Value::Null => assert_eq!(got, None, "{c}"),
            e => {
                let mode = match e["mode"].as_str().expect("mode") {
                    "range" => Mode::Range,
                    "instant" => Mode::Instant,
                    _ => Mode::Cumulative,
                };
                let want = Time {
                    s: num(&e["s"]),
                    e: num(&e["e"]),
                    mode,
                };
                assert_eq!(got, Some(want), "{c}");
                for w in c["windows"].as_array().expect("windows") {
                    assert_eq!(
                        shows(&want, &window(&w["window"])),
                        w["expect"].as_bool().expect("bool"),
                        "{c} {w}"
                    );
                }
            }
        }
    }
    for c in file["summaries"].as_array().expect("summaries") {
        let values: Vec<(Option<&str>, Option<&str>)> = c["values"]
            .as_array()
            .expect("values")
            .iter()
            .map(|v| (text(&v[0]), text(&v[1])))
            .collect();
        let (_, sum) = layer_times(rule(&c["rule"]), values);
        let e = &c["expect"];
        assert_eq!(sum.timed as u64, e["timed"].as_u64().expect("timed"), "{c}");
        assert_eq!(
            sum.timeless as u64,
            e["timeless"].as_u64().expect("timeless"),
            "{c}"
        );
        assert_eq!(
            sum.unreadable as u64,
            e["unreadable"].as_u64().expect("unreadable"),
            "{c}"
        );
        let extent = match &e["extent"] {
            Value::Null => None,
            v => Some((num(&v[0]), num(&v[1]))),
        };
        assert_eq!(sum.extent, extent, "{c}");
    }
}

/// ADR 0210 §12's budgets on 100 000 parcels whose validity starts a year
/// apart across a century (no end for half of them): reading their times from
/// the values, the store taking them, a window's filter over all of them (the
/// queries' and the renderer's test) and the slider's range. Release, by hand:
/// `cargo test --release -p kentos-geometry-core --test all time::timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn timing() {
    use kentos_geometry_core::store::Store;
    use std::time::Instant;

    const N: usize = 100_000;
    let values: Vec<(String, String)> = (0..N)
        .map(|i| {
            let y = 1925 + i % 100;
            let start = format!("{y}-{:02}-{:02}", 1 + i % 12, 1 + i % 28);
            let end = if i % 2 == 0 {
                format!("{:02}.06.{}", 1 + i % 28, y + 10)
            } else {
                String::new()
            };
            (start, end)
        })
        .collect();
    let rule = Rule {
        ranged: true,
        cumulative: false,
    };
    let row = |what: &str, ms: f64, budget: Option<f64>| match budget {
        Some(b) => println!(
            "{what:<46} {ms:>9.3} ms   bütçe {b:>5} ms  {}",
            if ms <= b { "✓" } else { "✗" }
        ),
        None => println!("{what:<46} {ms:>9.3} ms"),
    };
    let best = |f: &mut dyn FnMut() -> f64| {
        (0..5)
            .map(|_| f())
            .fold(f64::INFINITY, |a, b| if b < a { b } else { a })
    };

    let mut times = Vec::new();
    let read_ms = best(&mut || {
        let t = Instant::now();
        let (out, sum) = layer_times(
            rule,
            values
                .iter()
                .map(|(s, e)| (Some(s.as_str()), Some(e.as_str()))),
        );
        assert_eq!(sum.timed, N);
        times = out;
        t.elapsed().as_secs_f64() * 1000.0
    });
    row(
        "100 000 değerin zamanı (okuma, nesnenin zamanı)",
        read_ms,
        Some(30.0),
    );

    let mut store = Store::new();
    let take_ms = best(&mut || {
        let t = Instant::now();
        store.set_times(times.iter().enumerate().map(|(i, t)| (i as f64 + 1.0, *t)));
        t.elapsed().as_secs_f64() * 1000.0
    });
    row("deponun 100 000 zamanı alması", take_ms, None);

    let ids: Vec<f64> = (1..=N).map(|i| i as f64).collect();
    let at = |y: i64| (kentos_geometry_core::time::days_from_civil(y, 1, 1) * 86_400_000) as f64;
    let window_ms = best(&mut || {
        let t = Instant::now();
        store.set_time_window(Some(Window::Instant(at(1990))));
        let shown = store.time_mask(&ids).iter().filter(|&&m| m == 1).count();
        assert!(shown > 0 && shown < N);
        t.elapsed().as_secs_f64() * 1000.0
    });
    row(
        "pencere değişince 100 000 nesnenin süzgeci",
        window_ms,
        None,
    );

    let extent_ms = best(&mut || {
        let t = Instant::now();
        let e = kentos_geometry_core::time::extent(times.iter().flatten());
        assert!(e.is_some());
        let step = kentos_geometry_core::time::auto_step(e.unwrap_or_default());
        assert!(positions(e.unwrap_or_default(), step).is_some());
        t.elapsed().as_secs_f64() * 1000.0
    });
    row("sürgünün aralığı, adımı ve konumları", extent_ms, None);
}
