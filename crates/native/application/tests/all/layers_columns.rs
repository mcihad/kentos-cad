//! `cad.layers.columns` where a shared case cannot go (docs/adr/9318): a GeoPackage the file system keeps read-only takes
//! no column (`columns_not_permitted`, the file as it was), a table whose file is gone is said (`source_unreachable`),
//! and a column added is read back by every reader with the rows empty in it.

use kentos_commands::{
    CommandResult, LayerColumnsAction, LayerDataOperation, LayerField, LayerFieldKind,
    LayersColumns, LayersData, Workspace,
};
use kentos_data::{
    Coord, Driver, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value,
};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, data_describe, layers_columns};

/// A GeoPackage with `saha` (durum) and its two rows, in a folder of its own.
fn field_assets(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-sutun-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("saha.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg
        .open(&path.display().to_string())
        .expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "saha".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![Field::new("durum", FieldKind::Text { max: None })],
            geometry: Some(GeometryField {
                name: "geom".into(),
                type_name: "POINT".into(),
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
    let rows: Vec<NewFeature> = ["A", "B"]
        .iter()
        .enumerate()
        .map(|(i, d)| NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(
                600_000.0 + i as f64 * 10.0,
                4_410_000.0,
            )))),
            values: vec![("durum".into(), Value::Text((*d).into()))],
        })
        .collect();
    tx.insert("saha", &rows).expect("rows");
    tx.commit().expect("commit");
    path
}

/// A CBS project with the file's `saha` as a data layer; its id.
fn project(path: &std::path::Path) -> (Document, String) {
    let address = path.display().to_string();
    let source = kentos_data::registry()
        .source(&address)
        .expect("the source");
    let snapshot =
        kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
            name: "Saha".into(),
            srid: 5256,
            plot_scale: 1000.0,
            workspace: Workspace::Gis,
            drawing_font: kentos_commands::DrawingFont::Barlow,
            province: None,
            drawing_unit: None,
            annotation: None,
            cad_work: None,
            heights: None,
        })
        .expect("a project");
    let mut doc = Document::from_snapshot(snapshot).expect("a document");
    let d = data_describe::describe_in(source.as_ref(), &address, "saha").expect("described");
    let input = LayersData {
        operation: LayerDataOperation::Add,
        layer: None,
        name: Some("Saha varlıkları".into()),
        parent: None,
        index: None,
        data: Some(d.data.clone()),
        fields: Some(d.fields.clone()),
        expected_revision: None,
    };
    let layer = match kentos_native_application::layers_data::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ) {
        CommandResult::Completed { output, .. } => output.layer,
        other => panic!("not added: {other:?}"),
    };
    (doc, layer)
}

fn add(layer: &str, name: &str) -> LayersColumns {
    LayersColumns {
        action: LayerColumnsAction::Add,
        layer: layer.to_owned(),
        fields: Some(vec![LayerField::new(name, LayerFieldKind::Text)]),
        expected_revision: None,
    }
}

fn code<T: std::fmt::Debug>(r: &CommandResult<T>) -> String {
    match r {
        CommandResult::Failed { error } => error.code.clone(),
        other => format!("{other:?}"),
    }
}

#[test]
fn a_read_only_file_takes_no_column_and_stays_as_it_was() {
    kentos_data_gpkg::register();
    let path = field_assets("salt");
    let (mut doc, layer) = project(&path);
    let before = std::fs::read(&path).expect("the file");
    let mut perms = std::fs::metadata(&path).expect("meta").permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms.clone()).expect("read-only");
    kentos_data::registry().forget(&path.display().to_string());
    let refused = layers_columns::execute(
        &mut ExecutionContext::new(&mut doc),
        add(&layer, "saha_notu"),
    );
    assert_eq!(code(&refused), "columns_not_permitted");
    let message = match &refused {
        CommandResult::Failed { error } => error.message.clone(),
        _ => String::new(),
    };
    assert!(
        message.contains("“Saha varlıkları” katmanının tablosuna sütun eklenemez"),
        "{message}"
    );
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        before,
        "the file as it was"
    );
    assert_eq!(
        doc.layers().get(&layer).map(|n| n.fields.len()),
        Some(1),
        "the layer's fields as they were"
    );
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    let _ = std::fs::set_permissions(&path, perms);
}

#[test]
fn a_table_whose_file_is_gone_is_said() {
    kentos_data_gpkg::register();
    let path = field_assets("yok");
    let (mut doc, layer) = project(&path);
    let _ = std::fs::remove_dir_all(path.parent().expect("the folder"));
    kentos_data::registry().forget(&path.display().to_string());
    for action in [LayerColumnsAction::Add, LayerColumnsAction::Refresh] {
        let input = LayersColumns {
            action,
            ..add(&layer, "saha_notu")
        };
        let input = if action == LayerColumnsAction::Refresh {
            LayersColumns {
                fields: None,
                ..input
            }
        } else {
            input
        };
        let refused = layers_columns::execute(&mut ExecutionContext::new(&mut doc), input);
        assert_eq!(code(&refused), "source_unreachable", "{action:?}");
    }
}

#[test]
fn a_column_added_is_read_back_empty_and_stays_after_an_undo() {
    kentos_data_gpkg::register();
    let path = field_assets("geri");
    let (mut doc, layer) = project(&path);
    let done = layers_columns::execute(
        &mut ExecutionContext::new(&mut doc),
        add(&layer, "saha_notu"),
    );
    let CommandResult::Completed { output, .. } = done else {
        panic!("not added: {done:?}")
    };
    assert_eq!(
        (output.added, output.rows),
        (vec!["saha_notu".to_owned()], Some(2))
    );
    let source = kentos_data::registry()
        .source(&path.display().to_string())
        .expect("the source");
    let schema = source.schema("saha").expect("schema");
    assert!(
        schema.field("saha_notu").is_some_and(|f| f.nullable),
        "{schema:?}"
    );
    let read = kentos_data::driver::collect(
        source.as_ref(),
        "saha",
        &kentos_data::Request {
            columns: Some(vec!["saha_notu".into()]),
            ..kentos_data::Request::all()
        },
    )
    .expect("read");
    assert!(read.len() == 2 && read.iter().all(|f| f.values[0].is_null()));
    assert_eq!(doc.undo().as_deref(), Some("Kaynağa sütun ekle"));
    assert!(
        doc.layers()
            .get(&layer)
            .is_some_and(|n| n.fields.iter().all(|f| f.name != "saha_notu"))
    );
    assert!(
        source
            .schema("saha")
            .expect("schema")
            .field("saha_notu")
            .is_some(),
        "the column stays in the table"
    );
}
