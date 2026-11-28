//! Altlık modeller (docs/adr/8101; plan/ifc-uyum.md IFC-09, B18) worked by hand on the two-storey house of
//! `dated_sizes_tests` (10 × 5 m outside, 25 cm walls, an interior wall on x = 8, each storey 3.80 m), put on TUREF /
//! TM36 with ±0.00 at 1285.40 m and written as IFC by KentOS:
//!
//! - **Its own model back:** placed by its map conversion onto the same system, it lands where it was drawn: every
//!   object with a shape has a body, the box is 0–10 × 0–5 × 0–7.60 m, and the plan at Zemin kat's cut runs along the
//!   walls' faces (the outer ones on x = 0 and x = 10, the interior one's on 7.95 and 8.05).
//! - **Another system:** buildingSMART's IFC 4.3 architecture sample (EPSG:32760) is not placed: the window says why;
//!   placed at the origin when asked, it is read with the objects shown and those without a body listed. Its IFC2X3
//!   sample has no map conversion: the same.
//! - A click on an underlay's line in the plan, and on its body in 3B görünüm, shows the object; a file changed on disk
//!   is said until Yeniden oku takes it.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib underlays_tests::screens -- --ignored --nocapture
//! ```

use crate::arch::ArchState;
use crate::arch_ext::*;
use std::path::{Path, PathBuf};

use kentos_commands::UnderlayPlacement;
use serde_json::{Value, json};

use crate::app::{App, Message, Picker};
use crate::files_testing::{drive, scratch};
use crate::underlays::{Loaded, State};
use crate::underlays_window::Event;

const BSI43: &str = "../../fixtures/ifc/v1/disaridan/bsi-ifc4x3-building-architecture.ifc";
const BSI2X3: &str = "../../fixtures/ifc/v1/disaridan/bsi-ifc2x3-building-architecture.ifc";

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(rel)
        .canonicalize()
        .expect("the fixture")
}

fn send(app: &mut App, e: Event) {
    let task = app.update(Message::from(crate::arch::Event::Underlays(e)));
    drive(app, task);
}

fn run(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// The house on TUREF / TM36, ±0.00 at 1285.40 m, written as IFC into a folder of its own.
fn house_on_the_map() -> (App, PathBuf) {
    let (mut app, _, _) = crate::dated_sizes_tests::house();
    // The architecture settings as they are, ±0.00's height given (the command writes them whole).
    let mut arch = serde_json::to_value(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .settings()
            .arch
            .clone(),
    )
    .expect("the settings");
    arch["site"] = json!({"datum": 1285.4});
    run(
        &mut app,
        "cad.settings.set",
        json!({"srid": 5256, "arch": arch}),
    );
    let path = scratch("altlik").join("bina.ifc");
    run(
        &mut app,
        "cad.arch.ifc",
        json!({"path": path.to_string_lossy(), "overwrite": true}),
    );
    (app, path)
}

/// Altlık modeller open and `file` added through Ekle…, its read waited for.
fn added(app: &mut App, file: &Path) {
    if app.modules.arch().underlays_window.is_none() {
        let _ = app.update(Message::Run("arch.underlays"));
    }
    app.picker = Picker::File(file.to_path_buf());
    send(app, Event::Add);
    settle(app);
}

/// The reads the drawing wants, waited for (they run off the window's thread).
fn settle(app: &mut App) {
    let task = app.follow_underlays();
    drive(app, task);
}

/// The underlay `i` as read.
fn loaded(app: &App, i: usize) -> std::sync::Arc<Loaded> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let id = &doc.underlays()[i].id;
    match app.modules.arch().underlays.states.get(id).map(|s| &s.1) {
        Some(State::Ready(l)) => l.clone(),
        other => panic!("not read: {other:?}"),
    }
}

