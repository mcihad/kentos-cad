//! The GeoJSON and Shapefile windows driven as the user drives them
//! (docs/adr/0046, 0053), with the files the shared readers are tested
//! with (fixtures/formats/v1/gis). The sample drawing is in TUREF / TM36
//! (EPSG:5256); its “Parsel” layer is open and its “Bina” layer locked.

use std::path::PathBuf;

use kentos_commands::Entity;

use super::gis_import::{self, Source};
use super::tests::{count, fixture, run, said, send};
use super::words;
use super::{Event, Kind, Picked, Window, drawing_import, dxf_export, geojson_export};
use crate::app::{App, Dialog, Message, Picker};
use crate::crs::Note;
use crate::files_testing::{app_with_drawing, last_said, scratch};

fn gis(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formats/v1/gis")
        .join(name)
}

fn picked(name: &str) -> Picked {
    Picked {
        name: name.to_owned(),
        bytes: std::fs::read(gis(name)).expect("a fixture").into(),
        path: Some(gis(name)),
    }
}

fn window(app: &App) -> &gis_import::State {
    match &app.exchange {
        Some(Window::GisImport(s)) => s,
        other => panic!("the GeoJSON/Shapefile window, not {other:?}"),
    }
}

fn import(app: &mut App) {
    send(app, Event::GisImport(gis_import::Event::Run));
}

fn warnings(app: &App) -> String {
    let format = kentos_interaction::Format::default();
    window(app)
        .crs
        .notes(&5256.into(), &format)
        .into_iter()
        .filter_map(|n| match n {
            Note::Warn(w) => Some(w),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn a_geojson_in_the_projects_system_goes_in_as_one_undo_step() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(gis("tm.geojson"));
    let before = count(&app);
    run(&mut app, "file.import.geojson");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    let s = window(&app);
    assert!(!s.reading && s.result.is_some(), "{:?}", s.failed);
    // The file's crs member names the project's system: that is the answer.
    assert_eq!(s.crs.srid, Some(5256));
    import(&mut app);
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    let model = &app.document.as_ref().expect("open").model;
    // “Parsel” joins the drawing's layer, the objects without a layer go to a
    // new “tm”, the locked “Bina” takes nothing: 1 area and the two-point
    // object of the file's MultiPoint (docs/adr/0174) of 4.
    assert_eq!(model.len() - before, 2);
    let parcel = model
        .entities()
        .find(|e| e.base().label.as_deref() == Some("101/5"))
        .expect("the parcel");
    assert_eq!(parcel.base().layer_id, "parsel");
    let attr = |k: &str| parcel.base().attrs.get(k).map(String::as_str);
    assert_eq!(
        (attr("mahalle"), attr("alan")),
        (Some("Çamlıbel"), Some("2345.678"))
    );
    assert_eq!(
        last_said(&app),
        "“tm.geojson”: 2 nesne 2 katmana alındı; 1 yeni katman “tm.geojson” grubunda. Tek adımda geri alınabilir."
    );
    // The window writes with the command: its row in the journal names it, with the window's choices (docs/adr/0563 §2).
    let row = model.journal().last().expect("a row");
    assert_eq!(row.command.as_deref(), Some("file.import.geojson"));
    assert_eq!(row.label.as_deref(), Some("GeoJSON: tm.geojson"));
    let input = row.input.clone().expect("the input");
    assert!(
        input["path"]
            .as_str()
            .is_some_and(|p| p.ends_with("gis/tm.geojson")),
        "{input}"
    );
    // “Bina” is locked: the window leaves it out, and says so to the command.
    assert_eq!(input["skipLayers"], serde_json::json!(["Bina"]));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before, "one undo step takes all of it back");
}

/// What an import brings into a CBS project goes into its tables (docs/adr/9196 §2): the file's new layer becomes a
/// table of the GeoPackage beside the project, read back as a data layer; the project's file holds none of its objects.
#[test]
fn an_import_into_a_cbs_project_goes_into_its_tables() {
    crate::data_layers::register();
    let dir = scratch("cbs-ice-alma");
    let mut app = app_with_drawing();
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Gis);
        doc.model.set_settings(settings);
        doc.path = Some(dir.join("Kent.kcad"));
    }
    app.picker = Picker::File(gis("tm.geojson"));
    run(&mut app, "file.import.geojson");
    import(&mut app);
    let model = &app.document.as_ref().expect("open").model;
    let tm = model
        .layers()
        .leaves()
        .into_iter()
        .find(|n| n.name == "tm")
        .cloned()
        .expect("the file's layer");
    let data = tm.data.as_ref().expect("a data layer");
    assert!(data.address.ends_with("Kent.gpkg"), "{}", data.address);
    assert_eq!(model.count(&tm.id), 0, "its objects are its table's rows");
    let lines: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        lines.iter().any(|l| l.contains("CBS projesinin verisi tablolarda durur") && l.contains("Kent.gpkg")),
        "{lines:?}"
    );
    // The layer the drawing had already keeps the parcel it took: its own move is the user's (Veri kaynağına taşı).
    assert!(kentos_native_application::layer_kind::held_in_file(model).iter().all(|h| h.id != tm.id));
}

