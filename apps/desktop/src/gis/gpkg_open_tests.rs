//! A GeoPackage opened as a project (docs/adr/9116): Aç on a file of no project opens a new project of its tables in
//! memory, not a byte of the file changed (docs/adr/9122); its first Kaydet shows the file and the name before it keeps
//! the project there, and writes there again afterwards; Vazgeç writes nothing and stops a “Kaydet ve kapat”; a file of
//! several projects opens Veri kaynağından aç on them; one dropped on a window with no drawing opens the same way; Veri
//! kaynağından aç on a file of no project makes one under the name typed. Its tile tables open as rasters linked to the
//! file, found again where the file is carried, and Kaydet gives a changed look to its table's default style
//! (docs/adr/9117).

use std::path::PathBuf;

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::opening::Purpose;

/// A case of fixtures/data/v1/gpkg-open copied into a scratch folder: a save writes into the file.
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

/// The project window's Kaydet pressed, as the first Kaydet of a project made of a file's tables opens it.
fn keep_in_window(app: &mut App) {
    let w = app.gis.project_store.as_ref().expect("Veri kaynağına kaydet, before anything is written");
    assert!(w.offered_for_test(), "opened by the first Kaydet");
    let task = app.project_store_event(crate::project_store::Event::Run);
    drive(app, task);
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
fn ac_on_a_file_of_no_project_opens_it_unchanged_and_the_first_kaydet_keeps_it_there_after_showing_where() {
    let path = fixture("katmanlar.gpkg", "gpkg-ac");
    let before = std::fs::read(&path).expect("the file");
    let mut app = empty_app();
    let task = app.start_opening(path.clone(), Purpose::File);
    drive(&mut app, task);
    let doc = app.document.as_ref().expect("opened");
    assert!(doc.stored_source().is_none(), "kept nowhere yet");
    let offered = doc.offered_source().expect("the file and name Kaydet offers");
    assert_eq!(offered.name, "katmanlar");
    assert_eq!(offered.file(), Some(path.clone()));
    assert!(doc.path.is_none() && !doc.dirty(), "no file of its own, nothing to save");
    assert_eq!(layer_names(&app), ["Taslak", "kuyular", "yollar", "binalar", "bos", "malikler"]);
    assert_eq!(doc.model.settings().srid, 5256);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|t| t.contains("“katmanlar.gpkg” içinde KentOS projesi yok")
            && t.contains("5 veri katmanı")
            && t.contains("Dosyaya bir şey yazılmadı")),
        "{said:#?}"
    );
    assert!(app.recent.list().iter().any(|r| r.path == path), "the file is a recent one");
    assert!(std::fs::read(&path).expect("the file") == before, "opening wrote nothing into the file");
    assert!(kept(&path).is_empty());

    // A change, then Kaydet: the window shows the file and the name; nothing is written before its Kaydet.
    let doc = app.document.as_mut().expect("the drawing");
    doc.model.add_layer_at(kentos_domain::NewLayer::layer("Notlar"), None, Some(0), Some("Katman ekle"), false).expect("a layer");
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    let w = app.gis.project_store.as_ref().expect("Veri kaynağına kaydet");
    assert_eq!((w.path_for_test(), w.name_for_test()), (path.display().to_string(), "katmanlar".to_owned()));
    assert!(std::fs::read(&path).expect("the file") == before, "still nothing written");
    keep_in_window(&mut app);
    let doc = app.document.as_ref().expect("the drawing");
    assert!(!doc.dirty(), "saved");
    assert_eq!(doc.stored_source().map(|s| s.name.as_str()), Some("katmanlar"), "kept in the file now");
    let first = kept(&path);
    assert_eq!(first.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["katmanlar"]);

    // Kaydet again writes there at once: no window.
    let doc = app.document.as_mut().expect("the drawing");
    doc.model.add_layer_at(kentos_domain::NewLayer::layer("Ekler"), None, Some(0), Some("Katman ekle"), false).expect("a layer");
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    assert!(app.gis.project_store.is_none() && app.document.as_ref().is_some_and(|d| !d.dirty()));
    let after = kept(&path);
    assert_eq!(after.len(), 1, "the same project, written over");
    assert_ne!(after[0].1, first[0].1, "with the new layer");

    // Opened again, the file gives that project, not a new one.
    let mut again = empty_app();
    let task = again.start_opening(path.clone(), Purpose::File);
    drive(&mut again, task);
    assert!(layer_names(&again).iter().any(|n| n == "Notlar"), "{:?}", layer_names(&again));
    assert_eq!(kept(&path).len(), 1);
}

