//! Real drawings measured as `perf::large` measures its made one (plan/render.md RM-01, docs/adr/9201). Each file of
//! `KENTOS_PERF_FILES` (comma separated: a project, an NCZ, a DXF or a DWG) is opened through the app's own open in a
//! process of its own, then:
//!
//! - **açılış**: the open's thread, the first frame, the whole (`large::open`);
//! - **iz**: `large::trace`: the pointer over the drawing, the wheel (8 in, 8 out, twice), a middle-button pan;
//! - **patlama**: fourteen wheel notches in one frame, as the window hands a fast hand's turns to the frame it was
//!   late for, in and then out, and the frames after each until the picture stops changing: what the hand waits for;
//! - **tümü**: show everything (a middle double click), and the box it shows: whether the origin is in it;
//! - **seçim**: a click on the drawing's middle, and the view's share of its frame (the interface built again);
//! - **bellek**: resident memory after the open and the highest.
//!
//! The files never come into the repository; the report goes to `KENTOS_PERF_OUT`
//! (`files-desktop-<label>-<date>.json`). Not a correctness test and not run by default:
//!
//! ```text
//! KENTOS_PERF_FILES=/yol/sivas.kcad,/yol/plan.NCZ KENTOS_PERF_LABEL=once KENTOS_PERF_OUT=docs/perf \
//!   cargo test --release -p kentos-desktop --lib perf::files::files -- --ignored --nocapture --test-threads=1
//! ```

use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use iced::Event;
use serde_json::{Value, json};

use super::frame::{Harness, Parts, click, moved};
use super::large::{adapter, one, open, stats, trace, wheel};
use super::{environment, ms};
use crate::app::{App, Message};

/// Wheel notches a fast hand turns between two frames of a slow drawing.
const BURST: usize = 14;
/// How long the picture is watched after a burst, at most.
const WATCH: Duration = Duration::from_secs(4);
/// A picture unchanged this long is the last one: longer than the labels' settle after a move (150 ms).
const STILL: Duration = Duration::from_millis(400);

/// The process's resident and highest memory, MB (Linux's `/proc/self/status`; zero elsewhere).
fn memory() -> (f64, f64) {
    let text = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        text.lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<f64>().ok())
            .map_or(0.0, |kb| kb / 1024.0)
    };
    (field("VmRSS:"), field("VmHWM:"))
}

/// The picture's fingerprint: the drawing area's pixels only (the status bar's numbers change with the view).
fn fingerprint(h: &mut Harness, app: &App) -> u64 {
    let rgba = h.picture(app);
    let area = app.viewport.bounds;
    let width = super::frame::WINDOW.width as usize;
    let mut hash = DefaultHasher::new();
    let (x0, x1) = (area.x.max(0.0) as usize, (area.x + area.width) as usize);
    for y in area.y.max(0.0) as usize..(area.y + area.height) as usize {
        let row = &rgba[(y * width + x0) * 4..((y * width + x1.min(width)) * 4).min(rgba.len())];
        row.hash(&mut hash);
    }
    hash.finish()
}

/// `notches` wheel turns in one frame at the area's middle, then frames until the picture holds still: the burst's
/// frame, the frames after it, and when the picture stopped changing (milliseconds from the burst).
fn burst(app: &mut App, h: &mut Harness, notches: f32) -> Value {
    let center = app.viewport.bounds.center();
    h.frame(app, &[moved(center)]);
    let events: Vec<Event> = (0..BURST).map(|_| wheel(notches)).collect();
    let t0 = Instant::now();
    let first = h.frame(app, &events);
    let mut last = fingerprint(h, app);
    let mut changed = ms(t0);
    let mut since = Instant::now();
    let mut after = Vec::new();
    while t0.elapsed() < WATCH && since.elapsed() < STILL {
        std::thread::sleep(Duration::from_millis(16));
        after.push(h.frame(app, &[]));
        let now = fingerprint(h, app);
        if now != last {
            last = now;
            changed = ms(t0);
            since = Instant::now();
        }
    }
    json!({
        "notches": BURST, "burstFrame": one(&first), "framesAfter": stats(&after),
        "lastChangeMs": changed, "scale": app.viewport.camera.scale,
    })
}

/// Show everything, as a middle double click asks: its frame and the box the view shows then.
fn everything(app: &mut App, h: &mut Harness) -> Value {
    let f = h.frame_with(app, &[], |app| {
        let _ = app.update(Message::Viewport(crate::viewport::Event::Extents));
    });
    h.frame(app, &[]);
    let b = app.viewport.camera.visible_bounds();
    let origin = b.min_x <= 0.0 && b.max_x >= 0.0 && b.min_y <= 0.0 && b.max_y >= 0.0;
    json!({
        "frame": one(&f), "box": [b.min_x, b.min_y, b.max_x, b.max_y], "originInView": origin,
        "scale": app.viewport.camera.scale,
    })
}

