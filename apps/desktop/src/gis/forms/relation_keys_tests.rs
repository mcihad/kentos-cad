//! İlişkiler with a parent key (docs/adr/9135), the municipal report's case over its two data layers: `mahalleler`
//! (M1, M2, M3) the parent by its own `mahalle` code, `bakim` (M1, M1, M2, M2, M9, NULL) the child. Before Kaydet the
//! window says the key is unique, four works hold a neighbourhood's code, one holds M9 and one none; saved, the relation
//! keeps its key in the project file and İlişkiden kur makes a view joined by the code.

use crate::app::{App, Message};
use crate::data_layers_tests::{municipal, node};
use crate::exchange::words::Kind;
use crate::files_testing::{app_with_drawing, drive};
use crate::forms::relations::Event;
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Relations(e)));
    drive(app, task);
}

/// The report's two data layers and İlişkiler open on a new relation of them by the neighbourhood's code.
fn opened(tag: &str) -> (App, String, String, std::path::PathBuf) {
    crate::data_layers::register();
    let path = municipal(tag);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&address, "bakim").expect("described"),
        crate::data_layers::describe(&address, "mahalleler").expect("described"),
    ])
    .expect("added");
    let parent = node(&app, "mahalleler").expect("mahalleler").id.clone();
    let child = node(&app, "bakim").expect("bakim").id.clone();
    let task = app.update(Message::Run("data.relations"));
    drive(&mut app, task);
    ev(&mut app, Event::New);
    ev(&mut app, Event::Name("Mahalle bakımları".into()));
    ev(&mut app, Event::Parent(parent.clone()));
    ev(&mut app, Event::Child(child.clone()));
    ev(&mut app, Event::Field("mahalle".into()));
    ev(&mut app, Event::ParentKey(Some("mahalle".into())));
    (app, parent, child, path)
}

/// The checks of the relation chosen in the window, read until the data layers' rows are in.
fn checks(app: &mut App) -> Vec<(Kind, String)> {
    let relation = |app: &App| {
        app.gis
            .relations_window
            .as_ref()
            .and_then(|w| w.chosen_for_test().cloned())
            .expect("a relation")
    };
    for _ in 0..3 {
        let r = relation(app);
        let _ = app.relation_checks(&r);
        let task = app.data_rows_tasks();
        drive(app, task);
    }
    let r = relation(app);
    app.relation_checks(&r)
}

#[test]
fn the_window_ties_by_the_neighbourhoods_code_and_says_what_it_ties_before_saving() {
    let (mut app, parent, child, path) = opened("iliski-anahtar");
    let lines = checks(&mut app);
    let said: Vec<&str> = lines.iter().map(|(_, l)| l.as_str()).collect();
    assert!(
        said.contains(&"Üst anahtar “mahalle” benzersiz: 3 kaydın 3 ayrı değeri var."),
        "{said:#?}"
    );
    assert!(
        said.contains(&"Alt kayıtlar: 6; 4 tanesi bir üst kayda bağlı."),
        "{said:#?}"
    );
    assert!(
        said.contains(&"1 alt kayıt hiçbir üst kayıtta olmayan değeri tutuyor: “M9”."),
        "{said:#?}"
    );
    assert!(
        said.contains(&"1 alt kaydın alanı boş (NULL): üst kaydı yok."),
        "{said:#?}"
    );
    assert!(
        lines
            .iter()
            .any(|(k, l)| *k == Kind::Warn && l.contains("M9")),
        "M9 is a warning, NULL is not"
    );
    // Saved with its key.
    ev(&mut app, Event::Save);
    let doc = app.document.as_ref().expect("open");
    let saved = doc.model.settings().relations.clone();
    assert_eq!(saved.len(), 1);
    assert_eq!(
        (
            saved[0].parent.as_str(),
            saved[0].child.as_str(),
            saved[0].field.as_str(),
            saved[0].parent_key.as_deref()
        ),
        (parent.as_str(), child.as_str(), "mahalle", Some("mahalle"))
    );
    // The project file keeps it: written and read back.
    let bytes = kentos_kcad::encode_verified(&doc.model.to_snapshot_v2()).expect("written");
    let back = kentos_kcad::decode(&bytes).expect("read");
    assert_eq!(
        back.settings.relations, saved,
        "the key read back as written"
    );
    // İlişkiden kur: a view joined by the code, not the persistent id.
    let task = app.update(Message::Run("data.joinViews"));
    drive(&mut app, task);
    let task = app.update(Message::Gis(gis::Event::Joins(
        crate::joins_window::Event::FromRelation(0),
    )));
    drive(&mut app, task);
    let task = app.update(Message::Gis(gis::Event::Joins(
        crate::joins_window::Event::Save,
    )));
    drive(&mut app, task);
    let views = &app
        .document
        .as_ref()
        .expect("open")
        .model
        .settings()
        .join_views;
    let view = views.iter().find(|v| v.layer == parent).expect("made");
    assert_eq!(
        (view.key.as_str(), view.with_key.as_str()),
        ("mahalle", "mahalle")
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn another_parent_layer_clears_the_key_and_the_id_choice_shows_no_check() {
    let (mut app, _, child, path) = opened("iliski-anahtar-degisir");
    // A key of another layer is not this one's: another parent layer clears it.
    ev(&mut app, Event::Parent(child));
    let w = app.gis.relations_window.as_ref().expect("open");
    assert_eq!(w.chosen_for_test().and_then(|r| r.parent_key.clone()), None);
    // By the persistent id: no check is said (nothing to compare before Kaydet).
    let r = w.chosen_for_test().cloned().expect("a relation");
    assert!(app.relation_checks(&r).is_empty());
    let _ = std::fs::remove_file(&path);
}

/// Pictures (docs/adr/9135), light at 1440×900 and dark at 1100×760: İlişkiler on the neighbourhoods' works by the
/// neighbourhood's code, its checks under the form before Kaydet.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib relation_keys_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::files_testing::usage_shots("9135", "01-ust-anahtar", || {
        let (mut app, _, _, _) = opened("iliski-resim");
        let _ = checks(&mut app);
        app
    });
}

