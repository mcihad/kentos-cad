//! A data layer's edit session on the desktop (docs/adr/9103 §5): opened from Katmanlar, a row drawn, a row taken by
//! Seç and moved, a row deleted by its key, the save written to the GeoPackage and taken out of the drawing, the question
//! before a layer with changes closes and before the drawing is left.

use kentos_interaction::DataRegion;
use kentos_native_application::ExecutionContext;
use kentos_native_application::data_write;

use crate::app::{Dialog, Then};
use crate::data_edits::{Answer, Event};
use crate::data_layers_tests::{node, parcel_point, parcels};
use crate::files_testing::app_with_drawing;

/// A polygon drawn on `layer` by `cad.entities.create`, as a drawing tool writes it: a new row.
fn draw(app: &mut crate::app::App, layer: &str, x: f64, y: f64) -> kentos_commands::CommandResult<kentos_commands::EntitiesCreated> {
    let p = |dx: f64, dy: f64| serde_json::json!({ "x": x + dx, "y": y + dy });
    let input = serde_json::from_value(serde_json::json!({
        "layerId": layer,
        "objects": [{
            "geometry": { "kind": "polygon", "pts": [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)] },
            "attrs": { "ada": "300", "kullanim": "Yeni" }
        }]
    }))
    .expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    kentos_native_application::create::execute(&mut ExecutionContext::new(&mut doc.model), input)
}

/// The rows of `keys` as the table holds them now.
fn rows(app: &crate::app::App, layer: &kentos_commands::LayerNode, keys: &[i64]) -> Vec<(i64, kentos_commands::Entity)> {
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    data_write::rows_to_take(layer, &settings, keys, &|| false).expect("read")
}

/// The save the window runs on a thread, run here and its outcome handed back.
fn save(app: &mut crate::app::App, layer: &str) {
    let _ = app.data_edits_event(Event::Save(vec![layer.to_owned()]));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let writes = data_write::writes_of(doc, layer).expect("being edited");
    let outcome = data_write::write(&writes, doc.settings()).map(|keys| (writes, keys));
    let _ = app.data_edits_event(Event::Saved(layer.to_owned(), Box::new(outcome)));
}

