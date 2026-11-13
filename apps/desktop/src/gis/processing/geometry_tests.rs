//! The geometry tools in the desktop's window (docs/adr/0201), on the shared
//! geometry drawing (fixtures/processing/v1/geometry.kcad): Tampon's rings on
//! a layer of their own in one step, Geçerliliği denetle's table and
//! selection, Koordinat sistemine dönüştür's refusal of the project's own
//! system, Topolojiyi kurallarla denetle's findings on the topology drawing
//! (fixtures/processing/v1/topology.kcad, docs/adr/1980) and, asked to, on an
//! error layer in one step (docs/adr/2200), and the pictures for the owner.

use crate::gis;
use kentos_commands::DocumentSnapshotV1;
use serde_json::json;

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const GEOMETRY: &str = include_str!("../../../../../fixtures/processing/v1/geometry.kcad");
const TOPOLOGY: &str = include_str!("../../../../../fixtures/processing/v1/topology.kcad");
const ZONING: &str = include_str!("../../../../../fixtures/processing/v1/zoning-u-yol.kcad");
const LINKS: &str = include_str!("../../../../../fixtures/processing/v1/scenario-links.kcad");

/// The app with the geometry drawing open.
fn app_with_geometry() -> App {
    app_with(GEOMETRY)
}

/// The app with a processing drawing open: the v1 form, or the v2 form with its objects' lasting ids (topology.kcad,
/// docs/adr/2200).
fn app_with(text: &str) -> App {
    let (mut app, _) = App::boot(None);
    let v2 = serde_json::from_str::<serde_json::Value>(text).ok().and_then(|v| v["version"].as_u64()) == Some(2);
    let doc = if v2 {
        let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(text).expect("the drawing reads");
        Document::from_v2(snapshot, None).expect("the drawing opens")
    } else {
        let snapshot = DocumentSnapshotV1::from_json(text).expect("the drawing reads");
        Document::new(snapshot, None).expect("the drawing opens")
    };
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Processing(e)));
}

fn value(app: &mut App, name: &str, v: serde_json::Value) {
    event(app, Event::Value(name.into(), v));
}

fn status(app: &App) -> RunStatus {
    app.gis
        .processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

fn layer(id: &str) -> serde_json::Value {
    json!({ "scope": "layer", "layerId": id })
}

#[test]
fn buffer_rings_go_to_a_layer_of_their_own_in_one_step() {
    let mut app = app_with_geometry();
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    value(&mut app, "input", layer("agac"));
    value(&mut app, "distance", json!(2));
    value(&mut app, "rings", json!(2));
    value(&mut app, "dissolve", json!(true));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "3 nesnenin tamponu birleştirilerek yazıldı (2 alan).");
    let doc = &mut app.document.as_mut().expect("open").model;
    let layer = doc
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == "Tampon")
        .map(|l| l.id.clone())
        .expect("the new layer");
    assert_eq!(
        doc.entities()
            .filter(|e| e.base().layer_id == layer)
            .count(),
        2
    );
    assert_eq!(doc.undo().as_deref(), Some("Tampon"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "Tampon"));
}

#[test]
fn the_validity_check_shows_its_table_selects_and_changes_nothing() {
    let mut app = app_with_geometry();
    let _ = app.update(Message::Run("processing.run.geometry.validity"));
    value(&mut app, "input", layer("hatali"));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, table, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(
        text,
        "6 nesneden 5 nesnede 5 sorun bulundu; sorunlu nesneler seçildi."
    );
    let table = table.expect("the table");
    assert_eq!(table.columns, ["Nesne", "Katman", "Sorun", "Doğu", "Kuzey"]);
    assert_eq!(
        table.rows[0],
        [
            "#12",
            "Hatalı",
            "Kendini kesen halka",
            "487075.000",
            "4420005.000"
        ]
    );
    let selected: Vec<u32> = app.selection.ids().iter().map(|s| s.0).collect();
    assert_eq!(selected, [12, 13, 14, 15, 16]);
    assert!(!app.document.as_ref().expect("open").model.can_undo());
}

