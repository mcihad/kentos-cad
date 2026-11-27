//! The inspector's drawing rows with data layers (docs/adr/9340; the CBS review of 27 November 2026, item 12): three
//! GeoPackage tables of 1, 3 and 3 rows, saved and opened again. With nothing selected the drawing's own objects are
//! “Çizimdeki nesne 0”; the sources' rows are “sayılıyor…” until their sources count them, then 7, and each layer
//! says its own on a row of its own. A table that cannot be counted is said by name, its rows never taken as 0.

use kentos_data::{Coord, Driver as _, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive, scratch};

/// A GeoPackage with `qa_secim` (1 row), `qa_yollar` (3) and `qa_noktalar` (3), points all.
fn tables(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("qa.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut tx = source.begin().expect("begin");
    for (table, n) in [("qa_secim", 1), ("qa_yollar", 3), ("qa_noktalar", 3)] {
        tx.create_table(
            &NewTable {
                name: table.into(),
                title: None,
                description: None,
                key: "fid".into(),
                fields: vec![Field::new("ad", FieldKind::Text { max: Some(16) })],
                geometry: Some(GeometryField { name: "geom".into(), type_name: "POINT".into(), z: false, m: false, srid: Some(5256) }),
                spatial_index: true,
                system: None,
            },
            false,
        )
        .expect("table");
        let rows: Vec<NewFeature> = (0..n)
            .map(|i| NewFeature {
                fid: None,
                geometry: Some(Geometry::Point(Some(Coord::xy(487_000.0 + f64::from(i), 4_420_000.0)))),
                values: vec![("ad".into(), Value::Text(format!("{table}-{i}")))],
            })
            .collect();
        tx.insert(table, &rows).expect("rows");
    }
    tx.commit().expect("commit");
    path
}

/// The inspector's drawing rows, label and value.
fn drawing_rows(app: &App) -> Vec<(String, String)> {
    let doc = app.document.as_ref().expect("a drawing");
    let panel = app.properties_panel(doc);
    let section = panel.sections.iter().find(|s| s.id == "doc").expect("the drawing's section");
    section.rows.iter().map(|r| (r.label.to_string(), r.value.clone())).collect()
}

fn value(rows: &[(String, String)], label: &str) -> Option<String> {
    rows.iter().find(|(l, _)| l == label).map(|(_, v)| v.clone())
}

fn counted(app: &mut App) {
    let task = app.source_counts_tasks();
    drive(app, task);
}

#[test]
fn the_drawings_objects_and_the_sources_rows_are_said_apart() {
    crate::data_layers::register();
    let dir = scratch("kaynak-sayim");
    let address = tables(&dir).display().to_string();
    let mut app = app_with_drawing();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let all: Vec<kentos_domain::Slot> = model.entities().map(|e| kentos_domain::Slot(e.base().id)).collect();
        model.remove(&all);
    }
    app.add_data_layers(
        ["qa_noktalar", "qa_yollar", "qa_secim"]
            .into_iter()
            .map(|t| crate::data_layers::describe(&address, t).expect("described"))
            .collect(),
    )
    .expect("added");
    // Saved and opened again, as the review did.
    let path = dir.join("qa.kcad");
    let task = app.start_save(crate::saving::Target::File(path.clone()));
    drive(&mut app, task);
    let (mut app, _) = App::boot(None);
    let doc = crate::document::Document::read(&path).expect("reads");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let rows = drawing_rows(&app);
    assert_eq!(value(&rows, "Nesne sayısı"), None, "{rows:?}");
    assert_eq!(value(&rows, "Çizimdeki nesne").as_deref(), Some("0"));
    // Not counted yet: never 0.
    let mut fresh = App::boot(None).0;
    let _ = fresh.update(Message::Opened(Some(Ok(Box::new(crate::document::Document::read(&path).expect("reads"))))));
    let before = drawing_rows(&fresh);
    assert!(value(&before, "Kaynaktaki satır").is_some_and(|v| v == "sayılıyor…" || v == "7"), "{before:?}");
    counted(&mut app);
    let rows = drawing_rows(&app);
    assert_eq!(value(&rows, "Kaynaktaki satır").as_deref(), Some("7"), "{rows:?}");
    assert_eq!(value(&rows, "Veri katmanları").as_deref(), Some("3 katman"));
    for (layer, n) in [("qa_secim", "1 satır"), ("qa_yollar", "3 satır"), ("qa_noktalar", "3 satır")] {
        assert_eq!(value(&rows, &format!("  {layer}")).as_deref(), Some(n), "{rows:?}");
    }
    // A table whose file is gone: said by name, the others counted.
    let id = app
        .document
        .as_ref()
        .and_then(|d| d.model.layers().leaves().into_iter().find(|l| l.name == "qa_secim").map(|l| l.id.clone()))
        .expect("qa_secim");
    let gone = dir.join("yok").join("qa.gpkg").display().to_string();
    app.document.as_mut().expect("a drawing").model.set_layer_data_address(&id, &gone);
    let _ = drawing_rows(&app);
    counted(&mut app);
    let rows = drawing_rows(&app);
    assert_eq!(value(&rows, "Kaynaktaki satır").as_deref(), Some("6 (+ 1 katman sayılamadı)"), "{rows:?}");
    assert!(value(&rows, "  qa_secim").is_some_and(|v| v.starts_with("sayılamadı")), "{rows:?}");
}

/// A CAD project without data layers keeps its one row.
#[test]
fn without_data_layers_the_count_is_the_drawings() {
    let app = app_with_drawing();
    let rows = drawing_rows(&app);
    let n = app.document.as_ref().expect("a drawing").entity_count().to_string();
    assert_eq!(value(&rows, "Nesne sayısı"), Some(n));
    assert_eq!(value(&rows, "Kaynaktaki satır"), None);
}

#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::data_layers::register();
    let dir = scratch("kaynak-sayim-resim");
    let address = tables(&dir).display().to_string();
    crate::files_testing::usage_shots("9340", "02-denetci-kaynak-satirlari", || {
        let mut app = app_with_drawing();
        {
            let model = &mut app.document.as_mut().expect("a drawing").model;
            let all: Vec<kentos_domain::Slot> = model.entities().map(|e| kentos_domain::Slot(e.base().id)).collect();
            model.remove(&all);
        }
        app.add_data_layers(
            ["qa_noktalar", "qa_yollar", "qa_secim"]
                .into_iter()
                .map(|t| crate::data_layers::describe(&address, t).expect("described"))
                .collect(),
        )
        .expect("added");
        let _ = drawing_rows(&app);
        counted(&mut app);
        app
    });
}