/// A saved relation removed in the window says, before Kaydet, that the child records keep their values and which
/// form's related list shows the link (docs/adr/9153).
#[test]
fn removing_a_relation_says_what_it_touches_before_saving() {
    let (mut app, parent, child, path) = opened("iliski-kaldir");
    ev(&mut app, Event::Save);
    // A form on the neighbourhoods listing their works by the code.
    let body = serde_json::json!({ "revision": 1, "items": [
        { "type": "related", "id": "isler", "label": "Bakım işleri", "layer": child, "field": "mahalle" }
    ]});
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = serde_json::json!({
        "id": "mahalle-formu", "name": "Mahalle formu", "layer": parent, "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body, "note": "İlk yayın" }]
    });
    let doc = app.document.as_mut().expect("open");
    let mut settings = doc.model.settings().clone();
    settings
        .forms
        .push(serde_json::from_value(form).expect("a form"));
    doc.model.set_settings(settings);
    let task = app.update(Message::Run("data.relations"));
    drive(&mut app, task);
    ev(&mut app, Event::Choose(0));
    ev(&mut app, Event::Remove);
    let w = app.gis.relations_window.clone().expect("the window");
    let lines = app.relation_removals(&w);
    assert_eq!(
        lines,
        [
            "“Mahalle bakımları” Kaydet'te kaldırılır: alt kayıtların “mahalle” değerleri olduğu gibi kalır. İlişkili listesi bu bağı gösteren form: “Mahalle formu”."
        ]
    );
    let _ = std::fs::remove_file(&path);
}

/// Kaydet writes every relation or none (docs/adr/9401 §4): a relation the command refuses leaves the one made before
/// it unwritten, and the window stays open on both.
#[test]
fn a_refused_relation_leaves_the_others_unwritten() {
    let (mut app, parent, child, path) = opened("butun");
    // A second relation by a field the child layer does not have.
    ev(&mut app, Event::New);
    ev(&mut app, Event::Name("Bozuk".into()));
    ev(&mut app, Event::Parent(parent));
    ev(&mut app, Event::Child(child));
    ev(&mut app, Event::Field("yok-alan".into()));
    ev(&mut app, Event::Save);
    assert!(
        app.gis.relations_window.is_some(),
        "the window stays on what did not go in"
    );
    let doc = app.document.as_ref().expect("open");
    let names: Vec<&str> = doc
        .model
        .settings()
        .relations
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert!(
        names.is_empty(),
        "nothing of the Kaydet is written: {names:?}"
    );
    let _ = std::fs::remove_file(&path);
}