#[test]
fn a_wgs84_geojson_is_refused_until_the_user_says_it_is_the_projects() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(gis("crs84.geojson"));
    let before = count(&app);
    run(&mut app, "file.import.geojson");
    assert_eq!(window(&app).crs.srid, Some(4326));
    import(&mut app);
    assert_eq!(count(&app), before, "another system: nothing goes in");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    // The user says the coordinates are TM36 after all: allowed, and the
    // window says the file says otherwise and that they look like degrees.
    send(&mut app, Event::GisImport(gis_import::Event::Crs(5256)));
    let w = warnings(&app);
    assert!(w.contains("Dosya EPSG:4326 diyor"), "{w}");
    assert!(w.contains("derece (boylam, enlem) gibi görünüyor"), "{w}");
    import(&mut app);
    assert!(count(&app) > before);
}

#[test]
fn a_shapefile_is_its_files_chosen_together() {
    let mut app = app_with_drawing();
    let before = count(&app);
    let files = [
        "noktalar.shp",
        "noktalar.shx",
        "noktalar.dbf",
        "noktalar.prj",
        "parseller.dbf",
    ]
    .map(picked)
    .to_vec();
    send(
        &mut app,
        Event::PickedMany(Kind::Shapefile, Some(Ok(files))),
    );
    let s = window(&app);
    assert!(s.result.is_some(), "{:?}", s.failed);
    // The .prj says TUREF / TM36.
    assert!(s.crs.matches(&5256.into()));
    let Source::Shapefile(files) = &s.source else {
        panic!("a layer's files")
    };
    let set = gis_import::shapefile_set(files).expect("a layer");
    assert_eq!(set.unused, ["parseller.dbf"]);
    let points = s.result.as_ref().map_or(0, |r| r.entities.len());
    import(&mut app);
    assert_eq!(count(&app) - before, points);
    assert!(
        said(&app, "“noktalar.shp”:"),
        "{:?}",
        app.log.lines().collect::<Vec<_>>()
    );
}

#[test]
fn a_zipped_shapefile_offers_its_layers_one_at_a_time() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(gis("katmanlar.zip"));
    let before = count(&app);
    run(&mut app, "file.import.shp");
    let s = window(&app);
    let Source::Zip { layers, layer, .. } = &s.source else {
        panic!("an archive")
    };
    assert_eq!(layers, &["katmanlar/parseller", "katmanlar/yollar"]);
    assert_eq!(*layer, 0);
    let first = s.result.as_ref().expect("the first layer read");
    assert_eq!(first.layers[0].name, "parseller");
    assert_eq!(first.report.source[0].label, "Zip arşivinden");
    // Another layer of the archive is read in its place.
    send(&mut app, Event::GisImport(gis_import::Event::Layer(1)));
    let s = window(&app);
    let read = s.result.as_ref().expect("the second layer read");
    assert_eq!(read.layers[0].name, "yollar");
    let roads = read.entities.len();
    // ED50 / TM30: said by its .prj, refused until the user answers otherwise.
    assert!(!s.crs.matches(&5256.into()));
    send(&mut app, Event::GisImport(gis_import::Event::Crs(5256)));
    import(&mut app);
    assert_eq!(count(&app) - before, roads);
    let model = &app.document.as_ref().expect("open").model;
    // The archive's layer is the command's `entry`, the system the user chose over the .prj's its `srid`.
    let row = model.journal().last().expect("a row");
    assert_eq!(row.command.as_deref(), Some("file.import.shp"));
    assert_eq!(row.label.as_deref(), Some("Shapefile: katmanlar.zip"));
    let input = row.input.clone().expect("the input");
    assert_eq!(input["entry"], "katmanlar/yollar");
    assert_eq!(input["srid"], 5256);
    let group = model
        .layers()
        .nodes()
        .iter()
        .find(|n| n.name == "katmanlar.zip")
        .expect("a group named after the archive");
    assert_eq!(group.children[0].name, "yollar");
}

