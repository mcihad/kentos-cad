//! Düzenle's Düzenleme panel (docs/adr/9201): the active data layer's edit session started and ended by Düzenlemeyi
//! başlat, whose face says Düzenlemeyi bitir while it is open; off, with why, on a drawing layer and on a read-only
//! connection; Değişiklikleri kaydet, geri al and incele on only while something waits; geri al asked once, letting the
//! changes go and keeping the layer open. CBS's ribbon holds the panel first in Düzenle, its tools kept large.

use kentos_native_application::ExecutionContext;

use crate::app::{App, Dialog, Message};
use crate::data_edits::{Answer, Event};
use crate::data_layers_tests::{node, parcels};
use crate::edit_commands::{REVIEW, ROLLBACK, SAVE, TOGGLE};
use crate::files_testing::app_with_drawing;

/// The parcels' table added as a data layer (read-only when asked) and made the active layer.
fn drawn_on(name: &str, read_only: bool) -> (App, String) {
    crate::data_layers::register();
    let address = parcels(name, 10).display().to_string();
    let mut app = app_with_drawing();
    let mut described = crate::data_layers::describe(&address, "parseller").expect("described");
    described.data.read_only = read_only;
    app.add_data_layers(vec![described]).expect("added");
    let id = node(&app, "parseller").expect("the layer").id.clone();
    app.document.as_mut().expect("a drawing").model.set_active_layer(&id);
    (app, id)
}

/// A polygon drawn on `layer` by `cad.entities.create`, as a drawing tool writes it: a new row.
fn draw(app: &mut App, layer: &str) -> bool {
    let p = |x: f64, y: f64| serde_json::json!({ "x": 486_400.0 + x, "y": 4_420_100.0 + y });
    let input = serde_json::from_value(serde_json::json!({
        "layerId": layer,
        "objects": [{ "geometry": { "kind": "polygon", "pts": [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)] } }]
    }))
    .expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    matches!(
        kentos_native_application::create::execute(&mut ExecutionContext::new(&mut doc.model), input),
        kentos_commands::CommandResult::Completed { .. }
    )
}

fn editing(app: &App, id: &str) -> bool {
    app.document.as_ref().expect("a drawing").model.editing(id)
}

#[test]
fn the_ribbon_starts_and_ends_the_active_data_layers_session() {
    let (mut app, id) = drawn_on("serit-duzenleme", false);
    assert!(app.available(TOGGLE), "{:?}", app.why_disabled(TOGGLE));
    assert_eq!((app.checked(TOGGLE), app.edit_face(TOGGLE)), (Some(false), Some("Düzenlemeyi\nbaşlat")));
    let _ = app.update(Message::Run(TOGGLE));
    assert!(editing(&app, &id), "started");
    assert_eq!((app.checked(TOGGLE), app.edit_face(TOGGLE)), (Some(true), Some("Düzenlemeyi\nbitir")));
    // Nothing waits yet: Kaydet, Geri al and İncele are off, each saying why.
    for id in [SAVE, ROLLBACK] {
        assert!(!app.available(id));
        assert!(app.why_disabled(id).is_some_and(|w| w.contains("kaydedilmemiş değişiklik yok")), "{id}");
    }
    assert!(!app.available(REVIEW));
    // Ended with nothing changed: nothing asked.
    let _ = app.update(Message::Run(TOGGLE));
    assert!(!editing(&app, &id), "ended");
    assert!(app.dialog.is_none());
}

#[test]
fn a_drawing_layer_and_a_read_only_connection_are_off_with_why() {
    let (mut app, _) = drawn_on("serit-salt-okunur", true);
    assert!(!app.available(TOGGLE));
    let why = app.why_disabled(TOGGLE).expect("why");
    assert!(why.contains("salt okunur bağlantıdan geliyor"), "{why}");
    // A drawing layer active: not a data layer, said by name.
    app.document.as_mut().expect("a drawing").model.set_active_layer("parsel");
    assert!(!app.available(TOGGLE));
    let why = app.why_disabled(TOGGLE).expect("why");
    assert!(why.contains("bir veri katmanı değil"), "{why}");
    // Run anyway (Komut ara, the command line): refused with the same words, nothing opened.
    let _ = app.update(Message::Run(TOGGLE));
    assert!(app.document.as_ref().expect("a drawing").model.edited_layers().is_empty());
}

