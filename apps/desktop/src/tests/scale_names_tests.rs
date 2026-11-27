//! Navigation's names and the two scales (docs/adr/9343; the CBS review of 27 November 2026, items 4 and 8).
//!
//! - Seçime yakınlaştır is Seçime sığdır; Katmana sığdır frames the layer chosen in Katmanlar (else the active one),
//!   and Önceki görünüm and Sonraki görünüm bring back the centre and the scale exactly.
//! - The inspector says the view's scale beside the drawing's, each by its name.
//! - A CBS import into the project says where its tables go.

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive};

fn run(app: &mut App, id: &'static str) {
    let task = app.update(Message::Run(id));
    drive(app, task);
}

fn camera(app: &App) -> (f64, f64, f64) {
    let c = &app.viewport.camera;
    (c.center.x, c.center.y, c.scale)
}

#[test]
fn katmana_sigdir_frames_the_layer_and_previous_brings_the_view_back() {
    let mut app = app_with_drawing();
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    run(&mut app, "view.zoomExtents");
    assert_eq!(crate::catalog::catalog().get("view.zoomSelection").map(|c| c.title), Some("Seçime sığdır"));
    assert_eq!(crate::catalog::catalog().get("view.zoomLayer").map(|c| c.title), Some("Katmana sığdır"));
    let before = camera(&app);
    app.selected_layer = Some("bina".to_owned());
    assert!(app.available("view.zoomLayer"));
    run(&mut app, "view.zoomLayer");
    let fitted = camera(&app);
    assert_ne!(fitted, before, "the view moved to the buildings");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let b = doc
        .entities()
        .filter(|e| e.base().layer_id == "bina")
        .map(|e| kentos_geometry_core::entity::entity_bounds(&kentos_native_application::geometry::shape(e)))
        .map(|b| [b.min_x, b.min_y, b.max_x, b.max_y])
        .reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
        .expect("the buildings");
    let (cx, cy) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
    assert!((fitted.0 - cx).abs() < 1e-6 && (fitted.1 - cy).abs() < 1e-6, "centred on the buildings: {fitted:?} {cx} {cy}");
    run(&mut app, "view.previous");
    assert_eq!(camera(&app), before, "Önceki görünüm: the centre and the scale as they were");
    run(&mut app, "view.next");
    assert_eq!(camera(&app), fitted, "Sonraki görünüm: the fitted view again");
    // An empty layer is not framed, said why.
    app.selected_layer = None;
    let doc = app.document.as_mut().expect("a drawing");
    let empty = doc.model.add_layer(kentos_domain::NewLayer::layer("boş"), None, false).expect("a layer");
    doc.model.set_active_layer(&empty);
    assert!(!app.available("view.zoomLayer"));
    assert!(app.why_disabled("view.zoomLayer").is_some_and(|w| w.contains("boş katman sığdırılmaz")));
}

#[test]
fn the_inspector_names_the_drawings_scale_and_the_views() {
    let mut app = app_with_drawing();
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    let doc = app.document.as_ref().expect("a drawing");
    let rows: Vec<(String, String)> = app
        .properties_panel(doc)
        .sections
        .into_iter()
        .filter(|s| s.id == "doc")
        .flat_map(|s| s.rows)
        .map(|r| (r.label.to_string(), r.value))
        .collect();
    let at = |l: &str| rows.iter().position(|(k, _)| k == l);
    assert_eq!(rows[at("Çizim ölçeği").expect("the drawing's")].1, "1:1000");
    let view = at("Görünüm ölçeği").expect("the view's");
    assert_eq!(view, at("Çizim ölçeği").expect("the drawing's") + 1, "beside it");
    assert_eq!(rows[view].1, crate::screen_scale::scale_words(app.view_scale()));
}

#[test]
fn an_import_says_where_a_cbs_projects_tables_go() {
    let mut app = app_with_drawing();
    let mut settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    settings.workspace = Some(kentos_commands::Workspace::Gis);
    app.document.as_mut().expect("a drawing").model.set_settings(settings);
    assert_eq!(app.cbs_data_place_words(), "projenin veri yerine (ilk Kaydet'te sorulan GeoPackage'a)");
}

/// Pictures for the owner (`.run/gui-evidence/kullanim-9343/`): Gezinme with Katmana sığdır and the inspector's two
/// scales; Etiket ▾'s switches with their icons beside their ticks.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    for theme in ["light", "dark"] {
        let mut app = app_with_drawing();
        crate::files_testing::themed(&mut app, theme);
        let mut settings = app.document.as_ref().expect("a drawing").model.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Gis);
        app.document.as_mut().expect("a drawing").model.set_settings(settings);
        let _ = app.update(Message::RibbonTab("map"));
        let mut shot = crate::files_testing::pictured(iced::Size::new(1440.0, 900.0));
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        shot.settle(&mut app, App::view, &mut update);
        crate::files_testing::evidence(&mut shot, &app, "kullanim-9343", &format!("01-gezinme-olcekler-{theme}"));
        crate::tools_screens::open_more(&mut shot, &mut app, "Etiket");
        crate::files_testing::evidence(&mut shot, &app, "kullanim-9343", &format!("02-etiket-menusu-{theme}"));
    }
}
