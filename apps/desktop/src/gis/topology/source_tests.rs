//! The Topoloji tab over a data layer (docs/adr/9124): the rules check the layer's whole table read from its source,
//! not only what the drawing holds. The report's fixed data: three 100 × 100 m parcels, P1 and P2 sharing a 20 × 100 m
//! strip (one overlap of 2 000 m²), P3 apart; against it, P2 touching P1 along an edge only (no finding, three checked).
//! The same parcels drawn on a layer of the project give the same finding; a source that cannot be read leaves its
//! layer unchecked; a row taken into the drawing is checked as the drawing has it.

use kentos_commands::{TopologyRule, TopologyRuleKind, TopologySettings};
use kentos_data::{
    Coord, Curve, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value,
};

use super::Event;
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive, scratch};
use crate::gis;

/// The parcels' lower left corners in EPSG:5256; each is 100 m a side.
const OVERLAPPING: [(f64, f64); 3] = [
    (486_500.0, 4_420_200.0),
    (486_580.0, 4_420_200.0),
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

/// A GeoPackage with a `parseller` table of the three parcels P1, P2, P3 (keys 1, 2, 3).
fn parcels(tag: &str, corners: [(f64, f64); 3]) -> std::path::PathBuf {
    use kentos_data::Driver as _;
    let path = scratch(tag).join("parseller.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg
        .open(path.to_str().expect("utf-8"))
        .expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "parseller".into(),
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
    tx.insert("parseller", &rows).expect("rows");
    tx.commit().expect("commit");
    path
}

/// A CBS drawing with the GeoPackage's table as a data layer and the rule “Çakışmamalı” on it; the layer's id.
fn app_over(path: &std::path::Path) -> (App, String) {
    crate::data_layers::register();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described"),
    ])
    .expect("added");
    let id = crate::data_layers_tests::node(&app, "parseller")
        .expect("the layer")
        .id
        .clone();
    rule_on(&mut app, &id);
    (app, id)
}

fn rule_on(app: &mut App, layer: &str) {
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.workspace = Some(kentos_commands::Workspace::Gis);
    settings.topology = Some(TopologySettings {
        rules: vec![TopologyRule {
            id: "cakisma".into(),
            kind: TopologyRuleKind::MustNotOverlap,
            layer: layer.to_owned(),
            other: None,
            value: None,
        }],
        ..TopologySettings::default()
    });
    doc.model.set_settings(settings);
}

fn check(app: &mut App) {
    let task = app.update(Message::Run("topology.check"));
    drive(app, task);
}

fn said(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

/// Pictures (docs/adr/9124): the Topoloji tab after a check of the data layer, its one finding chosen; light at
/// 1440×900 and dark at 1100×760.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib topology::source_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::files_testing::usage_shots("9124", "01-kaynakta-cakisma", || {
        let path = parcels("topoloji-resim", OVERLAPPING);
        let (mut app, _) = app_over(&path);
        check(&mut app);
        let _ = app.update(Message::Gis(gis::Event::Topology(Event::Press(0))));
        app
    });
}

/// On a real PostgreSQL (the usage report's 14th item, docs/adr/9182): a check over a database's table keeps its
/// counters of rows added, changed and deleted. Asked off the interface's thread they stay while nothing is written;
/// another program's fourth parcel moves them and the findings are old, said as the source's change; checked again,
/// current.
#[test]
fn on_postgresql_a_write_outside_makes_the_findings_old() {
    let Some(pg) = crate::pg_testing::pg("PostgreSQL'de dışarıdaki yazım") else {
        return;
    };
    crate::data_layers::register();
    let t = crate::pg_testing::table("topoloji");
    let source = kentos_data::registry().source(&pg).expect("the database");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: t.clone(),
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
        true,
    )
    .expect("table");
    tx.commit().expect("made");
    // The parcels written by another program in SQL, its statistics flushed at once (a server flushes them by itself
    // within seconds).
    let insert = |name: &str, (x, y): (f64, f64)| {
        let (x2, y2) = (x + 100.0, y + 100.0);
        format!(
            "INSERT INTO {t} (ad, geom) VALUES ('{name}', ST_GeomFromText('POLYGON(({x} {y}, {x2} {y}, {x2} {y2}, {x} {y2}, {x} {y}))', 5256))"
        )
    };
    let mut statements: Vec<String> = OVERLAPPING
        .iter()
        .zip(["P1", "P2", "P3"])
        .map(|(c, n)| insert(n, *c))
        .collect();
    statements.push("SELECT pg_stat_force_next_flush()".into());
    crate::pg_testing::sql(
        &pg,
        &statements.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut app = app_with_drawing();
    let described = crate::data_layers::describe(&pg, &t).expect("described");
    let name = described.name.clone();
    app.add_data_layers(vec![described]).expect("added");
    let id = crate::data_layers_tests::node(&app, &name)
        .expect("the layer")
        .id
        .clone();
    rule_on(&mut app, &id);
    check(&mut app);
    let c = app.topology_checked().expect("checked");
    assert_eq!(
        c.findings.len(),
        1,
        "the overlap found in the database's table: {:?}",
        said(&app)
    );
    assert!(
        c.g.sources_read[0].fingerprint.is_some(),
        "its counters kept"
    );
    app.topology_ask_marks();
    assert!(!app.topology_stale(), "nothing written: current");
    crate::pg_testing::sql(
        &pg,
        &[
            &insert("P4", (487_000.0, 4_420_200.0)),
            "SELECT pg_stat_force_next_flush()",
        ],
    );
    app.topology_ask_marks();
    assert!(app.topology_stale(), "written outside: old");
    assert_eq!(
        app.topology_sources_changed(),
        std::slice::from_ref(&name),
        "said as the source's change"
    );
    check(&mut app);
    app.topology_ask_marks();
    assert!(!app.topology_stale(), "checked again: current");
    assert!(app.topology_sources_changed().is_empty());
    crate::pg_testing::drop_table(&pg, &t);
}

/// The picture (`.run/gui-evidence/kullanim-9171/`): the Topoloji tab after the data layer's file was written outside.
#[test]
#[ignore = "pictures for the owner"]
fn outside_screens() {
    use kentos_data::Driver as _;
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9171", "01-disaridan-degisti", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = parcels(&format!("topoloji-disaridan-resim-{n}"), OVERLAPPING);
        let (mut app, _) = app_over(&path);
        check(&mut app);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let source = kentos_data_gpkg::Gpkg
            .open(&path.display().to_string())
            .expect("open");
        let mut tx = source.begin().expect("begin");
        tx.insert(
            "parseller",
            &[NewFeature {
                fid: None,
                geometry: Some(square(487_000.0, 4_420_200.0)),
                values: vec![("ad".into(), Value::Text("P4".into()))],
            }],
        )
        .expect("a row");
        tx.commit().expect("written");
        app.command_expanded = true;
        app.bottom_tab = crate::bottom::BottomTab::Topology;
        app
    });
}