/// How many products the file gives a shape (a representation), from its text.
fn shaped(path: &Path) -> usize {
    let text = std::fs::read_to_string(path).expect("reads");
    let classes = [
        "IFCWALL(",
        "IFCSLAB(",
        "IFCCOLUMN(",
        "IFCBEAM(",
        "IFCROOF(",
        "IFCRAILING(",
        "IFCSTAIR(",
        "IFCCOVERING(",
        "IFCDOOR(",
        "IFCWINDOW(",
        "IFCMEMBER(",
        "IFCSPACE(",
        "IFCSPATIALZONE(",
    ];
    text.lines()
        .filter_map(|l| l.split_once('=').map(|(_, r)| r.trim()))
        .filter(|r| classes.iter().any(|c| r.starts_with(c)))
        // The seventh argument is the representation: `$` when it has none.
        .filter(|r| r.split(',').nth(6).is_some_and(|a| a.trim() != "$"))
        .count()
}

#[test]
fn the_house_s_own_model_comes_back_where_it_was_drawn() {
    let (mut app, path) = house_on_the_map();
    added(&mut app, &path);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let u = doc.underlays()[0].clone();
    assert_eq!(
        (u.placement, u.schema.as_str(), u.shown),
        (UnderlayPlacement::Map, "IFC4X3_ADD2", true)
    );
    let bytes = std::fs::read(&path).expect("the file");
    assert_eq!(u.sha256, kentos_commands::encoding::sha256_hex(&bytes));
    let l = loaded(&app, 0);
    assert!(
        l.not_shown.is_empty(),
        "every shape has a body: {:?}",
        l.not_shown
    );
    assert_eq!(
        l.bodies.len(),
        shaped(&path),
        "one body for each product with a shape"
    );
    let b = l.bounds.expect("a box");
    let want = [0.0, 0.0, 0.0, 10.0, 5.0, 7.6];
    assert!(
        b.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-3),
        "the box {b:?}"
    );
    // Zemin kat's plan: the cut along the walls' faces, the interior wall's two on x = 7.95 and 8.05.
    let (cut, floor) = app.underlay_level();
    assert_eq!(floor, 0.0);
    let lines = crate::underlays::plan_of(&l, cut, floor);
    let cuts: Vec<_> = lines.iter().filter(|l| l.cut).collect();
    let xs = |x: f64| {
        cuts.iter()
            .any(|l| (l.a[0] - x).abs() < 1e-3 && (l.b[0] - x).abs() < 1e-3)
    };
    for x in [0.0, 0.25, 7.95, 8.05, 9.75, 10.0] {
        assert!(xs(x), "a cut line on x = {x}");
    }
    let (lo, hi) = cuts
        .iter()
        .fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), l| {
            (
                [lo[0].min(l.a[0]).min(l.b[0]), lo[1].min(l.a[1]).min(l.b[1])],
                [hi[0].max(l.a[0]).max(l.b[0]), hi[1].max(l.a[1]).max(l.b[1])],
            )
        });
    assert!(
        (lo[0]).abs() < 1e-3
            && lo[1].abs() < 1e-3
            && (hi[0] - 10.0).abs() < 1e-3
            && (hi[1] - 5.0).abs() < 1e-3
    );
    println!(
        "kendi modeli: {} gövde, okuma {:.3} s, bellek {} B, plan {} kesit + {} alt çizgi",
        l.bodies.len(),
        l.took.as_secs_f64(),
        l.memory(),
        cuts.len(),
        lines.len() - cuts.len()
    );
    // One undo step.
    let _ = app.update(Message::Run("edit.undo"));
    assert!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .underlays()
            .is_empty()
    );
}

