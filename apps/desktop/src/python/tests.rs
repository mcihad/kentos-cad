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

/// The code box with `code` and the cursor at its end.
fn typed(app: &mut App, code: &str) {
    use iced::widget::text_editor::{Action, Motion};
    app.python.input = iced::widget::text_editor::Content::with_text(code);
    app.python.input.perform(Action::Move(Motion::DocumentEnd));
}

#[test]
fn an_answered_list_is_shown_narrowed_and_taken_and_a_late_answer_is_dropped() {
    let mut app = app_with_drawing();
    typed(&mut app, "x = cad.poly");
    app.python.asked_completion = 2;
    let items = json!([
        {"text": "polygon", "kind": "module", "detail": ""},
        {"text": "polyline", "kind": "module", "detail": ""},
        {"text": "point", "kind": "module", "detail": ""}
    ]);
    said(
        &mut app,
        json!({"type": "completions", "id": 1, "start": 8, "items": items}),
    );
    assert!(
        app.python.completion.is_none(),
        "an older question's answer"
    );
    said(
        &mut app,
        json!({"type": "completions", "id": 2, "start": 8, "items": items}),
    );
    let shown = |app: &App| {
        app.python
            .completion
            .as_ref()
            .map(|l| l.entries().map(|c| c.text.clone()).collect::<Vec<_>>())
    };
    assert_eq!(
        shown(&app),
        Some(vec!["polygon".to_owned(), "polyline".to_owned()]),
        "narrowed to what is typed"
    );
    let _ = app.update(Message::Python(Event::Step(1)));
    let _ = app.update(Message::Python(Event::Accept(None)));
    assert_eq!(app.python.input.text().trim_end(), "x = cad.polyline");
    assert!(app.python.completion.is_none());
    // Tab where nothing is to complete: four spaces.
    typed(&mut app, "");
    let _ = app.update(Message::Python(Event::Indent));
    assert_eq!(app.python.input.text().trim_end_matches('\n'), "    ");
}

