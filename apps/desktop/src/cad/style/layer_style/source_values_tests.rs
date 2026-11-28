//! Katman stili on a data layer (docs/adr/9110 §1): Değerlerden sınıfla and Sınıfla take the field's values from the
//! source, as the drawing holds none of its rows; the Nesne column counts the rows by the source's groups. The read runs
//! off the interface's thread in the window; here it is read on the test's and handed back, as the window does.

use super::source_values::{self, Then};
use super::{Event, Kind};
use crate::app::{App, Message};

fn opened() -> (App, std::path::PathBuf) {
    crate::data_layers::register();
    let path = crate::data_layers_tests::parcels("stil-kaynak", 41);
    let described =
        crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described");
    let mut app = crate::files_testing::app_with_drawing();
    let added = app.add_data_layers(vec![described]).expect("added");
    let _ = app.update(Message::LayerStyle(Event::Open(Some(added[0].clone()))));
    (app, path)
}

fn hand_back(app: &mut App, field: &str, then: Then) {
    let data = app
        .styles
        .layer_style
        .as_ref()
        .and_then(|w| w.data.clone())
        .expect("a data layer's window");
    let read = source_values::read(&data, field, then);
    let _ = app.update(Message::LayerStyle(Event::SourceValues(then, read)));
}

#[test]
fn a_data_layers_categories_and_classes_come_from_its_source() {
    let (mut app, path) = opened();
    let w = app.styles.layer_style.as_ref().expect("the window");
    assert_eq!(w.categorized.expr, "kullanim", "a data layer's text field");
    assert_eq!(w.graduated.expr, "ada", "and its number field");
    let _ = app.update(Message::LayerStyle(Event::Kind(Kind::Categorized)));
    // Değerlerden sınıfla asks the source: nothing is classified until its values come.
    let _ = app.update(Message::LayerStyle(Event::Classify));
    let w = app.styles.layer_style.as_ref().expect("the window");
    assert!(w.reading && w.categorized.categories.is_empty());
    hand_back(&mut app, "kullanim", Then::Classify);
    let w = app.styles.layer_style.as_ref().expect("the window");
    let values: Vec<&str> = w
        .categorized
        .categories
        .iter()
        .map(|k| k.value.as_str())
        .collect();
    assert_eq!(values, ["Konut", "Park"]);
    // The Nesne column: the rows each category takes, as the source counts them (41 rows: 21 Konut, 20 Park).
    assert_eq!(w.source_category_tally(), Some((vec![21, 20], 0)));

    // Sınıfla: the classes from the ada numbers the source holds (100, 101, 102: 20, 20 and 1 rows).
    let _ = app.update(Message::LayerStyle(Event::Kind(Kind::Graduated)));
    let _ = app.update(Message::LayerStyle(Event::Graduate));
    hand_back(&mut app, "ada", Then::Graduate);
    let w = app.styles.layer_style.as_ref().expect("the window");
    assert!(!w.graduated.classes.is_empty(), "classes made");
    let (counts, rest) = w.source_class_tally().expect("counted by the source");
    assert_eq!(counts.iter().sum::<usize>() + rest, 41, "{counts:?} {rest}");
    assert_eq!(rest, 0, "every number in a class");

    // A field the table has not is not asked: the window says why.
    let w = app.styles.layer_style.as_mut().expect("the window");
    w.categorized.expr = "$alan".into();
    let _ = app.update(Message::LayerStyle(Event::Classify));
    let w = app.styles.layer_style.as_ref().expect("the window");
    assert_eq!(
        w.said.as_ref().map(|(t, _)| t.as_str()),
        Some(source_values::FIELD_ONLY)
    );
    let _ = std::fs::remove_file(path);
}