#[test]
fn geri_al_asks_once_then_lets_the_changes_go_and_keeps_the_layer_open() {
    let (mut app, id) = drawn_on("serit-geri-al", false);
    let _ = app.update(Message::Run(TOGGLE));
    assert!(draw(&mut app, &id), "drawn on the open layer");
    assert!(app.available(SAVE) && app.available(ROLLBACK) && app.available(REVIEW));
    // Asked first: Vazgeç keeps everything.
    let _ = app.update(Message::Run(ROLLBACK));
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataEdits)));
    let _ = app.data_edits_question();
    let _ = app.data_edits_event(Event::Answer(Answer::Cancel));
    assert!(app.document.as_ref().expect("a drawing").model.data_modified(&id), "kept");
    // Değişiklikleri geri al: gone, the layer still open, nothing left to save.
    let _ = app.update(Message::Run(ROLLBACK));
    let _ = app.data_edits_event(Event::Answer(Answer::Discard));
    let model = &app.document.as_ref().expect("a drawing").model;
    assert!(!model.data_modified(&id) && model.count(&id) == 0, "let go");
    assert!(editing(&app, &id), "the layer stays open");
    assert!(!app.available(SAVE) && !app.available(ROLLBACK));
    // Düzenlemeyi bitir with a change waiting asks too (the session's own question), as before.
    assert!(draw(&mut app, &id));
    let _ = app.update(Message::Run(TOGGLE));
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataEdits)));
    let _ = app.data_edits_event(Event::Answer(Answer::Discard));
    assert!(!editing(&app, &id), "ended, its change let go");
}

#[test]
fn cbs_s_duzenle_opens_with_the_session_panel_and_its_tools_large() {
    use crate::catalog::{Item, Size};
    let (mut app, _) = drawn_on("serit-panel", false);
    let _ = app.update(Message::Run("workspace.gis"));
    let tab = app.ribbon_tabs().find(|t| t.id == "edit").expect("Düzenle");
    let first = &tab.panels[0];
    // Kept (docs/adr/9200): nothing is drawn on a closed layer, so the session's button outlasts Oluştur's large tools.
    assert_eq!((first.label, first.keep), ("Düzenleme", true));
    let items: Vec<(&str, bool)> = first
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Command { id, size } => Some((*id, *size == Size::Large)),
            _ => None,
        })
        .collect();
    assert_eq!(items, [(TOGGLE, true), (SAVE, false), (ROLLBACK, false), (REVIEW, false)]);
}

/// The pictures of docs/adr/9201 for the owner, light at 1440×900 and dark at 1100×760, into
/// `.run/gui-evidence/kullanim-9201/`: Düzenle with a data layer active and its session closed; open with a polygon
/// drawn (Düzenlemeyi bitir lit, Kaydet, Geri al and İncele on); Değişiklikleri geri al's question.
///
/// ```text
/// cargo test -p kentos-desktop --lib edit_commands_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let fresh = || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (mut app, id) = drawn_on(&format!("serit-duzenleme-resim-{n}"), false);
        let _ = app.update(Message::RibbonTab("edit"));
        let _ = app.update(Message::Run("view.zoomExtents"));
        (app, id)
    };
    crate::files_testing::usage_shots("9201", "1-oturum-kapali", || fresh().0);
    crate::files_testing::usage_shots("9201", "2-oturum-acik-degisiklik", || {
        let (mut app, id) = fresh();
        let _ = app.update(Message::Run(TOGGLE));
        assert!(draw(&mut app, &id));
        app
    });
    crate::files_testing::usage_shots("9201", "3-geri-al-sorusu", || {
        let (mut app, id) = fresh();
        let _ = app.update(Message::Run(TOGGLE));
        assert!(draw(&mut app, &id));
        let _ = app.update(Message::Run(ROLLBACK));
        app
    });
}