#[test]
fn a_data_layers_changes_wait_in_the_drawing_until_saved_and_are_asked_about_before_it_closes() {
    crate::data_layers::register();
    let path = parcels("oturum", 40);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let layer = node(&app, "parseller").expect("the layer").clone();
    let id = layer.id.clone();

    // Not being edited: a drawn polygon is refused, nothing written.
    assert!(matches!(draw(&mut app, &id, 486_400.0, 4_420_100.0), kentos_commands::CommandResult::Failed { .. }));
    assert_eq!(app.document.as_ref().expect("a drawing").model.count(&id), 0);

    // Düzenlemeyi başlat: the layer opens and becomes the active one.
    let _ = app.data_edits_event(Event::Start(id.clone()));
    let model = &app.document.as_ref().expect("a drawing").model;
    assert!(model.editing(&id));
    assert_eq!(model.layers().active(), id);

    // A new row drawn; the 22nd parcel taken by Seç's click and moved 3 m east; the 5th deleted by its key.
    assert!(matches!(draw(&mut app, &id, 486_400.0, 4_420_100.0), kentos_commands::CommandResult::Completed { .. }));
    let at = parcel_point(42.0, 42.0);
    app.gis.data_pick.wanted.push(DataRegion::Point { at, reach: 0.5 });
    let _ = app.data_pick_tasks();
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let keys = crate::data_pick::keys_in(&layer, &settings, DataRegion::Point { at, reach: 0.5 }, 10).expect("keys");
    assert_eq!(keys, vec![22]);
    let taken = data_write::rows_to_take(&layer, &settings, &keys, &|| false).expect("rows");
    let asked = app.gis.data_pick.asked;
    let _ = app.data_pick_event(crate::data_pick::Event::Answered(
        asked,
        vec![crate::data_pick::Got::Rows(id.clone(), taken)],
        Vec::new(),
    ));
    let model = &app.document.as_ref().expect("a drawing").model;
    let slot = *app.selection.ids().first().expect("the taken row is selected");
    let kentos_commands::Entity::Polygon(mut p) = model.get(slot).cloned().expect("its object") else {
        panic!("a parcel is an area");
    };
    for v in &mut p.pts {
        v.x += 3.0;
    }
    let doc = app.document.as_mut().expect("a drawing");
    assert!(doc.model.update(slot, kentos_commands::Entity::Polygon(p)));
    assert_eq!(doc.model.delete_data_rows(&id, &[5]), Ok(1));
    let c = doc.model.data_changes(&id);
    assert_eq!((c.added.len(), c.changed.len(), c.deleted.len()), (1, 1, 1));
    assert_eq!(crate::data_edits::counts_words(1, 1, 1), "1 yeni, 1 değişen, 1 silinen satır");

    // Değişiklikleri kaydet: one transaction; what was written leaves the drawing, the layer stays open.
    save(&mut app, &id);
    let model = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(model.count(&id), 0, "the rows are in the table now");
    assert!(model.editing(&id) && !model.data_modified(&id));
    assert!(rows(&app, &layer, &[5]).is_empty(), "the 5th is gone");
    let moved = rows(&app, &layer, &[22]);
    let kentos_commands::Entity::Polygon(p) = &moved[0].1 else {
        panic!("an area");
    };
    assert!(p.pts.iter().any(|v| (v.x - (486_500.0 + 30.0 + 3.0)).abs() < 1e-6), "{:?}", p.pts);
    let new = rows(&app, &layer, &[41]);
    assert_eq!(new.len(), 1, "the new row took the next key");
    assert_eq!(new[0].1.base().attrs.get("kullanim").map(String::as_str), Some("Yeni"));

    // Düzenlemeyi bitir with a change: asked; Kaydetme lets it go and closes the layer.
    assert!(matches!(draw(&mut app, &id, 486_420.0, 4_420_100.0), kentos_commands::CommandResult::Completed { .. }));
    let _ = app.data_edits_event(Event::Stop(id.clone()));
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataEdits)));
    let _ = app.data_edits_question();
    let _ = app.data_edits_event(Event::Answer(Answer::Discard));
    let model = &app.document.as_ref().expect("a drawing").model;
    assert!(!model.editing(&id) && model.count(&id) == 0);
    assert!(rows(&app, &layer, &[42]).is_empty(), "nothing more was written");

    // Leaving the drawing with a change: asked first; Vazgeç leaves everything as it was.
    let _ = app.data_edits_event(Event::Start(id.clone()));
    assert!(matches!(draw(&mut app, &id, 486_420.0, 4_420_100.0), kentos_commands::CommandResult::Completed { .. }));
    let _ = app.leave(Then::CloseTab);
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataEdits)));
    let _ = app.data_edits_event(Event::Answer(Answer::Cancel));
    let model = &app.document.as_ref().expect("a drawing").model;
    assert!(model.editing(&id) && model.data_modified(&id));
    assert!(app.dialog.is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_box_of_sec_keeps_the_rows_wholly_inside_it_or_meeting_it() {
    crate::data_layers::register();
    let path = parcels("cerceve", 40);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let layer = node(&app, "parseller").expect("the layer").clone();
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    // From inside the first parcel to inside the 23rd: only the 22nd (second row, second column) lies wholly inside.
    let (a, b) = (parcel_point(5.0, 5.0), parcel_point(65.0, 65.0));
    let window = crate::data_pick::keys_in(&layer, &settings, DataRegion::Box { a, b, crossing: false }, 100).expect("keys");
    assert_eq!(window, vec![22]);
    let mut crossing =
        crate::data_pick::keys_in(&layer, &settings, DataRegion::Box { a, b, crossing: true }, 100).expect("keys");
    crossing.sort_unstable();
    assert_eq!(crossing, vec![1, 2, 3, 21, 22, 23]);
    // Past the most a session takes: refused with the count.
    let refused = crate::data_pick::keys_in(&layer, &settings, DataRegion::Box { a, b, crossing: true }, 3);
    assert!(refused.is_err_and(|why| why.contains("en çok 3 satır")));
    let _ = std::fs::remove_file(&path);
}
