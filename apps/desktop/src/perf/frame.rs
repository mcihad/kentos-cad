//! The desktop's frame on a large drawing, measured (TODOS.md PERF-03,
//! PERF-08): what the UI thread does for one input, as Iced's runtime runs
//! it. The widgets take the event (“olay”), the app takes their messages
//! (“uygulama”), `App::view` builds the element tree (“görünüm”; the
//! drawing's scene is built here when it is out of date), Iced compares the
//! tree with the last one and lays it out (“düzen”), and the widgets record
//! the frame (“çizim”). The GPU's part (text and the scene prepared, the
//! frame drawn) is measured apart, with the picture read back (“GPU”), and
//! only with the wgpu renderer. Not a correctness test and not run by
//! default:
//!
//! ```text
//! KENTOS_PERF_LABEL=before KENTOS_PERF_OUT=docs/perf \
//!   cargo test --release -p kentos-desktop perf::frame -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `KENTOS_PERF_SIZES` (parcels, comma separated) changes the drawings;
//! `KENTOS_SNAPSHOT_BACKEND=tiny-skia` the renderer; `KENTOS_PERF_OUT`
//! writes `frame-desktop-<label>-<date>.{json,md}` there instead of printing.
//!
//! The last scenarios, up to 10 000 parcels, run Parsel ölçü yazıları on
//! every parcel: here, then on another thread (docs/adr/0125), where the UI
//! thread only starts it and applies its answer; how long the thread took
//! is printed.

use std::time::Instant;

use iced::advanced::clipboard;
use iced::advanced::renderer::{self, Headless};
use iced::futures::StreamExt as _;
use iced::keyboard::{self, key};
use iced::{Event, Pixels, Point, Rectangle, Renderer, Size, mouse, window};
use iced_runtime::user_interface::{self, UserInterface};
use kentos_ui::theme::typography;
use serde_json::{Value, json};

use super::{drawing, environment, median, ms};
use crate::app::{App, Message};
use crate::document::Document;

/// The window measured: the owner's reference size.
pub(super) const WINDOW: Size = Size::new(1440.0, 900.0);

/// One frame's parts, milliseconds.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Parts {
    event: f64,
    app: f64,
    view: f64,
    layout: f64,
    draw: f64,
    gpu: Option<f64>,
}

impl Parts {
    /// The UI thread's share: everything but the GPU's.
    fn cpu(&self) -> f64 {
        self.event + self.app + self.view + self.layout + self.draw
    }
}

/// The interface without a window, driven as Iced's runtime drives it.
pub(super) struct Harness {
    pub(super) renderer: Renderer,
    cache: user_interface::Cache,
    cursor: mouse::Cursor,
    gpu: bool,
}

fn redraw() -> Event {
    Event::Window(window::Event::RedrawRequested(iced::time::Instant::now()))
}

pub(super) fn moved(position: Point) -> Event {
    Event::Mouse(mouse::Event::CursorMoved { position })
}

/// A left click at a place: the pointer there, the button pressed and let go.
fn click(position: Point) -> [Event; 3] {
    [
        moved(position),
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]
}

impl Harness {
    pub(super) fn new() -> Self {
        typography::load();
        let asked = std::env::var("KENTOS_SNAPSHOT_BACKEND").unwrap_or_else(|_| "wgpu".into());
        let make = |backend: &str| {
            iced::futures::executor::block_on(<Renderer as Headless>::new(
                typography::ui(),
                Pixels(16.0),
                Some(backend),
            ))
        };
        let renderer = make(&asked)
            .or_else(|| make("tiny-skia"))
            .expect("a renderer, wgpu or tiny-skia");
        let gpu = renderer.name().contains("wgpu");
        Self {
            renderer,
            cache: user_interface::Cache::default(),
            cursor: mouse::Cursor::Unavailable,
            gpu,
        }
    }

