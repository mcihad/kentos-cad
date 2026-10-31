//! A related item's actions in the form's window (docs/adr/0762): what the form offers, what the
//! user may, Sil asked in the row, a child opened read-only.

use kentos_commands::RelatedAction as A;
use kentos_domain::Slot;
use serde_json::json;

use crate::app::{App, Message};
use crate::forms::entry::Event;

fn owners(actions: serde_json::Value, locked: bool) -> App {
    let mut app = crate::files_testing::app_with_drawing();
    crate::form_scenes::owner_actions(&mut app, actions, locked);
    assert!(app.form_entry.is_some(), "the building's form is open");
    app
}

fn offers(app: &App) -> Vec<(A, bool)> {
    let e = app.form_entry.as_ref().expect("the form");
    app.related_offers(e, "malikler")
        .into_iter()
        .map(|o| (o.action, o.allowed()))
        .collect()
}

fn linked(app: &App) -> Vec<String> {
    app.form_entry
        .as_ref()
        .expect("the form")
        .related_of("malikler")
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::FormEntry(e));
}

#[test]
fn the_item_offers_what_the_form_names() {
    let app = owners(json!(["link", "view", "show"]), false);
    assert_eq!(
        offers(&app),
        vec![(A::Link, true), (A::View, true), (A::Show, true)]
    );
    let app = owners(serde_json::Value::Null, false);
    assert_eq!(
        offers(&app).into_iter().map(|(a, _)| a).collect::<Vec<_>>(),
        A::DEFAULT.to_vec(),
        "none named: what related items always did"
    );
}

#[test]
fn an_action_not_offered_is_refused_and_says_why() {
    let mut app = owners(json!(["link", "view", "show"]), false);
    let before = linked(&app);
    event(
        &mut app,
        Event::Unlink("malikler".into(), before[0].clone()),
    );
    assert_eq!(linked(&app), before, "nothing is pending");
    assert!(
        crate::files_testing::last_said(&app).contains("“Bağı kopar” sunmuyor"),
        "{}",
        crate::files_testing::last_said(&app)
    );
    event(&mut app, Event::NewRelated("malikler".into()));
    assert!(crate::files_testing::last_said(&app).contains("“Yeni” sunmuyor"));
}

#[test]
fn a_locked_child_layer_keeps_reading_open() {
    let app = owners(
        json!([
            "create", "update", "delete", "link", "unlink", "view", "show"
        ]),
        true,
    );
    let open: Vec<A> = offers(&app)
        .into_iter()
        .filter(|(_, ok)| *ok)
        .map(|(a, _)| a)
        .collect();
    assert_eq!(open, vec![A::View, A::Show]);
    let e = app.form_entry.as_ref().expect("the form");
    let why = app
        .related_offers(e, "malikler")
        .into_iter()
        .find_map(|o| o.refused)
        .expect("a reason");
    assert!(why.contains("“Malik” katmanı kilitli"), "{why}");
}

#[test]
fn sil_asks_in_the_row_then_deletes_in_its_own_step() {
    let mut app = owners(
        json!([
            "create", "update", "delete", "link", "unlink", "view", "show"
        ]),
        false,
    );
    let before = linked(&app);
    event(
        &mut app,
        Event::DeleteRelated("malikler".into(), before[0].clone()),
    );
    assert_eq!(linked(&app).len(), 2, "asked, not deleted yet");
    event(&mut app, Event::KeepChild);
    event(&mut app, Event::ConfirmDelete);
    assert_eq!(linked(&app).len(), 2, "Vazgeç forgot it");
    event(
        &mut app,
        Event::DeleteRelated("malikler".into(), before[0].clone()),
    );
    event(&mut app, Event::ConfirmDelete);
    assert_eq!(linked(&app), vec![before[1].clone()], "the child deleted");
    let doc = app.document.as_ref().expect("the drawing");
    assert!(
        doc.model
            .slot_of(kentos_domain::Uuid::parse_str(&before[0]).expect("an id"))
            .is_none()
    );
    assert!(app.form_entry.is_some(), "the window stays");
}

#[test]
fn delete_is_not_offered_unless_named() {
    let mut app = owners(serde_json::Value::Null, false);
    let before = linked(&app);
    event(
        &mut app,
        Event::DeleteRelated("malikler".into(), before[0].clone()),
    );
    event(&mut app, Event::ConfirmDelete);
    assert_eq!(linked(&app).len(), 2);
    assert!(crate::files_testing::last_said(&app).contains("“Sil” sunmuyor"));
}

#[test]
fn a_child_opened_to_read_is_read_only() {
    let mut app = owners(json!(["view", "show"]), false);
    let first = linked(&app)[0].clone();
    event(
        &mut app,
        Event::OpenRelated("malikler".into(), first.clone()),
    );
    let e = app.form_entry.as_ref().expect("the owner's form");
    assert!(e.read_only);
    // Update offered: the same child opens to edit.
    let mut app = owners(json!(["view", "update"]), false);
    event(&mut app, Event::OpenRelated("malikler".into(), first));
    let e = app.form_entry.as_ref().expect("the owner's form");
    assert!(!e.read_only);
    assert!(app.selection.ids() != [Slot(0)]);
}

// A Veri girişi writes a child through the child layer's own form (docs/adr/0764 §4): the owners' form
// does not show their building field, so it neither links nor unlinks; withdrawn, it changes none.
#[test]
fn a_data_entry_writes_children_through_their_own_form() {
    let mut app = crate::files_testing::app_with_drawing();
    crate::form_scenes::actions_entry(&mut app);
    assert_eq!(
        offers(&app),
        vec![
            (A::Create, false),
            (A::Update, true),
            (A::Delete, false),
            (A::Link, false),
            (A::Unlink, false),
            (A::View, true),
            (A::Show, true),
        ]
    );
    let e = app.form_entry.as_ref().expect("the form");
    let link = app
        .related_offers(e, "malikler")
        .into_iter()
        .find(|o| o.action == A::Link)
        .and_then(|o| o.refused)
        .unwrap_or_default();
    assert!(
        link.contains("“Bina” alanı “Malik” katmanının formunda yok"),
        "{link}"
    );
    // The owners' form withdrawn: no child of theirs is changed.
    if let Some(doc) = app.document.as_mut() {
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_FORMS_EDIT,
            kentos_commands::CallOp::Execute,
            json!({ "edit": { "kind": "withdraw", "id": "malik-formu", "on": true } }),
        );
        assert_eq!(
            answer.as_ref().map(|a| a["status"].clone()).ok(),
            Some(json!("completed"))
        );
    }
    assert!(
        offers(&app)
            .iter()
            .filter(|(a, _)| matches!(a, A::Update | A::Link | A::Unlink))
            .all(|(_, ok)| !ok)
    );
}
