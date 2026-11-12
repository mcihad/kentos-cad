//! Eğimle kot ver's Hedef Referanstan (grade_reference.rs, docs/adr/3780) on the shared cases' drawing
//! (`fixtures/commands/v1/cad.entities.raise.json`): the kerb's fourth vertex 2 % up from the door's threshold previews
//! the shared case's answer and Uygula writes it in one undo step; Sahneden seç takes the reference from a vertex with
//! an elevation, then the vertex; the reference stays for the next object; a vertex without an elevation is said and
//! Uygula stays off; on the drawing the reference and the vertex are ringed. And the pictures. Docs/adr/4980: on the
//! two-part area Sahneden seç's vertex click by the first part's hole chooses that ring; Uygula writes the shared case's.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::grade_reference::By;
use crate::grade_window::{Event as G, Pick, Target};

const X0: f64 = 500000.0;
const Y0: f64 = 4400000.0;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.raise.json")).expect("cases")
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

/// The two-part area (slot 3) on Referanstan: the reference typed place by Sahneden seç's first click, the vertex by its
/// second by the first part's hole's second vertex (220, 10), 0.5 m down from 120.75 m (docs/adr/4980).
fn on_a_hole() -> App {
    let mut app = opened();
    app.selection.set([Slot(3)]);
    let _ = app.update(Message::Run("tool.grade"));
    let _ = app.update(Message::Grade(G::Target(Target::Reference)));
    let _ = app.update(Message::Grade(G::Pick));
    assert!(app.grade_picked(Some(kentos_interaction::Vec2 { x: X0 + 260.0, y: Y0 - 15.0 })));
    assert_eq!(app.grade.picking, Some(Pick::Vertex), "the vertex awaited");
    assert!(app.grade_picked(Some(kentos_interaction::Vec2 { x: X0 + 220.01, y: Y0 + 9.99 })));
    assert_eq!((app.grade.ring, app.grade.at), ((0, 1), 2.0), "the first part's hole, its second vertex");
    // A real click ends its pick step before the point arrives; called directly here, the step is ended as it would be.
    let _ = app.with_tool(|s, cx| s.cancel(cx));
    for e in [G::Height(120.75), G::By(By::Difference), G::Difference(-0.5)] {
        let _ = app.update(Message::Grade(e));
    }
    app
}

#[test]
fn the_vertex_click_chooses_the_hole_and_uygula_writes_the_case() {
    let mut app = on_a_hole();
    let want = case("Alan: ilk parçanın deliğinin");
    let raised = app.grade.raised.clone().expect("a plan").expect("planned");
    assert!((raised.after - 120.25).abs() < 1e-9, "{raised:?}");
    let _ = app.update(Message::Grade(G::Apply));
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Polygon(p)) = doc.model.get(Slot(3)) else { panic!("the area") };
    let got = serde_json::json!({"zs": p.zs, "holes": p.holes, "parts": p.parts});
    let fields = &want["steps"][0]["expect"]["fields"]["3"];
    for k in ["zs", "holes", "parts"] {
        let (g, w) = (&got[k], &fields[k]);
        let flat = |v: &Value| v.to_string();
        // Elevations within 1e-9: compared through the case's numbers.
        assert!(close(g, w), "{k}: {} vs {}", flat(g), flat(w));
    }
    let _ = app.update(Message::Run("edit.undo"));
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Polygon(p)) = doc.model.get(Slot(3)) else { panic!("the area") };
    assert_eq!(p.holes.as_ref().expect("holes")[0].zs.as_ref().expect("elevations")[1], Some(119.95), "one undo step");
}

fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => (x.as_f64().unwrap_or(f64::NAN) - y.as_f64().unwrap_or(f64::NAN)).abs() <= 1e-9,
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q)),
        (Value::Object(x), Value::Object(y)) => {
            y.iter().all(|(k, w)| x.get(k).is_some_and(|v| close(v, w))) && x.iter().all(|(k, v)| v.is_null() || y.contains_key(k))
        }
        _ => a == b,
    }
}

/// `.run/gui-evidence/cok-parcali-aralik-4980/`: Referanstan on the first part's hole, light wide and dark narrow.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_reference_tests::screens_on_a_hole -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_on_a_hole() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cok-parcali-aralik-4980");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = on_a_hole();
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // The first part and the reference (200–260, −15–30 from X0, Y0) left of the window, which stands in the middle.
        let camera = &mut app.viewport.camera;
        camera.scale = if size.width > 1200.0 { 3.0 } else { 1.8 };
        camera.center = kentos_interaction::Vec2::new(X0 + 230.0 + (camera.width / 2.0 - camera.width * 0.15) / camera.scale, Y0 + 7.5);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("3-referanstan-delikte-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