#[test]
fn a_read_only_file_opens_and_closes_unchanged() {
    use std::os::unix::fs::PermissionsExt;
    let path = fixture("katmanlar.gpkg", "gpkg-salt");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).expect("read only");
    let before = std::fs::read(&path).expect("the file");
    let mut app = empty_app();
    let task = app.start_opening(path.clone(), Purpose::File);
    drive(&mut app, task);
    let opened = app.document.as_ref().is_some_and(|d| d.offered_source().is_some() && !d.dirty());
    let task = app.leave(crate::app::Then::CloseTab);
    drive(&mut app, task);
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
    assert!(opened, "a read-only file opens as any other");
    assert!(app.dialog.is_none() && app.document.is_none(), "closed without a question: nothing changed");
    assert!(std::fs::read(&path).expect("the file") == before);
}

#[test]
fn vazgec_in_the_first_kaydet_writes_nothing_and_kaydet_ve_kapat_closes_once_kept() {
    use crate::app::{Dialog, Then};
    let path = fixture("katmanlar.gpkg", "gpkg-kapat");
    let before = std::fs::read(&path).expect("the file");
    let mut app = empty_app();
    let task = app.start_opening(path.clone(), Purpose::File);
    drive(&mut app, task);
    let doc = app.document.as_mut().expect("opened");
    doc.model.add_layer_at(kentos_domain::NewLayer::layer("Notlar"), None, Some(0), Some("Katman ekle"), false).expect("a layer");
    // Çizimi kapat asks; Kaydet ve kapat opens the window; its Vazgeç writes nothing and the drawing stays.
    let task = app.leave(Then::CloseTab);
    drive(&mut app, task);
    assert!(matches!(app.dialog, Some(Dialog::Unsaved(Then::CloseTab))), "asked first");
    let task = app.update(Message::DialogSave);
    drive(&mut app, task);
    assert!(app.gis.project_store.is_some(), "the window shows where before writing");
    let _ = app.project_store_event(crate::project_store::Event::Close);
    assert!(app.save_then.is_none(), "the closing stopped with the window");
    assert!(app.document.as_ref().is_some_and(|d| d.dirty()), "the drawing stays with its change");
    assert!(std::fs::read(&path).expect("the file") == before, "nothing written");
    // Asked again, and this time kept: the drawing closes once its project is in the file.
    let task = app.leave(Then::CloseTab);
    drive(&mut app, task);
    let task = app.update(Message::DialogSave);
    drive(&mut app, task);
    keep_in_window(&mut app);
    assert!(app.document.is_none(), "closed once kept");
    assert_eq!(kept(&path).iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["katmanlar"]);
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
    assert_eq!(doc.offered_source().map(|s| s.name.as_str()), Some("sistemsiz"));
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
    assert_eq!(doc.offered_source().map(|s| s.name.as_str()), Some("Karma sistemler"), "Kaydet offers that name");
    assert!(kept(&path).is_empty(), "opening wrote nothing");
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
    assert_eq!(there.document.as_ref().and_then(|d| d.offered_source()).map(|s| s.name.as_str()), Some("ortofotolu"));
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
    keep_in_window(&mut app);
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
/// names another file to picture instead (a downloaded one); it is copied to a scratch folder first, though opening
/// writes nothing into it (docs/adr/9122).
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

/// The data layers' rows drawn (their reader wakes the window by `Message::DataReady`, which the pictures send by
/// hand), then the picture.
fn settled_picture(snapshot: &mut kentos_ui::snapshot::Snapshot, app: &mut App, file: &std::path::Path) {
    let mut update = |app: &mut App, message| {
        let task = app.update(message);
        drive(app, task);
    };
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
    snapshot.render(app.view(), &app.theme()).save(file).expect("writes the picture");
    println!("{}", file.display());
}

/// Pictures of a GeoPackage of no project opened and its first Kaydet (docs/adr/9122): the map with what the opening
/// said and where Kaydet would keep it, then Veri kaynağına kaydet on that file and name, light and dark, and in a
/// small window.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib gpkg_open_tests::offer_screens -- --ignored --nocapture"]
fn offer_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/gpkg-oneri-9122");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (size, tag) in [(Size::new(1440.0, 900.0), ""), (Size::new(1100.0, 650.0), "-kucuk")] {
        for (mode, suffix) in [("light", ""), ("dark", "-koyu")] {
            let path = fixture("katmanlar.gpkg", &format!("gpkg-oneri{tag}{suffix}"));
            let mut app = empty_app();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let task = app.start_opening(path.clone(), Purpose::File);
            drive(&mut app, task);
            settled_picture(&mut snapshot, &mut app, &out.join(format!("01-acildi{tag}{suffix}.png")));
            let doc = app.document.as_mut().expect("the drawing");
            doc.model.add_layer_at(kentos_domain::NewLayer::layer("Notlar"), None, Some(0), Some("Katman ekle"), false).expect("a layer");
            let task = app.update(Message::Run("file.save"));
            drive(&mut app, task);
            settled_picture(&mut snapshot, &mut app, &out.join(format!("02-kaydet-yeri-gosterir{tag}{suffix}.png")));
        }
    }
}
