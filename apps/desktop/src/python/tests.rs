//! The console on the desktop's side (docs/adr/0132): what it answers on the
//! drawing, its undo step, and the drawing held while code runs. A stand-in
//! process's messages are fed in as the real one's would come; the real
//! Python runs in `real_python_writes_the_open_drawing` when one with the
//! kentos package is at hand (KENTOS_PYTHON, or the checkout's .run/py).

use serde_json::{Value, json};

use super::{Event, Kind, Said};
use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;

const SQUARE: &str = r#"[{"x": 423500, "y": 4512300}, {"x": 423520, "y": 4512300}, {"x": 423520, "y": 4512312.5}, {"x": 423500, "y": 4512312.5}]"#;

/// A run under way with no process: its messages are fed by the test.
fn running(app: &mut App) -> u64 {
    app.python.runs += 1;
    let id = app.python.runs;
    app.python.running = Some(super::Run {
        id,
        session: app.document.as_ref().map_or(0, |d| d.session),
        group: None,
        label: None,
        depth: 0,
        poisoned: false,
        told: false,
        title: "test".into(),
    });
    id
}

fn said(app: &mut App, message: Value) {
    let generation = app.python.generation;
    let _ = app.update(Message::Python(Event::Host(
        generation,
        Said::Message(message),
    )));
}

fn ask(app: &mut App, method: &str, params: Value) -> Result<Value, (String, String)> {
    app.python_answer(method, &params)
        .map_err(|e| (e.code.to_owned(), e.message))
}

