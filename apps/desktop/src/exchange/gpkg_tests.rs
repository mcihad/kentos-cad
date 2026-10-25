//! GeoPackage içe ve dışa aktar on the desktop (docs/adr/0502): the window lists the tables, takes the chosen ones as
//! one undo step through `file.import.gpkg`; a file in another system waits for Projenin sistemine çevir; Dışa aktar
//! writes through `file.export.gpkg`. Pictures: `gpkg-*` (`screens`, ignored).

use std::path::PathBuf;

use super::gpkg::Event as G;
use super::tests::{count, run, said, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, scratch};

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
    assert_eq!(s.srid, Some(5256));
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
            "“profil-qgis.gpkg” alındı. 2 tablodan 6 nesne (3 geometrisiz kayıt), 1 ilişki, 1 QGIS stili;"
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
    assert_eq!(window(&app).srid, Some(4326));
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
    let _ = app.update(Message::SourceStatus(crate::source_status::Event::Close));
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

/// A part of komsu.gpkg (docs/adr/0531) bound over a copy, parcel A's bump stretched a metre east; the rest of the
/// table (B, C outside the area) bound as a second layer once written, to show where it went.
fn neighbours_scene(name: &str) -> (App, PathBuf) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let file = scratch(name).join("komsu.gpkg");
    std::fs::copy(gpkg("komsu.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut run_on = |command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer["output"].clone()
    };
    run_on(
        "cad.sources.edit",
        serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": "parsel",
                                      "extent": [486990.0, 4419990.0, 487009.0, 4420030.0] } }),
    );
    let a = doc
        .model
        .by_layer_with_uids("import-parsel")
        .next()
        .map(|(u, _)| u.to_string())
        .expect("A");
    let mut run_on = |command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    };
    run_on(
        "cad.entities.stretch",
        serde_json::json!({ "window": { "min": { "x": 487011.5, "y": 4420004.5 },
                                        "max": { "x": 487012.5, "y": 4420005.5 } },
                            "dx": 1.0, "dy": 0.0, "uids": [a] }),
    );
    (app, file)
}