#[test]
fn a_file_on_another_system_is_said_and_placed_at_the_origin_only_when_asked() {
    let (mut app, _) = house_on_the_map();
    for (file, words) in [
        (fixture(BSI43), ["EPSG:32760", "EPSG:5256"]),
        (fixture(BSI2X3), ["harita dönüşümü", "IfcMapConversion"]),
    ] {
        let before = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .underlays()
            .len();
        added(&mut app, &file);
        let w = app.modules.arch().underlays_window.as_ref().expect("open");
        let (kept, why) = w.unplaced.clone().expect("not placed, said");
        assert_eq!(kept, file.display().to_string());
        assert!(words.iter().all(|x| why.contains(x)), "{why}");
        assert_eq!(
            app.document
                .as_ref()
                .expect("a drawing")
                .model
                .underlays()
                .len(),
            before,
            "nothing written"
        );
        send(&mut app, Event::Origin);
        settle(&mut app);
        let doc = &app.document.as_ref().expect("a drawing").model;
        assert_eq!(doc.underlays().len(), before + 1);
        assert_eq!(doc.underlays()[before].placement, UnderlayPlacement::Origin);
        assert!(
            app.modules
                .arch()
                .underlays_window
                .as_ref()
                .expect("open")
                .unplaced
                .is_none()
        );
        let l = loaded(&app, before);
        assert!(!l.bodies.is_empty());
        assert_eq!(l.bodies.len() + l.not_shown.len(), l.with_shape());
        println!(
            "{}: {} şekilli nesneden {} gösterildi (%{}), {} gösterilemedi; okuma {:.3} s, bellek {} B",
            l.schema,
            l.with_shape(),
            l.bodies.len(),
            l.bodies.len() * 100 / l.with_shape().max(1),
            l.not_shown.len(),
            l.took.as_secs_f64(),
            l.memory()
        );
        println!("  kutu {:?}", l.bounds);
        for (owner, why) in &l.not_shown {
            println!("  gösterilemeyen: {} — {why}", l.object_words(*owner));
        }
    }
}

#[test]
fn an_object_is_found_in_the_plan_and_in_3d_and_a_changed_file_is_said() {
    let (mut app, path) = house_on_the_map();
    added(&mut app, &path);
    let id = app.document.as_ref().expect("a drawing").model.underlays()[0]
        .id
        .clone();
    // Planda seç: a click on the west wall's outer face.
    send(&mut app, Event::Pick);
    assert!(
        app.modules.arch().underlay_picking && app.dialog.is_none(),
        "the window steps aside for the click"
    );
    assert!(app.underlays_picked(Some(kentos_interaction::Vec2::new(0.0, 2.5))));
    assert_eq!(
        app.dialog,
        Some(crate::arch::dialog(crate::arch::ArchDialogKind::Underlays))
    );
    let (got, object) = app
        .modules
        .arch()
        .underlays_window
        .as_ref()
        .expect("open")
        .object
        .clone()
        .expect("found");
    assert_eq!(got, id);
    let l = loaded(&app, 0);
    assert_eq!(l.model.objects[object].class, "IfcWall");
    assert!(
        l.model
            .details(&l.bytes, object)
            .is_ok_and(|d| !d.groups.is_empty()),
        "its details read"
    );
    // 3B görünüm: the underlay's bodies beside the building's; a click on one shows it.
    let _ = app.update(Message::Run("scene3d.view"));
    let b = app
        .scene3d_window
        .as_ref()
        .and_then(|w| w.building.as_ref())
        .expect("the building");
    assert_eq!(b.underlays.len(), 1);
    assert!(b.count() >= b.bodies() + l.bodies.len());
    let w3 = app.scene3d_window.as_ref().expect("the window");
    assert!(
        w3.scene_parts()
            .contains(&kentos_model_view::parts::UNDERLAY),
        "violet beside the building"
    );
    let v = b.viewer.view().expect("the model shown");
    assert!(
        v.input()
            .objects
            .iter()
            .any(|o| o.label.ends_with("(altlık model)")),
        "the underlay's project in the tree"
    );
    println!(
        "3B: {} kaynak, {} üçgen",
        w3.scene_sources(),
        w3.scene_triangles()
    );
    app.modules
        .arch_mut()
        .underlays_window
        .as_mut()
        .expect("open")
        .object = None;
    app.building3d_select(Some(format!(
        "{}{id}:{object}",
        crate::scene3d_building::UNDERLAY_OWNER
    )));
    assert_eq!(
        app.modules
            .arch()
            .underlays_window
            .as_ref()
            .expect("open")
            .object,
        Some((id.clone(), object))
    );
    // The file changed on disk: said, not shown, until Yeniden oku.
    let mut bytes = std::fs::read(&path).expect("the file");
    bytes.extend_from_slice(b"\n/* sonradan eklendi */\n");
    std::fs::write(&path, &bytes).expect("writes");
    app.modules.arch_mut().underlays.states.clear();
    settle(&mut app);
    match app.modules.arch().underlays.states.get(&id).map(|s| &s.1) {
        Some(State::Failed(why)) => assert!(why.contains("değişmiş"), "{why}"),
        other => panic!("not said: {other:?}"),
    }
    send(&mut app, Event::Reload);
    settle(&mut app);
    assert_eq!(
        app.document.as_ref().expect("a drawing").model.underlays()[0].sha256,
        kentos_commands::encoding::sha256_hex(&bytes)
    );
    assert!(matches!(
        app.modules.arch().underlays.states.get(&id).map(|s| &s.1),
        Some(State::Ready(_))
    ));
}

