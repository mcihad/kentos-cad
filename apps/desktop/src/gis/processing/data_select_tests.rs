//! İfadeyle seç on a data layer (docs/adr/9133), the municipal report's `bakim` (bedel 0, 100, 250, NULL, 50, 100):
//! `bedel >= 100` selects B02, B03 and B06 in the table and on the map, Seçime yakınlaştır frames them; `bedel IS NULL`
//! only B04; Seçime ekle, Seçimden çıkar and Seçim içinde ara work on the same keys; the preview counts the source's
//! rows; a stopped run leaves the selection as it was; CAD özellik süzgeci is refused there.

use std::collections::BTreeSet;

use serde_json::json;

use super::super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::data_layers_tests::{municipal, node};
use crate::files_testing::{app_with_drawing, drive};
use crate::gis;

fn event(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Processing(e)));
    drive(app, task);
}

fn status(app: &App) -> RunStatus {
    app.gis.processing.dialog.as_ref().map_or(RunStatus::Idle, |w| w.status.clone())
}

fn selected(app: &App) -> BTreeSet<i64> {
    app.gis.features.data.selected.clone()
}

/// İfadeyle seç open on the report's `bakim` as a data layer, its condition and Seçim biçimi given.
fn opened(tag: &str) -> (App, String, std::path::PathBuf) {
    crate::data_layers::register();
    let path = municipal(tag);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&address, "bakim").expect("described"),
        crate::data_layers::describe(&address, "mahalleler").expect("described"),
    ])
    .expect("added");
    let bakim = node(&app, "bakim").expect("bakim").id.clone();
    let task = app.update(Message::Run("processing.run.selection.byExpression"));
    drive(&mut app, task);
    (app, bakim, path)
}

fn on_layer(layer: &str) -> Event {
    Event::Value("input".into(), json!({ "scope": "layer", "layerId": layer }))
}

fn ask(app: &mut App, condition: &str, mode: &str) {
    event(app, Event::Text("condition".into(), condition.into()));
    event(app, Event::Value("mode".into(), json!(mode)));
    event(app, Event::Run);
}

