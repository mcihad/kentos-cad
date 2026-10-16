//! The pictures of docs/adr/0323: a room's name and its dimension made annotative for 1:100 with 1:50 and 1:200
//! too, the name's 1:50 place set apart, selected with Öznitelikler's Ölçekli, Kendi ölçeği, Diğer ölçekler and Ayrı değerler and the
//! Açıklama tab's Ölçek panel (olcekli-oznitelik); Ölçekteki yeri moving the name at 1:50, its preview at the cursor
//! (olcekli-yeri). `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=olcekli-oznitelik,olcekli-yeri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::{AnnotationsScales, CommandResult};
use kentos_domain::Slot;
use kentos_native_application::ExecutionContext;
use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, layer, open_edited, run};
use crate::tools_screens::Scene;

fn uid(app: &App, slot: u32) -> String {
    app.document
        .as_ref()
        .expect("a drawing")
        .model
        .uid(Slot(slot))
        .expect("an object")
        .to_string()
}

fn scales(app: &mut App, input: serde_json::Value) {
    let input: AnnotationsScales = serde_json::from_value(input).expect("an input");
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let result = kentos_native_application::annotations_scales::execute(
        &mut ExecutionContext::new(doc),
        input,
    );
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
}

/// A room with its name and its width, annotative at 1:100, also at 1:50 and 1:200; the name's 1:50 place set apart.
fn drawing(app: &mut App) -> (u32, u32) {
    let mut o = Objects::new();
    o.path(
        "duvar",
        &[[0.0, 0.0], [6.0, 0.0], [6.0, 4.0], [0.0, 4.0]],
        true,
    );
    let name = o.text("yazi", [2.4, 1.8], "SALON", 0.25, 0.0);
    let width = o.dimension(
        "olcu",
        None,
        [[0.0, 0.0], [6.0, 0.0]],
        None,
        -0.8,
        json!({ "height": 0.25 }),
    );
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("duvar", "Duvar", "#3E63DD", 0.5),
            layer("yazi", "Yazı", "#8E4EC6", 0.25),
            layer("olcu", "Ölçü", "#E5484D", 0.18),
        ]);
        d["activeLayer"] = json!("yazi");
        d["settings"]["plotScale"] = json!(100);
    });
    make_cad(app);
    let (n, w) = (uid(app, name), uid(app, width));
    scales(
        app,
        json!({ "operation": "make", "uids": [n, w], "scales": [50.0, 200.0] }),
    );
    scales(
        app,
        json!({ "operation": "set", "uids": [n], "scale": 50.0, "point": { "x": E + 2.0, "y": N + 2.6 } }),
    );
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.5,
            min_y: N - 2.0,
            max_x: E + 7.5,
            max_y: N + 5.0,
        },
        24.0,
    );
    (name, width)
}

fn listed(app: &mut App) {
    let (name, _) = drawing(app);
    app.tab = "annotate";
    app.selection.set([Slot(name)]);
    // The layers folded and Genel folded: the scale rows in view.
    let upper =
        kentos_ui::widget::docking::Slot::Docked(kentos_ui::widget::docking::Side::Right, 0);
    let _ = app.update(crate::app::Message::Dock(
        kentos_ui::widget::docking::Event::Collapsed(upper, true),
    ));
    let _ = app.update(crate::app::Message::Properties(
        crate::properties::Event::Toggle("general"),
    ));
    hover(app, [6.8, 4.6]);
}

fn placed(app: &mut App) {
    let _ = drawing(app);
    app.tab = "annotate";
    run(app, "tool.annotativePlace");
    click(app, [2.8, 1.9]);
    hover(app, [3.6, 3.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("olcekli-oznitelik", listed), ("olcekli-yeri", placed)]
}
