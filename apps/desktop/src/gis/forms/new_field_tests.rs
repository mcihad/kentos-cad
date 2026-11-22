//! Formlar on a layer without fields (docs/adr/9141), the municipal report's 8th item: the road layer had no field;
//! Yeni alan in the designer's palette makes `durum` on the layer and puts it on the form at once, the designer staying
//! on the form; a name the layer has already is refused, said. Yayınla's check says what keeps the form back kind by
//! kind (docs/adr/9161).

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive};
use crate::forms::designer::Event;
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Forms(e)));
    drive(app, task);
}

/// A layer of the test's own with no field, and the designer on a new form of it.
fn opened() -> (App, String) {
    let mut app = app_with_drawing();
    let input = serde_json::json!({ "operation": "add", "name": "Yol" });
    let doc = app.document.as_mut().expect("open");
    let done = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.layers.edit",
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    let layer = done["output"]["layer"].as_str().expect("the layer").to_owned();
    let task = app.update(Message::Run("data.forms"));
    drive(&mut app, task);
    ev(&mut app, Event::NewForm);
    ev(&mut app, Event::NewLayer(layer.clone()));
    ev(&mut app, Event::NewName("Yol formu".into()));
    ev(&mut app, Event::Create);
    (app, layer)
}

#[test]
fn yeni_alan_makes_the_layers_field_and_puts_it_on_the_form() {
    let (mut app, layer) = opened();
    assert!(app.layer_fields_of(&layer).is_empty(), "the layer has no field");
    ev(&mut app, Event::NewFieldName("durum".into()));
    ev(&mut app, Event::CreateField);
    // The layer's field, and the form's item bound to it: the designer stays on the form.
    assert_eq!(app.layer_fields_of(&layer).iter().map(|f| f.name.clone()).collect::<Vec<_>>(), ["durum"]);
    let w = app.gis.forms_window.as_ref().expect("the designer stays open");
    let body = w.body.as_ref().expect("the form's draft");
    assert!(
        body.walk().iter().any(|(_, i)| matches!(i, kentos_commands::FormItem::Field(f) if f.field == "durum")),
        "on the form"
    );
    assert!(w.new_field.is_empty(), "the name typed is cleared");
    // The same name again: refused, said.
    ev(&mut app, Event::NewFieldName("durum".into()));
    ev(&mut app, Event::CreateField);
    assert_eq!(app.layer_fields_of(&layer).len(), 1);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|t| t.contains("“durum” alanı katmanda var")), "{said:#?}");
}

/// The layer's fields durum (required), tur (Konut, Ticaret) and tip (no list); the draft's items tur with the default
/// “Köşk” and tip as a list, durum left out.
fn three_kinds(app: &mut App, layer: &str) {
    use kentos_commands::{FieldChoice, LayerField, LayerFieldKind};
    let choice = |code: &str| FieldChoice::new(code, code);
    let mut durum = LayerField::new("durum", LayerFieldKind::Text);
    durum.required = true;
    let mut tur = LayerField::new("tur", LayerFieldKind::Text);
    tur.values = Some(vec![choice("Konut"), choice("Ticaret")]);
    let tip = LayerField::new("tip", LayerFieldKind::Text);
    let doc = app.document.as_mut().expect("open");
    assert_eq!(doc.model.set_layer_fields(layer, vec![durum, tur, tip], &[]), Ok(true));
    let items: kentos_commands::FormBody = serde_json::from_value(serde_json::json!({ "revision": 1, "items": [
        { "type": "field", "id": "tur", "field": "tur", "default": "Köşk" },
        { "type": "field", "id": "tip", "field": "tip", "widget": "list" }
    ] }))
    .expect("a body");
    let w = app.gis.forms_window.as_mut().expect("the designer");
    let body = w.body.as_mut().expect("the draft");
    body.items = items.items;
    w.dirty = true;
}

/// Yayınla's check kind by kind (docs/adr/9161), the municipal report's 8th item: an empty form keeps it back, said as
/// “Boş form”; a required field left out, a default that is no value of its list and a list widget on a field without
/// values are each said under their heading in one refusal; nothing is published.
#[test]
fn yayinla_says_what_keeps_the_form_back_kind_by_kind() {
    let (mut app, layer) = opened();
    ev(&mut app, Event::Publish);
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("formu yayınlanamıyor. Boş form: Formda alan yok; kayıt bu formla düzenlenemez."), "{said}");
    three_kinds(&mut app, &layer);
    ev(&mut app, Event::Publish);
    let said = crate::files_testing::last_said(&app);
    let at = |words: &str| said.find(words).unwrap_or_else(|| panic!("“{words}” not in: {said}"));
    let (required, default, list) =
        (at("Zorunlu alan: “durum” alanı katmanda zorunlu ama formda yok"), at("Varsayılan değer: "), at("Değer listesi: "));
    assert!(required < default && default < list, "in their order: {said}");
    assert!(!said.contains("Boş form"), "{said}");
    assert!(app.chosen_form().expect("the form").releases.is_empty(), "nothing published");
}

/// Pictures (docs/adr/9141), light at 1440×900 and dark at 1100×760: Formlar on the layer without fields, its palette
/// saying so with Yeni alan; and after Oluştur, `durum` on the form.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib new_field_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::files_testing::usage_shots("9141", "01-alan-yok", || {
        let (mut app, _) = opened();
        ev(&mut app, Event::NewFieldName("durum".into()));
        app
    });
    crate::files_testing::usage_shots("9141", "02-alan-formda", || {
        let (mut app, _) = opened();
        ev(&mut app, Event::NewFieldName("durum".into()));
        ev(&mut app, Event::CreateField);
        app
    });
}

/// The picture (docs/adr/9161, `.run/gui-evidence/kullanim-9161/`): the designer's issues under their headings and the
/// footer saying what keeps Yayınla back.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib new_field_tests::publish_screens -- --ignored --nocapture"]
fn publish_screens() {
    crate::files_testing::usage_shots("9161", "01-yayin-denetimi", || {
        let (mut app, layer) = opened();
        three_kinds(&mut app, &layer);
        app
    });
}