/// A click on the drawing's middle: its frame and whether it selected anything; then Esc.
fn select(app: &mut App, h: &mut Harness) -> Value {
    let center = app.viewport.bounds.center();
    let f = h.frame(app, &click(center));
    let selected = app.selection.len();
    h.frame(app, &super::frame::key(iced::keyboard::key::Named::Escape));
    json!({ "frame": one(&f), "viewMs": f.view_ms(), "selected": selected })
}

fn measure(file: &Path) -> Value {
    let (mut app, _) = App::boot(None);
    // `KENTOS_PERF_SETTINGS` and `KENTOS_PERF_SHORTCUTS` as `perf::large` reads them: a diagnosis's settings.
    super::large::settings_from_env(&mut app);
    super::large::shortcuts_from_env(&mut app);
    let mut h = Harness::new();
    h.settle(&mut app);
    let (boot, _) = memory();
    let opened = open(&mut app, &mut h, file);
    let (resident, _) = memory();
    let objects = app.document.as_ref().map_or(0, |d| d.model.entities().count());
    let fit = app.viewport.camera.visible_bounds();
    // What the GPU draws for the opened view (plan/render.md F3): the work a far view's frame is made of.
    h.frame(&mut app, &[]);
    let drawn = scene_stats(&app);
    let shown = everything(&mut app, &mut h);
    let frames: Vec<(&str, Vec<Parts>)> = trace(&mut app, &mut h, 60);
    let frames: Value = frames
        .iter()
        .map(|(name, f)| ((*name).to_owned(), stats(f)))
        .collect::<serde_json::Map<_, _>>()
        .into();
    let inward = burst(&mut app, &mut h, 1.0);
    let outward = burst(&mut app, &mut h, -1.0);
    let selection = select(&mut app, &mut h);
    let (end, peak) = memory();
    json!({
        "file": file.file_name().map(|n| n.to_string_lossy().into_owned()),
        "bytes": std::fs::metadata(file).map_or(0, |m| m.len()),
        "objects": objects,
        "open": opened,
        "openFit": [fit.min_x, fit.min_y, fit.max_x, fit.max_y],
        "drawn": drawn,
        "everything": shown,
        "frames": frames,
        "burst": { "in": inward, "out": outward },
        "select": selection,
        "memory": { "residentBootMb": boot, "residentOpenMb": resident, "residentEndMb": end, "peakMb": peak },
        "onGpu": h.on_gpu(),
    })
}

/// The drawing area's last frame as the renderer counted it: pieces, triangles, marks, draw calls and the bytes the
/// view holds on the GPU.
fn scene_stats(app: &App) -> Value {
    let s = app.viewport.status().stats;
    json!({
        "segments": s.segments, "triangles": s.triangles, "markers": s.markers, "styledItems": s.styled_items,
        "drawCalls": s.draw_calls,
        "residentBytes": s.resident_bytes, "parkedBytes": s.parked_bytes, "pictureKept": s.picture_kept,
    })
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One file in this process (`KENTOS_FILES_CHILD`); prints one line of JSON.
#[test]
#[ignore = "run by perf::files::files in a process of its own"]
fn child() {
    let Ok(file) = std::env::var("KENTOS_FILES_CHILD") else {
        return;
    };
    println!("KENTOS_FILES {}", measure(Path::new(&file)));
}

fn run_child(file: &str) -> Value {
    let exe = std::env::current_exe().expect("the test binary");
    let output = Command::new(exe)
        .args(["perf::files::child", "--exact", "--ignored", "--nocapture"])
        .env("KENTOS_FILES_CHILD", file)
        .output()
        .expect("runs");
    let text = String::from_utf8_lossy(&output.stdout);
    match text.lines().find_map(|l| l.strip_prefix("KENTOS_FILES ")) {
        Some(line) => serde_json::from_str(line).expect("JSON"),
        None => json!({
            "file": file, "failed": true,
            "stderrTail": String::from_utf8_lossy(&output.stderr).lines().rev().take(20).collect::<Vec<_>>(),
        }),
    }
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn files() {
    let files: Vec<String> = std::env::var("KENTOS_PERF_FILES")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(str::to_owned)
        .collect();
    assert!(!files.is_empty(), "KENTOS_PERF_FILES names the drawings to measure");
    let label = std::env::var("KENTOS_PERF_LABEL").unwrap_or_else(|_| "latest".into());
    let mut rows = Vec::new();
    for file in &files {
        let measured = run_child(file);
        println!("{file}: {measured}");
        rows.push(measured);
    }
    let report = json!({
        "label": label, "environment": environment(&root()), "gpu": adapter(),
        "setup": { "window": [super::frame::WINDOW.width, super::frame::WINDOW.height],
            "burstNotches": BURST, "profile": "release (lto thin)" },
        "rows": rows,
    });
    let date = super::flat::chrono_free_date();
    match std::env::var("KENTOS_PERF_OUT") {
        Ok(out) => {
            let file = root().join(out).join(format!("files-desktop-{label}-{date}.json"));
            std::fs::write(&file, format!("{}\n", serde_json::to_string_pretty(&report).expect("JSON")))
                .expect("writes the report");
            println!("Yazıldı: {}", file.display());
        }
        Err(_) => println!("{}", serde_json::to_string_pretty(&report).expect("JSON")),
    }
}
