//! A data layer's box and its rows' outline as its source has them now (docs/adr/9327): a table written after it was
//! added has a box the layer did not keep; a table made empty has none; the outline holds the rows up to the most asked
//! and says whether the table had more.

use kentos_commands::{DataLayer, ProjectSettings};
use kentos_data::{Cancel, Coord, Driver, Geometry, GeometryField, NewFeature, NewTable};
use kentos_native_application::data_describe;
use kentos_native_application::data_extent::{data_extent, data_outline};

/// Points written into `table` of `source`, in one transaction.
fn add_points(source: &dyn kentos_data::Source, table: &str, at: &[(f64, f64)]) {
    let mut tx = source.begin().expect("begin");
    let rows: Vec<NewFeature> = at
        .iter()
        .map(|&(x, y)| NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(x, y)))),
            values: Vec::new(),
        })
        .collect();
    tx.insert(table, &rows).expect("inserted");
    tx.commit().expect("committed");
}

/// A new GeoPackage whose point table `table` (EPSG:5256) holds `at`; its address.
fn points(tag: &str, table: &str, at: &[(f64, f64)]) -> String {
    let dir = std::env::temp_dir().join(format!("kentos-kapsam-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join(format!("{table}.gpkg"));
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let address = path.display().to_string();
    let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
    let mut tx = source.begin().expect("begin");
    let made = NewTable {
        name: table.to_owned(),
        title: None,
        description: None,
        key: "fid".into(),
        fields: Vec::new(),
        geometry: Some(GeometryField {
            name: "geom".into(),
            type_name: "POINT".into(),
            z: false,
            m: false,
            srid: Some(5256),
        }),
        spatial_index: true,
        system: None,
    };
    tx.create_table(&made, false).expect("made");
    tx.commit().expect("committed");
    add_points(source.as_ref(), table, at);
    address
}

/// A CBS project's settings in EPSG:5256, the tables' system.
fn settings() -> ProjectSettings {
    let snapshot =
        kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
            name: "Kapsam".into(),
            srid: 5256,
            plot_scale: 1000.0,
            workspace: kentos_commands::Workspace::Gis,
            drawing_font: kentos_commands::DrawingFont::Barlow,
            province: None,
            drawing_unit: None,
            annotation: None,
            cad_work: None,
            heights: None,
        })
        .expect("a project");
    kentos_domain::Document::from_snapshot(snapshot)
        .expect("a document")
        .settings()
        .clone()
}

#[test]
fn the_source_says_the_box_and_the_rows_it_has_now() {
    kentos_data_gpkg::register();
    let address = points(
        "yazilan",
        "noktalar",
        &[(484_010.0, 4_420_020.0), (484_030.0, 4_420_060.0)],
    );
    let data = data_describe::describe(&address, "noktalar")
        .expect("described")
        .data;
    assert_eq!(
        data.extent,
        Some([484_010.0, 4_420_020.0, 484_030.0, 4_420_060.0]),
        "as added"
    );
    let b = data_extent(&settings(), &data)
        .expect("read")
        .expect("a box");
    assert_eq!(b.table, [484_010.0, 4_420_020.0, 484_030.0, 4_420_060.0]);
    assert_eq!(b.project, b.table, "the same system");
    // A row written after the layer was added: outside the box the layer keeps.
    let source = kentos_data::registry().source(&address).expect("opened");
    add_points(source.as_ref(), "noktalar", &[(483_995.0, 4_420_100.0)]);
    let b = data_extent(&settings(), &data)
        .expect("read")
        .expect("a box");
    assert_eq!(
        b.table,
        [483_995.0, 4_420_020.0, 484_030.0, 4_420_100.0],
        "the rows written since"
    );
    let all = data_outline(&settings(), &data, "L1", &b, 256.0, 10, &Cancel::new()).expect("read");
    assert_eq!((all.rows, all.more), (3, false));
    assert_eq!(all.store.len(), 3);
    // Past the most asked: the first rows, and the box after them as an area.
    let some = data_outline(&settings(), &data, "L1", &b, 256.0, 2, &Cancel::new()).expect("read");
    assert_eq!((some.rows, some.more), (2, true));
    assert_eq!(some.store.len(), 3, "two rows and the box");
    let e = some.store.overview_extent().expect("an extent");
    assert_eq!(
        [e.min_x, e.min_y, e.max_x, e.max_y],
        b.project,
        "the box whole"
    );
}

#[test]
fn an_empty_table_and_a_layer_without_geometry_have_no_box() {
    kentos_data_gpkg::register();
    let address = points("bos", "bos", &[]);
    let data = data_describe::describe(&address, "bos")
        .expect("described")
        .data;
    assert_eq!(data_extent(&settings(), &data).expect("read"), None);
    let bare = DataLayer {
        geometry: None,
        ..data
    };
    assert_eq!(data_extent(&settings(), &bare).expect("read"), None);
}

#[test]
fn a_source_that_is_not_there_says_why() {
    kentos_data_gpkg::register();
    let address = points("giden", "giden", &[(484_000.0, 4_420_000.0)]);
    let data = data_describe::describe(&address, "giden")
        .expect("described")
        .data;
    let gone = DataLayer {
        table: "baska".into(),
        ..data
    };
    let why = data_extent(&settings(), &gone).expect_err("no such table");
    assert!(why.contains("“baska”"), "{why}");
}
