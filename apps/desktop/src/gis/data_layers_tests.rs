//! Veri katmanı on the desktop (docs/adr/9101): a GeoPackage table added as a data layer without its rows, its columns
//! as the layer's fields and its kind as the layer's; one undo step; the rows a view reads moved between systems.

use std::path::PathBuf;

use kentos_commands::{LayerGeometry, LayerNode};
use kentos_data::{Coord, Curve, Driver, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};

use crate::files_testing::app_with_drawing;

/// A GeoPackage with a table of `n` parcels in TM36 around the sample drawing.
pub(crate) fn parcels(name: &str, n: usize) -> PathBuf {
    let dir = std::env::temp_dir().join("kentos-desktop-data-layers");
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join(format!("{name}-{}.gpkg", std::process::id()));
    let _ = std::fs::remove_file(&path);
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "parseller".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![
                Field::new("ada", FieldKind::Int),
                Field::new("kullanim", FieldKind::Text { max: Some(20) }),
                Field::new("alan", FieldKind::Real),
            ],
            geometry: Some(GeometryField {
                name: "geom".into(),
                type_name: "MULTIPOLYGON".into(),
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
    let rows: Vec<NewFeature> = (0..n)
        .map(|i| {
            let (x, y) = (486_500.0 + (i % 20) as f64 * 30.0, 4_420_200.0 + (i / 20) as f64 * 30.0);
            NewFeature {
                fid: None,
                geometry: Some(Geometry::Polygon(vec![Curve::Line(vec![
                    Coord::xy(x, y),
                    Coord::xy(x + 25.0, y),
                    Coord::xy(x + 25.0, y + 25.0),
                    Coord::xy(x, y + 25.0),
                    Coord::xy(x, y),
                ])])),
                values: vec![
                    ("ada".into(), Value::Int(100 + (i / 20) as i64)),
                    ("kullanim".into(), Value::Text(["Konut", "Park"][i % 2].into())),
                    ("alan".into(), Value::Real(625.0)),
                ],
            }
        })
        .collect();
    tx.insert("parseller", &rows).expect("rows");
    tx.commit().expect("commit");
    path
}

pub(crate) fn node<'a>(app: &'a crate::app::App, name: &str) -> Option<&'a LayerNode> {
    app.document
        .as_ref()?
        .model
        .layers()
        .leaves()
        .into_iter()
        .find(|n| n.name == name)
}

