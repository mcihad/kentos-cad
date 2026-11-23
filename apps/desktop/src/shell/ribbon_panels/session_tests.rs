//! The municipal report's 4th item (docs/adr/9169): with a data layer drawn on, Düzenle's Katmanlar panel shows its edit
//! session: Düzenlemeyi başlat while it is closed, Düzenlemeyi bitir while it is open, off with why on a read-only
//! connection; a press starts and ends it; a drawing layer has none.

use crate::app::App;
use crate::data_layers_tests::{node, parcels};
use crate::files_testing::app_with_drawing;

/// The parcels' table bound as a data layer (read-only when asked) and made the layer drawn on.
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

fn state(app: &App, id: &str) -> Option<(&'static str, bool, String)> {
    let doc = app.document.as_ref()?;
    let n = doc.model.layers().get(id)?.clone();
    app.session_state(&n).map(|(w, press, body)| (w, press.is_some(), body))
}

#[test]
fn the_panel_starts_and_ends_the_data_layers_session() {
    let (mut app, id) = drawn_on("serit-oturum", false);
    let (words, on, body) = state(&app, &id).expect("a data layer's session");
    assert_eq!((words, on), ("Düzenlemeyi başlat", true));
    assert!(body.contains("düzenlemeye kapalı"), "{body}");
    let n = app.document.as_ref().expect("a drawing").model.layers().get(&id).cloned().expect("it");
    let (_, press, _) = app.session_state(&n).expect("it");
    let _ = app.update(press.expect("a press"));
    assert!(app.document.as_ref().expect("a drawing").model.editing(&id), "started");
    let (words, on, body) = state(&app, &id).expect("it");
    assert_eq!((words, on), ("Düzenlemeyi bitir", true));
    assert!(body.contains("düzenleniyor"), "{body}");
    let (_, press, _) = app.session_state(&n).expect("it");
    let _ = app.update(press.expect("a press"));
    assert!(!app.document.as_ref().expect("a drawing").model.editing(&id), "ended: nothing changed, nothing asked");
}

#[test]
fn a_read_only_connection_is_off_with_why_and_a_drawing_layer_has_none() {
    let (app, id) = drawn_on("serit-salt-okunur", true);
    let (words, on, body) = state(&app, &id).expect("a data layer's session");
    assert_eq!((words, on), ("Salt okunur", false));
    assert!(body.contains("salt okunur bağlantıdan geliyor"), "{body}");
    assert!(state(&app, "parsel").is_none(), "a drawing layer has no session");
}

/// The picture (`.run/gui-evidence/kullanim-9169/`): Düzenle's Katmanlar panel with the data layer drawn on.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9169", "01-duzenle-oturum", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (mut app, _) = drawn_on(&format!("serit-resim-{n}"), false);
        let _ = app.update(crate::app::Message::RibbonTab("edit"));
        app
    });
}
