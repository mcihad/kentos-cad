//! Bilgi on data layers' rows (docs/adr/9341; the CBS review of 27 November 2026, item 6). Only data layers of a
//! GeoPackage in EPSG:5256: `qa_yollar` with the roads AB, BC and CB (BC again, the other way) and `qa_secim` with AB.
//!
//! - A click at C, where BC and CB end, finds both: the card is BC's (key 2), Sonraki goes to CB's (3) and round
//!   again; the order is the same at every click.
//! - The card offers Seç, Tabloda aç and Formu göster. Bilgi changes no selection by itself; Seç chooses the row in its
//!   table (the table comes to its layer), Tabloda aç also shows the chosen row; with no form Formu göster is off.

use std::collections::BTreeSet;

use kentos_data::{
    Coord, Curve, Driver as _, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable,
    Value,
};
use kentos_interaction::Vec2;

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive, scratch};
use crate::info_card::Event as Card;

const A: (f64, f64) = (487_000.0, 4_420_000.0);
const B: (f64, f64) = (487_100.0, 4_420_050.0);
const C: (f64, f64) = (487_200.0, 4_420_100.0);

fn roads(tag: &str) -> std::path::PathBuf {
    let path = scratch(tag).join("qa.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg
        .open(path.to_str().expect("utf-8"))
        .expect("open");
    let mut tx = source.begin().expect("begin");
    for table in ["qa_yollar", "qa_secim"] {
        tx.create_table(
            &NewTable {
                name: table.into(),
                title: None,
                description: None,
                key: "fid".into(),
                fields: vec![Field::new("ad", FieldKind::Text { max: Some(16) })],
                geometry: Some(GeometryField {
                    name: "geom".into(),
                    type_name: "LINESTRING".into(),
                    z: false,
                    m: false,
                    srid: Some(5256),
                }),
                spatial_index: true,
                system: None,
            },
            false,
        )
        .expect("table");
    }
    let line = |ad: &str, a: (f64, f64), b: (f64, f64)| NewFeature {
        fid: None,
        geometry: Some(Geometry::Curve(Curve::Line(vec![
            Coord::xy(a.0, a.1),
            Coord::xy(b.0, b.1),
        ]))),
        values: vec![("ad".into(), Value::Text(ad.into()))],
    };
    tx.insert(
        "qa_yollar",
        &[line("AB", A, B), line("BC", B, C), line("CB", C, B)],
    )
    .expect("roads");
    tx.insert("qa_secim", &[line("AB", A, B)]).expect("chosen");
    tx.commit().expect("commit");
    path
}

fn scene(tag: &str) -> (App, String) {
    crate::data_layers::register();
    let address = roads(tag).display().to_string();
    let mut app = app_with_drawing();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let all: Vec<kentos_domain::Slot> = model
            .entities()
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        model.remove(&all);
    }
    let ids = app
        .add_data_layers(
            ["qa_yollar", "qa_secim"]
                .into_iter()
                .map(|t| crate::data_layers::describe(&address, t).expect("described"))
                .collect(),
        )
        .expect("added");
    let task = app.update(Message::Run("tool.identify"));
    drive(&mut app, task);
    (app, ids[0].clone())
}

/// Bilgi's click on no object of the drawing at `at`, answered.
fn click(app: &mut App, at: (f64, f64)) {
    app.gis.data_info.wanted.push((Vec2::new(at.0, at.1), 0.5));
    let task = app.data_info_tasks();
    drive(app, task);
}

fn card(app: &App) -> Option<(String, i64)> {
    app.gis
        .data_info
        .card
        .as_ref()
        .map(|f| (f.layer.clone(), f.key))
}

fn press(app: &mut App, e: Card) {
    let task = app.update(Message::InfoCard(e));
    drive(app, task);
}

#[test]
fn every_row_under_a_click_is_a_candidate_in_one_order() {
    let (mut app, yollar) = scene("bilgi-adaylar");
    click(&mut app, C);
    let rows: Vec<(String, i64)> = app
        .gis
        .data_info
        .rows
        .iter()
        .map(|f| (f.layer.clone(), f.key))
        .collect();
    assert_eq!(rows, [(yollar.clone(), 2), (yollar.clone(), 3)]);
    assert_eq!(card(&app), Some((yollar.clone(), 2)));
    assert_eq!(
        app.info_place(),
        Some((0, 2)),
        "Bu noktada 2 veri satırı · 1/2"
    );
    press(&mut app, Card::Next);
    assert_eq!(card(&app), Some((yollar.clone(), 3)));
    press(&mut app, Card::Next);
    assert_eq!(card(&app), Some((yollar.clone(), 2)), "round to the first");
    // The same order at the next click.
    click(&mut app, C);
    let again: Vec<i64> = app.gis.data_info.rows.iter().map(|f| f.key).collect();
    assert_eq!(again, [2, 3]);
    // Bilgi itself selects nothing.
    assert!(app.selection.is_empty() && app.gis.features.data.selected_layer().is_none());
}

#[test]
fn the_card_chooses_its_row_and_shows_it_in_the_table() {
    let (mut app, yollar) = scene("bilgi-eylemler");
    let mid = ((B.0 + C.0) / 2.0, (B.1 + C.1) / 2.0);
    click(&mut app, mid);
    press(&mut app, Card::Next);
    assert_eq!(card(&app), Some((yollar.clone(), 3)), "CB");
    assert_eq!(app.info_card_layer().as_deref(), Some(yollar.as_str()));
    assert!(
        app.form_of_layer(&yollar).is_none(),
        "no form: Formu göster is off"
    );
    press(&mut app, Card::Choose);
    assert_eq!(
        app.gis.features.data.selected_layer(),
        Some(yollar.as_str())
    );
    assert_eq!(app.gis.features.data.selected, BTreeSet::from([3]));
    assert_eq!(
        app.data_table_layer().map(|n| n.id),
        Some(yollar.clone()),
        "the table on the row's layer"
    );
    assert!(app.selection.is_empty(), "no object of the drawing chosen");
    press(&mut app, Card::Table);
    assert_eq!(
        app.gis.features.show,
        crate::features::Show::Selected,
        "the chosen row shown"
    );
    assert_eq!(app.gis.features.data.selected, BTreeSet::from([3]));
}

#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9341", "01-bilgi-karti-eylemler", || {
        let (mut app, _) = scene("bilgi-resim");
        let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0)),
        )));
        app.zoom_to_bounds(&kentos_commands::Bounds {
            min_x: A.0 - 60.0,
            min_y: A.1 - 60.0,
            max_x: C.0 + 60.0,
            max_y: C.1 + 60.0,
        });
        click(&mut app, C);
        app
    });
}
