//! Kaynaklar as the user works it (docs/adr/0199 §7): a folder's rows from
//! the disk, the kept list, a row opened and closed, KentOS without an
//! account, and a project's layer taken with its objects as one undo step.
//! The listing's rules are the shared cases' (crates/native/interaction).

use std::path::Path;
use std::sync::Arc;

use kentos_commands::{DocumentSnapshotV2, ProjectStorage};

use super::view::Line;
use super::{
    Event, Kept, Origin, PRESETS, ProjectDrawing, Read, SavedService, SourceFolders, folder_key,
    project_key, read_folder, table_key, tables_key,
};
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, last_said};

fn send(app: &mut App, event: Event) {
    let _ = app.update(Message::Sources(event));
}

/// The rows as words: a caret's “▸”/“▾” for what opens, then the name and its meta.
fn rows(app: &App) -> Vec<String> {
    app.source_lines()
        .into_iter()
        .map(|l| match l {
            Line::Section { label, open, .. } => {
                format!("{} {label}", if open { "▾" } else { "▸" })
            }
            Line::Folder {
                label, open, depth, ..
            } => format!(
                "{}{} {label}",
                "  ".repeat(depth),
                if open { "▾" } else { "▸" }
            ),
            Line::File { file, depth, .. } => {
                format!(
                    "{}{} · {}",
                    "  ".repeat(depth),
                    file.name,
                    file.kind.label()
                )
            }
            Line::List {
                label, open, depth, ..
            } => format!(
                "{}{} {label}",
                "  ".repeat(depth),
                if open { "▾" } else { "▸" }
            ),
            Line::Project {
                project,
                open,
                depth,
                ..
            } => format!(
                "{}{} {}",
                "  ".repeat(depth),
                if open { "▾" } else { "▸" },
                project.name
            ),
            Line::Group {
                label, open, depth, ..
            } => format!(
                "{}{} {label}",
                "  ".repeat(depth),
                if open { "▾" } else { "▸" }
            ),
            Line::Layer {
                label,
                count,
                depth,
                ..
            } => format!("{}{label} · {count} nesne", "  ".repeat(depth)),
            Line::Note { label, depth, .. } => format!("{}({label})", "  ".repeat(depth)),
            Line::Database {
                connection, open, ..
            } => format!("  {} {}", if open { "▾" } else { "▸" }, connection.name),
            Line::FileTable {
                table,
                bound,
                depth,
                open,
                ..
            } => format!(
                "{}{} {}{}",
                "  ".repeat(depth),
                if open { "▾" } else { "▸" },
                table.name,
                bound.map_or_else(String::new, |(_, l)| format!(" · Bağlı: {l}"))
            ),
            Line::Branch {
                label, open, depth, ..
            } => format!(
                "{}{} {label}",
                "  ".repeat(depth),
                if open { "▾" } else { "▸" }
            ),
            Line::Service {
                service,
                open,
                depth,
                ..
            } => format!(
                "{}{} {} · {}",
                "  ".repeat(depth),
                if open { "▾" } else { "▸" },
                service.name,
                service.kind.label()
            ),
            Line::Kept { kept, depth, .. } => {
                format!("{}{} · {}", "  ".repeat(depth), kept.label(), kept.meta())
            }
            Line::Extent { extent, depth, .. } => {
                format!("{}[kapsam {:?}]", "  ".repeat(depth), extent)
            }
            Line::Preset { name, depth, .. }
            | Line::ServiceLayer {
                title: name, depth, ..
            } => {
                format!("{}{name}", "  ".repeat(depth))
            }
            Line::Schema {
                label, open, depth, ..
            } => format!(
                "{}{} {label}",
                "  ".repeat(depth),
                if open { "▾" } else { "▸" }
            ),
            Line::Table {
                relation, depth, ..
            } => format!("{}{}", "  ".repeat(depth), relation.name),
        })
        .collect()
}

/// A folder of files and a folder inside it, under the test's own temporary place.
fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-kaynaklar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Pafta 2")).expect("a folder");
    for f in [
        "parsel.shp",
        "parsel.dbf",
        "parsel.prj",
        "yol.dxf",
        "noktalar.ncn",
        "rapor.pdf",
        ".gizli.dxf",
    ] {
        std::fs::write(dir.join(f), b"x").expect("a file");
    }
    std::fs::write(dir.join("Pafta 2").join("imar.ncz"), b"x").expect("a file");
    dir
}

