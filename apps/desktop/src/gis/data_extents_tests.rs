//! Tümünü göster and Genel bakış on a data layer's rows as its source has them now (the CBS review's 7th and 8th items,
//! docs/adr/9327): a layer made empty keeps no box; a row written through its edit session is in the box its source
//! says, so Tümünü göster shows it and Genel bakış draws it. A lone row keeps the view's scale (no “1:0”); two rows are
//! fitted whole. The overview says why it is empty while the box is read, and that nothing is there when nothing is.

use kentos_data::{Driver, GeometryField, NewTable};
use kentos_native_application::data_write;

use super::Extent;
use crate::app::{App, Message};
use crate::data_layers_tests::node;
use crate::files_testing::{app_with_drawing, drive, last_said};

/// A CBS project of the sample drawing's system (EPSG:5256) whose objects are gone: its data layers alone.
fn data_only() -> App {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.workspace = Some(kentos_commands::Workspace::Gis);
    doc.model.set_settings(settings);
    doc.model.set_layer_locked("bina", false);
    let uids: Vec<String> = doc
        .model
        .entities_with_uids()
        .map(|(u, _)| u.to_string())
        .collect();
    crate::files_testing::command(
        &mut app,
        "cad.entities.delete",
        serde_json::json!({ "uids": uids }),
    );
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
        iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0)),
    )));
    app
}