#[test]
fn a_condition_selects_a_data_layers_rows_by_their_keys_in_every_mode() {
    let (mut app, bakim, path) = opened("ifadeyle-veri");
    event(&mut app, on_layer(&bakim));
    ask(&mut app, "bedel >= 100", "new");
    let RunStatus::Ok { text, selected: chose, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "3 / 6 satır koşulu sağladı; seçimde 3 satır var.");
    assert!(chose, "the run chose the selection itself");
    assert_eq!(selected(&app), BTreeSet::from([2, 3, 6]));
    // The table shows the layer with its rows selected; the map outlines them and Seçime yakınlaştır frames them.
    assert_eq!(app.gis.features.layer.as_deref(), Some(bakim.as_str()));
    assert_eq!(app.data_table_layer().map(|n| n.id), Some(bakim.clone()));
    assert!(app.data_rows_outlined(), "the three rows outlined on the map");
    assert!(app.available("view.zoomSelection"));
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 800.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    let task = app.update(Message::Run("view.zoomSelection"));
    drive(&mut app, task);
    // The rows where the drawing has them (moved from the file's system into the project's): all three in the view.
    let node = node(&app, "bakim").expect("bakim").clone();
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let input = kentos_commands::LayersRows {
        layer: bakim.clone(),
        keys: Some(vec![2, 3, 6]),
        geometry: Some(true),
        ..Default::default()
    };
    let rows = kentos_native_application::layers_rows::read(&node, &settings, &input, &|| false).expect("read").rows;
    let b = app.viewport.camera.visible_bounds();
    assert_eq!(rows.len(), 3);
    for r in &rows {
        let p = kentos_data::wkt::read_wkt(r.wkt.as_deref().expect("a shape")).expect("wkt");
        let kentos_data::Geometry::Point(Some(c)) = p else { panic!("{p:?}") };
        assert!(b.min_x < c.x && c.x < b.max_x && b.min_y < c.y && c.y < b.max_y, "{c:?} out of {b:?}");
    }
    assert!(b.max_x - b.min_x < 200.0, "framed close: {b:?}");
    // NULL is its own: only B04.
    ask(&mut app, "bedel IS NULL", "new");
    assert_eq!(selected(&app), BTreeSet::from([4]));
    // Seçime ekle, Seçimden çıkar, Seçim içinde ara on the same keys.
    ask(&mut app, "bedel >= 100", "add");
    assert_eq!(selected(&app), BTreeSet::from([2, 3, 4, 6]));
    ask(&mut app, "bedel >= 250", "remove");
    assert_eq!(selected(&app), BTreeSet::from([2, 4, 6]));
    ask(&mut app, "bedel = 100", "within");
    assert_eq!(selected(&app), BTreeSet::from([2, 6]));
    let RunStatus::Ok { text, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "2 / 6 satır koşulu sağladı; seçimde 2 satır var.");
    // A condition no row meets selects none and says so as a warning.
    ask(&mut app, "bedel > 1000", "new");
    assert!(matches!(status(&app), RunStatus::Ok { warning: true, .. }), "{:?}", status(&app));
    assert!(selected(&app).is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_preview_counts_the_sources_rows_and_a_stopped_run_keeps_the_selection() {
    let (mut app, bakim, path) = opened("ifadeyle-onizleme");
    // The rows read after the message that chose the layer (its task not run yet), then counted.
    let read = app.update(Message::Gis(gis::Event::Processing(on_layer(&bakim))));
    let preview = |app: &App| app.gis.processing.dialog.as_ref().and_then(|w| w.previews.get("condition").cloned());
    assert_eq!(preview(&app).as_deref(), Some(super::words::READING));
    drive(&mut app, read);
    event(&mut app, Event::Text("condition".into(), "bedel >= 100".into()));
    assert_eq!(preview(&app).as_deref(), Some("3 / 6 satır koşulu sağlıyor."));
    // A field the layer lacks is said before the run.
    event(&mut app, Event::Text("condition".into(), "ad = 'Kale'".into()));
    let said = preview(&app).expect("said");
    assert!(said.contains("“ad” alanı “bakim” katmanında yok"), "{said}");
    // A run stopped before its answer: the selection stays.
    ask(&mut app, "bedel >= 100", "new");
    assert_eq!(selected(&app), BTreeSet::from([2, 3, 6]));
    event(&mut app, Event::Text("condition".into(), "bedel IS NULL".into()));
    let run = app.update(Message::Gis(gis::Event::Processing(Event::Run)));
    assert!(matches!(status(&app), RunStatus::Running { .. }), "{:?}", status(&app));
    event(&mut app, Event::Stop);
    drive(&mut app, run);
    assert!(matches!(status(&app), RunStatus::Error(ref w) if w == super::words::STOPPED), "{:?}", status(&app));
    assert_eq!(selected(&app), BTreeSet::from([2, 3, 6]), "the selection before the run");
    // CAD özellik süzgeci asks of the drawing's objects alone.
    event(&mut app, Event::Value("cadFilter".into(), json!(true)));
    event(&mut app, Event::Run);
    assert!(matches!(status(&app), RunStatus::Invalid(ref w) if w == super::words::CAD), "{:?}", status(&app));
    let _ = std::fs::remove_file(&path);
}

/// Pictures (docs/adr/9133), light at 1440×900 and dark at 1100×760: İfadeyle seç on `bakim` with its preview, the run
/// done with the table's three rows selected, and Seçime yakınlaştır framing them on the map.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib data_select_tests::screens -- --ignored --nocapture"]
fn screens() {
    let prepare = |step: usize| {
        move || {
            let (mut app, bakim, _) = opened(&format!("ifadeyle-resim-{step}"));
            let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1100.0, 420.0));
            let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
            event(&mut app, on_layer(&bakim));
            event(&mut app, Event::Text("condition".into(), "bedel >= 100".into()));
            if step >= 2 {
                event(&mut app, Event::Run);
            }
            if step >= 3 {
                event(&mut app, Event::Results);
            }
            app
        }
    };
    crate::files_testing::usage_shots("9133", "01-onizleme", prepare(1));
    crate::files_testing::usage_shots("9133", "02-secildi", prepare(2));
    crate::files_testing::usage_shots("9133", "03-secime-yakinlastir", prepare(3));
}