/// A folder added, read as the off-thread read would read it.
fn add(app: &mut App, dir: &Path) {
    send(app, Event::FolderPicked(Some(dir.to_path_buf())));
    send(app, Event::Listed(dir.to_path_buf(), read_folder(dir)));
}

#[test]
fn a_folder_shows_its_folders_and_the_files_it_adds_and_reads_again_when_opened_again() {
    let mut app = app_with_drawing();
    let dir = folder("satirlar");
    add(&mut app, &dir);
    let name = super::folder_name(&dir);
    assert_eq!(
        rows(&app)[..10],
        [
            "▾ Klasörler".to_owned(),
            format!("  ▾ {name}"),
            "    ▸ Pafta 2".to_owned(),
            "    noktalar.ncn · Koordinat listesi".to_owned(),
            "    parsel.shp · Shapefile".to_owned(),
            "    yol.dxf · DXF".to_owned(),
            // No connection yet: the database section offers one (docs/adr/0623).
            "▾ Veritabanları".to_owned(),
            "  (Bağlantı yok. Bir PostgreSQL veritabanı ekleyin.)".to_owned(),
            "  (Size verilen bir bağlantı dosyasını içe aktarın…)".to_owned(),
            "▾ Servisler".to_owned(),
        ]
    );
    // Closing forgets what was read; opening reads again.
    send(&mut app, Event::Toggle(folder_key(&dir)));
    assert!(!app.sources.listings.contains_key(&dir));
    let _ = app.update(Message::Sources(Event::Toggle(folder_key(&dir))));
    assert!(matches!(
        app.sources.listings.get(&dir),
        Some(Read::Reading)
    ));
    // A folder inside opens with its own files.
    send(&mut app, Event::Listed(dir.clone(), read_folder(&dir)));
    let inner = dir.join("Pafta 2");
    send(&mut app, Event::Toggle(folder_key(&inner)));
    send(&mut app, Event::Listed(inner.clone(), read_folder(&inner)));
    assert!(
        rows(&app).contains(&"      imar.ncz · Netcad NCZ".to_owned()),
        "{:?}",
        rows(&app)
    );
    // An answer for a row closed since is dropped.
    send(&mut app, Event::Toggle(folder_key(&inner)));
    send(&mut app, Event::Listed(inner.clone(), read_folder(&inner)));
    assert!(!app.sources.listings.contains_key(&inner));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_folders_are_kept_and_one_taken_off_the_list_stays_on_the_disk() {
    let dir = folder("liste");
    let keep = dir.join("durum");
    let mut kept = SourceFolders::open(&keep);
    assert!(kept.add(&dir));
    assert!(!kept.add(&dir), "once");
    assert_eq!(
        SourceFolders::open(&keep).list(),
        std::slice::from_ref(&dir)
    );
    let mut app = app_with_drawing();
    app.sources.folders = SourceFolders::open(&keep);
    send(&mut app, Event::Remove(dir.clone()));
    assert!(last_said(&app).contains("listeden kaldırıldı"));
    assert!(SourceFolders::open(&keep).list().is_empty());
    assert!(dir.join("yol.dxf").exists(), "the folder stays");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn without_an_account_kentos_asks_to_sign_in() {
    let app = app_with_drawing();
    let all = rows(&app);
    assert_eq!(
        all.last().map(String::as_str),
        Some("  (Projeleri görmek için giriş yapın.)")
    );
    assert!(all.contains(&"  (Klasör yok. Klasör ekle ile bir klasör ekleyin.)".to_owned()));
}

/// The exchange cases' source drawing, as a project downloaded.
fn downloaded() -> Arc<ProjectDrawing> {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/exchange/v1/cases.json"))
            .expect("the cases");
    let drawing: DocumentSnapshotV2 =
        serde_json::from_value(cases["drawings"]["theirs"].clone()).expect("a drawing");
    let mut counts = std::collections::HashMap::new();
    for e in &drawing.entities {
        *counts.entry(e.base().layer_id.clone()).or_insert(0) += 1;
    }
    Arc::new(ProjectDrawing { drawing, counts })
}

#[test]
fn a_projects_layer_comes_with_its_objects_as_one_undo_step() {
    let mut app = crate::cloud::tests::signed_in();
    let doc = crate::files_testing::app_with_drawing().document;
    app.document = doc;
    let p = crate::cloud::tests::summary(
        "0192f5a0-0000-7000-8000-000000000001",
        "Kaynak",
        ProjectStorage::Database,
    );
    let id = p.id.clone();
    app.sources.open.insert(super::list_key("mine"));
    app.sources.lists.insert("mine", Read::Ready((vec![p], 1)));
    app.sources.open.insert(project_key(&id));
    app.sources
        .drawings
        .insert(id.clone(), Read::Ready(downloaded()));
    let all = rows(&app);
    assert!(all.contains(&"  ▾ Projelerim".to_owned()), "{all:?}");
    assert!(all.contains(&"      ▾ Kadastro".to_owned()), "{all:?}");
    assert!(
        all.contains(&"        Parsel · 5 nesne".to_owned()),
        "{all:?}"
    );
    assert!(all.contains(&"      Yol · 2 nesne".to_owned()), "{all:?}");
    let before = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .count();
    send(&mut app, Event::AddLayer(id, "Yol".to_owned()));
    assert_eq!(
        last_said(&app),
        "“Kaynak” içinden “Yol” alındı: 2 nesne, 2 yeni katman ya da grup, 2 blok (tek adımda geri alınır)."
    );
    let model = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(model.entities().count(), before + 2);
    // Yol, and 0 where the lamp's post (the block Direk inside Lamba) is drawn.
    assert!(model.layers().leaves().iter().any(|l| l.name == "Yol"));
    assert!(model.layers().leaves().iter().any(|l| l.name == "0"));
    let _ = app.update(Message::Run("edit.undo"));
    let model = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(model.entities().count(), before);
    assert!(!model.layers().leaves().iter().any(|l| l.name == "Yol"));
}

// ── The source browser (docs/adr/0599) ───────────────────────────────

fn fixture_gpkg() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/geopackage/v1/qgis.gpkg")
}

/// A folder with a GeoPackage, a DXF, a raster, a point cloud, a FlatGeobuf and a GML.
fn browser_folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-gezgin-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    std::fs::copy(fixture_gpkg(), dir.join("kadastro.gpkg")).expect("a copy");
    for f in [
        "yol.dxf",
        "orto.tif",
        "bulut.laz",
        "binalar.fgb",
        "sinir.gml",
    ] {
        std::fs::write(dir.join(f), b"x").expect("a file");
    }
    dir
}

