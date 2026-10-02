//! Kenar eşleme's tests: the window through the app over
//! `fixtures/interaction/v1/edgematch.kcad` (docs/adr/0159 §9): two sheets
//! digitised apart, their roads, a building line and a fence reaching the
//! shared edge a few centimetres short, over or beside their continuation;
//! a street crossing the edge at right angles, two roads meeting at the
//! edge, a parcel. The steps of the web's `edgematch` scenes (shots.mjs),
//! and their pictures.

use kentos_contracts::Entity;
use kentos_geometry_core::ops::edgematch::{Meet, Method};

use super::*;
use crate::app::{App, Dialog as Asking};
use crate::calc::Window;
use crate::exchange::words::Kind as Line;
use crate::files_testing::last_said;

/// The sheets' edge, x east.
const EDGE_X: f64 = 487200.0;

fn app_with_edge_drawing() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../fixtures/interaction/v1/edgematch.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn calc(app: &mut App, e: Calc) {
    let _ = app.update(Message::Calc(e));
}

fn edge(app: &mut App, e: Event) {
    calc(app, Calc::Edgematch(e));
}

/// The window opened on the scene: Pafta 1 and Pafta 2 as the drawing
/// offers them. The drawing in an 800 × 600 area, the sheets' edge in the
/// middle, 2 px a metre.
fn opened() -> App {
    let mut app = app_with_edge_drawing();
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
        iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(800.0, 600.0)),
    )));
    app.viewport.camera.center = kentos_interaction::Vec2::new(EDGE_X, 4420075.0);
    app.viewport.camera.scale = 2.0;
    let _ = app.run("transform.edgematch");
    assert_eq!(app.dialog, Some(Asking::Calc));
    assert_eq!(app.calc.open, Some(Window::Edgematch));
    app
}

fn key(app: &mut App, named: iced::keyboard::key::Named) {
    use iced::keyboard::key::{NativeCode, Physical};
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key: iced::keyboard::Key::Named(named),
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers: iced::keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    }));
}

/// Sınır shown on the drawing as a user shows it: the pick, a click on the
/// sheets' edge past the frames' corners (only it lies there), Enter.
fn pick_border(app: &mut App) {
    use crate::viewport::Event as View;
    edge(app, Event::PickBorder);
    let [x, y] = app
        .viewport
        .camera
        .world_to_screen(kentos_interaction::Vec2::new(EDGE_X, 4420152.5));
    let p = iced::Point::new(x as f32, y as f32);
    for e in [View::Moved(p), View::Pressed(p), View::Released(p)] {
        let _ = app.update(Message::Viewport(e));
    }
    key(app, iced::keyboard::key::Named::Enter);
}

