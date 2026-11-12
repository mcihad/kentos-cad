//! Tipli katman, the Profil row and the shift table (map_work_panel.rs, plan/map-cad.md MAP-01, docs/adr/4880): a class
//! of a data profile given to the design from the panel (its layer added and joining the item, a second class kept
//! beside the first); the design approved, one line moved and the other removed: Değişenler draws both shapes as
//! approved and lists the moved line's shift, its numbers the query's. And the pictures.

use kentos_commands::WorkStatus as W;
use serde_json::json;

use crate::app::{App, Message, Panel};
use crate::map_work_panel::Event as P;
use crate::map_work_stamps_tests::{opened, send, tree, uid};

pub(crate) fn run(app: &mut App, command: &str, input: serde_json::Value) {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        command,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
}

fn give(app: &mut App, class: &str) {
    send(app, P::ProfileItem("is-4".into()));
    send(app, P::ProfileClass(class.into()));
    send(app, P::ProfileAdd);
}

#[test]
fn tipli_katman_gives_classes_to_an_item() {
    let mut app = opened();
    let layers_before = tree(&app).items[3].layers.len();
    give(&mut app, "harita-varliklari/kontrol-noktasi");
    let row = tree(&app).items[3].clone();
    let profile = row.profile.clone().expect("a profile");
    assert_eq!(profile.name, "harita-varliklari");
    assert!(profile.found);
    assert_eq!(profile.classes.iter().map(|c| c.class.as_str()).collect::<Vec<_>>(), ["kontrol-noktasi"]);
    assert_eq!(row.layers.len(), layers_before + 1, "the class's layer joined the item");
    assert_eq!(profile.free, ["tasarim"], "the design surface's layer stays free");
    // A second class of the same profile: both kept.
    give(&mut app, "harita-varliklari/rogar");
    let profile = tree(&app).items[3].profile.clone().expect("a profile");
    assert_eq!(profile.classes.iter().map(|c| c.class.as_str()).collect::<Vec<_>>(), ["kontrol-noktasi", "rogar"]);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|l| l.contains("sınıfı öğeye verildi")), "{said:?}");
    // Taken away: the layers stay.
    let layers = tree(&app).items[3].layers.clone();
    send(&mut app, P::ProfileRemove("is-4".into()));
    assert!(tree(&app).items[3].profile.is_none());
    assert_eq!(tree(&app).items[3].layers, layers);
}

/// The design approved, its line at slot 6 moved (`dx`, `dy`), its line at slot 7 removed.
pub(crate) fn approved_then_moved_and_removed(app: &mut App, dx: f64, dy: f64) -> String {
    send(app, P::Status("is-4".into(), W::Approved));
    let moved = uid(app, 6);
    let removed = uid(app, 7);
    run(app, "cad.entities.transform", json!({"uids": [moved], "transform": {"kind": "move", "dx": dx, "dy": dy}}));
    // Erased as the user erases it, so the scene follows.
    let slot = app.document.as_ref().and_then(|d| d.model.slot_of(kentos_domain::Uuid::parse_str(&removed).ok()?)).expect("the line");
    app.selection.set(vec![slot]);
    let _ = app.update(Message::Run("tool.erase"));
    moved
}

#[test]
fn degisenler_lists_the_shift_and_draws_the_removed_shape() {
    let mut app = opened();
    let moved = approved_then_moved_and_removed(&mut app, 0.3, 0.4);
    let c = tree(&app).items[3].changes.clone().expect("changes");
    let shift = c.objects.iter().find(|o| o.uid == moved).and_then(|o| o.shift.clone()).expect("a shift");
    assert!(shift.paired);
    send(&mut app, P::Changes("is-4".into()));
    let shown = app.map_work.approved.clone().expect("the approved shapes");
    assert_eq!(shown.objects, 2, "the moved line's and the removed line's");
    assert_eq!(app.map_work.shifts.len(), 1);
    let row = &app.map_work.shifts[0];
    let d = shift.largest.as_ref().expect("paired").d;
    assert_eq!(row[1], "2");
    assert_eq!(row[2], format!("{d:.3}"));
    assert_eq!(row[3], "1");
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|l| l.contains("Geometrisi değişen 1 ve silinen 1 nesnenin onaylanan şekli") && l.contains("En büyük kayma")),
        "{said:?}"
    );
}

/// `.run/gui-evidence/tipli-katman-4880/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_work_profile_tests::screens -- --ignored --nocapture
/// ```
/// One app's picture, rendered by its own snapshot (a snapshot keeps its first app's scene).
pub(crate) fn picture(app: &mut App, size: iced::Size, file: &std::path::Path) {
    let mut snapshot = kentos_ui::snapshot::Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(app, App::view, &mut update);
    snapshot.render(app.view(), &app.theme()).save(file).expect("writes");
    println!("{}", file.display());
}

/// The app in the theme and size, Harita işi in the dock with room, the campaigns folded.
pub(crate) fn staged(theme: &str, size: iced::Size) -> App {
    let mut app = opened();
    let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
    app.apply_settings();
    let _ = app.update(Message::WindowResized(size));
    app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapWork));
    app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.72, 0.28]));
    // A troubled branch opens by itself, a quiet one stays shut: the campaigns (out-of-date derivations) folded.
    send(&mut app, P::Fold("kind:campaign".into()));
    app
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tipli-katman-4880");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = staged(theme, size);
        give(&mut app, "harita-varliklari/kontrol-noktasi");
        // A point on the class's layer without its name: a finding.
        let layer = tree(&app).items[3].profile.clone().and_then(|p| p.classes[0].layer.clone()).expect("the layer");
        run(&mut app, "cad.entities.create", json!({"layerId": layer, "objects": [{"geometry": {"kind": "point", "p": {"x": 500004.0, "y": 4400003.0}}}]}));
        let _ = app.update(Message::Run("view.zoomExtents"));
        picture(&mut app, size, &out.join(format!("1-tipli-katman-{name}.png")));
        let mut app = staged(theme, size);
        let _ = approved_then_moved_and_removed(&mut app, 0.6, 0.8);
        let _ = app.update(Message::Run("view.zoomExtents"));
        send(&mut app, P::Changes("is-4".into()));
        picture(&mut app, size, &out.join(format!("2-kaymalar-{name}.png")));
    }
}
