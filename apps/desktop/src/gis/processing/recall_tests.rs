//! İfadeyle seç's first state and what a selection says (docs/adr/9341; the CBS review of 27 November 2026, item 7).
//!
//! - The last run's condition is not written into the field again: the tool's own is, the last one offered as Son
//!   ifade, which writes it in.
//! - The inspector's Katman names the layer by its group path (“Kadastro / Parsel”).
//! - Tümünü seç says how many of the project's objects it took, and why the others were not.

use serde_json::json;

use super::Event;
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive, last_said};
use crate::gis;

fn event(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Processing(e)));
    drive(app, task);
}

fn open(app: &mut App) {
    let task = app.update(Message::Run("processing.run.selection.byExpression"));
    drive(app, task);
}

fn condition(app: &App) -> String {
    let window = app.gis.processing.dialog.as_ref().expect("open");
    window
        .values
        .get("condition")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn the_last_condition_is_offered_not_written_in() {
    let mut app = app_with_drawing();
    open(&mut app);
    assert_eq!(condition(&app), "$alan > 500", "the tool's own first");
    event(
        &mut app,
        Event::Value("input".into(), json!({ "scope": "all" })),
    );
    event(
        &mut app,
        Event::Text("condition".into(), "bedel >= 100".into()),
    );
    event(&mut app, Event::Run);
    event(&mut app, Event::Close);
    open(&mut app);
    assert_eq!(condition(&app), "$alan > 500", "not the last one");
    let window = app.gis.processing.dialog.as_ref().expect("open");
    assert_eq!(
        window.recalled.get("condition").map(String::as_str),
        Some("bedel >= 100")
    );
    // Son ifade writes it in.
    event(
        &mut app,
        Event::Text("condition".into(), "bedel >= 100".into()),
    );
    assert_eq!(condition(&app), "bedel >= 100");
}

#[test]
fn the_inspector_names_the_layer_by_its_group_path() {
    let mut app = app_with_drawing();
    let doc = &app.document.as_ref().expect("a drawing").model;
    let parcel = doc
        .entities()
        .find(|e| e.base().layer_id == "parsel")
        .map(|e| kentos_domain::Slot(e.base().id));
    let slot = parcel.expect("a parcel");
    app.selection.set([slot]);
    let doc = app.document.as_ref().expect("a drawing");
    let panel = app.properties_panel(doc);
    let katman = panel
        .sections
        .iter()
        .flat_map(|s| &s.rows)
        .find(|r| r.label == "Katman")
        .and_then(|r| match &r.editor {
            Some(crate::properties::rows::Editor::Select { text, .. }) => Some(text.clone()),
            _ => None,
        });
    assert_eq!(katman.as_deref(), Some("Kadastro / Parsel"));
}

#[test]
fn select_all_says_what_it_took_of_the_project() {
    let mut app = app_with_drawing();
    let total = app.document.as_ref().expect("a drawing").model.len();
    let task = app.update(Message::Run("edit.selectAll"));
    drive(&mut app, task);
    let n = app.selection.len();
    assert!(n < total, "the hidden layer's objects are not taken");
    assert_eq!(
        last_said(&app),
        format!(
            "{n} nesne seçildi: görünen ve seçilebilir katmanların nesneleri. Projedeki {total} nesneden {} tanesi seçilmedi: gizli, donmuş ya da seçilemez katmanda ya da seçim süzgecinin dışında.",
            total - n
        )
    );
}

#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9341", "02-ifadeyle-sec-son-ifade", || {
        let mut app = app_with_drawing();
        open(&mut app);
        event(
            &mut app,
            Event::Text("condition".into(), "bedel >= 100".into()),
        );
        event(&mut app, Event::Run);
        event(&mut app, Event::Close);
        open(&mut app);
        app
    });
}
