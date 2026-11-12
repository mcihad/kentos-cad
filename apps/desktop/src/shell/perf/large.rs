//! A large mixed 2D drawing on the desktop, measured part by part (plan/cad-advanced.md CAD-ADV-19, docs/adr/1062):
//! the drawing of `perf::mixed` (10 000, 100 000 and 1 000 000 objects: lines, texts, curves, nested blocks, dense
//! hatches, dimensions, long polylines) opened through the app's own open (`opening.rs`, a thread of its own), then
//! one fixed trace of pointer, wheel and keys, the same on every run:
//!
//! - **açılış**: the open's thread (read, check, decode, document), the frame that puts it on screen, the whole;
//! - **ilk etkileşim**: the first pointer move, wheel turn and pan frame after it;
//! - **kare**: the pointer over the drawing, the wheel, a middle-button pan, far (all of it) and near (1:200), run
//!   twice (cold, then warm), UI thread and GPU apart, p50/p95/p99;
//! - **yazı girişi**: a key typed into the command line and into the value field while drawing a line;
//! - **komut**: one small edit each (a text moved, a text's words changed, an insert moved, a line deleted, undo,
//!   redo) and select all, with the styled parts each built again out of all the parts built;
//! - **iptal**: a second open stopped with Esc: the Esc frame (what the user sees) and when its thread stopped;
//! - **bellek**: resident memory, `footprint`'s physical footprint and peak, its GPU memory (IOAccelerator) and the
//!   view's own GPU buffers.
//!
//! Each size runs in a process of its own (`KENTOS_LARGE_CHILD`), so its peak is its own. The drawing's file goes to
//! `.run/perf/` (never committed); its fingerprint is printed. Not a correctness test and not run by default:
//!
//! ```text
//! KENTOS_PERF_LABEL=once KENTOS_PERF_OUT=docs/perf KENTOS_PERF_SIZES=10000,100000,1000000 \
//!   cargo test --release -p kentos-desktop --lib perf::large::large -- --ignored --nocapture --test-threads=1
//! ```
//!
//! The GPU column is the headless frame drawn and read back (`Renderer::screenshot`); an empty window's frame, drawn
//! the same way before the drawing opens, is reported beside it as the read back's floor. A window has no read back.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use iced::futures::StreamExt as _;
use iced::keyboard::{self, key};
use iced::{Event, Point, mouse};
use kentos_domain::Slot;
use serde_json::{Value, json};

use super::flat::{ask, resident};
use super::frame::{Harness, Parts, click, key, moved, pan, path, percentile};
use super::{environment, mixed, ms};
use crate::app::{App, Message};
use crate::document::Document;

/// 60 Hz: one frame's budget, milliseconds.
const BUDGET_MS: f64 = 1000.0 / 60.0;
/// 1:200 at 96 dpi, logical pixels per metre: the near view.
const NEAR: f64 = 1.0 / (200.0 * 0.000_264_58);

fn wheel(lines: f32) -> Event {
    Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 0.0, y: lines },
    })
}

/// A character typed, as the window gives it: pressed with its text, then let go.
fn typed(ch: char) -> Vec<Event> {
    let s = ch.to_string();
    let physical = key::Physical::Unidentified(key::NativeCode::Unidentified);
    let k = keyboard::Key::Character(s.as_str().into());
    vec![
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: k.clone(),
            modified_key: k.clone(),
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: Some(s.as_str().into()),
            repeat: false,
        }),
        Event::Keyboard(keyboard::Event::KeyReleased {
            key: k.clone(),
            modified_key: k,
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
        }),
    ]
}

