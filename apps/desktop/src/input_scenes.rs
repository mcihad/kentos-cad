//! The pictures of docs/adr/0266 (lengths with their own unit and sums in brackets): a millimetre project, Çizgi
//! with `@2.5m<0`, `@(1.2+0.3)m<90` and `@-50cm,(100*2)` typed, the cursor's value field holding `(1000/4)` while
//! the next segment waits; and docs/adr/0268's: Shift held over a polyline, ortho ruling for the moment and polar
//! tracking (on) shown out of use in the status bar. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=giris-birim,giris-oncelik cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::DrawingUnit;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// A millimetre drawing; Çizgi from the origin with three typed segments; the cursor up to the north-west.
fn units(app: &mut App) {
    open(app, Objects::new());
    make_cad(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    s.srid = 0;
    s.second_srid = None;
    s.drawing_unit = Some(DrawingUnit::Mm);
    s.angle_unit = kentos_contracts::AngleUnit::Deg;
    s.plot_scale = 20.0;
    doc.model.set_settings(s);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.8,
            min_y: N - 0.6,
            max_x: E + 3.4,
            max_y: N + 2.4,
        },
        24.0,
    );
    run(app, "tool.line");
    click(app, [0.0, 0.0]);
    for text in ["@2.5m<0", "@(1.2+0.3)m<90", "@-50cm,(100*2)"] {
        typed(app, text);
    }
    hover(app, [1.0, 2.2]);
}

/// Polar tracking on, ortho off; Shift held: Orto lit for the moment, Kutupsal out of use, the preview square.
fn priority(app: &mut App) {
    open(app, Objects::new());
    make_cad(app);
    let _ = app.settings.choose(&[
        ("drafting.polar", serde_json::Value::Bool(true)),
        ("drafting.snap", serde_json::Value::Bool(false)),
    ]);
    app.apply_settings();
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 4.0,
            max_x: E + 30.0,
            max_y: N + 18.0,
        },
        24.0,
    );
    run(app, "tool.polyline");
    click(app, [0.0, 0.0]);
    click(app, [12.0, 0.0]);
    hover(app, [22.0, 8.5]);
    let _ = app.update(crate::app::Message::Modifiers(
        iced::keyboard::Modifiers::SHIFT,
    ));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("giris-birim", units), ("giris-oncelik", priority)]
}
