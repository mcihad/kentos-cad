//! Seçilenleri kaydet in a CBS project (docs/adr/9339; the CBS review of 27 November 2026, item 1). A road AB, `ad=AB`,
//! on a drawing layer of a CBS project in EPSG:5256 beside three data layers.
//!
//! - The copy's kind is asked first; with its GeoPackage the copy reads the road from `<ad>.gpkg`, its project file
//!   holds no object. Opened, a GeoPackage table added as a data layer, Kaydet writes at once (no move asked); opened
//!   again, the row's key, its value and the system are the same.
//! - A table of the same name: Yeni tablo makes `<ad>_2`, Üzerine yaz makes it again (one row), Ekle adds to it (two
//!   rows), each read back from the file.
//! - Çizim kopyası keeps the objects in the project file, as before.
//!
//! Pictures: `kullanim-9339` (`screens`, ignored).

use std::path::Path;

use kentos_commands::ExportExisting;

use super::selection_save::Event as S;
use super::source_export_tests::{qa_gpkg, read_rows, scene};
use super::tests::{run, said, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{drive, scratch};

/// The review's project: the three data layers and a drawing layer `yollar` holding the road AB.
fn project(dir: &Path) -> App {
    let mut app = scene(&qa_gpkg(dir, "qa.gpkg"));
    let model = &mut app.document.as_mut().expect("a drawing").model;
    let mut layer = kentos_domain::NewLayer::layer("yollar");
    layer.fields = vec![serde_json::from_value(serde_json::json!({ "name": "ad", "kind": "text" })).expect("a field")];
    let id = model.add_layer(layer, None, false).expect("a layer");
    let road: kentos_commands::Entity = serde_json::from_value(serde_json::json!({
        "kind": "polyline", "id": 0, "layerId": id, "attrs": { "ad": "AB" },
        "pts": [{ "x": 487000.0, "y": 4420000.0 }, { "x": 487100.0, "y": 4420050.0 }]
    }))
    .expect("a road");
    let slot = model.add(road).expect("added");
    app.selection.set(vec![slot]);
    app
}

/// Seçilenleri kaydet to `path`, the copy with its tables and `same` for a table there.
fn save_selection(app: &mut App, path: &Path, same: ExportExisting) {
    app.picker = Picker::File(path.to_path_buf());
    run(app, "file.saveSelection");
    assert!(matches!(app.exchange, Some(Window::SelectionSave(_))), "the copy's kind is asked first");
    send(app, Event::SelectionSave(S::Same(same)));
    send(app, Event::SelectionSave(S::Save));
}

/// The rows of `table` in the GeoPackage: each key and `ad`.
fn rows(gpkg: &Path, table: &str) -> Vec<(i64, String)> {
    let gp = kentos_geopackage::GeoPackage::open(gpkg, false).expect("opens");
    let conn = gp.connection();
    let mut q = conn.prepare(&format!("SELECT fid, ad FROM {} ORDER BY fid", kentos_geopackage::quote(table))).expect("a query");
    q.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .expect("read")
        .map(|r| r.expect("a row"))
        .collect()
}

fn open(path: &Path) -> App {
    let (mut app, _) = App::boot(None);
    let doc = crate::document::Document::read(path).expect("reads");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn log(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

#[test]
fn a_cbs_selection_is_a_project_with_its_geopackage_and_saves_at_once() {
    let dir = scratch("secim-cbs");
    let mut app = project(&dir);
    let copy = dir.join("kopya").join("secim.kcad");
    std::fs::create_dir_all(copy.parent().expect("a folder")).expect("made");
    save_selection(&mut app, &copy, ExportExisting::Refuse);
    assert!(said(&app, "Seçilenler CBS projesi olarak kaydedildi: 1 nesne"), "{:?}", log(&app));
    let gpkg = copy.with_extension("gpkg");
    assert_eq!(rows(&gpkg, "yollar"), [(1, "AB".to_owned())]);
    // The copy: no object in its project file, its layer reading the table, its path from the project.
    let mut copied = open(&copy);
    {
        let model = &copied.document.as_ref().expect("open").model;
        assert!(kentos_native_application::layer_kind::held_in_file(model).is_empty());
        let node = model.layers().leaves().into_iter().find(|l| l.name == "yollar").expect("the layer").clone();
        let data = node.data.as_ref().expect("a data layer");
        assert_eq!((data.table.as_str(), data.relative.as_deref()), ("yollar", Some("secim.gpkg")));
        assert_eq!(data.geometry.as_ref().and_then(|g| g.srid), Some(5256));
    }
    // A GeoPackage table added (Dosyada bırak, veri katmanı olarak ekle), then Kaydet: written at once.
    let other = qa_gpkg(&dir, "ek.gpkg");
    copied
        .add_data_layers(vec![crate::data_layers::describe(&other.display().to_string(), "qa_noktalar").expect("described")])
        .expect("added");
    let task = copied.start_save(crate::saving::Target::File(copy.clone()));
    drive(&mut copied, task);
    assert!(copied.gis.layer_migrate.is_none(), "no move asked: {:?}", log(&copied));
    assert!(!copied.document.as_ref().expect("open").model.is_dirty(), "saved: {:?}", log(&copied));
    // Opened again: the same row, value and system.
    let session = kentos_headless::Session::open(&copy).expect("opens");
    let doc = session.document();
    assert_eq!(doc.settings().srid, 5256);
    let node = doc.layers().leaves().into_iter().find(|l| l.name == "yollar").expect("the layer");
    let read = kentos_native_application::data_write::all_rows(node, doc.settings(), &Default::default(), &|| false).expect("read");
    let (key, road) = &read.rows[0];
    assert_eq!((read.rows.len(), *key, road.base().attrs.get("ad").map(String::as_str)), (1, 1, Some("AB")));
    assert!(doc.layers().leaves().iter().any(|l| l.name == "qa_noktalar"), "the new layer kept");
}

#[test]
fn a_table_of_the_same_name_is_given_its_own_made_again_or_added_to() {
    let dir = scratch("secim-cbs-ayni");
    let mut app = project(&dir);
    let copy = dir.join("secim.kcad");
    let gpkg = copy.with_extension("gpkg");
    save_selection(&mut app, &copy, ExportExisting::Refuse);
    assert_eq!(rows(&gpkg, "yollar").len(), 1);
    // Ekle: the row after the table's own.
    save_selection(&mut app, &copy, ExportExisting::Append);
    assert!(said(&app, "Seçilenler CBS projesi olarak kaydedildi"), "{:?}", log(&app));
    assert_eq!(rows(&gpkg, "yollar"), [(1, "AB".to_owned()), (2, "AB".to_owned())]);
    // Üzerine yaz: the table made again with the selection alone.
    save_selection(&mut app, &copy, ExportExisting::ReplaceLayer);
    assert_eq!(rows(&gpkg, "yollar"), [(1, "AB".to_owned())]);
    // Yeni tablo: a name of its own, the table there untouched; the copy reads the new one.
    save_selection(&mut app, &copy, ExportExisting::Refuse);
    assert_eq!(rows(&gpkg, "yollar"), [(1, "AB".to_owned())]);
    assert_eq!(rows(&gpkg, "yollar_2"), [(1, "AB".to_owned())]);
    let session = kentos_headless::Session::open(&copy).expect("opens");
    let node = session.document().layers().leaves().into_iter().find(|l| l.name == "yollar").expect("the layer").clone();
    assert_eq!(node.data.as_ref().map(|d| d.table.as_str()), Some("yollar_2"));
}

#[test]
fn a_drawing_copy_keeps_the_objects_in_its_file() {
    let dir = scratch("secim-cizim");
    let mut app = project(&dir);
    let copy = dir.join("cizim.kcad");
    app.picker = Picker::File(copy.clone());
    run(&mut app, "file.saveSelection");
    send(&mut app, Event::SelectionSave(S::Tables(false)));
    send(&mut app, Event::SelectionSave(S::Save));
    assert!(said(&app, "Seçilenler kaydedildi: 1 nesne"), "{:?}", log(&app));
    assert!(!copy.with_extension("gpkg").exists(), "no GeoPackage for a drawing copy");
    let copied = open(&copy);
    let held = kentos_native_application::layer_kind::held_in_file(&copied.document.as_ref().expect("open").model);
    assert_eq!(held.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(), ["yollar"]);
}

#[test]
fn a_layer_that_no_table_holds_writes_nothing() {
    let dir = scratch("secim-cbs-olcu");
    let mut app = project(&dir);
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let layer = layer_id_of(model, "yollar");
        let dimension: kentos_commands::Entity = serde_json::from_value(serde_json::json!({
            "kind": "dimension", "id": 0, "layerId": layer, "attrs": {},
            "a": { "x": 487000.0, "y": 4420000.0 }, "b": { "x": 487100.0, "y": 4420000.0 }, "offset": 5.0, "height": 2.5
        }))
        .expect("a dimension");
        let slot = model.add(dimension).expect("added");
        let mut all = app.selection.ids().to_vec();
        all.push(slot);
        app.selection.set(all);
    }
    let copy = dir.join("olculu.kcad");
    save_selection(&mut app, &copy, ExportExisting::Refuse);
    assert!(log(&app).iter().any(|l| l.contains("Hiçbir dosya yazılmadı")), "{:?}", log(&app));
    assert!(!copy.exists() && !copy.with_extension("gpkg").exists(), "nothing left of the copy");
}

fn layer_id_of(model: &kentos_domain::Document, name: &str) -> String {
    model.layers().leaves().into_iter().find(|l| l.name == name).expect("the layer").id.clone()
}

#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    let dir = scratch("secim-resim");
    crate::files_testing::usage_shots("9339", "01-secilenleri-kaydet", || {
        let mut app = project(&dir);
        read_rows(&mut app);
        app.picker = Picker::File(dir.join("resim.kcad"));
        run(&mut app, "file.saveSelection");
        app
    });
}
