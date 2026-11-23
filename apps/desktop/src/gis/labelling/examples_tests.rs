//! The labelling window's examples on the municipal report's 10th item (docs/adr/9164): on the same data, the first and
//! the longest text, how many have none, the scales the label shows at with the view's, and its letters on paper.

use kentos_commands::LabelStyle;
use kentos_geometry_core::scale::px_per_metre;

use super::{print_words, scale_words};
use crate::app::{App, Message};
use crate::exchange::words::Kind;
use crate::files_testing::app_with_drawing;
use crate::gis;
use crate::labelling::{Event, form::Key};

const LONG: &str = "Atatürk Bulvarı Kuzey Yan Yolu Kavşağı Doğu Kolu";

/// The sample drawing fitted in a 1000 × 700 view, its Çizim layer's P1 named as before and its second object given a
/// long name, the labels window on the layer with `{Ad}` typed.
fn named() -> App {
    let mut app = app_with_drawing();
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    let _ = app.update(Message::Run("view.zoomExtents"));
    let doc = app.document.as_mut().expect("open");
    let second = doc.model.by_layer("cizim").nth(1).map(|e| kentos_domain::Slot(e.base().id)).expect("a second object");
    let mut e = doc.model.get(second).cloned().expect("it");
    e.base_mut().attrs.insert("Ad".into(), LONG.into());
    assert!(doc.model.update(second, e));
    app.open_labelling(Some("cizim".into()));
    let _ = app.update(Message::Gis(gis::Event::Labelling(Event::Type(Key::Template, "{Ad}".into()))));
    app
}

fn lines(app: &App) -> Vec<String> {
    app.gis.labelling.as_ref().expect("the window").lines.iter().map(|(_, l)| l.clone()).collect()
}

#[test]
fn the_window_shows_the_first_the_longest_and_the_empty_texts_its_scales_and_its_print() {
    let app = named();
    let said = lines(&app);
    let has = |words: &str| said.iter().any(|l| l.contains(words));
    assert!(has("Örnek metin (ilk nesne): “P1”."), "{said:#?}");
    // Cut at 40 letters, its whole length said.
    assert!(has("En uzun metin: “Atatürk Bulvarı Kuzey Yan Yolu Kavşağı D…” (48 harf)."), "{said:#?}");
    // The layer's other objects without a name (its text objects label with their own words).
    assert!(has("İlk 9 nesnenin 4 tanesinde metin boş; onlara etiket yazılmaz."), "{said:#?}");
    // The window's new label: shown from 1:1.890 nearer; the fitted view is nearer still.
    assert!(said.iter().any(|l| l.starts_with("Ölçek: 1:1.890 ve daha yakın ölçeklerde görünür; şimdiki görünüm 1:")
        && l.ends_with(", görünür.")), "{said:#?}");
    assert!(has("Baskı: yazı 10.5 px, kâğıtta yaklaşık 2.8 mm."), "{said:#?}");
}

#[test]
fn the_scales_are_said_as_ranges_with_the_views_and_the_print_in_millimetres() {
    let style = LabelStyle {
        min_scale: Some(px_per_metre(5000.0)),
        max_scale: Some(px_per_metre(500.0)),
        ..LabelStyle::default()
    };
    assert_eq!(
        scale_words(&style, px_per_metre(1000.0)),
        (Kind::Info, "Ölçek: 1:5.000 ile 1:500 arasında görünür; şimdiki görünüm 1:1.000, görünür.".to_owned())
    );
    assert_eq!(
        scale_words(&style, px_per_metre(25_000.0)),
        (Kind::Warn, "Ölçek: 1:5.000 ile 1:500 arasında görünür; şimdiki görünüm 1:25.000, görünmez.".to_owned())
    );
    let far = LabelStyle { min_scale: Some(px_per_metre(2000.0)), ..LabelStyle::default() };
    assert_eq!(
        scale_words(&far, px_per_metre(1000.0)).1,
        "Ölçek: 1:2.000 ve daha yakın ölçeklerde görünür; şimdiki görünüm 1:1.000, görünür."
    );
    // 10 px at 96 a inch: 2.6 mm; one growing with the scale says its most.
    assert_eq!(print_words(&LabelStyle::default()), "Baskı: yazı 10 px, kâğıtta yaklaşık 2.6 mm.");
    let grows = LabelStyle { grow: Some(0.5), max_size: Some(24.0), ..LabelStyle::default() };
    assert_eq!(
        print_words(&grows),
        "Baskı: yazı 10 px, kâğıtta yaklaşık 2.6 mm. Ölçekle büyür, en çok 24 px (6.3 mm)."
    );
}

/// The picture (`.run/gui-evidence/kullanim-9164/`): the labels window's examples on the named layer.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9164", "01-etiket-ornekleri", named);
}
