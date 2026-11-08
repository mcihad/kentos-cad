//! The pictures and checks of docs/adr/2600 (Paralel kaydır's shared edge with the vertices inside it) in a CAD
//! project at 1:500: parcel A west of two parcels B1 and B2 that meet on A's east edge (a T junction), and a branch line
//! ending on the same edge inside B2. Topolojik düzenleme on, A's east edge 3 m east: the preview draws B1, B2 and the
//! branch following, their shared corner and the branch's end on the moved edge; written, the message says the two
//! places inside the edge went with it. `tools_screens` takes them:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=paralel-kaydir-ucuncu-birlesim,paralel-kaydir-ucuncu-birlesim-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// A (1), B1 (2), B2 (3) and the branch (4).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path("parsel", &[[200.0, -10.0], [210.0, -10.0], [210.0, 10.0], [200.0, 10.0]], true);
    o.path("parsel", &[[210.0, -10.0], [220.0, -10.0], [220.0, 0.0], [210.0, 0.0]], true);
    o.path("parsel", &[[210.0, 0.0], [220.0, 0.0], [220.0, 10.0], [210.0, 10.0]], true);
    o.path("yol", &[[210.0, 5.0], [218.0, 5.0]], false);
    o
}

/// The drawing at 1:500, Topolojik düzenleme on, Paralel kaydır running.
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
        &kentos_render_wgpu::Bounds {
            min_x: E + 192.0,
            min_y: N - 16.0,
            max_x: E + 232.0,
            max_y: N + 16.0,
        },
        24.0,
    );
    run(app, "draft.topology");
    run(app, "tool.edgeShift");
}

/// A's east edge picked inside A, the cursor 3 m east.
fn shown(app: &mut App) {
    opened(app);
    click(app, [209.9, -6.0]);
    hover(app, [213.0, -6.0]);
}

/// The edge 3 m out, written: one step, the neighbours and the places inside the edge with it.
fn written(app: &mut App) {
    shown(app);
    typed(app, "3");
    hover(app, [228.0, -13.0]);
}

fn corners(app: &App, slot: u32) -> Vec<[f64; 2]> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    match doc.get(Slot(slot)) {
        Some(kentos_commands::Entity::Polygon(p)) => p.pts.iter().map(|q| [q.x - E, q.y - N]).collect(),
        Some(kentos_commands::Entity::Polyline(p)) => p.pts.iter().map(|q| [q.x - E, q.y - N]).collect(),
        other => panic!("{other:?}"),
    }
}

fn has(pts: &[[f64; 2]], p: [f64; 2]) -> bool {
    pts.iter().any(|q| (q[0] - p[0]).abs() < 1e-6 && (q[1] - p[1]).abs() < 1e-6)
}

#[test]
fn a_shared_edge_carries_the_t_junction_and_the_branch_s_end() {
    let mut app = App::boot(None).0;
    shown(&mut app);
    // The preview draws the three neighbours (their outlines) beside the shape.
    let preview = app.session.preview(&app.format()).expect("a preview");
    assert!(preview.strokes.len() >= 4, "the edge, the moved shape and the neighbours: {}", preview.strokes.len());
    typed(&mut app, "3");
    assert!(has(&corners(&app, 2), [213.0, 0.0]) && has(&corners(&app, 3), [213.0, 0.0]), "the T junction moved");
    assert!(has(&corners(&app, 4), [213.0, 5.0]), "the branch's end moved: {:?}", corners(&app, 4));
    let said = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(
        said.contains("Topolojik düzenleme: 3 komşu nesne de değişti; kenarın ortasındaki 2 ortak köşe de kenarla kaydı."),
        "{said}"
    );
    // One undo step takes them all back.
    let _ = app.update(crate::app::Message::Run("edit.undo"));
    assert!(has(&corners(&app, 2), [210.0, 0.0]) && has(&corners(&app, 4), [210.0, 5.0]));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("paralel-kaydir-ucuncu-birlesim", shown),
        ("paralel-kaydir-ucuncu-birlesim-yazildi", written),
    ]
}