#[test]
fn a_part_writes_its_neighbours_while_topology_is_on() {
    // docs/adr/0531: Kaynağa yaz on a part with Topoloji on follows the neighbours outside the area (strict).
    let (mut app, file) = neighbours_scene("gpkg-komsu");
    let _ = app
        .settings
        .choose(&[("drafting.topology", serde_json::Value::Bool(true))]);
    app.apply_settings();
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(
        says(&app, "1 değişti, 2 alanın dışındaki komşu izledi"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    // The whole table read back: every parcel has the bump a metre east.
    let mut other = app_with_drawing();
    let whole = &mut other.document.as_mut().expect("a drawing").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(whole),
        "file.import.gpkg",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "path": file.display().to_string(), "tables": ["parsel"] }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let layer = answer["output"]["tables"][0]["layer"]
        .as_str()
        .unwrap_or_else(|| panic!("{answer}"))
        .to_owned();
    let shapes: Vec<String> = whole
        .by_layer_with_uids(&layer)
        .map(|(_, e)| serde_json::to_string(e).expect("entity"))
        .collect();
    let moved = whole
        .by_layer_with_uids(&layer)
        .filter(|(_, e)| {
            let v = serde_json::to_value(e).expect("entity");
            v["pts"].as_array().is_some_and(|pts| {
                pts.iter()
                    .any(|p| p["x"] == 487_013.0 && p["y"] == 4_420_005.0)
            })
        })
        .count();
    assert_eq!(moved, 3, "{answer} {shapes:?}");
    // The menu of a part has the loose write too.
    let node = app
        .document
        .as_ref()
        .and_then(|d| d.model.layers().get("import-parsel").cloned())
        .expect("the layer");
    assert!(format!("{:?}", app.layer_menu(&node, false)).contains("Kaynağa yaz, komşular gevşek"));
}

#[test]
#[ignore = "pictures, run by hand"]
fn neighbour_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        for mode in ["dark", "light"] {
            let (mut app, file) = neighbours_scene(&format!("gpkg-komsu-{mode}-{width}"));
            let _ = app.settings.choose(&[
                ("appearance.theme", serde_json::Value::from(mode)),
                ("drafting.topology", serde_json::Value::Bool(true)),
            ]);
            app.apply_settings();
            let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
                "import-parsel".into(),
            ))));
            // The file as it now is in place of the part: the binding undone, A taken out, the whole table taken in.
            let doc = app.document.as_mut().expect("a drawing");
            let a: Vec<String> = doc
                .model
                .by_layer_with_uids("import-parsel")
                .map(|(u, _)| u.to_string())
                .collect();
            for (command, input) in [
                (
                    "cad.sources.edit",
                    serde_json::json!({ "edit": { "kind": "unlink", "id": "kaynak-1" } }),
                ),
                ("cad.entities.delete", serde_json::json!({ "uids": a })),
            ] {
                let answer = kentos_native_application::registry::run(
                    &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                    command,
                    kentos_commands::CallOp::Execute,
                    input,
                )
                .expect("runs");
                assert_eq!(answer["status"], "completed", "{answer}");
            }
            let answer = kentos_native_application::registry::run(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                "file.import.gpkg",
                kentos_commands::CallOp::Execute,
                serde_json::json!({ "path": file.display().to_string(), "tables": ["parsel"],
                                    "layers": { "parsel": "import-parsel" } }),
            )
            .expect("runs");
            assert_eq!(answer["status"], "completed", "{answer}");
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            // The first frames settle the window and its view; the view is brought to the parcels after them.
            let slots: Vec<f64> = {
                let doc = app.document.as_ref().expect("a drawing");
                doc.model
                    .by_layer_with_uids("import-parsel")
                    .filter_map(|(u, _)| doc.model.slot_of(u))
                    .map(|s| f64::from(s.0))
                    .collect()
            };
            app.zoom_to_objects(&slots);
            // Selected, so the parcels are seen with their shared bump where A put it.
            let picked: Vec<kentos_domain::Slot> = slots
                .iter()
                .map(|s| kentos_domain::Slot(*s as u32))
                .collect();
            app.selection.set(picked);
            snapshot.settle(&mut app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let _ = snapshot.render(app.view(), &app.theme());
            let path = out.join(format!("gpkg-{mode}-10-komsular-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        }
    }
}

/// komsu-gecerlilik.gpkg (docs/adr/0585) bound by A's area over a copy, the rows outside the area drawn beside it on the
/// sample's Parsel layer to see them, and A's corner (10, 4) pulled out east where B would cross itself.
fn validity_scene(name: &str) -> (App, String) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let file = scratch(name).join("komsu-gecerlilik.gpkg");
    std::fs::copy(gpkg("komsu-gecerlilik.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let path = file.display().to_string();
    let run_on = |doc: &mut kentos_domain::Document, command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer["output"].clone()
    };
    // The sample drawing's Parsel layer, on screen already.
    let table = "parsel".to_owned();
    assert!(
        doc.model.layers().get(&table).is_some(),
        "the sample's Parsel layer"
    );
    // The table's rows outside the area, drawn plainly beside the part (the file's own ids are the part's).
    for ring in [
        &[(10.0, 0.0), (20.0, 0.0), (20.0, 4.0), (10.0, 4.0)][..],
        &[(10.0, 6.0), (14.0, 8.0), (10.0, 10.0)][..],
        &[(10.0, 10.0), (20.0, 20.0), (20.0, 10.0), (10.0, 20.0)][..],
    ] {
        let pts: Vec<serde_json::Value> = ring
            .iter()
            .map(|(x, y)| serde_json::json!({ "x": 487000.0 + x, "y": 4420000.0 + y }))
            .collect();
        run_on(
            &mut doc.model,
            "cad.polygon.create",
            serde_json::json!({ "layerId": table, "pts": pts }),
        );
    }
    run_on(
        &mut doc.model,
        "cad.sources.edit",
        serde_json::json!({ "edit": { "kind": "link", "path": path, "table": "parsel",
                                      "extent": [486990.0, 4419990.0, 487005.0, 4420030.0] } }),
    );
    let a = doc
        .model
        .by_layer_with_uids("import-parsel")
        .next()
        .map(|(u, _)| u.to_string())
        .expect("A");
    run_on(
        &mut doc.model,
        "cad.entities.stretch",
        serde_json::json!({ "window": { "min": { "x": 487009.5, "y": 4420003.5 },
                                        "max": { "x": 487010.5, "y": 4420004.5 } },
                            "dx": 15.0, "dy": -2.0, "uids": [a] }),
    );
    (app, table)
}

#[test]
fn a_neighbour_left_invalid_stops_a_strict_write_and_says_so() {
    let (mut app, _) = validity_scene("gpkg-gecerlilik-test");
    let _ = app
        .settings
        .choose(&[("drafting.topology", serde_json::Value::Bool(true))]);
    app.apply_settings();
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    let said = crate::files_testing::last_said(&app);
    assert!(
        said.contains("1 komşu kayıt") && said.contains("geçersiz kalırdı"),
        "{said}"
    );
    // Loose: A is written, B kept as it is and said.
    let _ = app.update(Message::Bound(crate::bound::Event::WriteLoose(Some(
        "import-parsel".into(),
    ))));
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("olduğu gibi kaldı"), "{said}");
}

