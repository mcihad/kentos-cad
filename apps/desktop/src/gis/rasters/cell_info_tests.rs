//! Bilgi on a local raster (docs/adr/9137), the shared raster drawing's `bolge.tif` (12 × 10 Float32 cells of 2 m from
//! 500 000, 4 420 000; NoData NaN). GDAL's `gdallocationinfo` reads cell (0, 0) as 110.529121398926 and (11, 9) as
//! 131.867736816406, (6, 2) as NaN: Bilgi's card says the same, the no-data cell as no value, and a click left of the
//! raster says it is outside. Beside an object the card offers the raster's value and back.

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::drive;

const DRAWING: &str = include_str!("../../../../../fixtures/processing/v1/raster-ops.kcad");

/// The raster drawing, taken to be in its rasters' folder, only `bolge` shown of its layers (its rasters lie one over
/// another), with Bilgi on.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let mut json: serde_json::Value = serde_json::from_str(DRAWING).expect("the drawing reads");
    fn only_bolge(nodes: &mut serde_json::Value) {
        for n in nodes.as_array_mut().into_iter().flatten() {
            if n["type"] == "layer" && n["id"] != "bolge" {
                n["visible"] = serde_json::Value::Bool(false);
            }
            only_bolge(&mut n["children"]);
        }
    }
    only_bolge(&mut json["layers"]);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&json.to_string()).expect("the drawing reads");
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/processing/v1/raster-ops/raster-ops.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::tools_scenes::run(&mut app, "tool.identify");
    app
}

/// Bilgi's click on no object at the world point (`x`, `y`), its asks answered.
fn click(app: &mut App, x: f64, y: f64) {
    app.gis.data_info.wanted.push((kentos_interaction::Vec2::new(x, y), 0.5));
    let task = app.data_info_tasks();
    drive(app, task);
}

fn rows(app: &App) -> Vec<(String, String)> {
    app.raster_card().map(|(c, _)| c.rows).unwrap_or_default()
}

fn band(app: &App) -> String {
    rows(app).into_iter().find(|(k, _)| k == "Bant 1").map(|(_, v)| v).unwrap_or_default()
}

#[test]
fn bilgi_says_a_rasters_cell_as_gdal_reads_it() {
    let mut app = app();
    // Cell (0, 0): its centre 500 001, 4 419 999.
    click(&mut app, 500_001.0, 4_419_999.0);
    assert!(app.gis.data_info.showing_raster, "no row: the raster's cell is the card");
    let card = app.raster_card().expect("the card").0;
    assert_eq!(card.title, "Raster: Bölge rasteri");
    // The file's 32-bit value, shown by the digits that give it back.
    let v: f32 = band(&app).parse().expect("a number");
    assert_eq!(v, 110.529121398926_f64 as f32, "{v}");
    assert_eq!(band(&app), "110.52912");
    assert!(rows(&app).contains(&("Hücre".to_owned(), "sütun 0, satır 0".to_owned())), "{:?}", rows(&app));
    assert!(rows(&app).iter().any(|(k, v)| k == "Konum" && v.starts_with("Y 500") && v.contains("X 4")), "{:?}", rows(&app));
    assert!(app.info_card_view().is_some(), "drawn beside the click");
    // Cell (11, 9), the last.
    click(&mut app, 500_023.0, 4_419_981.0);
    let v: f32 = band(&app).parse().expect("a number");
    assert_eq!(v, 131.867736816406_f64 as f32, "{v}");
    // Cell (6, 2) holds the file's no-data value.
    click(&mut app, 500_013.0, 4_419_995.0);
    assert_eq!(band(&app), "değer yok (NoData)");
    // Left of every raster: said, no card.
    click(&mut app, 499_000.0, 4_419_999.0);
    assert!(!app.gis.data_info.showing_raster);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|t| t == "Tıklanan nokta gösterilen rasterlerin kapsamı dışında."), "{said:#?}");
}

#[test]
fn beside_an_object_the_card_offers_the_rasters_value_and_back() {
    let mut app = app();
    // A click on an object over the raster: its card, and the raster under it asked after the update.
    let slot = app.document.as_ref().expect("open").model.entities().map(|e| kentos_domain::Slot(e.base().id)).next().expect("an object");
    let at = kentos_interaction::Vec2::new(500_001.0, 4_419_999.0);
    app.info_card = Some((slot, at));
    app.gis.data_info.beside.push((at, 0.5));
    let task = app.data_info_tasks();
    drive(&mut app, task);
    assert!(!app.gis.data_info.showing_raster, "the object's card first");
    assert_eq!(app.gis.data_info.cells.len(), 1, "the raster's value offered");
    let _ = app.update(Message::InfoCard(crate::info_card::Event::Raster(true)));
    assert!(app.gis.data_info.showing_raster);
    let _ = app.update(Message::InfoCard(crate::info_card::Event::Raster(false)));
    assert!(!app.gis.data_info.showing_raster, "back to the object");
}

/// Pictures (docs/adr/9137), light at 1440×900 and dark at 1100×760: Bilgi's card on a cell of the raster, and on its
/// no-data cell.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib cell_info_tests::screens -- --ignored --nocapture"]
fn screens() {
    let prepare = |x: f64, y: f64| {
        move || {
            let mut app = app();
            let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1100.0, 600.0));
            let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
            let _ = app.update(Message::Run("view.zoomExtents"));
            click(&mut app, x, y);
            app
        }
    };
    crate::files_testing::usage_shots("9137", "01-hucre", prepare(500_005.0, 4_419_991.0));
    crate::files_testing::usage_shots("9137", "02-nodata", prepare(500_013.0, 4_419_995.0));
}