/// docs/adr/0142: the file's line says how many objects came with elevations, as the
/// DXF and NCZ windows say it (`words::facts`), and only when some did; and after it
/// how many came with measures (M, docs/adr/0224 §3), only when some did.
#[test]
fn the_file_line_says_how_many_objects_came_with_elevations_and_only_when_some_did() {
    let elevated = |app: &App| {
        let r = window(app).result.as_ref().expect("read");
        r.entities
            .iter()
            .filter(|e| kentos_interaction::elevation::has_any(e))
            .count()
    };
    let line = |app: &App| {
        let r = window(app).result.as_ref().expect("read");
        words::facts(&r.report.source)
    };
    // GeoJSON: nine of the ten objects of kotlu.geojson have a third number in a position.
    let mut app = app_with_drawing();
    app.picker = Picker::File(gis("kotlu.geojson"));
    run(&mut app, "file.import.geojson");
    assert_eq!(elevated(&app), 9);
    let text = line(&app);
    assert!(text.ends_with(", Kotlu nesne: 9"), "{text}");
    // A file without any: no such fact.
    let mut app = app_with_drawing();
    app.picker = Picker::File(gis("feature-epsg.geojson"));
    run(&mut app, "file.import.geojson");
    assert_eq!(elevated(&app), 0);
    assert!(!line(&app).contains("Kotlu"), "{}", line(&app));
    // Shapefile: PolyLineZ's Z; the fact follows the layer's parts in the window's line.
    let mut app = app_with_drawing();
    let files = ["yollarz.shp", "yollarz.shx", "yollarz.dbf", "yollarz.prj"]
        .map(picked)
        .to_vec();
    send(
        &mut app,
        Event::PickedMany(Kind::Shapefile, Some(Ok(files))),
    );
    let n = elevated(&app);
    assert!(n > 0, "the fixture has lines with a Z");
    // PolyLineZ's M values are its vertices' measures.
    let m = window(&app)
        .result
        .as_ref()
        .expect("read")
        .entities
        .iter()
        .filter(|e| kentos_formats::import::has_measure(e))
        .count();
    assert!(m > 0, "the fixture's lines have M values");
    let text = line(&app);
    assert!(
        text.ends_with(&format!(", Kotlu nesne: {n}, Ölçülü nesne: {m}")),
        "{text}"
    );
    // The same roads without a Z.
    let plain = ["yollar.shp", "yollar.shx", "yollar.dbf", "yollar.prj"]
        .map(picked)
        .to_vec();
    let mut app = app_with_drawing();
    send(
        &mut app,
        Event::PickedMany(Kind::Shapefile, Some(Ok(plain))),
    );
    assert!(!line(&app).contains("Kotlu"), "{}", line(&app));
    assert!(!line(&app).contains("Ölçülü"), "{}", line(&app));
}

#[test]
fn files_without_a_layer_say_why_and_nothing_is_read() {
    let mut app = app_with_drawing();
    send(
        &mut app,
        Event::PickedMany(
            Kind::Shapefile,
            Some(Ok(vec![picked("parseller.dbf"), picked("parseller.prj")])),
        ),
    );
    let s = window(&app);
    assert!(s.failed.as_deref().is_some_and(|e| e.contains(".shp yok")));
    assert!(s.result.is_none() && !s.reading);
    // A broken archive: the reader's reason, never a panic.
    let mut app = app_with_drawing();
    let mut broken = picked("parseller.zip");
    broken.bytes = broken.bytes[..broken.bytes.len() / 2].to_vec().into();
    send(
        &mut app,
        Event::PickedMany(Kind::Shapefile, Some(Ok(vec![broken]))),
    );
    assert!(window(&app).failed.is_some());
}