/// The strict write refused over the neighbour B would leave crossing itself, with the whole table beside it,
/// `.run/shots/gpkg-*-11-gecerlilik-*`:
///
/// ```text
/// cargo test -p kentos-desktop exchange::gpkg_tests::validity_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn validity_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        for mode in ["dark", "light"] {
            let (mut app, table) = validity_scene(&format!("gpkg-gecerlilik-{mode}-{width}"));
            let _ = app.settings.choose(&[
                ("appearance.theme", serde_json::Value::from(mode)),
                ("drafting.topology", serde_json::Value::Bool(true)),
            ]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            // The part's A and the rows drawn beside it (the sample's own parcel left out), selected: the
            // scene's direct edits show through the selection.
            let picked: Vec<kentos_domain::Slot> = {
                let doc = app.document.as_ref().expect("a drawing");
                doc.model
                    .by_layer_with_uids(&table)
                    .chain(doc.model.by_layer_with_uids("import-parsel"))
                    .filter(|(_, e)| e.base().label.is_none())
                    .filter_map(|(u, _)| doc.model.slot_of(u))
                    .collect()
            };
            let slots: Vec<f64> = picked.iter().map(|s| f64::from(s.0)).collect();
            app.zoom_to_objects(&slots);
            app.selection.set(picked);
            let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
                "import-parsel".into(),
            ))));
            snapshot.settle(&mut app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let path = out.join(format!("gpkg-{mode}-11-gecerlilik-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        }
    }
}

/// uzlas.gpkg (docs/adr/0538) bound over a copy, the layer edited as the shared cases edit it, then the copy changed
/// as QGIS changed it (uzlas-qgis.gpkg): both sides changed since they last matched.
fn reconcile_scene(name: &str) -> (App, PathBuf) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let file = scratch(name).join("uzlas.gpkg");
    std::fs::copy(gpkg("uzlas.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let run_on = |doc: &mut kentos_domain::Document, command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    };
    run_on(
        doc,
        "cad.sources.edit",
        serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": "parsel" } }),
    );
    let uid = |doc: &kentos_domain::Document, n: u32| {
        doc.by_layer_with_uids("import-parsel")
            .find(|(_, e)| e.base().attrs.get("parsel_no") == Some(&n.to_string()))
            .map(|(u, _)| u.to_string())
            .expect("a parcel")
    };
    for (n, attrs) in [
        (4, serde_json::json!({ "malik": "Veli" })),
        (5, serde_json::json!({ "parsel_no": "50" })),
        (6, serde_json::json!({ "malik": "Mehmet" })),
        (8, serde_json::json!({ "malik": "Selin" })),
        (10, serde_json::json!({ "malik": "Ortak" })),
    ] {
        let u = uid(doc, n);
        run_on(
            doc,
            "cad.entities.set",
            serde_json::json!({ "uids": [u], "attrs": attrs, "operation": "attributes" }),
        );
    }
    for n in [7, 9] {
        let u = uid(doc, n);
        run_on(
            doc,
            "cad.entities.delete",
            serde_json::json!({ "uids": [u] }),
        );
    }
    run_on(
        doc,
        "cad.polygon.create",
        serde_json::json!({ "layerId": "import-parsel",
                            "pts": [{ "x": 487400.0, "y": 4420000.0 }, { "x": 487420.0, "y": 4420000.0 },
                                    { "x": 487420.0, "y": 4420020.0 }, { "x": 487400.0, "y": 4420020.0 }] }),
    );
    std::fs::copy(gpkg("uzlas-qgis.gpkg"), &file).expect("changed in QGIS");
    (app, file)
}

#[test]
fn a_write_over_a_changed_file_opens_the_reconcile_and_puts_both_sides_together() {
    use crate::source_reconcile::Event as R;
    use kentos_commands::ReconcileSide;
    let (mut app, file) = reconcile_scene("gpkg-uzlas");
    let before = std::fs::read(&file).expect("read");
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(
        app.reconcile.is_some(),
        "the refused write opens the window"
    );
    assert_eq!(
        std::fs::read(&file).expect("read"),
        before,
        "nothing written over"
    );
    // Uzlaştır waits for every conflict's side.
    let view = format!("{:?}", app.reconcile);
    assert!(
        view.contains("RemovedInLayer") && view.contains("RemovedInTable"),
        "{view}"
    );
    let _ = app.update(Message::Reconcile(R::All(ReconcileSide::Layer)));
    let _ = app.update(Message::Reconcile(R::Apply));
    assert!(app.reconcile.is_none(), "closed when done");
    assert!(
        says(&app, "tablosuyla uzlaştırıldı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    assert_ne!(std::fs::read(&file).expect("read"), before, "written");
    // The two match now: Kaynağa yaz has nothing to write.
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(
        says(&app, "yazılacak bir şey yok"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
}

#[test]
#[ignore = "pictures, run by hand"]
fn reconcile_screens() {
    use crate::source_reconcile::Event as R;
    use iced::Size;
    use kentos_commands::ReconcileSide;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let (mut app, _file) = reconcile_scene(&format!("gpkg-uzlas-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let _ = snapshot.render(app.view(), &app.theme());
            let path = out.join(format!("gpkg-uzlastir-{mode}-{label}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        picture(&mut app, "cizim");
        let slots: Vec<f64> = {
            let doc = app.document.as_ref().expect("a drawing");
            doc.model
                .by_layer_with_uids("import-parsel")
                .filter_map(|(u, _)| doc.model.slot_of(u))
                .map(|s| f64::from(s.0))
                .collect()
        };
        app.zoom_to_objects(&slots);
        let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
            "import-parsel".into(),
        ))));
        picture(&mut app, "pencere");
        // One conflict to the table's side, the rest to the layer's.
        let first = app
            .reconcile
            .as_ref()
            .and_then(|w| w.first_conflict())
            .expect("a conflict");
        let _ = app.update(Message::Reconcile(R::All(ReconcileSide::Layer)));
        let _ = app.update(Message::Reconcile(R::Side(first, ReconcileSide::Table)));
        picture(&mut app, "secildi");
        let _ = app.update(Message::Reconcile(R::Apply));
        app.zoom_to_objects(&slots);
        picture(&mut app, "uzlasti");
    }
}

/// uzlas-komsu.gpkg (docs/adr/0553) bound over a copy, both tables (four parcels side by side, three buildings), the
/// project's relation, the layers edited as tests/all/reconcile.rs edits them, then the copy changed as QGIS changed it.
fn komsu_scene(name: &str) -> (App, PathBuf) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let file = scratch(name).join("uzlas-komsu.gpkg");
    std::fs::copy(gpkg("uzlas-komsu.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let run_on = |doc: &mut kentos_domain::Document, command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    };
    for table in ["komsu", "bina"] {
        run_on(
            doc,
            "cad.sources.edit",
            serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": table } }),
        );
    }
    run_on(
        doc,
        "cad.relations.edit",
        serde_json::json!({ "edit": { "kind": "create", "relation": { "id": "bina-komsu", "name": "Parselin binaları",
            "parent": "import-komsu", "child": "import-bina", "field": "komsu" } } }),
    );
    let by = |doc: &kentos_domain::Document, layer: &str, name: &str, value: &str| {
        doc.by_layer_with_uids(layer)
            .find(|(_, e)| e.base().attrs.get(name).map(String::as_str) == Some(value))
            .map(|(u, _)| u.to_string())
            .expect("an object")
    };
    let (x0, y0) = (487000.0, 4420000.0);
    let p = |x: f64, y: f64| serde_json::json!({ "x": x0 + x, "y": y0 + y });
    let u = by(doc, "import-komsu", "ad", "K1");
    run_on(
        doc,
        "cad.entities.set",
        serde_json::json!({ "uids": [u], "attrs": { "ad": "Z1" }, "operation": "attributes" }),
    );
    for (k, pts) in [
        (
            "K2",
            [p(20.0, 0.0), p(40.0, 0.0), p(42.0, 22.0), p(20.0, 20.0)],
        ),
        (
            "K3",
            [p(40.0, 0.0), p(60.0, 0.0), p(60.0, 20.0), p(42.0, 22.0)],
        ),
    ] {
        let u = by(doc, "import-komsu", "ad", k);
        run_on(
            doc,
            "cad.entities.edit",
            serde_json::json!({ "operation": "grip", "changes": [{ "kind": "update", "uid": u,
                "geometry": { "kind": "polygon", "pts": pts } }] }),
        );
    }
    let u = by(doc, "import-komsu", "ad", "K4");
    run_on(
        doc,
        "cad.entities.delete",
        serde_json::json!({ "uids": [u] }),
    );
    for (k, kat) in [("K2", "5"), ("K3", "7")] {
        let parent = by(doc, "import-komsu", "ad", k);
        let u = by(doc, "import-bina", "komsu", &parent);
        run_on(
            doc,
            "cad.entities.set",
            serde_json::json!({ "uids": [u], "attrs": { "kat": kat }, "operation": "attributes" }),
        );
    }
    std::fs::copy(gpkg("uzlas-komsu-qgis.gpkg"), &file).expect("changed in QGIS");
    (app, file)
}

#[test]
fn a_reconcile_takes_the_files_other_layers_and_resolves_a_group_whole() {
    use crate::source_reconcile::Event as R;
    use kentos_commands::ReconcileSide;
    let (mut app, file) = komsu_scene("gpkg-komsu");
    let before = std::fs::read(&file).expect("read");
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-komsu".into(),
    ))));
    let w = app
        .reconcile
        .as_ref()
        .expect("the refused write opens the window");
    let view = format!("{w:?}");
    // The file's other layer goes with it; the plan says what merged and the two groups.
    assert!(view.contains("with: [\"import-bina\"]"), "{view}");
    assert!(
        view.contains("SharedVertex") && view.contains("Relation"),
        "{view}"
    );
    assert!(view.contains("merged: [ReconcileMerge"), "{view}");
    assert_eq!(
        std::fs::read(&file).expect("read"),
        before,
        "nothing written over"
    );
    // A side taken for one member of a group is taken for all of them.
    let member = w
        .plan_conflicts()
        .into_iter()
        .find(|(_, group)| group.is_some())
        .expect("a group's member");
    let _ = app.update(Message::Reconcile(R::Side(
        member.0.clone(),
        ReconcileSide::Table,
    )));
    let sides = app.reconcile.as_ref().expect("the window").sides_taken();
    assert!(sides >= 2, "the group's members all took the side: {sides}");
    let _ = app.update(Message::Reconcile(R::All(ReconcileSide::Layer)));
    let _ = app.update(Message::Reconcile(R::Apply));
    assert!(app.reconcile.is_none(), "closed when done");
    assert_ne!(std::fs::read(&file).expect("read"), before, "written");
    // Both tables went in one call: nothing is left to write for either layer.
    for layer in ["import-komsu", "import-bina"] {
        let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
            layer.into(),
        ))));
        assert!(
            says(&app, "yazılacak bir şey yok"),
            "{layer}: {:?}",
            app.log.lines().last().map(|l| l.text.clone())
        );
    }
}