#[test]
fn a_signature_answer_is_shown_and_running_code_hides_it() {
    let mut app = app_with_drawing();
    app.python.asked_signature = 3;
    said(
        &mut app,
        json!({"type": "signature", "id": 3, "label": "measure(uid: str) -> Measure", "doc": "Ölç.", "argument": "uid: str"}),
    );
    let s = app.python.signature.clone().expect("the signature");
    assert_eq!(s.label, "measure(uid: str) -> Measure");
    assert_eq!(s.argument.as_deref(), Some("uid: str"));
    said(&mut app, json!({"type": "signature", "id": 3}));
    assert!(app.python.signature.is_none(), "outside a call");
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

/// Sends `event` to the real console and drives its messages until `done` holds.
fn ask_real(app: &mut App, event: Event, heard: &mut Option<Heard>, done: impl Fn(&App) -> bool) {
    use iced::futures::StreamExt as _;
    let task = app.update(Message::Python(event));
    if heard.is_none() {
        *heard = iced_runtime::task::into_stream(task);
    }
    let Some(stream) = heard.as_mut() else {
        return;
    };
    while !done(app) {
        match iced::futures::executor::block_on(stream.next()) {
            Some(iced_runtime::Action::Output(message)) => {
                let _ = app.update(message);
            }
            Some(_) => {}
            None => break,
        }
    }
}

/// The real console completes a name and shows the signature of the call
/// being written. Needs the checkout's Python (`pnpm py:test`).
#[test]
#[ignore = "needs a Python with the kentos package (pnpm py:test)"]
fn real_python_completes_and_shows_signatures() {
    let mut app = app_with_drawing();
    let mut heard = None;
    typed(&mut app, "cad.polygon.cr");
    ask_real(&mut app, Event::Complete, &mut heard, |app| {
        app.python.completion.is_some()
    });
    let _ = app.update(Message::Python(Event::Accept(None)));
    assert_eq!(app.python.input.text().trim_end(), "cad.polygon.create");
    typed(&mut app, "cad.polygon.create(doc, layer_id=");
    ask_real(
        &mut app,
        Event::Edit(iced::widget::text_editor::Action::Move(
            iced::widget::text_editor::Motion::DocumentEnd,
        )),
        &mut heard,
        |app| app.python.signature.is_some(),
    );
    let s = app.python.signature.clone().expect("the signature");
    assert!(
        s.label
            .starts_with("create(doc: Document, /, *, layer_id: str"),
        "{}",
        s.label
    );
    assert_eq!(s.argument.as_deref(), Some("layer_id: str"));
    let _ = app.update(Message::Python(Event::Stop));
}

/// The script the pictures show: parcels listed with their areas, a new
/// layer's worth of points at their centres.
const SCRIPT: &str = r#""""Parsellerin alanları ve ağırlık merkezlerine nokta."""
from kentos.cad import Vec2

toplam = 0.0
for r in doc.entities(kinds="polygon"):
    m = doc.measure(r.uid)
    toplam += m.area or 0
    b = m.bounds
    orta = Vec2((b.min_x + b.max_x) / 2, (b.min_y + b.max_y) / 2)
    cad.point.create(doc, layer_id=r.entity.layer_id, p=orta)
    print(f"{r.entity.label or r.uid[:8]}: {m.area:.2f} m²")

print(f"Toplam: {toplam:.2f} m²")  # 1 dönüm = 1000 m²
"#;

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
            for name in [
                "bos",
                "calismalar",
                "calisiyor",
                "tamamlama",
                "imza",
                "betik",
                "ajan",
                "serit",
            ] {
                if std::env::var("KENTOS_SHOTS_ONLY")
                    .is_ok_and(|only| !only.split(',').any(|o| o == name))
                {
                    continue;
                }
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                if name == "serit" {
                    // Opened from Araçlar › Komut › Python konsolu.
                    app.tab = "tools";
                    let _ = app.update(Message::Run(crate::catalog::PYTHON_CONSOLE));
                } else {
                    let _ = app.update(Message::BottomTab(crate::bottom::BottomTab::Python));
                }
                let _ = app.update(Message::BottomResized(if height > 800.0 {
                    330.0
                } else {
                    250.0
                }));
                let mut heard = None;
                if !matches!(name, "bos" | "serit") {
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
                if name == "betik" {
                    app.python.script.content =
                        iced::widget::text_editor::Content::with_text(SCRIPT);
                    app.python.script.dirty = true;
                    app.python.lines.clear();
                    let _ = app.update(Message::Python(Event::Mode(super::Mode::Script)));
                    ask_real(
                        &mut app,
                        Event::Script(super::script::Event::RunAll),
                        &mut heard,
                        |app| app.python.running.is_none() && app.python.lines.len() > 2,
                    );
                }
                #[cfg(unix)]
                let _link_dir = if name == "ajan" {
                    let dir = crate::files_testing::scratch("ajan-resim");
                    let path = dir.join("masaustu.sock");
                    let (open, _task) = super::link::Link::open(&path, |heard| {
                        Message::Python(Event::Agent(heard))
                    })
                    .expect("the link");
                    app.python.lines.clear();
                    app.python.push(
                        Kind::Note,
                        "Ajan bağlantısı açık: /run/user/1000/kentos-cad/masaustu.sock. Bu kullanıcının programları (MCP'de desktop.attach) açık çizimi okuyup komutlarla yazabilir; her yazma bir geri alma adımıdır.",
                    );
                    app.python.link = Some(open);
                    let (answerer, _answers) = super::link::Answerer::testing();
                    let _ = app.update(Message::Python(Event::Agent(
                        super::link::Heard::Connected(1, answerer),
                    )));
                    let layer = app
                        .document
                        .as_ref()
                        .map(|d| d.model.layers().active().to_owned())
                        .expect("a layer");
                    let input: Value = serde_json::from_str(&format!(
                        r#"{{"layerId": "{layer}", "pts": {SQUARE}}}"#
                    ))
                    .expect("input");
                    let _ = app.update(Message::Python(Event::Agent(super::link::Heard::Request(
                        1,
                        json!({ "id": 1, "method": "run", "params": { "command": "cad.polygon.create", "op": "execute", "input": input } }),
                    ))));
                    Some(dir)
                } else {
                    None
                };
                if name == "tamamlama" {
                    typed(&mut app, "cad.poly");
                    ask_real(&mut app, Event::Complete, &mut heard, |app| {
                        app.python.completion.is_some()
                    });
                }
                if name == "imza" {
                    typed(&mut app, "cad.polygon.create(doc, layer_id=");
                    ask_real(
                        &mut app,
                        Event::Edit(iced::widget::text_editor::Action::Move(
                            iced::widget::text_editor::Motion::DocumentEnd,
                        )),
                        &mut heard,
                        |app| app.python.signature.is_some(),
                    );
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

fn script_text(app: &App) -> String {
    app.python.script.content.text()
}

fn script_event(app: &mut App, event: super::script::Event) {
    let _ = app.update(Message::Python(Event::Script(event)));
}

#[test]
fn the_script_is_kept_as_a_draft_and_comes_back() {
    use iced::widget::text_editor::{Action, Edit};
    let dir = crate::files_testing::scratch("python-betik");
    let mut app = app_with_drawing();
    app.python.script = super::script::Script::load(&dir);
    let _ = app.update(Message::Python(Event::Mode(super::Mode::Script)));
    script_event(
        &mut app,
        super::script::Event::Edit(Action::Edit(Edit::Paste(std::sync::Arc::new(
            "print('taslak')".into(),
        )))),
    );
    assert!(app.python.script.dirty);
    let again = super::script::Script::load(&dir);
    assert_eq!(again.content.text().trim_end(), "print('taslak')");
    assert!(again.dirty, "unsaved, as it was left");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn yeni_over_unsaved_work_asks_first() {
    use iced::widget::text_editor::{Action, Edit};
    let mut app = app_with_drawing();
    script_event(
        &mut app,
        super::script::Event::Edit(Action::Edit(Edit::Paste(std::sync::Arc::new(
            "a = 1".into(),
        )))),
    );
    script_event(&mut app, super::script::Event::New);
    assert_eq!(app.python.script.asking, Some(super::script::Pending::New));
    assert_eq!(script_text(&app).trim_end(), "a = 1", "nothing lost yet");
    script_event(&mut app, super::script::Event::Cancel);
    assert!(app.python.script.asking.is_none());
    assert_eq!(script_text(&app).trim_end(), "a = 1");
    script_event(&mut app, super::script::Event::New);
    script_event(&mut app, super::script::Event::DropAndGo);
    assert_eq!(script_text(&app).trim_end(), "", "a new script");
    assert!(!app.python.script.dirty);
}

#[test]
fn saving_writes_the_file_and_a_new_line_keeps_its_indentation() {
    use iced::widget::text_editor::{Action, Edit, Motion};
    let dir = crate::files_testing::scratch("python-kaydet");
    let path = dir.join("parseller.py");
    let mut app = app_with_drawing();
    script_event(
        &mut app,
        super::script::Event::Edit(Action::Edit(Edit::Paste(std::sync::Arc::new(
            "for p in x:".into(),
        )))),
    );
    script_event(
        &mut app,
        super::script::Event::Edit(Action::Move(Motion::DocumentEnd)),
    );
    script_event(&mut app, super::script::Event::Newline);
    assert_eq!(script_text(&app), "for p in x:\n    ");
    script_event(&mut app, super::script::Event::SavedAs(Some(path.clone())));
    assert_eq!(
        std::fs::read_to_string(&path).expect("written"),
        "for p in x:\n    "
    );
    assert!(!app.python.script.dirty);
    assert_eq!(app.python.script.run_name(), path.display().to_string());
    let _ = std::fs::remove_dir_all(dir);
}

/// A script run whole in the real console: its output beside it, and on an
/// error the cursor on the line it stopped at. Needs `pnpm py:test`'s Python.
#[test]
#[ignore = "needs a Python with the kentos package (pnpm py:test)"]
fn real_python_runs_the_script_and_shows_where_it_failed() {
    use iced::widget::text_editor::Content;
    let mut app = app_with_drawing();
    app.python.script.content =
        Content::with_text("n = len(doc)\nprint(n)\nraise ValueError('dur')\n");
    let mut heard = None;
    ask_real(
        &mut app,
        Event::Script(super::script::Event::RunAll),
        &mut heard,
        |app| {
            app.python.running.is_none() && app.python.lines.iter().any(|l| l.kind == Kind::Error)
        },
    );
    let out: Vec<&str> = app
        .python
        .lines
        .iter()
        .filter(|l| l.kind == Kind::Out)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(out, ["13"]);
    assert_eq!(
        app.python.script.content.cursor().position.line,
        2,
        "the failed line"
    );
    let _ = app.update(Message::Python(Event::Stop));
}

/// An agent's request answered on the open drawing (agents.rs), the
/// answers read from a stand-in connection.
fn ask_agent(
    app: &mut App,
    answers: &std::sync::mpsc::Receiver<String>,
    method: &str,
    params: Value,
) -> Value {
    let _ = app.update(Message::Python(Event::Agent(super::link::Heard::Request(
        7,
        json!({ "id": 1, "method": method, "params": params }),
    ))));
    serde_json::from_str(&answers.try_recv().expect("an answer")).expect("JSON")
}

#[cfg(unix)]
#[test]
fn an_agent_reads_and_writes_the_open_drawing_each_write_its_own_step() {
    let mut app = app_with_drawing();
    let dir = crate::files_testing::scratch("ajan");
    let (open, _task) = super::link::Link::open(&dir.join("m.sock"), |heard| {
        Message::Python(Event::Agent(heard))
    })
    .expect("the link");
    app.python.link = Some(open);
    let (answerer, answers) = super::link::Answerer::testing();
    let _ = app.update(Message::Python(Event::Agent(
        super::link::Heard::Connected(7, answerer),
    )));
    let before = objects(&app);

    let hello = ask_agent(&mut app, &answers, "hello", json!({}));
    assert_eq!(hello["ok"], true);
    assert_eq!(hello["result"]["objects"], before);
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a layer");
    let input: Value =
        serde_json::from_str(&format!(r#"{{"layerId": "{layer}", "pts": {SQUARE}}}"#))
            .expect("input");
    let written = ask_agent(
        &mut app,
        &answers,
        "run",
        json!({ "command": "cad.polygon.create", "op": "execute", "input": input }),
    );
    assert_eq!(written["result"]["status"], "completed");
    assert_eq!(objects(&app), before + 1);
    assert!(
        app.log
            .lines()
            .any(|l| l.text == "Ajan: cad.polygon.create"),
        "the command history says an agent wrote"
    );
    let model = &mut app.document.as_mut().expect("open").model;
    assert!(model.undo().is_some(), "its own undo step");
    assert_eq!(model.len(), before);

    // While console code runs, agents wait.
    running(&mut app);
    let busy = ask_agent(&mut app, &answers, "summary", json!({}));
    assert_eq!(
        (busy["ok"].clone(), busy["code"].clone()),
        (json!(false), json!("busy"))
    );
    app.python.running = None;
    let unknown = ask_agent(&mut app, &answers, "undo", json!({}));
    assert_eq!(unknown["code"], "unknown_method", "undo is the user's");

    // Closed: every request is refused.
    let _ = app.update(Message::Python(Event::Link));
    assert!(app.python.link.is_none());
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn the_link_is_this_users_alone_and_goes_with_its_socket() {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    let dir = crate::files_testing::scratch("ajan-soket");
    let path = dir.join("m.sock");
    let mut app = app_with_drawing();
    let (open, task) = super::link::Link::open(&path, |heard| Message::Python(Event::Agent(heard)))
        .expect("the link");
    app.python.link = Some(open);
    let mode = std::fs::metadata(&path)
        .expect("the socket")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    assert!(
        super::link::Link::open(&path, |heard| Message::Python(Event::Agent(heard))).is_err(),
        "a second window does not take a link that is in use"
    );

    let mut client = std::os::unix::net::UnixStream::connect(&path).expect("connects");
    writeln!(client, r#"{{"id": 5, "method": "hello", "params": {{}}}}"#).expect("asked");
    let mut stream = iced_runtime::task::into_stream(task).expect("the link's messages");
    let mut reader = BufReader::new(client.try_clone().expect("its reader"));
    // Until the request is answered (the second window's probe came and went first).
    loop {
        use iced::futures::StreamExt as _;
        match iced::futures::executor::block_on(stream.next()) {
            Some(iced_runtime::Action::Output(message)) => {
                let request = matches!(
                    message,
                    Message::Python(Event::Agent(super::link::Heard::Request(..)))
                );
                let _ = app.update(message);
                if request {
                    break;
                }
            }
            Some(_) => {}
            None => panic!("the link ended"),
        }
    }
    let mut line = String::new();
    reader.read_line(&mut line).expect("an answer");
    let answer: Value = serde_json::from_str(&line).expect("JSON");
    assert_eq!(
        (answer["id"].clone(), answer["ok"].clone()),
        (json!(5), json!(true))
    );
    let _ = app.update(Message::Python(Event::Link));
    assert!(!path.exists(), "the socket goes with the link");
    let _ = std::fs::remove_dir_all(dir);
}

/// A real agent: `kentos-mcp` (built beside this test) attaches to the
/// desktop through its link and draws on the open drawing over MCP, while
/// the app answers. `cargo build -p kentos-mcp` first.
#[cfg(unix)]
#[test]
#[ignore = "needs the kentos-mcp program (cargo build -p kentos-mcp)"]
fn a_real_agent_draws_on_the_open_drawing_through_mcp() {
    use iced::futures::StreamExt as _;
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    let mcp = std::env::current_exe()
        .expect("this test")
        .ancestors()
        .skip(1)
        .take(3)
        .map(|d| d.join("kentos-mcp"))
        .find(|p| p.exists())
        .expect("target/<profile>/kentos-mcp: cargo build -p kentos-mcp");
    let dir = crate::files_testing::scratch("ajan-mcp");
    let path = dir.join("m.sock");
    let mut app = app_with_drawing();
    let (open, task) = super::link::Link::open(&path, |heard| Message::Python(Event::Agent(heard)))
        .expect("the link");
    app.python.link = Some(open);
    let before = objects(&app);
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a layer");
    let agent = std::thread::spawn(move || {
        let mut child = Command::new(mcp)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("kentos-mcp");
        let mut stdin = child.stdin.take().expect("its input");
        let mut lines = BufReader::new(child.stdout.take().expect("its output")).lines();
        let meta = json!({
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientCapabilities": {}
        });
        let mut call = |id: u64, name: &str, arguments: Value| -> Value {
            let request = json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": { "_meta": meta, "name": name, "arguments": arguments } });
            writeln!(stdin, "{request}").expect("asked");
            stdin.flush().expect("sent");
            let answer: Value =
                serde_json::from_str(&lines.next().expect("a line").expect("read")).expect("JSON");
            answer["result"]["structuredContent"].clone()
        };
        let attached = call(1, "desktop.attach", json!({ "path": path }));
        let handle = attached["drawing"].as_str().expect("a handle").to_owned();
        let pts: Value = serde_json::from_str(SQUARE).expect("points");
        let written = call(
            2,
            "cad.polygon.create",
            json!({ "drawing": handle, "layerId": layer, "pts": pts }),
        );
        let uid = written["output"]["uid"]
            .as_str()
            .expect("its id")
            .to_owned();
        let measured = call(
            3,
            "drawing.measure",
            json!({ "drawing": handle, "uid": uid }),
        );
        let _ = child.kill();
        let _ = child.wait();
        (attached, written, measured)
    });
    // The app answers until the agent's connection ends.
    let mut stream = iced_runtime::task::into_stream(task).expect("the link's messages");
    loop {
        match iced::futures::executor::block_on(stream.next()) {
            Some(iced_runtime::Action::Output(message)) => {
                let closed = matches!(
                    message,
                    Message::Python(Event::Agent(super::link::Heard::Closed(_)))
                );
                let _ = app.update(message);
                if closed {
                    break;
                }
            }
            Some(_) => {}
            None => break,
        }
    }
    let (attached, written, measured) = agent.join().expect("the agent");
    assert_eq!(attached["desktop"]["objects"], before);
    assert_eq!(written["status"], "completed", "{written}");
    assert_eq!(measured["area"], 250.0);
    assert_eq!(
        objects(&app),
        before + 1,
        "the drawing on the screen changed"
    );
    assert!(
        app.log
            .lines()
            .any(|l| l.text == "Ajan: cad.polygon.create")
    );
    let _ = app.update(Message::Python(Event::Link));
    let _ = std::fs::remove_dir_all(dir);
}
