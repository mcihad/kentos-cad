//! topology › source_tests's tests that need the CBS module's surfaces (its project's bottom tabs and panels; docs/adr/9199
//! §4.14): moved from the core's gis/topology/source_tests.rs with the module's package; the work they test is the core's.

use kentos_commands::{TopologyRule, TopologyRuleKind, TopologySettings};
use kentos_data::{Coord, Curve, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};
use kentos_native_application::data_write;
use crate::topology::Event;
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive, scratch};
use crate::gis;

const TOUCHING: [(f64, f64); 3] = [(486_500.0, 4_420_200.0), (486_600.0, 4_420_200.0), (486_800.0, 4_420_200.0)];

/// The parcels' lower left corners in EPSG:5256; each is 100 m a side.
const OVERLAPPING: [(f64, f64); 3] = [(486_500.0, 4_420_200.0), (486_580.0, 4_420_200.0), (486_800.0, 4_420_200.0)];

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
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
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
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described")])
        .expect("added");
    let id = crate::data_layers_tests::node(&app, "parseller").expect("the layer").id.clone();
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

#[test]
fn a_data_layers_overlap_is_found_in_its_source_and_its_rows_are_gone_to() {
    let path = parcels("topoloji-kaynak", OVERLAPPING);
    let (mut app, layer) = app_over(&path);
    check(&mut app);
    let c = app.gis.topology.checked.clone().expect("checked");
    assert_eq!(c.g.checked(), 3, "the three parcels read from the source");
    assert_eq!(c.g.sources_read.len(), 1);
    assert_eq!((c.g.sources_read[0].read, c.g.sources_read[0].checked), (3, 3));
    assert!(c.g.incomplete.is_empty(), "{:?}", c.g.incomplete);
    assert_eq!(c.findings.len(), 1, "one overlap");
    let f = &c.findings[0];
    let measure = f.measure.expect("its area");
    assert!((measure - 2000.0).abs() < 1e-6, "the shared strip: {measure} m²");
    let mut keys: Vec<i64> = f.objects.iter().filter_map(|&k| c.g.rows[k].clone()).map(|(_, key)| key).collect();
    keys.sort_unstable();
    assert_eq!(keys, [1, 2], "P1 and P2; P3 is no finding's");
    let (_, rows) = app.topology_seen().expect("the tab is open");
    assert!(rows[0].ends_with("| satır 1, satır 2 | 2000.00 m²"), "{rows:?}");
    // The finding pressed: its rows are the table's selection of the layer.
    let _ = app.update(Message::Gis(gis::Event::Topology(Event::Press(0))));
    let chosen: Vec<i64> = app.gis.features.data.selected.iter().copied().collect();
    assert_eq!(chosen, [1, 2]);
    assert_eq!(app.gis.features.data.selected_of_for_test(), Some(layer.as_str()));
    // A fix would change a row the drawing does not hold: refused with what to do.
    let _ = app.update(Message::Gis(gis::Event::Topology(Event::Fix("subtractFirst"))));
    let last = said(&app).last().cloned().unwrap_or_default();
    assert!(last.contains("“parseller” katmanının") && last.contains("satır çizimde değil"), "{last}");
}

#[test]
fn parcels_meeting_along_an_edge_are_all_checked_and_clean() {
    let path = parcels("topoloji-temas", TOUCHING);
    let (mut app, _) = app_over(&path);
    check(&mut app);
    let c = app.gis.topology.checked.clone().expect("checked");
    assert!(c.findings.is_empty());
    assert_eq!(c.g.checked(), 3);
    assert!(
        said(&app).iter().any(|t| t == "Bulgu yok: 1 kural, 3 nesne denetlendi (parseller: kaynaktan 3 satır)."),
        "{:#?}",
        said(&app)
    );
}

#[test]
fn the_same_parcels_drawn_in_the_project_give_the_same_finding() {
    let mut app = app_with_drawing();
    let layer = {
        let doc = app.document.as_mut().expect("a drawing");
        doc.model.add_layer_at(kentos_domain::NewLayer::layer("Parseller"), None, Some(0), Some("Katman ekle"), false).expect("a layer")
    };
    for (x, y) in OVERLAPPING {
        let p = |dx: f64, dy: f64| serde_json::json!({ "x": x + dx, "y": y + dy });
        let input = serde_json::from_value(serde_json::json!({
            "layerId": layer,
            "objects": [{ "geometry": { "kind": "polygon", "pts": [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)] } }]
        }))
        .expect("an input");
        let doc = app.document.as_mut().expect("a drawing");
        let done = kentos_native_application::create::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            input,
        );
        assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    }
    rule_on(&mut app, &layer);
    check(&mut app);
    let c = app.gis.topology.checked.clone().expect("checked");
    assert_eq!((c.g.checked(), c.findings.len()), (3, 1));
    assert!((c.findings[0].measure.expect("its area") - 2000.0).abs() < 1e-6);
}

