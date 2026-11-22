//! Katman stili on a data layer (docs/adr/9110 §1): Değerlerden sınıfla and Sınıfla take the field's values from the
//! source, as the drawing holds none of its rows; the Nesne column counts the rows by the source's groups. The read runs
//! off the interface's thread in the window; here it is read on the test's and handed back, as the window does.

use super::source_values::{self, Then};
use super::{Event, Kind};
use crate::app::{App, Message};

fn opened() -> (App, std::path::PathBuf) {
    crate::data_layers::register();
    let path = crate::data_layers_tests::parcels("stil-kaynak", 41);
    let described = crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described");
    let mut app = crate::files_testing::app_with_drawing();
    let added = app.add_data_layers(vec![described]).expect("added");
    let _ = app.update(Message::LayerStyle(Event::Open(Some(added[0].clone()))));
    (app, path)
}

fn hand_back(app: &mut App, field: &str, then: Then) {
    let data = app.styles.layer_style.as_ref().and_then(|w| w.data.clone()).expect("a data layer's window");
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
    let values: Vec<&str> = w.categorized.categories.iter().map(|k| k.value.as_str()).collect();
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
    assert_eq!(w.said.as_ref().map(|(t, _)| t.as_str()), Some(source_values::FIELD_ONLY));
    let _ = std::fs::remove_file(path);
}
