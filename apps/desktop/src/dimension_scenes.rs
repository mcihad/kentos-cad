//! The pictures of docs/adr/0147 §7, Ölçülendirme's new methods at work over
//! a parcel beside a curved road: Koordinat with the parcel's corner taken and
//! the cursor off to the right (its X and its jogged line); Yay uzunluğu with the road's
//! edge taken and the dimension arc following the cursor; Kısmi with its first
//! point on the arc and the part to the cursor lit; Kırıklı yarıçap with the
//! jog following the cursor; Semt on the parcel's west edge; Eğim between two
//! points with elevations, and its question for a bare corner's elevation;
//! Ölçülendirme ▾'s methods. The web's are `shots.mjs dimensions`
//! (`dimension-ordinate`, `dimension-arc-length`, `dimension-partial`,
//! `dimension-jogged`, `dimension-azimuth`, `dimension-slope`,
//! `dimension-slope-ask`, `dimension-methods`).
//! `tools_screens` takes them in the dark and the light theme at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=olcu-koordinat cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::f64::consts::PI;

use crate::app::App;
use crate::tools_scenes::{Objects, click, forget, hover, method, open, typed};
use crate::tools_screens::{Pointed, Scene, open_split};

/// The road's edges: arcs about (20, −60), 50 and 42 m out, from 50° to 130°.
const C: [f64; 2] = [20.0, -60.0];

/// The point `deg` degrees round the road's outer edge.
fn on_edge(deg: f64) -> [f64; 2] {
    let t = deg * PI / 180.0;
    [C[0] + 50.0 * t.cos(), C[1] + 50.0 * t.sin()]
}

/// A parcel and the curved road below it.
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[0.0, 0.0], [40.0, 0.0], [44.0, 32.0], [2.0, 30.0]],
        true,
    );
    for r in [50.0, 42.0] {
        o.arc("yol", C, r, 50.0 * PI / 180.0, 130.0 * PI / 180.0);
    }
    // Two levelled points east of the parcel.
    o.point("parsel", [50.0, 0.0], Some(102.4));
    o.point("parsel", [54.0, 32.0], Some(101.15));
    o
}

/// Koordinat: the parcel's north-east corner taken, the cursor off to the right and a little up.
fn ordinate(app: &mut App) {
    open(app, ground());
    method(app, "tool.dimension", "O");
    click(app, [44.0, 32.0]);
    forget(app);
    hover(app, [66.0, 35.0]);
}

/// Yay uzunluğu: the road's edge taken, the dimension arc 5 m out with the cursor.
fn arc_length(app: &mut App) {
    open(app, ground());
    method(app, "tool.dimension", "U");
    click(app, on_edge(80.0));
    forget(app);
    hover(app, [24.0, -4.8]);
}

/// Kısmi: the edge taken, its first point at the top, the part to the cursor lit.
fn partial(app: &mut App) {
    open(app, ground());
    method(app, "tool.dimension", "U");
    typed(app, "K");
    click(app, on_edge(80.0));
    click(app, on_edge(90.0));
    forget(app);
    hover(app, on_edge(115.0));
}

/// Kırıklı yarıçap: the road's edge, the centre shown 20 m back along the
/// radius and 3 m aside, the point on the edge, the jog with the cursor.
fn jogged(app: &mut App) {
    open(app, ground());
    method(app, "tool.dimension", "I");
    click(app, on_edge(60.0));
    click(app, [32.402, -32.519]);
    click(app, on_edge(60.0));
    forget(app);
    hover(app, [38.0, -24.0]);
}

/// Semt: the parcel's west edge, the arrow 4 m out to the west.
fn azimuth(app: &mut App) {
    open(app, ground());
    method(app, "tool.dimension", "T");
    click(app, [0.0, 0.0]);
    click(app, [2.0, 30.0]);
    forget(app);
    hover(app, [-3.0, 15.0]);
}

/// Eğim: between the two levelled points, the arrow east of them.
fn slope(app: &mut App) {
    open(app, ground());
    method(app, "tool.dimension", "E");
    click(app, [50.0, 0.0]);
    click(app, [54.0, 32.0]);
    forget(app);
    hover(app, [57.0, 16.0]);
}

/// Eğim on the parcel's bare corner: its elevation asked for.
fn slope_ask(app: &mut App) {
    open(app, ground());
    method(app, "tool.dimension", "E");
    click(app, [0.0, 0.0]);
    forget(app);
    hover(app, [20.0, 4.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("olcu-koordinat", ordinate),
        ("olcu-yay-uzunlugu", arc_length),
        ("olcu-kismi", partial),
        ("olcu-kirikli", jogged),
        ("olcu-semt", azimuth),
        ("olcu-egim", slope),
        ("olcu-egim-kot", slope_ask),
    ]
}

/// Ölçülendirme ▾: its methods and the family's tools.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![(
        "olcu-yontemleri",
        |app| app.tab = "draw",
        |s, app| open_split(s, app, "dimension"),
    )]
}
