//! The municipal report's 10th item, its last part (docs/adr/9177): a road line layer of a GeoPackage classified by its
//! durum while every road was planlı, then one road turned kontrol in the file by another program. The style window's
//! count, the map, the paper (a sheet's map and its PDF), the table and the legend follow one rule. With Diğer değerler
//! off the kontrol road is counted as taken by no category, drawn neither on the screen nor on paper, keeps its kontrol
//! in the table and has no legend row; Diğer değerleri aç draws it on both and gives it its legend row.

use kentos_data::{
    Coord, Curve, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value,
};

use crate::app::{App, Message};
use crate::data_layers_tests::{answer_pages, node, read_with};
use crate::features::data::DataEvent;
use crate::files_testing::app_with_drawing;
use crate::style::layer_style::source_values::{self, Then};
use crate::style::layer_style::{Event as Style, Field as StyleField, Kind};

/// `yollar` (ad, durum): Y1 and Y2, both planlı, two east–west roads 50 m apart inside the view the reads cover.
fn roads(tag: &str) -> (String, std::path::PathBuf) {
    let path = crate::files_testing::scratch(tag).join("Yollar.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let address = path.display().to_string();
    let source = kentos_data::registry()
        .source(&address)
        .expect("the source");
    let mut tx = source.begin().expect("begin");
    let text = |n: &str| Field::new(n, FieldKind::Text { max: Some(16) });
    tx.create_table(
        &NewTable {
            name: "yollar".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![text("ad"), text("durum")],
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
    .expect("the table");
    let road = |y: f64, ad: &str| NewFeature {
        fid: None,
        geometry: Some(Geometry::Curve(Curve::Line(vec![
            Coord::xy(486_600.0, y),
            Coord::xy(486_900.0, y),
        ]))),
        values: vec![
            ("ad".into(), Value::Text(ad.into())),
            ("durum".into(), Value::Text("planlı".into())),
        ],
    };
    tx.insert(
        "yollar",
        &[road(4_420_200.0, "Y1"), road(4_420_250.0, "Y2")],
    )
    .expect("the roads");
    tx.commit().expect("written");
    (address, path)
}

fn style(app: &mut App, event: Style) {
    let _ = app.update(Message::LayerStyle(event));
}

/// The window's read of `field` from the source, run here and handed back as the window's thread would.
fn hand_back(app: &mut App, field: &str, then: Then) {
    let data = app
        .styles
        .layer_style
        .as_ref()
        .and_then(|w| w.data.clone())
        .expect("a data layer's window");
    let read = source_values::read(&data, field, then);
    style(app, Style::SourceValues(then, read));
}

/// Katman stili opened on the roads, its counts read from the source: the Nesne column and what no category takes.
fn counted(app: &mut App, layer: &str) -> (Vec<usize>, usize) {
    style(app, Style::Open(Some(layer.to_owned())));
    hand_back(app, "durum", Then::Show);
    app.styles
        .layer_style
        .as_ref()
        .and_then(|w| w.source_category_tally())
        .expect("counted by the source")
}

/// The roads drawn from a read of their own: how far apart in north the drawn lines lie (0: one road, 50: both) and
/// how many symbols draw them; on the model screen (`hold`) or on paper.
fn drawn(app: &App, layer: &str, hold: bool) -> (f64, usize) {
    let node = node(app, "yollar").expect("the roads").clone();
    let view = crate::data_view::DataView::new();
    let _ = read_with(&view, app, &node, false, hold);
    let (mut low, mut high, mut symbols) = (f64::INFINITY, f64::NEG_INFINITY, 0);
    for part in view.parts(layer) {
        for b in part.batches() {
            if matches!(
                b.kind,
                kentos_native_style::batches::BatchKind::Stroke { .. }
            ) {
                low = low.min(b.origin[1] + b.bounds[1]);
                high = high.max(b.origin[1] + b.bounds[3]);
                symbols += 1;
            }
        }
    }
    assert!(symbols > 0, "a road is drawn");
    ((high - low).round(), symbols)
}

/// The table's durum of the road named `ad`, its rows read from the source.
fn table_durum(app: &mut App, layer: &str, ad: &str) -> Option<String> {
    app.gis.features.layer = Some(layer.to_owned());
    app.command_expanded = true;
    app.bottom_tab = crate::bottom::BottomTab::Table;
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    answer_pages(app);
    let data = &app.gis.features.data;
    (0..data.total_rows().unwrap_or(0) as usize)
        .filter_map(|i| data.row(i))
        .find(|r| r.attrs.get("ad").map(String::as_str) == Some(ad))
        .and_then(|r| r.attrs.get("durum").cloned())
}

/// The legend's rows of the roads, as the Lejant window and a sheet's legend build them.
fn legend(app: &App) -> Vec<String> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let window = crate::style::legend::LegendWindow::default();
    let node = node(app, "yollar").expect("the roads").id.clone();
    window
        .groups(doc, &app.styles.library)
        .iter()
        .filter(|g| g.layer_id == node)
        .flat_map(|g| g.entries.iter().map(|e| e.label.clone()))
        .collect()
}

#[test]
fn a_road_turned_kontrol_is_counted_drawn_listed_and_in_the_legend_by_one_rule() {
    crate::data_layers::register();
    let (address, path) = roads("planli-kontrol");
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&address, "yollar").expect("described"),
    ])
    .expect("added");
    let layer = node(&app, "yollar").expect("the roads").id.clone();

    // Kategorili by durum, classified from the source while both roads are planlı; Diğer değerler off.
    style(&mut app, Style::Open(Some(layer.clone())));
    style(&mut app, Style::Kind(Kind::Categorized));
    style(
        &mut app,
        Style::Expr(StyleField::Categories, "durum".into()),
    );
    style(&mut app, Style::Classify);
    hand_back(&mut app, "durum", Then::Classify);
    let w = app.styles.layer_style.as_ref().expect("the window");
    let values: Vec<&str> = w
        .categorized
        .categories
        .iter()
        .map(|k| k.value.as_str())
        .collect();
    assert_eq!(values, ["planlı"]);
    // A road layer's symbols are lines: its table's geometry says so, though the drawing holds none of its rows.
    assert_eq!(
        w.classes,
        [kentos_native_style::renderer::GeometryClass::Line]
    );
    let planli = &w.categorized.categories[0].symbols;
    assert!(
        planli.line.is_some() && planli.fill.is_none() && planli.marker.is_none(),
        "{planli:?}"
    );
    assert!(w.categorized.other.is_none(), "Diğer değerler off");
    style(&mut app, Style::Done);
    assert_eq!(drawn(&app, &layer, true).0, 50.0, "both planlı roads drawn");
    assert_eq!(
        legend(&app),
        ["planlı"],
        "the data layer's legend row, though the drawing holds none of its rows"
    );

    // Another program turns Y2 kontrol in the file.
    let gp = kentos_geopackage::GeoPackage::open(&path, true).expect("the file");
    gp.connection()
        .execute("UPDATE yollar SET durum = 'kontrol' WHERE ad = 'Y2'", [])
        .expect("turned");
    drop(gp);
    kentos_data::registry().forget(&address);

    // One rule, Diğer değerler off: counted as taken by no category, drawn neither on the screen nor on paper, listed
    // with its kontrol, no legend row.
    assert_eq!(
        counted(&mut app, &layer),
        (vec![1], 1),
        "planlı 1, one road no category takes"
    );
    style(&mut app, Style::Close);
    assert_eq!(
        drawn(&app, &layer, true),
        (0.0, 1),
        "the screen draws Y1 alone"
    );
    assert_eq!(drawn(&app, &layer, false), (0.0, 1), "paper draws Y1 alone");
    assert_eq!(
        table_durum(&mut app, &layer, "Y2").as_deref(),
        Some("kontrol")
    );
    assert_eq!(
        table_durum(&mut app, &layer, "Y1").as_deref(),
        Some("planlı")
    );
    assert_eq!(legend(&app), ["planlı"]);

    // Diğer değerleri aç: both drawn on the screen and on paper, each by its own symbol, and the legend says how.
    style(&mut app, Style::Open(Some(layer.clone())));
    hand_back(&mut app, "durum", Then::Show);
    style(&mut app, Style::Other(true));
    style(&mut app, Style::Done);
    assert_eq!(counted(&mut app, &layer), (vec![1], 1), "the same count");
    assert!(
        app.styles
            .layer_style
            .as_ref()
            .is_some_and(|w| w.categorized.other.is_some()),
        "now drawn by Diğer"
    );
    style(&mut app, Style::Close);
    assert_eq!(
        drawn(&app, &layer, true),
        (50.0, 2),
        "the screen draws both, each by its symbol"
    );
    assert_eq!(drawn(&app, &layer, false), (50.0, 2), "paper draws both");
    assert_eq!(legend(&app), ["planlı", "Diğer değerler"]);
}

