//! The approved shape of an object whose geometry changed, over the drawing (approved_layer.rs, map_work_panel.rs,
//! plan/map-cad.md MAP-01, docs/adr/4480): the design approved from the panel (its values kept), then one of its lines
//! moved and the other given a height. Değişenler selects both; the moved line's shape as approved, read from the
//! query's answer, is drawn dashed beside it (the other's geometry did not change: not drawn), the status bar says so;
//! another selection takes the shapes away. And the pictures.

use kentos_commands::WorkStatus as W;
use serde_json::json;

use crate::app::{App, Message, Panel};
use crate::map_work_panel::Event as P;
use crate::map_work_stamps_tests::{change_line, opened, send, tree, uid};

/// The design (is-4) approved; its line at slot 6 moved `dx` east and `dy` north, its line at slot 7 given a height. The
/// moved line's id and its ends as approved.
fn approved_then_moved(app: &mut App, dx: f64, dy: f64) -> (String, [[f64; 2]; 2]) {
    send(app, P::Status("is-4".into(), W::Approved));
    let moved = uid(app, 6);
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Line(l)) = doc.model.get(kentos_domain::Slot(6)) else { panic!("a line") };
    let then = [[l.a.x, l.a.y], [l.b.x, l.b.y]];
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.entities.transform",
        kentos_commands::CallOp::Execute,
        json!({"uids": [moved], "transform": {"kind": "move", "dx": dx, "dy": dy}}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    change_line(app, 7, "1060");
    (moved, then)
}

#[test]
fn degisenler_draws_the_moved_line_s_approved_shape() {
    let mut app = opened();
    let (moved, then) = approved_then_moved(&mut app, 0.25, -0.1);
    let c = tree(&app).items[3].changes.clone().expect("changes");
    assert_eq!(c.changed, 2);
    let with_shape: Vec<&str> = c.objects.iter().filter(|o| o.approved.is_some()).map(|o| o.uid.as_str()).collect();
    assert_eq!(with_shape, [moved.as_str()], "only the moved line's shape");
    send(&mut app, P::Changes("is-4".into()));
    let shown = app.map_work.approved.clone().expect("the approved shapes");
    assert_eq!(shown.objects, 1);
    assert_eq!(shown.lines, vec![vec![then[0], then[1]]], "the line as approved");
    assert_eq!(shown.version, app.selection.version(), "with Değişenler's selection");
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|l| l.ends_with("Geometrisi değişen 1 nesnenin onaylanan şekli kesik çizgiyle gösteriliyor.")), "{said:?}");
    // Another selection: the shapes go with Değişenler's.
    app.selection.set(vec![kentos_domain::Slot(6)]);
    assert_ne!(app.map_work.approved.as_ref().map(|a| a.version), Some(app.selection.version()));
}

/// `.run/gui-evidence/onayli-sekil-4480/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_work_geometry_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/onayli-sekil-4480");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapWork));
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.68, 0.32]));
        send(&mut app, P::Fold("kind:campaign".into()));
        // Moved far enough to be seen apart at the drawing's scale, the whole drawing in view.
        let _ = approved_then_moved(&mut app, 0.6, -0.8);
        let _ = app.update(Message::Run("view.zoomExtents"));
        send(&mut app, P::Changes("is-4".into()));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-onayli-sekil-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
