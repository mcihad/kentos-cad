//! Ölçeklerin özeti in the status bar's scale menu (docs/adr/1231, plan/map-cad.md MAP-02): a map project on TUREF /
//! TM30 with a mean height gives the drawing scale, the screen, and at the screen's middle the projection's scale, the
//! height's factor and their product; a building on its own coordinates gives none; a GIS project's view scale menu
//! carries the same rows. And the menu's picture.

use kentos_interaction::Vec2;

use crate::app::App;
use crate::files_testing::{app_with_drawing, make_cad};

fn map_project(app: &mut App, srid: u32, height: Option<f64>) {
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.settings().clone();
    settings.srid = srid;
    settings.cad_work = Some(kentos_commands::CadWork::Map);
    settings.survey = height.map(|h| kentos_commands::SurveySettings { ground_height: Some(h), ..Default::default() });
    doc.model.set_settings(settings);
}

fn labels(app: &App) -> Vec<&'static str> {
    app.scale_summary().iter().map(|r| r.label).collect()
}

#[test]
fn a_map_project_shows_its_scales_apart_and_a_building_none() {
    let mut app = app_with_drawing();
    make_cad(&mut app);
    map_project(&mut app, 5254, Some(850.0));
    app.viewport.camera.center = Vec2::new(550_000.0, 4_540_000.0);
    assert_eq!(
        labels(&app),
        ["Çizim ölçeği", "Ekran", "Projeksiyon ölçeği", "Yükseklik çarpanı", "Birleşik çarpan", "Ölçülen uzunluklar"]
    );
    // PROJ: proj -V +proj=tmerc +lon_0=30 +k=1 +x_0=500000 +ellps=GRS80 at 550 000 / 4 540 000 → k = 1.00003076.
    let k = app.scale_summary().into_iter().find(|r| r.label == "Projeksiyon ölçeği").expect("k").value;
    assert!(k.starts_with("1.0000307") && k.ends_with("(+30.8 ppm)"), "{k}");
    // Zooming changes the screen's row only.
    let before: Vec<String> = app.scale_summary().iter().filter(|r| r.label != "Ekran").map(|r| r.value.clone()).collect();
    app.viewport.camera.scale *= 8.0;
    let after: Vec<String> = app.scale_summary().iter().filter(|r| r.label != "Ekran").map(|r| r.value.clone()).collect();
    assert_eq!(before, after, "the camera's zoom changes no factor");
    // A building on its own coordinates: no summary under the scale menu.
    map_project(&mut app, 0, None);
    assert!(app.scale_summary().is_empty());
}

/// The scale menu with its summary, light; `.run/gui-evidence/scale-summary-1231/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scale_summary_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::widget::{container, space};
    use iced::{Point, Size};
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/scale-summary-1231");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        map_project(&mut app, 5254, Some(850.0));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        app.viewport.camera.center = Vec2::new(550_000.0, 4_540_000.0);
        let summary = app.scale_summary();
        let size = Size::new(560.0, 620.0);
        for (what, whole) in [("menu", true), ("ozet", false)] {
            let stage = container(space()).width(size.width).height(size.height).style(kentos_ui::style::container::window);
            let rows = summary.clone();
            let menu = kentos_ui::widget::ContextMenu::controlled(
                stage,
                Some(Point::new(24.0, 24.0)),
                move |_| {
                    if whole {
                        crate::annotation_scale::plot_scale_menu(1000.0, kentos_commands::DrawingUnit::M, &rows)
                    } else {
                        crate::annotation_scale::summary_menu(&rows)
                    }
                },
                crate::app::Message::Opened(None),
            );
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let file = out.join(format!("olcek-{what}-{name}.png"));
            snapshot.render(menu.into(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
