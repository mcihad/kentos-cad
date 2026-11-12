//! The pictures and checks of docs/adr/2580 (Paralel kaydır's Referans and Bu uzaklığı koru) in a CAD project at
//! 1:500: a parcel 6 m north of a road's edge, a setback area inside it. The parcel's south edge picked and Referans
//! (R) on the road: the road dashed, a guide square to it and “referanstan 5.000 m” beside the cursor; Koru (K) on;
//! 5 typed: written with the bond d1 = 5, its mark on the edge; the road moved 2 m south: the edge followed.
//! `tools_screens` takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=paralel-kaydir-referans,paralel-kaydir-koru,paralel-kaydir-koru-yazildi,paralel-kaydir-bag-izler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The parcel (1), the road's edge (2) and the setback area (3).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path("parsel", &[[120.0, -10.0], [140.0, -10.0], [140.0, 20.0], [120.0, 20.0]], true);
    o.path("yol", &[[110.0, -16.0], [160.0, -16.0]], false);
    o.path("parsel", &[[123.0, -5.0], [137.0, -5.0], [137.0, 17.0], [123.0, 17.0]], true);
    o
}

/// The drawing in a CAD project at 1:500, Paralel kaydır running.
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
    // Snaps off: the edge's own middle would catch the cursor the pictures put 5 m from the road.
    app.draft.snap = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + 104.0,
            min_y: N - 24.0,
            max_x: E + 166.0,
            max_y: N + 26.0,
        },
        24.0,
    );
    run(app, "tool.edgeShift");
}

/// The parcel's south edge picked, the road as its reference, the cursor 5 m from the road.
fn reference_shown(app: &mut App) {
    opened(app);
    click(app, [130.0, -10.0]);
    typed(app, "R");
    click(app, [130.0, -16.0]);
    hover(app, [130.0, -11.0]);
}

/// Koru on: the distance will be kept.
fn keep_shown(app: &mut App) {
    reference_shown(app);
    typed(app, "K");
    hover(app, [130.0, -11.0]);
}

/// 5 typed with Koru on: the edge 5 m from the road and the bond d1 = 5 written in one step.
fn keep_written(app: &mut App) {
    keep_shown(app);
    typed(app, "5");
    hover(app, [156.0, 12.0]);
}

/// The road moved 2 m south by Taşı's command: the bonded edge followed.
fn bond_follows(app: &mut App) {
    keep_written(app);
    move_road(app, -2.0);
    hover(app, [156.0, 12.0]);
}

/// The road moved `dy` metres north through `cad.entities.transform`, as Taşı writes it.
fn move_road(app: &mut App, dy: f64) {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let uid = doc.uid(Slot(2)).expect("the road").to_string();
    let input: kentos_commands::EntitiesTransform = serde_json::from_value(serde_json::json!({
        "uids": [uid], "transform": {"kind": "move", "dx": 0.0, "dy": dy}}))
    .expect("an input");
    let done = kentos_native_application::transform::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        input,
    );
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
}

/// The parcel's south edge's north, from the drawing's origin.
fn south(app: &App) -> f64 {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let Some(kentos_commands::Entity::Polygon(p)) = doc.get(Slot(1)) else {
        panic!("the parcel")
    };
    p.pts[0].y - N
}

fn said(app: &App) -> String {
    app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n")
}

fn parameters(app: &App) -> Vec<(String, String)> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.constraints().parameters.iter().map(|p| (p.name.clone(), p.expression.clone())).collect()
}

#[test]
fn referans_and_koru_write_the_bond_and_the_road_drags_the_edge() {
    let mut app = App::boot(None).0;
    keep_shown(&mut app);
    let prompt = app.session.prompt();
    assert!(prompt.options.iter().any(|o| o.label == "Koru" && o.value.as_deref() == Some("açık")), "{prompt:?}");
    typed(&mut app, "5");
    assert!((south(&app) + 11.0).abs() < 1e-6, "5 m from the road: {}", south(&app));
    assert_eq!(parameters(&app), [("d1".to_owned(), "5".to_owned())]);
    assert!(said(&app).contains("uzaklık d1 olarak korunuyor"), "{}", said(&app));
    // The road 2 m south: the edge follows, one step.
    move_road(&mut app, -2.0);
    assert!((south(&app) + 13.0).abs() < 1e-6, "followed: {}", south(&app));
    let _ = app.update(Message::Run("edit.undo"));
    assert!((south(&app) + 11.0).abs() < 1e-6, "undone together: {}", south(&app));
    // The bond's mark stands on the edge (Kısıtları göster is on).
    let marks = app.constraint_marks().expect("the bond's marks");
    assert!(!marks.glyphs.is_empty());
}

#[test]
fn a_reference_not_parallel_is_refused_and_enter_keeps_the_distance_now() {
    let mut app = App::boot(None).0;
    opened(&mut app);
    click(&mut app, [130.0, -10.0]);
    typed(&mut app, "R");
    // The parcel's own west edge: square to the south edge.
    click(&mut app, [120.0, 5.0]);
    assert!(said(&app).contains("Kenar referansa paralel değil (aralarında 90.0000° var)"), "{}", said(&app));
    click(&mut app, [130.0, -16.0]);
    assert!(said(&app).contains("Referans seçildi: kenar şimdi 6.000 m uzakta"), "{}", said(&app));
    // Koru without a distance: Enter keeps the 6 m there are.
    typed(&mut app, "K");
    run(&mut app, "tool.confirm");
    assert!((south(&app) + 10.0).abs() < 1e-6, "the edge stays: {}", south(&app));
    assert_eq!(parameters(&app), [("d1".to_owned(), "6".to_owned())]);
    // Past the road the edge would leave its side: said, nothing written.
    click(&mut app, [130.0, -10.0]);
    click(&mut app, [130.0, -21.0]);
    assert!(said(&app).contains("İmleç referansın öbür yanında"), "{}", said(&app));
    assert!((south(&app) + 10.0).abs() < 1e-6);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("paralel-kaydir-referans", reference_shown),
        ("paralel-kaydir-koru", keep_shown),
        ("paralel-kaydir-koru-yazildi", keep_written),
        ("paralel-kaydir-bag-izler", bond_follows),
    ]
}
