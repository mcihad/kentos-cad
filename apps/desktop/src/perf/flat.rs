//! ADV-07 (docs/adr/0776 §4): the 2+1 apartment of the acceptance flow (fixtures/acceptance/v1/flat.json, drawn
//! by its commands through the agent link) copied into 1, 20 and 100 apartments, and the interaction on it measured
//! with real Iced events as `perf::frame` measures a frame: panning the whole plan, Kapı's preview along a wall,
//! a click picking a wall, a design change (Mesafe with Bu ölçüyü koru, the rooms, dimensions, tags and schedules
//! following in the same step) and Metraj's query, and the process's resident memory. The UI thread's share
//! (“cpu”) and, with wgpu, the GPU's with the picture read back; no window is presented. Not a correctness test and
//! not run by default:
//!
//! ```text
//! cargo test --release -p kentos-desktop --lib perf::flat -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `KENTOS_PERF_FLATS` (comma separated) changes the counts; `KENTOS_PERF_OUT` writes
//! `flat-desktop-<date>.json` there. `KENTOS_PERF_SCHEDULES=each` copies the flat's two schedules with it (200 at 100
//! flats, as docs/adr/0776 measured); otherwise the building keeps its two, reading every flat (docs/adr/0782).
//! The memory's peak during the design changes is the most `ps` saw, read every 5 ms on a thread.

use std::collections::BTreeMap;
use std::time::Instant;

use iced::{Event, Point, mouse};
use serde_json::{Value, json};

use super::frame::{Harness, Parts, WINDOW, moved, pan, summary};
use super::{environment, median, ms};
use crate::app::{App, Message};

/// One apartment's box, metres, and the step between copies.
const STEP: [f64; 2] = [14.0, 12.0];

fn ask(app: &mut App, method: &str, params: Value) -> Value {
    app.agents_answer(&json!({ "id": 1, "method": method, "params": params }))
        .unwrap_or_else(|(code, message)| panic!("{method}: {code}: {message}"))
}

/// The process's resident memory, MB (`ps`, as macOS and Linux both give it).
fn resident() -> f64 {
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|t| t.trim().parse::<f64>().ok())
        .map_or(f64::NAN, |kb| kb / 1024.0)
}

fn at(app: &App, x: f64, y: f64) -> Point {
    let [x, y] = app.viewport.camera.world_to_screen(kentos_interaction::Vec2::new(x, y));
    Point::new(x as f32, y as f32)
}

fn click(at: Point) -> [Event; 3] {
    [
        moved(at),
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]
}

fn uids(app: &App, tables: bool) -> Vec<String> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities_with_uids()
        .filter(|(_, e)| tables || !matches!(e, kentos_commands::Entity::Table(_)))
        .map(|(u, _)| u.to_string())
        .collect()
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
    let most = watch.join().unwrap_or(f64::NAN);
    (out, most)
}