#[test]
fn a_geojson_export_writes_what_the_reader_takes_back() {
    let mut app = app_with_drawing();
    let dir = scratch("export-geojson");
    let path = dir.join("cizim.geojson");
    app.picker = Picker::File(path.clone());
    run(&mut app, "file.export.geojson");
    send(
        &mut app,
        Event::GeoJsonExport(geojson_export::Event::Scope(dxf_export::Scope::All)),
    );
    // The project's own coordinates, asked for (RFC 7946's WGS 84 is the default, GIS-011).
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Wgs84(false)));
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    assert!(
        said(&app, "“cizim.geojson” yazıldı:"),
        "{:?}",
        app.log.lines().collect::<Vec<_>>()
    );
    let bytes = std::fs::read(&path).expect("written");
    let back = kentos_formats::geojson::read(
        &bytes,
        &kentos_commands::GeoJsonReadOptions {
            layer: "cizim".into(),
            max_entities: 0,
        },
    )
    .expect("reads");
    let model = &app.document.as_ref().expect("open").model;
    let writable = model
        .entities()
        .filter(|e| {
            !matches!(
                e,
                Entity::Text(_) | Entity::Dimension(_) | Entity::Xline(_) | Entity::Ray(_)
            )
        })
        .count();
    assert_eq!(back.entities.len(), writable);
    // A TM36 project: its own coordinates, named by the crs member, not RFC 7946.
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("EPSG::5256"),
        "{}",
        &text[..200.min(text.len())]
    );
    assert!(!model.is_dirty(), "an export is not an edit");
}