/// The interior wall of the house (its axis on x = 8) by its persistent id.
fn interior_wall(app: &App) -> String {
    let snap = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .to_snapshot_v2();
    snap.entities
        .iter()
        .zip(&snap.uids)
        .find(|(e, _)| {
            let v = serde_json::to_value(e).expect("json");
            v["kind"] == "wall"
                && v["pts"][0]["x"]
                    .as_f64()
                    .is_some_and(|x| (x - 8.0).abs() < 1e-9)
        })
        .map(|(_, u)| u.to_string())
        .expect("the interior wall")
}

/// The window's pictures (`.run/gui-evidence/altlik-8101/`), light and dark: the house's own model with a wall picked;
/// the sample on another system said and then placed at the origin; the plan with the interior wall moved 1 m off its
/// place in the model under it; 3B görünüm with both models; the command's place.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/altlik-8101");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        let (mut app, path) = house_on_the_map();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let shot = |app: &mut App, snapshot: &mut Snapshot, file: &str| {
            app.toasts.clear();
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        added(&mut app, &path);
        send(&mut app, Event::Discipline("Mimari".into()));
        send(&mut app, Event::Revision("R1".into()));
        send(&mut app, Event::Apply);
        app.modules.arch_mut().underlay_picking = true;
        let _ = app.underlays_picked(Some(kentos_interaction::Vec2::new(0.0, 2.5)));
        shot(&mut app, &mut snapshot, "1-kendi-modeli");

        // The sample on another system: said; then placed at the origin when asked.
        added(&mut app, &fixture(BSI43));
        shot(&mut app, &mut snapshot, "2-baska-sistem");
        send(&mut app, Event::Origin);
        settle(&mut app);
        shot(&mut app, &mut snapshot, "3-ornek");

        // The plan: the sample hidden, the interior wall moved 1 m east; the model under it keeps the wall on x = 8.
        send(&mut app, Event::Shown(false));
        send(&mut app, Event::Close);
        let wall = interior_wall(&app);
        run(
            &mut app,
            "cad.entities.transform",
            json!({"uids": [wall], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}),
        );
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, &mut snapshot, "4-plan");

        // 3B görünüm with both models.
        let _ = app.update(Message::Run("arch.underlays"));
        let sample = app.document.as_ref().expect("a drawing").model.underlays()[1]
            .id
            .clone();
        send(&mut app, Event::Choose(sample));
        send(&mut app, Event::Shown(true));
        settle(&mut app);
        send(&mut app, Event::Close);
        let _ = app.update(Message::Run("scene3d.view"));
        snapshot.settle(&mut app, App::scene3d_shown, &mut update);
        snapshot.settle(&mut app, App::scene3d_shown, &mut update);
        let file = out.join(format!("5-3b-{name}.png"));
        snapshot
            .render(app.scene3d_shown(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());

        // Its place on the ribbon: Mimari › Proje ▾.
        let _ = app.update(Message::RibbonSearch("Altlık modeller".into()));
        let _ = app.update(Message::RibbonSearchReveal(0));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Click(iced::Point::new(1278.0, 133.0)),
        );
        shot(&mut app, &mut snapshot, "6-serit-yeri");
    }
}