/// UI thread and GPU apart: p50, p95, p99 and the worst, and the frames over 60 Hz's budget.
fn stats(frames: &[Parts]) -> Value {
    let ui: Vec<f64> = frames.iter().map(Parts::cpu).collect();
    let gpu: Vec<f64> = frames.iter().map(Parts::gpu_ms).collect();
    json!({
        "frames": frames.len(),
        "uiP50": percentile(&ui, 0.5), "uiP95": percentile(&ui, 0.95), "uiP99": percentile(&ui, 0.99),
        "uiMax": percentile(&ui, 1.0),
        "gpuP50": percentile(&gpu, 0.5), "gpuP95": percentile(&gpu, 0.95), "gpuP99": percentile(&gpu, 0.99),
        "gpuMax": percentile(&gpu, 1.0),
        "uiOverBudget": ui.iter().filter(|&&t| t > BUDGET_MS).count(),
        "viewP50": percentile(&frames.iter().map(|p| p.view_ms()).collect::<Vec<_>>(), 0.5),
        // The UI thread's parts at its p95 frame: event, app, view, layout, draw.
        "uiP95Parts": frames
            .iter()
            .min_by(|a, b| (a.cpu() - percentile(&ui, 0.95)).abs().total_cmp(&(b.cpu() - percentile(&ui, 0.95)).abs()))
            .map(|p| p.split().to_vec()),
    })
}

fn one(p: &Parts) -> Value {
    json!({ "uiMs": p.cpu(), "gpuMs": p.gpu_ms(), "viewMs": p.view_ms() })
}

/// The task's messages, each as it came, on a thread of its own (as Iced's runtime runs a blocking task).
fn drive(task: iced::Task<Message>) -> mpsc::Receiver<(Instant, Message)> {
    let (tx, rx) = mpsc::channel();
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        std::thread::spawn(move || {
            while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                if let iced_runtime::Action::Output(m) = action
                    && tx.send((Instant::now(), m)).is_err()
                {
                    break;
                }
            }
        });
    }
    rx
}

fn is_done(m: &Message) -> bool {
    matches!(m, Message::Opening(crate::opening::Event::Done { .. }))
}

/// `footprint`'s physical footprint, its peak and the GPU's (IOAccelerator) memory of this process, MB (macOS; empty
/// elsewhere).
fn footprint() -> Value {
    let Ok(out) = Command::new("footprint")
        .args(["-p", &std::process::id().to_string()])
        .output()
    else {
        return Value::Null;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mb = |s: &str| -> f64 {
        let mut w = s.split_whitespace();
        let v: f64 = w.next().and_then(|v| v.parse().ok()).unwrap_or(f64::NAN);
        match w.next().unwrap_or("B") {
            "KB" => v / 1024.0,
            "MB" => v,
            "GB" => v * 1024.0,
            _ => v / 1024.0 / 1024.0,
        }
    };
    let field = |name: &str| {
        text.lines()
            .find_map(|l| l.trim().strip_prefix(name))
            .map_or(f64::NAN, |v| mb(v.trim()))
    };
    // The categories' first column is dirty memory: "12 MB  0 B  0 B  3  IOAccelerator (graphics)".
    let gpu: f64 = text
        .lines()
        .filter(|l| l.contains("IOAccelerator") || l.contains("IOSurface"))
        .map(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            if w.len() >= 2 {
                mb(&format!("{} {}", w[0], w[1]))
            } else {
                0.0
            }
        })
        .sum();
    json!({
        "physFootprintMb": field("phys_footprint:"),
        "physFootprintPeakMb": field("phys_footprint_peak:"),
        "gpuMemoryMb": gpu,
    })
}

/// The view's GPU buffers and what the last frame drew (`FrameStats`).
fn view_stats(app: &App) -> Value {
    let s = app.viewport.status().stats;
    json!({
        "residentBytes": s.resident_bytes, "targetBytes": s.target_bytes, "segments": s.segments,
        "triangles": s.triangles, "drawCalls": s.draw_calls, "samples": s.samples,
    })
}

