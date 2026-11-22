//! GeoPackage dışa aktar as a delivery (docs/adr/9139), the municipal report's 6th item: the window says each layer
//! with its objects, what goes into the file (tables, styles, the relations whose two layers go) and what does not
//! (forms, join views, the workspace), and that it is a copy; a layer named in Turkish is a table of that name read
//! back with its rows and system; a file that cannot be written is refused and left as it was.

use super::super::Event;
use super::super::tests::{run, send};
use crate::app::Picker;
use crate::files_testing::{app_with_drawing, scratch};

#[test]
fn the_window_says_what_goes_into_the_file_and_what_does_not() {
    let mut app = app_with_drawing();
    run(&mut app, "file.export.gpkg");
    let lines = app.gpkg_out_summary(super::super::gpkg_out::Scope::All);
    assert!(lines[0].contains("nesne,") && lines[0].contains("katman yazılır:"), "{lines:#?}");
    assert!(lines[0].contains('('), "each layer with its objects: {}", lines[0]);
    assert!(lines.iter().any(|l| l.starts_with("Dosyaya giden: her katman bir tablo")), "{lines:#?}");
    // The sample's texts and dimensions are said as staying out, by kind.
    assert!(lines.iter().any(|l| l.starts_with("Geometrisi olmadığı için yazılmayan:")), "{lines:#?}");
    assert!(lines.iter().any(|l| l.starts_with("Dosyaya gitmeyen: formlar, birleşik görünümler")), "{lines:#?}");
    assert!(lines.iter().any(|l| l.starts_with("Bu, verinin bir kopyasıdır")), "{lines:#?}");
}

#[test]
fn a_turkish_layer_name_is_its_table_and_a_file_that_cannot_be_written_is_left_alone() {
    let dir = scratch("gpkg-teslim");
    let mut app = app_with_drawing();
    // The layer with objects, named in Turkish; its objects a table takes (a text has no geometry there).
    let (layer, count) = {
        let doc = app.document.as_ref().expect("open");
        let layer = doc.model.entities().next().map(|e| e.base().layer_id.clone()).expect("an object");
        let count = doc
            .model
            .by_layer(&layer)
            .filter(|e| kentos_native_application::geopackage::geometry_of(e).is_ok())
            .count();
        (layer, count)
    };
    if let Some(doc) = app.document.as_mut() {
        doc.model.rename_layer(&layer, "Yol ekseni çğüşıö");
    }
    let out = dir.join("teslim.gpkg");
    app.picker = Picker::File(out.clone());
    run(&mut app, "file.export.gpkg");
    send(&mut app, Event::GpkgOut(super::super::gpkg_out::Event::Scope(super::super::gpkg_out::Scope::All)));
    send(&mut app, Event::GpkgOut(super::super::gpkg_out::Event::Save));
    assert!(out.exists(), "{:?}", app.log.lines().last().map(|l| l.text.clone()));
    let gp = kentos_geopackage::GeoPackage::open(&out, false).expect("opens");
    let tables = gp.layers().expect("its tables");
    let table = tables.iter().find(|t| t.name.contains("Yol ekseni")).unwrap_or_else(|| {
        panic!("{:?}", tables.iter().map(|t| t.name.clone()).collect::<Vec<_>>())
    });
    assert_eq!(table.name, "Yol ekseni çğüşıö", "the name as written");
    let rows = gp.rows(table, &kentos_geopackage::Query::default()).expect("rows");
    assert_eq!(rows.len(), count);
    let srid = app.document.as_ref().expect("open").model.settings().srid;
    assert_eq!(table.srs_id.map(|s| s as u32), Some(srid));
    // The non-linear types are registered by their own names, as readers know them; never gpkg_geom_GEOMETRY.
    let mut st = gp.connection().prepare("SELECT extension_name FROM gpkg_extensions").expect("a query");
    let names: Vec<String> = st.query_map([], |r| r.get(0)).expect("rows").map(|r| r.expect("a row")).collect();
    drop(st);
    assert!(!names.iter().any(|n| n == "gpkg_geom_GEOMETRY"), "{names:?}");
    assert!(names.iter().any(|n| n.starts_with("gpkg_geom_")), "the sample's arcs are registered: {names:?}");
    drop(gp);
    // KENTOS_KEEP_DELIVERY=<path> keeps the delivery for an outside reader (GDAL's ogrinfo).
    if let Some(keep) = std::env::var_os("KENTOS_KEEP_DELIVERY") {
        std::fs::copy(&out, keep).expect("kept");
    }
    // A file that cannot be written: refused, the file as it was.
    let before = std::fs::read(&out).expect("the file");
    let mut perms = std::fs::metadata(&out).expect("the file").permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&out, perms.clone()).expect("read only");
    run(&mut app, "file.export.gpkg");
    send(&mut app, Event::GpkgOut(super::super::gpkg_out::Event::Save));
    assert_eq!(std::fs::read(&out).expect("the file"), before, "left as it was");
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    let _ = std::fs::set_permissions(&out, perms);
    let _ = std::fs::remove_dir_all(&dir);
}
