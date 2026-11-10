//! The pictures and checks of docs/adr/5000 (Köşeyi sil joins two arcs of one circle) in a CAD project at 1:500: parcel
//! A, a half disc about (20, 0) of radius 25 whose round edge is two quarter arcs meeting at (20, 25), and parcel B
//! above it along the same arcs the other way. Topolojik düzenleme on, A selected, a right click on the grip at (20, 25)
//! and Köşeyi sil: A's two arcs become one half circle (bulge 1) and B's the same, the other way (−1); the shared
//! boundary stays one. docs/adr/5000 §7: Köşe tablosu open on A, an arrow by each edge's middle the way the vertices
//! run. `tools_screens` takes the pictures:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=yay-birlestir-menu,yay-birlestir-yazildi,kose-gidis-oku cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_interaction::grip_menu::GripAction;
use serde_json::json;

use crate::app::{App, Message};
use crate::drawing_menus::Event as Menu;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, at, open, run, xy};
use crate::tools_screens::Scene;
use crate::viewport::Event;

/// tan(π/8): a quarter circle's bulge.
fn quarter() -> f64 {
    (std::f64::consts::PI / 8.0).tan()
}

/// A (1) and B (2).
fn ground() -> Objects {
    let b = quarter();
    let mut o = Objects::new();
    let pts = |p: &[[f64; 2]]| p.iter().map(|q| xy(*q)).collect::<Vec<_>>();
    o.push(
        "parsel",
        json!({ "kind": "polygon", "pts": pts(&[[-5.0, 0.0], [20.0, 0.0], [45.0, 0.0], [20.0, 25.0]]),
                "bulges": [0.0, 0.0, b, b] }),
    );
    o.push(
        "parsel",
        json!({ "kind": "polygon", "pts": pts(&[[-5.0, 0.0], [20.0, 25.0], [45.0, 0.0], [45.0, 40.0], [-5.0, 40.0]]),
                "bulges": [-b, -b, 0.0, 0.0, 0.0] }),
    );
    o
}

/// The drawing at 1:500, Topolojik düzenleme on, A selected, the parcels in the drawing area.
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
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E - 20.0, min_y: N - 10.0, max_x: E + 60.0, max_y: N + 45.0 },
        24.0,
    );
    run(app, "draft.topology");
    app.selection.set([Slot(1)]);
    let doc = app.document.as_ref().expect("a drawing");
    app.spatial.sync(&doc.model);
}

/// The grip menu open on A's vertex at (20, 25).
fn menu(app: &mut App) {
    opened(app);
    let p = at(app, [20.0, 25.0]);
    let _ = app.update(Message::Viewport(Event::RightPressed(p)));
    let _ = app.update(Message::Viewport(Event::RightClick(p)));
}

/// Köşeyi sil chosen in the menu.
fn removed(app: &mut App) {
    menu(app);
    let _ = app.update(Message::DrawingMenu(Menu::Grip(GripAction::RemoveVertex)));
}

fn path(app: &App, slot: u32) -> (Vec<[f64; 2]>, Vec<f64>) {
    let doc = &app.document.as_ref().expect("a drawing").model;
    match doc.get(Slot(slot)) {
        Some(kentos_commands::Entity::Polygon(p)) => (
            p.pts.iter().map(|q| [q.x - E, q.y - N]).collect(),
            p.bulges.clone().unwrap_or_default(),
        ),
        other => panic!("{other:?}"),
    }
}

/// docs/adr/5000: the two quarter arcs become one half circle in A and in B, one undo step puts both back.
#[test]
fn removing_the_vertex_between_two_arcs_of_one_circle_leaves_one_arc() {
    let mut app = App::boot(None).0;
    menu(&mut app);
    let open = app.drawing_menu.as_ref().expect("a menu");
    assert_eq!(open.grip.map(|g| g.header()).as_deref(), Some("Köşe 4"));
    let _ = app.update(Message::DrawingMenu(Menu::Grip(GripAction::RemoveVertex)));
    let (pts, bulges) = path(&app, 1);
    assert_eq!(pts.len(), 3, "{pts:?}");
    assert!((bulges[2] - 1.0).abs() < 1e-12, "{bulges:?}");
    let (pts, bulges) = path(&app, 2);
    assert_eq!(pts.len(), 4, "{pts:?}");
    assert!((bulges[0] + 1.0).abs() < 1e-12, "{bulges:?}");
    let words = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(words.contains("Köşe sil: tamam."), "{words}");
    assert!(words.contains("Topolojik düzenleme: 1 komşu nesne de değişti."), "{words}");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(path(&app, 1).0.len(), 4);
    assert_eq!(path(&app, 2).0.len(), 5);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("yay-birlestir-menu", menu), ("yay-birlestir-yazildi", removed), ("kose-gidis-oku", table_ways)]
}

/// docs/adr/5000 §7: Köşe tablosu open on A: an arrow at each edge's middle the way the vertices run.
fn table_ways(app: &mut App) {
    opened(app);
    let _ = app.update(Message::Run("view.coords"));
}

/// docs/adr/5000 §7: A's four edges, each with its middle (the arcs' on the arc) and its way; none with the table shut.
#[test]
fn the_table_shows_the_way_the_vertices_run() {
    let mut app = App::boot(None).0;
    opened(&mut app);
    assert!(app.vertex_ways().is_empty());
    let _ = app.update(Message::Run("view.coords"));
    let ways = app.vertex_ways();
    assert_eq!(ways.len(), 4, "{ways:?}");
    // The first edge (−5, 0) → (20, 0): its middle, east.
    let (mid, dir, len) = ways[0];
    assert!((mid.x - E - 7.5).abs() < 1e-9 && (mid.y - N).abs() < 1e-9 && (dir.x - 1.0).abs() < 1e-12 && len == 25.0);
    // The arc (45, 0) → (20, 25) about (20, 0): its middle at 45° on the circle, the way north-west.
    let (mid, dir, _) = ways[2];
    let h = 25.0 / std::f64::consts::SQRT_2;
    assert!((mid.x - E - 20.0 - h).abs() < 1e-9 && (mid.y - N - h).abs() < 1e-9, "{mid:?}");
    assert!((dir.x + std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12 && (dir.y - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
}