/// Topolojiyi kurallarla denetle (docs/adr/1980): the project's two rules from the toolbox give cad.topology.check's
/// findings in the table, select their objects and change nothing.
#[test]
fn the_topology_rules_from_the_toolbox_give_the_commands_findings() {
    let mut app = app_with(TOPOLOGY);
    let _ = app.update(Message::Run("processing.run.topology.check"));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, table, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "2 kural 5 nesnede 3 bulgu verdi (3 değerlendirilmedi); bulgulu nesneler seçildi.");
    let table = table.expect("the table");
    assert_eq!(table.columns, ["Kural", "Sorun", "Durum", "Nesneler", "Doğu", "Kuzey", "Ölçü", "Gerekçe"]);
    assert_eq!(
        table.rows[2],
        ["r2", "Kısa kenar", "Değerlendirilmedi", "#5", "487050.250", "4420010.000", "0.500 m", ""]
    );
    let selected: Vec<u32> = app.selection.ids().iter().map(|s| s.0).collect();
    assert_eq!(selected, [1, 2, 3, 4, 5]);
    assert!(!app.document.as_ref().expect("open").model.can_undo());
}

/// Bulguları hata katmanına yaz (docs/adr/2200): each finding a point at its place on Topoloji bulguları, with the
/// table's fields, the objects' lasting ids and the finding's key; one undo step, nothing selected.
#[test]
fn the_findings_go_to_an_error_layer_in_one_step() {
    let mut app = app_with(TOPOLOGY);
    let _ = app.update(Message::Run("processing.run.topology.check"));
    value(&mut app, "write", json!(true));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, table, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "2 kural 5 nesnede 3 bulgu verdi (3 değerlendirilmedi); 3 bulgu hata katmanına yazıldı.");
    assert_eq!(table.expect("the table").rows.len(), 3);
    assert!(app.selection.ids().is_empty(), "the points are the result, not the objects' selection");
    let doc = &mut app.document.as_mut().expect("open").model;
    let layer = doc
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == "Topoloji bulguları")
        .map(|l| l.id.clone())
        .expect("the error layer");
    let points: Vec<_> = doc.entities().filter(|e| e.base().layer_id == layer).cloned().collect();
    assert_eq!(points.len(), 3);
    let kentos_commands::Entity::Point(short) = &points[2] else {
        panic!("{:?}", points[2]);
    };
    assert_eq!((short.p.x, short.p.y), (487050.25, 4420010.0));
    let attrs = &short.base.attrs;
    assert_eq!(attrs["Kural"], "r2");
    assert_eq!(attrs["Sorun"], "Kısa kenar");
    assert_eq!(attrs["Ölçü"], "0.500 m");
    assert_eq!(attrs["Nesneler"], "0192f5c0-2200-7000-8000-000000000005");
    assert_eq!(attrs["Anahtar"].len(), 16);
    assert!(!attrs.contains_key("Gerekçe"));
    assert_eq!(doc.undo().as_deref(), Some("Topolojiyi kurallarla denetle"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "Topoloji bulguları"));
    assert!(!doc.can_undo());
}

/// İmar planını denetle (docs/adr/3840) on the U road's drawing: the toolbox's tool, its one finding in the table;
/// Bulguları hata katmanına yaz writes its two regions as areas on İmar bulguları (1/2, 2/2, the finding's fields),
/// one undo step, nothing selected.
#[test]
fn the_zoning_regions_go_to_an_error_layer_in_one_step() {
    let mut app = app_with(ZONING);
    let _ = app.update(Message::Run("processing.run.zoning.check"));
    value(&mut app, "write", json!(true));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, table, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert!(text.ends_with("1 bulgu verdi; 1 bulgunun 2 bölgesi hata katmanına yazıldı."), "{text}");
    let table = table.expect("the table");
    assert_eq!((table.rows.len(), table.rows[0][0].as_str(), table.rows[0][3].as_str()), (1, "Yola giren karar", "2"));
    assert!(app.selection.ids().is_empty(), "the areas are the result, not the objects' selection");
    let doc = &mut app.document.as_mut().expect("open").model;
    let layer = doc
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == "İmar bulguları")
        .map(|l| l.id.clone())
        .expect("the error layer");
    let areas: Vec<_> = doc.entities().filter(|e| e.base().layer_id == layer).cloned().collect();
    assert_eq!(areas.len(), 2);
    for (k, a) in areas.iter().enumerate() {
        let kentos_commands::Entity::Polygon(p) = a else { panic!("{a:?}") };
        assert_eq!(p.base.attrs["Bölge"], format!("{}/2", k + 1));
        assert_eq!(p.base.attrs["Tür"], "Yola giren karar");
        assert!(p.base.attrs["İleti"].contains("giriyor"), "{:?}", p.base.attrs);
    }
    assert_eq!(doc.undo().as_deref(), Some("İmar planını denetle"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "İmar bulguları"));
}