#[test]
#[ignore = "pictures, run by hand"]
fn reconcile_group_screens() {
    use crate::source_reconcile::Event as R;
    use iced::Size;
    use kentos_commands::ReconcileSide;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let (mut app, _file) = komsu_scene(&format!("gpkg-komsu-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let _ = snapshot.render(app.view(), &app.theme());
            let path = out.join(format!(
                "gpkg-uzlastir-grup-{mode}-{label}-{width}x{height}.png"
            ));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        let slots: Vec<f64> = {
            let doc = app.document.as_ref().expect("a drawing");
            doc.model
                .by_layer_with_uids("import-komsu")
                .filter_map(|(u, _)| doc.model.slot_of(u))
                .map(|s| f64::from(s.0))
                .collect()
        };
        app.zoom_to_objects(&slots);
        let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
            "import-komsu".into(),
        ))));
        picture(&mut app, "pencere");
        // One group to the table's side, the rest to the layer's.
        let member = app
            .reconcile
            .as_ref()
            .and_then(|w| w.plan_conflicts().into_iter().find(|(_, g)| g.is_some()))
            .expect("a group's member");
        let _ = app.update(Message::Reconcile(R::All(ReconcileSide::Layer)));
        let _ = app.update(Message::Reconcile(R::Side(member.0, ReconcileSide::Table)));
        picture(&mut app, "secildi");
        let _ = app.update(Message::Reconcile(R::Apply));
        app.zoom_to_objects(&slots);
        picture(&mut app, "uzlasti");
    }
}

