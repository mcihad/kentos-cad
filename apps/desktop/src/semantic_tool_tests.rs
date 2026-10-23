//! The drawing tools brought to their own semantic commands (docs/adr/0622 §9, TODOS.md `UX-14`: Daire dilimi,
//! Revizyon bulutu), driven through the
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

/// The screen point of a world point in the drawing area.
fn screen(app: &App, w: kentos_render_wgpu::Vec2) -> Point {
    let b = app.viewport.bounds;
    let [x, y] = app.viewport.camera.world_to_screen(w);
    Point::new(b.x + x as f32, b.y + y as f32)
}

/// Revizyon bulutu, its rectangle: two corners clicked.
fn draw_cloud(snapshot: &mut Snapshot, app: &mut App) {
    start(snapshot, app, "tool.revcloud");
    let (a, b) = (at(app, 0.42, 0.62), at(app, 0.62, 0.78));
    click(snapshot, app, a);
    snapshot.input(app, App::view, &mut update, Input::Move(b));
    click(snapshot, app, b);
}

/// The middle of a visible, unlocked area's first edge, and the area's persistent id.
fn an_area_edge(app: &App) -> (Point, String) {
    use kentos_commands::Entity;
    let doc = &app.document.as_ref().expect("a drawing").model;
    let layers = doc.layers();
    let (uid, pts) = doc
        .entities_with_uids()
        .find_map(|(uid, e)| match e {
            Entity::Polygon(p)
                if layers.is_visible(&p.base.layer_id) && !layers.is_locked(&p.base.layer_id) =>
            {
                Some((uid, p.pts.clone()))
            }
            _ => None,
        })
        .expect("an area");
    let mid = kentos_render_wgpu::Vec2::new((pts[0].x + pts[1].x) / 2.0, (pts[0].y + pts[1].y) / 2.0);
    (screen(app, mid), uid.to_string())
}

#[test]
fn the_cloud_tool_writes_through_its_command() {
    let mut snapshot = crate::files_testing::offscreen(Size::new(1440.0, 900.0));
    let mut app = opened(&mut snapshot, "dark");
    let before = app.document.as_ref().expect("a drawing").model.len();
    draw_cloud(&mut snapshot, &mut app);
    let (command, input) = last_call(&app);
    assert_eq!(command, "cad.revcloud.create");
    assert_eq!(input["frame"]["kind"], "rectangle", "{input}");
    assert!(input["arc"].as_f64().is_some_and(|a| a > 0.0), "{input}");
    assert_eq!(app.document.as_ref().expect("a drawing").model.len(), before + 1);
    assert!(
        app.python.repl.entries().any(|e| e.text.contains("cad.revcloud.create")),
        "the echo carries the call"
    );
    // Nesne: the object's id, not its outline, goes to the command.
    let (edge, uid) = an_area_edge(&app);
    // Nesne (N) typed in the command line, as the prompt's option says.
    update(&mut app, Message::CommandInput("N".into()));
    update(&mut app, Message::CommandSubmitted);
    snapshot.settle(&mut app, App::view, &mut update);
    click(&mut snapshot, &mut app, edge);
    let (command, input) = last_call(&app);
    assert_eq!(command, "cad.revcloud.create");
    assert_eq!(input["frame"], serde_json::json!({ "kind": "object", "uid": uid }), "{input}");
    assert_eq!(app.document.as_ref().expect("a drawing").model.len(), before + 2);
}

/// A ring of clicks closed by Enter.
fn draw_ring(snapshot: &mut Snapshot, app: &mut App, tool: &'static str, ring: &[(f32, f32)]) {
    start(snapshot, app, tool);
    for &(fx, fy) in ring {
        let p = at(app, fx, fy);
        click(snapshot, app, p);
    }
    // Enter in the command line, empty: the tool's confirm.
    update(app, Message::CommandSubmitted);
    snapshot.settle(app, App::view, &mut update);
}

