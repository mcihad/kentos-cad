//! Write receipts (docs/adr/9126): a write's receipt goes in with its rows in one transaction, so a commit whose answer
//! was lost is asked afterwards. Through the lossy test source over a GeoPackage: the commit made and its answer lost,
//! the row is there once and the receipt gives its key; the connection ended before the commit, nothing and no
//! receipt; a source whose receipts were never set up gets none and the failure says it cannot be asked.

use kentos_commands::{CommandResult, LayerDataOperation, LayersData, LayersSession, SessionAction, Workspace};
use kentos_data::lossy::{self, Loss};
use kentos_data::{Coord, Driver, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};
use kentos_domain::Document;
use kentos_native_application::data_write::{self, Receipting, WriteFailure};
use kentos_native_application::{ExecutionContext, data_describe};

/// A GeoPackage with a `nokta` table of two points; its path.
fn points(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-makbuz-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("nokta.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(&path.display().to_string()).expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "nokta".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![Field::new("durum", FieldKind::Text { max: Some(20) })],
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
    let rows: Vec<NewFeature> = [(484_000.0, 4_420_000.0, "acik"), (484_010.0, 4_420_000.0, "izleme")]
        .iter()
        .map(|&(x, y, d)| NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(x, y)))),
            values: vec![("durum".into(), Value::Text(d.into()))],
        })
        .collect();
    tx.insert("nokta", &rows).expect("rows");
    tx.commit().expect("commit");
    path
}

/// The rows of `nokta` as a reader of its own counts them.
fn count(path: &std::path::Path) -> i64 {
    let gp = kentos_geopackage::GeoPackage::open(path, false).expect("the file");
    gp.connection().query_row("SELECT count(*) FROM nokta", [], |r| r.get(0)).expect("count")
}

/// A CBS project with the lossy source over the file's `nokta` as a data layer being edited, and a point drawn on it.
fn project_with_a_new_point(path: &std::path::Path) -> (Document, String) {
    let address = lossy::address_of(&path.display().to_string());
    let source = kentos_data::registry().source(&address).expect("the lossy source");
    let snapshot = kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
        name: "Kesinti".into(),
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
    let d = data_describe::describe_in(source.as_ref(), &address, "nokta").expect("described");
    let input = LayersData {
        operation: LayerDataOperation::Add,
        layer: None,
        name: Some("nokta".into()),
        parent: None,
        index: None,
        data: Some(d.data.clone()),
        fields: Some(d.fields.clone()),
        expected_revision: None,
    };
    let layer = match kentos_native_application::layers_data::execute(&mut ExecutionContext::new(&mut doc), input) {
        CommandResult::Completed { output, .. } => output.layer,
        other => panic!("not added: {other:?}"),
    };
    let started = kentos_native_application::layers_session::execute(
        &mut ExecutionContext::new(&mut doc),
        LayersSession { action: SessionAction::Start, layers: Some(vec![layer.clone()]), keys: None, force: None },
    );
    assert!(matches!(started, CommandResult::Completed { .. }), "{started:?}");
    let input = serde_json::from_value(serde_json::json!({
        "layerId": layer,
        "objects": [{ "geometry": { "kind": "point", "p": { "x": 484_020.0, "y": 4_420_000.0 } }, "attrs": { "durum": "yeni" } }]
    }))
    .expect("an input");
    let done = kentos_native_application::create::execute(&mut ExecutionContext::new(&mut doc), input);
    assert!(matches!(done, CommandResult::Completed { .. }), "{done:?}");
    (doc, layer)
}

fn receipt(id: &str) -> Receipting {
    Receipting { id: id.into(), by: Some("sınama".into()) }
}

#[test]
fn a_lost_answer_after_the_commit_is_found_by_its_receipt_and_one_before_it_wrote_nothing() {
    kentos_data_gpkg::register();
    lossy::register();
    let path = points("alindi");
    let (doc, layer) = project_with_a_new_point(&path);
    let writes = data_write::writes_of(&doc, &layer).expect("being edited");
    data_write::set_up_receipts(&writes.data).expect("set up");
    assert!(data_write::receipts_ready(&writes.data).expect("asked"));
    let new_uid = writes.changes.added[0].0;

    // The connection ends before the commit reaches the source: nothing written, no receipt.
    lossy::lose(&path.display().to_string(), Loss::BeforeCommit);
    match data_write::write_group_receipted(std::slice::from_ref(&writes), doc.settings(), false, Some(&receipt("y1"))) {
        Err(WriteFailure::Uncertain(words, Some(id))) => {
            assert_eq!(id, "y1");
            assert!(words.contains("sonucu bilinmiyor"), "{words}");
        }
        other => panic!("not uncertain: {other:?}"),
    }
    assert_eq!(count(&path), 2, "nothing written");
    assert_eq!(data_write::receipt_keys(&writes.data, "y1").expect("asked"), None, "no receipt: it did not commit");

    // The commit made and its answer lost: the row is there once, and the receipt gives its key by the object's id.
    lossy::lose(&path.display().to_string(), Loss::AfterCommit);
    match data_write::write_group_receipted(std::slice::from_ref(&writes), doc.settings(), false, Some(&receipt("y2"))) {
        Err(WriteFailure::Uncertain(_, Some(id))) => assert_eq!(id, "y2"),
        other => panic!("not uncertain: {other:?}"),
    }
    assert_eq!(count(&path), 3, "written once");
    let keys = data_write::receipt_keys(&writes.data, "y2").expect("asked").expect("its receipt");
    assert_eq!(keys.get("nokta").cloned(), Some(vec![(new_uid, 3)]));
}

#[test]
fn a_source_without_receipts_gives_no_id_to_ask_by() {
    kentos_data_gpkg::register();
    lossy::register();
    let path = points("makbuzsuz");
    let (doc, layer) = project_with_a_new_point(&path);
    let writes = data_write::writes_of(&doc, &layer).expect("being edited");
    assert!(!data_write::receipts_ready(&writes.data).expect("asked"), "never set up by itself");
    lossy::lose(&path.display().to_string(), Loss::AfterCommit);
    match data_write::write_group_receipted(std::slice::from_ref(&writes), doc.settings(), false, Some(&receipt("y3"))) {
        Err(WriteFailure::Uncertain(_, None)) => {}
        other => panic!("not uncertain without a receipt: {other:?}"),
    }
    assert_eq!(count(&path), 3);
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("the file");
    let made: i64 = gp
        .connection()
        .query_row("SELECT count(*) FROM sqlite_master WHERE name = 'kentos_writes'", [], |r| r.get(0))
        .expect("the catalog");
    assert_eq!(made, 0, "no table made in the owner's file without asking");
}
