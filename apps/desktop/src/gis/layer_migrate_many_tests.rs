//! Kaydet of a CBS project whose file would hold layers' objects (the CBS review's 15th item, docs/adr/9321): one window
//! lists every such layer, the place is chosen once, Taşı ve kaydet moves them all and writes the project. The layers
//! keep their ids, so a network naming one still names it after the project is opened again from its file. A table
//! that is there and a name two layers share are said with free names before anything is written. The first import of
//! a project whose data has no place yet asks for the place once.

use kentos_data::Driver;
use serde_json::json;

use crate::app::{App, Dialog, Message};
use crate::files_testing::{drive, scratch};
use crate::layer_migrate::{After, Event};
use crate::tools_scenes::{Objects, layer, open_edited};

fn send(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(crate::gis::Event::LayerMigrate(e)));
    drive(app, task);
}

fn lines(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

/// A CBS project kept at `path`: Noktalar (3 points), Yollar (2 lines, a network's edges), Alanlar (1 area), each a
/// drawing layer whose objects its file would hold; Açıklama the active annotation layer.
fn held_project(app: &mut App, path: Option<&std::path::Path>, yol_name: &str) {
    crate::data_layers::register();
    let mut o = Objects::new();
    for i in 0..3 {
        o.point("noktalar", [10.0 * f64::from(i), 5.0], None);
    }
    o.push("yollar", json!({ "kind": "line", "a": crate::tools_scenes::xy([0.0, 0.0]), "b": crate::tools_scenes::xy([50.0, 0.0]) }));
    o.push("yollar", json!({ "kind": "line", "a": crate::tools_scenes::xy([50.0, 0.0]), "b": crate::tools_scenes::xy([50.0, 40.0]) }));
    o.push("alanlar", json!({ "kind": "polygon", "pts": [crate::tools_scenes::xy([0.0, 10.0]), crate::tools_scenes::xy([20.0, 10.0]),
        crate::tools_scenes::xy([20.0, 30.0])] }));
    open_edited(app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        d["settings"]["networks"] = json!([{ "id": "yol-agi", "name": "Yol ağı", "kind": "road", "edges": [{ "layer": "yollar" }],
            "connect": "ends", "tolerance": 0.01, "direction": { "kind": "both" } }]);
        let typed = |id: &str, name: &str, g: &str| {
            let mut l = layer(id, name, "fg", 0.25);
            l["geometry"] = json!(g);
            l
        };
        d["layers"] = json!([typed("aciklama", "Açıklama", "annotation"), typed("noktalar", "Noktalar", "point"),
            typed("yollar", yol_name, "line"), typed("alanlar", "Alanlar", "area")]);
        d["activeLayer"] = json!("aciklama");
    });
    if let Some(path) = path {
        app.document.as_mut().expect("a drawing").path = Some(path.to_path_buf());
    }
}

/// A table's rows, read apart from the app.
fn rows_in(gpkg: &std::path::Path, table: &str) -> usize {
    let address = gpkg.display().to_string();
    kentos_data::registry().forget(&address);
    let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
    kentos_data::driver::collect(&*source, table, &kentos_data::Request::all()).expect("rows").len()
}

#[test]
fn kaydet_lists_every_held_layer_and_tasi_ve_kaydet_moves_them_and_writes_the_project() {
    let dir = scratch("cbs-toplu-kayit");
    let path = dir.join("Kent.kcad");
    let (mut app, _) = App::boot(None);
    held_project(&mut app, Some(&path), "Yollar");
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    assert!(!path.exists(), "nothing written before the layers move");
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::LayerMigrate)));
    let w = app.gis.layer_migrate.as_ref().expect("one window");
    assert_eq!(w.layers_for_test(), ["noktalar", "yollar", "alanlar"], "every held layer, in the tree's order");
    assert_eq!(w.after_for_test(), &After::Save(crate::saving::Target::File(path.clone())));
    // Taşı ve kaydet: the tables beside the project, the layers reading them under their ids, the project written.
    send(&mut app, Event::Run);
    assert!(app.gis.layer_migrate.is_none(), "{:?}", lines(&app));
    assert!(path.exists(), "{:?}", lines(&app));
    let gpkg = path.with_extension("gpkg");
    assert_eq!((rows_in(&gpkg, "noktalar"), rows_in(&gpkg, "yollar"), rows_in(&gpkg, "alanlar")), (3, 2, 1));
    // Opened again from its file, as a new session reads it: the network still names its edges' layer, which reads
    // its table in the project's system.
    let opened = crate::document::Document::read(&path).expect("the project opens");
    let model = &opened.model;
    assert_eq!(model.settings().networks[0].edges[0].layer, "yollar");
    let yollar = model.layers().get("yollar").cloned().expect("the network's layer, under its id");
    let data = yollar.data.clone().expect("it reads its table");
    assert_eq!((data.table.as_str(), data.geometry.as_ref().and_then(|g| g.srid)), ("yollar", Some(5256)));
    let read = kentos_native_application::layers_rows::read(
        &yollar,
        model.settings(),
        &kentos_commands::LayersRows { layer: yollar.id.clone(), count: Some(true), limit: Some(0), ..Default::default() },
        &|| false,
    )
    .expect("read from the table");
    assert_eq!(read.total, Some(2));
    assert_eq!(model.len(), 0, "no object in the project file");
}

