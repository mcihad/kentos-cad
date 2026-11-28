//! A data layer's rows asked with a condition naming a field the layer lacks (docs/adr/9132): the reader says which
//! field, on which layer, and the fields it has, instead of answering “no row” as if the data said so. The municipal
//! report's case: `bedel >= 100`, written on `bakim`, asked of `mahalleler` (mahalle, ad).

use kentos_commands::{CommandResult, LayerDataOperation, LayersData, LayersRows, Workspace};
use kentos_data::{
    Coord, Driver, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value,
};
use kentos_domain::Document;
use kentos_native_application::layers_rows::{self, missing_fields};
use kentos_native_application::{ExecutionContext, data_describe};

/// A GeoPackage with `mahalleler`: M1 Çınar, M2 İstasyon, M3 Kale.
fn neighbourhoods() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-alan-yok-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("Mahalle-Verileri.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg
        .open(&path.display().to_string())
        .expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "mahalleler".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![
                Field::new("mahalle", FieldKind::Text { max: Some(4) }),
                Field::new("ad", FieldKind::Text { max: Some(32) }),
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
    let rows: Vec<NewFeature> = [("M1", "Çınar"), ("M2", "İstasyon"), ("M3", "Kale")]
        .iter()
        .enumerate()
        .map(|(i, &(m, ad))| NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(
                484_000.0 + i as f64 * 10.0,
                4_420_000.0,
            )))),
            values: vec![
                ("mahalle".into(), Value::Text(m.into())),
                ("ad".into(), Value::Text(ad.into())),
            ],
        })
        .collect();
    tx.insert("mahalleler", &rows).expect("rows");
    tx.commit().expect("commit");
    path
}

/// A CBS project with the file's `mahalleler` as a data layer; its id.
fn project(path: &std::path::Path) -> (Document, String) {
    let address = path.display().to_string();
    let source = kentos_data::registry()
        .source(&address)
        .expect("the source");
    let snapshot =
        kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
            name: "Mahalleler".into(),
            srid: 5255,
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
    let d = data_describe::describe_in(source.as_ref(), &address, "mahalleler").expect("described");
    let input = LayersData {
        operation: LayerDataOperation::Add,
        layer: None,
        name: Some("mahalleler".into()),
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

#[test]
fn a_condition_naming_a_field_the_layer_lacks_is_said_not_answered_empty() {
    kentos_data_gpkg::register();
    let path = neighbourhoods();
    let (doc, layer) = project(&path);
    let node = doc.layers().get(&layer).expect("the layer").clone();
    let ask = |filter: &str| {
        layers_rows::read(
            &node,
            doc.settings(),
            &LayersRows {
                layer: layer.clone(),
                filter: Some(filter.into()),
                count: Some(true),
                ..LayersRows::default()
            },
            &|| false,
        )
    };
    let refused = ask("bedel >= 100").expect_err("refused");
    assert_eq!(refused.code, "field_not_found");
    assert_eq!(
        refused.message,
        "Süzgeç uygulanmadı: “bedel” alanı “mahalleler” katmanında yok. Katmanın alanları: mahalle, ad."
    );
    // Its own fields are asked: a true no-row answer is still one.
    let read = ask("ad = 'Kale'").expect("read");
    assert_eq!(read.total, Some(1));
    let none = ask("mahalle = 'M9'").expect("read");
    assert_eq!(none.total, Some(0));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_words_name_every_missing_field_and_a_wide_tables_first_fields() {
    let named = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        missing_fields(&named(&["ad"]), &named(&["mahalle", "ad"]), "mahalleler"),
        None
    );
    assert_eq!(
        missing_fields(
            &named(&["bedel", "ad", "durum"]),
            &named(&["mahalle", "ad"]),
            "mahalleler"
        )
        .as_deref(),
        Some(
            "Süzgeç uygulanmadı: “bedel”, “durum” alanları “mahalleler” katmanında yok. Katmanın alanları: mahalle, ad."
        )
    );
    assert_eq!(
        missing_fields(&named(&["x"]), &[], "boş").as_deref(),
        Some("Süzgeç uygulanmadı: “x” alanı “boş” katmanında yok. Katmanın alanı yok.")
    );
    let wide: Vec<String> = (1..=14).map(|i| format!("a{i}")).collect();
    let words = missing_fields(&named(&["x"]), &wide, "geniş").expect("said");
    assert!(
        words.ends_with(
            "Katmanın alanları: a1, a2, a3, a4, a5, a6, a7, a8, a9, a10 ve 4 alan daha."
        ),
        "{words}"
    );
}
