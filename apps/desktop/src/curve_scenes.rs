//! The pictures of docs/adr/0289 (Daire's, Yay's and Elips's methods through `cad.curves.construct`): Daire's
//! TTY between two parallel lines 10 m apart and 5 m, the circle beside the picks; the same with 3 m, refused with
//! the command's words; Yay given clockwise, then Devam going on from the last point given; Yay's BSY with the
//! direction shown by the pointer; Elips's arc after its start angle, the end under the pointer; Yay's three
//! points on a line, refused. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=egri-tty-paralel,egri-tty-uyari,egri-devam,egri-bsy,egri-eliptik-yay,egri-dogru cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// Two parallel road edges 10 m apart, in a CAD project, the view on them.
fn ground(app: &mut App) {
    let mut o = Objects::new();
    o.line("yol", [-4.0, 0.0], [36.0, 0.0]);
    o.line("yol", [-4.0, 10.0], [36.0, 10.0]);
    open(app, o);
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 6.0,
            min_y: N - 14.0,
            max_x: E + 38.0,
            max_y: N + 16.0,
        },
        24.0,
    );
}

/// Daire, TTY: both edges picked, 5 m typed: the circle between them, across from the picks' middle.
fn ttr_parallel(app: &mut App) {
    ground(app);
    run(app, "tool.circle");
    typed(app, "TTY");
    click(app, [4.0, 0.0]);
    click(app, [12.0, 10.0]);
    typed(app, "5");
    hover(app, [24.0, 5.0]);
}

/// The same with 3 m: refused with `cad.curves.construct`'s words.
fn ttr_refused(app: &mut App) {
    ground(app);
    run(app, "tool.circle");
    typed(app, "TTY");
    click(app, [4.0, 0.0]);
    click(app, [12.0, 10.0]);
    typed(app, "3");
    hover(app, [24.0, 5.0]);
}

/// Yay given clockwise (west, north, east), then Devam: on from its east end, the last point given.
fn continued(app: &mut App) {
    ground(app);
    run(app, "tool.arc");
    click(app, [2.0, -8.0]);
    click(app, [10.0, -2.0]);
    click(app, [18.0, -8.0]);
    typed(app, "D");
    hover(app, [26.0, -9.0]);
}

/// Yay's BSY: start and end on the lower road's edge, the direction shown by the pointer.
fn start_end_direction(app: &mut App) {
    ground(app);
    run(app, "tool.arc");
    typed(app, "BSY");
    click(app, [4.0, -6.0]);
    click(app, [24.0, -6.0]);
    hover(app, [9.0, -1.0]);
}

/// Elips, Yay: axis ends, the other half-axis typed, the start angle typed; the end under the pointer.
fn elliptic_arc(app: &mut App) {
    ground(app);
    run(app, "tool.ellipse");
    typed(app, "Y");
    click(app, [4.0, -8.0]);
    click(app, [28.0, -8.0]);
    typed(app, "4");
    typed(app, "0");
    hover(app, [10.0, -4.0]);
}

/// Yay: three points on one line: refused with the command's words, the points kept.
fn collinear(app: &mut App) {
    ground(app);
    run(app, "tool.arc");
    click(app, [2.0, -6.0]);
    click(app, [12.0, -6.0]);
    click(app, [22.0, -6.0]);
    hover(app, [26.0, -8.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("egri-tty-paralel", ttr_parallel),
        ("egri-tty-uyari", ttr_refused),
        ("egri-devam", continued),
        ("egri-bsy", start_end_direction),
        ("egri-eliptik-yay", elliptic_arc),
        ("egri-dogru", collinear),
    ]
}