fn said(app: &App) -> Vec<String> {
    app.calc
        .edgematch
        .summary_lines()
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

/// The slot of the object with this attribute value.
fn slot_named(app: &App, key: &str, value: &str) -> Slot {
    let doc = app.document.as_ref().expect("open");
    doc.model
        .entities()
        .find(|e| e.base().attrs.get(key).map(String::as_str) == Some(value))
        .map(|e| Slot(e.base().id))
        .expect("the object")
}

/// Pafta 1's line ends that lie on the edge, exactly.
fn ends_on_edge(app: &App) -> usize {
    let doc = app.document.as_ref().expect("open");
    doc.model
        .by_layer("pafta1")
        .flat_map(|e| match e {
            Entity::Line(l) => vec![l.a, l.b],
            Entity::Polyline(p) => vec![p.pts[0], p.pts[p.pts.len() - 1]],
            _ => Vec::new(),
        })
        .filter(|p| p.x == EDGE_X)
        .count()
}

/// On opening, the first two layers holding line work; the four
/// continuations found, the fence that turns north unmatched, the junction
/// and the parcel said; the building line's gap the largest.
#[test]
fn the_window_finds_the_scenes_links() {
    let app = opened();
    let form = &app.calc.edgematch;
    assert_eq!(form.source.as_deref(), Some("pafta1"));
    assert_eq!(form.adjacent.as_deref(), Some("pafta2"));
    assert_eq!(form.scope, Scope::Layer, "nothing is selected");
    assert_eq!(form.rows(), 4);
    assert_eq!(form.get(0, SOURCE), "Pafta 1: Yol");
    assert_eq!(form.get(3, SOURCE), "Pafta 1: Bina");
    assert_eq!(form.get(3, GAP), "215.4");
    assert_eq!(form.mark(3), Some(Mark::Worst));
    assert_eq!(
        said(&app),
        [
            "4 bağ bulundu; 4 bağ kullanılacak.",
            "En büyük aralık 215.4 mm: 4. satır; ortalama 123.7 mm.",
            "1 uç eşsiz kaldı: komşu bir uca yakın ama devamı bulunamadı.",
            "2 kavşak ucu eşlenmedi; yalnız biri taşınırsa kavşak bozulurdu.",
            "1 nesne katılmadı: yalnız çizgiler ve açık çoklu çizgiler eşlenir.",
        ]
    );
    assert_eq!(
        form.changes.len(),
        4,
        "the sources' ends move to their neighbours"
    );
}

/// Kullan off: the link is remembered as left out while the links are
/// found again; Uygula writes the rest.
#[test]
fn kullan_leaves_a_link_out() {
    let mut app = opened();
    calc(&mut app, Calc::Cell(3, USE, "0".into()));
    let form = &app.calc.edgematch;
    assert_eq!(form.mark(3), Some(Mark::Off));
    assert_eq!(said(&app)[0], "4 bağ bulundu; 3 bağ kullanılacak.");
    assert_eq!(form.changes.len(), 3);
    // Another distance finds the links again; the one left out stays out.
    edge(&mut app, Event::Distance("0.6".into()));
    assert_eq!(app.calc.edgematch.get(3, USE), "0");
    assert_eq!(app.calc.edgematch.changes.len(), 3);
}

/// Sınır shown on the drawing: the window steps aside, the edge picked
/// comes back as the border; the ends meet on it with the shift fading
/// along the lines; one step, Kenar eşle; the lines put right selected.
#[test]
fn the_ends_meet_on_the_border_in_one_step() {
    let mut app = opened();
    edge(&mut app, Event::PickBorder);
    assert_eq!(app.dialog, None, "the window steps aside");
    assert_eq!(
        app.session.prompt().text().split(':').next(),
        Some("Sınır"),
        "{}",
        app.session.prompt().text()
    );
    app.cancel();
    assert_eq!(app.dialog, Some(Asking::Calc), "Esc: the window comes back");
    assert!(app.calc.edgematch.border.is_none(), "nothing picked");
    pick_border(&mut app);
    assert_eq!(app.dialog, Some(Asking::Calc), "the window comes back");
    assert!(app.selection.is_empty(), "the selection as it was");
    let border = slot_named(&app, "Ad", "Pafta kenarı");
    let uid = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(border))
        .expect("an id");
    assert_eq!(
        app.calc.edgematch.border,
        Some(uid.to_string()),
        "the sheets' edge"
    );
    edge(&mut app, Event::Meet(Meet::Border));
    edge(&mut app, Event::Method(Method::Adjust));
    assert_eq!(
        said(&app)[2],
        "1 uç eşsiz kaldı: sınıra yakın ama devamı bulunamadı."
    );
    assert_eq!(ends_on_edge(&app), 0);
    edge(&mut app, Event::Apply);
    assert_eq!(app.dialog, None, "the window closes");
    assert_eq!(ends_on_edge(&app), 4, "every used link meets on the edge");
    assert_eq!(
        last_said(&app),
        "Kenar eşleme: 4 bağ yazıldı, 8 çizgi düzeltildi. Ctrl+Z geri alır."
    );
    assert_eq!(app.selection.len(), 8);
    let doc = app.document.as_mut().expect("open");
    assert_eq!(doc.model.undo().as_deref(), Some("Kenar eşle"));
    assert_eq!(ends_on_edge(&app), 0, "undone in one step");
}

/// Sınırda is off without a border; taking the border away goes back to
/// the neighbour's end.
#[test]
fn sinirda_needs_a_border() {
    let mut app = opened();
    edge(&mut app, Event::Meet(Meet::Border));
    assert_eq!(app.calc.edgematch.meet, Meet::Adjacent);
    pick_border(&mut app);
    edge(&mut app, Event::Meet(Meet::Border));
    assert_eq!(app.calc.edgematch.meet, Meet::Border);
    edge(&mut app, Event::ClearBorder);
    assert_eq!(app.calc.edgematch.meet, Meet::Adjacent);
    assert!(app.calc.edgematch.border.is_none());
}

/// Göster: the window steps aside, the link's two lines are selected, the
/// prompt says how to come back; Esc brings the window and the selection back.
#[test]
fn goster_looks_at_a_link_and_comes_back() {
    let mut app = opened();
    edge(&mut app, Event::Show(1));
    assert_eq!(app.dialog, None);
    assert_eq!(app.selection.len(), 2);
    assert_eq!(
        app.session.prompt().text(),
        "Kenar eşleme: 2. bağ, aralık 104.4 mm [Pencereye dön (tıklama, Enter ya da Esc)]"
    );
    app.cancel();
    assert_eq!(app.dialog, Some(Asking::Calc));
    assert_eq!(app.calc.open, Some(Window::Edgematch));
    assert!(app.selection.is_empty(), "the selection as it was");
}