/// docs/adr/4640: İmar planını denetle with Bulguları hata katmanına yaz and Sonucun yeri a GeoPackage: the run is made on
/// a copy, the two regions go to the file's İmar bulguları table, the message says so, the drawing does not change.
#[test]
fn the_zoning_error_layer_goes_to_a_geopackage() {
    use super::sink::{Place, SinkEvent, run_to_sink};
    let mut app = app_with(ZONING);
    let _ = app.update(Message::Run("processing.run.zoning.check"));
    value(&mut app, "write", json!(true));
    let dir = crate::files_testing::scratch("imar-bulgulari-gpkg");
    let out = dir.join("imar-bulgulari.gpkg");
    event(&mut app, Event::Sink(SinkEvent::Place(Place::File)));
    event(&mut app, Event::Sink(SinkEvent::Path(out.display().to_string())));
    let before = app.document.as_ref().expect("open").model.revision();
    let input = app.processing_sink_input().expect("an input");
    event(&mut app, Event::Run);
    let mut copy = app.document.as_ref().expect("open").model.reading_copy();
    let end = run_to_sink(&mut copy, &input, std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    event(&mut app, Event::Sink(SinkEvent::Ran(end)));
    let RunStatus::Ok { text, undo, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert!(!undo);
    assert!(text.contains("1 bulgunun 2 bölgesi hata katmanına yazıldı."), "{text}");
    assert!(text.contains("Sonuç imar-bulgulari.gpkg dosyasına yazıldı: İmar bulguları (2 satır)."), "{text}");
    assert!(out.exists(), "the file is written");
    assert_eq!(app.document.as_ref().expect("open").model.revision(), before, "the drawing does not change");
}

/// İmar planını denetle with Bulguları hata katmanına yaz on the drawing with four decisions and three parcels in a
/// wide road, then the error layer on the drawing; `.run/gui-evidence/imar-hata-3840/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib processing::geometry_tests::zoning_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn zoning_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/imar-hata-3840");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1440.0, 900.0);
    let mut app = app_with(include_str!("../../../../../fixtures/processing/v1/zoning-yollar.kcad"));
    app.program = Some(kentos_launch::Module::Map);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let _ = app.update(Message::Run("processing.run.zoning.check"));
    value(&mut app, "write", json!(true));
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("1-pencere-acik-genis.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
    event(&mut app, Event::Run);
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("2-sonuc-acik-genis.png");
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes");
    println!("{}", file.display());
    app.gis.processing.dialog = None;
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("3-cizimde-acik-genis.png");
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes");
    println!("{}", file.display());
}

/// Altyapı bağlarını çiz (docs/adr/3860) on the scenario case's drawing: Alternatif A's areas and the field state's
/// joined to the sewer line 40 m; eleven lines on Altyapı bağları, each from an area's centroid to the line's nearest
/// place with the side, the area, the line and the distance; one undo step.
#[test]
fn the_scenario_s_areas_are_joined_to_the_line_in_one_step() {
    let mut app = app_with(LINKS);
    let _ = app.update(Message::Run("processing.run.scenarios.links"));
    value(&mut app, "scenario", json!("Alternatif A"));
    value(&mut app, "infrastructure", layer("altyapi"));
    value(&mut app, "distance", json!(40));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, table, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "“Alternatif A”: 11 alan en yakın altyapı nesnesine bağlandı; 11 bağ yazıldı.");
    assert_eq!(table.expect("the table").rows.len(), 11);
    let doc = &mut app.document.as_mut().expect("open").model;
    let id = doc.layers().leaves().into_iter().find(|l| l.name == "Altyapı bağları").map(|l| l.id.clone()).expect("the layer");
    let lines: Vec<_> = doc.entities().filter(|e| e.base().layer_id == id).cloned().collect();
    assert_eq!(lines.len(), 11);
    let kentos_commands::Entity::Line(first) = &lines[0] else { panic!("{:?}", lines[0]) };
    assert_eq!((first.a.x, first.a.y, first.b.x, first.b.y), (487020.0, 4420025.0, 487020.0, 4420060.0));
    assert_eq!((first.base.attrs["Taraf"].as_str(), first.base.attrs["Uzaklık"].as_str()), ("Mevcut durum", "35.000 m"));
    assert_eq!(doc.undo().as_deref(), Some("Altyapı bağlarını çiz"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "Altyapı bağları"));
}

/// Altyapı bağlarını çiz's window after its run and the links on the drawing, `.run/gui-evidence/altyapi-baglari-3860/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib processing::geometry_tests::links_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn links_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/altyapi-baglari-3860");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1440.0, 900.0);
    let mut app = app_with(LINKS);
    app.program = Some(kentos_launch::Module::Map);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let _ = app.update(Message::Run("processing.run.scenarios.links"));
    value(&mut app, "scenario", json!("Alternatif A"));
    value(&mut app, "infrastructure", layer("altyapi"));
    value(&mut app, "distance", json!(40));
    value(&mut app, "side", json!("base"));
    event(&mut app, Event::Run);
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("1-pencere-acik-genis.png");
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes");
    println!("{}", file.display());
    app.gis.processing.dialog = None;
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("2-cizimde-acik-genis.png");
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes");
    println!("{}", file.display());
}

