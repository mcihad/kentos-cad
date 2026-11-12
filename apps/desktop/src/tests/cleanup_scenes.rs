//! The pictures of docs/adr/0305 (Çizimi temizle by its command) in a CAD project: a survey sketch carrying what an
//! import leaves behind: a fence line drawn twice (once reversed), a kerb in three overlapping pieces, a parcel
//! boundary with repeated and straight vertices, an empty line and a duplicated tree circle 0.4 mm off. Shown first
//! at the default tolerance (what goes dashed, what changes in the accent, the counts on the command line); then at
//! 1 mm with Uç uca on; and what the write left. `tools_screens` takes them in the dark and the light theme at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=temizle-bulunan,temizle-milimetre,temizle-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, open, run, typed};
use crate::tools_screens::Scene;

fn ground() -> Objects {
    let mut o = Objects::new();
    // The fence, twice.
    o.line("cizim", [0.0, 0.0], [20.0, 0.0]);
    o.line("cizim", [20.0, 0.0], [0.0, 0.0]);
    // The kerb in three pieces.
    o.line("yol", [0.0, 6.0], [9.0, 6.0]);
    o.line("yol", [7.0, 6.0], [16.0, 6.0]);
    o.line("yol", [16.0, 6.0], [24.0, 6.0]);
    // The parcel with a repeated and a straight vertex.
    o.path(
        "parsel",
        &[
            [2.0, 10.0],
            [2.0, 10.0],
            [10.0, 10.0],
            [18.0, 10.0],
            [18.0, 16.0],
            [2.0, 16.0],
        ],
        true,
    );
    // An empty line and a tree drawn twice.
    o.line("cizim", [22.0, 12.0], [22.0, 12.0]);
    o.circle("cizim", [26.0, 13.0], 1.5);
    o.circle("cizim", [26.0004, 13.0], 1.5);
    o
}

fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 3.0,
            max_x: E + 30.0,
            max_y: N + 18.0,
        },
        24.0,
    );
}

/// The default tolerance: the fence's twin, the kerb's overlap, the parcel's vertices and the empty line.
fn found(app: &mut App) {
    drawn(app);
    run(app, "tool.cleanup");
    hover(app, [28.0, -2.0]);
}

/// 1 mm and Uç uca: the tree's twin and the kerb's last piece too.
fn millimetre(app: &mut App) {
    found(app);
    typed(app, "T");
    typed(app, "0.001");
    typed(app, "U");
    hover(app, [28.0, -2.0]);
}

/// Enter: what is left, and the message with the farthest move.
fn written(app: &mut App) {
    millimetre(app);
    typed(app, "");
    hover(app, [28.0, -2.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("temizle-bulunan", found),
        ("temizle-milimetre", millimetre),
        ("temizle-yazildi", written),
    ]
}
