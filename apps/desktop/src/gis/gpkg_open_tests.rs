//! A GeoPackage opened as a project (docs/adr/9116): Aç on a file of no project opens a new project of its tables,
//! written into the file, and Kaydet writes there again; a file of several projects opens Veri kaynağından aç on them;
//! one dropped on a window with no drawing opens the same way; Veri kaynağından aç on a file of no project makes one
//! under the name typed. Its tile tables open as rasters linked to the file, found again where the file is carried, and
//! Kaydet gives a changed look to its table's default style (docs/adr/9117).

use std::path::PathBuf;

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::opening::Purpose;

/// A case of fixtures/data/v1/gpkg-open copied into a scratch folder: opening writes into the file.
fn fixture(file: &str, tag: &str) -> PathBuf {
    let from = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/data/v1/gpkg-open").join(file);
    let to = crate::files_testing::scratch(tag).join(file);
    std::fs::copy(&from, &to).expect("the copy");
    to
}

fn empty_app() -> App {
    crate::data_layers::register();
    App::boot(None).0
}

fn layer_names(app: &App) -> Vec<String> {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model.layers().nodes().iter().map(|n| n.name.clone()).collect()
}

fn kept(path: &std::path::Path) -> Vec<(String, u64)> {
    let address = path.display().to_string();
    kentos_native_application::project_store::list(&address, None)
        .expect("listed")
        .into_iter()
        .map(|p| (p.name, p.size))
        .collect()
}

#[test]
fn ac_on_a_file_of_no_project_makes_one_inside_it_and_kaydet_writes_there() {
    let path = fixture("katmanlar.gpkg", "gpkg-ac");
    let mut app = empty_app();
    let task = app.start_opening(path.clone(), Purpose::File);
    drive(&mut app, task);
    let doc = app.document.as_ref().expect("opened");
    let stored = doc.stored_source().expect("the project kept in the file");
    assert_eq!(stored.name, "katmanlar");
    assert_eq!(stored.file(), Some(path.clone()));
    assert!(doc.path.is_none() && !doc.dirty(), "saved in the file, no file of its own");
    assert_eq!(layer_names(&app), ["Taslak", "kuyular", "yollar", "binalar", "bos", "malikler"]);
    assert_eq!(doc.model.settings().srid, 5256);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|t| t.contains("“katmanlar.gpkg” içinde KentOS projesi yoktu") && t.contains("5 veri katmanı")),
        "{said:#?}"
    );
    assert!(app.recent.list().iter().any(|r| r.path == path), "the file is a recent one");
    let before = kept(&path);
    assert_eq!(before.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["katmanlar"]);

    // A change, then Kaydet: written into the file again, the project bigger by a layer.
    let doc = app.document.as_mut().expect("the drawing");
    doc.model.add_layer_at(kentos_domain::NewLayer::layer("Notlar"), None, Some(0), Some("Katman ekle"), false).expect("a layer");
    assert!(app.document.as_ref().is_some_and(|d| d.dirty()));
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    assert!(app.document.as_ref().is_some_and(|d| !d.dirty()), "saved");
    let after = kept(&path);
    assert_eq!(after.len(), 1, "the same project, written over");
    assert_ne!(after[0].1, before[0].1, "with the new layer");

    // Opened again, the file gives that project, not a new one.
    let mut again = empty_app();
    let task = again.start_opening(path.clone(), Purpose::File);
    drive(&mut again, task);
    assert!(layer_names(&again).iter().any(|n| n == "Notlar"), "{:?}", layer_names(&again));
    assert_eq!(kept(&path).len(), 1);
}

#[test]
fn a_file_of_several_projects_opens_veri_kaynagindan_ac_on_them() {
    let path = fixture("iki-projeli.gpkg", "gpkg-iki");
    let mut app = empty_app();
    let task = app.start_opening(path.clone(), Purpose::File);
    drive(&mut app, task);
    assert!(app.document.is_none(), "none opened before one is chosen");
    let w = app.gis.project_store.as_ref().expect("Veri kaynağından aç");
    assert_eq!(w.path_for_test(), path.display().to_string());
    assert_eq!(w.listed_for_test(), Some(vec!["Çamlık".to_owned(), "Zeytinlik".to_owned()]));
}