/// The GPU the frames are drawn on, as wgpu tells it (no device is opened for it).
fn adapter() -> Value {
    use iced::wgpu;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter =
        iced::futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }));
    match adapter {
        Ok(a) => {
            let i = a.get_info();
            json!({ "name": i.name, "backend": format!("{:?}", i.backend), "driver": i.driver, "driverInfo": i.driver_info })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

/// The fixed trace in the view as it is: the pointer over the drawing, the wheel in and out twice, a pan.
fn trace(app: &mut App, h: &mut Harness, hover: usize) -> Vec<(&'static str, Vec<Parts>)> {
    let area = app.viewport.bounds;
    let center = area.center();
    // `KENTOS_PERF_TRACE_VIEW` (a diagnosis, docs/adr/1064 §5): what the view did for each pointer and wheel frame.
    let tell = std::env::var("KENTOS_PERF_TRACE_VIEW").is_ok();
    let hovering = path(area, hover)
        .into_iter()
        .map(|p| {
            let f = h.frame(app, &[moved(p)]);
            if tell {
                let s = app.viewport.status().stats;
                println!(
                    "imleç: GPU {:.2} ms, resim tutuldu {}, yüklenen {} B, parça {}, çizim çağrısı {}",
                    f.gpu_ms(),
                    s.picture_kept,
                    s.uploaded_bytes,
                    s.segments,
                    s.draw_calls
                );
            }
            f
        })
        .collect();
    h.frame(app, &[moved(center)]);
    let mut wheels = Vec::new();
    for _ in 0..2 {
        for lines in [1.0, -1.0] {
            for _ in 0..8 {
                let f = h.frame(app, &[wheel(lines)]);
                if tell {
                    let s = app.viewport.status().stats;
                    println!(
                        "tekerlek: GPU {:.2} ms, MİB {:?}, ölçeklendi {}, tutuldu {}, yüklenen {} B, çizim çağrısı {}",
                        f.gpu_ms(),
                        f.split().map(|x| (x * 10.0).round() / 10.0),
                        s.picture_zoomed,
                        s.picture_kept,
                        s.uploaded_bytes,
                        s.draw_calls
                    );
                }
                wheels.push(f);
            }
        }
        // Screen-sized symbols and labels settle after the wheel stops (150 ms).
        std::thread::sleep(Duration::from_millis(200));
        h.frame(app, &[]);
    }
    let pans = pan(app, h, center);
    std::thread::sleep(Duration::from_millis(200));
    h.frame(app, &[]);
    vec![
        ("imleç", hovering),
        ("tekerlek", wheels),
        ("kaydırma", pans),
    ]
}

/// The drawing's middle cell's `k`th object (its id is its place + 1).
fn middle_id(n: usize, k: usize) -> u32 {
    let side = mixed::side(n);
    let cells = n.div_ceil(10);
    let rows = cells.div_ceil(side);
    let cell = ((rows / 2) * side + side / 2).min(cells - 1);
    (cell * 10 + k + 1) as u32
}

fn uid(app: &App, id: u32) -> String {
    app.document
        .as_ref()
        .and_then(|d| d.model.uid(Slot(id)))
        .map(|u| u.to_string())
        .unwrap_or_else(|| panic!("object {id}"))
}

/// One command as a frame: its UI and GPU time, and the styled parts built again out of all of them.
fn command(app: &mut App, h: &mut Harness, name: &str, act: impl FnOnce(&mut App)) -> Value {
    let _ = app.viewport.take_styled_build();
    let p = h.frame_with(app, &[], act);
    let (built, parts) = app.viewport.take_styled_build();
    let next = h.frame(app, &[]);
    let (later, _) = app.viewport.take_styled_build();
    json!({
        "command": name, "uiMs": p.cpu(), "gpuMs": p.gpu_ms(), "viewMs": p.view_ms(),
        "partsBuilt": built.map_or(0, |b| b.1), "buildMs": built.map(|b| b.0.as_secs_f64() * 1000.0),
        "partsTotal": parts, "nextFrameUiMs": next.cpu(), "nextFramePartsBuilt": later.map_or(0, |b| b.1),
    })
}

fn run(app: &mut App, input: Value) {
    let answer = ask(app, "run", input);
    assert_eq!(answer["status"], "completed", "{answer}");
}

/// Opens `file` through the app's open, frames drawn meanwhile as the window would: its parts.
pub(super) fn open(app: &mut App, h: &mut Harness, file: &Path) -> Value {
    let t0 = Instant::now();
    let rx = drive(app.start_opening(file.to_owned(), crate::opening::Purpose::File));
    let mut during = Vec::new();
    let (done_at, done) = loop {
        match rx.recv_timeout(Duration::from_millis(16)) {
            Ok((at, m)) if is_done(&m) => break (at, m),
            Ok((_, m)) => during.push(h.frame_with(app, &[], |app| {
                let _ = app.update(m);
            })),
            Err(mpsc::RecvTimeoutError::Timeout) => during.push(h.frame(app, &[])),
            Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the open ended without an answer"),
        }
    };
    let worker = (done_at - t0).as_secs_f64() * 1000.0;
    let first = h.frame_with(app, &[], |app| {
        let _ = app.update(done);
    });
    let placed = ms(t0);
    // The project's questions (its type, its CAD work) are not part of the drawing's frames.
    app.dialog = None;
    h.settle(app);
    assert!(app.document.is_some(), "the drawing opened");
    json!({
        "threadMs": worker, "firstFrame": one(&first), "toFirstFrameMs": placed,
        "framesWhileOpening": stats(&during), "settledMs": ms(t0),
    })
}

/// A second open of `file`, stopped with Esc once its thread is reading: the Esc frame, and how long after it the
/// thread stopped.
fn cancel(app: &mut App, h: &mut Harness, file: &Path) -> Value {
    let rx = drive(app.start_opening(file.to_owned(), crate::opening::Purpose::File));
    let started = Instant::now();
    // Esc once the open has said where it is, or after 40 ms.
    let mut said = 0;
    while said == 0 && started.elapsed() < Duration::from_millis(40) {
        if let Ok((_, m)) = rx.recv_timeout(Duration::from_millis(2)) {
            if is_done(&m) {
                return json!({ "stopped": false, "note": "the open ended before Esc" });
            }
            let _ = app.update(m);
            said += 1;
        }
    }
    let before = ms(started);
    let pressed = Instant::now();
    let esc = h.frame(app, &key(key::Named::Escape));
    let shown = app.opening.is_none();
    let end = loop {
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok((at, m)) if is_done(&m) => break Some((at, m)),
            Ok(_) => {}
            Err(_) => break None,
        }
    };
    let stop_ms = end
        .as_ref()
        .map(|(at, _)| (*at - pressed).as_secs_f64() * 1000.0);
    let kept = matches!(
        end,
        Some((
            _,
            Message::Opening(crate::opening::Event::Done { result: None, .. })
        ))
    );
    if let Some((_, m)) = end {
        let _ = app.update(m);
    }
    h.frame(app, &[]);
    json!({
        "stopped": true, "escAfterMs": before, "escFrame": one(&esc), "escUiPlusGpuMs": esc.cpu() + esc.gpu_ms(),
        "windowGoneOnEscFrame": shown, "threadStoppedAfterMs": stop_ms, "threadGaveNothing": kept,
    })
}

/// Everything measured on the drawing of `n` objects in `file`, in this process.
fn measure(n: usize, file: &Path) -> Value {
    let (mut app, _) = App::boot(None);
    shortcuts_from_env(&mut app);
    // The commands go through the agents' link, as Python and MCP send them (a socket's path must be short).
    let socket = std::env::temp_dir().join(format!("kentos-large-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&socket);
    let (link, _task) = crate::python::link::Link::open(&socket, |heard| {
        Message::Python(crate::python::Event::Agent(heard))
    })
    .expect("the agent link");
    app.python.link = Some(link);
    let mut h = Harness::new();
    h.settle(&mut app);
    let floor: Vec<Parts> = (0..20).map(|_| h.frame(&mut app, &[])).collect();
    let rss_boot = resident();
    let opened = open(&mut app, &mut h, file);
    let after_open = (resident(), footprint(), view_stats(&app));
    let area = app.viewport.bounds;
    let center = area.center();
    // The first input after the drawing came: the pointer, the wheel, a pan step.
    let first_move = h.frame(
        &mut app,
        &[moved(Point::new(center.x + 3.0, center.y + 2.0))],
    );
    let first_wheel = h.frame(&mut app, &[wheel(1.0)]);
    h.frame(&mut app, &[wheel(-1.0)]);
    std::thread::sleep(Duration::from_millis(200));
    h.frame(&mut app, &[]);
    let first_pan = {
        h.frame(
            &mut app,
            &[
                moved(center),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)),
            ],
        );
        let p = h.frame(
            &mut app,
            &[moved(Point::new(center.x + 9.0, center.y + 5.0))],
        );
        h.frame(
            &mut app,
            &[
                moved(center),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)),
            ],
        );
        p
    };
    let fitted = app.viewport.camera;
    let mut frames = Vec::new();
    for run in ["ilk", "ısınmış"] {
        app.viewport.camera = fitted;
        h.frame(&mut app, &[moved(center)]);
        for (name, f) in trace(&mut app, &mut h, 120) {
            frames.push(json!({ "run": run, "view": "uzak", "trace": name, "stats": stats(&f) }));
        }
        // Near: 1:200 over the drawing's middle cell.
        let e = mixed::extent(n);
        app.viewport.camera.center.x = (e[0] + e[2]) / 2.0;
        app.viewport.camera.center.y = (e[1] + e[3]) / 2.0;
        app.viewport.camera.scale = NEAR;
        h.frame(&mut app, &[moved(center)]);
        for (name, f) in trace(&mut app, &mut h, 60) {
            frames.push(
                json!({ "run": run, "view": "yakın 1:200", "trace": name, "stats": stats(&f) }),
            );
        }
    }
    let near_stats = view_stats(&app);
    // Keys: into the command line, then into the value field while drawing a line (near view).
    h.frame(&mut app, &[moved(center)]);
    let line_keys: Vec<Parts> = "cizgi uzunlugu"
        .chars()
        .map(|c| h.frame(&mut app, &typed(c)))
        .collect();
    app.command_input.clear();
    h.frame(&mut app, &key(key::Named::Escape));
    h.frame_with(&mut app, &[], |app| {
        let _ = app.run("tool.line");
    });
    h.frame(&mut app, &click(Point::new(center.x - 100.0, center.y)));
    h.frame(
        &mut app,
        &[moved(Point::new(center.x + 60.0, center.y - 40.0))],
    );
    let field_keys: Vec<Parts> = "12.5,7.25"
        .chars()
        .map(|c| h.frame(&mut app, &typed(c)))
        .collect();
    let field_open = app.field.is_some();
    for _ in 0..3 {
        h.frame(&mut app, &key(key::Named::Escape));
    }
    let _ = app.run("tool.select");
    h.frame(&mut app, &[]);
    // One small edit each, in the middle cell; undo and redo; then all selected and let go.
    let text = uid(&app, middle_id(n, 4));
    let text2 = uid(&app, middle_id(n, 5));
    let insert = uid(&app, middle_id(n, 7));
    let line = uid(&app, middle_id(n, 0));
    // The first command through the link pays once for what the link sets up (its catalogue): measured apart.
    let first = command(&mut app, &mut h, "İlk komut (bir yazıyı taşı)", |app| {
        run(
            app,
            json!({ "command": "cad.entities.transform", "op": "execute", "input": {
            "uids": [text.clone()], "transform": { "kind": "move", "dx": 0.25, "dy": 0.0 } } }),
        );
    });
    let mut commands = vec![
        first,
        command(&mut app, &mut h, "Bir yazıyı taşı", |app| {
            run(
                app,
                json!({ "command": "cad.entities.transform", "op": "execute", "input": {
                "uids": [text], "transform": { "kind": "move", "dx": 0.5, "dy": 0.25 } } }),
            );
        }),
        command(&mut app, &mut h, "Bir yazının metnini değiştir", |app| {
            run(
                app,
                json!({ "command": "cad.texts.edit", "op": "execute", "input": { "edit": {
                "kind": "replace", "uids": [text2], "find": "/", "with": "-", "matchCase": true, "wholeWord": false } } }),
            );
        }),
        command(&mut app, &mut h, "Bir blok yerleşimini taşı", |app| {
            run(
                app,
                json!({ "command": "cad.entities.transform", "op": "execute", "input": {
                "uids": [insert], "transform": { "kind": "move", "dx": -0.5, "dy": 0.5 } } }),
            );
        }),
        command(&mut app, &mut h, "Bir çizgiyi sil", |app| {
            run(
                app,
                json!({ "command": "cad.entities.delete", "op": "execute", "input": { "uids": [line] } }),
            );
        }),
    ];
    for (name, id) in [("Geri al", "edit.undo"), ("Yinele", "edit.redo")] {
        commands.push(command(&mut app, &mut h, name, |app| {
            let _ = app.run(id);
        }));
    }
    commands.push(command(&mut app, &mut h, "Hepsini seç (Ctrl+A)", |app| {
        let _ = app.run("edit.selectAll");
    }));
    let esc = h.frame(&mut app, &key(key::Named::Escape));
    commands
        .push(json!({ "command": "Seçimi bırak (Esc)", "uiMs": esc.cpu(), "gpuMs": esc.gpu_ms() }));
    let cancelled = cancel(&mut app, &mut h, file);
    json!({
        "objects": n,
        "readbackFloor": stats(&floor),
        "open": opened,
        "memory": {
            "residentBootMb": rss_boot, "residentOpenMb": after_open.0, "footprintOpen": after_open.1,
            "footprintEnd": footprint(), "residentEndMb": resident(),
        },
        "viewFar": after_open.2, "viewNear": near_stats,
        "firstInteraction": { "pointer": one(&first_move), "wheel": one(&first_wheel), "pan": one(&first_pan) },
        "frames": frames,
        "keys": { "commandLine": stats(&line_keys), "valueField": stats(&field_keys), "valueFieldOpened": field_open },
        "commands": commands,
        "cancel": cancelled,
    })
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The drawing's file: made by this code from the seed, kept in `.run/perf/` between runs.
pub(super) fn file_of(n: usize) -> PathBuf {
    root().join(format!(".run/perf/karma-{n}-s{}.kcad", mixed::SEED))
}

