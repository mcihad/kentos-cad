//! Eğimle kot ver's Hedef Referanstan (grade_reference.rs, docs/adr/3780) on the shared cases' drawing
//! (`fixtures/commands/v1/cad.entities.raise.json`): the kerb's fourth vertex 2 % up from the door's threshold previews
//! the shared case's answer and Uygula writes it in one undo step; Sahneden seç takes the reference from a vertex with
//! an elevation, then the vertex; the reference stays for the next object; a vertex without an elevation is said and
//! Uygula stays off; on the drawing the reference and the vertex are ringed. And the pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::grade_reference::By;
use crate::grade_window::{Event as G, Pick, Target};

const X0: f64 = 500000.0;
const Y0: f64 = 4400000.0;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.entities.raise.json")).expect("cases")
}

fn opened() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn zs(app: &App, slot: u32) -> Vec<Option<f64>> {
    match app.document.as_ref().and_then(|d| d.model.get(Slot(slot))) {
        Some(kentos_commands::Entity::Polyline(p)) => p.zs.clone().unwrap_or_default(),
        _ => panic!("a polyline"),
    }
}

/// The kerb selected, Eğimle kot ver open on Referanstan.
fn on_reference() -> App {
    let mut app = opened();
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("tool.grade"));
    let _ = app.update(Message::Grade(G::Target(Target::Reference)));
    app
}

/// The threshold typed, the fourth vertex, 2 %.
fn typed(app: &mut App) {
    for e in [G::East(X0 - 20.0), G::North(Y0 + 10.0), G::Height(849.75), G::At(4.0), G::Percent(2.0)] {
        let _ = app.update(Message::Grade(e));
    }
}

fn case(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name))).expect("the case").clone()
}

#[test]
fn referanstan_previews_the_case_and_writes_one_step() {
    let mut app = on_reference();
    assert_eq!((app.dialog, app.grade.target), (Some(Dialog::Grade), Target::Reference));
    assert!(app.grade.raised.is_none() && !app.grade.referenced, "nothing until a reference is given");
    typed(&mut app);
    let step = &case("Eğimle: köşe eşikten %2 yukarıda")["steps"][0];
    let want = &step["result"]["output"];
    let got = serde_json::to_value(app.grade.raised.clone().expect("a plan").expect("previewed")).expect("json");
    for key in ["distance", "before", "after", "by", "grade"] {
        let (a, b) = (got[key].as_f64().expect("a number"), want[key].as_f64().expect("a number"));
        assert!((a - b).abs() < 1e-9, "{key}: {a} ≠ {b}");
    }
    assert_eq!((&got["moved"], &got["unset"]), (&want["moved"], &want["unset"]));
    let _ = app.update(Message::Grade(G::Apply));
    let after: Vec<Option<f64>> = serde_json::from_value(step["expect"]["fields"]["1"]["zs"].clone()).expect("zs");
    for (a, b) in zs(&app, 1).iter().zip(&after) {
        assert!(a.zip(*b).is_none_or(|(a, b)| (a - b).abs() < 1e-9) && a.is_some() == b.is_some(), "{a:?} ≠ {b:?}");
    }
    let said = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains("Referanstan kot verildi: 4. köşe"), "{said}");
    assert_eq!(app.dialog, None);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(zs(&app, 1), vec![Some(850.0), None, Some(850.4), Some(851.25), Some(851.0)], "one step");
}

/// Sahneden seç: the first click takes the reference from the line's end with its elevation, the second the kerb's
/// vertex; a first click by no elevation takes the place and says the elevation is to be typed.
#[test]
fn two_clicks_take_the_reference_and_the_vertex() {
    let mut app = on_reference();
    let _ = app.update(Message::Grade(G::Pick));
    assert_eq!((app.dialog, app.grade.picking), (None, Some(Pick::Reference)));
    // The line's second end (60, 180), 901.5 m.
    assert!(app.grade_picked(Some(kentos_interaction::Vec2 { x: X0 + 60.01, y: Y0 + 179.99 })));
    assert_eq!(app.grade.picking, Some(Pick::Vertex), "the vertex awaited");
    assert_eq!((app.grade.east, app.grade.north, app.grade.height), (X0 + 60.0, Y0 + 180.0, 901.5));
    // The kerb's third vertex (30, 40).
    assert!(app.grade_picked(Some(kentos_interaction::Vec2 { x: X0 + 30.01, y: Y0 + 40.0 })));
    assert_eq!((app.grade.at, app.dialog), (3.0, Some(Dialog::Grade)));
    let r = app.grade.raised.clone().expect("a plan").expect("previewed");
    assert!((r.distance - (30.0f64).hypot(140.0)).abs() < 1e-9);
    // By no elevation: the place, the elevation kept to type.
    let _ = app.update(Message::Grade(G::Pick));
    assert!(app.grade_picked(Some(kentos_interaction::Vec2 { x: X0 + 150.0, y: Y0 + 250.0 })));
    assert_eq!((app.grade.east, app.grade.north, app.grade.height), (X0 + 150.0, Y0 + 250.0, 901.5));
    let said = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains("tıklanan yerde kotlu nokta ya da köşe yok"), "{said}");
}