#[test]
fn a_geopackage_dropped_on_an_empty_window_opens_as_a_project() {
    let path = fixture("sistemsiz.gpkg", "gpkg-birak");
    let mut app = empty_app();
    let task = app.update(crate::sources::msg(crate::sources::Event::Dropped(path.clone())));
    drive(&mut app, task);
    let doc = app.document.as_ref().expect("opened");
    assert_eq!(doc.stored_source().map(|s| s.name.as_str()), Some("sistemsiz"));
    assert_eq!(doc.model.settings().srid, 0, "no system, said");
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|t| t.contains("koordinat sistemi yok")), "{said:#?}");
}

#[test]
fn veri_kaynagindan_ac_on_a_file_of_no_project_makes_one_under_the_name_typed() {
    use crate::project_store::Event;
    let path = fixture("karisik.gpkg", "gpkg-pencere");
    let mut app = empty_app();
    let task = app.open_project_store(false);
    drive(&mut app, task);
    let _ = app.project_store_event(Event::Path(path.display().to_string()));
    let task = app.project_store_event(Event::List);
    drive(&mut app, task);
    let w = app.gis.project_store.as_ref().expect("the window");
    assert_eq!(w.listed_for_test(), Some(Vec::new()));
    assert_eq!(w.name_for_test(), "karisik", "the file's name offered");
    let _ = app.project_store_event(Event::Name("Karma sistemler".into()));
    let task = app.project_store_event(Event::Run);
    drive(&mut app, task);
    assert!(app.gis.project_store.is_none(), "the window closed");
    let doc = app.document.as_ref().expect("opened");
    assert_eq!(doc.stored_source().map(|s| s.name.as_str()), Some("Karma sistemler"));
    assert_eq!(kept(&path).iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["Karma sistemler"]);
}

/// The raster's file as the drawing holds it.
fn raster_file(app: &App) -> Option<String> {
    app.document.as_ref()?.model.entities().find_map(|e| match e {
        kentos_commands::Entity::Raster(r) => r.raster.file.clone(),
        _ => None,
    })
}

#[test]
fn ac_on_a_file_with_tiles_opens_them_as_a_raster_found_where_the_file_is_carried() {
    let path = fixture("ortofotolu.gpkg", "gpkg-orto");
    let mut app = empty_app();
    let task = app.start_opening(path.clone(), Purpose::File);
    drive(&mut app, task);
    assert_eq!(layer_names(&app), ["Taslak", "parseller", "ortofoto"], "the raster under the tables");
    let address = kentos_native_application::gpkg_open::address_of(&path);
    assert_eq!(raster_file(&app), Some(address), "linked to the file itself");
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|t| t.contains("(1 veri katmanı, döşeme tablolarından 1 raster)")), "{said:#?}");

    // The file carried to another folder and opened from there: the raster is read from where the file is now.
    let moved = crate::files_testing::scratch("gpkg-orto-tasindi").join("ortofotolu.gpkg");
    std::fs::copy(&path, &moved).expect("carried");
    let mut there = empty_app();
    let task = there.start_opening(moved.clone(), Purpose::File);
    drive(&mut there, task);
    assert_eq!(there.document.as_ref().and_then(|d| d.stored_source()).map(|s| s.name.as_str()), Some("ortofotolu"));
    assert_eq!(raster_file(&there), Some(kentos_native_application::gpkg_open::address_of(&moved)));
}

#[test]
fn kaydet_gives_a_changed_look_to_its_tables_default_style() {
    let path = fixture("katmanlar.gpkg", "gpkg-gorunus");
    let mut app = empty_app();
    let task = app.start_opening(path.clone(), Purpose::File);
    drive(&mut app, task);
    let doc = app.document.as_mut().expect("opened");
    let binalar = doc.model.layers().nodes().iter().find(|n| n.name == "binalar").map(|n| n.id.clone()).expect("binalar");
    let single = serde_json::json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
        { "id": "f", "type": "simpleFill", "color": "#2a6fdb" },
        { "id": "o", "type": "simpleLine", "color": "#123456", "width": 0.5 }
    ] } } });
    let done = kentos_native_application::layers_renderer::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        kentos_commands::LayersRenderer { layer: binalar, renderer: Some(single), expected_revision: None },
    );
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    assert!(app.document.as_ref().is_some_and(|d| d.dirty()));
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|t| t.contains("varsayılan stil olarak yazıldı: binalar.")), "{said:#?}");
    let defaults = |path: &std::path::Path| -> i64 {
        let gp = kentos_geopackage::GeoPackage::open(path, false).expect("the file");
        gp.connection()
            .query_row(
                "SELECT count(*) FROM layer_styles WHERE f_table_name = 'binalar' AND useAsDefault AND description = 'KentOS'",
                [],
                |r| r.get(0),
            )
            .expect("the style table")
    };
    assert_eq!(defaults(&path), 1, "binalar's default style is KentOS's look");

    // Kaydet again with the look unchanged: no new style.
    let rows = |path: &std::path::Path| -> i64 {
        let gp = kentos_geopackage::GeoPackage::open(path, false).expect("the file");
        gp.connection().query_row("SELECT count(*) FROM layer_styles", [], |r| r.get(0)).expect("the style table")
    };
    let before = rows(&path);
    let doc = app.document.as_mut().expect("the drawing");
    doc.model.add_layer_at(kentos_domain::NewLayer::layer("Notlar"), None, Some(0), Some("Katman ekle"), false).expect("a layer");
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    assert_eq!(rows(&path), before, "nothing more written");
}

