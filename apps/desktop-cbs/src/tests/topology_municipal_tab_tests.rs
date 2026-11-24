//! topology › municipal_tests's tests that need the CBS module's surfaces (its project's bottom tabs and panels; docs/adr/9199
//! §4.14): moved from the core's gis/topology/municipal_tests.rs with the module's package; the work they test is the core's.

use crate::app::{App, Message};
use crate::gis;
use kentos_domain::Slot;
use crate::topology::Event;

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/topology-rules.kcad");

/// The drawing in a 1000 × 700 view, checked.
fn checked() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    let _ = app.update(Message::Run("topology.check"));
    app
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Topology(e)));
}

fn rows(app: &App) -> Vec<String> {
    app.topology_seen().expect("the tab").1
}

/// The row shown of the finding of `rule` at (`x`, `y`).
fn row_at(app: &App, rule: &str, x: f64, y: f64) -> usize {
    let c = app.topology_checked().expect("checked");
    app.topology_shown()
        .iter()
        .position(|&i| {
            let f = &c.findings[i];
            c.g.rules[f.rule].id == rule && (f.at.x - x).abs() < 1e-6 && (f.at.y - y).abs() < 1e-6
        })
        .unwrap_or_else(|| panic!("no {rule} finding at {x}, {y}"))
}

fn run(app: &mut App, id: &str, input: serde_json::Value) {
    let doc = &mut app.document.as_mut().expect("open").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        id,
        kentos_commands::CallOp::Execute,
        input,
    );
    assert!(answer.is_ok(), "{id}: {answer:?}");
}

#[test]
fn every_finding_is_gone_to_an_exception_kept_a_fix_undone_and_checked_again() {
    let mut app = checked();
    assert_eq!(app.topology_seen().expect("the tab").0, "19 bulgu (açık 19, istisna 0)");
    let all = rows(&app);
    assert!(all.contains(&"Parsel | Çakışmamalı | Çakışma | #2, #3 | 12.00 m²".to_owned()), "{all:#?}");
    assert!(all.contains(&"Yol | Sarkan uç olmamalı | Sarkan uç | #18 | 0.600 m".to_owned()), "{all:#?}");
    // Every finding gone to: its row chosen, the view on its place.
    for i in 0..app.topology_shown().len() {
        ev(&mut app, Event::Press(i));
        let c = app.topology_checked().expect("checked");
        let f = &c.findings[app.gis.topology.selected[0]];
        let b = app.viewport.camera.visible_bounds();
        let (x, y) = ((f.bounds.min_x + f.bounds.max_x) / 2.0, (f.bounds.min_y + f.bounds.max_y) / 2.0);
        assert!(x >= b.min_x && x <= b.max_x && y >= b.min_y && y <= b.max_y, "row {i}: ({x}, {y}) out of {b:?}");
    }
    // Road #16's west end, at the sheet's edge: an exception with its reason.
    let edge = row_at(&app, "kural-6", 486_990.0, 4_419_992.0);
    ev(&mut app, Event::Press(edge));
    ev(&mut app, Event::Reason("Pafta kenarı: yol paftanın dışında sürüyor".into()));
    ev(&mut app, Event::Exception(true));
    let settings = app.document.as_ref().and_then(|d| d.model.settings().topology.clone()).expect("settings");
    assert_eq!(settings.exceptions.len(), 1);
    assert_eq!(settings.exceptions[0].reason.as_deref(), Some("Pafta kenarı: yol paftanın dışında sürüyor"));
    assert_eq!(app.topology_seen().expect("the tab").0, "18 bulgu (açık 18, istisna 1)");
    // The overlap fixed in one step and checked again; undone, the findings are old; checked again, it is back.
    ev(&mut app, Event::Press(0));
    ev(&mut app, Event::Fix("subtractFirst"));
    assert!(!rows(&app).iter().any(|r| r.starts_with("Parsel | Çakışmamalı | Çakışma | #2, #3")), "{:#?}", rows(&app));
    assert!(!app.topology_stale(), "the fix checked again");
    let _ = app.update(Message::Run("edit.undo"));
    assert!(app.topology_stale(), "undone, the findings are old");
    ev(&mut app, Event::Recheck);
    assert!(!app.topology_stale());
    assert!(rows(&app).iter().any(|r| r.starts_with("Parsel | Çakışmamalı | Çakışma | #2, #3")), "{:#?}", rows(&app));
    assert_eq!(app.topology_seen().expect("the tab").0, "18 bulgu (açık 18, istisna 1)", "the exception holds");
    // What the check covered.
    let scope = app.topology_scope().expect("the scope");
    assert!(scope.starts_with("Kapsam: Parsel, Bina, Yol · 6 kural · 18 nesne · değişenler ("), "{scope}");
    assert!(scope.ends_with(" · tolerans 0.001 m"), "{scope}");
    ev(&mut app, Event::Check);
    let scope = app.topology_scope().expect("the scope");
    assert!(scope.contains(" · bütün çizim · çizim sürümü "), "{scope}");
    assert!(!scope.contains("şimdi"), "{scope}");
}

#[test]
fn a_label_change_leaves_the_findings_while_a_move_or_the_tolerance_makes_them_old() {
    let mut app = checked();
    // Cartographic: the parcels' labels.
    run(
        &mut app,
        "cad.layers.labels",
        serde_json::json!({ "layer": "parsel", "label": { "placement": "center", "size": 11.0, "template": "{label}" } }),
    );
    assert!(!app.topology_stale(), "a label change is no change of what the check read");
    let scope = app.topology_scope().expect("the scope");
    assert!(scope.contains("(şimdi "), "the version moved on, said: {scope}");
    // A road moved: old.
    let uid = app.document.as_ref().and_then(|d| d.model.uid(Slot(17))).expect("the road").to_string();
    run(
        &mut app,
        "cad.entities.transform",
        serde_json::json!({ "uids": [uid], "transform": { "kind": "move", "dx": 0.4, "dy": 0.0 } }),
    );
    assert!(app.topology_stale(), "a road moved");
    ev(&mut app, Event::Check);
    assert!(!app.topology_stale());
    // The tolerance changed: old.
    let doc = app.document.as_mut().expect("open");
    let mut settings = doc.model.settings().clone();
    settings.topology.as_mut().expect("topology").tolerance = Some(0.002);
    doc.model.set_settings(settings);
    assert!(app.topology_stale(), "the tolerance changed");
}