/// Kaynak durumu's scene (docs/adr/0543): the reconcile scene's parcels (both sides changed), a copy of qgis.gpkg's
/// points with one attribute edited (only the layer changed), wgs84.gpkg's places in their own system, and a file
/// bound then gone.
fn status_scene(name: &str) -> (App, PathBuf) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let (mut app, _) = reconcile_scene(name);
    let dir = scratch(&format!("{name}-durum"));
    let points = dir.join("noktalar.gpkg");
    let gone = dir.join("tasindi.gpkg");
    std::fs::copy(gpkg("qgis.gpkg"), &points).expect("copied");
    std::fs::copy(gpkg("qgis.gpkg"), &gone).expect("copied");
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let run_on = |doc: &mut kentos_domain::Document, input: serde_json::Value, command: &str| {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer
    };
    let linked = run_on(
        doc,
        serde_json::json!({ "edit": { "kind": "link", "path": points.display().to_string(), "table": "nokta" } }),
        "cad.sources.edit",
    );
    let layer = linked["output"]["link"]["layer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    run_on(
        doc,
        serde_json::json!({ "edit": { "kind": "link", "path": gpkg("wgs84.gpkg").display().to_string(), "table": "yer", "reproject": true } }),
        "cad.sources.edit",
    );
    run_on(
        doc,
        serde_json::json!({ "edit": { "kind": "link", "path": gone.display().to_string(), "table": "yol" } }),
        "cad.sources.edit",
    );
    let first = doc
        .by_layer_with_uids(&layer)
        .next()
        .map(|(u, _)| u.to_string())
        .expect("a point");
    run_on(
        doc,
        serde_json::json!({ "uids": [first], "attrs": { "ad": "Yeni ad" }, "operation": "attributes" }),
        "cad.entities.set",
    );
    std::fs::remove_file(&gone).expect("gone");
    (app, points)
}

