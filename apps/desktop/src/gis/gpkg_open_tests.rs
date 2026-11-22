//! A GeoPackage opened as a project (docs/adr/9116): Aç on a file of no project opens a new project of its tables,
//! written into the file, and Kaydet writes there again; a file of several projects opens Veri kaynağından aç on them;
//! one dropped on a window with no drawing opens the same way; Veri kaynağından aç on a file of no project makes one
//! under the name typed.

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