#[test]
fn reprojecting_from_the_projects_own_system_is_refused() {
    let mut app = app_with_geometry();
    let _ = app.update(Message::Run("processing.run.geometry.reproject"));
    value(&mut app, "input", layer("eski"));
    value(&mut app, "source", json!("5254"));
    event(&mut app, Event::Run);
    assert_eq!(
        status(&app),
        RunStatus::Error(
            "Kaynak sistem projenin sistemiyle aynı; dönüştürülecek bir şey yok.".into()
        )
    );
    assert!(!app.document.as_ref().expect("open").model.can_undo());
}

/// The geometry tools' windows for the owner, after a run, and two results in
/// the drawing: Tampon (rings around the roads), Kesişim, Geçerliliği denetle
/// with its table, Onar with its report, Sadeleştir, Koordinat sistemine
/// dönüştür with its systems' list open. Not run by default:
/// `cargo test -p kentos-desktop processing::geometry_tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "tampon",
        "tampon-cizim",
        "kesisim",
        "kesisim-cizim",
        "gecerlilik",
        "onar",
        "sadelestir",
        "donustur",
        "topoloji",
        "topoloji-hata",
        "topoloji-hata-cizim",
    ];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in views {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let mut app =
                    if name.starts_with("topoloji") { app_with(TOPOLOGY) } else { app_with_geometry() };
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "tampon" | "tampon-cizim" => {
                        let _ = app.update(Message::Run("processing.run.geometry.buffer"));
                        value(&mut app, "input", layer("yol"));
                        value(&mut app, "distance", json!(3));
                        value(&mut app, "rings", json!(2));
                        value(&mut app, "dissolve", json!(true));
                        event(&mut app, Event::Run);
                    }
                    "kesisim" | "kesisim-cizim" => {
                        let _ = app.update(Message::Run("processing.run.geometry.intersection"));
                        value(&mut app, "input", layer("parsel"));
                        value(&mut app, "overlay", layer("imar"));
                        value(&mut app, "prefix", json!("İmar "));
                        value(&mut app, "apportion", json!("Değer"));
                        event(&mut app, Event::Run);
                    }
                    "gecerlilik" => {
                        let _ = app.update(Message::Run("processing.run.geometry.validity"));
                        value(&mut app, "input", layer("hatali"));
                        event(&mut app, Event::Run);
                    }
                    "onar" => {
                        let _ = app.update(Message::Run("processing.run.geometry.repair"));
                        value(&mut app, "input", layer("hatali"));
                        event(&mut app, Event::Run);
                    }
                    "topoloji" => {
                        let _ = app.update(Message::Run("view.zoomExtents"));
                        let _ = app.update(Message::Run("processing.run.topology.check"));
                        event(&mut app, Event::Run);
                    }
                    "topoloji-hata" | "topoloji-hata-cizim" => {
                        let _ = app.update(Message::Run("view.zoomExtents"));
                        let _ = app.update(Message::Run("processing.run.topology.check"));
                        value(&mut app, "write", json!(true));
                        event(&mut app, Event::Run);
                    }
                    "sadelestir" => {
                        let _ = app.update(Message::Run("processing.run.geometry.simplify"));
                        value(&mut app, "input", layer("sinir"));
                        value(&mut app, "tolerance", json!(0.05));
                        event(&mut app, Event::Run);
                    }
                    _ => {
                        let _ = app.update(Message::Run("processing.run.geometry.reproject"));
                        value(&mut app, "input", layer("eski"));
                        value(&mut app, "source", json!("2320"));
                        event(&mut app, Event::Run);
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "topoloji-hata-cizim" {
                    // The window closed: the three error points on the parcels.
                    event(&mut app, Event::Close);
                    let _ = app.update(Message::Run("view.zoomExtents"));
                    snapshot.settle(&mut app, App::view, &mut update);
                } else if name.ends_with("-cizim") {
                    // The window closed, the view on everything but the old sheet (17 km away).
                    event(&mut app, Event::Close);
                    let near: Vec<kentos_domain::Slot> = app
                        .document
                        .as_ref()
                        .expect("open")
                        .model
                        .entities()
                        .filter(|e| e.base().layer_id != "eski")
                        .map(|e| kentos_domain::Slot(e.base().id))
                        .collect();
                    app.selection.set(near);
                    let _ = app.update(Message::Run("view.zoomSelection"));
                    app.selection.clear();
                    // Kesişim's pieces lie on the zoning areas: the inputs hidden, the pieces show.
                    if name == "kesisim-cizim" {
                        let doc = &mut app.document.as_mut().expect("open").model;
                        doc.set_layer_visible("parsel", false);
                        doc.set_layer_visible("imar", false);
                    }
                    snapshot.settle(&mut app, App::view, &mut update);
                } else if matches!(name, "gecerlilik" | "onar" | "sadelestir") {
                    // The form scrolled to its end: the table.
                    snapshot.step(
                        &mut app,
                        App::view,
                        &mut update,
                        &[
                            iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                                position: iced::Point::new(width / 2.0 - 120.0, height / 2.0),
                            }),
                            iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                                delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: -40.0 },
                            }),
                        ],
                    );
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!(
                    "islem-geometri-{name}-{width}x{height}{suffix}.png"
                ));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// UR-067: an inward buffer of parcels drawn by a categorized fill: its new layer goes right above the parcels', so
