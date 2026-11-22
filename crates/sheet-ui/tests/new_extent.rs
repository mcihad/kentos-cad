//! Yeni pafta's extent (docs/adr/9147), as the desktop drives it: the window of the sheet's questions offers where
//! the main map looks. The fixed scale (what a new sheet did before) says the scale the whole drawing needs and keeps
//! the cut; Çizimin tamamı and Katman fit their box at the next standard scale, Görünüm the drawing area; the sheet is
//! made with the view the window said.

use kentos_sheet::kinds::{GroundPoint, ItemKind, MapView};
use kentos_sheet::profile::Capabilities;
use kentos_sheet_ui::new_extent::{Extent, scale_words};
use kentos_sheet_ui::questions::QuestionsMessage;
use kentos_sheet_ui::{Context, Designer, Effect, Message};

/// A drawing 600 m across and 300 m down about (500 300, 4 420 150); the drawing area 200 m × 120 m about its
/// middle; a parcel layer of 80 m × 60 m; the plot scale 1:1000.
fn context() -> Context {
    Context {
        capabilities: Capabilities { georeferenced: true, attribute_layers: true, plot_scale: Some(1000) },
        center: Some(GroundPoint { x: 500_300.0, y: 4_420_150.0 }),
        view_size: Some([200.0, 120.0]),
        drawing_bounds: Some([500_000.0, 4_420_000.0, 500_600.0, 4_420_300.0]),
        layers: vec![("parsel".into(), "Kadastro / Parsel".into()), ("yol".into(), "Yol".into())],
        layer_bounds: vec![("parsel".into(), [500_100.0, 4_420_100.0, 500_180.0, 4_420_160.0])],
        ..Context::default()
    }
}

fn words(d: &Designer) -> String {
    d.questions().expect("the questions").placed.1.clone()
}

/// The new sheet's main map: its centre and scale.
fn main_view(d: &Designer) -> (GroundPoint, u32) {
    let sheet = d.open_sheet().expect("a sheet");
    sheet
        .items
        .iter()
        .find_map(|it| match &it.kind {
            ItemKind::Map(m) if m.overview_of.is_none() && m.source.is_drawing() => match &m.view {
                MapView::Fixed(v) => v.center.map(|c| (c, v.scale)),
                _ => None,
            },
            _ => None,
        })
        .expect("a main map")
}

fn ask(d: &mut Designer, m: QuestionsMessage) {
    let _ = d.update(Message::Questions(m));
}

#[test]
fn the_fixed_scale_says_the_cut_and_what_the_whole_drawing_needs() {
    let mut d = Designer::new(context());
    assert!(d.update(Message::NewSheet).is_empty(), "the questions first");
    let q = d.questions().expect("the questions");
    assert_eq!(q.extent, Some(Extent::Fixed), "as before: the plot scale at the drawing area's centre");
    let said = words(&d);
    assert!(said.starts_with("Ana harita 1:1.000, çizim alanının ortasında."), "{said}");
    assert!(said.contains("Çizimin bir kısmı dışarıda kalır; tamamı 1:"), "{said}");
    assert!(said.ends_with("Kırpma bilerek korunur."), "{said}");
    // Made as it is: the cut kept.
    assert_eq!(d.update(Message::Questions(QuestionsMessage::Make)), [Effect::ShowSheet]);
    let (c, scale) = main_view(&d);
    assert_eq!((c.x, c.y, scale), (500_300.0, 4_420_150.0, 1000));
}

#[test]
fn the_whole_drawing_a_layer_and_the_view_are_fitted_at_a_standard_scale() {
    let mut d = Designer::new(context());
    let _ = d.update(Message::NewSheet);
    // The whole drawing.
    ask(&mut d, QuestionsMessage::Extent(Extent::Drawing));
    let (placed, said) = d.questions().expect("the questions").placed.clone();
    let scale = placed.scale.expect("a scale");
    assert!(kentos_sheet::paper::is_standard_scale(scale), "{scale}");
    assert!(scale > 1000, "600 m does not go on the frame at 1:1000: {scale}");
    assert_eq!(said, format!("Çizimin tamamı %5 payla sığar: ana harita {}.", scale_words(scale)));
    assert_eq!(placed.center.map(|c| (c.x, c.y)), Some((500_300.0, 4_420_150.0)));
    // A layer: its own box, a nearer scale.
    ask(&mut d, QuestionsMessage::Extent(Extent::Layer));
    assert_eq!(d.questions().expect("the questions").layer, Some(0), "the first layer with objects");
    let (layer, said) = d.questions().expect("the questions").placed.clone();
    assert!(said.starts_with("Kadastro / Parsel katmanının nesneleri %5 payla sığar: ana harita 1:"), "{said}");
    assert_eq!(layer.center.map(|c| (c.x, c.y)), Some((500_140.0, 4_420_130.0)));
    assert!(layer.scale.expect("a scale") < scale, "{layer:?} against {scale}");
    // The drawing area.
    ask(&mut d, QuestionsMessage::Extent(Extent::View));
    let (view, said) = d.questions().expect("the questions").placed.clone();
    assert!(said.starts_with("Görünümdeki alan %5 payla sığar: ana harita 1:"), "{said}");
    assert_eq!(view.center.map(|c| (c.x, c.y)), Some((500_300.0, 4_420_150.0)));
    // Made with the whole drawing: the main map is what the window said.
    ask(&mut d, QuestionsMessage::Extent(Extent::Drawing));
    assert_eq!(d.update(Message::Questions(QuestionsMessage::Make)), [Effect::ShowSheet]);
    let (c, made) = main_view(&d);
    assert_eq!((c.x, c.y, made), (500_300.0, 4_420_150.0, scale));
}

#[test]
fn a_choice_with_nothing_to_fit_keeps_the_fixed_placement_and_says_why() {
    let mut d = Designer::new(Context { layer_bounds: Vec::new(), drawing_bounds: None, ..context() });
    let _ = d.update(Message::NewSheet);
    ask(&mut d, QuestionsMessage::Extent(Extent::Layer));
    assert_eq!(words(&d), "Bir katman seçin.");
    ask(&mut d, QuestionsMessage::Extent(Extent::Drawing));
    assert!(words(&d).starts_with("Çizimin tamamı sığdırılamadı"), "{}", words(&d));
}
