//! Noktalar's new row in a CBS project (docs/adr/9320) with the module's surfaces: its bottom tab and, above the table,
//! the row's layer and what holds it back. The work is the core's (gis/points/target.rs), its rules tested there.

use kentos_data::Driver;
use serde_json::json;

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::layer_create::Event as Create;
use crate::points::{Event, Walk};

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(crate::gis::Event::Points(e)));
    crate::files_testing::drive(app, task);
}

/// A new CBS project: its one layer Açıklama, active.
fn new_cbs() -> App {
    crate::data_layers::register();
    let (mut app, _) = App::boot(None);
    let mut d: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/interaction/v1/empty.kcad")).expect("reads");
    d["settings"]["workspace"] = json!("gis");
    d["layers"] = json!([{
        "id": "aciklama", "name": "Açıklama", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "geometry": "annotation", "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
        "children": []
    }]);
    d["activeLayer"] = json!("aciklama");
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&d.to_string()).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// A table of points `noktalar` (ad, kod; no Z) in a GeoPackage of the test's own, added as a data layer.
fn with_point_table(app: &mut App, tag: &str) {
    let path = crate::files_testing::scratch(tag).join("noktalar.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let address = path.display().to_string();
    let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
    let mut tx = source.begin().expect("begin");
    let text = || kentos_data::FieldKind::Text { max: None };
    tx.create_table(
        &kentos_data::NewTable {
            name: "noktalar".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![kentos_data::Field::new("ad", text()), kentos_data::Field::new("kod", text())],
            geometry: Some(kentos_data::GeometryField {
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
    tx.commit().expect("commit");
    let described = crate::data_layers::describe(&address, "noktalar").expect("described");
    app.add_data_layers(vec![described]).expect("added");
}

/// Noktalar opened, a row added and typed from Ad to Kod.
fn typed(app: &mut App) {
    let _ = app.update(Message::Run("point.editor"));
    ev(app, Event::AddRow);
    for (i, v) in ["QA-A", "500000", "4320000", "12.5", "QA"].iter().enumerate() {
        ev(app, Event::Input((*v).into()));
        if i < 4 {
            ev(app, Event::Finish(Some(Walk::Right)));
        }
    }
}

#[test]
fn a_rows_warning_leaves_noktalar_on_screen_in_a_cbs_project() {
    let mut app = new_cbs();
    typed(&mut app);
    assert_eq!(app.bottom_tab, BottomTab::Points, "the tab is the module's and stays");
    assert!(app.command_expanded);
}

/// The pictures (`.run/gui-evidence/kullanim-9320/`): a new CBS project's row with no layer to take it, the window
/// for its table, the row written to the table made, a data layer closed for editing.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop-cbs points_target_tab_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    let roomy = |app: &mut App| drop(app.update(Message::BottomResized(260.0)));
    crate::files_testing::usage_shots("9320", "01-katman-yok", || {
        let mut app = new_cbs();
        typed(&mut app);
        roomy(&mut app);
        app
    });
    crate::files_testing::usage_shots("9320", "02-nokta-tablosu-penceresi", || {
        let mut app = new_cbs();
        typed(&mut app);
        roomy(&mut app);
        ev(&mut app, Event::CreateLayer);
        let gpkg = crate::files_testing::scratch("resim-nokta").join("noktalar.gpkg");
        let _ = app.update(Message::Gis(crate::gis::Event::LayerCreate(Create::Path(gpkg.display().to_string()))));
        app
    });
    crate::files_testing::usage_shots("9320", "03-satir-tabloya-yazildi", || {
        let mut app = new_cbs();
        typed(&mut app);
        roomy(&mut app);
        ev(&mut app, Event::CreateLayer);
        let gpkg = crate::files_testing::scratch("resim-yazildi").join("noktalar.gpkg");
        let _ = app.update(Message::Gis(crate::gis::Event::LayerCreate(Create::Path(gpkg.display().to_string()))));
        let task = app.update(Message::Gis(crate::gis::Event::LayerCreate(Create::Run)));
        crate::files_testing::drive(&mut app, task);
        ev(&mut app, Event::Finish(Some(Walk::Down)));
        app
    });
    crate::files_testing::usage_shots("9320", "04-duzenlemeye-kapali", || {
        let mut app = new_cbs();
        with_point_table(&mut app, "resim-kapali");
        typed(&mut app);
        roomy(&mut app);
        app
    });
}