#[test]
fn a_table_there_and_a_shared_name_get_free_names_before_anything_is_written() {
    let dir = scratch("cbs-toplu-cakisma");
    let path = dir.join("Kent.kcad");
    let gpkg = path.with_extension("gpkg");
    // The GeoPackage beside the project already has a table “noktalar”.
    {
        kentos_geopackage::GeoPackage::create(&gpkg).expect("a file");
        let source = kentos_data_gpkg::Gpkg.open(&gpkg.display().to_string()).expect("open");
        crate::data_layers_tests::point_table(&*source, "noktalar", &[("ad", kentos_data::FieldKind::Text { max: None })], &[]);
    }
    let (mut app, _) = App::boot(None);
    // Yollar named so that its table would be Alanlar's.
    held_project(&mut app, Some(&path), "Alanlar");
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    send(&mut app, Event::Run);
    let w = app.gis.layer_migrate.as_ref().expect("the window stays");
    assert_eq!(w.said_for_test(0).as_deref(), Some("“noktalar” tablosu kaynakta var; “noktalar_2” önerildi."));
    assert_eq!(w.said_for_test(1), None);
    assert_eq!(w.said_for_test(2).as_deref(), Some("“alanlar” adını listedeki başka bir katman da alıyor; “alanlar_2” önerildi."));
    assert_eq!((w.table_for_test(0), w.table_for_test(2)), ("noktalar_2".to_owned(), "alanlar_2".to_owned()));
    assert_eq!(app.data_held_in_file().len(), 3, "nothing moved");
    assert_eq!(rows_in(&gpkg, "noktalar"), 0, "the table there untouched");
    // Taşı again: the names offered taken, every layer moved once, the project written.
    send(&mut app, Event::Run);
    assert!(app.gis.layer_migrate.is_none(), "{:?}", lines(&app));
    assert_eq!((rows_in(&gpkg, "noktalar_2"), rows_in(&gpkg, "alanlar"), rows_in(&gpkg, "alanlar_2")), (3, 2, 1));
    let address = gpkg.display().to_string();
    kentos_data::registry().forget(&address);
    let tables: Vec<String> = kentos_data_gpkg::Gpkg.open(&address).expect("open").tables().expect("tables").into_iter().map(|t| t.name).collect();
    assert_eq!(tables.len(), 4, "no table made twice: {tables:?}");
    assert!(path.exists(), "{:?}", lines(&app));
}

#[test]
fn the_first_layers_brought_into_a_project_without_a_place_ask_for_it_once() {
    let (mut app, _) = App::boot(None);
    held_project(&mut app, None, "Yollar");
    assert!(app.data_place_unasked());
    let held = app.data_held_in_file();
    app.data_into_tables(held.clone());
    let w = app.gis.layer_migrate.as_ref().expect("the place asked");
    assert_eq!(w.after_for_test(), &After::Place);
    assert_eq!(w.layers_for_test().len(), 3);
    // Vazgeç: not asked again this session; the save asks for them.
    send(&mut app, Event::Close);
    assert!(!app.data_place_unasked());
    app.data_into_tables(held);
    assert!(app.gis.layer_migrate.is_none(), "asked once");
    // Chosen: the layers move there, and the place is the project's from then on.
    let gpkg = scratch("cbs-veri-yeri").join("Kent.gpkg");
    app.gis.data_place_asked = false;
    let held = app.data_held_in_file();
    app.data_into_tables(held);
    send(&mut app, Event::Path(gpkg.display().to_string()));
    send(&mut app, Event::Run);
    assert!(app.data_held_in_file().is_empty(), "{:?}", lines(&app));
    assert_eq!(app.cbs_data_place().map(|(_, place, _)| place), Some(gpkg.display().to_string()));
    assert!(!app.data_place_unasked(), "the data has a place now");
}

/// The pictures (`.run/gui-evidence/kullanim-9321/`): Kaydet's window on every held layer, and the same with a table
/// that is there and a shared name, their free names offered.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib layer_migrate_many_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9321", "01-kaydet-tek-pencere", || {
        let path = scratch("resim-toplu").join("Kent.kcad");
        let (mut app, _) = App::boot(None);
        held_project(&mut app, Some(&path), "Yollar");
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        app
    });
    crate::files_testing::usage_shots("9321", "02-cakisma-onerileri", || {
        let path = scratch("resim-cakisma").join("Kent.kcad");
        let gpkg = path.with_extension("gpkg");
        kentos_geopackage::GeoPackage::create(&gpkg).expect("a file");
        let source = kentos_data_gpkg::Gpkg.open(&gpkg.display().to_string()).expect("open");
        crate::data_layers_tests::point_table(&*source, "noktalar", &[("ad", kentos_data::FieldKind::Text { max: None })], &[]);
        let (mut app, _) = App::boot(None);
        held_project(&mut app, Some(&path), "Alanlar");
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        send(&mut app, Event::Run);
        app
    });
}
