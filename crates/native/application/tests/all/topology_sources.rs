//! A topology check over a data layer's table (docs/adr/9124): the rows read whole from the source, the report's fixed
//! data (three 100 × 100 m parcels; P1 and P2 sharing a 20 × 100 m strip, P3 apart). On a GeoPackage always; on PostGIS
//! when `KENTOS_DATA_PG` names a database (`postgresql://user@host:port/db`, its password in `PGPASSWORD`), as the
//! driver's conformance suite does: the same finding, the same rows, the same area.

use kentos_commands::{
    CommandResult, LayerDataOperation, LayersData, TopologyRule, TopologyRuleKind,
    TopologySettings, Workspace,
};
use kentos_data::{
    Coord, Curve, Driver, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value,
};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, data_describe, topology_check};

const OVERLAPPING: [(f64, f64); 3] = [
    (486_500.0, 4_420_200.0),
    (486_580.0, 4_420_200.0),
    (486_800.0, 4_420_200.0),
];
const TOUCHING: [(f64, f64); 3] = [
    (486_500.0, 4_420_200.0),
    (486_600.0, 4_420_200.0),
    (486_800.0, 4_420_200.0),
];

fn square(x: f64, y: f64) -> Geometry {
    Geometry::Polygon(vec![Curve::Line(vec![
        Coord::xy(x, y),
        Coord::xy(x + 100.0, y),
        Coord::xy(x + 100.0, y + 100.0),
        Coord::xy(x, y + 100.0),
        Coord::xy(x, y),
    ])])
}

/// The parcels P1, P2, P3 written into `table` of `source` (keys 1, 2, 3 on a new table).
fn write_parcels(source: &dyn kentos_data::Source, table: &str, corners: [(f64, f64); 3]) {
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: table.into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![Field::new("ad", FieldKind::Text { max: Some(8) })],
            geometry: Some(GeometryField {
                name: "geom".into(),
                type_name: "POLYGON".into(),
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
    let rows: Vec<NewFeature> = corners
        .iter()
        .zip(["P1", "P2", "P3"])
        .map(|(&(x, y), name)| NewFeature {
            fid: None,
            geometry: Some(square(x, y)),
            values: vec![("ad".into(), Value::Text(name.into()))],
        })
        .collect();
    tx.insert(table, &rows).expect("rows");
    tx.commit().expect("commit");
}

/// A new CBS project in EPSG:5256 with `table` of the source at `address` as a data layer and “Çakışmamalı” on it.
fn project_over(source: &dyn kentos_data::Source, address: &str, table: &str) -> Document {
    let snapshot =
        kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
            name: "Parsel kalite denetimi".into(),
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
    let d = data_describe::describe_in(source, address, table).expect("described");
    let input = LayersData {
        operation: LayerDataOperation::Add,
        layer: None,
        name: Some("parseller".into()),
        parent: None,
        index: None,
        data: Some(d.data.clone()),
        fields: (!d.fields.is_empty()).then(|| d.fields.clone()),
        expected_revision: None,
    };
    let layer = match kentos_native_application::layers_data::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ) {
        CommandResult::Completed { output, .. } => output.layer,
        other => panic!("not added: {other:?}"),
    };
    let mut settings = doc.settings().clone();
    settings.topology = Some(TopologySettings {
        rules: vec![TopologyRule {
            id: "cakisma".into(),
            kind: TopologyRuleKind::MustNotOverlap,
            layer,
            other: None,
            value: None,
        }],
        ..TopologySettings::default()
    });
    doc.set_settings(settings);
    doc
}

/// The check's findings as (keys, area) and how many objects it looked at.
fn checked(doc: &Document) -> (Vec<(Vec<i64>, f64)>, usize) {
    let g = topology_check::gather(doc, None);
    assert!(g.incomplete.is_empty(), "{:?}", g.incomplete);
    let found = topology_check::run(&g);
    let findings = found
        .findings
        .iter()
        .map(|f| {
            let mut keys: Vec<i64> = f
                .objects
                .iter()
                .filter_map(|&k| g.rows[k].clone())
                .map(|(_, k)| k)
                .collect();
            keys.sort_unstable();
            (keys, f.measure.unwrap_or(f64::NAN))
        })
        .collect();
    (findings, g.checked())
}

fn the_report_holds(
    source: &dyn kentos_data::Source,
    address: &str,
    overlapping: &str,
    touching: &str,
) {
    write_parcels(source, overlapping, OVERLAPPING);
    write_parcels(source, touching, TOUCHING);
    let (findings, objects) = checked(&project_over(source, address, overlapping));
    assert_eq!(objects, 3, "the three parcels");
    assert_eq!(findings.len(), 1, "one overlap: {findings:?}");
    assert_eq!(findings[0].0, [1, 2], "P1 and P2; P3 is no finding's");
    assert!(
        (findings[0].1 - 2000.0).abs() < 1e-6,
        "the shared strip: {} m²",
        findings[0].1
    );
    let (findings, objects) = checked(&project_over(source, address, touching));
    assert_eq!(
        (findings.len(), objects),
        (0, 3),
        "parcels meeting along an edge: all checked, none overlapping"
    );
}

#[test]
fn a_geopackage_layers_table_is_checked_whole() {
    kentos_data_gpkg::register();
    let dir = std::env::temp_dir().join(format!("kentos-topoloji-kaynak-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("parseller.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let address = path.display().to_string();
    let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
    the_report_holds(source.as_ref(), &address, "parseller", "temas");
}

#[test]
fn a_postgis_layers_table_gives_the_same_finding() {
    let Some(address) = std::env::var("KENTOS_DATA_PG")
        .ok()
        .filter(|a| !a.is_empty())
    else {
        eprintln!("KENTOS_DATA_PG yok: PostGIS topoloji durumu atlandı");
        return;
    };
    kentos_data_postgis::register();
    let source = kentos_data_postgis::PostGis.open(&address).expect("open");
    let (a, b) = (
        format!("public.kentos_topoloji_{}", std::process::id()),
        format!("public.kentos_topoloji_temas_{}", std::process::id()),
    );
    the_report_holds(source.as_ref(), &address, &a, &b);
    let mut tx = source.begin().expect("begin");
    tx.drop_table(&a).expect("drop");
    tx.drop_table(&b).expect("drop");
    tx.commit().expect("commit");
}
