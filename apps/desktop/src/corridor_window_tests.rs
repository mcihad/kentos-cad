//! Koridor (plan/road.md Y7): the corridor's slope plan drawn on the drawing (`road.corridors.draw`), one undo step;
//! and its pictures.

use crate::app::{App, Message};

/// A road project with `road.corridors.draw`'s drawing: a surface, the main and side alignments, their profiles, two
/// templates, the superelevation schedules and two corridors.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.corridors.draw.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

fn draw(app: &mut App, corridor: &str, tick: f64) -> kentos_commands::CorridorsDrawn {
    let input = kentos_commands::CorridorsDraw {
        corridor: corridor.into(),
        layer_id: "0".into(),
        tick: Some(tick),
        height: None,
        expected_revision: None,
    };
    let mut cx = kentos_native_application::ExecutionContext::new(model(app));
    match kentos_native_application::road_corridors_draw::execute(&mut cx, input) {
        kentos_commands::CommandResult::Completed { output, .. } => output,
        other => panic!("drawn: {other:?}"),
    }
}

#[test]
fn the_slope_plan_is_drawn_in_one_step() {
    let mut app = app();
    let before = model(&mut app).entities().count();
    let drawn = draw(&mut app, "c-1", 5.0);
    assert!(drawn.lines > 10 && drawn.ticks > 50 && drawn.elevations > 20, "{drawn:?}");
    let doc = model(&mut app);
    assert_eq!(doc.entities().count(), before + (drawn.lines + drawn.ticks + drawn.elevations) as usize);
    assert_eq!(doc.undo().as_deref(), Some("Şevli plan"));
    assert_eq!(doc.entities().count(), before);
}

/// The slope plan over the drawing, light, wide; `.run/gui-evidence/yol-y7-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib corridor_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut app = app();
    let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from("light"))]);
    app.apply_settings();
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let mut shot = |app: &mut App, file: &str| {
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(format!("yol-y7-{file}-acik-genis.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    };
    draw(&mut app, "c-1", 5.0);
    draw(&mut app, "c-2", 5.0);
    let _ = app.update(Message::Run("view.zoomExtents"));
    shot(&mut app, "sevli-plan");
    // Closer: the curve, its ticks in cut and fill and the red elevations.
    app.zoom_to_bounds(&kentos_commands::Bounds { min_x: 200.0, min_y: 20.0, max_x: 360.0, max_y: 130.0 });
    shot(&mut app, "sevli-plan-yakin");
}