#[test]
fn the_source_status_says_each_layers_way_out_and_takes_it() {
    use crate::source_status::Event as S;
    use kentos_commands::SourceNext;
    let (mut app, points) = status_scene("gpkg-durum");
    let _ = app.update(Message::Bound(crate::bound::Event::Status));
    let next = |app: &App, layer: &str| {
        let i = app.source_status_row(layer).expect("a row");
        app.source_status_next(i)
    };
    let layers: Vec<String> = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .sources
        .iter()
        .map(|s| s.layer.clone())
        .collect();
    assert_eq!(layers.len(), 4, "{layers:?}");
    assert_eq!(
        layers.iter().map(|l| next(&app, l)).collect::<Vec<_>>(),
        [
            SourceNext::Reconcile,
            SourceNext::Write,
            SourceNext::None,
            SourceNext::Relink
        ]
    );
    // Kaynağa yaz on the points: the file written, the row now the same.
    let before = std::fs::read(&points).expect("read");
    let i = app.source_status_row(&layers[1]).expect("a row");
    let _ = app.update(Message::SourceStatus(S::Act(i)));
    assert_ne!(std::fs::read(&points).expect("read"), before, "written");
    assert_eq!(next(&app, &layers[1]), SourceNext::None);
    // Uzlaştır… opens its own window in this one's place.
    let i = app.source_status_row(&layers[0]).expect("a row");
    let _ = app.update(Message::SourceStatus(S::Act(i)));
    assert!(app.reconcile.is_some() && app.source_status.is_none());
}