/// Makes the drawing of `n` objects and writes it as the app saves (`saving::write_shared`).
fn make(n: usize, file: &Path) -> Value {
    let t = Instant::now();
    let (snapshot, print) = mixed::drawing(n, mixed::SEED);
    let kinds = mixed::kinds(&snapshot);
    let doc = Document::from_v2(snapshot, None).expect("the drawing opens");
    let (head, objects) = doc.model.to_snapshot_v2_shared();
    crate::saving::write_shared(
        &head,
        &objects,
        file,
        &std::sync::atomic::AtomicBool::new(false),
        &mut |_| {},
        &crate::saving::Faults::NONE,
    )
    .expect("writes the drawing");
    json!({
        "fingerprint": print, "kinds": kinds, "bytes": std::fs::metadata(file).map_or(0, |m| m.len()),
        "makeMs": ms(t),
    })
}

/// One task in this process: `make:<n>` or `measure:<n>`; prints one line of JSON.
#[test]
#[ignore = "run by perf::large::large in a process of its own"]
fn child() {
    let Ok(task) = std::env::var("KENTOS_LARGE_CHILD") else {
        return;
    };
    let (op, n) = task.split_once(':').unwrap_or_default();
    let n: usize = n.parse().unwrap_or(0);
    let file = file_of(n);
    let out = match op {
        "make" => make(n, &file),
        _ => measure(n, &file),
    };
    println!("KENTOS_LARGE {out}");
}