#[test]
fn the_reference_stays_for_the_next_object() {
    let mut app = on_reference();
    typed(&mut app);
    let _ = app.update(Message::Grade(G::By(By::Difference)));
    let _ = app.update(Message::Grade(G::Difference(-0.4)));
    let _ = app.update(Message::Grade(G::Apply));
    app.selection.set([Slot(2)]);
    let _ = app.update(Message::Run("tool.grade"));
    let s = &app.grade;
    assert_eq!((s.target, s.referenced, s.east, s.north, s.height, s.by), (Target::Reference, true, X0 - 20.0, Y0 + 10.0, 849.75, By::Difference));
    assert_eq!(s.at, 1.0, "the new object's first vertex");
    let r = s.raised.clone().expect("a plan").expect("previewed");
    assert!((r.after - (849.75 - 0.4)).abs() < 1e-9);
    // Vazgeç lets it go.
    let _ = app.update(Message::Grade(G::Close));
    let _ = app.update(Message::Run("tool.grade"));
    assert_eq!((app.grade.target, app.grade.referenced), (Target::Grade, false));
}

#[test]
fn a_vertex_without_an_elevation_is_said_and_uygula_stays_off() {
    let mut app = on_reference();
    typed(&mut app);
    let _ = app.update(Message::Grade(G::At(2.0)));
    let why = app.grade.raised.clone().expect("a plan").expect_err("refused");
    assert_eq!(why, "1. köşenin kotu yok; fark bulunamıyor. Kotlu bir köşe seçin ya da önce Kot ver ile kot verin.");
    let before = zs(&app, 1);
    let _ = app.update(Message::Grade(G::Apply));
    assert_eq!(zs(&app, 1), before, "nothing written");
}

#[test]
fn the_reference_and_the_vertex_are_marked() {
    let mut app = on_reference();
    assert_eq!(app.grade_marks(), (Vec::new(), Vec::new()), "no reference yet");
    typed(&mut app);
    let r = kentos_commands::Vec2 { x: X0 - 20.0, y: Y0 + 10.0 };
    let v = kentos_commands::Vec2 { x: X0 + 80.0, y: Y0 + 40.0 };
    assert_eq!(app.grade_marks(), (vec![r, v], vec![r, v]));
    let _ = app.update(Message::Grade(G::Close));
    assert_eq!(app.grade_marks(), (Vec::new(), Vec::new()), "closed");
}

/// The window on Referanstan with the kerb's fourth vertex 2 % up from the threshold, the kerb and the threshold left
/// of the window, light wide and dark narrow; then after Uygula with the kerb's vertices in Koordinat listesi.
/// `.run/gui-evidence/referanstan-kot-3780/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_reference_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/referanstan-kot-3780");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // The threshold and the kerb in the left third, where the window does not cover them.
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: X0 - 30.0, min_y: Y0 - 20.0, max_x: X0 - 30.0 + 3.4 * 140.0, max_y: Y0 + 60.0 },
            24.0,
        );
        app.selection.set([Slot(1)]);
        let _ = app.update(Message::Run("tool.grade"));
        let _ = app.update(Message::Grade(G::Target(Target::Reference)));
        typed(&mut app);
        app.selection.set(Vec::new());
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // After Uygula: the kerb's vertices in Koordinat listesi, their elevations moved by the same difference.
        let _ = app.update(Message::Grade(G::Apply));
        app.selection.set([Slot(1)]);
        app.show_bottom(crate::bottom::BottomTab::Coords);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("uygulandi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