/// The state of the layer the table shows: its way out and whether the reading is of an earlier drawing.
fn table_source(app: &mut App, layer: &str) -> Option<(kentos_commands::SourceNext, bool)> {
    let _ = app.update(Message::Features(crate::features::Event::Layer(
        layer.to_owned(),
    )));
    let doc = app.document.as_ref().expect("a drawing");
    let rows = app.feature_rows(doc);
    app.source_state(&rows).map(|(s, stale)| (s.next, stale))
}

#[test]
fn the_table_of_a_bound_layer_says_its_state_and_takes_the_way_out() {
    use crate::features::{Event as F, source::SourceEvent};
    use kentos_commands::SourceNext;
    let (mut app, points) = status_scene("gpkg-tablo");
    let _ = app.update(Message::Run("data.featureTable"));
    let layers: Vec<String> = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .sources
        .iter()
        .map(|s| s.layer.clone())
        .collect();
    // The same four states Kaynak durumu reads, from the same rule.
    let ways: Vec<_> = layers
        .iter()
        .map(|l| table_source(&mut app, l).map(|(n, _)| n))
        .collect();
    assert_eq!(
        ways,
        [
            Some(SourceNext::Reconcile),
            Some(SourceNext::Write),
            Some(SourceNext::None),
            Some(SourceNext::Relink)
        ]
    );
    // A layer that is not bound has no source line.
    let free = {
        let doc = app.document.as_ref().expect("a drawing");
        doc.model
            .layers()
            .leaves()
            .iter()
            .map(|n| n.id.clone())
            .find(|id| !layers.contains(id))
    };
    if let Some(free) = free {
        assert_eq!(table_source(&mut app, &free), None);
    }
    // Kaynağa yaz from the table's line: the file written, the line now the same.
    let _ = table_source(&mut app, &layers[1]);
    let before = std::fs::read(&points).expect("read");
    let _ = app.update(Message::Features(F::Source(SourceEvent::Act)));
    assert_ne!(std::fs::read(&points).expect("read"), before, "written");
    assert_eq!(
        table_source(&mut app, &layers[1]).map(|(n, _)| n),
        Some(SourceNext::None)
    );
    // An edit of the layer is “Katmanda yazılmamış” at once, on a layer small enough to follow the drawing.
    let uid = {
        let doc = app.document.as_ref().expect("a drawing");
        doc.model
            .by_layer_with_uids(&layers[1])
            .next()
            .map(|(u, _)| u.to_string())
            .expect("a point")
    };
    let input = serde_json::json!({ "uids": [uid], "attrs": { "ad": "Başka ad" }, "operation": "attributes" });
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(
            &mut app.document.as_mut().expect("a drawing").model,
        ),
        "cad.entities.set",
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    assert_eq!(
        table_source(&mut app, &layers[1]),
        Some((SourceNext::Write, false))
    );
    // Uzlaştır… from the table opens its window.
    let _ = table_source(&mut app, &layers[0]);
    let _ = app.update(Message::Features(F::Source(SourceEvent::Act)));
    assert!(app.reconcile.is_some());
}

