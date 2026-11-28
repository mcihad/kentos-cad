//! The municipal report's 14th item over the shared topology drawing (`fixtures/interaction/v1/topology-rules.kcad`,
//! docs/adr/9146): parcels #2 and #3 overlap by a 0.4 m strip, road #18 stops 0.6 m short of road #17, and road #16's
//! west end at the sheet's edge is made an exception with its reason. Every finding is gone to; the overlap is fixed,
//! undone and checked again; a label change leaves the findings as they are while a move or a tolerance change makes
//! them old; the tab says what the check covered.

use crate::app::{App, Message};
use crate::gis;

const DRAWING: &str = include_str!("../../../../../fixtures/interaction/v1/topology-rules.kcad");

/// The drawing in a 1000 × 700 view, checked.
fn checked() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    let _ = app.update(Message::Run("topology.check"));
    app
}

fn run(app: &mut App, id: &str, input: serde_json::Value) {
    let doc = &mut app.document.as_mut().expect("open").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        id,
        kentos_commands::CallOp::Execute,
        input,
    );
    assert!(answer.is_ok(), "{id}: {answer:?}");
}

/// The pictures (`.run/gui-evidence/kullanim-9146/`): the rules window saying each length's unit and comparison with a
/// profile, and the tab saying what the check covered after a label change left its findings as they were.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9146", "01-tolerans-aciklamasi", || {
        let mut app = checked();
        let _ = app.update(Message::Run("topology.rules"));
        for e in [
            super::rules::Event::Profile("Belediye CBS".into()),
            super::rules::Event::Precision("0.0001".into()),
            super::rules::Event::Acceptance("0.05".into()),
        ] {
            let _ = app.update(Message::Gis(gis::Event::TopologyRules(e)));
        }
        app
    });
    crate::files_testing::usage_shots("9146", "02-kapsam-etiket-sonrasi", || {
        let mut app = checked();
        run(
            &mut app,
            "cad.layers.labels",
            serde_json::json!({ "layer": "parsel", "label": { "placement": "center", "size": 11.0, "template": "{label}" } }),
        );
        app
    });
}
