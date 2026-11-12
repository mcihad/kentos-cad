//! ADV-07 (docs/adr/0798 §4): the acceptance corpus's other samples (fixtures/acceptance/v1/<id>.json, drawn by
//! their commands through the agent link) measured as `perf::flat` measures the 2+1 apartment, once and copied into
//! twenty: panning the whole plan, a click picking a wall on its body, the sample's design change (its own
//! command, undone after each run) with the frame drawn after it, Metraj's query, and the process's resident
//! memory. Not a correctness test and not run by default:
//!
//! ```text
//! cargo test --release -p kentos-desktop --lib perf::corpus -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `KENTOS_PERF_COPIES` (comma separated) changes the counts, `KENTOS_PERF_SAMPLES` the samples; `KENTOS_PERF_OUT`
//! writes `corpus-desktop-<date>.json` there.

use std::collections::BTreeMap;
use std::time::Instant;

use iced::Point;
use serde_json::{Value, json};

use super::flat::{ask, at, chrono_free_date, click, cpu, peak, resident};
use super::frame::{Harness, Parts, WINDOW, pan, summary};
use super::{environment, median, ms};
use crate::app::{App, Message};

/// A sample's measuring: its box (metres), the step between its copies, the wall a click picks (on its body), the
/// design change (a command with `$name`s and `{i}` for the run's number).
struct Sample {
    id: &'static str,
    bounds: [f64; 4],
    step: [f64; 2],
    pick: [f64; 2],
    change: fn(&BTreeMap<String, String>, usize) -> Value,
}

fn samples() -> Vec<Sample> {
    vec![
        Sample { id: "change", bounds: [-1.0, -2.0, 12.0, 9.0], step: [14.0, 12.0], pick: [8.825, 2.0],
            change: |n, i| json!({ "command": "cad.walls.edit", "op": "execute", "input": { "uids": [n["p3"]],
                "operation": { "kind": "distance", "side": "right", "distance": 3.7 + 0.01 * i as f64 } } }) },
        Sample { id: "house", bounds: [-1.0, -1.0, 11.0, 9.0], step: [13.0, 11.0], pick: [5.075, 2.0],
            change: |n, i| json!({ "command": "cad.walls.edit", "op": "execute", "input": { "uids": [n["a"]],
                "operation": { "kind": "distance", "side": "right", "distance": 4.9 - 0.01 * i as f64 } } }) },
        Sample { id: "curved", bounds: [-1.0, -1.0, 15.0, 9.0], step: [17.0, 11.0], pick: [3.8, 2.0],
            change: |n, i| json!({ "command": "cad.walls.edit", "op": "execute", "input": { "uids": [n["p1"]],
                "operation": { "kind": "distance", "side": "right", "distance": 3.7 + 0.01 * i as f64 } } }) },
        Sample { id: "renovation", bounds: [-1.0, -1.0, 10.0, 8.0], step: [12.0, 10.0], pick: [3.375, 2.0],
            change: |n, i| json!({ "command": "cad.walls.edit", "op": "execute", "input": { "uids": [n["partition"]],
                "operation": { "kind": "distance", "side": "right", "distance": 3.2 - 0.01 * i as f64 } } }) },
    ]
}