/// Öznitelik tablosu's source line (docs/adr/0546 §3): a bound layer with changes on both sides, one with its own
/// changes, one in its own system, one whose file is gone; the second after Kaynağa yaz
/// (`.run/shots/gpkg-tablo-kaynak-*`).
#[test]
#[ignore = "pictures, run by hand"]
fn table_source_screens() {
    use crate::features::{Event as F, source::SourceEvent};
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let (mut app, _) = status_scene(&format!("gpkg-tablo-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let layers: Vec<String> = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .settings()
            .sources
            .iter()
            .map(|s| s.layer.clone())
            .collect();
        let _ = app.update(Message::Run("data.featureTable"));
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let _ = snapshot.render(app.view(), &app.theme());
            let path = out.join(format!(
                "gpkg-tablo-kaynak-{mode}-{label}-{width}x{height}.png"
            ));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        let _ = app.update(Message::WindowResized(Size::new(width, height)));
        for (layer, label) in [(0, "uzlastir"), (1, "yaz"), (2, "wgs84"), (3, "yok")] {
            let _ = app.update(Message::Features(F::Layer(layers[layer].clone())));
            picture(&mut app, label);
        }
        let _ = app.update(Message::Features(F::Layer(layers[1].clone())));
        let _ = app.update(Message::Features(F::Source(SourceEvent::Act)));
        picture(&mut app, "yazildi");
    }
}

#[test]
#[ignore = "pictures, run by hand"]
fn source_status_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let (mut app, _) = status_scene(&format!("gpkg-durum-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let _ = snapshot.render(app.view(), &app.theme());
            let path = out.join(format!(
                "gpkg-kaynak-durumu-{mode}-{label}-{width}x{height}.png"
            ));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        let _ = app.update(Message::Bound(crate::bound::Event::Status));
        let missing = app.source_status_row(
            &app.document
                .as_ref()
                .expect("a drawing")
                .model
                .settings()
                .sources[3]
                .layer
                .clone(),
        );
        let _ = app.update(Message::SourceStatus(crate::source_status::Event::Pick(
            missing.expect("a row"),
        )));
        picture(&mut app, "pencere");
        let i = app
            .source_status_row(
                &app.document
                    .as_ref()
                    .expect("a drawing")
                    .model
                    .settings()
                    .sources[1]
                    .layer
                    .clone(),
            )
            .expect("a row");
        let _ = app.update(Message::SourceStatus(crate::source_status::Event::Act(i)));
        picture(&mut app, "yazildi");
    }
}

// docs/adr/0598 §4: undoing a step already written to the source says so (the source still holds it; Kaynağa yaz puts
// the undo there too); undoing a step not yet written says nothing more.
#[test]
fn undoing_a_written_step_says_the_source_still_holds_it() {
    let dir = scratch("gpkg-undo-written");
    let file = dir.join("kadastro.gpkg");
    std::fs::copy(gpkg("qgis.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    app.picker = Picker::File(file.clone());
    run(&mut app, "source.link");
    parcels_only(&mut app);
    send(&mut app, Event::Gpkg(G::Run));
    delete_first(&mut app, "import-parsel");
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(says(&app, "tablosuna yazıldı: 1 silindi"));
    app.step_history(true);
    assert!(
        says(&app, "kaynağına yazılmıştı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(
        says(&app, "tablosuna yazıldı: 1 eklendi"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    // Not written yet: a plain undo.
    delete_first(&mut app, "import-parsel");
    let before = app
        .log
        .lines()
        .filter(|l| l.text.contains("kaynağına yazılmıştı"))
        .count();
    app.step_history(true);
    assert_eq!(
        app.log
            .lines()
            .filter(|l| l.text.contains("kaynağına yazılmıştı"))
            .count(),
        before
    );
    assert!(said(&app, "Geri alındı"));
}
