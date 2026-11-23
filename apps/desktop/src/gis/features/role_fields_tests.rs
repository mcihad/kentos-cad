//! The municipal report's 29th item (docs/adr/9150): the buildings' published form shows Durum, Kat, Açıklama, Tarih
//! and Onaylı; a Not field it does not show holds “Teknik ekip notu” on the second building. One who works through forms
//! (a Veri girişi) has no Not column in the attribute table, the table's foot names it as not read, and Bilgi's card
//! leaves it out and names it; anyone else sees the column and the value.

use crate::app::{App, Message};
use crate::form_scenes::buildings;
use kentos_domain::{ObjectPolicy, Slot};
use kentos_native_application::ExecutionContext;

/// The buildings with their published form and a Not field the form does not show, written on the second building.
fn with_note() -> App {
    let mut app = App::boot(None).0;
    buildings(&mut app, true);
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc.model.layers().get("bina").expect("the layer").fields.clone();
    fields.push(kentos_commands::LayerField::new("Not", kentos_commands::LayerFieldKind::Text));
    let mut input = kentos_commands::LayersEdit::new(kentos_commands::LayerEditOperation::Fields);
    input.layer = Some("bina".into());
    input.fields = Some(fields);
    let done = kentos_native_application::layers_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    let mut e = doc.model.get(Slot(2)).cloned().expect("the second building");
    e.base_mut().attrs.insert("Not".into(), "Teknik ekip notu".into());
    assert!(doc.model.update(Slot(2), e));
    app.gis.features.reset(None);
    let _ = app.update(Message::Run("data.featureTable"));
    app
}

fn headers(app: &App) -> Vec<String> {
    app.features_seen().map(|(_, h, _)| h).unwrap_or_default()
}

#[test]
fn a_field_the_role_does_not_read_is_named_not_shown_empty() {
    // The owner: the column and its value.
    let mut app = with_note();
    assert!(headers(&app).iter().any(|h| h == "Not"), "{:?}", headers(&app));
    assert!(app.withheld_fields("bina").is_empty());
    // A Veri girişi: no column, the foot names it, the card leaves it out and names it.
    app.document.as_mut().expect("a drawing").model.set_object_policy(ObjectPolicy::AttributesOnly);
    app.gis.features.version += 1;
    assert_eq!(app.withheld_fields("bina"), ["Not"]);
    assert!(!headers(&app).iter().any(|h| h == "Not"), "{:?}", headers(&app));
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(app.feature_rows(doc).withheld, ["Not"]);
    assert_eq!(
        super::withheld_words(&["Not".to_owned()]),
        "Rolünüz bu alanları okumuyor, gösterilmiyor: Not"
    );
    let card = app.slot_card(Slot(2)).expect("its card");
    assert!(!card.rows.iter().any(|(k, _)| k == "Not"), "{:?}", card.rows);
    assert!(card.rows.iter().any(|(k, v)| k == "Rolünüz okumuyor" && v == "Not"), "{:?}", card.rows);
    // A code of the value list by its label, as the table shows it (docs/adr/9194).
    assert!(card.rows.iter().any(|(k, v)| k == "Durum" && v == "İyi"), "{:?}", card.rows);
}

/// The picture (`.run/gui-evidence/kullanim-9150/`): a Veri girişi's attribute table with no Not column, its foot
/// naming it.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9150", "01-okunmayan-alan", || {
        let mut app = with_note();
        app.document.as_mut().expect("a drawing").model.set_object_policy(ObjectPolicy::AttributesOnly);
        app.gis.features.version += 1;
        app
    });
}
