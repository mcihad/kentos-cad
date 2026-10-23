//! The drawing tools brought to their own semantic commands (docs/adr/0622 §9, TODOS.md `UX-14`), driven through the
//! widgets with real mouse events: the tool writes through its command (the journal's last call names it, with the
//! user's input, not a ready-made ring), a right click places nothing, and the Python echo carries the same call.
//! Pictures: `semantic_tool_screens` (ignored), `.run/shots/arac-komut-*`.

use iced::{Point, Size};
use kentos_ui::snapshot::{Input, Snapshot};
use serde_json::Value;

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, make_cad};

fn update(app: &mut App, message: Message) {
    let _ = app.update(message);
}

/// A tool started from its command, its tasks run against the view (focus and the like).
fn start(snapshot: &mut Snapshot, app: &mut App, tool: &'static str) {
    let task = app.update(Message::Run(tool));
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        use iced::futures::StreamExt as _;
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Widget(operation) = action {
                snapshot.operate(app.view(), operation);
            }
        }
    }
}

/// The drawing area's point at the fractions `fx`, `fy` of its width and height.
fn at(app: &App, fx: f32, fy: f32) -> Point {
    let b = app.viewport.bounds;
    Point::new(b.x + b.width * fx, b.y + b.height * fy)
}

/// The CAD sample drawing laid out, the Python echo on.
fn opened(snapshot: &mut Snapshot, theme: &str) -> App {
    let mut app = app_with_drawing();
    let _ = app.settings.choose(&[
        ("appearance.theme", Value::from(theme)),
        (crate::python::echo::SETTING, Value::Bool(true)),
    ]);
    app.apply_settings();
    make_cad(&mut app);
    update(&mut app, Message::DialogClosed);
    snapshot.settle(&mut app, App::view, &mut update);
    app
}

fn click(snapshot: &mut Snapshot, app: &mut App, p: Point) {
    snapshot.input(app, App::view, &mut update, Input::Move(p));
    snapshot.input(app, App::view, &mut update, Input::Click(p));
}

/// The journal's last call: its command and its input.
fn last_call(app: &App) -> (String, Value) {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let call = doc.journal().last().expect("a call");
    (
        call.command.clone().unwrap_or_default(),
        call.input.clone().unwrap_or(Value::Null),
    )
}

/// Daire dilimi: centre, start and end clicked; `cad.sector.create` with the centre, the radius and the two angles.
fn draw_sector(snapshot: &mut Snapshot, app: &mut App) {
    start(snapshot, app, "tool.sector");
    let (c, s, e) = (at(app, 0.55, 0.5), at(app, 0.7, 0.5), at(app, 0.55, 0.3));
    click(snapshot, app, c);
    click(snapshot, app, s);
    snapshot.input(app, App::view, &mut update, Input::Move(e));
    click(snapshot, app, e);
}

#[test]
fn the_sector_tool_writes_through_its_command() {
    let mut snapshot = crate::files_testing::offscreen(Size::new(1440.0, 900.0));
    let mut app = opened(&mut snapshot, "dark");
    let before = app.document.as_ref().expect("a drawing").model.len();
    draw_sector(&mut snapshot, &mut app);
    let (command, input) = last_call(&app);
    assert_eq!(command, "cad.sector.create");
    assert!(input["r"].as_f64().is_some_and(|r| r > 0.0), "{input}");
    let sweep = (input["a1"].as_f64().expect("a1") - input["a0"].as_f64().expect("a0"))
        .rem_euclid(std::f64::consts::TAU);
    assert!((sweep - std::f64::consts::FRAC_PI_2).abs() < 0.05, "a quarter: {input}");
    assert!(input.get("pts").is_none(), "the user's input, not a ring: {input}");
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.len(), before + 1);
    assert!(
        app.python.repl.entries().any(|e| e.text.contains("cad.sector.create")),
        "the echo carries the call"
    );
    // A right click ends nothing and places nothing.
    let p = at(&app, 0.3, 0.3);
    snapshot.input(&mut app, App::view, &mut update, Input::RightClick(p));
    assert_eq!(app.document.as_ref().expect("a drawing").model.len(), before + 1);
}

/// The tools writing through their commands, as the owner looks at them: the slice drawn, its call in the Python
/// console (`.run/shots/arac-komut-*`):
///
/// ```text
/// cargo test -p kentos-desktop --bin kentos-cad semantic_tool_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn semantic_tool_screens() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, suffix) in [
        ("dark", Size::new(1440.0, 900.0), ""),
        ("light", Size::new(1100.0, 650.0), "-acik-1100"),
    ] {
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut app = opened(&mut snapshot, theme);
        let _ = app.update(Message::WindowResized(size));
        snapshot.settle(&mut app, App::view, &mut update);
        draw_sector(&mut snapshot, &mut app);
        app.show_bottom(crate::bottom::BottomTab::Python);
        app.bottom_dragged(Some(260.0), std::time::Instant::now());
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("arac-komut-dilim{suffix}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}