/// A DXF's blocks going out to GeoJSON (docs/adr/0144 §5): an insert is its
/// block's objects placed, one GeometryCollection read back as those
/// objects on the insert's layer; an insert of a block that draws nothing is
/// left out and said.
#[test]
fn a_geojson_export_writes_an_insert_as_its_blocks_objects() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("blocks.dxf"));
    run(&mut app, "file.import.dxf");
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    let dir = scratch("export-geojson-blocks");
    let path = dir.join("bloklar.geojson");
    app.picker = Picker::File(path.clone());
    run(&mut app, "file.export.geojson");
    send(
        &mut app,
        Event::GeoJsonExport(geojson_export::Event::Scope(dxf_export::Scope::All)),
    );
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    assert!(
        said(&app, "“bloklar.geojson” yazıldı:"),
        "{}",
        last_said(&app)
    );
    let log: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        log.iter().any(|l| l.contains("GeometryCollection")),
        "{log:?}"
    );
    let back = kentos_formats::geojson::read(
        &std::fs::read(&path).expect("written"),
        &kentos_commands::GeoJsonReadOptions {
            layer: "bloklar".into(),
            max_entities: 0,
        },
    )
    .expect("reads");
    let model = &app.document.as_ref().expect("open").model;
    let plain = model
        .entities()
        .filter(|e| {
            !matches!(
                e,
                Entity::Text(_)
                    | Entity::Dimension(_)
                    | Entity::Xline(_)
                    | Entity::Ray(_)
                    | Entity::Insert(_)
            )
        })
        .count();
    // DAIRE's stretched insert (kept, docs/adr/0315 §3): its ellipse; KENDI draws nothing and is left out. KAPI came in
    // opened for its height (docs/adr/2100 §2): its point and the nested NO's circle and line are plain objects, the
    // point and the circle on the insert's layer.
    assert_eq!(back.entities.len(), plain + 1);
    assert!(
        back.entities
            .iter()
            .filter(|e| e.base().layer_id == "KAPILAR")
            .count()
            >= 2
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Pictures of the GeoJSON and Shapefile windows for the owner, dark and
/// light, at 1440×900 and at the smallest window (1100×650), written to
/// `.run/shots` (never committed); not run by default:
///
/// ```text
/// cargo test -p kentos-desktop exchange::gis_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
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
            app.picker = Picker::File(gis("tm.geojson"));
            run(&mut app, "file.import.geojson");
            picture(&mut app, &format!("gis-{mode}-1-geojson"));

            let mut app = fresh();
            app.picker = Picker::File(gis("crs84.geojson"));
            run(&mut app, "file.import.geojson");
            picture(&mut app, &format!("gis-{mode}-2-wgs84"));
            send(&mut app, Event::GisImport(gis_import::Event::Crs(5256)));
            picture(&mut app, &format!("gis-{mode}-3-wgs84-proje-sistemi"));

            let mut app = fresh();
            app.picker = Picker::File(gis("katmanlar.zip"));
            run(&mut app, "file.import.shp");
            picture(&mut app, &format!("gis-{mode}-4-zip"));

            let mut app = fresh();
            let files = [
                "noktalar.shp",
                "noktalar.shx",
                "noktalar.dbf",
                "noktalar.prj",
                "parseller.dbf",
            ]
            .map(picked)
            .to_vec();
            send(
                &mut app,
                Event::PickedMany(Kind::Shapefile, Some(Ok(files))),
            );
            picture(&mut app, &format!("gis-{mode}-5-shapefile"));

            let mut app = fresh();
            run(&mut app, "file.export.geojson");
            picture(&mut app, &format!("gis-{mode}-6-geojson-ver"));

            // With a DXF's blocks: the window says how an insert is written.
            let mut app = fresh();
            app.picker = Picker::File(fixture("blocks.dxf"));
            run(&mut app, "file.import.dxf");
            send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
            run(&mut app, "file.export.geojson");
            send(
                &mut app,
                Event::GeoJsonExport(geojson_export::Event::Scope(dxf_export::Scope::All)),
            );
            picture(&mut app, &format!("gis-{mode}-7-geojson-ver-bloklar"));

            // The windows write with their commands (docs/adr/0563): the history says what was asked and its
            // answer's words, one row each, named by the command.
            let mut app = fresh();
            app.command_expanded = true;
            app.picker = Picker::File(gis("tm.geojson"));
            run(&mut app, "file.import.geojson");
            import(&mut app);
            picture(&mut app, &format!("gis-{mode}-8-geojson-komutla"));

            let mut app = fresh();
            app.command_expanded = true;
            let files = [
                "parseller.shp",
                "parseller.shx",
                "parseller.dbf",
                "parseller.prj",
            ]
            .map(picked)
            .to_vec();
            send(
                &mut app,
                Event::PickedMany(Kind::Shapefile, Some(Ok(files))),
            );
            send(&mut app, Event::GisImport(gis_import::Event::Convert(true)));
            picture(&mut app, &format!("gis-{mode}-9-shapefile-cevirme"));
            import(&mut app);
            picture(&mut app, &format!("gis-{mode}-10-shapefile-komutla"));

            let mut app = fresh();
            app.command_expanded = true;
            app.picker = Picker::File(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures/formats/v1/kml/parsel.kml"),
            );
            run(&mut app, "file.import.kml");
            send(&mut app, Event::GisImport(gis_import::Event::Convert(true)));
            import(&mut app);
            picture(&mut app, &format!("gis-{mode}-11-kml-komutla"));
        }
    }
}

/// GIS-011 (docs/adr/1077): a TM36 project's GeoJSON is RFC 7946 by default — WGS 84 degrees, no crs member, a
/// defined whole number a JSON number — and the choice of the project's coordinates is kept for the next export of the
/// session; one layer's objects are one line, not a table of one row.
#[test]
fn a_geojson_export_is_rfc_7946_by_default_and_keeps_the_choice() {
    let mut app = app_with_drawing();
    let dir = scratch("export-geojson-rfc");
    let path = dir.join("web.geojson");
    app.picker = Picker::File(path.clone());
    let doc = &app.document.as_ref().expect("open").model;
    let first = doc.entities_with_uids().find(|(_, e)| e.kind() == "polygon").map(|(u, _)| u).expect("a parcel");
    let slot = doc.slot_of(first).expect("its slot");
    app.selection.set([slot]);
    run(&mut app, "file.export.geojson");
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    let mut update = |app: &mut App, m| drop(app.update(m));
    shot.settle(&mut app, App::view, &mut update);
    crate::files_testing::evidence(&mut shot, &app, "gis-011", "tek-katman-wgs84");
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).expect("written")).expect("json");
    assert!(v.get("crs").is_none(), "RFC 7946 has no crs member");
    let ring = &v["features"][0]["geometry"]["coordinates"][0][0];
    let (lon, lat) = (ring[0].as_f64().unwrap(), ring[1].as_f64().unwrap());
    assert!((25.0..46.0).contains(&lon) && (35.0..43.0).contains(&lat), "{lon}, {lat}");
    // The project's coordinates chosen once are the next export's.
    run(&mut app, "file.export.geojson");
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Wgs84(false)));
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    run(&mut app, "file.export.geojson");
    let Some(super::Window::GeoJsonExport(s)) = &app.exchange else {
        panic!("the window");
    };
    assert!(!s.wgs84_chosen(), "the last choice is kept");
}

