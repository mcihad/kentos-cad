//! Formlar on a layer without fields (docs/adr/9141), the municipal report's 8th item: the road layer had no field;
//! Yeni alan in the designer's palette makes `durum` on the layer and puts it on the form at once, the designer staying
//! on the form; a name the layer has already is refused, said.

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
