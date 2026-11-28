//! The layer drawn on (docs/adr/9128): in a CBS project one press on a layer row makes it the target (a CAD drawing
//! keeps the double click); a data layer is drawn on only in its edit session and only with its table's geometry,
//! the drawing tools saying why not and offering Düzenlemeyi başlat. On a real PostgreSQL (`KENTOS_DATA_PG`) the same,
//! a binding through an account that may only read refused by the server's own right.

use crate::app::{App, Message};
use crate::data_layers_tests::{node, parcels};
use crate::files_testing::app_with_drawing;

fn active(app: &App) -> String {
    app.document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .unwrap_or_default()
}

/// A drawing with a GeoPackage's parcels (MULTIPOLYGON) as a data layer; `gis` makes it a CBS project.
fn app_with_parcels(tag: &str, gis: bool) -> (App, String) {
    crate::data_layers::register();
    let path = parcels(tag, 3);
    let mut app = app_with_drawing();
    if gis {
        let _ = app.update(Message::Run("workspace.gis"));
    } else {
        crate::files_testing::make_cad(&mut app);
    }
    app.add_data_layers(vec![
        crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described"),
    ])
    .expect("added");
    let id = node(&app, "parseller").expect("the layer").id.clone();
    (app, id)
}

#[test]
fn one_press_on_a_layer_makes_it_the_target_in_a_cbs_project() {
    let (mut app, parcels) = app_with_parcels("hedef-cbs", true);
    let other = app
        .document
        .as_ref()
        .and_then(|d| {
            d.model
                .layers()
                .leaves()
                .into_iter()
                .find(|n| n.id != parcels && n.data.is_none())
                .map(|n| n.id.clone())
        })
        .expect("another layer");
    app.layer_pressed(other.clone());
    assert_eq!(active(&app), other);
    app.layer_pressed(parcels.clone());
    assert_eq!(active(&app), parcels, "one press");
}

#[test]
fn a_cad_drawing_keeps_the_double_click() {
    let (mut app, parcels) = app_with_parcels("hedef-cad", false);
    let before = active(&app);
    app.layer_pressed(parcels.clone());
    assert_eq!(active(&app), before, "one press only chooses the row");
    app.layer_pressed(parcels.clone());
    assert_eq!(
        active(&app),
        parcels,
        "the double click makes it the target"
    );
}

#[test]
fn a_data_layer_is_drawn_on_only_in_its_session_and_with_its_geometry() {
    let (mut app, parcels) = app_with_parcels("hedef-oturum", true);
    app.layer_pressed(parcels.clone());
    // The session closed: the area tool is off, says why, and offers to open it.
    assert!(!app.available("tool.polygon"));
    let why = app.why_disabled("tool.polygon").expect("why");
    assert!(
        why.contains("düzenlemeye kapalı") && why.contains("Düzenlemeyi başlat"),
        "{why}"
    );
    assert_eq!(app.closed_data_layer().as_deref(), Some(parcels.as_str()));
    let _ = app.update(Message::Run("tool.polygon"));
    let offered = app.toasts.iter().any(|(_, t, _)| {
        t.action_of()
            .is_some_and(|(label, _)| label == "Düzenlemeyi başlat")
    });
    assert!(offered, "the notice offers Düzenlemeyi başlat");
    // Opened: the area tool draws; the point tool does not fit an area table.
    let _ = app.data_edits_event(crate::data_edits::Event::Start(parcels.clone()));
    assert!(
        app.available("tool.polygon"),
        "{:?}",
        app.why_disabled("tool.polygon")
    );
    assert!(!app.available("tool.point"));
    let why = app.why_disabled("tool.point").expect("why");
    assert!(
        why.contains("alan katmanı") && why.contains("nokta"),
        "{why}"
    );
}

/// Two edit sessions open (docs/adr/9169, the 4th item's last check): Kapalı alan draws on the target alone, the
/// other session's layer and the drawing's own layers untouched; the target changed, the next area goes to the new
/// one; nothing falls to another layer quietly.
#[test]
fn with_two_sessions_open_an_area_goes_to_the_target_alone() {
    crate::data_layers::register();
    let first = parcels("iki-oturum-a", 3).display().to_string();
    let second = parcels("iki-oturum-b", 3).display().to_string();
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("workspace.gis"));
    let ids = app
        .add_data_layers(vec![
            crate::data_layers::describe(&first, "parseller").expect("described"),
            crate::data_layers::describe(&second, "parseller").expect("described"),
        ])
        .expect("added");
    let (a, b) = (ids[0].clone(), ids[1].clone());
    for id in [&a, &b] {
        let _ = app.data_edits_event(crate::data_edits::Event::Start(id.clone()));
    }
    let others = |app: &App| -> usize {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities()
            .filter(|e| e.base().layer_id != a && e.base().layer_id != b)
            .count()
    };
    let added = |app: &App, id: &str| {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .data_changes(id)
            .added
            .len()
    };
    let before = others(&app);
    // An area of three typed corners, Enter closing it.
    let area = |app: &mut App, x: f64| {
        let _ = app.update(Message::Run("tool.polygon"));
        for (dx, dy) in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)] {
            let _ = app.submit_line(&format!("{},{}", x + dx, 4_420_300.0 + dy));
        }
        let _ = app.submit_line("");
    };
    app.layer_pressed(a.clone());
    assert_eq!(active(&app), a);
    area(&mut app, 486_600.0);
    assert_eq!(
        (added(&app, &a), added(&app, &b)),
        (1, 0),
        "the target's alone"
    );
    app.layer_pressed(b.clone());
    assert_eq!(active(&app), b);
    area(&mut app, 486_700.0);
    assert_eq!(
        (added(&app, &a), added(&app, &b)),
        (1, 1),
        "the new target's"
    );
    assert_eq!(others(&app), before, "no other layer took one");
}