/// An empty point table `yeni` of the project's system in a new GeoPackage, added as a data layer; its id.
fn empty_layer(app: &mut App, tag: &str) -> String {
    crate::data_layers::register();
    let path = crate::files_testing::scratch(tag).join("yeni.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let address = path.display().to_string();
    let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
    let mut tx = source.begin().expect("begin");
    let made = NewTable {
        name: "yeni".into(),
        title: None,
        description: None,
        key: "fid".into(),
        fields: Vec::new(),
        geometry: Some(GeometryField {
            name: "geom".into(),
            type_name: "POINT".into(),
            z: false,
            m: false,
            srid: Some(5256),
        }),
        spatial_index: true,
        system: None,
    };
    tx.create_table(&made, false).expect("made");
    tx.commit().expect("committed");
    app.add_data_layers(vec![
        crate::data_layers::describe(&address, "yeni").expect("described"),
    ])
    .expect("added");
    node(app, "yeni").expect("the layer").id.clone()
}

/// A point written on `layer` through its edit session and saved to its table, as Noktalar or a drawing tool does.
fn write_point(app: &mut App, layer: &str, x: f64, y: f64) {
    let _ = app.data_edits_event(crate::data_edits::Event::Start(layer.to_owned()));
    let input = serde_json::from_value(serde_json::json!({
        "layerId": layer,
        "objects": [{ "geometry": { "kind": "point", "p": { "x": x, "y": y } }, "attrs": {} }]
    }))
    .expect("an input");
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let made = kentos_native_application::create::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        input,
    );
    assert!(
        matches!(made, kentos_commands::CommandResult::Completed { .. }),
        "{made:?}"
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let writes = data_write::writes_of(doc, layer).expect("being edited");
    let outcome = data_write::write_group(std::slice::from_ref(&writes), doc.settings(), false)
        .map(|keys| (vec![writes], keys));
    assert!(outcome.is_ok(), "written");
    let _ = app.data_edits_event(crate::data_edits::Event::Saved(
        vec![layer.to_owned()],
        Box::new(outcome),
    ));
    assert!(
        app.document
            .as_ref()
            .is_some_and(|d| d.model.by_layer(layer).next().is_none()),
        "taken out of the drawing"
    );
}

/// Tümünü göster, its reads from the sources answered.
fn show_all(app: &mut App) {
    let task = app.update(Message::Run("view.zoomExtents"));
    drive(app, task);
}

fn centre(app: &App) -> (f64, f64) {
    let c = app.viewport.camera.world_center();
    (c.x, c.y)
}

#[test]
fn tumunu_goster_frames_the_rows_a_layer_made_empty_was_written_since() {
    let mut app = data_only();
    let layer = empty_layer(&mut app, "kapsam-yeni");
    let layer_node = node(&app, "yeni").expect("the layer").clone();
    assert_eq!(
        layer_node.data.as_ref().and_then(|d| d.extent),
        None,
        "made empty, it kept no box"
    );
    // Nothing yet: the table has no row.
    show_all(&mut app);
    assert_eq!(app.data_extent_of(&layer), Some(Extent::Read(None)));
    assert_eq!(last_said(&app), super::NOTHING_SHOWN);
    // A row written through the session: the source's box is the point, and the view goes there at its own scale.
    let scale = app.viewport.camera.scale;
    write_point(&mut app, &layer, 486_510.0, 4_420_210.0);
    assert_eq!(
        layer_node.data.as_ref().and_then(|d| d.extent),
        node(&app, "yeni").and_then(|n| n.data.as_ref()?.extent),
        "the layer keeps the box it had"
    );
    show_all(&mut app);
    assert_eq!(
        last_said(&app),
        super::FITTED,
        "it said it waited, and that the view is on the rows"
    );
    let (x, y) = centre(&app);
    assert!(
        (x - 486_510.0).abs() < 1e-6 && (y - 4_420_210.0).abs() < 1e-6,
        "on the point: {x} {y}"
    );
    assert_eq!(
        app.viewport.camera.scale, scale,
        "a lone point keeps the view's scale"
    );
    let words = crate::screen_scale::scale_words(app.view_scale());
    assert!(!words.contains(":0") && !words.starts_with("0:"), "{words}");
    // A second row: both in the view.
    write_point(&mut app, &layer, 486_910.0, 4_420_510.0);
    show_all(&mut app);
    let b = app.viewport.camera.visible_bounds();
    for (px, py) in [(486_510.0, 4_420_210.0), (486_910.0, 4_420_510.0)] {
        assert!(
            b.min_x < px && px < b.max_x && b.min_y < py && py < b.max_y,
            "{px} {py} in {b:?}"
        );
    }
    // Katmana yakınlaştır frames the same box.
    app.viewport.camera.zoom_at(0.01, 500.0, 350.0);
    app.zoom_to_layer(&layer);
    let b = app.viewport.camera.visible_bounds();
    assert!(b.min_x < 486_510.0 && 486_910.0 < b.max_x, "{b:?}");
}

/// Katmana yakınlaştır before the box is read (the second round's T16): offered on a data layer whatever its box; the
/// click waits for the box being read and frames it when it comes; a table without rows says so.
#[test]
fn katmana_yakinlastir_waits_for_the_box_and_says_why_there_is_none() {
    let mut app = data_only();
    let layer = empty_layer(&mut app, "kapsam-bekle");
    write_point(&mut app, &layer, 486_510.0, 4_420_210.0);
    write_point(&mut app, &layer, 486_910.0, 4_420_510.0);
    let n = node(&app, "yeni").expect("the layer").clone();
    // Its box not read since the rows were written, and the layer kept none: offered all the same.
    assert!(!app.data_box_below(&n), "no box yet");
    assert!(app.data_layer_below(&n), "the menu offers it");
    app.viewport.camera.zoom_at(0.01, 500.0, 350.0);
    app.zoom_to_layer(&layer);
    assert_eq!(
        last_said(&app),
        "“yeni” kapsamı kaynaktan okunuyor; gelince yakınlaştırılır."
    );
    let task = app.data_extents_tasks();
    drive(&mut app, task);
    let b = app.viewport.camera.visible_bounds();
    for (px, py) in [(486_510.0, 4_420_210.0), (486_910.0, 4_420_510.0)] {
        assert!(
            b.min_x < px && px < b.max_x && b.min_y < py && py < b.max_y,
            "{px} {py} in {b:?}"
        );
    }
    // A table without rows: said once its box is read, the view kept.
    let empty = empty_layer(&mut app, "kapsam-bos");
    let before = app.viewport.camera.visible_bounds();
    app.zoom_to_layer(&empty);
    let task = app.data_extents_tasks();
    drive(&mut app, task);
    assert_eq!(
        last_said(&app),
        "“yeni” tablosunda geometrili satır yok; yakınlaştırılacak kapsam yok."
    );
    assert_eq!(app.viewport.camera.visible_bounds(), before);
}

#[test]
fn genel_bakis_draws_a_data_only_project_and_says_why_it_is_empty() {
    let mut app = data_only();
    app.toggle_overview();
    assert_eq!(
        app.overview_words(),
        Some("Çizimde gösterilecek nesne yok."),
        "a project with nothing"
    );
    let layer = empty_layer(&mut app, "kapsam-genel");
    write_point(&mut app, &layer, 486_510.0, 4_420_210.0);
    write_point(&mut app, &layer, 486_910.0, 4_420_510.0);
    // The box is read off the thread: until it is in, the overview says so.
    app.overview_now();
    assert_eq!(
        app.overview_words(),
        Some("Veri katmanları kaynaklarından okunuyor…")
    );
    let task = app.data_extents_tasks();
    drive(&mut app, task);
    // The box in, the outline asked; once both are in, the picture holds the two rows in the layer's colour.
    app.overview_now();
    let task = app.data_extents_tasks();
    drive(&mut app, task);
    app.overview_now();
    assert_eq!(app.overview_words(), None, "a picture");
    let extent = app.overview_extent().flatten().expect("its extent");
    assert_eq!(extent, [486_510.0, 4_420_210.0, 486_910.0, 4_420_510.0]);
    for (x, y) in [(486_510.0, 4_420_210.0), (486_910.0, 4_420_510.0)] {
        let px = app
            .overview_pixel(kentos_render_wgpu::Vec2::new(x, y))
            .expect("in the picture");
        assert_eq!(px[3], 255, "the row's dot at {x} {y}: {px:?}");
    }
    // A press on the picture moves the main view there.
    let at = app
        .overview_point(kentos_render_wgpu::Vec2::new(486_910.0, 4_420_510.0))
        .expect("on the picture");
    app.navigation_event(crate::navigation_cards::Event::Press(at));
    let (x, y) = centre(&app);
    assert!(
        (x - 486_910.0).abs() < 5.0 && (y - 4_420_510.0).abs() < 5.0,
        "{x} {y}"
    );
    // The layer hidden: the overview says hidden layers are not shown.
    let _ = app.update(Message::LayerVisible(layer.clone()));
    assert!(node(&app, "yeni").is_some_and(|n| !n.visible), "hidden");
    app.overview_now();
    assert_eq!(
        app.overview_words(),
        Some("Görünen katmanlarda nesne yok; gizli katmanlar gösterilmez.")
    );
}

/// A data-only CBS project whose layer, made empty, had six rows written through its edit session, Tümünü göster run.
fn written_project() -> (App, String) {
    let mut app = data_only();
    let layer = empty_layer(&mut app, "resim-kapsam");
    for (x, y) in [
        (486_300.0, 4_420_100.0),
        (486_520.0, 4_420_260.0),
        (486_610.0, 4_420_080.0),
        (486_800.0, 4_420_420.0),
        (486_950.0, 4_420_300.0),
        (487_120.0, 4_420_520.0),
    ] {
        write_point(&mut app, &layer, x, y);
    }
    show_all(&mut app);
    (app, layer)
}

/// The pictures (`.run/gui-evidence/kullanim-9327/`): three linked views at the deepest zoom, the status bar, the badges
/// and the scale bar saying it alike; a data-only project's Genel bakış and Büyüteç on its rows; the overview of the
/// project with its layer hidden.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib data_extents::tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9327", "01-en-yakin-olcek", || {
        let mut app = app_with_drawing();
        // Sized first: the drawing's opening fit is spent, and the zoom below stays.
        let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1100.0, 650.0)),
        )));
        let _ = app.update(Message::Run("view.splitLink"));
        let _ = app.update(Message::Run("view.splitAdd"));
        let _ = app.update(Message::Viewport(crate::viewport::Event::Zoomed {
            factor: 1e12,
            at: iced::Point::new(200.0, 300.0),
        }));
        app
    });
    crate::files_testing::usage_shots_in("9327", "02-genel-bakis-ve-buyutec", |theme| {
        let (mut app, _) = written_project();
        // The overview's picture in the theme's colours: drawn after the theme is set.
        crate::files_testing::themed(&mut app, theme);
        app.toggle_overview();
        for _ in 0..2 {
            let task = app.data_extents_tasks();
            drive(&mut app, task);
            app.overview_now();
        }
        app.toggle_magnifier();
        // A little off the row, so the lens shows it beside its cross.
        let at = app
            .viewport
            .camera
            .world_to_screen(kentos_render_wgpu::Vec2::new(486_794.0, 4_420_416.0));
        let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(
            iced::Point::new(at[0] as f32, at[1] as f32),
        )));
        app
    });
    crate::files_testing::usage_shots_in("9327", "03-gizli-katman", |theme| {
        let (mut app, layer) = written_project();
        crate::files_testing::themed(&mut app, theme);
        let _ = app.update(Message::LayerVisible(layer));
        app.toggle_overview();
        app.overview_now();
        app
    });
}
