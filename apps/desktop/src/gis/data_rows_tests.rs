//! İstatistik over a data layer (docs/adr/9130), the report's fixed data: six maintenance points whose bedel is 0, 100,
//! 250, NULL, 50 and 100 and whose durum is acil, planlı and tamam twice each. Their rows are read from the source:
//! while they are read the tab says so, then 5 filled, 1 empty, sum 500, mean 100, least 0; durum's values three, two
//! each. A row changed in the edit session is counted as the drawing has it.

use kentos_data::{Coord, Driver as _, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive, scratch};
use crate::gis;

/// A GeoPackage with the `bakim` table of the report.
fn bakim(tag: &str) -> std::path::PathBuf {
    let path = scratch(tag).join("bakim.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "bakim".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![
                Field::new("kod", FieldKind::Text { max: Some(8) }),
                Field::new("bedel", FieldKind::Real),
                Field::new("durum", FieldKind::Text { max: Some(16) }),
            ],
            geometry: Some(GeometryField {
                name: "geom".into(),
                type_name: "POINT".into(),
                z: false,
                m: false,
                srid: Some(5255),
            }),
            spatial_index: true,
            system: None,
        },
        false,
    )
    .expect("table");
    let bedel = [Some(0.0), Some(100.0), Some(250.0), None, Some(50.0), Some(100.0)];
    let durum = ["acil", "acil", "planlı", "planlı", "tamam", "tamam"];
    let rows: Vec<NewFeature> = (0..6)
        .map(|i| NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(484_000.0 + i as f64 * 10.0, 4_420_000.0)))),
            values: vec![
                ("kod".into(), Value::Text(format!("B0{}", i + 1))),
                ("bedel".into(), bedel[i].map_or(Value::Null, Value::Real)),
                ("durum".into(), Value::Text(durum[i].into())),
            ],
        })
        .collect();
    tx.insert("bakim", &rows).expect("rows");
    tx.commit().expect("commit");
    path
}

fn ev(app: &mut App, e: crate::statistics::Event) {
    let task = app.update(Message::Gis(gis::Event::Statistics(e)));
    drive(app, task);
}

/// A row of İstatistik's summary by its field's label: its cells.
fn summary(app: &App, label: &str) -> Vec<String> {
    let (_, rows) = app.statistics_seen().expect("the tab is open");
    rows.into_iter()
        .map(|r| r.split(" | ").map(str::to_owned).collect::<Vec<_>>())
        .find(|cells| cells.first().is_some_and(|c| c == label))
        .unwrap_or_else(|| panic!("no summary of {label}"))
}

#[test]
fn istatistik_counts_a_data_layers_rows_from_its_source() {
    crate::data_layers::register();
    let path = bakim("istatistik-veri");
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "bakim").expect("described")])
        .expect("added");
    let layer = crate::data_layers_tests::node(&app, "bakim").expect("the layer").id.clone();
    let task = app.update(Message::Run("data.statistics"));
    drive(&mut app, task);
    ev(&mut app, crate::statistics::Event::Layer(layer.clone()));
    // Counted once: the rows asked of the source, the tab saying it reads them.
    let (count, _) = app.statistics_seen().expect("the tab is open");
    assert_eq!(count, "0 nesne");
    assert_eq!(app.layer_rows_from(), Some(crate::charts::data::From::Reading));
    // Read after the message (a frame does it in the program), then counted.
    let task = app.data_rows_tasks();
    drive(&mut app, task);
    assert_eq!(app.layer_rows_from(), None, "read again for the rows");
    let bedel = summary(&app, "bedel");
    // Alan | Tür | Dolu | Boş | Benzersiz | En az | En çok | Ortalama | Toplam
    assert_eq!((bedel[2].as_str(), bedel[3].as_str()), ("5", "1"), "{bedel:?}");
    assert_eq!(bedel[5].replace(',', ".").parse::<f64>().ok(), Some(0.0), "{bedel:?}");
    assert_eq!(bedel[7].replace(',', ".").parse::<f64>().ok(), Some(100.0), "{bedel:?}");
    assert_eq!(bedel[8].replace(',', ".").parse::<f64>().ok(), Some(500.0), "{bedel:?}");
    let durum = summary(&app, "durum");
    assert_eq!((durum[2].as_str(), durum[4].as_str()), ("6", "3"), "{durum:?}");
    // Seçililer (docs/adr/9156): B02 and B03 chosen in the table: two rows, 100 and 250.
    app.gis.features.data.choose_rows(&layer, [2, 3].into());
    ev(&mut app, crate::statistics::Event::Show(crate::features::Show::Selected));
    let bedel = summary(&app, "bedel");
    assert_eq!(bedel[2].as_str(), "2", "{bedel:?}");
    assert_eq!(bedel[8].replace(',', ".").parse::<f64>().ok(), Some(350.0), "{bedel:?}");
    // Görünümdekiler: the view on the first three points alone, as the project's system has them (the source's 5255
    // drawn in the project's).
    let rows = app.data_layer_rows(&layer).expect("a data layer").expect("read").expect("ready");
    let first: Vec<(f64, f64)> = rows
        .iter()
        .filter(|e| matches!(e.base().attrs.get("kod").map(String::as_str), Some("B01" | "B02" | "B03")))
        .map(|e| {
            let b = kentos_geometry_core::entity::entity_bounds(&kentos_native_application::geometry::shape(e));
            (b.min_x, b.min_y)
        })
        .collect();
    assert_eq!(first.len(), 3);
    let (x0, x1) = first.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (y0, y1) = first.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: x0 - 1.0, min_y: y0 - 1.0, max_x: x1 + 1.0, max_y: y1 + 1.0 },
        0.0,
    );
    ev(&mut app, crate::statistics::Event::Show(crate::features::Show::InView));
    let bedel = summary(&app, "bedel");
    assert_eq!(bedel[2].as_str(), "3", "B01, B02, B03 in the view: {bedel:?}");
    assert_eq!(bedel[8].replace(',', ".").parse::<f64>().ok(), Some(350.0), "{bedel:?}");
}

#[test]
fn katman_suzgeci_counts_a_data_layers_rows_from_its_source() {
    crate::data_layers::register();
    let path = bakim("suzgec-veri");
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "bakim").expect("described")])
        .expect("added");
    let layer = crate::data_layers_tests::node(&app, "bakim").expect("the layer").id.clone();
    let task = app.open_layer_filter(Some(layer.clone()));
    drive(&mut app, task);
    let filter = |app: &mut App, e: crate::layer_filters::Event| {
        let task = app.update(Message::Gis(gis::Event::LayerFilter(e)));
        drive(app, task);
    };
    // Typed: the rows asked of the source (the count says it waits for them), read, and counted over again.
    filter(&mut app, crate::layer_filters::Event::Expression("bedel >= 100".into()));
    let lines = app.gis.layer_filter.as_ref().expect("the window").lines_for_test();
    assert!(lines.iter().any(|l| l.starts_with("3 / 6 nesne süzgeçten geçiyor")), "{lines:?}");
    filter(&mut app, crate::layer_filters::Event::Save);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|t| t.contains("süzgeci yazıldı: 3 / 6 satır görünür")), "{said:#?}");
}
