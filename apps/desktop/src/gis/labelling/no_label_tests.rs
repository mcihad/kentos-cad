//! The label check and Bilgi's card on the municipal report's 13th item (docs/adr/9145): the window samples the text
//! the first object takes and names a template's field the layer does not have; Denetle counts the candidates before
//! the engine and says each reason apart (an empty field, the scale, the least object, the view); Alan ekle writes a
//! field into the template. Bilgi's card is info_card_tests.rs.

use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;
use crate::gis;
use crate::labelling::{Event, form::Key};
use kentos_native_application::ExecutionContext;

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Labelling(e)));
}

fn lines(app: &App) -> Vec<String> {
    app.gis.labelling.as_ref().expect("the window").lines.iter().map(|(_, l)| l.clone()).collect()
}

fn has(lines: &[String], words: &str) -> bool {
    lines.iter().any(|l| l.contains(words))
}

/// The sample drawing in a 1000 × 700 view fitted to it, the labels window open on its one parcel.
fn on_parcels() -> App {
    let mut app = app_with_drawing();
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    let _ = app.update(Message::Run("view.zoomExtents"));
    let _ = app.open_labelling(Some("parsel".into()));
    app
}

/// The parcel layer given a declared `durum` field no parcel has a value of.
fn declare_durum(app: &mut App) {
    let doc = app.document.as_mut().expect("open");
    let mut input = kentos_commands::LayersEdit::new(kentos_commands::LayerEditOperation::Fields);
    input.layer = Some("parsel".into());
    input.fields = Some(vec![kentos_commands::LayerField::new("durum", kentos_commands::LayerFieldKind::Text)]);
    let done = kentos_native_application::layers_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
}

#[test]
fn a_field_the_layer_does_not_have_is_said_and_shows_as_typed() {
    let mut app = on_parcels();
    ev(&mut app, Event::Type(Key::Template, "{durum}".into()));
    let said = lines(&app);
    assert!(has(&said, "Örnek metin (ilk nesne): “{durum}”."), "{said:#?}");
    assert!(
        has(&said, "Şablondaki {durum} katmanın alanı değil: etikette yazıldığı gibi çıkar. Katmanın alanları: Ada, Parsel, Tapu alanı."),
        "{said:#?}"
    );
    ev(&mut app, Event::Report);
    let said = lines(&app);
    assert!(has(&said, "Adaylar: görünümde 1 nesne, etiket metni olan 1; örnek metin “{durum}”."), "{said:#?}");
    assert!(has(&said, "Şablondaki {durum} katmanın alanı değil"), "{said:#?}");
}

#[test]
fn an_empty_field_the_scale_the_least_object_and_the_view_are_said_apart() {
    let mut app = on_parcels();
    declare_durum(&mut app);
    // A declared field empty on every parcel: no candidate, and why.
    ev(&mut app, Event::Type(Key::Template, "{durum}".into()));
    assert!(has(&lines(&app), "İlk 1 nesnenin etiket metni boş."), "{:#?}", lines(&app));
    ev(&mut app, Event::Report);
    let said = lines(&app);
    assert!(has(&said, "Görünümde 0 etiket yerleşti"), "{said:#?}");
    assert!(has(&said, "Etiket adayı yok: şablondaki {durum} görünümdeki 1 nesnenin hepsinde boş."), "{said:#?}");
    // The parcel's own values: the sample is its text.
    ev(&mut app, Event::Type(Key::Template, "{Ada}/{Parsel}".into()));
    assert!(has(&lines(&app), "Örnek metin (ilk nesne): “101/7”."), "{:#?}", lines(&app));
    // The farthest end nearer than the view: out of the scale range.
    ev(&mut app, Event::Type(Key::MinScale, "1".into()));
    ev(&mut app, Event::Report);
    let said = lines(&app);
    assert!(has(&said, "1 aday bu ölçekte gösterilmez: görünüm 1:"), "{said:#?}");
    assert!(has(&said, "en uzak 1:1, en yakın sınırsız"), "{said:#?}");
    // In range, but smaller than the least object.
    ev(&mut app, Event::Type(Key::MinScale, String::new()));
    ev(&mut app, Event::Type(Key::MinFeature, "100000".into()));
    ev(&mut app, Event::Report);
    let said = lines(&app);
    assert!(has(&said, "1 aday en küçük nesne sınırından (100000 px) küçük"), "{said:#?}");
    assert!(!has(&said, "bu ölçekte gösterilmez"), "{said:#?}");
    // The view far from every parcel.
    app.viewport.camera.center.x = 0.0;
    app.viewport.camera.center.y = 0.0;
    ev(&mut app, Event::Report);
    let said = lines(&app);
    assert!(has(&said, "katmanın 1 nesnesinin hiçbiri görünür alanda değil"), "{said:#?}");
}

#[test]
fn alan_ekle_writes_the_field_into_the_template() {
    let mut app = on_parcels();
    let fields = app.gis.labelling.as_ref().expect("the window").fields.clone();
    assert_eq!(fields, ["Ada", "Parsel", "Tapu alanı"]);
    ev(&mut app, Event::Type(Key::Template, String::new()));
    ev(&mut app, Event::Field(0));
    ev(&mut app, Event::Field(1));
    let w = app.gis.labelling.as_ref().expect("the window");
    assert_eq!(w.single.template.as_deref(), Some("{Ada} {Parsel}"));
    assert!(has(&lines(&app), "Örnek metin (ilk nesne): “101 7”."), "{:#?}", lines(&app));
    // A trace reaches it by the field's name.
    let m = app.labelling_control(crate::traces::Control::Fill("Alan ekle", "Tapu alanı")).expect("the picker");
    assert!(m.is_some());
}

/// The labels window's pictures (`.run/gui-evidence/kullanim-9145/`): Alan ekle beside the template with the sample, and
/// Denetle's candidates and reason over an empty declared field.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9145", "01-alan-ekle-ornek", || {
        let mut app = on_parcels();
        ev(&mut app, Event::Type(Key::Template, "{Ada}/{Parsel} {durum}".into()));
        app
    });
    crate::files_testing::usage_shots("9145", "02-denetle-nedenler", || {
        let mut app = on_parcels();
        declare_durum(&mut app);
        ev(&mut app, Event::Type(Key::Template, "{durum}".into()));
        ev(&mut app, Event::Report);
        app
    });
}