    /// One input: `act` is the app taking it (by default, the widgets'
    /// messages for `events`), then the interface built, laid out and drawn.
    pub(super) fn frame_with(
        &mut self,
        app: &mut App,
        events: &[Event],
        act: impl FnOnce(&mut App),
    ) -> Parts {
        for event in events {
            if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
                self.cursor = mouse::Cursor::Available(*position);
            }
        }
        let mut parts = Parts::default();
        // The interface the last frame left: the runtime keeps it between
        // inputs, so building it again here is not measured.
        let mut messages = Vec::new();
        if !events.is_empty() {
            let mut ui = UserInterface::build(
                app.view(),
                WINDOW,
                std::mem::take(&mut self.cache),
                &mut self.renderer,
            );
            let t = Instant::now();
            let (_, statuses) = ui.update(
                events,
                self.cursor,
                &mut self.renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            // Keys reach the app through its subscription, after the widgets saw them.
            for (event, status) in events.iter().zip(statuses) {
                if let Event::Keyboard(_) = event
                    && let Some(message) =
                        crate::keys::key_event(event.clone(), status, window::Id::unique())
                {
                    messages.push(message);
                }
            }
            parts.event = ms(t);
            self.cache = ui.into_cache();
        }
        let t = Instant::now();
        for message in messages {
            let _ = app.update(message);
        }
        act(app);
        parts.app = ms(t);
        let theme = app.theme();
        let t = Instant::now();
        let element = app.view();
        parts.view = ms(t);
        let t = Instant::now();
        let mut ui = UserInterface::build(
            element,
            WINDOW,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );
        parts.layout = ms(t);
        let t = Instant::now();
        let mut later = Vec::new();
        let _ = ui.update(
            &[redraw()],
            self.cursor,
            &mut self.renderer,
            &mut clipboard::Null,
            &mut later,
        );
        ui.draw(
            &mut self.renderer,
            &theme,
            &renderer::Style {
                text_color: theme.palette().text,
            },
            self.cursor,
        );
        parts.draw = ms(t);
        self.cache = ui.into_cache();
        if self.gpu {
            let t = Instant::now();
            let _ = self.renderer.screenshot(
                Size::new(WINDOW.width as u32, WINDOW.height as u32),
                1.0,
                theme.palette().background,
            );
            parts.gpu = Some(ms(t));
        }
        // What the redraw asked for goes to the app before the next input, as in the runtime.
        for message in later {
            let _ = app.update(message);
        }
        parts
    }

    pub(super) fn frame(&mut self, app: &mut App, events: &[Event]) -> Parts {
        self.frame_with(app, events, |_| {})
    }

    /// Frames until the interface asks for nothing more (the area's size, the first fit).
    pub(super) fn settle(&mut self, app: &mut App) {
        for _ in 0..12 {
            let mut messages = Vec::new();
            let mut ui = UserInterface::build(
                app.view(),
                WINDOW,
                std::mem::take(&mut self.cache),
                &mut self.renderer,
            );
            let _ = ui.update(
                &[redraw()],
                self.cursor,
                &mut self.renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            self.cache = ui.into_cache();
            if messages.is_empty() {
                break;
            }
            for message in messages {
                let _ = app.update(message);
            }
        }
    }
}

/// A pan with the middle button from `center`, 20 steps out and back: its frames.
pub(super) fn pan(app: &mut App, h: &mut Harness, center: Point) -> Vec<Parts> {
    h.frame(
        app,
        &[
            moved(center),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)),
        ],
    );
    let frames = (1..=40)
        .map(|i| {
            let step = if i <= 20 { i as f32 } else { (40 - i) as f32 };
            h.frame(
                app,
                &[moved(Point::new(
                    center.x + 9.0 * step,
                    center.y + 5.0 * step,
                ))],
            )
        })
        .collect();
    h.frame(
        app,
        &[Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Middle,
        ))],
    );
    frames
}

/// A key pressed and let go, as the window gives it.
fn key(named: key::Named) -> Vec<Event> {
    let physical = key::Physical::Unidentified(key::NativeCode::Unidentified);
    let key = keyboard::Key::Named(named);
    vec![
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key.clone(),
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }),
        Event::Keyboard(keyboard::Event::KeyReleased {
            key: key.clone(),
            modified_key: key,
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
        }),
    ]
}

