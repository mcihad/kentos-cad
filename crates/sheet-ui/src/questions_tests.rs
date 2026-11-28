//! Yeni pafta's picture and the template that asks nothing (docs/adr/9187): the window draws the page with the main
//! map's frame and the drawing's box as the choice places them (past the frame at the fixed scale: the cut; inside it
//! once fitted); a template without questions asks Kapsam alone when it has a main map, and one with neither (a print
//! calibration sheet) is made at once.

use kentos_sheet::kinds::{GroundPoint, ItemKind};
use kentos_sheet::profile::Capabilities;

use crate::designer::{Context, Designer, Effect};
use crate::message::Message;
use crate::new_extent::Extent;
use crate::questions::QuestionsMessage;

/// A drawing 600 m across and 300 m down about (500 300, 4 420 150); the drawing area 200 m × 120 m about its middle;
/// a parcel layer of 80 m × 60 m; the plot scale 1:1000 (the extent tests' context).
fn context() -> Context {
    Context {
        capabilities: Capabilities {
            georeferenced: true,
            attribute_layers: true,
            plot_scale: Some(1000),
        },
        center: Some(GroundPoint {
            x: 500_300.0,
            y: 4_420_150.0,
        }),
        view_size: Some([200.0, 120.0]),
        drawing_bounds: Some([500_000.0, 4_420_000.0, 500_600.0, 4_420_300.0]),
        layers: vec![("parsel".into(), "Kadastro / Parsel".into())],
        layer_bounds: vec![(
            "parsel".into(),
            [500_100.0, 4_420_100.0, 500_180.0, 4_420_160.0],
        )],
        ..Context::default()
    }
}

/// Whether `inner` lies within `outer` (left, top, width, height, millimetres).
fn within(inner: [f64; 4], outer: [f64; 4]) -> bool {
    inner[0] >= outer[0] - 1e-9
        && inner[1] >= outer[1] - 1e-9
        && inner[0] + inner[2] <= outer[0] + outer[2] + 1e-9
        && inner[1] + inner[3] <= outer[1] + outer[3] + 1e-9
}

#[test]
fn the_picture_shows_the_main_maps_frame_with_the_cut_or_the_fit() {
    let mut d = Designer::new(context());
    let _ = d.update(Message::NewSheet);
    let p = d
        .questions()
        .expect("the questions")
        .picture
        .clone()
        .expect("a picture");
    assert!(
        within(p.frame, [0.0, 0.0, p.page[0], p.page[1]]),
        "the frame on the page: {p:?}"
    );
    // The fixed scale: 600 m at 1:1000 is 600 mm, past the frame: the cut.
    let drawing = p.drawing.expect("the drawing's box");
    assert!(
        (drawing[2] - 600.0).abs() < 1e-6 && (drawing[3] - 300.0).abs() < 1e-6,
        "{drawing:?}"
    );
    assert!(!within(drawing, p.frame), "the cut shows: {p:?}");
    // The whole drawing fitted: inside the frame, in its middle.
    let _ = d.update(Message::Questions(QuestionsMessage::Extent(
        Extent::Drawing,
    )));
    let p = d
        .questions()
        .expect("the questions")
        .picture
        .clone()
        .expect("a picture");
    let drawing = p.drawing.expect("the drawing's box");
    assert!(within(drawing, p.frame), "fitted: {p:?}");
    let middle = |b: [f64; 4]| (b[0] + b[2] / 2.0, b[1] + b[3] / 2.0);
    let (a, b) = (middle(drawing), middle(p.frame));
    assert!(
        (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6,
        "{a:?} {b:?}"
    );
    // A layer: its box is the chosen one, inside the frame; the drawing past it.
    let _ = d.update(Message::Questions(QuestionsMessage::Extent(Extent::Layer)));
    let p = d
        .questions()
        .expect("the questions")
        .picture
        .clone()
        .expect("a picture");
    assert!(within(p.chosen.expect("the layer's box"), p.frame), "{p:?}");
    assert!(
        !within(p.drawing.expect("the drawing's box"), p.frame),
        "{p:?}"
    );
}

#[test]
fn a_template_that_asks_nothing_asks_kapsam_alone_and_a_calibration_sheet_is_made_at_once() {
    let mut d = Designer::new(context());
    let _ = d.update(Message::NewSheet);
    let mut t = d.questions().expect("the questions").template.clone();
    d.dialog = None;
    t.variables.clear();
    // No question, a main map: Kapsam alone, nothing made yet.
    let effects = d.ask_then_use(&t, None, "Yeni pafta".into());
    assert!(effects.is_empty(), "{effects:?}");
    let q = d.questions().expect("Kapsam alone");
    assert_eq!(
        (q.extent, q.template.variables.len()),
        (Some(Extent::Fixed), 0)
    );
    assert!(q.picture.is_some());
    assert_eq!(
        d.update(Message::Questions(QuestionsMessage::Make)),
        [Effect::ShowSheet]
    );
    // No question and no map (a print calibration sheet): made at once.
    let mut calibration = t.clone();
    // (Its texts and border alone: what tied to a map goes with the maps.)
    calibration
        .sheet
        .items
        .retain(|it| matches!(it.kind, ItemKind::Text(_) | ItemKind::Border(_)));
    calibration.sheet.variants.clear();
    calibration.sheet.active_variant = None;
    calibration.sheet.base_layout = None;
    calibration.sheet.atlas = None;
    calibration.master = None;
    assert!(!crate::new_extent::has_main_map(&calibration));
    d.dialog = None;
    let made = d.ask_then_use(&calibration, None, "Kalibrasyon".into());
    assert_eq!(made, [Effect::ShowSheet], "{:?}", d.error);
    assert!(d.questions().is_none());
}
