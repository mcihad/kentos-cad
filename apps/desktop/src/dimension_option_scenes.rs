//! The pictures of docs/adr/0321: a dimension's options written (a symmetric tolerance and its value in mm, a short
//! dimension's value outside, the trailing zeros away, a value written with `<>`), Öznitelikler's rows of them
//! (olcu-secenekler), three dimensions spaced from a base (olcu-aralik) and a dimension broken where a wall crosses it,
//! its value moved (olcu-kirma). `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=olcu-secenekler,olcu-aralik,olcu-kirma cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::{
    AltUnits, CommandResult, DimensionEdit, DimensionFit, DimensionOptions, DimensionTolerance, DimensionZeros,
    DimensionsEdit, DrawingUnit, ToleranceKind,
};
use kentos_domain::Slot;
use kentos_native_application::ExecutionContext;
use kentos_native_application::dimensions_edit;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited};
use crate::tools_screens::Scene;

fn drawing(app: &mut App) {
    let mut o = Objects::new();
    // A wall and four dimensions along it, one short, one crossed by a pipe.
    o.line("duvar", [0.0, 0.0], [8.0, 0.0]);
    o.dimension("olcu", None, [[0.0, 0.0], [8.0, 0.0]], None, 1.0, serde_json::json!({ "height": 0.125 }));
    o.dimension("olcu", None, [[0.0, 0.0], [8.0, 0.0]], None, 1.6, serde_json::json!({ "height": 0.125 }));
    o.dimension("olcu", None, [[0.0, 0.0], [5.0, 0.0]], None, 2.9, serde_json::json!({ "height": 0.125 }));
    o.dimension("olcu", None, [[8.0, 0.0], [8.4, 0.0]], None, -0.8, serde_json::json!({ "height": 0.125 }));
    o.line("boru", [3.0, -1.5], [3.0, 4.5]);
    o.dimension(
        "olcu",
        None,
        [[0.0, 6.0], [6.0, 6.0]],
        None,
        1.0,
        serde_json::json!({ "height": 0.125, "text": "<> (tipik)" }),
    );
    open_edited(app, o, |d| {
        d["layers"] = serde_json::json!([
            layer("duvar", "Duvar", "#3E63DD", 0.35),
            layer("boru", "Boru", "#12A594", 0.25),
            layer("olcu", "Ölçü", "#E5484D", 0.18),
        ]);
        d["settings"]["plotScale"] = serde_json::json!(50);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.0,
            min_y: N - 2.0,
            max_x: E + 10.0,
            max_y: N + 8.0,
        },
        24.0,
    );
}

fn uid(app: &App, slot: u32) -> String {
    app.document.as_ref().expect("a drawing").model.uid(Slot(slot)).expect("an object").to_string()
}

fn edit(app: &mut App, edit: DimensionEdit) {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let input = DimensionsEdit { edit, expected_revision: None };
    assert!(matches!(
        dimensions_edit::execute(&mut ExecutionContext::new(doc), input),
        CommandResult::Completed { .. }
    ));
}

/// The options written: a tolerance and mm on the first, the short one's value outside, zeros away; the first
/// selected, Öznitelikler's rows.
fn options(app: &mut App) {
    drawing(app);
    let first = vec![uid(app, 2)];
    edit(
        app,
        DimensionEdit::Options {
            uids: first,
            options: Some(DimensionOptions {
                tolerance: Some(DimensionTolerance {
                    kind: ToleranceKind::Symmetric,
                    upper: 0.005,
                    lower: None,
                    decimals: None,
                }),
                alt: Some(AltUnits {
                    unit: DrawingUnit::Mm,
                    decimals: Some(0),
                }),
                ..DimensionOptions::default()
            }),
        },
    );
    let short = vec![uid(app, 5)];
    edit(
        app,
        DimensionEdit::Options {
            uids: short,
            options: Some(DimensionOptions {
                fit: Some(DimensionFit::Auto),
                zeros: Some(DimensionZeros::Trailing),
                ..DimensionOptions::default()
            }),
        },
    );
    app.selection.set([Slot(2)]);
    // The layers folded and Genel folded: the dimension's rows in view.
    let upper =
        kentos_ui::widget::docking::Slot::Docked(kentos_ui::widget::docking::Side::Right, 0);
    let _ = app.update(crate::app::Message::Dock(kentos_ui::widget::docking::Event::Collapsed(
        upper, true,
    )));
    let _ = app.update(crate::app::Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    hover(app, [9.5, 7.5]);
}

/// The three dimensions spaced 0.5 m from the base.
fn spaced(app: &mut App) {
    drawing(app);
    let (base, others) = (uid(app, 2), vec![uid(app, 3), uid(app, 4)]);
    edit(
        app,
        DimensionEdit::Space {
            base,
            uids: others,
            spacing: Some(0.5),
        },
    );
    app.selection.set([Slot(2), Slot(3), Slot(4)]);
    hover(app, [9.5, 7.5]);
}

/// The dimensions broken where the pipe crosses them; the first's value moved along.
fn broken(app: &mut App) {
    drawing(app);
    let uids = vec![uid(app, 2), uid(app, 3), uid(app, 4)];
    edit(app, DimensionEdit::Break { uids, objects: None });
    let first = uid(app, 2);
    edit(
        app,
        DimensionEdit::MoveText {
            uid: first,
            along: 1.5,
            across: 0.0,
        },
    );
    hover(app, [9.5, 7.5]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("olcu-secenekler", options), ("olcu-aralik", spaced), ("olcu-kirma", broken)]
}