#[test]
fn a_geopackage_in_degrees_fills_the_view_and_zooms_in_further() {
    use iced::{Point, Rectangle, Size};
    let path = fixture("derece.gpkg", "gpkg-derece");
    let mut app = empty_app();
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(Rectangle::new(
        Point::ORIGIN,
        Size::new(1000.0, 800.0),
    ))));
    let task = app.start_opening(path.clone(), Purpose::File);
    drive(&mut app, task);
    assert_eq!(app.document.as_ref().map(|d| d.model.settings().srid), Some(4326));
    // The home, 0.002° × 0.0024° (about 170 m × 270 m), fills the area within its margins: 800 − 2 × 48 pixels.
    let scale = app.viewport.camera.scale;
    assert!((0.0024 * scale - 704.0).abs() < 1e-6, "{} pixels for the home's height", 0.0024 * scale);
    // Zoomed in on: past a project of metres' 10 000 pixels a unit, a step at a time.
    app.zoom_in();
    assert!((app.viewport.camera.scale / scale - 1.5).abs() < 1e-9, "{} after {scale}", app.viewport.camera.scale);
}

/// Pictures of a GeoPackage in degrees opened (docs/adr/9117 §5): its home, then three and eleven steps closer
/// (a project of metres' deepest zoom, 0.1 mm a pixel, is far past both). `KENTOS_GPKG`
/// names another file to picture instead (a downloaded one); it is copied to a scratch folder first, since opening
/// writes the project into the file.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib gpkg_open_tests::screens -- --ignored --nocapture"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let (file, folder) = match std::env::var_os("KENTOS_GPKG").map(PathBuf::from) {
        Some(other) => {
            let to = crate::files_testing::scratch("gpkg-resim").join(other.file_name().expect("a file"));
            std::fs::copy(&other, &to).expect("the copy");
            let stem = to.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            (to, format!("gpkg-{stem}-9117"))
        }
        None => (fixture("derece.gpkg", "gpkg-derece-resim"), "gpkg-derece-9117".to_owned()),
    };
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence").join(folder);
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut app = empty_app();
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let task = app.update(message);
        drive(app, task);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let task = app.start_opening(file, Purpose::File);
    drive(&mut app, task);
    // The data layers' rows come from their reader's thread, which wakes the window (`Message::DataReady`, a
    // subscription the pictures do not run): woken by hand until each shown table has drawn parts and a read more.
    let mut picture = |app: &mut App, name: &str| {
        let tables: Vec<String> = app
            .document
            .as_ref()
            .map(|d| d.model.layers().leaves().iter().filter(|n| n.data.is_some()).map(|n| n.id.clone()).collect())
            .unwrap_or_default();
        let mut settled = 0;
        for _ in 0..200 {
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            if tables.iter().all(|id| !crate::data_view::service().parts(id).is_empty()) {
                settled += 1;
                if settled > 10 {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            update(app, Message::DataReady);
        }
        let file = out.join(name);
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    };
    picture(&mut app, "01-acildi.png");
    for _ in 0..3 {
        app.zoom_in();
    }
    picture(&mut app, "02-uc-adim-yakin.png");
    for _ in 0..8 {
        app.zoom_in();
    }
    picture(&mut app, "03-on-bir-adim-yakin.png");
}