#[test]
fn a_geopackage_table_is_added_as_a_data_layer_without_its_rows() {
    crate::data_layers::register();
    let path = parcels("eklenir", 400);
    let address = path.display().to_string();
    let described = crate::data_layers::describe(&address, "parseller").expect("described");
    assert_eq!(described.data.driver, "gpkg");
    assert_eq!(described.data.key, "fid");
    assert_eq!(
        described.data.geometry.as_ref().map(|g| (g.type_name.as_str(), g.srid)),
        Some(("MULTIPOLYGON", Some(5256)))
    );
    assert!(described.data.extent.is_some_and(|b| b[0] >= 486_500.0 && b[2] <= 486_500.0 + 600.0));
    let names: Vec<&str> = described.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["ada", "kullanim", "alan"]);
    assert_eq!(described.fields[1].length, Some(20));

    let mut app = app_with_drawing();
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    let added = app.add_data_layers(vec![described]).expect("added");
    assert_eq!(added.len(), 1);
    let n = node(&app, "parseller").expect("the layer");
    assert_eq!(n.geometry, Some(LayerGeometry::Area));
    assert_eq!(n.data.as_ref().map(|d| d.table.as_str()), Some("parseller"));
    let doc = &app.document.as_ref().expect("a drawing").model;
    // Its 400 rows stay in the file: the drawing holds no object of it.
    assert_eq!(doc.entities().count(), before);
    assert_eq!(doc.by_layer(&n.id).count(), 0);
    // One undo step takes it away.
    let _ = app.update(crate::app::Message::Run("edit.undo"));
    assert!(node(&app, "parseller").is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_table_in_another_system_is_drawn_in_the_projects() {
    let settings = app_with_drawing().document.expect("a drawing").model.settings().clone();
    let system = kentos_native_application::sources::TableSystem::of(&settings, Some(4326))
        .expect("the systems meet")
        .expect("another system");
    // A point near Sivas in WGS 84 lands in TM36's eastings and northings.
    let (x, y) = system.to_project(37.0, 39.75).expect("moved");
    assert!((500_000.0..700_000.0).contains(&x) && (4_350_000.0..4_450_000.0).contains(&y), "{x} {y}");
    let b = system.to_table([x - 100.0, y - 100.0, x + 100.0, y + 100.0]).expect("the box moved back");
    assert!(b[0] < 37.0 && b[2] > 37.0 && b[1] < 39.75 && b[3] > 39.75, "{b:?}");
}

/// The world point at `x`, `y` metres from the parcels' corner.
pub(crate) fn parcel_point(x: f64, y: f64) -> kentos_interaction::Vec2 {
    kentos_interaction::Vec2::new(486_500.0 + x, 4_420_200.0 + y)
}

#[test]
fn bilgi_on_no_object_asks_the_data_layers_and_shows_the_row_read_from_its_table() {
    crate::data_layers::register();
    let path = parcels("bilgi", 40);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    // Bilgi's click on no object of the drawing, inside the 22nd parcel (its second row, second column): the data
    // layers are asked after the update, off the thread.
    crate::tools_scenes::run(&mut app, "tool.identify");
    let at = parcel_point(42.0, 42.0);
    crate::tools_scenes::click(&mut app, [at.x - crate::tools_scenes::E, at.y - crate::tools_scenes::N]);
    assert!(app.info_card.is_none(), "no object of the drawing is there");
    assert_eq!(app.gis.data_info.asked, 1, "the data layers were asked");
    // What the thread answers: the row under the point with every column.
    let layer = node(&app, "parseller").expect("the layer").clone();
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let (found, said) = crate::data_info::ask(std::slice::from_ref(&layer), &settings, at, 0.5);
    assert!(said.is_empty(), "{said:?}");
    let found = found.expect("a row under the point");
    assert_eq!(found.key, 22);
    let _ = app.data_info_event(crate::data_info::Event::Answered(1, Some(Box::new(found)), Vec::new()));
    // The card: the layer's name, the columns in the table's order, where the row is.
    let (card, _, _) = app.data_card().expect("the row's card");
    assert_eq!(card.title, "parseller");
    let names: Vec<&str> = card.rows.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, ["ada", "kullanim", "alan"]);
    assert_eq!(card.rows[0].1, "101");
    assert_eq!(card.rows[1].1, "Park");
    assert!(
        card.source.as_ref().is_some_and(|(line, _)| line.ends_with("› parseller · fid 22")),
        "{:?}",
        card.source
    );
    assert!(app.info_card_view().is_some(), "the card is drawn beside the click");
    // An answer to an earlier click is stale.
    let _ = app.data_info_event(crate::data_info::Event::Answered(0, None, Vec::new()));
    assert!(app.data_card().is_some());
    // Between two parcels: no row.
    let gap = parcel_point(27.5, 12.0);
    assert!(crate::data_info::ask(&[layer], &settings, gap, 0.5).0.is_none());
    let _ = std::fs::remove_file(&path);
}

/// Asks a data view of its own for `layer` over the parcels and waits for its read to be drawn.
fn read_whole(
    view: &crate::data_view::DataView,
    app: &crate::app::App,
    layer: &LayerNode,
) -> Option<(String, u64, Box<kentos_domain::Document>)> {
    read_whole_as(view, app, layer, false)
}

/// [`read_whole`] with the layer being edited or not.
pub(crate) fn read_whole_as(
    view: &crate::data_view::DataView,
    app: &crate::app::App,
    layer: &LayerNode,
    editing: bool,
) -> Option<(String, u64, Box<kentos_domain::Document>)> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let look = kentos_plot::layers::Look {
        line_types: Default::default(),
        palette: kentos_native_style::StylePalette::graphite(),
        symbol_scale: 1000.0,
        screen: false,
        hairlines: false,
        hold_weights: false,
        origin: kentos_render_wgpu::scene::scene_origin(app.document.as_ref().expect("a drawing")),
        view_build: Default::default(),
        view_colors: Default::default(),
        phasing: Default::default(),
        reading_unit: kentos_native_style::reading_unit(doc.settings()),
    };
    let library = kentos_native_style::library::StyleLibrary::default();
    let bounds = kentos_geometry_core::geometry::Bounds {
        min_x: 486_480.0,
        min_y: 4_420_180.0,
        max_x: 487_150.0,
        max_y: 4_420_300.0,
    };
    view.want(&[layer], doc.settings(), doc.styles(), &bounds, 1.0, &look, &library, true, &|_| Default::default(), &|_| editing);
    let start = std::time::Instant::now();
    let mut labels = None;
    while view.drawn_read(&layer.id).is_none() {
        labels = labels.or_else(|| view.take_labels().into_iter().next());
        assert!(start.elapsed() < std::time::Duration::from_secs(30), "the read is drawn");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    labels.or_else(|| view.take_labels().into_iter().next())
}

