//! A CBS project's data in its tables (docs/adr/9196 §2, plan/ayrim.md AY-05), as the user and a script meet it: Kaydet
//! takes no drawing layer's objects into the project file and opens Veri kaynağına taşı; a table bound to a copy of
//! its rows is refused by the command itself, for every client; the cloud upload takes the data layers' rows up with
//! the project. The import (exchange/gis_tests.rs) and the GeoPackage window (exchange/gpkg_tests.rs) have their own.

use std::path::Path;

use kentos_commands::{CallOp, Workspace};
use serde_json::json;

use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, drive, scratch};

/// The sample drawing made a CBS project kept at `path`.
fn cbs(path: Option<&Path>) -> App {
    crate::data_layers::register();
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.settings().clone();
    settings.workspace = Some(Workspace::Gis);
    doc.model.set_settings(settings);
    doc.path = path.map(Path::to_path_buf);
    app
}

fn lines(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

/// Kaydet writes no drawing layer's objects into a CBS project's file: it says which layers and how, and opens Veri
/// kaynağına taşı on every one of them; Taşı ve kaydet makes them tables and writes the project (docs/adr/9321).
#[test]
fn a_cbs_projects_file_takes_its_tables_not_its_drawing_layers_objects() {
    let dir = scratch("cbs-kayit");
    let path = dir.join("Kent.kcad");
    let mut app = cbs(Some(&path));
    // The parcels alone: the locked buildings and the mixed drawing layer (with a construction line) are no table.
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        model.set_layer_locked("bina", false);
        model.remove_layer("bina").expect("removed");
        model.remove_layer("cizim").expect("removed");
    }
    let held = app.data_held_in_file();
    assert_eq!(held, [("parsel".to_owned(), "Parsel".to_owned())]);
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    assert!(!path.exists(), "nothing is written");
    let said = lines(&app);
    assert!(
        said.iter().any(|l| l.starts_with("Kaydedilmedi: bu bir CBS projesi") && l.contains(&format!("“{}”", held[0].1))),
        "{said:?}"
    );
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::LayerMigrate)));
    let w = app.gis.layer_migrate.as_ref().expect("the window on every held layer");
    assert_eq!(w.layers_for_test(), [held[0].0.clone()]);
    assert_eq!(w.after_for_test(), &crate::layer_migrate::After::Save(crate::saving::Target::File(path.clone())));
    // Taşı ve kaydet: moved into the GeoPackage beside the project, the layers leave nothing for the file but its
    // structure, and the save goes on.
    let task = app.update(Message::Gis(crate::gis::Event::LayerMigrate(crate::layer_migrate::Event::Run)));
    drive(&mut app, task);
    assert!(app.data_held_in_file().is_empty(), "{:?}", lines(&app));
    assert!(path.exists(), "{:?}", lines(&app));
    // A CAD project's file keeps everything.
    let mut cad = app_with_drawing();
    crate::files_testing::make_cad(&mut cad);
    assert!(cad.data_held_in_file().is_empty());
}

/// A table bound to a copy of its rows is not a CBS project's (docs/adr/9196 §2): the command refuses it, whichever
/// client asks, and says to add it as a data layer.
#[test]
fn a_cbs_project_binds_no_table() {
    let mut app = cbs(None);
    let gpkg = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geopackage/v1/qgis.gpkg");
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": gpkg.display().to_string(), "table": "parsel" } }),
    )
    .expect("an answer");
    assert_eq!(answer["status"], "failed", "{answer}");
    assert_eq!(answer["error"]["code"], "bound_in_gis", "{answer}");
    assert!(answer["error"]["message"].as_str().is_some_and(|m| m.contains("cad.layers.data")), "{answer}");
    assert!(doc.model.settings().sources.is_empty());
}

