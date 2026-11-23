//! GeoPackage içe ve dışa aktar on the desktop (docs/adr/0502): the window lists the tables, takes the chosen ones as
//! one undo step through `file.import.gpkg`; a file in another system waits for Projenin sistemine çevir; Dışa aktar
//! writes through `file.export.gpkg`. Pictures: `gpkg-*` (`screens`, ignored).

use crate::gis;
use std::path::PathBuf;

use super::gpkg::Event as G;
use super::tests::{count, run, said, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, scratch};

mod delivery;
mod neighbours;
mod reconcile;
mod status;

use reconcile::reconcile_scene;

fn gpkg(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/geopackage/v1")
        .join(name)
}

fn window(app: &App) -> &super::gpkg::State {
    match &app.exchange {
        Some(Window::Gpkg(s)) => s,
        _ => panic!("the GeoPackage window"),
    }
}

#[test]
fn a_geopackage_goes_in_as_one_undo_step_with_its_tables_as_layers() {
    let mut app = app_with_drawing();
    let before = count(&app);
    app.picker = Picker::File(gpkg("qgis.gpkg"));
    run(&mut app, "file.import.gpkg");
    let s = window(&app);
    assert_eq!(s.tables.len(), 6);
    assert_eq!(s.tables.iter().find_map(|t| t.srid), Some(5256));
    assert!(s.chosen.iter().all(|c| *c));
    // Without the road table.
    send(&mut app, Event::Gpkg(G::Toggle(1)));
    send(&mut app, Event::Gpkg(G::Run));
    assert!(
        app.exchange.is_none(),
        "{:?}",
        app.exchange.as_ref().map(|_| "open")
    );
    assert_eq!(count(&app), before + 8);
    assert!(
        said(&app, "“qgis.gpkg” alındı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before);
}

/// QGIS's relation and default style come in with the tables (docs/adr/0627 §6, §7); the mapping and style tables
/// are not listed as layers, and the announcement counts them. The sample drawing's own “Parsel” layer takes the
/// parcels and keeps its look; a layer the import makes takes QGIS's.
#[test]
fn a_geopackage_brings_its_relation_and_qgis_style() {
    let parcels = |app: &App| {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .layers()
            .leaves()
            .into_iter()
            .find(|l| l.name == "Parsel")
            .map(|l| (l.id.clone(), l.style.clone()))
    };
    let mut app = app_with_drawing();
    let (id, look) = parcels(&app).expect("the sample's parcels");
    app.picker = Picker::File(gpkg("profil-qgis.gpkg"));
    run(&mut app, "file.import.gpkg");
    let names: Vec<&str> = window(&app)
        .tables
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, ["parsel", "malik"]);
    assert_eq!((window(&app).relations, window(&app).styles), (1, 1));
    send(&mut app, Event::Gpkg(G::Run));
    assert!(app.exchange.is_none());
    assert!(
        said(
            &app,
            "“profil-qgis.gpkg” alındı. 2 tablodan 6 nesne (3 geometrisiz kayıt), 1 ilişki;"
        ),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    assert_eq!(parcels(&app), Some((id.clone(), look)), "its own look kept");
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.model.settings().relations.len(), 1);
    // Without a layer of the table's name the import makes one, and it takes QGIS's style.
    let mut app = app_with_drawing();
    app.document
        .as_mut()
        .expect("a drawing")
        .model
        .rename_layer(&id, "Kadastro parseli");
    app.picker = Picker::File(gpkg("profil-qgis.gpkg"));
    run(&mut app, "file.import.gpkg");
    send(&mut app, Event::Gpkg(G::Run));
    assert!(
        said(
            &app,
            "“profil-qgis.gpkg” alındı. 2 tablodan 6 nesne (3 geometrisiz kayıt), 1 ilişki, 1 katman stili (QML);"
        ),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
}

#[test]
fn a_file_in_another_system_waits_for_the_move_to_be_asked() {
    let mut app = app_with_drawing();
    let before = count(&app);
    app.picker = Picker::File(gpkg("wgs84.gpkg"));
    run(&mut app, "file.import.gpkg");
    assert_eq!(window(&app).tables.iter().find_map(|t| t.srid), Some(4326));
    // İçe aktar is off until the move is asked; asked, the point goes into TM36.
    send(&mut app, Event::Gpkg(G::Run));
    assert_eq!(count(&app), before, "nothing moved silently");
    assert!(
        window(&app)
            .status
            .as_ref()
            .is_some_and(|(e, t)| *e && t.contains("reproject"))
    );
    send(&mut app, Event::Gpkg(G::Reproject));
    send(&mut app, Event::Gpkg(G::Run));
    assert_eq!(count(&app), before + 1);
}

#[test]
fn the_drawing_goes_out_as_a_geopackage() {
    let dir = scratch("gpkg-export");
    let mut app = app_with_drawing();
    app.picker = Picker::File(gpkg("qgis.gpkg"));
    run(&mut app, "file.import.gpkg");
    send(&mut app, Event::Gpkg(G::Run));
    let out = dir.join("teslim.gpkg");
    app.picker = Picker::File(out.clone());
    run(&mut app, "file.export.gpkg");
    send(&mut app, Event::GpkgOut(super::gpkg_out::Event::Save));
    assert!(
        out.exists(),
        "{:?}",
        app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
    assert!(
        said(&app, "“teslim.gpkg” yazıldı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
}

/// Whether a line says `part` anywhere.
fn says(app: &App, part: &str) -> bool {
    app.log.lines().any(|l| l.text.contains(part))
}

/// The tables after the first unticked: only the parcels taken.
fn parcels_only(app: &mut App) {
    for i in 1..window(app).tables.len() {
        send(app, Event::Gpkg(G::Toggle(i)));
    }
}

fn delete_first(app: &mut App, layer: &str) {
    let doc = app.document.as_mut().expect("a drawing");
    let slot = doc
        .model
        .by_layer_with_uids(layer)
        .next()
        .and_then(|(u, _)| doc.model.slot_of(u))
        .expect("an object");
    doc.model.remove(&[slot]);
}

#[test]
fn a_table_bound_to_a_layer_is_written_back_and_read_again() {
    let dir = scratch("gpkg-bound");
    let file = dir.join("kadastro.gpkg");
    std::fs::copy(gpkg("qgis.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    app.picker = Picker::File(file.clone());
    run(&mut app, "source.link");
    assert!(
        window(&app).bind,
        "Kaynağa bağla opens with the binding ticked"
    );
    parcels_only(&mut app);
    send(&mut app, Event::Gpkg(G::Run));
    assert!(app.exchange.is_none());
    assert!(
        said(&app, "“kadastro.gpkg” bağlandı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let link = app.bound_link("import-parsel").expect("bound");
    assert_eq!(link.table, "parsel");
    let node = app
        .document
        .as_ref()
        .and_then(|d| d.model.layers().get("import-parsel").cloned())
        .expect("the layer");
    assert!(format!("{:?}", app.layer_menu(&node, false)).contains("Kaynağa yaz"));
    // An edit written back: the file changes, the binding follows it.
    delete_first(&mut app, "import-parsel");
    let before = std::fs::read(&file).expect("read");
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(
        says(&app, "tablosuna yazıldı: 1 silindi"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    assert_ne!(std::fs::read(&file).expect("read"), before);
    assert_ne!(
        app.bound_link("import-parsel").expect("bound").revision,
        link.revision
    );
    // A reload over the layer's own edit asks first; Değişiklikleri at ve oku brings the table's rows back.
    delete_first(&mut app, "import-parsel");
    let _ = app.update(Message::Bound(crate::bound::Event::Read(Some(
        "import-parsel".into(),
    ))));
    assert_eq!(app.dialog, Some(crate::app::Dialog::SourceDiscard));
    let _ = app.update(Message::DialogConfirmed);
    assert!(
        says(&app, "tablosundan okundu: 1 eklendi"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    // Kaynak durumu (a window since docs/adr/0543) now has nothing waiting on the layer.
    let _ = app.update(Message::Run("source.status"));
    let row = app.source_status_row("import-parsel").expect("a row");
    assert_eq!(
        app.source_status_next(row),
        kentos_commands::SourceNext::None
    );
    let _ = app.update(Message::Gis(gis::Event::SourceStatus(
        crate::source_status::Event::Close,
    )));
    let _ = app.update(Message::Bound(crate::bound::Event::Unlink(Some(
        "import-parsel".into(),
    ))));
    assert!(app.bound_link("import-parsel").is_none());
}

#[test]
fn a_file_in_another_system_is_bound_only_when_the_move_is_asked() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(gpkg("wgs84.gpkg"));
    run(&mut app, "source.link");
    let before = count(&app);
    send(&mut app, Event::Gpkg(G::Run));
    assert_eq!(count(&app), before, "nothing moved silently");
    send(&mut app, Event::Gpkg(G::Reproject));
    send(&mut app, Event::Gpkg(G::Run));
    assert_eq!(count(&app), before + 1);
    let link = app.bound_link("import-yer").expect("bound");
    assert_eq!(link.srid, Some(4326), "the table keeps its own system");
}

#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").unwrap_or_default();
    let wanted = |name: &str| only.is_empty() || only.split(',').any(|w| name.contains(w));
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            if !wanted(name) {
                return;
            }
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            let file = out.join(format!("{name}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for mode in ["dark", "light"] {
            let fresh = || {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app
            };
            let mut app = fresh();
            app.picker = Picker::File(gpkg("qgis.gpkg"));
            run(&mut app, "file.import.gpkg");
            picture(&mut app, &format!("gpkg-{mode}-1-ice-aktar"));
            send(&mut app, Event::Gpkg(G::Run));
            picture(&mut app, &format!("gpkg-{mode}-3-alindi"));
            // Shapefile dışa aktar (docs/adr/0514): what each layer makes and what the format cannot hold, before
            // anything is written; Windows-1254; written to an archive and said.
            run(&mut app, "file.export.shp");
            picture(&mut app, &format!("shp-{mode}-1-pencere"));
            send(
                &mut app,
                Event::ShpExport(super::shp_export::Event::Encoding(super::shp_export::Code(
                    kentos_commands::ShpEncoding::Windows1254,
                ))),
            );
            picture(&mut app, &format!("shp-{mode}-2-1254"));
            let zip = scratch(&format!("shp-{mode}-{width}")).join("teslim.zip");
            send(
                &mut app,
                Event::ShpExport(super::shp_export::Event::Picked(Some(zip.clone()))),
            );
            for _ in 0..200 {
                if zip.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            picture(&mut app, &format!("shp-{mode}-3-yazildi"));
            let mut app = fresh();
            app.picker = Picker::File(gpkg("wgs84.gpkg"));
            run(&mut app, "file.import.gpkg");
            send(&mut app, Event::Gpkg(G::Reproject));
            picture(&mut app, &format!("gpkg-{mode}-2-cevir"));
            // Bağlı katman (docs/adr/0503): the window, the badge, the question over the layer's own edit.
            let mut app = fresh();
            app.picker = Picker::File(gpkg("qgis.gpkg"));
            run(&mut app, "source.link");
            picture(&mut app, &format!("gpkg-{mode}-4-bagla"));
            send(&mut app, Event::Gpkg(G::Run));
            picture(&mut app, &format!("gpkg-{mode}-5-bagli"));
            delete_first(&mut app, "import-parsel");
            let _ = app.update(Message::Bound(crate::bound::Event::Read(Some(
                "import-parsel".into(),
            ))));
            picture(&mut app, &format!("gpkg-{mode}-6-soru"));
            // Anahtar alanı (docs/adr/0636 §7): the bind window's key, then the key taken away from Kaynak ▸.
            {
                let mut app = fresh();
                app.picker = Picker::File(gpkg("qgis.gpkg"));
                run(&mut app, "source.link");
                parcels_only(&mut app);
                send(&mut app, Event::Gpkg(G::Key("ada".into())));
                picture(&mut app, &format!("gpkg-{mode}-14-anahtar"));
                send(&mut app, Event::Gpkg(G::Run));
                let _ = app.update(Message::Bound(crate::bound::Event::Key(
                    "import-parsel".into(),
                    None,
                )));
                picture(&mut app, &format!("gpkg-{mode}-15-anahtar-kalkti"));
            }
            // Paketle (docs/adr/0506): a real package of the drawing, its answer said.
            let _ = app.update(Message::DialogClosed);
            let out = scratch(&format!("gpkg-paket-{mode}-{width}")).join("teslim");
            let doc = app.document.as_ref().expect("a drawing");
            let input = kentos_commands::PackageProject {
                path: out.display().to_string(),
                name: Some("kadastro".into()),
                from: None,
                ..Default::default()
            };
            let planned = kentos_headless::package::plan(&doc.model, &input).expect("planned");
            kentos_headless::package::write(&planned).expect("packed");
            send(
                &mut app,
                Event::Package(super::package::Event::Done(Ok(Box::new((
                    planned.output,
                    planned.warnings,
                ))))),
            );
            picture(&mut app, &format!("gpkg-{mode}-7-paket"));
            // A part of the table (docs/adr/0510): Yalnız görünen alanın satırları over the first parcel's view.
            let mut app = fresh();
            // The first frames settle the window and its view; the view is brought to the parcel after them.
            let mut settled = Snapshot::new(Size::new(width, height)).expect("a renderer");
            settled.settle(&mut app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            view_first_parcel(&mut app);
            app.picker = Picker::File(gpkg("qgis.gpkg"));
            run(&mut app, "source.link");
            parcels_only(&mut app);
            send(&mut app, Event::Gpkg(G::Area));
            picture(&mut app, &format!("gpkg-{mode}-8-alan"));
            send(&mut app, Event::Gpkg(G::Run));
            picture(&mut app, &format!("gpkg-{mode}-9-alan-bagli"));
            // The relation and style profile (docs/adr/0627 §6, §7): QGIS's file taken with its relation and the
            // layers' default QGIS style (the sample's own “Parsel” layer renamed, so the import makes the parcels').
            let mut app = fresh();
            let doc = app.document.as_mut().expect("a drawing");
            let id = doc
                .model
                .layers()
                .leaves()
                .into_iter()
                .find(|l| l.name == "Parsel")
                .map(|l| l.id.clone())
                .expect("the sample's parcels");
            doc.model.rename_layer(&id, "Kadastro parseli");
            app.picker = Picker::File(gpkg("profil-qgis.gpkg"));
            run(&mut app, "file.import.gpkg");
            picture(&mut app, &format!("gpkg-{mode}-12-profil"));
            send(&mut app, Event::Gpkg(G::Run));
            picture(&mut app, &format!("gpkg-{mode}-13-profil-alindi"));
        }
    }
}

#[test]
fn the_bind_window_takes_a_key_and_kaynak_changes_it_as_an_undo_step() {
    // docs/adr/0636 §5, §7.
    let mut app = app_with_drawing();
    app.picker = Picker::File(gpkg("qgis.gpkg"));
    run(&mut app, "source.link");
    parcels_only(&mut app);
    send(&mut app, Event::Gpkg(G::Key(" ada ".into())));
    send(&mut app, Event::Gpkg(G::Run));
    let key = |app: &App| app.bound_link("import-parsel").and_then(|l| l.key);
    assert_eq!(
        key(&app).as_deref(),
        Some("ada"),
        "the window's key, trimmed"
    );
    let _ = app.update(Message::Bound(crate::bound::Event::Key(
        "import-parsel".into(),
        None,
    )));
    assert_eq!(key(&app), None);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    assert_eq!(doc.undo().as_deref(), Some("Bağın anahtarı: parsel"));
    assert_eq!(key(&app).as_deref(), Some("ada"), "undo gives the key back");
    // A field whose value repeats is refused and said; the key stays.
    let _ = app.update(Message::Bound(crate::bound::Event::Key(
        "import-parsel".into(),
        Some("tapulu".into()),
    )));
    assert_eq!(key(&app).as_deref(), Some("ada"));
}

/// The view brought to the first parcel only (docs/adr/0510's area).
fn view_first_parcel(app: &mut App) {
    app.viewport.camera.fit_size(
        kentos_render_wgpu::Vec2 {
            x: 487_020.0,
            y: 4_420_010.0,
        },
        60.0,
        40.0,
        0.0,
    );
}

#[test]
fn a_part_of_a_table_is_bound_from_the_view() {
    // docs/adr/0510: Yalnız görünen alanın satırları binds the rows in the view; the area kept in the binding.
    let mut app = app_with_drawing();
    view_first_parcel(&mut app);
    let b = app.viewport.camera.visible_bounds();
    assert!(b.max_x < 487_100.0 && b.min_x <= 487_000.0, "{b:?}");
    app.picker = Picker::File(gpkg("qgis.gpkg"));
    run(&mut app, "source.link");
    parcels_only(&mut app);
    send(&mut app, Event::Gpkg(G::Area));
    assert!(window(&app).area);
    send(&mut app, Event::Gpkg(G::Run));
    assert!(
        app.exchange.is_none(),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let link = app.bound_link("import-parsel").expect("bound");
    assert_eq!(link.extent, Some([b.min_x, b.min_y, b.max_x, b.max_y]));
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(
        doc.model.by_layer_with_uids("import-parsel").count(),
        1,
        "only the parcel in the view"
    );
}

/// Two layers of one file moved (docs/adr/0740 §2): relinking one relinks the other to the same new place, as QGIS's
/// and ArcGIS Pro's repair of a data source mends every layer of the moved file; one step each, said once.
#[test]
fn the_layers_of_a_moved_file_are_bound_again_together() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let dir = scratch("gpkg-relink-together");
    let file = dir.join("kadastro.gpkg");
    std::fs::copy(gpkg("qgis.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    for table in ["parsel", "nokta"] {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            "cad.sources.edit",
            CallOp::Execute,
            serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": table } }),
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let moved = dir.join("tasindi").join("kadastro.gpkg");
    std::fs::create_dir_all(moved.parent().expect("a folder")).expect("a folder");
    std::fs::rename(&file, &moved).expect("moved");
    let _ = app.update(Message::Bound(crate::bound::Event::Relinked(
        "kaynak-1".into(),
        Some(moved.clone()),
    )));
    for layer in ["import-parsel", "import-nokta"] {
        assert_eq!(
            app.bound_link(layer).expect("bound").path,
            moved.display().to_string(),
            "{layer}"
        );
    }
    assert!(
        says(&app, "ve aynı dosyaya bağlı 1 katman daha"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
}

#[test]
fn a_moved_file_is_bound_again_and_a_package_is_said() {
    // docs/adr/0506: Kaynak ▸ Yeniden bağla takes the same file in its new place, refuses another; Paketle's answer
    // is said with what it left.
    let dir = scratch("gpkg-relink");
    let file = dir.join("kadastro.gpkg");
    std::fs::copy(gpkg("qgis.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    app.picker = Picker::File(file.clone());
    run(&mut app, "source.link");
    parcels_only(&mut app);
    send(&mut app, Event::Gpkg(G::Run));
    let moved = dir.join("tasindi.gpkg");
    std::fs::rename(&file, &moved).expect("moved");
    app.picker = Picker::File(moved.clone());
    let task = app.update(Message::Bound(crate::bound::Event::Relink(Some(
        "import-parsel".into(),
    ))));
    drop(task);
    let _ = app.update(Message::Bound(crate::bound::Event::Relinked(
        "kaynak-1".into(),
        Some(moved.clone()),
    )));
    assert!(
        says(&app, "yeniden bağlandı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    assert_eq!(
        app.bound_link("import-parsel").expect("bound").path,
        moved.display().to_string()
    );
    let _ = app.update(Message::Bound(crate::bound::Event::Relinked(
        "kaynak-1".into(),
        Some(gpkg("kentos.gpkg")),
    )));
    assert!(
        says(&app, "son eşleştiği dosya değil"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let out = kentos_commands::ProjectPackaged {
        path: "/teslim/paket".into(),
        project: "/teslim/paket/proje.kcad".into(),
        project_bytes: 10,
        project_sha256: "0".repeat(64),
        files: Vec::new(),
        left: vec![kentos_commands::PackageLeft {
            kind: kentos_commands::PackagedKind::Picture,
            path: "/yok/kayip.jpg".into(),
            why: kentos_commands::LeftWhy::Missing,
            at: String::new(),
            objects: None,
        }],
        embedded: Vec::new(),
        manifest: "/teslim/paket/paket.json".into(),
        manifest_sha256: "0".repeat(64),
        faces: Vec::new(),
        resources: Vec::new(),
        exchange: None,
        bytes: 10,
    };
    let warning = kentos_commands::CommandWarning {
        code: "package_left".into(),
        message: "1 bağ pakete kopyalanmadı".into(),
        path: None,
    };
    send(
        &mut app,
        Event::Package(super::package::Event::Done(Ok(Box::new((
            out,
            vec![warning],
        ))))),
    );
    assert!(says(&app, "Paket yapıldı: /teslim/paket/proje.kcad"));
    assert!(says(&app, "1 bağ pakete kopyalanmadı"));
}

/// GIS-005 (docs/adr/1071): a curve polygon written by GDAL, its few-micrometre arc nearly straight, comes in at its
/// place in a TM33 CBS project; Tümünü göster frames the park, not hundreds of kilometres. With `KENTOS_SHOTS=1` the
/// picture goes to `.run/gui-evidence/gis-005/`.
#[test]
fn a_curve_polygons_near_straight_arc_stays_in_the_park() {
    use serde_json::json;
    let (mut app, _) = App::boot(None);
    crate::tools_scenes::open_edited(&mut app, crate::tools_scenes::Objects::new(), |d| {
        d["settings"]["workspace"] = json!("gis");
        d["settings"]["srid"] = json!(5255);
    });
    app.picker = Picker::File(gpkg("egri-tampon.gpkg"));
    run(&mut app, "file.import.gpkg");
    send(&mut app, Event::Gpkg(G::Run));
    let doc = &app.document.as_ref().expect("open").model;
    let area = doc
        .entities()
        .find(|e| e.kind() == "polygon")
        .expect("the buffer came in");
    let bulges: Vec<f64> = serde_json::to_value(area).expect("json")["bulges"]
        .as_array()
        .expect("arcs")
        .iter()
        .filter_map(serde_json::Value::as_f64)
        .collect();
    assert!(bulges.iter().all(|b| b.abs() < 0.5), "{bulges:?}");
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    let mut update = |app: &mut App, m| drop(app.update(m));
    shot.settle(&mut app, App::view, &mut update);
    let _ = app.update(Message::Run("view.zoomExtents"));
    shot.settle(&mut app, App::view, &mut update);
    // Framed, the park's 244 m fill the drawing area at a town's scale, not a country's.
    let width = app.viewport.camera.width / app.viewport.camera.map_px();
    assert!(width < 1_000.0, "the view is {width} m wide");
    crate::files_testing::evidence(&mut shot, &app, "gis-005", "egri-tampon-tumunu-goster");
}

/// A TM33 CBS project with the report's two-system file open in GeoPackage içe aktar (GIS-006).
fn two_systems() -> App {
    use serde_json::json;
    let (mut app, _) = App::boot(None);
    crate::tools_scenes::open_edited(&mut app, crate::tools_scenes::Objects::new(), |d| {
        d["settings"]["workspace"] = json!("gis");
        d["settings"]["srid"] = json!(5255);
    });
    app.picker = Picker::File(gpkg("cift-sistem.gpkg"));
    run(&mut app, "file.import.gpkg");
    app
}

/// The points of the layer the table `table` went onto, with the layer's recorded source system.
fn points_of(app: &App, table: &str) -> (Vec<(f64, f64)>, Option<u32>) {
    let doc = &app.document.as_ref().expect("open").model;
    let layer = doc
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == table)
        .cloned()
        .unwrap_or_else(|| panic!("the layer {table}"));
    let pts = doc
        .entities()
        .filter(|e| e.base().layer_id == layer.id)
        .map(|e| {
            let v = serde_json::to_value(e).expect("json");
            (v["p"]["x"].as_f64().unwrap(), v["p"]["y"].as_f64().unwrap())
        })
        .collect();
    (pts, layer.source_crs.and_then(|c| c.srid))
}

/// GIS-006 (docs/adr/1072): each table of a GeoPackage keeps its own system. Both tables chosen, the WGS 84 one asks
/// for the move and the TM33 one does not; moved, the WGS 84 trees land on the TM33 ones (pyproj 3.7, PROJ 9.8.1:
/// within a millimetre) and each layer records its table's system. The WGS 84 table alone asks for the move too.
#[test]
fn a_geopackages_tables_each_keep_their_own_system() {
    let mut app = two_systems();
    let s = window(&app);
    let systems: Vec<Option<u32>> = s.tables.iter().map(|t| t.srid).collect();
    assert_eq!(systems, [Some(5255), Some(4326)]);
    // Without the move the WGS 84 table cannot come in; asked, both do.
    send(&mut app, Event::Gpkg(G::Run));
    assert!(app.exchange.is_some(), "the import waits for the move");
    send(&mut app, Event::Gpkg(G::Reproject));
    send(&mut app, Event::Gpkg(G::Run));
    assert!(app.exchange.is_none(), "{:?}", app.log.lines().last().map(|l| l.text.clone()));
    let (a, a_crs) = points_of(&app, "A_TM33");
    let (b, b_crs) = points_of(&app, "B_WGS84");
    assert_eq!(a_crs, Some(5255));
    assert_eq!(b_crs, Some(4326));
    let reference = [
        (487_871.452_153_085_5, 4_422_114.306_225_088),
        (487_985.549_516_165_4, 4_422_175.158_152_065),
        (488_175.711_787_969_3, 4_422_205.584_115_55),
    ];
    for (got, want) in [(&a, &reference), (&b, &reference)] {
        assert_eq!(got.len(), 3);
        for (p, w) in got.iter().zip(want.iter()) {
            assert!((p.0 - w.0).abs() < 1e-3 && (p.1 - w.1).abs() < 1e-3, "{p:?} for {w:?}");
        }
    }
}

#[test]
fn the_wgs84_table_alone_asks_for_the_move() {
    let mut app = two_systems();
    send(&mut app, Event::Gpkg(G::Toggle(0)));
    send(&mut app, Event::Gpkg(G::Run));
    assert!(app.exchange.is_some(), "the WGS 84 table waits for the move");
    send(&mut app, Event::Gpkg(G::Reproject));
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    let mut update = |app: &mut App, m| drop(app.update(m));
    shot.settle(&mut app, App::view, &mut update);
    crate::files_testing::evidence(&mut shot, &app, "gis-006", "yalniz-wgs84");
    send(&mut app, Event::Gpkg(G::Run));
    assert!(app.exchange.is_none());
    let (b, _) = points_of(&app, "B_WGS84");
    assert_eq!(b.len(), 3);
}

/// Two parks, a road and three trees in a CBS project, the second park selected (GIS-007).
fn inventory() -> App {
    use serde_json::json;
    let (mut app, _) = App::boot(None);
    let mut o = crate::tools_scenes::Objects::new();
    o.path("parsel", &[[0.0, 0.0], [20.0, 0.0], [20.0, 15.0], [0.0, 15.0]], true);
    o.path("parsel", &[[30.0, 0.0], [50.0, 0.0], [50.0, 15.0], [30.0, 15.0]], true);
    o.line("yol", [-5.0, -5.0], [55.0, -5.0]);
    for x in [5.0, 25.0, 45.0] {
        o.point("agac", [x, -8.0], None);
    }
    crate::tools_scenes::open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        d["layers"] = json!([
            crate::tools_scenes::layer("parsel", "Park sınırları", "#3E63DD", 0.35),
            crate::tools_scenes::layer("yol", "Yürüyüş yolları", "#E5484D", 0.5),
            crate::tools_scenes::layer("agac", "Ağaçlar", "#30A46C", 0.25),
        ]);
        d["activeLayer"] = json!("parsel");
    });
    let doc = &app.document.as_ref().expect("open").model;
    let second = doc
        .entities_with_uids()
        .filter(|(_, e)| e.base().layer_id == "parsel")
        .filter_map(|(uid, _)| doc.slot_of(uid))
        .nth(1)
        .expect("the second park");
    app.selection.set([second]);
    app
}

/// The rows of each table the file at `path` holds, by table name.
fn rows_of(path: &std::path::Path) -> Vec<(String, usize)> {
    let read = kentos_native_application::geopackage::read(path, None).expect("the file reads");
    read.tables.iter().map(|t| (t.name.clone(), t.objects.len())).collect()
}

/// GIS-007 (docs/adr/1073): GeoPackage dışa aktar asks what goes in before the file, each scope counted; with one park
/// selected the file holds one park, the active layer both parks, the whole project every layer.
#[test]
fn the_geopackage_export_writes_the_scope_it_shows() {
    use super::gpkg_out::{Event as O, Scope};
    let dir = scratch("gpkg-scope");
    let mut app = inventory();
    run(&mut app, "file.export.gpkg");
    let Some(Window::GpkgOut(s)) = &app.exchange else {
        panic!("the export window");
    };
    assert_eq!(s.scope, Scope::Selection, "something is selected: the selection first");
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    let mut update = |app: &mut App, m| drop(app.update(m));
    shot.settle(&mut app, App::view, &mut update);
    crate::files_testing::evidence(&mut shot, &app, "gis-007", "secili-kapsam");
    for (scope, file, want) in [
        (Scope::Selection, "secili.gpkg", vec![("Park sınırları", 1)]),
        (Scope::Active, "etkin.gpkg", vec![("Park sınırları", 2)]),
        (Scope::All, "tum.gpkg", vec![("Park sınırları", 2), ("Yürüyüş yolları", 1), ("Ağaçlar", 3)]),
    ] {
        if app.exchange.is_none() {
            run(&mut app, "file.export.gpkg");
        }
        send(&mut app, Event::GpkgOut(O::Scope(scope)));
        let out = dir.join(file);
        app.picker = Picker::File(out.clone());
        send(&mut app, Event::GpkgOut(O::Save));
        assert!(app.exchange.is_none(), "{:?}", app.log.lines().last().map(|l| l.text.clone()));
        let mut got = rows_of(&out);
        got.sort();
        let mut want: Vec<(String, usize)> = want.into_iter().map(|(n, r)| (n.to_owned(), r)).collect();
        want.sort();
        assert_eq!(got, want, "{scope:?}");
    }
}

/// GIS-008 (docs/adr/1074): in a CBS project a table's declared type becomes its layer's: the curve polygon table an
/// area layer (the point tool off on it), the point tables point layers; kept after saving and opening again. A CAD
/// project's layers stay of any kind.
#[test]
fn a_geopackage_tables_declared_type_types_its_layer() {
    use kentos_commands::LayerGeometry;
    use serde_json::json;
    let open = |workspace: &str, file: &str| {
        let (mut app, _) = App::boot(None);
        crate::tools_scenes::open_edited(&mut app, crate::tools_scenes::Objects::new(), |d| {
            d["settings"]["workspace"] = json!(workspace);
            d["settings"]["srid"] = json!(5255);
        });
        app.picker = Picker::File(gpkg(file));
        run(&mut app, "file.import.gpkg");
        send(&mut app, Event::Gpkg(G::Reproject));
        send(&mut app, Event::Gpkg(G::Run));
        assert!(app.exchange.is_none(), "{:?}", app.log.lines().last().map(|l| l.text.clone()));
        app
    };
    let type_of = |app: &App, name: &str| {
        let doc = &app.document.as_ref().expect("open").model;
        doc.layers().leaves().into_iter().find(|l| l.name == name).and_then(|l| l.geometry)
    };
    let mut app = open("gis", "egri-tampon.gpkg");
    assert_eq!(type_of(&app, "Park koruma 10 m"), Some(LayerGeometry::Area));
    let doc = &mut app.document.as_mut().expect("open").model;
    let id = doc.layers().leaves().into_iter().find(|l| l.name == "Park koruma 10 m").map(|l| l.id.clone()).unwrap();
    assert!(doc.set_active_layer(&id));
    assert!(crate::layer_types::misfit(doc, "tool.point").is_some(), "the point tool is off");
    assert!(crate::layer_types::misfit(doc, "tool.polygon").is_none());
    let again = kentos_domain::Document::from_snapshot(doc.to_snapshot()).expect("opens again");
    assert_eq!(again.layers().get(&id).and_then(|l| l.geometry), Some(LayerGeometry::Area));
    let app = open("gis", "cift-sistem.gpkg");
    assert_eq!(type_of(&app, "A_TM33"), Some(LayerGeometry::Point));
    assert_eq!(type_of(&app, "B_WGS84"), Some(LayerGeometry::Point));
    let app = open("hybrid", "egri-tampon.gpkg");
    assert_eq!(type_of(&app, "Park koruma 10 m"), None);
}

/// GIS-010 (docs/adr/1076): a line layer with a required, named code field and no route yet is written as an empty
/// LINESTRING table with its column, so a field team enters the first route without making a column; left out when
/// the box is cleared.
#[test]
fn a_layers_empty_template_goes_out_as_an_empty_table() {
    use super::gpkg_out::Event as O;
    use serde_json::json;
    let dir = scratch("gpkg-template");
    let (mut app, _) = App::boot(None);
    let mut o = crate::tools_scenes::Objects::new();
    o.point("nokta", [0.0, 0.0], None);
    crate::tools_scenes::open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        d["settings"]["srid"] = json!(5255);
        let mut route = crate::tools_scenes::layer("rota", "Bakım rotaları", "#E5484D", 0.5);
        route["geometry"] = json!("line");
        route["fields"] = json!([{ "name": "rota_kodu", "alias": "Rota kodu", "kind": "text", "required": true }]);
        d["layers"] = json!([crate::tools_scenes::layer("nokta", "Saha noktaları", "#30A46C", 0.25), route]);
        d["activeLayer"] = json!("nokta");
    });
    run(&mut app, "file.export.gpkg");
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    let mut update = |app: &mut App, m| drop(app.update(m));
    shot.settle(&mut app, App::view, &mut update);
    crate::files_testing::evidence(&mut shot, &app, "gis-010", "bos-katman-sablonu");
    let out = dir.join("saha.gpkg");
    app.picker = Picker::File(out.clone());
    send(&mut app, Event::GpkgOut(O::Save));
    assert!(app.exchange.is_none(), "{:?}", app.log.lines().last().map(|l| l.text.clone()));
    let gp = kentos_geopackage::GeoPackage::open(&out, false).expect("opens");
    let layers = gp.layers().expect("tables");
    let route = layers.iter().find(|l| l.name == "Bakım rotaları").expect("the empty template");
    assert_eq!(route.geometry.as_ref().map(|g| g.type_name.as_str()), Some("LINESTRING"));
    assert!(route.columns.iter().any(|c| c.name == "rota_kodu"));
    assert_eq!(gp.count(route).expect("rows"), 0);
    drop(gp);
    // Read back, the template is a typed line layer with its field and no object.
    let read = kentos_native_application::geopackage::read(&out, None).expect("reads");
    let t = read.tables.iter().find(|t| t.name == "Bakım rotaları").expect("read");
    assert!(t.objects.is_empty());
    assert!(t.fields.iter().any(|f| f.name == "rota_kodu" && f.alias.as_deref() == Some("Rota kodu") && f.required));
    // The box cleared: only the layer with objects.
    run(&mut app, "file.export.gpkg");
    send(&mut app, Event::GpkgOut(O::Empty));
    let out = dir.join("yalniz-dolu.gpkg");
    app.picker = Picker::File(out.clone());
    send(&mut app, Event::GpkgOut(O::Save));
    let gp = kentos_geopackage::GeoPackage::open(&out, false).expect("opens");
    assert!(gp.layers().expect("tables").iter().all(|l| l.name != "Bakım rotaları"));
}

/// The area's length and area, each bulged edge its arc (geometry-core's own sums).
fn arc_measures(e: &kentos_commands::Entity) -> (f64, f64) {
    let kentos_commands::Entity::Polygon(p) = e else { panic!("an area: {}", e.kind()) };
    let bulges = p.bulges.as_deref();
    let pts: Vec<kentos_geometry_core::Vec2> = p.pts.iter().map(|q| kentos_geometry_core::Vec2::new(q.x, q.y)).collect();
    (
        kentos_geometry_core::geom::bulge::bulge_path_length(&pts, bulges, true),
        kentos_geometry_core::geom::bulge::bulge_ring_area(&pts, bulges).abs(),
    )
}

/// GIS-043 (docs/adr/1071, 1096): the park buffer of GIS-005 (a TM33 curve polygon, its arcs few micrometres to ten metres)
/// written to FlatGeobuf, GML and a CSV with WKT and read back by KentOS's own readers keeps its perimeter
/// 790.231 m and its area 38 956.519 m² (the 60-digit reference of scripts/fixtures/gpkg_arc_area.py) within the
/// millimetre and the square centimetre, every bulge below 0.5; the exchange check (GIS-039) keeps its curves' shape,
/// length and area for FlatGeobuf and GML.
#[test]
fn the_park_buffer_keeps_its_arcs_through_fgb_gml_and_csv() {
    use kentos_commands::{ExchangeFormat as F, ExchangeState as S};
    use serde_json::json;
    const AREA: f64 = 38_956.519_339_831;
    const PERIMETER: f64 = 790.231_084_908_664;
    let (mut app, _) = App::boot(None);
    crate::tools_scenes::open_edited(&mut app, crate::tools_scenes::Objects::new(), |d| {
        d["settings"]["workspace"] = json!("gis");
        d["settings"]["srid"] = json!(5255);
    });
    app.picker = Picker::File(gpkg("egri-tampon.gpkg"));
    run(&mut app, "file.import.gpkg");
    send(&mut app, Event::Gpkg(G::Run));
    let doc = &app.document.as_ref().expect("open").model;
    let buffer = doc.entities().find(|e| e.kind() == "polygon").expect("the buffer").clone();
    let (perimeter, area) = arc_measures(&buffer);
    assert!((perimeter - PERIMETER).abs() < 1e-3 && (area - AREA).abs() < 1e-4, "{perimeter} {area}");
    // The exchange check's round trips through FlatGeobuf and GML.
    let _ = app.update(Message::Run("file.exchangeCheck"));
    let _ = app.update(Message::ExchangeCheck(crate::exchange_check_window::Event::Matrix));
    let m = app.exchange_check.matrix.clone().expect("a matrix").expect("checked");
    for f in [F::Fgb, F::Gml] {
        for aspect in ["curves", "position", "shape", "length", "area"] {
            let cell = m
                .rows
                .iter()
                .find(|r| r.aspect == aspect)
                .and_then(|r| r.cells.iter().find(|c| c.format == f))
                .expect("a cell");
            assert_eq!(cell.state, S::Kept, "{f:?} {aspect}: {:?}", m.checks.iter().find(|c| c.format == f).map(|c| &c.numeric));
        }
    }
    // A CSV with WKT: written and read back by KentOS's own CSV writer and reader.
    let dir = crate::files_testing::scratch("park-csv");
    let path = dir.join("park.csv");
    let export: kentos_commands::ExportCsv =
        serde_json::from_value(json!({ "path": path.to_string_lossy() })).expect("the input");
    let doc = &app.document.as_ref().expect("open").model;
    let made = kentos_native_application::csv_table::export(doc, &export).expect("written");
    let import: kentos_commands::ImportCsv =
        serde_json::from_value(json!({ "path": path.to_string_lossy() })).expect("the input");
    let read = kentos_native_application::csv_table::read(&path, &made.csv, &import).expect("read back");
    let back = read.tables[0].objects.iter().map(|(_, e)| e).find(|e| e.kind() == "polygon").expect("the buffer back");
    let (perimeter, area) = arc_measures(back);
    assert!((perimeter - PERIMETER).abs() < 1e-3, "CSV perimeter {perimeter}");
    assert!((area - AREA).abs() < 1e-4, "CSV area {area}");
    let kentos_commands::Entity::Polygon(p) = back else { unreachable!() };
    assert!(p.bulges.iter().flatten().all(|b| b.abs() < 0.5), "{:?}", p.bulges);
}

/// GIS-047 (docs/adr/1104): with the link's key set (parsel_no), each conflict's row starts with the key and its value
/// and names its layer, so two parcels of one block with the same other values are told apart.
#[test]
fn a_conflict_s_row_names_the_key_and_the_layer() {
    let (mut app, _) = reconcile_scene("gpkg-uzlas-anahtar");
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut settings = doc.settings().clone();
    settings.sources[0].key = Some("parsel_no".into());
    doc.set_settings(settings);
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some("import-parsel".into()))));
    let titles = app.reconcile_titles();
    assert!(!titles.is_empty(), "the window with its conflicts");
    let layer = app
        .document
        .as_ref()
        .and_then(|d| d.model.layers().get("import-parsel").map(|n| n.name.clone()))
        .expect("the layer");
    assert!(titles.iter().any(|t| t.starts_with("parsel_no 6") && t.ends_with(&format!("({layer})"))), "{titles:?}");
    assert!(titles.iter().filter(|t| t.starts_with("parsel_no ")).count() >= 2, "{titles:?}");
}

/// Writing over a GeoPackage that is there (docs/adr/9168): the window says which of its tables go (another file's
/// table that this export does not write) before Üstüne yaz; Geri writes nothing; Üstüne yaz writes the new file.
#[test]
fn writing_over_a_geopackage_says_which_of_its_tables_go() {
    use kentos_data::Driver as _;
    let dir = scratch("gpkg-uzerine");
    let out = dir.join("var-olan.gpkg");
    // A file there with a table of its own.
    kentos_geopackage::GeoPackage::create(&out).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(out.to_str().expect("utf-8")).expect("open");
    crate::data_layers_tests::point_table(
        &*source,
        "eski_tablo",
        &[("ad", kentos_data::FieldKind::Text { max: Some(8) })],
        &[vec![kentos_data::Value::Text("A".into())]],
    );
    drop(source);
    kentos_data::registry().forget(out.to_str().expect("utf-8"));
    let mut app = app_with_drawing();
    app.picker = Picker::File(out.clone());
    run(&mut app, "file.export.gpkg");
    send(&mut app, Event::GpkgOut(super::gpkg_out::Event::Scope(super::gpkg_out::Scope::All)));
    send(&mut app, Event::GpkgOut(super::gpkg_out::Event::Save));
    let Some(super::Window::GpkgOut(state)) = &app.exchange else { panic!("the window") };
    let confirm = state.confirm.clone().expect("asked before writing over");
    assert_eq!(confirm.tables.as_deref(), Some(&["eski_tablo".to_owned()][..]));
    let lines: Vec<String> = app.gpkg_out_confirm_lines(state, &confirm).into_iter().map(|(_, l)| l).collect();
    assert!(lines.iter().any(|l| l == "Dosyadan silinen tablolar (1): eski_tablo."), "{lines:#?}");
    assert!(lines.iter().any(|l| l.starts_with("Yeni tablolar: ")), "{lines:#?}");
    assert!(lines.iter().any(|l| l.starts_with("Yazılan: ")), "{lines:#?}");
    // Geri: the file as it was.
    let before = std::fs::read(&out).expect("the file");
    send(&mut app, Event::GpkgOut(super::gpkg_out::Event::Back));
    assert_eq!(std::fs::read(&out).expect("the file"), before);
    // Üstüne yaz: the new file, without the old table.
    send(&mut app, Event::GpkgOut(super::gpkg_out::Event::Save));
    send(&mut app, Event::GpkgOut(super::gpkg_out::Event::Confirm));
    assert!(app.exchange.is_none(), "{:?}", app.log.lines().last().map(|l| l.text.clone()));
    let gp = kentos_geopackage::GeoPackage::open(&out, false).expect("opens");
    let names: Vec<String> = gp.layers().expect("tables").into_iter().map(|t| t.name).collect();
    assert!(!names.contains(&"eski_tablo".to_owned()), "{names:?}");
    assert!(!names.is_empty());
    drop(gp);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The picture (`.run/gui-evidence/kullanim-9168/`): writing over a GeoPackage that is there, asked.
#[test]
#[ignore = "pictures for the owner"]
fn writing_over_screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    use kentos_data::Driver as _;
    crate::files_testing::usage_shots("9168", "01-uzerine-yaz", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = scratch(&format!("gpkg-uzerine-resim-{n}"));
        let out = dir.join("teslim.gpkg");
        kentos_geopackage::GeoPackage::create(&out).expect("a file");
        let source = kentos_data_gpkg::Gpkg.open(out.to_str().expect("utf-8")).expect("open");
        crate::data_layers_tests::point_table(
            &*source,
            "eski_tablo",
            &[("ad", kentos_data::FieldKind::Text { max: Some(8) })],
            &[vec![kentos_data::Value::Text("A".into())]],
        );
        drop(source);
        kentos_data::registry().forget(out.to_str().expect("utf-8"));
        let mut app = app_with_drawing();
        app.picker = Picker::File(out);
        run(&mut app, "file.export.gpkg");
        send(&mut app, Event::GpkgOut(super::gpkg_out::Event::Scope(super::gpkg_out::Scope::All)));
        send(&mut app, Event::GpkgOut(super::gpkg_out::Event::Save));
        app
    });
}