/// On real PostgreSQL tables (the usage report's 4th item): a database's layer is drawn on only in its session and with
/// its table's geometry; a binding through an account that may only read is read-only by the server's right and its
/// session does not open; with two sessions open an area goes to the target alone, and each table takes its own row.
#[test]
fn on_postgresql_a_layer_is_drawn_on_only_in_its_session_and_two_sessions_keep_their_targets() {
    let Some(pg) = crate::pg_testing::pg("PostgreSQL'de çizim hedefi") else {
        return;
    };
    crate::data_layers::register();
    let (ta, tb) = (
        crate::pg_testing::table("hedef_a"),
        crate::pg_testing::table("hedef_b"),
    );
    let owner = kentos_data::registry().source(&pg).expect("the database");
    crate::data_layers_tests::parcels_in(owner.as_ref(), &ta, 3);
    crate::data_layers_tests::parcels_in(owner.as_ref(), &tb, 3);
    let name = format!("kentos_hedef_okur_{}", std::process::id());
    let reader = crate::pg_testing::account(&pg, &name, &ta, "SELECT");
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("workspace.gis"));
    let ids = app
        .add_data_layers(vec![
            crate::data_layers::describe(&pg, &ta).expect("described"),
            crate::data_layers::describe(&pg, &tb).expect("described"),
            crate::data_layers::describe(&reader, &ta).expect("described"),
        ])
        .expect("added");
    let (a, b, r) = (ids[0].clone(), ids[1].clone(), ids[2].clone());
    // The session closed: Kapalı alan is off and says why.
    app.layer_pressed(a.clone());
    assert!(!app.available("tool.polygon"));
    let why = app.why_disabled("tool.polygon").expect("why");
    assert!(
        why.contains("düzenlemeye kapalı") && why.contains("Düzenlemeyi başlat"),
        "{why}"
    );
    // The reader's binding: read-only by the server's right, its session not opened.
    let read_only = |app: &App, id: &str| {
        app.document
            .as_ref()
            .and_then(|d| d.model.layers().get(id))
            .and_then(|n| n.data.as_ref())
            .is_some_and(|d| d.read_only)
    };
    assert!(read_only(&app, &r) && !read_only(&app, &a));
    let _ = app.data_edits_event(crate::data_edits::Event::Start(r.clone()));
    assert!(
        !app.document.as_ref().expect("a drawing").model.editing(&r),
        "a read-only binding is not edited"
    );
    // Two sessions: an area to each target alone; each table takes its own row.
    for id in [&a, &b] {
        let _ = app.data_edits_event(crate::data_edits::Event::Start(id.clone()));
    }
    assert!(!app.available("tool.point"), "an area table takes no point");
    let added = |app: &App, id: &str| {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .data_changes(id)
            .added
            .len()
    };
    let area = |app: &mut App, x: f64| {
        let _ = app.update(Message::Run("tool.polygon"));
        for (dx, dy) in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)] {
            let _ = app.submit_line(&format!("{},{}", x + dx, 4_420_300.0 + dy));
        }
        let _ = app.submit_line("");
    };
    app.layer_pressed(a.clone());
    area(&mut app, 486_600.0);
    app.layer_pressed(b.clone());
    area(&mut app, 486_700.0);
    assert_eq!((added(&app, &a), added(&app, &b)), (1, 1));
    let task = app.data_edits_event(crate::data_edits::Event::Save(vec![a.clone(), b.clone()]));
    crate::files_testing::drive(&mut app, task);
    assert_eq!(
        (
            crate::pg_testing::rows(&pg, &ta),
            crate::pg_testing::rows(&pg, &tb)
        ),
        (4, 4),
        "each table its own row"
    );
    crate::pg_testing::drop_table(&pg, &ta);
    crate::pg_testing::drop_table(&pg, &tb);
    crate::pg_testing::sql(
        &pg,
        &[
            &format!("DROP OWNED BY {name}"),
            &format!("DROP ROLE {name}"),
        ],
    );
}