#[test]
fn a_source_that_cannot_be_read_leaves_its_layer_unchecked() {
    let path = parcels("topoloji-kayip", OVERLAPPING);
    let (mut app, _) = app_over(&path);
    std::fs::remove_file(&path).expect("the file gone");
    kentos_data::registry().forget(&path.display().to_string());
    check(&mut app);
    let c = app.gis.topology.checked.clone().expect("checked");
    assert_eq!(c.g.incomplete.len(), 1, "{:?}", c.g.incomplete);
    assert!(c.g.incomplete[0].reason.starts_with("kaynağı okunamadı, katman denetlenemedi"), "{:?}", c.g.incomplete);
    assert!(!said(&app).iter().any(|t| t.starts_with("Bulgu yok:")), "never said clean: {:#?}", said(&app));
}

#[test]
fn a_row_taken_into_the_drawing_is_checked_as_the_drawing_has_it() {
    let path = parcels("topoloji-duzenleme", OVERLAPPING);
    let (mut app, layer) = app_over(&path);
    let _ = app.data_edits_event(crate::data_edits::Event::Start(layer.clone()));
    // P2 taken and moved 30 m east: it meets P1 along an edge no more, no overlap.
    let node = crate::data_layers_tests::node(&app, "parseller").expect("the layer").clone();
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let rows = data_write::rows_to_take(&node, &settings, &[2], &|| false).expect("read");
    let doc = app.document.as_mut().expect("a drawing");
    let uid = doc.model.take_data_rows(&layer, rows).expect("taken")[0].expect("P2");
    let slot = doc.model.slot_of(uid).expect("its object");
    let mut json = serde_json::to_value(doc.model.get(slot).expect("P2")).expect("its JSON");
    for p in json["pts"].as_array_mut().expect("a polygon's points") {
        p["x"] = serde_json::json!(p["x"].as_f64().expect("x") + 30.0);
    }
    let moved: kentos_commands::Entity = serde_json::from_value(json).expect("moved");
    assert!(doc.model.update(slot, moved));
    check(&mut app);
    let c = app.gis.topology.checked.clone().expect("checked");
    assert!(c.findings.is_empty(), "{:?}", c.findings.len());
    assert_eq!(c.g.checked(), 3, "P1 and P3 from the source, P2 from the drawing");
    assert_eq!(c.g.sources_read[0].held, 1);
}

/// The 14th item's last part (docs/adr/9171): the findings over a data layer go old when its file is written outside
/// the drawing (another program adds a parcel), are current again once checked, and go old when the layer is bound to
/// another file; a check not followed by a write stays current.
#[test]
fn a_data_layers_file_written_outside_or_bound_elsewhere_makes_the_findings_old() {
    use kentos_data::Driver as _;
    let path = parcels("topoloji-disaridan", OVERLAPPING);
    let (mut app, id) = app_over(&path);
    check(&mut app);
    assert!(app.topology_checked().is_some(), "{:?}", said(&app));
    assert!(!app.topology_stale(), "checked now");
    // Another program writes a fourth parcel into the file: nothing of the drawing changed, the findings are old.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let address = path.display().to_string();
    let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
    let mut tx = source.begin().expect("begin");
    tx.insert(
        "parseller",
        &[NewFeature { fid: None, geometry: Some(square(487_000.0, 4_420_200.0)), values: vec![("ad".into(), Value::Text("P4".into()))] }],
    )
    .expect("a row");
    tx.commit().expect("written");
    drop(source);
    assert!(app.topology_stale(), "written outside: old");
    assert_eq!(app.topology_sources_changed(), ["parseller"], "said as the source's change");
    // Checked again: current.
    check(&mut app);
    assert!(!app.topology_stale(), "checked again");
    assert!(app.topology_sources_changed().is_empty());
    // The layer bound to another file with the same table: old.
    let other = parcels("topoloji-baska-dosya", TOUCHING);
    app.relink_data_layers(&id, &other).expect("relinked");
    assert!(app.topology_stale(), "bound elsewhere: old");
}