#[test]
fn the_parcel_and_the_closed_area_write_through_their_commands() {
    let mut snapshot = crate::files_testing::offscreen(Size::new(1440.0, 900.0));
    let mut app = opened(&mut snapshot, "dark");
    let ring = [(0.62, 0.62), (0.72, 0.62), (0.72, 0.75), (0.62, 0.75)];
    draw_ring(&mut snapshot, &mut app, "tool.parcel", &ring);
    let (command, input) = last_call(&app);
    assert_eq!(command, "cad.parcels.create");
    assert!(input.get("number").is_none(), "the command numbers it: {input}");
    assert_eq!(input["pts"].as_array().map(Vec::len), Some(4), "{input}");
    let ring = [(0.75, 0.3), (0.85, 0.3), (0.85, 0.4)];
    draw_ring(&mut snapshot, &mut app, "tool.polygon", &ring);
    let (command, _) = last_call(&app);
    assert_eq!(command, "cad.polygon.create");
}

/// The tools writing through their commands, as the owner looks at them: the slice and the cloud drawn, the cloud's call in the Python
/// console (`.run/shots/arac-komut-dilim-bulut*`):
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
        draw_cloud(&mut snapshot, &mut app);
        app.show_bottom(crate::bottom::BottomTab::Python);
        app.bottom_dragged(Some(260.0), std::time::Instant::now());
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("arac-komut-dilim-bulut{suffix}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Dördüncü köşe: three corners clicked; `cad.constructions.create` computes the fourth (docs/adr/0622 §9).
fn draw_fourth_corner(snapshot: &mut Snapshot, app: &mut App) {
    start(snapshot, app, "tool.fourthCorner");
    for (fx, fy) in [(0.30, 0.70), (0.40, 0.70), (0.44, 0.60)] {
        let p = at(app, fx, fy);
        click(snapshot, app, p);
    }
}

/// Two overlapping squares drawn as closed areas, then joined by Alan birleştir through `cad.areas.edit`.
fn union_two_squares(snapshot: &mut Snapshot, app: &mut App) {
    draw_ring(snapshot, app, "tool.polygon", &[(0.55, 0.30), (0.65, 0.30), (0.65, 0.42), (0.55, 0.42)]);
    draw_ring(snapshot, app, "tool.polygon", &[(0.60, 0.36), (0.70, 0.36), (0.70, 0.48), (0.60, 0.48)]);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let mut slots: Vec<u32> = doc.entities_with_uids().map(|(_, e)| e.base().id).collect();
    slots.sort_unstable();
    let last: Vec<kentos_domain::Slot> = slots.iter().rev().take(2).map(|&s| kentos_domain::Slot(s)).collect();
    app.selection.set(last);
    start(snapshot, app, "tool.areaUnion");
    snapshot.settle(app, App::view, &mut update);
}

#[test]
fn the_construction_and_area_tools_write_through_their_commands() {
    let mut snapshot = crate::files_testing::offscreen(Size::new(1440.0, 900.0));
    let mut app = opened(&mut snapshot, "dark");
    let before = app.document.as_ref().expect("a drawing").model.len();
    draw_fourth_corner(&mut snapshot, &mut app);
    let (command, input) = last_call(&app);
    assert_eq!(command, "cad.constructions.create");
    assert_eq!(input["construction"]["kind"], "fourthCorner", "{input}");
    assert!(input["construction"].get("d").is_none(), "the user's corners, not the answer: {input}");
    assert_eq!(app.document.as_ref().expect("a drawing").model.len(), before + 1);
    union_two_squares(&mut snapshot, &mut app);
    let (command, input) = last_call(&app);
    assert_eq!(command, "cad.areas.edit");
    assert_eq!(input["work"]["kind"], "union", "{input}");
    assert_eq!(input["work"]["ids"].as_array().map(Vec::len), Some(2), "{input}");
    // Two areas in, one out: the fourth corner's point and the union.
    assert_eq!(app.document.as_ref().expect("a drawing").model.len(), before + 2);
    assert!(
        app.python.repl.entries().any(|e| e.text.contains("cad.areas.edit")),
        "the echo carries the call"
    );
}

/// The construction and the area command as the owner looks at them (`.run/shots/arac-komut-yapim-alan*`):
///
/// ```text
/// cargo test -p kentos-desktop --bin kentos-cad construction_area_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn construction_area_screens() {
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
        draw_fourth_corner(&mut snapshot, &mut app);
        union_two_squares(&mut snapshot, &mut app);
        app.show_bottom(crate::bottom::BottomTab::Python);
        app.bottom_dragged(Some(260.0), std::time::Instant::now());
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("arac-komut-yapim-alan{suffix}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}
