//! The file exports and Seçilenleri kaydet with data layers' rows (docs/adr/9344; the CBS review of 27 November 2026,
//! second round, T1 and T2). The review's project: `qa_noktalar` (3), `qa_yollar` (3) and a second layer named
//! `qa_yollar` reading `qa_secim` (1) in the group Senaryo 1.
//!
//! - GeoPackage dışa aktar › Tüm proje writes the three tables with 3, 3 and 1 rows, the second `qa_yollar` as
//!   `qa_yollar_2`, named before the file is asked; the success says what is on the disk.
//! - FlatGeobuf, GeoParquet, GML, CSV and Shapefile plan the rows; KML counts them.
//! - A row chosen in its table is a selection for Seçilenleri kaydet: a new CBS project and GeoPackage with that row.

use super::source_export_tests::{qa_gpkg, read_rows, scene};
use super::tests::{run, said, send};
use super::{Event, Window};
use crate::app::{App, Picker};
use crate::files_testing::scratch;

/// The review's project: the Senaryo 1 group holding a second `qa_yollar` (the `qa_secim` table).
fn project(dir: &std::path::Path) -> App {
    let mut app = scene(&qa_gpkg(dir, "qa.gpkg"));
    let model = &mut app.document.as_mut().expect("a drawing").model;
    let secim = model
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == "qa_secim")
        .map(|l| l.id.clone())
        .expect("qa_secim");
    model.rename_layer(&secim, "qa_yollar");
    let group = model
        .add_layer(kentos_domain::NewLayer::group("Senaryo 1"), None, false)
        .expect("a group");
    model
        .move_layer(&secim, Some(&group), None)
        .expect("into the group");
    // The rows asked for, as an export window asks when it opens, and read.
    let _ = app.source_copy();
    read_rows(&mut app);
    app
}

fn rows(path: &std::path::Path, table: &str) -> i64 {
    let gp = kentos_geopackage::GeoPackage::open(path, false).expect("opens");
    gp.connection()
        .query_row(
            &format!("SELECT count(*) FROM {}", kentos_geopackage::quote(table)),
            [],
            |r| r.get(0),
        )
        .expect("counted")
}

fn log(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

#[test]
fn geopackage_writes_the_rows_and_names_same_named_layers_apart() {
    let dir = scratch("kaynak-gpkg-disari");
    let mut app = project(&dir);
    run(&mut app, "file.export.gpkg");
    send(
        &mut app,
        Event::GpkgOut(super::gpkg_out::Event::Scope(super::gpkg_out::Scope::All)),
    );
    let summary = app.gpkg_out_summary(super::gpkg_out::Scope::All);
    assert!(
        summary
            .iter()
            .any(|l| l.starts_with("7 nesne, 3 katman yazılır")),
        "{summary:#?}"
    );
    assert!(
        summary
            .iter()
            .any(|l| l.contains("Senaryo 1 / qa_yollar → qa_yollar_2")),
        "{summary:#?}"
    );
    let path = dir.join("disari.gpkg");
    app.picker = Picker::File(path.clone());
    send(&mut app, Event::GpkgOut(super::gpkg_out::Event::Save));
    assert_eq!(
        (
            rows(&path, "qa_noktalar"),
            rows(&path, "qa_yollar"),
            rows(&path, "qa_yollar_2")
        ),
        (3, 3, 1),
        "{:?}",
        log(&app)
    );
}

#[test]
fn the_other_formats_plan_the_rows() {
    let dir = scratch("kaynak-bicimler");
    let app = project(&dir);
    assert!(app.fgb_plan().is_ok(), "{:?}", app.fgb_plan().err());
    assert!(app.gml_plan(true).is_ok(), "{:?}", app.gml_plan(true).err());
    assert!(
        app.csv_plan(
            kentos_commands::CsvGeometry::Wkt,
            kentos_commands::CsvSeparator::Comma
        )
        .is_ok()
    );
    let shp = app
        .shp_plan(kentos_commands::ShpEncoding::Utf8)
        .expect("Shapefiles");
    assert_eq!(shp.0.len(), 3, "a Shapefile a layer");
}

#[test]
fn a_row_chosen_in_its_table_is_saved_with_its_geopackage() {
    let dir = scratch("kaynak-secim-kaydet");
    let mut app = project(&dir);
    let yollar = app
        .document
        .as_ref()
        .and_then(|d| {
            d.model
                .layers()
                .leaves()
                .into_iter()
                .find(|l| l.name == "qa_yollar")
                .map(|l| l.id.clone())
        })
        .expect("qa_yollar");
    assert!(!app.available("file.saveSelection"));
    app.gis.features.data.choose_rows(&yollar, [1].into());
    assert!(
        app.available("file.saveSelection"),
        "a chosen row is a selection"
    );
    let copy = dir.join("secim.kcad");
    app.picker = Picker::File(copy.clone());
    run(&mut app, "file.saveSelection");
    assert!(
        matches!(app.exchange, Some(Window::SelectionSave(_))),
        "the copy's kind asked"
    );
    send(
        &mut app,
        Event::SelectionSave(super::selection_save::Event::Save),
    );
    assert!(
        said(&app, "Seçilenler CBS projesi olarak kaydedildi: 1 nesne"),
        "{:?}",
        log(&app)
    );
    let gpkg = copy.with_extension("gpkg");
    assert_eq!(rows(&gpkg, "qa_yollar"), 1);
    let gp = kentos_geopackage::GeoPackage::open(&gpkg, false).expect("opens");
    let (ad, srs): (String, i64) = gp
        .connection()
        .query_row("SELECT ad, (SELECT srs_id FROM gpkg_geometry_columns WHERE table_name = 'qa_yollar') FROM qa_yollar", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .expect("the row");
    assert_eq!((ad.as_str(), srs), ("AB", 5256));
}

/// The command itself, as Python, MCP and an agent call it (docs/adr/9344): `file.export.gpkg` on the drawing as it
/// is reads the data layers' rows from their sources.
#[test]
fn the_command_reads_the_rows_for_every_channel() {
    let dir = scratch("kaynak-gpkg-komut");
    let app = project(&dir);
    let mut model = app.document.as_ref().expect("a drawing").model.clone();
    let path = dir.join("komut.gpkg");
    let answer = kentos_headless::file_commands::run(
        &mut model,
        "file.export.gpkg",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "path": path.display().to_string() }),
    )
    .expect("answered");
    assert_eq!(answer["status"], "completed", "{answer}");
    assert_eq!(
        (
            rows(&path, "qa_noktalar"),
            rows(&path, "qa_yollar"),
            rows(&path, "qa_yollar_2")
        ),
        (3, 3, 1)
    );
}

#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    let dir = scratch("kaynak-gpkg-resim");
    crate::files_testing::usage_shots("9344", "01-geopackage-kaynak", || {
        let mut app = project(&dir);
        run(&mut app, "file.export.gpkg");
        send(
            &mut app,
            Event::GpkgOut(super::gpkg_out::Event::Scope(super::gpkg_out::Scope::All)),
        );
        app
    });
}