/// the results are drawn over the fill rather than hidden under it, and its own look draws them.
#[test]
fn an_inward_buffer_result_goes_above_its_source() {
    use kentos_native_application::{ExecutionContext, layers_renderer};
    let mut app = app_with_geometry();
    let doc = &mut app.document.as_mut().expect("open").model;
    let input = kentos_commands::LayersRenderer {
        layer: "parsel".into(),
        renderer: Some(json!({ "type": "categorized", "expr": "Tür", "categories": [
            { "value": "Konut", "label": "Konut", "symbols": { "fill": { "type": "fill", "color": "#ff0000" } } },
            { "value": "Ticaret", "label": "Ticaret", "symbols": { "fill": { "type": "fill", "color": "#0000ff" } } }
        ] })),
        expected_revision: None,
    };
    assert!(matches!(
        layers_renderer::execute(&mut ExecutionContext::new(doc), input),
        kentos_commands::CommandResult::Completed { .. }
    ));
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    value(&mut app, "input", layer("parsel"));
    value(&mut app, "distance", json!(-1));
    event(&mut app, Event::Run);
    let RunStatus::Ok { pick, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    let doc = &app.document.as_ref().expect("open").model;
    let out_layer = doc.get(pick[0]).expect("a result").base().layer_id.clone();
    let order: Vec<String> = doc.layers().leaves().iter().map(|l| l.id.clone()).collect();
    let at = |id: &str| order.iter().position(|l| l == id).expect("in the tree");
    // The tree's first is drawn on top: the results' layer right above the parcels'.
    assert_eq!(at(&out_layer) + 1, at("parsel"), "{order:?}");
    let uids: Vec<String> = pick.iter().filter_map(|s| doc.uid(*s)).map(|u| u.to_string()).collect();
    assert_eq!(app.unstyled_note(&uids), None, "its own look draws every result");
}
