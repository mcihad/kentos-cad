//! topology › object_tests's tests that need the CBS module's surfaces (its project's bottom tabs and panels; docs/adr/9199
//! §4.14): moved from the core's gis/topology/object_tests.rs with the module's package; the work they test is the core's.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;
use crate::app::{App, Message};

fn opened(module: Module) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.topology.check.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(module);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The panel's Topoloji section: its title and its rows (label, value, a note).
#[allow(clippy::type_complexity)]
fn section(app: &App) -> Option<(String, Vec<(String, String, bool)>)> {
    let doc = app.document.as_ref()?;
    let panel = app.properties_panel(doc);
    panel.sections.iter().find(|s| s.id == "topology").map(|s| {
        (s.title.to_string(), s.rows.iter().map(|r| (r.label.to_string(), r.value.clone(), r.note)).collect())
    })
}

#[test]
fn gis_shows_the_section_yol_does_not() {
    let mut gis = opened(Module::Gis);
    gis.selection.set([Slot(1)]);
    assert!(section(&gis).is_some());
    let mut road = opened(Module::Road);
    road.selection.set([Slot(1)]);
    assert!(section(&road).is_none());
}