/// The roads with Y2 turned kontrol and Diğer değerler off.
fn turned(tag: &str) -> (App, String) {
    crate::data_layers::register();
    let (address, path) = roads(tag);
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&address, "yollar").expect("described"),
    ])
    .expect("added");
    let layer = node(&app, "yollar").expect("the roads").id.clone();
    style(&mut app, Style::Open(Some(layer.clone())));
    style(&mut app, Style::Kind(Kind::Categorized));
    style(
        &mut app,
        Style::Expr(StyleField::Categories, "durum".into()),
    );
    style(&mut app, Style::Classify);
    hand_back(&mut app, "durum", Then::Classify);
    style(&mut app, Style::Done);
    let gp = kentos_geopackage::GeoPackage::open(&path, true).expect("the file");
    gp.connection()
        .execute("UPDATE yollar SET durum = 'kontrol' WHERE ad = 'Y2'", [])
        .expect("turned");
    drop(gp);
    kentos_data::registry().forget(&address);
    (app, layer)
}

/// The pictures (`.run/gui-evidence/kullanim-9177/`): Katman stili counting the road no category takes and saying it
/// is not drawn; the Lejant window with the roads' planlı row.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9177", "01-sinif-sayaci", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (mut app, layer) = turned(&format!("planli-kontrol-resim-{n}"));
        let _ = counted(&mut app, &layer);
        app
    });
    crate::files_testing::usage_shots("9177", "02-lejant", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (mut app, _) = turned(&format!("planli-kontrol-lejant-{n}"));
        let _ = app.update(Message::Run("style.legend"));
        app
    });
}