/// The GeoPackage of `dir` opened, its tables read as the off-thread read reads them.
fn open_tables(app: &mut App, dir: &Path) -> Origin {
    add(app, dir);
    let origin = Origin::File(dir.join("kadastro.gpkg"));
    send(app, Event::Toggle(tables_key(&origin)));
    let tables = crate::exchange::list_tables(origin.path());
    send(app, Event::Tables(origin.clone(), tables));
    origin
}

/// A table's preview opened, its rows read as the off-thread read reads them.
fn open_preview(app: &mut App, origin: &Origin, table: &str) {
    let key = table_key(origin, table);
    send(app, Event::Toggle(key.clone()));
    let columns = match app.sources.tables.get(origin) {
        Some(Read::Ready(t)) => t
            .iter()
            .find(|t| t.name == table)
            .map(|t| {
                t.columns
                    .iter()
                    .take(super::SAMPLE_COLUMNS)
                    .map(|c| c.name.clone())
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let settings = app.document.as_ref().map(|d| d.settings().clone());
    let rows = super::sample_rows(origin, table, columns, settings);
    send(app, Event::Sample(key, rows));
}

fn window_kind(app: &App) -> Option<(bool, Vec<bool>)> {
    match &app.exchange {
        Some(crate::exchange::Window::Gpkg(s)) => Some((s.bind, s.chosen.clone())),
        _ => None,
    }
}

#[test]
fn the_new_kinds_are_listed_and_a_geopackage_opens_into_its_tables_and_a_preview() {
    let mut app = app_with_drawing();
    let dir = browser_folder("tablolar");
    let origin = open_tables(&mut app, &dir);
    let all = rows(&app);
    for row in [
        "    binalar.fgb · FlatGeobuf",
        "    bulut.laz · Nokta bulutu (LAS, LAZ)",
        "    kadastro.gpkg · GeoPackage",
        "    orto.tif · Raster (GeoTIFF, VRT)",
        "    sinir.gml · GML",
        "      ▸ parsel",
    ] {
        assert!(all.contains(&row.to_owned()), "{row}: {all:?}");
    }
    open_preview(&mut app, &origin, "parsel");
    let all = rows(&app);
    assert!(
        all.contains(&"        [kapsam [487000.0, 4420000.0, 487110.0, 4420020.0]]".to_owned()),
        "{all:?}"
    );
    let samples: Vec<&String> = all
        .iter()
        .filter(|r| r.starts_with("        (ada: "))
        .collect();
    assert_eq!(samples.len(), 3, "the table's three rows: {all:?}");
    assert!(samples[0].contains("parsel_no: "), "{samples:?}");
    // Closed, what was read is forgotten and read again when opened.
    send(&mut app, Event::Toggle(table_key(&origin, "parsel")));
    assert!(app.sources.samples.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_table_is_bound_from_the_browser_then_gone_to_and_never_bound_twice() {
    let mut app = app_with_drawing();
    let dir = browser_folder("bag");
    let origin = open_tables(&mut app, &dir);
    let names: Vec<String> = match app.sources.tables.get(&origin) {
        Some(Read::Ready(t)) => t.iter().map(|t| t.name.clone()).collect(),
        _ => panic!("the tables"),
    };
    // Kopya olarak al: the import window, the table alone chosen, not bound.
    send(
        &mut app,
        Event::OpenTable(origin.clone(), "parsel".into(), false),
    );
    let (bind, chosen) = window_kind(&app).expect("the GeoPackage window");
    assert!(!bind);
    assert_eq!(
        chosen,
        names.iter().map(|n| n == "parsel").collect::<Vec<_>>()
    );
    app.exchange = None;
    // Bağlı katman olarak aç: the same window with the binding ticked; Çalıştır binds it.
    send(
        &mut app,
        Event::OpenTable(origin.clone(), "parsel".into(), true),
    );
    assert_eq!(window_kind(&app).map(|w| w.0), Some(true));
    let _ = app.update(Message::Exchange(Box::new(crate::exchange::Event::Gpkg(
        crate::exchange::gpkg::Event::Run,
    ))));
    assert!(app.exchange.is_none(), "{}", last_said(&app));
    let all = rows(&app);
    let bound = all
        .iter()
        .find(|r| r.contains("▸ parsel · Bağlı: "))
        .unwrap_or_else(|| panic!("the bound row: {all:?}"));
    assert!(bound.ends_with("parsel"), "{bound}");
    // Katmana git instead of a second binding: the layer chosen in Katmanlar.
    let id = app
        .bound_layer_of(&origin.path(), "parsel")
        .expect("the layer");
    send(&mut app, Event::GoToLayer(id.clone()));
    assert_eq!(app.selected_layer.as_deref(), Some(id.as_str()));
    // Both opened tables are the newest recent sources, once.
    assert_eq!(
        app.sources.folders.recent(),
        [Kept::Table {
            path: dir.join("kadastro.gpkg"),
            table: "parsel".into()
        }]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_services_list_the_basemaps_and_the_services_added_before() {
    let mut app = app_with_drawing();
    send(&mut app, Event::Toggle(PRESETS.to_owned()));
    let preset = &kentos_services::presets::catalog().presets[0];
    let all = rows(&app);
    assert!(all.contains(&"  ▾ Hazır altlıklar".to_owned()), "{all:?}");
    assert!(all.contains(&format!("    {}", preset.name)), "{all:?}");
    let layers = |app: &App| {
        app.document
            .as_ref()
            .map_or(0, |d| d.model.layers().leaves().len())
    };
    let before = layers(&app);
    let kept = Kept::Basemap {
        command: kentos_services::presets::command_of(&preset.id),
        name: preset.name.clone(),
    };
    send(&mut app, Event::Use(kept.clone()));
    assert_eq!(layers(&app), before + 1, "{}", last_said(&app));
    assert_eq!(app.sources.folders.recent(), [kept]);
    app.sources.folders.add_service(SavedService {
        kind: kentos_commands::ServiceKind::Wms,
        url: "https://cbs.ornek.gov.tr/geoserver/wms".into(),
        name: "Belediye WMS".into(),
        connection: None,
    });
    let all = rows(&app);
    assert!(all.contains(&"  ▾ Kayıtlı servisler".to_owned()), "{all:?}");
    assert!(
        all.contains(&"    ▸ Belediye WMS · WMS".to_owned()),
        "{all:?}"
    );
    send(&mut app, Event::ServiceRemove(0));
    assert!(app.sources.folders.services().is_empty());
}

#[test]
fn the_search_keeps_the_rows_that_match_and_their_ancestors() {
    let mut app = app_with_drawing();
    let dir = browser_folder("arama");
    open_tables(&mut app, &dir);
    let name = super::folder_name(&dir);
    send(&mut app, Event::Search("multipolygon 5256".into()));
    let all = rows(&app);
    assert_eq!(
        all,
        [
            "(1 satır eşleşti. Yalnız açılmış satırlar aranır.)".to_owned(),
            "▾ Klasörler".to_owned(),
            format!("  ▾ {name}"),
            "    kadastro.gpkg · GeoPackage".to_owned(),
            "      ▸ parsel".to_owned(),
        ]
    );
    send(&mut app, Event::Search("RASTER".into()));
    assert!(rows(&app).contains(&"    orto.tif · Raster (GeoTIFF, VRT)".to_owned()));
    send(&mut app, Event::Search("böyle bir şey yok".into()));
    assert_eq!(
        rows(&app),
        ["(Eşleşen satır yok. Yalnız açılmış satırlar aranır.)"]
    );
    send(&mut app, Event::Search(String::new()));
    assert!(rows(&app).len() > 5);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_recent_and_favourite_sources_are_kept_with_the_folders() {
    let keep = std::env::temp_dir().join(format!("kentos-gezgin-liste-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&keep);
    std::fs::create_dir_all(&keep).expect("a folder");
    // A version 1 list is read.
    std::fs::write(
        keep.join("kaynak-klasorleri.json"),
        r#"{"format":"kentos.source-folders","version":1,"folders":["/veri/İmar"]}"#,
    )
    .expect("a list");
    let mut list = SourceFolders::open(&keep);
    assert_eq!(list.list(), [std::path::PathBuf::from("/veri/İmar")]);
    let a = Kept::File {
        path: "/veri/a.dxf".into(),
    };
    let b = Kept::Database {
        connection: "Belediye".into(),
        table: "public.parsel".into(),
    };
    list.used(&a);
    list.used(&b);
    list.used(&a);
    assert!(list.star(&b));
    let again = SourceFolders::open(&keep);
    assert_eq!(again.recent(), [a.clone(), b.clone()]);
    assert_eq!(again.favourites(), std::slice::from_ref(&b));
    let text = std::fs::read_to_string(keep.join("kaynak-klasorleri.json")).expect("the list");
    assert!(text.contains("\"version\": 2"), "{text}");
    // The rows: favourites under their section, a file no longer there said.
    let mut app = app_with_drawing();
    app.sources.folders = SourceFolders::open(&keep);
    let all = rows(&app);
    assert_eq!(all[0], "▾ Son kullanılanlar");
    assert!(all.contains(&"▾ Sık kullanılanlar".to_owned()), "{all:?}");
    send(&mut app, Event::Use(a));
    assert!(
        last_said(&app).contains("bulunamadı"),
        "{}",
        last_said(&app)
    );
    send(&mut app, Event::Star(b));
    assert!(app.sources.folders.favourites().is_empty());
    let _ = std::fs::remove_dir_all(&keep);
}

#[test]
fn a_row_dragged_onto_the_drawing_is_added_there_and_a_file_dropped_on_the_window_too() {
    let mut app = app_with_drawing();
    let dir = browser_folder("surukle");
    add(&mut app, &dir);
    app.viewport.bounds = iced::Rectangle::new(
        iced::Point::new(300.0, 100.0),
        iced::Size::new(600.0, 400.0),
    );
    let file = kentos_interaction::sources::SourceFile {
        name: "kadastro.gpkg".into(),
        kind: kentos_interaction::sources::SourceKind::GeoPackage,
        parts: vec!["kadastro.gpkg".into()],
    };
    let add_it = super::msg(Event::AddFile(dir.clone(), file));
    let key = "f:surukle".to_owned();
    // Let go over the panel: a click, nothing added.
    send(
        &mut app,
        Event::Press(
            key.clone(),
            Some(Box::new(super::msg(Event::Focus(key.clone())))),
            Box::new(add_it.clone()),
        ),
    );
    assert!(app.sources.drag.is_some());
    send(&mut app, Event::DragAt(iced::Point::new(40.0, 200.0)));
    send(&mut app, Event::DragEnd);
    assert!(app.sources.drag.is_none() && window_kind(&app).is_none());
    // Let go over the drawing: added as Katman olarak ekle adds it.
    send(&mut app, Event::Press(key, None, Box::new(add_it)));
    send(&mut app, Event::DragAt(iced::Point::new(500.0, 300.0)));
    send(&mut app, Event::DragEnd);
    assert_eq!(window_kind(&app).map(|w| w.0), Some(false));
    app.exchange = None;
    // A file from the system's file manager.
    send(&mut app, Event::Dropped(dir.join("kadastro.gpkg")));
    assert!(window_kind(&app).is_some());
    app.exchange = None;
    std::fs::write(dir.join("rapor.pdf"), b"x").expect("a file");
    send(&mut app, Event::Dropped(dir.join("rapor.pdf")));
    assert!(last_said(&app).contains("tanımıyor"), "{}", last_said(&app));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_extent_is_shown_framed_and_unmarked() {
    let mut app = app_with_drawing();
    let srid = app
        .document
        .as_ref()
        .map(|d| d.settings().srid)
        .expect("a drawing");
    let b = [487000.0, 4420000.0, 487110.0, 4420020.0];
    send(&mut app, Event::ShowExtent(None, b, "parsel".into()));
    if srid != 0 {
        assert!(
            last_said(&app).contains("bilinmiyor"),
            "{}",
            last_said(&app)
        );
        assert!(app.extent_mark().is_none());
    }
    send(
        &mut app,
        Event::ShowExtent(Some(srid).filter(|s| *s != 0), b, "parsel".into()),
    );
    let (shown, name) = app.extent_mark().expect("the frame");
    assert_eq!((shown, name.as_str()), (b, "parsel"));
    assert!(app.available("data.unmark"));
    let _ = app.update(Message::Run("data.unmark"));
    assert!(app.extent_mark().is_none());
}

/// The browser's pictures (docs/adr/0599): a GeoPackage's tables with one bound and one previewed, a database's
/// tables by schema, the services and the kept sources (`kaynaklar-gezgin`); the search (`kaynaklar-arama`); a
/// table's extent shown over the drawing (`kaynaklar-kapsam`).
fn browser_scene(app: &mut App, data: &Path, scene: &str) {
    let dir = data.join("İmar verisi");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).expect("a folder");
        std::fs::copy(fixture_gpkg(), dir.join("kadastro.gpkg")).expect("a copy");
        for f in ["Orto 2024.tif", "bulut.laz", "yol.dxf", "binalar.fgb"] {
            std::fs::write(dir.join(f), b"x").expect("a file");
        }
    }
    app.sources.folders = SourceFolders::memory();
    app.sources.open.remove(super::KENTOS);
    let origin = open_tables(app, &dir);
    // One table bound, as Bağlı katman olarak aç binds it.
    send(app, Event::OpenTable(origin.clone(), "parsel".into(), true));
    let _ = app.update(Message::Exchange(Box::new(crate::exchange::Event::Gpkg(
        crate::exchange::gpkg::Event::Run,
    ))));
    app.exchange = None;
    let tables = match app.sources.tables.get(&origin) {
        Some(Read::Ready(t)) => t.clone(),
        _ => Vec::new(),
    };
    if let Some(other) = tables
        .iter()
        .find(|t| t.name != "parsel")
        .map(|t| t.name.clone())
    {
        open_preview(app, &origin, &other);
    } else {
        open_preview(app, &origin, "parsel");
    }
    app.sources.folders.add_service(SavedService {
        kind: kentos_commands::ServiceKind::Wms,
        url: "https://cbs.ornek.gov.tr/geoserver/wms".into(),
        name: "Belediye imar planı (WMS)".into(),
        connection: None,
    });
    app.sources.folders.star(&Kept::Database {
        connection: "Belediye".into(),
        table: "kadastro.parsel".into(),
    });
    app.sources.folders.star(&Kept::Basemap {
        command: "basemap.osmStandard".into(),
        name: "OpenStreetMap".into(),
    });
    if scene == "kaynaklar-arama" {
        send(app, Event::Search("parsel 5256".into()));
    }
    // Veritabanları and Servisler in view: the sections above closed, the ready basemaps open.
    if scene == "kaynaklar-veritabani" {
        for key in [super::RECENT, super::FAVOURITES, super::FOLDERS] {
            send(app, Event::Toggle(key.to_owned()));
        }
        send(app, Event::Toggle(PRESETS.to_owned()));
    }
}

/// Kapsamı göster once the window has its size, as the user presses it.
fn browser_extent(app: &mut App) {
    let srid = app
        .document
        .as_ref()
        .map(|d| d.settings().srid)
        .filter(|s| *s != 0);
    let found = app.sources.tables.values().find_map(|r| match r {
        Read::Ready(t) => t
            .iter()
            .find(|t| t.name == "parsel" && t.extent.is_some())
            .map(|t| (t.extent.unwrap_or_default(), t.name.clone())),
        _ => None,
    });
    if let Some((extent, name)) = found {
        send(app, Event::ShowExtent(srid, extent, name));
    }
}

/// Kaynaklar as the owner looks at it, dark and light, at 1440×900 and
/// 1100×650 (`.run/shots/kaynaklar-*`; the web's `shots.mjs sources` and
/// cloud-shots.mjs's “sources-*”): a folder added with a folder inside it
/// open and KentOS's Projelerim with a project's layers (`kaynaklar`), and
/// the panel before anything (`kaynaklar-bos`):
///
/// ```text
/// cargo test -p kentos-desktop sources::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let data = std::env::temp_dir().join(format!("kentos-kaynaklar-resim-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let dir = data.join("Kaynak verisi");
    std::fs::create_dir_all(dir.join("Pafta 2")).expect("a folder");
    for f in [
        "parsel.shp",
        "parsel.shx",
        "parsel.dbf",
        "parsel.prj",
        "yollar.geojson",
        "imar.dxf",
        "pafta.ncz",
        "rota.gpx",
        "alim.nmea",
        "noktalar.ncn",
        "rapor.pdf",
        ".gizli.dxf",
    ] {
        std::fs::write(dir.join(f), b"x").expect("a file");
    }
    std::fs::write(dir.join("Pafta 2").join("kot.csv"), b"x").expect("a file");
    let layout = data.join("yerlesim");
    std::fs::create_dir_all(&layout).expect("a folder");
    std::fs::write(
        layout.join(crate::layout::FILE_NAME),
        serde_json::json!({ "dockTab": "sources", "layersFraction": 0.72 }).to_string(),
    )
    .expect("a kept layout");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in [
                "kaynaklar",
                "kaynaklar-bos",
                "kaynaklar-gezgin",
                "kaynaklar-arama",
                "kaynaklar-kapsam",
                "kaynaklar-veritabani",
            ] {
                let mut app = if scene == "kaynaklar" {
                    crate::cloud::tests::signed_in()
                } else {
                    App::boot(None).0
                };
                app.document = crate::files_testing::app_with_drawing().document;
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app.layout = crate::layout::Keeper::open(&layout);
                app.apply_layout();
                if scene == "kaynaklar" {
                    add(&mut app, &dir);
                    let inner = dir.join("Pafta 2");
                    send(&mut app, Event::Toggle(folder_key(&inner)));
                    send(&mut app, Event::Listed(inner.clone(), read_folder(&inner)));
                    let survey = crate::cloud::tests::summary(
                        "0192f5a0-0000-7000-8000-000000000001",
                        "Ada 1246 ölçü projesi",
                        ProjectStorage::Database,
                    );
                    let sheet = crate::cloud::tests::summary(
                        "0192f5a0-0000-7000-8000-000000000002",
                        "Kadastro paftası 2026",
                        ProjectStorage::File,
                    );
                    let id = survey.id.clone();
                    app.sources.open.insert(super::list_key("mine"));
                    app.sources
                        .lists
                        .insert("mine", Read::Ready((vec![survey, sheet], 2)));
                    app.sources.open.insert(project_key(&id));
                    app.sources
                        .drawings
                        .insert(id.clone(), Read::Ready(downloaded()));
                    app.sources.focused = Some(format!("y:{id}\0Yol"));
                }
                if scene.starts_with("kaynaklar-") && scene != "kaynaklar-bos" {
                    browser_scene(&mut app, &data, scene);
                }
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                // The area has its size now: Kapsamı göster pressed.
                if scene == "kaynaklar-kapsam" {
                    browser_extent(&mut app);
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
    let _ = std::fs::remove_dir_all(&data);
}