/// `count` places over the drawing area, along a path that crosses it
/// (a Lissajous figure), so the pointer passes over many objects.
fn path(area: Rectangle, count: usize) -> Vec<Point> {
    (0..count)
        .map(|i| {
            let t = i as f32 / count as f32 * std::f32::consts::TAU;
            Point::new(
                area.x + area.width * (0.5 + 0.42 * (3.0 * t).sin()),
                area.y + area.height * (0.5 + 0.42 * (2.0 * t).cos()),
            )
        })
        .collect()
}

/// The app with `n` parcels open, fitted in the window.
/// The frame that puts the drawing on screen is measured: the app takes
/// the read drawing (the store, the styled layers' first build in the view,
/// the first upload and picture), the file's reading and decoding aside.
fn app_with(n: usize) -> (App, Harness, Parts) {
    let (mut app, _) = App::boot(None);
    let doc = Document::from_v2(drawing(n), None).expect("the drawing opens");
    let mut harness = Harness::new();
    harness.settle(&mut app);
    let first = harness.frame_with(&mut app, &[], move |app| {
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    });
    harness.settle(&mut app);
    (app, harness, first)
}

/// Every scenario on a drawing of `n` parcels: its name and its frames.
fn scenarios(n: usize) -> (String, Vec<(&'static str, Vec<Parts>)>) {
    let (mut app, mut h, first) = app_with(n);
    let renderer = h.renderer.name();
    let area = app.viewport.bounds;
    let center = area.center();
    let mut out = vec![("Çizimi açma (ilk kare)", vec![first])];
    // Warm the caches the first frames fill (the scene, text), unmeasured.
    for p in path(area, 10) {
        h.frame(&mut app, &[moved(p)]);
    }

    out.push((
        "Boş kare (yeniden çizim)",
        (0..30).map(|_| h.frame(&mut app, &[])).collect(),
    ));
    out.push((
        "İmleç çizimin üstünde (seçim aracı)",
        path(area, 120)
            .into_iter()
            .map(|p| h.frame(&mut app, &[moved(p)]))
            .collect(),
    ));
    let wheel = |lines: f32| {
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: lines },
        })
    };
    h.frame(&mut app, &[moved(center)]);
    let mut frames = Vec::new();
    for round in 0..2 {
        for lines in [1.0, -1.0] {
            for _ in 0..8 {
                frames.push(h.frame(&mut app, &[wheel(lines)]));
            }
        }
        // Screen-sized symbols settle after the wheel stops (the web's 150 ms).
        if round == 0 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            h.frame(&mut app, &[]);
        }
    }
    out.push(("Tekerlekle yakınlaştırma", frames));
    std::thread::sleep(std::time::Duration::from_millis(200));
    h.frame(&mut app, &[]);

    let frames = pan(&mut app, &mut h, center);
    out.push(("Orta tuşla kaydırma", frames));

    // The same pan at 1:1000 (96 dpi), where a parcel is about 120 px wide
    // and its label shows: most of the drawing is out of view.
    let fitted = app.viewport.camera;
    app.viewport.camera.scale = 1.0 / (1000.0 * 0.000_264_58);
    assert!((app.viewport.camera.screen_scale() - 1000.0).abs() < 1.0);
    let frames = pan(&mut app, &mut h, center);
    out.push(("Orta tuşla kaydırma, 1:1000", frames));
    app.viewport.camera = fitted;
    h.frame(&mut app, &[moved(center)]);

    h.frame_with(&mut app, &[], |app| {
        let _ = app.run("tool.polyline");
    });
    let start = Point::new(area.x + area.width * 0.3, area.y + area.height * 0.4);
    h.frame(
        &mut app,
        &[
            moved(start),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    );
    out.push((
        "Çoklu çizgi çizerken imleç (kenet açık)",
        path(area, 120)
            .into_iter()
            .map(|p| h.frame(&mut app, &[moved(p)]))
            .collect(),
    ));
    h.frame(&mut app, &key(key::Named::Escape));
    h.frame(&mut app, &key(key::Named::Escape));

    // One object clicked, deleted, and the step taken back and again.
    h.frame(
        &mut app,
        &[
            moved(center),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    );
    let before = app.document.as_ref().map_or(0, Document::entity_count);
    let clicked = app.selection.len();
    out.push((
        "Bir nesneyi silme",
        vec![h.frame_with(&mut app, &[], |app| {
            let _ = app.run("tool.erase");
        })],
    ));
    let after = app.document.as_ref().map_or(0, Document::entity_count);
    assert!(
        n == 0 || (clicked == 1 && after + 1 == before),
        "one object clicked ({clicked}) and deleted ({before} → {after})"
    );
    out.push((
        "Geri al ve yinele (bir nesne)",
        (0..10)
            .map(|i| {
                h.frame_with(&mut app, &[], |app| {
                    let _ = app.run(if i % 2 == 0 { "edit.undo" } else { "edit.redo" });
                })
            })
            .collect(),
    ));
    let _ = app.run("tool.select");

    out.push((
        "Hepsini seç (Ctrl+A)",
        vec![h.frame_with(&mut app, &[], |app| {
            let _ = app.run("edit.selectAll");
        })],
    ));
    out.push((
        "İmleç çizimin üstünde (hepsi seçili)",
        path(area, 60)
            .into_iter()
            .map(|p| h.frame(&mut app, &[moved(p)]))
            .collect(),
    ));
    // Everything moved with Taşı: the base point, the pointer on the way, the second point, undone.
    h.frame_with(&mut app, &[], |app| {
        let _ = app.run("tool.move");
    });
    let from = Point::new(area.x + area.width * 0.4, area.y + area.height * 0.5);
    h.frame(&mut app, &click(from));
    out.push((
        "Taşırken imleç (hepsi seçili)",
        (1..=30)
            .map(|i| {
                let to = Point::new(from.x + 2.0 * i as f32, from.y + 1.5 * i as f32);
                h.frame(&mut app, &[moved(to)])
            })
            .collect(),
    ));
    let to = Point::new(from.x + 62.0, from.y + 46.5);
    let before = app.document.as_ref().map(|d| d.model.revision());
    out.push((
        "Hepsini taşıma (ikinci nokta)",
        vec![h.frame(&mut app, &click(to))],
    ));
    assert!(
        n == 0 || app.document.as_ref().map(|d| d.model.revision()) != before,
        "the move was applied"
    );
    out.push((
        "Taşımayı geri alma",
        vec![h.frame_with(&mut app, &[], |app| {
            let _ = app.run("edit.undo");
        })],
    ));
    let _ = app.run("tool.select");
    out.push((
        "Seçimi bırakma (Esc)",
        vec![h.frame(&mut app, &key(key::Named::Escape))],
    ));
    assert!(app.selection.is_empty(), "Esc let the selection go");
    // Up to 10 000 parcels: the model writes 40 objects a parcel (400 000
    // here), run twice with a copy between; at 50 000 that is 2 million
    // objects and more memory than the measuring machine (15 GB) has.
    if (1..=10_000).contains(&n) {
        processing(&mut app, &mut h, &mut out);
    }
    (renderer, out)
}

/// Parsel ölçü yazıları on every parcel: here, where the UI thread does it
/// all, then on another thread, where it starts the run and later applies
/// the answer (docs/adr/0125).
fn processing(app: &mut App, h: &mut Harness, out: &mut Vec<(&'static str, Vec<Parts>)>) {
    use crate::processing::Event;
    let revision = |app: &App| app.document.as_ref().map(|d| d.model.revision());
    let _ = app.run("edit.selectAll");
    let _ = app.update(Message::Run("processing.model.builtin.parcelSheet"));
    let _ = app.update(Message::Processing(Event::Target("client".into())));
    h.settle(app);
    let before = revision(app);
    out.push((
        "Parsel ölçü yazıları, bu bilgisayarda",
        vec![h.frame_with(app, &[], |app| {
            let _ = app.update(Message::Processing(Event::Run));
        })],
    ));
    assert_ne!(revision(app), before, "the model ran here");
    let _ = app.run("edit.undo");
    let _ = app.update(Message::Processing(Event::Target("worker".into())));
    h.settle(app);
    let before = revision(app);
    let mut task = None;
    let started = Instant::now();
    out.push((
        "Parsel ölçü yazıları, arka planda: Çalıştır",
        vec![h.frame_with(app, &[], |app| {
            task = Some(app.update(Message::Processing(Event::Run)));
        })],
    ));
    // What the thread says, heard once it is done.
    let mut said = Vec::new();
    if let Some(mut stream) = task.and_then(iced_runtime::task::into_stream) {
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Output(message) = action {
                said.push(message);
            }
        }
    }
    println!(
        "arka plandaki iş: {:.0} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    let at = said
        .iter()
        .position(crate::processing::is_answer)
        .expect("the thread's answer");
    let answer = said.remove(at);
    for message in said {
        let _ = app.update(message);
    }
    assert_eq!(revision(app), before, "nothing changes before the answer");
    out.push((
        "Parsel ölçü yazıları, arka planda: sonucun uygulanması",
        vec![h.frame_with(app, &[], |app| {
            let _ = app.update(answer);
        })],
    ));
    assert_ne!(revision(app), before, "the answer was applied");
    // The drawing with the result, the run's window closed: the pointer
    // moving over it, frame by frame.
    app.processing.dialog = None;
    app.dialog = None;
    h.settle(app);
    let area = app.viewport.bounds;
    out.push((
        "Parsel ölçü yazıları: sonuçlu çizimde imleç",
        path(area, 20)
            .into_iter()
            .map(|p| h.frame(app, &[moved(p)]))
            .collect(),
    ));
    out.push((
        "Parsel ölçü yazıları: sonuçlu çizimde kaydırma",
        pan(app, h, area.center()),
    ));
    // At 1:1000 the labels of the parcels in view show.
    let fitted = app.viewport.camera;
    app.viewport.camera.scale = 1.0 / (1000.0 * 0.000_264_58);
    h.frame(app, &[moved(area.center())]);
    out.push((
        "Parsel ölçü yazıları: sonuçlu çizimde kaydırma, 1:1000",
        pan(app, h, area.center()),
    ));
    app.viewport.camera = fitted;
}

fn percentile(xs: &[f64], p: f64) -> f64 {
    let mut xs = xs.to_vec();
    xs.sort_by(f64::total_cmp);
    if xs.is_empty() {
        return f64::NAN;
    }
    let at = ((xs.len() - 1) as f64 * p).round() as usize;
    xs[at.min(xs.len() - 1)]
}

pub(super) fn summary(frames: &[Parts]) -> Value {
    let part = |f: &dyn Fn(&Parts) -> f64| median(frames.iter().map(f).collect());
    let cpu: Vec<f64> = frames.iter().map(Parts::cpu).collect();
    let gpu: Vec<f64> = frames.iter().filter_map(|p| p.gpu).collect();
    json!({
        "frames": frames.len(),
        "cpuP50": percentile(&cpu, 0.5),
        "cpuP95": percentile(&cpu, 0.95),
        "cpuMax": percentile(&cpu, 1.0),
        "eventP50": part(&|p| p.event),
        "appP50": part(&|p| p.app),
        "viewP50": part(&|p| p.view),
        "layoutP50": part(&|p| p.layout),
        "drawP50": part(&|p| p.draw),
        "gpuP50": if gpu.is_empty() { Value::Null } else { json!(percentile(&gpu, 0.5)) },
    })
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn frame() {
    let sizes: Vec<usize> = std::env::var("KENTOS_PERF_SIZES")
        .unwrap_or_else(|_| "0,10000,50000,100000".into())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let label = std::env::var("KENTOS_PERF_LABEL").unwrap_or_else(|_| "latest".into());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut rows = Vec::new();
    let mut renderer = String::new();
    for &n in &sizes {
        let (name, runs) = scenarios(n);
        renderer = name;
        for (scenario, frames) in runs {
            let s = summary(&frames);
            println!("{n} parsel, {scenario}: {s}");
            rows.push(json!({ "parcels": n, "scenario": scenario, "summary": s }));
        }
    }
    let env = environment(&root);
    let (date, commit, dirty) = (
        env["date"].as_str().unwrap_or_default().to_owned(),
        env["commit"].as_str().unwrap_or_default().to_owned(),
        env["dirtyTree"].as_bool().unwrap_or(false),
    );
    let report = json!({
        "label": label,
        "environment": env,
        "setup": {
            "sizes": sizes,
            "window": [WINDOW.width, WINDOW.height],
            "renderer": renderer,
            "profile": "release (lto thin)",
        },
        "rows": rows,
    });
    let f = |v: &Value| v.as_f64().map_or("–".to_owned(), |x| format!("{x:.2}"));
    let mut md = vec![
        format!(
            "# Masaüstünün karesi büyük çizimde: {label} ({date}, {commit}{})",
            if dirty {
                ", kaydedilmemiş değişiklikle"
            } else {
                ""
            }
        ),
        String::new(),
        format!(
            "{}, `--release` (lto thin). Pencere {}×{}, çizici {renderer}. Test `apps/desktop/src/perf/frame.rs`: arayüz pencere açmadan Iced'in çalışma zamanının sırasıyla sürülür. “Olay” bileşenlerin olayı alması, “uygulama” mesajların uygulanması, “görünüm” `App::view` (sahne eskidiyse kurulması dahil), “düzen” Iced'in ağacı karşılaştırıp yerleştirmesi, “çizim” bileşenlerin kareyi kaydetmesidir; “MİB” bunların toplamıdır (arayüz iş parçacığı). “GPU” karenin GPU'da hazırlanıp çizilmesi ve resmin geri okunmasıdır (yalnız wgpu ile; geri okuma pencerede yoktur).",
            env["machineText"].as_str().unwrap_or_default(),
            WINDOW.width,
            WINDOW.height,
        ),
        String::new(),
        "Çizim: parsel başına 20 köşeli bir alan, üç öznitelik ve etiket, tek katman (`perf.rs`'in çizimi), bütünü pencereye sığmış. Süreler milisaniye; parçalar ortanca.".to_owned(),
        String::new(),
        "| Parsel | Durum | Kare | MİB p50 | MİB p95 | MİB en çok | Olay | Uygulama | Görünüm | Düzen | Çizim | GPU p50 |".to_owned(),
        "|---|---|---|---|---|---|---|---|---|---|---|---|".to_owned(),
    ];
    for r in &rows {
        let s = &r["summary"];
        md.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r["parcels"],
            r["scenario"].as_str().unwrap_or_default(),
            s["frames"],
            f(&s["cpuP50"]),
            f(&s["cpuP95"]),
            f(&s["cpuMax"]),
            f(&s["eventP50"]),
            f(&s["appP50"]),
            f(&s["viewP50"]),
            f(&s["layoutP50"]),
            f(&s["drawP50"]),
            f(&s["gpuP50"]),
        ));
    }
    match std::env::var("KENTOS_PERF_OUT") {
        Ok(out) => {
            let base = root.join(out).join(format!("frame-desktop-{label}-{date}"));
            std::fs::write(
                base.with_extension("json"),
                format!("{}\n", serde_json::to_string_pretty(&report).expect("JSON")),
            )
            .expect("writes the report");
            std::fs::write(base.with_extension("md"), format!("{}\n", md.join("\n")))
                .expect("writes the report");
            println!("Yazıldı: {}.{{json,md}}", base.display());
        }
        Err(_) => println!("{}", md.join("\n")),
    }
}