/// The second round's T23 and T26 (docs/adr/9357) on the municipal `bakim` (mahalle M1 ×2, M2 ×2, M9, one without):
/// the categories of a listed field are named by its labels, its codes still what matches, an unknown code as it is;
/// the row without a value has a category of its own, Değer yok. The window's path is copied.
#[test]
fn listed_codes_are_named_by_their_labels_and_rows_without_a_value_have_a_category() {
    crate::data_layers::register();
    let path = crate::data_layers_tests::municipal("stil-liste");
    let mut app = crate::files_testing::app_with_drawing();
    let id = app
        .add_data_layers(vec![
            crate::data_layers::describe(&path.display().to_string(), "bakim").expect("described"),
        ])
        .expect("added")
        .remove(0);
    {
        let doc = app.document.as_mut().expect("open");
        let mut fields = doc
            .model
            .layers()
            .get(&id)
            .expect("the layer")
            .fields
            .clone();
        let f = fields
            .iter_mut()
            .find(|f| f.name == "mahalle")
            .expect("mahalle");
        f.values = Some(vec![
            kentos_commands::FieldChoice::new("M1", "Çınar"),
            kentos_commands::FieldChoice::new("M2", "İstasyon"),
        ]);
        doc.model.set_layer_fields(&id, fields, &[]).expect("set");
    }
    let _ = app.update(Message::LayerStyle(Event::Open(Some(id.clone()))));
    let _ = app.update(Message::LayerStyle(Event::Kind(Kind::Categorized)));
    app.styles
        .layer_style
        .as_mut()
        .expect("the window")
        .categorized
        .expr = "mahalle".into();
    hand_back(&mut app, "mahalle", Then::Classify);
    let w = app.styles.layer_style.as_ref().expect("the window");
    let got: Vec<(&str, &str)> = w
        .categorized
        .categories
        .iter()
        .map(|k| (k.value.as_str(), k.label.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("M1", "Çınar"),
            ("M2", "İstasyon"),
            ("M9", "M9"),
            ("", super::NO_VALUE)
        ]
    );
    // Every row drawn: nothing left to Diğer değerler.
    assert_eq!(w.source_category_tally(), Some((vec![2, 2, 1, 1], 0)));
    // Its path, copied.
    let _ = app.update(Message::LayerStyle(Event::CopyPath));
    let said = crate::files_testing::last_said(&app);
    assert_eq!(said, "Katmanın yolu panoya kopyalandı: bakim");
}

/// The picture of T23 and T26 (`.run/gui-evidence/kullanim-9357/03-stil-deger-yok`): the categories named by the value
/// list with Değer yok, the window's layer by its path and source.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib source_values_tests::screens_9357 -- --ignored --nocapture"]
fn screens_9357() {
    crate::files_testing::usage_shots("9357", "03-stil-deger-yok", || {
        crate::data_layers::register();
        let path = crate::data_layers_tests::municipal("stil-liste-resim");
        let mut app = crate::files_testing::app_with_drawing();
        let id = app
            .add_data_layers(vec![
                crate::data_layers::describe(&path.display().to_string(), "bakim")
                    .expect("described"),
            ])
            .expect("added")
            .remove(0);
        {
            let doc = app.document.as_mut().expect("open");
            let mut fields = doc
                .model
                .layers()
                .get(&id)
                .expect("the layer")
                .fields
                .clone();
            let f = fields
                .iter_mut()
                .find(|f| f.name == "mahalle")
                .expect("mahalle");
            f.values = Some(vec![
                kentos_commands::FieldChoice::new("M1", "Çınar"),
                kentos_commands::FieldChoice::new("M2", "İstasyon"),
            ]);
            doc.model.set_layer_fields(&id, fields, &[]).expect("set");
        }
        let _ = app.update(Message::LayerStyle(Event::Open(Some(id.clone()))));
        let _ = app.update(Message::LayerStyle(Event::Kind(Kind::Categorized)));
        app.styles
            .layer_style
            .as_mut()
            .expect("the window")
            .categorized
            .expr = "mahalle".into();
        hand_back(&mut app, "mahalle", Then::Classify);
        app
    });
}
