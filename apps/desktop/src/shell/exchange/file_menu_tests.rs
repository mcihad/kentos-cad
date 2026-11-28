//! The file and data menus' prerequisites (docs/adr/9342; the CBS review of 27 November 2026, items 3 and 9).
//!
//! - Seçilenleri kaydet is off without a selection, its tip saying why; a local project's cloud actions say that they
//!   are a cloud project's, apart from a missing session or a conflict-free sync.
//! - Paketle lists what the package copies before the folder is asked.
//! - Veri ekle's titled blocks read under their headers; a GeoPackage holding vectors and tiles shows both, the tiles a
//!   press away.

use super::source_export_tests::{qa_gpkg, scene};
use super::tests::{run, send};
use super::{Event, Window};
use crate::app::Picker;
use crate::files_testing::{app_with_drawing, scratch};

#[test]
fn save_selection_is_off_without_a_selection_and_says_why() {
    let mut app = app_with_drawing();
    assert!(!app.available("file.saveSelection"));
    assert!(
        app.why_disabled("file.saveSelection")
            .is_some_and(|w| w.starts_with("Önce kaydedilecek nesneleri seçin"))
    );
    let first = app.document.as_ref().and_then(|d| {
        d.model
            .entities()
            .next()
            .map(|e| kentos_domain::Slot(e.base().id))
    });
    app.selection.set(first.into_iter().collect::<Vec<_>>());
    assert!(app.available("file.saveSelection"));
    assert_eq!(app.why_disabled("file.saveSelection"), None);
}

#[test]
fn a_local_projects_cloud_actions_say_they_are_a_cloud_projects() {
    let app = app_with_drawing();
    for id in ["cloud.share", "cloud.history", "cloud.conflicts"] {
        assert!(!app.available(id), "{id}");
        let why = app.why_disabled(id).unwrap_or_default();
        assert!(why.starts_with("Yalnız bulut projesinde"), "{id}: {why}");
    }
}

#[test]
fn paketle_lists_what_the_package_copies_before_the_folder() {
    let dir = scratch("paket-onizleme");
    let mut app = scene(&qa_gpkg(&dir, "qa.gpkg"));
    run(&mut app, "file.package");
    let Some(Window::Package(s)) = &app.exchange else {
        panic!("Paketle's window");
    };
    let Some(Ok(preview)) = &s.preview else {
        panic!("the plan's lists: {:?}", s.preview);
    };
    assert!(
        preview
            .files
            .iter()
            .any(|(path, from)| path == "veri/qa.gpkg" && from.contains("qa.gpkg")),
        "{preview:?}"
    );
    // Bağlı bırak: the file stays, said.
    send(
        &mut app,
        Event::Package(super::package::Event::GeoPackages(super::package::Way(
            kentos_commands::PackageWay::Keep,
        ))),
    );
    let Some(Window::Package(s)) = &app.exchange else {
        panic!("Paketle's window");
    };
    let Some(Ok(preview)) = &s.preview else {
        panic!("the plan's lists");
    };
    assert!(
        preview.files.iter().all(|(path, _)| path != "veri/qa.gpkg"),
        "{preview:?}"
    );
    assert!(
        preview
            .left
            .iter()
            .any(|(path, why)| path.contains("qa.gpkg") && why == "bağlı kalır"),
        "{preview:?}"
    );
}

#[test]
fn a_menus_titled_blocks_read_under_their_headers() {
    let ids = ["a.one", "a.two", "b.one", "c.one"];
    let blocks = vec![
        ("Vektör", vec!["a.one", "a.two"]),
        ("Raster", vec!["b.one"]),
        ("Çizim", vec!["c.one"]),
    ];
    assert_eq!(
        crate::view::ribbon::block_heads(&ids, &blocks),
        [Some("Vektör"), None, Some("Raster"), Some("Çizim")]
    );
    // One titled block: no header.
    assert_eq!(
        crate::view::ribbon::block_heads(&ids[..2], &blocks[..1]),
        [None, None]
    );
}

#[test]
fn a_geopackage_of_vectors_and_tiles_shows_both() {
    crate::data_layers::register();
    let dir = scratch("gpkg-karisik");
    let path = dir.join("karisik.gpkg");
    std::fs::copy(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/gpkg-tiles/v1/orto.gpkg"),
        &path,
    )
    .expect("copied");
    {
        use kentos_data::Driver as _;
        let source = kentos_data_gpkg::Gpkg
            .open(path.to_str().expect("utf-8"))
            .expect("open");
        let mut tx = source.begin().expect("begin");
        tx.create_table(
            &kentos_data::NewTable {
                name: "yollar".into(),
                title: None,
                description: None,
                key: "fid".into(),
                fields: vec![kentos_data::Field::new(
                    "ad",
                    kentos_data::FieldKind::Text { max: Some(16) },
                )],
                geometry: Some(kentos_data::GeometryField {
                    name: "geom".into(),
                    type_name: "POINT".into(),
                    z: false,
                    m: false,
                    srid: Some(5256),
                }),
                spatial_index: true,
                system: None,
            },
            false,
        )
        .expect("table");
        tx.commit().expect("commit");
    }
    let mut app = app_with_drawing();
    app.picker = Picker::File(path.clone());
    run(&mut app, "file.import.gpkg");
    let Some(Window::Gpkg(s)) = &app.exchange else {
        panic!("GeoPackage's window");
    };
    assert_eq!(
        s.tables.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        ["yollar"]
    );
    assert_eq!(s.tiles.len(), 1, "{:?}", s.tiles);
    send(&mut app, Event::Gpkg(super::gpkg::Event::Tiles));
    assert!(
        matches!(app.exchange, Some(Window::GpkgTilesRead(_))),
        "the tiles' window on the same file"
    );
}

/// Pictures for the owner (`.run/gui-evidence/kullanim-9342/`): Veri ekle under its headers; Paketle's list.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    for theme in ["light", "dark"] {
        let mut app = app_with_drawing();
        crate::files_testing::themed(&mut app, theme);
        let mut settings = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .settings()
            .clone();
        settings.workspace = Some(kentos_commands::Workspace::Gis);
        app.document
            .as_mut()
            .expect("a drawing")
            .model
            .set_settings(settings);
        let _ = app.update(crate::app::Message::RibbonTab("map"));
        let mut shot = crate::files_testing::pictured(iced::Size::new(1440.0, 900.0));
        let mut update = |app: &mut crate::app::App, m: crate::app::Message| {
            let _ = app.update(m);
        };
        shot.settle(&mut app, crate::app::App::view, &mut update);
        crate::tools_screens::open_ribbon_menu(&mut shot, &mut app, "Veri ekle");
        crate::files_testing::evidence(
            &mut shot,
            &app,
            "kullanim-9342",
            &format!("01-veri-ekle-{theme}"),
        );
    }
    let dir = scratch("paket-onizleme-resim");
    let file = qa_gpkg(&dir, "qa.gpkg");
    crate::files_testing::usage_shots("9342", "02-paketle-liste", || {
        let mut app = scene(&file);
        run(&mut app, "file.package");
        app
    });
}