#[test]
fn a_labelled_data_layer_keeps_its_read_for_the_label_engine_and_another_keeps_nothing() {
    crate::data_layers::register();
    let path = parcels("etiket", 40);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let plain = node(&app, "parseller").expect("the layer").clone();
    // Without a label: the read draws, nothing is kept.
    assert!(read_whole(&crate::data_view::DataView::new(), &app, &plain).is_none());
    // Labelled by its ada: the read's 40 rows in a document of their own, labelled as the drawing's objects are.
    let mut labelled = plain.clone();
    labelled.style.label = Some(kentos_commands::LabelStyle {
        template: Some("{ada}".into()),
        ..Default::default()
    });
    let (layer, _, doc) = read_whole(&crate::data_view::DataView::new(), &app, &labelled).expect("kept");
    assert_eq!(layer, labelled.id);
    assert_eq!(doc.len(), 40);
    assert!(doc.layers().get(&labelled.id).is_some_and(|n| n.data.is_none()));
    let spatial = kentos_interaction::Spatial::of(&doc);
    assert_eq!(spatial.label_texts(kentos_domain::Slot(1)), ["100"]);
    assert_eq!(spatial.label_texts(kentos_domain::Slot(40)), ["101"]);
    let _ = std::fs::remove_file(&path);
}

/// What the table's reads would answer: each page read as its task reads it, answered in order.
pub(crate) fn answer_pages(app: &mut crate::app::App) {
    use crate::features::data::DataEvent;
    for _ in 0..4 {
        let _ = app.data_table_tasks();
        let asked = app.gis.features.data.pending();
        if asked.is_empty() {
            return;
        }
        for (generation, page, input) in asked {
            let node = app.data_table_layer().expect("a data layer in the table");
            let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
            let answer = kentos_native_application::layers_rows::read(&node, &settings, &input, &|| false)
                .map(Box::new)
                .map_err(|e| e.message);
            let _ = app.data_table_event(DataEvent::Answered(generation, page, answer));
        }
    }
}

#[test]
fn a_data_layer_s_table_reads_its_pages_from_the_source_and_orders_and_searches_there() {
    use crate::features::data::DataEvent;
    crate::data_layers::register();
    let path = parcels("tablo", 450);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let layer = node(&app, "parseller").expect("the layer").id.clone();
    app.gis.features.layer = Some(layer);
    app.command_expanded = true;
    app.bottom_tab = crate::bottom::BottomTab::Table;
    // The table shows its first rows: the first page and the count come from the source.
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    answer_pages(&mut app);
    let data = &app.gis.features.data;
    assert_eq!(data.total_rows(), Some(450));
    assert_eq!(data.row(0).map(|r| r.key), Some(1));
    assert_eq!(data.row(199).map(|r| r.key), Some(200));
    assert!(data.row(200).is_none(), "a page the table does not show is not read");
    // Scrolled to the end: its page is read.
    let _ = app.data_table_event(DataEvent::Range(440, 450));
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.row(449).map(|r| r.key), Some(450));
    // A header orders by its field, down the second time: kullanim, Turkish letters in place.
    let _ = app.data_table_event(DataEvent::Sort("kullanim".into()));
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    answer_pages(&mut app);
    let first = app.gis.features.data.row(0).and_then(|r| r.attrs.get("kullanim").cloned());
    assert_eq!(first.as_deref(), Some("Konut"));
    let _ = app.data_table_event(DataEvent::Sort("kullanim".into()));
    answer_pages(&mut app);
    let first = app.gis.features.data.row(0).and_then(|r| r.attrs.get("kullanim").cloned());
    assert_eq!(first.as_deref(), Some("Park"));
    // Ara is asked with Enter, not at every letter.
    app.gis.features.search = "park".into();
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(450), "typed, not asked");
    let _ = app.data_table_event(DataEvent::Apply);
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(225));
    // An answer to an earlier question is stale.
    let _ = app.data_table_event(DataEvent::Answered(0, 0, Err("eski".into())));
    assert!(app.gis.features.data.row(0).is_some());
    let _ = std::fs::remove_file(&path);
}
