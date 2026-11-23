//! Kaynaklar (docs/adr/9192): a file's data layer, bound layer and the copy an import took, all under the file; the copy
//! keeps where it came from (its table, the file's SHA-256, when), says whether the file changed since and when it is
//! gone; the import's undo takes the copy and its origin; a project written and read again keeps the origin.

use std::path::Path;

use kentos_native_application::ExecutionContext;

use super::{Source, Way};
use crate::app::{App, Message};
use crate::data_layers_tests::parcels;
use crate::files_testing::{app_with_drawing, drive};

/// The file's table taken into the drawing as a copy, by `file.import.gpkg`.
fn import(app: &mut App, path: &Path) -> kentos_commands::GpkgImported {
    let input: kentos_commands::ImportGpkg =
        serde_json::from_value(serde_json::json!({ "path": path.display().to_string(), "tables": ["parseller"] }))
            .expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    match kentos_native_application::file_import_gpkg::execute(&mut ExecutionContext::new(&mut doc.model), input) {
        kentos_commands::CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

/// The sample's “parsel” layer bound to the file's table, as Kaynak ▸ Bağla leaves it.
fn bind(app: &mut App, path: &Path) {
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.sources.push(
        serde_json::from_value(serde_json::json!({
            "id": "kaynak-1",
            "kind": "geopackage",
            "path": path.display().to_string(),
            "table": "parseller",
            "layer": "parsel",
            "revision": "1".repeat(64),
            "digest": "0".repeat(64),
            "writable": true,
        }))
        .expect("a link"),
    );
    doc.model.set_settings(settings);
}

/// A file with a data layer, a bound layer and a copy of its table; the copy's layer id.
fn three_ways(tag: &str) -> (App, std::path::PathBuf, String) {
    crate::data_layers::register();
    let path = parcels(tag, 3);
    let mut app = app_with_drawing();
    // The copy first: a table goes onto the drawing's layer of its name, the data layer's too.
    let taken = import(&mut app, &path);
    let copy = taken.layers.first().expect("a layer made").id.clone();
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described")])
        .expect("added");
    bind(&mut app, &path);
    (app, path, copy)
}

fn the_file<'a>(catalog: &'a [Source], path: &Path) -> &'a Source {
    catalog.iter().find(|s| s.address == path.display().to_string()).expect("the file's source")
}

fn copy_state(app: &App, path: &Path) -> String {
    let catalog = app.source_catalog();
    let source = the_file(&catalog, path);
    let entry = source.entries.iter().find(|e| e.way == Way::Copy).expect("the copy");
    let digests = app.gis.source_status.as_ref().map(|w| w.digests.clone()).unwrap_or_default();
    App::copy_state(source, entry, &digests).1
}

#[test]
fn a_files_data_layer_bound_layer_and_copy_stand_under_the_file() {
    let (mut app, path, copy) = three_ways("katalog");
    // The copy keeps where its objects came from.
    let origin = app.document.as_ref().expect("a drawing").model.layers().get(&copy).and_then(|n| n.origin.clone());
    let origin = origin.expect("the copy's origin");
    assert_eq!(origin.driver, "gpkg");
    assert_eq!(origin.address, path.display().to_string());
    assert_eq!(origin.table.as_deref(), Some("parseller"));
    let bytes = std::fs::read(&path).expect("the file");
    assert_eq!(origin.sha256.as_deref(), Some(kentos_commands::encoding::sha256_hex(&bytes).as_str()));
    assert!(kentos_commands::layer_origin_problem(&origin).is_none(), "{origin:?}");
    // One source, the three ways under it.
    let catalog = app.source_catalog();
    let source = the_file(&catalog, &path);
    assert!(source.file);
    let mut ways: Vec<&str> = source.entries.iter().map(|e| e.way.words()).collect();
    ways.sort_unstable();
    assert_eq!(ways, ["Alınmış kopya", "Bağlı katman", "Veri katmanı"]);
    assert!(source.entries.iter().all(|e| e.table.as_deref() == Some("parseller")));
    // The window reads the copy's file again: the same as when it was taken.
    let task = app.update(Message::Run("source.status"));
    assert!(copy_state(&app, &path).ends_with("dosya okunuyor…"), "{}", copy_state(&app, &path));
    drive(&mut app, task);
    assert!(copy_state(&app, &path).ends_with("(UTC) alındı; dosya değişmedi"), "{}", copy_state(&app, &path));
    // The file changed since, then gone.
    let mut changed = bytes.clone();
    changed.extend_from_slice(b"\0");
    std::fs::write(&path, &changed).expect("changed");
    let task = app.update(Message::Gis(crate::gis::Event::SourceStatus(super::super::Event::Refresh)));
    drive(&mut app, task);
    assert!(copy_state(&app, &path).ends_with("alındı; dosya sonra değişti"), "{}", copy_state(&app, &path));
    std::fs::remove_file(&path).expect("gone");
    assert!(copy_state(&app, &path).ends_with("dosya yerinde yok"), "{}", copy_state(&app, &path));
    // Pressing a layer of the catalog chooses it in Katmanlar.
    let _ = app.update(Message::Gis(crate::gis::Event::SourceStatus(super::super::Event::PickLayer(copy.clone()))));
    assert_eq!(app.selected_layer.as_deref(), Some(copy.as_str()));
}

#[test]
fn the_imports_undo_takes_the_copy_and_a_written_project_keeps_its_origin() {
    let (mut app, path, copy) = three_ways("katalog-geri");
    // Written and read again: the origin with it (.kcad GIS data 7).
    let snapshot = app.document.as_ref().expect("a drawing").model.to_snapshot_v2();
    let bytes = kentos_kcad::encode(&snapshot).expect("written");
    let read = kentos_kcad::decode(&bytes).expect("read");
    let find = |nodes: &[kentos_commands::LayerNode]| -> Option<kentos_commands::LayerNode> {
        fn walk(nodes: &[kentos_commands::LayerNode], id: &str) -> Option<kentos_commands::LayerNode> {
            nodes.iter().find_map(|n| if n.id == id { Some(n.clone()) } else { walk(&n.children, id) })
        }
        walk(nodes, &copy)
    };
    let before = find(&snapshot.layers).and_then(|n| n.origin);
    assert!(before.is_some());
    assert_eq!(find(&read.layers).and_then(|n| n.origin), before, "the origin read back as written");
    // Undo back past the import (the data layer and the link came after it): the copy and its origin go.
    let doc = app.document.as_mut().expect("a drawing");
    while doc.model.layers().get(&copy).is_some() {
        assert!(doc.model.undo().is_some(), "undone");
    }
    let catalog = app.source_catalog();
    let ways: Vec<Way> = catalog.iter().flat_map(|s| s.entries.iter().map(|e| e.way)).collect();
    assert!(!ways.contains(&Way::Copy), "{ways:?}");
    // Redo: the copy comes back with where it came from.
    let doc = app.document.as_mut().expect("a drawing");
    assert!(doc.model.redo().is_some());
    assert_eq!(doc.model.layers().get(&copy).and_then(|n| n.origin.clone()), before);
    let _ = std::fs::remove_file(&path);
}

/// The picture (`.run/gui-evidence/kullanim-9192/`): Kaynaklar over a file's data layer, bound layer and copy.
#[test]
#[ignore = "pictures for the owner"]
fn catalog_screens() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    crate::files_testing::usage_shots("9192", "01-kaynaklar", || {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let (mut app, _, _) = three_ways(&format!("katalog-resim-{n}"));
        let task = app.update(Message::Run("source.status"));
        drive(&mut app, task);
        app
    });
}
