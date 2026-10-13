//! The pictures of docs/adr/0260 (an object's look): lines of their own line
//! type and pattern scale, an area half transparent over another, a solid
//! hatch sent to the back of its layer; the dashed wall selected with
//! Öznitelikler's Çizgi tipi, Çizgi tipi ölçeği and Saydamlık; the hatch
//! brought to the front by Öne getir. `tools_screens` takes them in both
//! themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=gorunus-cizim,gorunus-oznitelikler,gorunus-one-getir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::json;

use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, open, run};
use crate::tools_screens::Scene;

/// The walls (1 dashed ×2, 2 dotted, 3 dash-dot), the parcel (4) and the
/// half transparent area over it (5),
/// a line (6) and the solid hatch drawn after it but sent to the back (7, order −1).
fn ground() -> Objects {
    let mut o = Objects::new();
    let a = o.line("cizim", [0.0, 0.0], [14.0, 0.0]);
    o.fields(a, json!({ "lineType": "dashed", "lineTypeScale": 2.0 }));
    let b = o.line("cizim", [0.0, 3.0], [14.0, 3.0]);
    o.fields(b, json!({ "lineType": "dotted" }));
    let c = o.line("yol", [0.0, 6.0], [14.0, 6.0]);
    o.fields(c, json!({ "lineType": "dashdot", "lineTypeScale": 0.5 }));
    o.path("parsel", &[[18.0, 0.0], [28.0, 0.0], [28.0, 8.0], [18.0, 8.0]], true);
    let over = o.path("yol", &[[22.0, 4.0], [32.0, 4.0], [32.0, 12.0], [22.0, 12.0]], true);
    o.fields(over, json!({ "transparency": 0.5, "color": "#E5484D" }));
    let line = o.line("parsel", [16.0, 14.0], [32.0, 14.0]);
    let h = o.hatch(
        "parsel",
        &[[18.0, 12.0], [30.0, 12.0], [30.0, 17.0], [18.0, 17.0]],
        json!({ "type": "solid", "angle": 0, "spacing": 1 }),
    );
    o.fields(h, json!({ "order": -1, "color": "#46A758" }));
    o.fields(line, json!({ "lineWeight": 1.0 }));
    o
}

fn fit(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 2.0,
            max_x: E + 34.0,
            max_y: N + 19.0,
        },
        24.0,
    );
}

fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    fit(app);
    hover(app, [33.0, -1.5]);
}

/// The dashed wall selected: its look's rows in Öznitelikler.
fn props(app: &mut App) {
    drawn(app);
    app.selection.set([Slot(1)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    hover(app, [33.0, -1.5]);
}

/// Öne getir on the hatch: drawn over the line it was under.
fn front(app: &mut App) {
    drawn(app);
    app.selection.set([Slot(7)]);
    run(app, "edit.bringToFront");
    app.selection.set(Vec::new());
    hover(app, [33.0, -1.5]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("gorunus-cizim", drawn),
        ("gorunus-oznitelikler", props),
        ("gorunus-one-getir", front),
    ]
}

