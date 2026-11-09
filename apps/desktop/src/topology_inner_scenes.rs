//! The pictures and checks of docs/adr/3400 (the neighbours' vertices inside an edge a grip, Köşe tablosu or Esnet
//! turns) in a CAD project at 1:500: parcel A with B1 and B2 north of it meeting on A's north edge (a T junction), and
//! the line J inside B2 ending on the same edge. Topolojik düzenleme on, A's north-east corner dragged by its grip 4 m
//! east and 2 m north: the preview draws B1, B2 and J following, the junction and J's end on the turned edge at the same
//! fractions of it; written, the message says the two places inside the edge went with it. Köşe tablosu does the same
//! with a typed coordinate. `tools_screens` takes the pictures:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tutamac-ic-kose-onizleme,tutamac-ic-kose-yazildi,kose-tablosu-ic-kose cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, at, open, run};
use crate::tools_screens::Scene;
use crate::viewport::Event;

/// A (1), B1 (2), B2 (3), J (4).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path("parsel", &[[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]], true);
    o.path("parsel", &[[0.0, 30.0], [20.0, 30.0], [20.0, 50.0], [0.0, 50.0]], true);
    o.path("parsel", &[[20.0, 30.0], [40.0, 30.0], [40.0, 50.0], [20.0, 50.0]], true);
    o.path("yol", &[[30.0, 40.0], [30.0, 30.0]], false);
    o
}

/// The drawing at 1:500, Topolojik düzenleme on, A selected, the parcels in the left of the drawing area.
fn opened(app: &mut App) {
    open(app, ground());
    make_cad(app);
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 500.0;
        doc.model.set_settings(settings);
    }
    app.tab = "home";
    app.command_expanded = false;
    app.draft.snap = false;
    fit(app);
    run(app, "draft.topology");
    app.selection.set([Slot(1)]);
}

fn fit(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E - 10.0, min_y: N - 10.0, max_x: E + 70.0, max_y: N + 58.0 },
        24.0,
    );
}

/// A's north-east grip pressed and dragged 4 m east and 2 m north, the button still down.
fn dragging(app: &mut App) {
    opened(app);
    let (from, to) = (at(app, [40.0, 30.0]), at(app, [44.0, 32.0]));
    let _ = app.update(Message::Viewport(Event::Moved(from)));
    let _ = app.update(Message::Viewport(Event::Pressed(from)));
    let _ = app.update(Message::Viewport(Event::Moved(to)));
}

/// The grip let go: written in one step.
fn dropped(app: &mut App) {
    dragging(app);
    let to = at(app, [44.0, 32.0]);
    let _ = app.update(Message::Viewport(Event::Released(to)));
}

/// Köşe tablosu: A's third vertex's north typed 2 m up (X 4420032).
fn typed_in_the_table(app: &mut App) {
    opened(app);
    type_in_the_table(app);
}

/// The table's steps on the opened drawing.
fn type_in_the_table(app: &mut App) {
    use crate::vertices::{Event as V, Walk};
    use crate::vertices::edit::Column;
    let _ = app.update(Message::Run("view.coords"));
    let _ = app.update(Message::Vertices(V::Edit(2, Column::North)));
    let _ = app.update(Message::Vertices(V::Input(format!("{}", N + 32.0))));
    let _ = app.update(Message::Vertices(V::Finish(Some(Walk::Down))));
}

fn corners(app: &App, slot: u32) -> Vec<[f64; 2]> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    match doc.get(Slot(slot)) {
        Some(kentos_commands::Entity::Polygon(p)) | Some(kentos_commands::Entity::Polyline(p)) => {
            p.pts.iter().map(|q| [q.x - E, q.y - N]).collect()
        }
        other => panic!("{other:?}"),
    }
}

fn has(pts: &[[f64; 2]], p: [f64; 2]) -> bool {
    pts.iter().any(|q| (q[0] - p[0]).abs() < 1e-6 && (q[1] - p[1]).abs() < 1e-6)
}

fn said(app: &App) -> String {
    app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n")
}

/// A drawing area of its own size, the camera on the parcels again: the cursor's place on the screen is a place on
/// the drawing.
fn app() -> App {
    let mut app = App::boot(None).0;
    let size = iced::Size::new(1440.0, 900.0);
    let _ = app.update(Message::WindowResized(size));
    opened(&mut app);
    let mut screen = crate::files_testing::offscreen(size);
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    screen.settle(&mut app, App::view, &mut update);
    fit(&mut app);
    app
}

#[test]
fn a_grip_carries_the_t_junction_and_the_line_s_end_with_the_edge() {
    let mut app = app();
    let (from, to) = (at(&app, [40.0, 30.0]), at(&app, [44.0, 32.0]));
    let _ = app.update(Message::Viewport(Event::Moved(from)));
    let _ = app.update(Message::Viewport(Event::Pressed(from)));
    let _ = app.update(Message::Viewport(Event::Moved(to)));
    let _ = app.update(Message::Viewport(Event::Released(to)));
    // The corner where the pointer let go (about 4 m east, 2 m north; the screen's pixel), the turned edge from (0, 30)
    // to it: the junction at its half, J's end a quarter from the corner.
    let c = corners(&app, 1)[2];
    assert!((c[0] - 44.0).abs() < 0.01 && (c[1] - 32.0).abs() < 0.01, "{c:?}");
    let half = [c[0] / 2.0, (30.0 + c[1]) / 2.0];
    let quarter = [c[0] - 0.25 * c[0], c[1] + 0.25 * (30.0 - c[1])];
    assert!(has(&corners(&app, 2), half) && has(&corners(&app, 3), half), "{half:?} {:?}", corners(&app, 3));
    assert!(has(&corners(&app, 3), c), "the shared corner");
    assert!(has(&corners(&app, 4), quarter), "J's end {quarter:?}: {:?}", corners(&app, 4));
    let words = said(&app);
    assert!(
        words.contains("Topolojik düzenleme: 3 komşu nesne de değişti; kenarın ortasındaki 2 ortak köşe de kenarla gitti."),
        "{words}"
    );
    // One undo step takes them all back.
    let _ = app.update(Message::Run("edit.undo"));
    assert!(has(&corners(&app, 2), [20.0, 30.0]) && has(&corners(&app, 4), [30.0, 30.0]));
}

#[test]
fn the_vertex_table_carries_them_too() {
    let mut app = app();
    type_in_the_table(&mut app);
    // The edge (40, 32)–(0, 30): the junction at (20, 31), J's end at (30, 31.5).
    assert!(has(&corners(&app, 1), [40.0, 32.0]), "{:?}", corners(&app, 1));
    assert!(has(&corners(&app, 2), [20.0, 31.0]) && has(&corners(&app, 3), [20.0, 31.0]), "{:?}", corners(&app, 3));
    assert!(has(&corners(&app, 4), [30.0, 31.5]), "J's end: {:?}", corners(&app, 4));
    let words = said(&app);
    assert!(words.contains("kenarın ortasındaki 2 ortak köşe de kenarla gitti"), "{words}");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("tutamac-ic-kose-onizleme", dragging),
        ("tutamac-ic-kose-yazildi", dropped),
        ("kose-tablosu-ic-kose", typed_in_the_table),
    ]
}