/// GIS-042 (docs/adr/1099): the exchange sample's layers written as one archive (Shapefile dışa aktar › Tek arşiv), then
/// imported into an empty GIS project of the same system: every layer of the archive is read and listed with its
/// objects and system, and İçe aktar takes them all in one undo step, the command with no `entry`; Ctrl+Z takes them
/// all back. An archive whose layers name different systems (katmanlar.zip) still takes one at a time.
#[test]
fn an_archive_s_layers_come_in_together() {
    // The archive, written by the export command.
    let dir = scratch("shp-arsiv");
    let zip = dir.join("envanter.zip");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(include_str!("../../../../../fixtures/exchange/v1/devir.json")).expect("the sample");
    let sample = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let input: kentos_commands::ExportShp =
        serde_json::from_value(serde_json::json!({ "path": zip.to_string_lossy() })).expect("the input");
    let made = kentos_headless::shp_export::make(&sample, &input).expect("made");
    kentos_headless::shp_export::write(&made).expect("written");
    assert!(zip.exists());
    let members = kentos_formats::shp::zip_layers(&std::fs::read(&zip).expect("read")).expect("listed");
    assert!(members.len() >= 4, "{members:?}");
    // An empty GIS project of the sample's system.
    let mut drawing: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/interaction/v1/empty.kcad")).expect("the drawing");
    drawing["settings"]["srid"] = serde_json::json!(5256);
    drawing["settings"]["workspace"] = serde_json::json!("gis");
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&drawing.to_string()).expect("reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.picker = Picker::File(zip.clone());
    run(&mut app, "file.import.shp");
    let s = window(&app);
    let Source::Zip { layers, systems, .. } = &s.source else {
        panic!("an archive")
    };
    assert_eq!(layers.len(), members.len());
    assert!(systems.iter().all(|(_, sys)| sys == "EPSG:5256"), "{systems:?}");
    let read = s.result.as_ref().expect("read");
    assert_eq!(read.layers.len(), members.len(), "every layer read: {:?}", read.layers.iter().map(|l| &l.name).collect::<Vec<_>>());
    let objects = read.entities.len();
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    shot.settle(&mut app, App::view, &mut |app: &mut App, m| drop(app.update(m)));
    let head = format!("Bütün katmanlar ({})", members.len());
    assert!(crate::files_testing::find_text(&mut shot, &app, &head).is_some(), "{head}");
    crate::files_testing::evidence(&mut shot, &app, "cbs-kullanim", "gis-042-zip-katmanlar");
    let before = count(&app);
    import(&mut app);
    assert_eq!(count(&app) - before, objects, "every layer's objects");
    let model = &app.document.as_ref().expect("open").model;
    let row = model.journal().last().expect("a row");
    assert_eq!(row.command.as_deref(), Some("file.import.shp"));
    let input = row.input.clone().expect("the input");
    assert!(input.get("entry").is_none(), "{input}");
    let doc = app.document.as_mut().expect("open");
    assert!(doc.model.undo().is_some());
    assert_eq!(count(&app), before, "one undo step");
    // Layers that name different systems: one at a time, as before.
    let mut app = app_with_drawing();
    app.picker = Picker::File(gis("katmanlar.zip"));
    run(&mut app, "file.import.shp");
    assert_eq!(window(&app).result.as_ref().expect("read").layers.len(), 1);
}