/// A CBS project goes up to KentOS Bulut with its data (docs/adr/9196 §2): each data layer's rows are read from its
/// table into the upload's copy as objects of the layer, with their values; the project on this computer stays as it
/// was.
#[test]
fn a_cbs_projects_data_layers_go_up_with_their_rows() {
    let dir = scratch("cbs-bulut");
    let mut app = cbs(Some(&dir.join("Kent.kcad")));
    let objects = app.document.as_ref().expect("open").model.len();
    let held = app.data_held_in_file();
    app.data_into_tables(held);
    let model = &app.document.as_ref().expect("open").model;
    let tables = model.layers().leaves().iter().filter(|n| n.data.is_some()).count();
    assert!(tables > 0);
    let left = model.len();
    let mut snapshot = model.to_snapshot_v2();
    let taken = kentos_native_application::data_take::take_rows(&mut snapshot, &kentos_data::Cancel::new())
        .expect("the rows are read");
    assert_eq!(taken.layers, tables);
    assert!(taken.left.is_empty(), "{:?}", taken.left);
    assert_eq!(snapshot.entities.len(), left + taken.rows as usize);
    assert_eq!(taken.rows as usize + left, objects, "every object of the drawing went up");
    let mut tree = snapshot.layers.clone();
    kentos_native_application::data_take::plain(&mut tree);
    assert_eq!(tree, snapshot.layers, "the catalog's tree is the file's");
    let up = crate::document::Document::from_v2(snapshot, None).expect("the upload reads back");
    assert!(up.model.layers().leaves().iter().all(|n| n.data.is_none()));
    assert_eq!(up.model.len(), objects);
    // The project here still reads its tables.
    let here = &app.document.as_ref().expect("open").model;
    assert_eq!(here.layers().leaves().iter().filter(|n| n.data.is_some()).count(), tables);
    // Bulutta a CBS project is a database project alone.
    assert_eq!(
        crate::cloud::kinds::storages_of(kentos_commands::ProjectType::Gis),
        [kentos_commands::ProjectStorage::Database]
    );
}

/// The owner's pictures (docs/adr/9196 §5): an import gone into the project's tables, Kaydet refused with Veri
/// kaynağına taşı open, the GeoPackage window of a CBS project and its Buluta yükle; light 1440 × 900 and dark
/// 1100 × 760 into `.run/gui-evidence/kullanim-9196` (never committed).
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib cbs_storage_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use crate::files_testing::usage_shots;

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    usage_shots("9196", "2-ice-alma-tabloya", || {
        let dir = scratch("cbs-ice-alma-resim");
        let mut app = cbs(Some(&dir.join("Kent.kcad")));
        app.picker = crate::app::Picker::File(fixtures.join("formats/v1/gis/tm.geojson"));
        let task = app.run("file.import.geojson");
        drive(&mut app, task);
        let task = app.update(Message::Exchange(Box::new(crate::exchange::Event::GisImport(
            crate::exchange::gis_import::Event::Run,
        ))));
        drive(&mut app, task);
        app.command_expanded = true;
        app
    });
    usage_shots("9196", "3-kayit-reddi", || {
        let dir = scratch("cbs-kayit-resim");
        let mut app = cbs(Some(&dir.join("Kent.kcad")));
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        app.command_expanded = true;
        app
    });
    usage_shots("9196", "4-geopackage-veri-katmani", || {
        let mut app = cbs(None);
        app.picker = crate::app::Picker::File(fixtures.join("geopackage/v1/qgis.gpkg"));
        let task = app.run("file.import.gpkg");
        drive(&mut app, task);
        let task = app.update(Message::Exchange(Box::new(crate::exchange::Event::Gpkg(
            crate::exchange::gpkg::Event::Bind,
        ))));
        drive(&mut app, task);
        app
    });
    usage_shots("9196", "5-buluta-yukle-cbs", || {
        let dir = scratch("cbs-bulut-resim");
        let mut app = crate::cloud::tests::signed_in();
        let _ = app.update(Message::Opened(Some(Ok(Box::new(crate::files_testing::drawing(0))))));
        {
            let doc = app.document.as_mut().expect("a drawing");
            let mut settings = doc.settings().clone();
            settings.workspace = Some(Workspace::Gis);
            doc.model.set_settings(settings);
            doc.path = Some(dir.join("Kent.kcad"));
        }
        crate::data_layers::register();
        let held = app.data_held_in_file();
        app.data_into_tables(held);
        app.open_upload(kentos_commands::ProjectStorage::File);
        app
    });
}
