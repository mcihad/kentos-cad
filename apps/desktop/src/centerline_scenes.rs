//! The pictures of docs/adr/0190 (Orta hat) in a CAD project at 1:500, the
//! shared trace's ground (`centerline.kcad`): a road's two sides with a
//! quarter-turn bend, a stream's banks closing in and a bank in two lines.
//! The road's sides picked, their axis dashed edge by edge; the stream's
//! axis sampled every 5 m; and what was written. The web's are
//! `shots.mjs centerline`. `tools_screens` takes them in the dark and the
//! light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=orta-hat,orta-hat-dere,orta-hat-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The road's sides (1, 2), the stream's banks (3, 4), a bank in two
/// lines (5, 6) and the other bank (7); the banks on Parsel, the scenes'
/// blue layer (the trace's Dere).
fn ground() -> Objects {
    let mut o = Objects::new();
    let quarter = (std::f64::consts::PI / 8.0).tan();
    o.bulged(
        "yol",
        &[[-40.0, -23.0], [-20.0, -23.0], [-13.0, -16.0]],
        &[0.0, quarter],
    );
    o.bulged(
        "yol",
        &[[-40.0, -29.0], [-20.0, -29.0], [-7.0, -16.0]],
        &[0.0, quarter],
    );
    o.line("parsel", [-40.0, 4.0], [0.0, 4.0]);
    o.path("parsel", &[[-40.0, 12.0], [-20.0, 10.0], [0.0, 8.0]], false);
    o.line("parsel", [-40.0, 24.0], [-20.0, 24.0]);
    o.line("parsel", [-20.0, 24.0], [0.0, 24.0]);
    o.line("parsel", [-40.0, 30.0], [0.0, 30.0]);
    o
}

/// The drawing in a CAD project at 1:500, Orta hat with the road's sides picked.
fn shown(app: &mut App) {
    open(app, ground());
    make_cad(app);
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 500.0;
        doc.model.set_settings(settings);
    }
    app.tab = "home";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 48.0,
            min_y: N - 34.0,
            max_x: E + 12.0,
            max_y: N + 34.0,
        },
        24.0,
    );
    run(app, "tool.centerline");
    click(app, [-30.0, -23.0]);
    click(app, [-30.0, -29.0]);
    hover(app, [6.0, -6.0]);
}

/// The stream's banks closing in, their axis sampled every 5 m.
fn stream(app: &mut App) {
    shown(app);
    click(app, [-30.0, 4.0]);
    click(app, [-30.0, 11.0]);
    for t in ["B", "5"] {
        typed(app, t);
    }
    hover(app, [6.0, -6.0]);
}

/// What was written: the road's axis with its arc, the stream's and the
/// two-line bank's (Zincir) on the active layer.
fn written(app: &mut App) {
    shown(app);
    let _ = app.update(crate::app::Message::Run("tool.confirm"));
    click(app, [-30.0, 4.0]);
    click(app, [-30.0, 11.0]);
    for t in ["B", "5"] {
        typed(app, t);
    }
    let _ = app.update(crate::app::Message::Run("tool.confirm"));
    click(app, [-30.0, 24.0]);
    click(app, [-30.0, 30.0]);
    let _ = app.update(crate::app::Message::Run("tool.confirm"));
    hover(app, [6.0, -6.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("orta-hat", shown),
        ("orta-hat-dere", stream),
        ("orta-hat-yazildi", written),
    ]
}