/// The apartment copied into `n`: rows of ten, each copy one step (`cad.entities.transform`'s copy); how long.
fn copied(app: &mut App, n: usize, tables: bool) -> f64 {
    let t = Instant::now();
    let one = uids(app, tables);
    for c in 1..n.min(10) {
        let answer = ask(app, "run", json!({ "command": "cad.entities.transform", "op": "execute", "input": {
            "uids": one, "transform": { "kind": "move", "dx": STEP[0] * c as f64, "dy": 0.0 }, "copy": true } }));
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let row = uids(app, tables);
    for r in 1..n.div_ceil(10) {
        let answer = ask(app, "run", json!({ "command": "cad.entities.transform", "op": "execute", "input": {
            "uids": row, "transform": { "kind": "move", "dx": 0.0, "dy": STEP[1] * r as f64 }, "copy": true } }));
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    ms(t)
}

fn cpu(frames: &[Parts]) -> Vec<f64> {
    frames.iter().map(Parts::cpu).collect()
}

#[cfg(unix)]
#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn flats() {
    let counts: Vec<usize> = std::env::var("KENTOS_PERF_FLATS")
        .unwrap_or_else(|_| "1,20,100".into())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let mut rows = Vec::new();
    let mut renderer = String::new();
    for &n in &counts {
        let (mut app, _dir) = crate::acceptance_flat_tests::opened(&format!("perf-daire-{n}"));
        app.window_size = WINDOW;
        let _ = app.settings.choose(&[
            ("appearance.textSize", json!(13)),
            ("appearance.theme", json!("dark")),
            ("drafting.ortho", json!(false)),
            ("drafting.polar", json!(false)),
        ]);
        app.apply_settings();
        let names: BTreeMap<String, String> = crate::acceptance_flat_tests::drawn(&mut app);
        let each = std::env::var("KENTOS_PERF_SCHEDULES").is_ok_and(|v| v == "each");
        let copy_ms = copied(&mut app, n, each);
        let objects = app.document.as_ref().map_or(0, |d| d.model.entities().count());
        let mut h = Harness::new();
        renderer = iced::advanced::renderer::Headless::name(&h.renderer).to_owned();
        h.settle(&mut app);
        let (cols, lines) = (n.min(10) as f64, n.div_ceil(10) as f64);
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds {
                min_x: -3.0,
                min_y: -4.0,
                max_x: STEP[0] * cols,
                max_y: STEP[1] * lines,
            },
            24.0,
        );
        h.settle(&mut app);
        let built = resident();
        // Panning the whole plan.
        let middle = Point::new(WINDOW.width / 2.0, WINDOW.height / 2.0);
        let whole = pan(&mut app, &mut h, middle);
        // One apartment at about 1:50, panned.
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: -1.0, min_y: -1.0, max_x: 12.0, max_y: 9.0 },
            24.0,
        );
        h.settle(&mut app);
        let close = pan(&mut app, &mut h, middle);
        // Kapı's preview along Bölme 1, 60 positions.
        let _ = app.update(Message::Run("tool.select"));
        h.settle(&mut app);
        h.frame_with(&mut app, &[], |app| {
            let _ = app.update(Message::Run("tool.door"));
        });
        let preview: Vec<Parts> = (0..60)
            .map(|i| {
                let p = at(&app, 4.575, 1.0 + 6.0 * (i as f64) / 60.0);
                h.frame(&mut app, &[moved(p)])
            })
            .collect();
        let _ = app.update(Message::Run("tool.select"));
        h.settle(&mut app);
        // A click picking a wall, on the walls of ten apartments in turn.
        let picks: Vec<Parts> = (0..20)
            .map(|i| {
                let k = (i % n.min(10)) as f64;
                let p = at(&app, STEP[0] * k + 4.575, 2.0);
                h.frame(&mut app, &click(p))
            })
            .collect();
        let _ = app.update(Message::Run("tool.select"));
        app.selection.clear();
        h.settle(&mut app);
        // The design change, the drawing redrawn after it, then undone: five times.
        let p3 = names["p3"].clone();
        let mut change = Vec::new();
        let mut command = Vec::new();
        let before_change = resident();
        let ((), change_peak) = peak(|| {
            for i in 0..5 {
                let mut took = 0.0;
                change.push(h.frame_with(&mut app, &[], |app| {
                    let t = Instant::now();
                    let answer = ask(app, "run", json!({ "command": "cad.walls.edit", "op": "execute", "input": {
                        "uids": [p3], "operation": { "kind": "distance", "side": "right", "distance": 3.7 + 0.01 * i as f64 } } }));
                    took = ms(t);
                    assert_eq!(answer["status"], "completed", "{answer}");
                    // The app's update follows the drawing before the frame (app.rs's `follow_document`): the
                    // store the labels are read from takes the change (docs/adr/0786 §5).
                    app.follow_document();
                }));
                command.push(took);
                app.undo();
                app.follow_document();
                h.settle(&mut app);
            }
        });
        let after_change = resident();
        let quantities: Vec<f64> = (0..5)
            .map(|_| {
                let t = Instant::now();
                let answer = ask(&mut app, "run", json!({ "command": "cad.spaces.quantities", "op": "execute", "input": {} }));
                assert_eq!(answer["status"], "completed");
                ms(t)
            })
            .collect();
        // Where a design change's time goes (docs/adr/0776 §4): its plan alone, a plain Kaydır, an attribute
        // written on the same wall, each undone; the memory after.
        let mut parts = serde_json::Map::new();
        if std::env::var("KENTOS_PERF_PARTS").is_ok() {
            let wall = |op: Value| json!({ "command": "cad.walls.edit", "op": "execute", "input": { "uids": [p3], "operation": op } });
            let runs: [(&str, Value); 4] = [
                ("planMs", json!({ "command": "cad.walls.edit", "op": "plan", "input": { "uids": [p3],
                    "operation": { "kind": "distance", "side": "right", "distance": 3.7 } } })),
                ("distanceMs", wall(json!({ "kind": "distance", "side": "right", "distance": 3.7 }))),
                ("shiftMs", wall(json!({ "kind": "shift", "by": 0.05 }))),
                ("attributeMs", json!({ "command": "cad.entities.set", "op": "execute", "input": { "uids": [p3],
                    "operation": "attributes", "attrs": { "Not": "x" } } })),
            ];
            for (name, params) in runs {
                let mut took = Vec::new();
                for _ in 0..3 {
                    let t = Instant::now();
                    let answer = ask(&mut app, "run", params.clone());
                    took.push(ms(t));
                    assert_eq!(answer["status"], "completed", "{name}: {answer}");
                    if params["op"] == "execute" {
                        app.undo();
                    }
                }
                parts.insert(name.into(), json!(median(took)));
                parts.insert(format!("{name}ResidentMb"), json!(resident()));
            }
        }
        let worst =[cpu(&whole), cpu(&preview), cpu(&picks)].iter().flatten().copied().fold(0.0, f64::max);
        let row = json!({
            "apartments": n, "objects": objects, "schedulesEach": each, "copyMs": copy_ms, "parts": parts,
            "residentMb": built, "residentAfterMb": resident(),
            "changeMemoryMb": { "before": before_change, "peak": change_peak, "after": after_change },
            "panWhole": summary(&whole), "panOne": summary(&close), "doorPreview": summary(&preview),
            "pick": summary(&picks), "designChangeFrame": summary(&change),
            "designChangeCommandMs": { "p50": median(command.clone()), "max": command.iter().copied().fold(0.0, f64::max) },
            "quantitiesMs": { "p50": median(quantities.clone()), "max": quantities.iter().copied().fold(0.0, f64::max) },
            "cpuMax": worst,
        });
        println!("FLAT {n}: {row}");
        rows.push(row);
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = json!({
        "format": "kentos.flat-runtime", "version": 1, "environment": environment(&root),
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" }, "renderer": renderer,
        "window": [WINDOW.width, WINDOW.height], "textSize": 13,
        "method": "Iced runtime events and offscreen redraw (perf::frame); gpu includes the picture's readback, no window present.",
        "rows": rows,
    });
    if let Some(out) = std::env::var_os("KENTOS_PERF_OUT") {
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).expect("the folder");
        let path = out.join(format!("flat-desktop-{}.json", chrono_free_date()));
        std::fs::write(&path, serde_json::to_vec_pretty(&report).expect("json")).expect("written");
        println!("FLAT_REPORT {}", path.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&report).expect("json"));
    }
}

/// Today as `YYYY-MM-DD` from the system clock (days since 1970, the civil calendar).
fn chrono_free_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
