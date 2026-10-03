//! The pictures of docs/adr/0148 §9, Topolojik temizlik over the trace's
//! drawing (`fixtures/interaction/v1/topology.kcad`: a parcel block whose line
//! ends almost meet, a survey point, a locked road and two neighbouring
//! areas): Değiştir › Nesne ▾ with the tool beside Çizimi temizle; the
//! finding at 0.05 m with Köşeler on, the whole block; then close up, at
//! 2 500 px a metre, the node three ends go to, the end cut where it runs
//! past and the end extended to the road. The web's are `shots.mjs topology`
//! (`topology-list`, `topology-preview`, `topology-node`, `topology-trim`,
//! `topology-extend`). `tools_screens` takes them in the dark and the light
//! theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=topoloji-dugum cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_render_wgpu::Vec2;

use crate::app::{App, Message};
use crate::document::Document;
use crate::tools_scenes::{E, N, hover, run, typed};
use crate::tools_screens::{Pointed, Scene};

const DRAWING: &str = include_str!("../../../fixtures/interaction/v1/topology.kcad");

/// Opens the trace's drawing, as the app would open the file: the view fits it.
fn open(app: &mut App) {
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(DRAWING).expect("reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

/// The tool at 0.05 m with Köşeler on, as the trace leaves it before Enter.
fn found(app: &mut App) {
    open(app);
    run(app, "tool.topology");
    typed(app, "0.05");
    typed(app, "K");
}

/// The view `scale` px a metre about a point (east and north from the drawing's origin),
/// the lines as hairlines: close up, a 0.25 mm weight at 1:1000 would be hundreds of pixels wide.
fn look(app: &mut App, at: [f64; 2], scale: f64) {
    app.viewport.camera.center = Vec2::new(E + at[0], N + at[1]);
    app.viewport.camera.scale = scale;
    run(app, "view.lineWeights");
}

/// Close up: 2 500 px a metre, so that a centimetre is 25 px.
const CLOSE: f64 = 2500.0;

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("topoloji-onizleme", |app| {
            found(app);
            hover(app, [-4.0, -14.0]);
        }),
        // The block's south-west node at (0, 0): the line from the west ends there, the one
        // to the east starts 6 mm off and the divider stops 3 cm short.
        ("topoloji-dugum", |app| {
            found(app);
            look(app, [0.01, 0.0], CLOSE);
            hover(app, [-0.11, -0.05]);
        }),
        // The divider at x = −10 runs 2 cm past the north edge: cut where it crosses.
        ("topoloji-buda", |app| {
            found(app);
            look(app, [-10.0, 15.0], CLOSE);
            hover(app, [-10.12, 14.95]);
        }),
        // The stub at x = 10 stops 3 cm short of the locked road: extended along its edge.
        ("topoloji-uzat", |app| {
            found(app);
            look(app, [10.0, -9.99], CLOSE);
            hover(app, [9.88, -10.04]);
        }),
    ]
}

/// Düzenle › Nesne ▾ (a CBS project's; a CAD project's Değiştir): the panel's seldom used
/// tools, Topolojik temizlik beside Çizimi temizle.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![(
        "topoloji-liste",
        |app| app.tab = "edit",
        |s, app| {
            use iced::futures::StreamExt as _;
            let task: iced::Task<Message> =
                kentos_ui::widget::context_menu::open_menu(crate::ribbon_keys::more_id("Nesne"));
            if let Some(mut stream) = iced_runtime::task::into_stream(task) {
                while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                    if let iced_runtime::Action::Widget(operation) = action {
                        s.operate(app.view(), operation);
                    }
                }
            }
        },
    )]
}