fn run_child(task: &str) -> Value {
    let exe = std::env::current_exe().expect("the test binary");
    let output = Command::new(exe)
        .args(["perf::large::child", "--exact", "--ignored", "--nocapture"])
        .env("KENTOS_LARGE_CHILD", task)
        .output()
        .expect("runs");
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("KENTOS_LARGE "))
        .unwrap_or_else(|| {
            panic!(
                "{task}: no result\n{text}\n{}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
    serde_json::from_str(line).expect("JSON")
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn large() {
    let sizes: Vec<usize> = std::env::var("KENTOS_PERF_SIZES")
        .unwrap_or_else(|_| "10000,100000,1000000".into())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let label = std::env::var("KENTOS_PERF_LABEL").unwrap_or_else(|_| "latest".into());
    std::fs::create_dir_all(root().join(".run/perf")).expect("a folder for the files");
    let mut rows = Vec::new();
    for &n in &sizes {
        let file = file_of(n);
        let side = file.with_extension("json");
        let made = match std::fs::read_to_string(&side)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
        {
            Some(v) if file.exists() => v,
            _ => {
                let v = run_child(&format!("make:{n}"));
                std::fs::write(&side, v.to_string()).expect("writes the drawing's note");
                v
            }
        };
        println!("{n}: çizim {made}");
        let measured = run_child(&format!("measure:{n}"));
        println!("{n}: {measured}");
        rows.push(json!({ "objects": n, "drawing": made, "measured": measured }));
    }
    let env = environment(&root());
    let report = json!({
        "label": label, "environment": env, "gpu": adapter(),
        "setup": { "sizes": sizes, "window": [super::frame::WINDOW.width, super::frame::WINDOW.height],
            "profile": "release (lto thin)", "seed": mixed::SEED, "budgetMs": BUDGET_MS,
            "shortcuts": std::env::var("KENTOS_PERF_SHORTCUTS").unwrap_or_else(|_| "on".into()) },
        "rows": rows,
    });
    let date = super::flat::chrono_free_date();
    match std::env::var("KENTOS_PERF_OUT") {
        Ok(out) => {
            let file = root()
                .join(out)
                .join(format!("large-desktop-{label}-{date}.json"));
            std::fs::write(
                &file,
                format!("{}\n", serde_json::to_string_pretty(&report).expect("JSON")),
            )
            .expect("writes the report");
            println!("Yazıldı: {}", file.display());
        }
        Err(_) => println!("{}", serde_json::to_string_pretty(&report).expect("JSON")),
    }
}

/// `KENTOS_PERF_SETTINGS` (`graphics.msaa=1;graphics.hiDpi=false`): typed settings chosen before a diagnosis runs,
/// as the settings window would; never set for a measured report.
pub(super) fn settings_from_env(app: &mut App) {
    let Ok(list) = std::env::var("KENTOS_PERF_SETTINGS") else {
        return;
    };
    for one in list.split(';').filter(|s| !s.trim().is_empty()) {
        let (key, value) = one.split_once('=').expect("key=value");
        let value = serde_json::from_str(value).unwrap_or_else(|_| Value::from(value));
        let refused = app.settings.choose(&[(key.trim(), value)]);
        assert!(refused.is_empty(), "{one}: refused");
    }
    app.apply_settings();
}

/// `KENTOS_PERF_SHORTCUTS` (docs/adr/1063, 1064, 1065): the drawing area's shortcuts a run measures, `off` (the plain
/// build, the "önce" of a comparison), `instancing`, `lod` or `line` (one of them), `adr1063` (the first two, without
/// the long strokes' levels: the "önce" of ADR 1064), `adr1064` (the three, without the blocks' far proxy: the "önce"
/// of ADR 1065); unset, all four (the app's own).
pub(super) fn shortcuts_from_env(app: &mut App) {
    let asked = std::env::var("KENTOS_PERF_SHORTCUTS").unwrap_or_default();
    let (instancing, lod, line, proxy) = match asked.as_str() {
        "off" => (false, false, false, false),
        "instancing" => (true, false, false, false),
        "lod" => (false, true, false, false),
        "line" => (false, false, true, false),
        "adr1063" => (true, true, false, false),
        "adr1064" => (true, true, true, false),
        _ => (true, true, true, true),
    };
    app.styles.instancing = instancing;
    app.styles.curve_lod = lod;
    app.styles.line_lod = line;
    app.styles.block_proxy = proxy;
    // A moving view's turned texts from their pictures kept for another quarter pixel (docs/adr/1064 §6): with the
    // long strokes' levels, off in the "önce" of ADR 1064.
    crate::label_picture::set_nearest_while_moving(!matches!(asked.as_str(), "off" | "adr1063"));
}

/// Which kind of object the far view's GPU time goes to (docs/adr/1062 §3): the mixed drawing of
/// `KENTOS_PERF_SIZES` (100 000) with one layer's objects only, the whole of it in the window, panned; the GPU's p50
/// and the view's buffers per layer, then all of them.
///
/// `cargo test --release -p kentos-desktop --lib perf::large::layers -- --ignored --nocapture --test-threads=1`
#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn layers() {
    let n: usize = std::env::var("KENTOS_PERF_SIZES")
        .ok()
        .and_then(|v| v.split(',').next().and_then(|s| s.trim().parse().ok()))
        .unwrap_or(100_000);
    let (whole, _) = mixed::drawing(n, mixed::SEED);
    let mut h = Harness::new();
    let only_layers: Vec<String> = std::env::var("KENTOS_PERF_LAYERS")
        .map(|v| v.split(',').map(str::to_owned).collect())
        .unwrap_or_default();
    for only in mixed::LAYERS.iter().map(|l| Some(l.0)).chain([None]) {
        if !only_layers.is_empty() && !only_layers.iter().any(|l| Some(l.as_str()) == only) {
            continue;
        }
        let mut snapshot = whole.clone();
        if let Some(layer) = only {
            let keep: Vec<bool> = snapshot
                .entities
                .iter()
                .map(|e| e.base().layer_id == layer)
                .collect();
            let mut k = keep.iter();
            snapshot.entities.retain(|_| *k.next().unwrap_or(&false));
            let mut k = keep.iter();
            snapshot.uids.retain(|_| *k.next().unwrap_or(&false));
        }
        let count = snapshot.entities.len();
        let (mut app, _) = App::boot(None);
        settings_from_env(&mut app);
        shortcuts_from_env(&mut app);
        h.settle(&mut app);
        let doc = Document::from_v2(snapshot, None).expect("the drawing opens");
        let first = h.frame_with(&mut app, &[], move |app| {
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        });
        app.dialog = None;
        h.settle(&mut app);
        // The whole drawing's box in the window, whatever one layer's own box is.
        let e = mixed::extent(n);
        app.viewport.show(&kentos_render_wgpu::Bounds {
            min_x: e[0],
            min_y: e[1],
            max_x: e[2],
            max_y: e[3],
        });
        h.frame(&mut app, &[]);
        let center = app.viewport.bounds.center();
        let frames = pan(&mut app, &mut h, center);
        let s = stats(&frames);
        println!(
            "{n} karma, katman {}: {count} nesne; ilk kare MİB {:.1} ms; kaydırma GPU p50 {:.1} ms p95 {:.1} ms, MİB p50 {:.2} ms; görünüm {}",
            only.unwrap_or("hepsi"),
            first.cpu(),
            s["gpuP50"].as_f64().unwrap_or(f64::NAN),
            s["gpuP95"].as_f64().unwrap_or(f64::NAN),
            s["uiP50"].as_f64().unwrap_or(f64::NAN),
            view_stats(&app)
        );
    }
}