/// Every object's id, the sample copied into `n` (rows of ten, each copy one step); how long.
fn copied(app: &mut App, n: usize, step: [f64; 2]) -> f64 {
    let ids = |app: &App| -> Vec<String> {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities_with_uids().map(|(u, _)| u.to_string()).collect()
    };
    let t = Instant::now();
    let one = ids(app);
    for c in 1..n.min(10) {
        let answer = ask(app, "run", json!({ "command": "cad.entities.transform", "op": "execute", "input": {
            "uids": one, "transform": { "kind": "move", "dx": step[0] * c as f64, "dy": 0.0 }, "copy": true } }));
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let row = ids(app);
    for r in 1..n.div_ceil(10) {
        let answer = ask(app, "run", json!({ "command": "cad.entities.transform", "op": "execute", "input": {
            "uids": row, "transform": { "kind": "move", "dx": 0.0, "dy": step[1] * r as f64 }, "copy": true } }));
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    ms(t)
}

#[cfg(unix)]
#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn corpus() {
    let counts: Vec<usize> = std::env::var("KENTOS_PERF_COPIES")
        .unwrap_or_else(|_| "1,20".into())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let only = std::env::var("KENTOS_PERF_SAMPLES").ok();
    let mut rows = Vec::new();
    let mut renderer = String::new();
    for sample in samples() {
        if only.as_ref().is_some_and(|o| !o.split(',').any(|x| x == sample.id)) {
            continue;
        }
        let text = crate::acceptance_flat_tests::CORPUS.iter().find(|(id, _)| *id == sample.id).expect("the sample").1;
        for &n in &counts {
            let (mut app, _dir) = crate::acceptance_flat_tests::opened_with(text, &format!("perf-{}-{n}", sample.id));
            app.window_size = WINDOW;
            let _ = app.settings.choose(&[
                ("appearance.textSize", json!(13)),
                ("appearance.theme", json!("dark")),
                ("drafting.ortho", json!(false)),
                ("drafting.polar", json!(false)),
            ]);
            app.apply_settings();
            let names = crate::acceptance_flat_tests::drawn_with(&mut app, text);
            let copy_ms = copied(&mut app, n, sample.step);
            let objects = app.document.as_ref().map_or(0, |d| d.model.entities().count());
            let mut h = Harness::new();
            renderer = iced::advanced::renderer::Headless::name(&h.renderer).to_owned();
            h.settle(&mut app);
            let [x0, y0, x1, y1] = sample.bounds;
            let (cols, lines) = (n.min(10) as f64, n.div_ceil(10) as f64);
            app.viewport.camera.fit(
                &kentos_render_wgpu::Bounds {
                    min_x: x0,
                    min_y: y0,
                    max_x: x1 + sample.step[0] * (cols - 1.0),
                    max_y: y1 + sample.step[1] * (lines - 1.0),
                },
                24.0,
            );
            h.settle(&mut app);
            let built = resident();
            let middle = Point::new(WINDOW.width / 2.0, WINDOW.height / 2.0);
            let whole = pan(&mut app, &mut h, middle);
            app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: x0, min_y: y0, max_x: x1, max_y: y1 }, 24.0);
            h.settle(&mut app);
            let _ = app.update(Message::Run("tool.select"));
            h.settle(&mut app);
            let picks: Vec<Parts> = (0..20)
                .map(|_| {
                    let p = at(&app, sample.pick[0], sample.pick[1]);
                    h.frame(&mut app, &click(p))
                })
                .collect();
            app.selection.clear();
            h.settle(&mut app);
            let mut change = Vec::new();
            let mut command = Vec::new();
            let before_change = resident();
            let ((), change_peak) = peak(|| {
                for i in 0..5 {
                    let mut took = 0.0;
                    change.push(h.frame_with(&mut app, &[], |app| {
                        let t = Instant::now();
                        let answer = ask(app, "run", (sample.change)(&names, i));
                        took = ms(t);
                        assert_eq!(answer["status"], "completed", "{}: {answer}", sample.id);
                        app.follow_document();
                    }));
                    command.push(took);
                    app.undo();
                    app.follow_document();
                    h.settle(&mut app);
                }
            });
            let quantities: Vec<f64> = (0..5)
                .map(|_| {
                    let t = Instant::now();
                    let answer = ask(&mut app, "run", json!({ "command": "cad.spaces.quantities", "op": "execute", "input": {} }));
                    assert_eq!(answer["status"], "completed");
                    ms(t)
                })
                .collect();
            let worst = [cpu(&whole), cpu(&picks)].iter().flatten().copied().fold(0.0, f64::max);
            let row = json!({
                "sample": sample.id, "copies": n, "objects": objects, "copyMs": copy_ms,
                "residentMb": built, "changeMemoryMb": { "before": before_change, "peak": change_peak, "after": resident() },
                "panWhole": summary(&whole), "pick": summary(&picks), "designChangeFrame": summary(&change),
                "designChangeCommandMs": { "p50": median(command.clone()), "max": command.iter().copied().fold(0.0, f64::max) },
                "quantitiesMs": { "p50": median(quantities.clone()), "max": quantities.iter().copied().fold(0.0, f64::max) },
                "cpuMax": worst,
            });
            println!("CORPUS {} {n}: {row}", sample.id);
            rows.push(row);
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = json!({
        "format": "kentos.corpus-runtime", "version": 1, "environment": environment(&root),
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" }, "renderer": renderer,
        "window": [WINDOW.width, WINDOW.height], "textSize": 13,
        "method": "Iced runtime events and offscreen redraw (perf::frame); gpu includes the picture's readback, no window present.",
        "rows": rows,
    });
    if let Some(out) = std::env::var_os("KENTOS_PERF_OUT") {
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).expect("the folder");
        let path = out.join(format!("corpus-desktop-{}.json", chrono_free_date()));
        std::fs::write(&path, serde_json::to_vec_pretty(&report).expect("json")).expect("written");
        println!("CORPUS_REPORT {}", path.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&report).expect("json"));
    }
}
