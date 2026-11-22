//! The municipal report's recovery row over GeoPackage (docs/adr/9155): the report's file with its two tables (bakim,
//! mahalleler) bound as two data layers is moved; pointing one layer at the new place moves both. A package whose
//! same-named table differs (another key, no field the layer has) is refused with each difference, nothing bound.

use crate::app::App;
use crate::data_layers_tests::{municipal, node};
use kentos_data::{Driver as _, Field, FieldKind, GeometryField, NewTable};
use crate::files_testing::{app_with_drawing, scratch};

/// Another package with a table named bakim of other columns: an integer `no` and a text `ad`, points in 4326.
fn other_bakim() -> std::path::PathBuf {
    let path = scratch("baska-paket").join("baska.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "bakim".into(),
            title: None,
            description: None,
            key: "no".into(),
            fields: vec![Field::new("ad", FieldKind::Text { max: Some(20) })],
            geometry: Some(GeometryField { name: "geom".into(), type_name: "POINT".into(), z: false, m: false, srid: Some(4326) }),
            spatial_index: false,
            system: None,
        },
        false,
    )
    .expect("table");
    tx.commit().expect("commit");
    path
}

fn address_of(app: &App, name: &str) -> String {
    node(app, name).and_then(|n| n.data.as_ref()).map(|d| d.address.clone()).expect("a data layer")
}

#[test]
fn a_moved_file_is_pointed_at_once_and_a_different_package_is_refused() {
    crate::data_layers::register();
    let path = municipal("tasinan");
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&address, "bakim").expect("described"),
        crate::data_layers::describe(&address, "mahalleler").expect("described"),
    ])
    .expect("added");
    // Moved to another folder (another computer).
    let moved = scratch("tasinan-yeni").join("kurum-verisi.gpkg");
    std::fs::copy(&path, &moved).expect("copied");
    std::fs::remove_file(&path).expect("removed");
    // A package with a same-named table that is not the layer's: refused, nothing bound.
    let other = other_bakim();
    let bakim = node(&app, "bakim").expect("bakim").id.clone();
    let refused = app.relink_data_layers(&bakim, &other).expect_err("not the layer's table");
    assert!(refused.contains("“bakim” tablosu “bakim” katmanının tablosu değil"), "{refused}");
    assert!(refused.contains("Hiçbir katman bağlanmadı"), "{refused}");
    assert_eq!(address_of(&app, "bakim"), address, "nothing bound");
    // The new place: both layers of the file follow.
    let done = app.relink_data_layers(&bakim, &moved).expect("relinked");
    assert_eq!(done.len(), 2, "{done:?}");
    assert_eq!(address_of(&app, "bakim"), moved.display().to_string());
    assert_eq!(address_of(&app, "mahalleler"), moved.display().to_string());
    // Undone as one step each: the drawing's undo goes back.
    assert!(app.document.as_ref().expect("a drawing").model.can_undo());
    let _ = std::fs::remove_file(&moved);
    let _ = std::fs::remove_file(&other);
}

/// The picture (`.run/gui-evidence/kullanim-9155/`): Kaynak durumu with the moved file's two data layers not read, each
/// with Yeniden bağla….
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9155", "01-yeniden-bagla", || {
        crate::data_layers::register();
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = municipal(&format!("tasinan-resim-{n}"));
        let address = path.display().to_string();
        let mut app = app_with_drawing();
        app.add_data_layers(vec![
            crate::data_layers::describe(&address, "bakim").expect("described"),
            crate::data_layers::describe(&address, "mahalleler").expect("described"),
        ])
        .expect("added");
        std::fs::remove_file(&path).expect("moved away");
        kentos_data::registry().forget(&address);
        let task = app.update(crate::app::Message::Run("source.status"));
        crate::files_testing::drive(&mut app, task);
        app
    });
}
