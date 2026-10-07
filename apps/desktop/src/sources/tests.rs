//! Kaynaklar as the user works it (docs/adr/0199 §7): a folder's rows from
//! the disk, the kept list, a row opened and closed, KentOS without an
//! account, and a project's layer taken with its objects as one undo step.
//! The listing's rules are the shared cases' (crates/native/interaction).

use std::path::Path;
use std::sync::Arc;

use kentos_contracts::{DocumentSnapshotV2, ProjectStorage};

use super::view::Line;
use super::{Event, ProjectDrawing, Read, SourceFolders, folder_key, project_key, read_folder};
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
        rows(&app)[..7],
        [
            "▾ Klasörler".to_owned(),
            format!("  ▾ {name}"),
            "    ▸ Pafta 2".to_owned(),
            "    noktalar.ncn · Koordinat listesi".to_owned(),
            "    parsel.shp · Shapefile".to_owned(),
            "    yol.dxf · DXF".to_owned(),
            "▾ KentOS".to_owned(),
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
            for scene in ["kaynaklar", "kaynaklar-bos"] {
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
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
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
