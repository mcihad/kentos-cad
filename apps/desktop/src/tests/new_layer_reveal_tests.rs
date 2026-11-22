//! GIS-004: a new typed layer made from + › Nokta katmanı in a long layer tree is chosen and scrolled into view, so
//! its row takes a right click at once; a search that leaves it out stays and the message says so. In a CBS project
//! the layer is a new table of a GeoPackage (docs/adr/9107 §7): the window asks where, Oluştur makes it. With
//! `KENTOS_SHOTS=1` the picture goes to `.run/gui-evidence/gis-004/`. Test code only.

use serde_json::json;

use crate::app::{App, Message};
use crate::layer_create::Event as Create;
use crate::tools_scenes::{Objects, layer, open_edited};

/// + › Nokta katmanı in a CBS project: the window, its table in a GeoPackage of the test's own, Oluştur.
fn new_point_layer(app: &mut App, name: &str) -> std::path::PathBuf {
    crate::data_layers::register();
    let gpkg = crate::files_testing::scratch(name).join("katmanlar.gpkg");
    let _ = app.update(Message::Run("layer.newPoint"));
    assert!(app.gis.layer_create.is_some(), "the window asks where its table goes");
    let send = |app: &mut App, e: Create| drop(app.update(Message::Gis(crate::gis::Event::LayerCreate(e))));
    send(app, Create::Path(gpkg.display().to_string()));
    send(app, Create::Run);
    assert!(app.gis.layer_create.is_none(), "made: {}", crate::files_testing::last_said(app));
    gpkg
}

/// A CBS drawing of forty layers, the first one active: the tree is longer than the panel.
fn long_tree(app: &mut App) {
    let mut o = Objects::new();
    o.point("l00", [0.0, 0.0], None);
    open_edited(app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        d["layers"] = json!(
            (0..40)
                .map(|i| layer(&format!("l{i:02}"), &format!("Katman {i:02}"), "#3E63DD", 0.25))
                .collect::<Vec<_>>()
        );
        d["activeLayer"] = json!("l00");
    });
}

#[test]
fn a_new_point_layer_in_a_long_tree_is_chosen_and_in_view() {
    let (mut app, _) = App::boot(None);
    long_tree(&mut app);
    let mut shot = crate::files_testing::offscreen(iced::Size::new(1440., 900.));
    let mut update = |app: &mut App, m| drop(app.update(m));
    shot.settle(&mut app, App::view, &mut update);
    // The tree scrolled to its end first: the active first layer is out of view, as in the report.
    let gpkg = new_point_layer(&mut app, "gis-004-yeni");
    shot.settle(&mut app, App::view, &mut update);
    let doc = &app.document.as_ref().expect("open").model;
    let made = doc.layers().active().to_owned();
    assert_eq!(doc.layers().get(&made).map(|n| n.name.as_str()), Some("Yeni nokta katmanı"));
    // A table of the GeoPackage, not a layer of the project file.
    let data = doc.layers().get(&made).and_then(|n| n.data.clone()).expect("a data layer");
    assert_eq!((data.address.as_str(), data.table.as_str()), (gpkg.display().to_string().as_str(), "yeni_nokta_katmani"));
    assert_eq!(app.selected_layer.as_deref(), Some(made.as_str()));
    assert_eq!(app.layer_reveal.as_deref(), Some(made.as_str()));
    assert!(
        crate::inspector_screens::text_at(&mut shot, &app, "Yeni nokta katmanı").is_some(),
        "the new row is drawn in the panel"
    );
    crate::files_testing::evidence(&mut shot, &app, "gis-004", "yeni-nokta-katmani");
}

#[test]
fn a_search_that_leaves_the_new_layer_out_stays_and_says_so() {
    let (mut app, _) = App::boot(None);
    long_tree(&mut app);
    app.layer_query = "Katman 3".into();
    let _ = new_point_layer(&mut app, "gis-004-arama");
    assert_eq!(app.layer_query, "Katman 3", "the search is the user's");
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("Katman araması onu gizliyor"), "{said}");
}