/// Parça ekle: the building line becomes a three-vertex polyline in its
/// place, its id and data kept.
#[test]
fn parca_ekle_makes_a_line_a_polyline() {
    let mut app = opened();
    edge(&mut app, Event::Method(Method::Segment));
    let building = slot_named(&app, "Tür", "Bina");
    let uid = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(building))
        .expect("an id");
    edge(&mut app, Event::Apply);
    let doc = app.document.as_ref().expect("open");
    let slot = doc.model.slot_of(uid).expect("the same object");
    match doc.model.get(slot) {
        Some(Entity::Polyline(p)) => {
            assert_eq!(p.pts.len(), 3);
            assert_eq!(p.base.attrs.get("Tür").map(String::as_str), Some("Bina"));
        }
        other => panic!("{other:?}"),
    }
}

/// What is wrong is said, Uygula has nothing to write.
#[test]
fn a_search_distance_of_nothing_is_said() {
    let mut app = opened();
    edge(&mut app, Event::Distance("0".into()));
    assert_eq!(
        app.calc.edgematch.summary_lines(),
        [(
            Line::Warn,
            "Arama uzaklığı sıfırdan büyük bir uzunluk olmalı (metre).".to_owned()
        )]
    );
    assert!(app.calc.edgematch.changes.is_empty());
    edge(&mut app, Event::Distance("0.5".into()));
    edge(&mut app, Event::Adjacent("pafta1".into()));
    assert_eq!(
        said(&app),
        ["Kaynak ve komşu aynı katman. Komşu paftanın katmanını seçin."]
    );
}

/// The report: the settings, the links and the counts.
#[test]
fn the_report_lists_the_settings_and_the_links() {
    let app = opened();
    let doc = app.document.as_ref().expect("open");
    let lines = app.calc.edgematch.report(&doc.model, 0).expect("a report");
    assert_eq!(lines[0], ["Kenar eşleme"]);
    assert_eq!(lines[1], ["Kaynak", "Pafta 1"]);
    assert_eq!(lines[2], ["Komşu", "Pafta 2"]);
    assert_eq!(lines[3], ["Sınır", "—"]);
    assert_eq!(lines[9][0], "Kullan");
    assert_eq!(
        lines[10],
        ["evet", "1", "Pafta 1: Yol", "Pafta 2: Yol", "85.4", "0.5"]
    );
    assert_eq!(lines.len(), 10 + 4 + 1);
    assert_eq!(
        lines[14],
        ["Eşsiz uç", "1", "Kavşak ucu", "2", "Katılmayan", "1"]
    );
}

/// Scrolls every scrollable to its end (the window's foot in view).
struct SnapAll;

impl iced::advanced::widget::Operation for SnapAll {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: iced::Rectangle,
        _content_bounds: iced::Rectangle,
        _translation: iced::Vector,
        state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        state.snap_to(iced::widget::scrollable::RelativeOffset::END.into());
    }
}

/// The window in the web's scenes, for the owner. Not run by default:
/// `cargo test -p kentos-desktop calc::edgematch::tests::screens -- --ignored --nocapture`
/// (`KENTOS_SHOTS=kenar,…` for some of them); the web's are
/// `node apps/web/scripts/e2e/shots.mjs edgematch`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS").unwrap_or_default();
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "kenar",
                "kenar-sinir",
                "kenar-goster",
                "kenar-uygulandi",
                "kenar-uyari",
            ] {
                if !only.is_empty() && !only.split(',').any(|o| o == name) {
                    continue;
                }
                let mut app = opened();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let on_border = |app: &mut App| {
                    pick_border(app);
                    edge(app, Event::Meet(Meet::Border));
                    edge(app, Event::Method(Method::Adjust));
                };
                match name {
                    "kenar-sinir" => on_border(&mut app),
                    "kenar-goster" => edge(&mut app, Event::Show(1)),
                    "kenar-uygulandi" => {
                        on_border(&mut app);
                        edge(&mut app, Event::Apply);
                        // The second road's link, where the lines now meet on the edge.
                        app.viewport.change(kentos_interaction::ViewChange::Fit {
                            bounds: kentos_geometry_core::geometry::Bounds {
                                min_x: EDGE_X - 6.0,
                                min_y: 4420055.0,
                                max_x: EDGE_X + 6.0,
                                max_y: 4420067.0,
                            },
                            padding: 24.0,
                        });
                    }
                    "kenar-uyari" => edge(&mut app, Event::Distance("0".into())),
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "kenar-uyari" {
                    snapshot.operate(app.view(), Box::new(SnapAll));
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