fn polygon(app: &mut App) -> Value {
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a drawing");
    let input: Value =
        serde_json::from_str(&format!(r#"{{"layerId": "{layer}", "pts": {SQUARE}}}"#))
            .expect("the input");
    ask(
        app,
        "run",
        json!({"command": "cad.polygon.create", "op": "execute", "input": input}),
    )
    .expect("runs")
}

fn objects(app: &App) -> usize {
    app.document.as_ref().map_or(0, |d| d.model.len())
}

#[test]
fn a_run_that_ends_well_is_one_undo_step_named_python() {
    let mut app = app_with_drawing();
    let before = objects(&app);
    let id = running(&mut app);
    assert_eq!(polygon(&mut app)["status"], "completed");
    assert_eq!(polygon(&mut app)["status"], "completed");
    assert_eq!(objects(&app), before + 2);
    said(&mut app, json!({"type": "done", "id": id, "ok": true}));
    assert!(app.python.running.is_none());
    let model = &mut app.document.as_mut().expect("open").model;
    assert_eq!(
        model.undo().as_deref(),
        Some(super::STEP),
        "both in one step"
    );
    assert_eq!(model.len(), before);
}

#[test]
fn a_run_that_raises_takes_back_what_it_wrote_and_shows_the_traceback() {
    let mut app = app_with_drawing();
    let before = objects(&app);
    let could_undo = app.document.as_ref().is_some_and(|d| d.model.can_undo());
    let id = running(&mut app);
    polygon(&mut app);
    said(
        &mut app,
        json!({"type": "done", "id": id, "ok": false, "exception": "ValueError",
               "error": "Traceback (most recent call last):\n  File \"<konsol 1>\", line 2\nValueError: bozuk\n"}),
    );
    assert_eq!(objects(&app), before);
    assert_eq!(
        app.document.as_ref().is_some_and(|d| d.model.can_undo()),
        could_undo,
        "nothing recorded"
    );
    let errors: Vec<&str> = app
        .python
        .lines
        .iter()
        .filter(|l| l.kind == Kind::Error)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(errors.last().copied(), Some("ValueError: bozuk"));
    assert!(
        app.python
            .lines
            .iter()
            .any(|l| l.kind == Kind::Note && l.text.contains("geri alındı"))
    );
}

#[test]
fn a_group_names_the_step_and_a_failed_one_takes_the_whole_run_back() {
    let mut app = app_with_drawing();
    let id = running(&mut app);
    ask(&mut app, "begin_group", json!({"label": "Parseller"})).expect("begun");
    polygon(&mut app);
    ask(&mut app, "end_group", json!({})).expect("ended");
    said(&mut app, json!({"type": "done", "id": id, "ok": true}));
    let model = &mut app.document.as_mut().expect("open").model;
    assert_eq!(model.undo().as_deref(), Some("Parseller"));

    let mut app = app_with_drawing();
    let before = objects(&app);
    let id = running(&mut app);
    polygon(&mut app);
    ask(&mut app, "begin_group", json!({"label": "Bozuk"})).expect("begun");
    ask(&mut app, "cancel_group", json!({})).expect("cancelled");
    // The script caught its error and went on: the run is taken back all the same.
    said(&mut app, json!({"type": "done", "id": id, "ok": true}));
    assert_eq!(objects(&app), before);
}

#[test]
fn stopping_takes_the_run_back_and_later_messages_of_that_process_are_dropped() {
    let mut app = app_with_drawing();
    let before = objects(&app);
    let id = running(&mut app);
    polygon(&mut app);
    let old = app.python.generation;
    let _ = app.update(Message::Python(Event::Stop));
    assert_eq!(objects(&app), before);
    assert!(app.python.running.is_none());
    assert_ne!(app.python.generation, old);
    let lines = app.python.lines.len();
    let _ = app.update(Message::Python(Event::Host(
        old,
        Said::Message(json!({"type": "out", "stream": "out", "text": "geç kalan\n"})),
    )));
    let _ = app.update(Message::Python(Event::Host(
        old,
        Said::Message(json!({"type": "done", "id": id, "ok": true})),
    )));
    assert_eq!(
        app.python.lines.len(),
        lines,
        "a stopped process is not heard"
    );
}

#[test]
fn the_drawing_is_held_while_code_runs_and_said_once() {
    let mut app = app_with_drawing();
    let before = objects(&app);
    running(&mut app);
    let _ = app.update(Message::Run("draw.line"));
    let _ = app.update(Message::Run("edit.undo"));
    let _ = app.update(Message::CommandRun("L".into()));
    assert!(!app.session.is_running(), "no tool started");
    assert_eq!(objects(&app), before);
    let told = app
        .log
        .lines()
        .filter(|l| l.text.contains("Python kodu çalışıyor"))
        .count();
    assert_eq!(told, 1);
}

#[test]
fn a_run_that_fails_before_writing_says_nothing_was_taken_back() {
    let mut app = app_with_drawing();
    let id = running(&mut app);
    let refused = ask(
        &mut app,
        "run",
        json!({"command": "cad.polygon.create", "op": "execute",
               "input": {"layerId": "yok", "pts": [{"x": 0, "y": 0}, {"x": 1, "y": 0}, {"x": 1, "y": 1}]}}),
    )
    .expect("answers");
    assert_eq!(refused["status"], "failed");
    let warned = app.log.unseen();
    said(
        &mut app,
        json!({"type": "done", "id": id, "ok": false, "exception": "CommandFailed", "error": "CommandFailed: …\n"}),
    );
    assert_eq!(app.log.unseen(), warned, "no warning: nothing was written");
    assert!(
        !app.python
            .lines
            .iter()
            .any(|l| l.text.contains("geri alındı"))
    );
}

#[test]
fn requests_are_answered_on_the_open_drawing() {
    let mut app = app_with_drawing();
    running(&mut app);
    let summary = ask(&mut app, "summary", json!({})).expect("a summary");
    assert_eq!(summary["objects"], objects(&app));
    let made = polygon(&mut app);
    let uid = made["output"]["uid"].as_str().expect("its id").to_owned();
    assert_eq!(
        ask(&mut app, "measure", json!({"uid": uid})).expect("measures")["area"],
        250.0
    );
    let (code, _) = ask(&mut app, "undo", json!({})).expect_err("the desktop's own");
    assert_eq!(code, "unknown_method");

    let (mut empty, _) = App::boot(None);
    running(&mut empty);
    let (code, _) = ask(&mut empty, "summary", json!({})).expect_err("no drawing");
    assert_eq!(code, "no_document");
}

#[test]
fn output_is_kept_in_lines_and_the_oldest_go() {
    let mut app = app_with_drawing();
    said(
        &mut app,
        json!({"type": "out", "stream": "out", "text": "bir\niki\n"}),
    );
    said(
        &mut app,
        json!({"type": "out", "stream": "err", "text": "uyarı"}),
    );
    let tail: Vec<(Kind, &str)> = app
        .python
        .lines
        .iter()
        .map(|l| (l.kind, l.text.as_str()))
        .collect();
    assert_eq!(
        tail,
        [(Kind::Out, "bir"), (Kind::Out, "iki"), (Kind::Err, "uyarı")]
    );
    for i in 0..super::MOST_LINES {
        app.python.push(Kind::Out, format!("{i}"));
    }
    assert_eq!(app.python.lines.len(), super::MOST_LINES);
    assert_eq!(app.python.lines[0].text, "0");
}

#[test]
fn enter_runs_whole_code_and_waits_for_the_rest_of_a_block() {
    use super::view::whole;
    assert!(whole("1 + 1"));
    assert!(whole("doc.measure(u)"));
    assert!(!whole("f(1,"));
    assert!(!whole("for p in pts:"));
    assert!(!whole("for p in pts:\n    print(p)"));
    assert!(whole("for p in pts:\n    print(p)\n"));
    assert!(!whole("s = 'açık"));
    assert!(whole("s = 'a(' # (yorum"));
    assert!(!whole("x = 1 + \\"));
    assert!(!whole("   "));
}

#[test]
fn up_and_down_go_through_the_runs() {
    let mut app = app_with_drawing();
    app.python.remember("a = 1");
    app.python.remember("a + 1");
    app.python.input = iced::widget::text_editor::Content::with_text("yarım");
    let text = |app: &App| app.python.input.text().trim_end().to_owned();
    app.python_history(-1);
    assert_eq!(text(&app), "a + 1");
    app.python_history(-1);
    assert_eq!(text(&app), "a = 1");
    app.python_history(-1);
    assert_eq!(text(&app), "a = 1");
    app.python_history(1);
    assert_eq!(text(&app), "a + 1");
    app.python_history(1);
    assert_eq!(text(&app), "yarım", "back to what was typed");
}

/// The real Python: `python -m kentos.host` from KENTOS_PYTHON or the
/// checkout's `.run/py` (`pnpm py:test` makes it). Not run by default, as it
/// needs that Python: `cargo test -p kentos-desktop python::tests -- --ignored`.
#[test]
#[ignore = "needs a Python with the kentos package (pnpm py:test)"]
fn real_python_writes_the_open_drawing() {
    use iced::futures::StreamExt as _;
    let mut app = app_with_drawing();
    let before = objects(&app);
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a drawing");
    let task = app.python_run(
        format!(
            "made = cad.polygon.create(doc, layer_id={layer:?}, pts=[(423500, 4512300), (423520, 4512300), (423520, 4512312.5), (423500, 4512312.5)])\nprint(doc.measure(made.uid).area)"
        ),
        None,
    );
    let mut stream = iced_runtime::task::into_stream(task).expect("the console's messages");
    while app.python.running.is_some() {
        match iced::futures::executor::block_on(stream.next()) {
            Some(iced_runtime::Action::Output(message)) => {
                let _ = app.update(message);
            }
            Some(_) => {}
            None => break,
        }
    }
    let out: Vec<&str> = app
        .python
        .lines
        .iter()
        .filter(|l| l.kind == Kind::Out)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(out, ["250.0"], "{:?}", app.python.lines);
    assert!(
        app.python
            .ready
            .as_deref()
            .is_some_and(|r| r.starts_with("Python 3."))
    );
    assert_eq!(objects(&app), before + 1);
    let model = &mut app.document.as_mut().expect("open").model;
    assert_eq!(model.undo().as_deref(), Some(super::STEP));
    let _ = app.update(Message::Python(Event::Stop));
}

/// The real console's messages: the stream of its process's start.
type Heard = iced::futures::stream::BoxStream<'static, iced_runtime::Action<Message>>;

/// Runs `code` in the real console to its end, its messages driven here.
fn run_real(app: &mut App, code: &str, heard: &mut Option<Heard>) {
    use iced::futures::StreamExt as _;
    let task = app.python_run(code.to_owned(), None);
    // The first run's task carries the process's messages; a later one's only scrolls.
    if heard.is_none() {
        *heard = iced_runtime::task::into_stream(task);
    }
    let Some(stream) = heard.as_mut() else {
        return;
    };
    while app.python.running.is_some() {
        match iced::futures::executor::block_on(stream.next()) {
            Some(iced_runtime::Action::Output(message)) => {
                let _ = app.update(message);
            }
            Some(_) => {}
            None => break,
        }
    }
}

/// Pictures of the Python tab for the owner, with real runs (needs the
/// checkout's Python, `pnpm py:test`): dark and light, at 1440×900 and
/// 1100×650; `.run/shots/python-konsol-*`:
///
/// ```text
/// cargo test -p kentos-desktop python::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["bos", "calismalar", "calisiyor"] {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::BottomTab(crate::bottom::BottomTab::Python));
                let _ = app.update(Message::BottomResized(if height > 800.0 {
                    330.0
                } else {
                    250.0
                }));
                let mut heard = None;
                if name != "bos" {
                    run_real(&mut app, "len(doc), doc.settings.srid", &mut heard);
                    run_real(
                        &mut app,
                        "for r in doc.entities(kinds=\"polygon\"):\n    m = doc.measure(r.uid)\n    print(f\"{r.entity.label or r.uid[:8]}: {m.area:.2f} m², çevre {m.length:.2f} m\")\n",
                        &mut heard,
                    );
                    run_real(
                        &mut app,
                        "cad.polygon.create(doc, layer_id=\"yok\", pts=[(0, 0), (1, 0), (1, 1)])",
                        &mut heard,
                    );
                    app.python.input = iced::widget::text_editor::Content::with_text(
                        "cad.point.create(doc, layer_id=doc.active_layer, p=(486510.25, 4420180.5))",
                    );
                }
                if name == "calisiyor" {
                    running(&mut app);
                }
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("python-konsol-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
                if name == "calisiyor" {
                    app.python.running = None;
                }
                let _ = app.update(Message::Python(Event::Stop));
            }
        }
    }
}

/// How long a request takes from Python to the drawing and back: 1 000
/// points written one by one from the real console, its messages driven
/// here (the app's event loop not counted). `cargo test --release -p
/// kentos-desktop python::tests::round_trips -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, needs a Python with the kentos package"]
fn round_trips() {
    let mut app = app_with_drawing();
    let mut heard = None;
    run_real(&mut app, "0", &mut heard);
    let started = std::time::Instant::now();
    run_real(
        &mut app,
        "for i in range(1000):\n    cad.point.create(doc, layer_id=doc.active_layer, p=(486500 + i, 4420200))\n",
        &mut heard,
    );
    let took = started.elapsed();
    let errors: Vec<&str> = app
        .python
        .lines
        .iter()
        .filter(|l| l.kind == Kind::Error)
        .map(|l| l.text.as_str())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    println!(
        "1 000 cad.point.create: {:.0} ms, {:.0} µs each",
        took.as_secs_f64() * 1000.0,
        took.as_secs_f64() * 1000.0
    );
    let _ = app.update(Message::Python(Event::Stop));
}
