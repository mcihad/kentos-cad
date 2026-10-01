//! The pictures of docs/adr/0147 §7, Ölçülendirme's new methods at work over
//! a parcel beside a curved road: Koordinat with the parcel's corner taken and
//! the cursor off to the right (its X and its jogged line); Yay uzunluğu with the road's
//! edge taken and the dimension arc following the cursor; Kısmi with its first
//! point on the arc and the part to the cursor lit; Kırıklı yarıçap with the
//! jog following the cursor; Semt on the parcel's west edge; Eğim between two
//! points with elevations, and its question for a bare corner's elevation;
//! Açı from the road's edge (Yaydan) and from a manhole (Daireden, Zemin on);
//! Doğrusal along a typed direction; Öznitelikler's rows of a slope and of
//! two ordinates; Hızlı ölçü over two parcels and a road edge, as the cursor
//! places them and as written; Ölçülendirme ▾'s methods. The web's are `shots.mjs dimensions`
//! (`dimension-ordinate`, `dimension-arc-length`, `dimension-partial`,
//! `dimension-jogged`, `dimension-azimuth`, `dimension-slope`,
//! `dimension-slope-ask`, `dimension-angle-arc`, `dimension-angle-circle`,
//! `dimension-linear-angle`, `dimension-properties`,
//! `dimension-properties-many`, `dimension-quick`, `dimension-quick-result`,
//! `dimension-methods`).
//! `tools_screens` takes them in the dark and the light theme at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=olcu-koordinat cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::f64::consts::PI;

use serde_json::json;

use crate::app::{App, Message};
use kentos_ui::widget::docking;
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

/// A manhole east of the road, 5 m across its centre.
const MANHOLE: [f64; 2] = [62.0, -22.0];

/// Açı, Yaydan: the road's outer edge taken, its own angle drawn 4 m out of it with the cursor.
fn angle_arc(app: &mut App) {
    open(app, ground());
    method(app, "tool.dimension", "A");
    click(app, on_edge(90.0));
    forget(app);
    hover(app, [20.0, -6.0]);
}

/// Açı, Daireden: Zemin on, the manhole's east point, a second point north
/// of its centre, the arc with the cursor between them.
fn angle_circle(app: &mut App) {
    let mut o = ground();
    o.circle("yol", MANHOLE, 5.0);
    open(app, o);
    method(app, "tool.dimension", "A");
    typed(app, "Z");
    click(app, [MANHOLE[0] + 5.0, MANHOLE[1]]);
    click(app, [MANHOLE[0], MANHOLE[1] + 12.0]);
    forget(app);
    hover(app, [MANHOLE[0] + 6.0, MANHOLE[1] + 6.0]);
}

/// Doğrusal along a typed direction: the parcel's south-west and north-east
/// corners, 60 grads typed (54°), the dimension line south-east of them.
fn linear_angle(app: &mut App) {
    open(app, ground());
    method(app, "tool.dimension", "D");
    click(app, [0.0, 0.0]);
    click(app, [44.0, 32.0]);
    typed(app, "A");
    typed(app, "60");
    forget(app);
    hover(app, [40.0, -8.0]);
}

/// The ground with a slope between the levelled points (4 m east), the
/// north-west corner's Y (up) and the south-west corner's X (left, with Zemin).
fn measured() -> Objects {
    let mut o = ground();
    o.dimension(
        "parsel",
        Some("slope"),
        [[50.0, 0.0], [54.0, 32.0]],
        None,
        -4.0,
        json!({ "za": 102.4, "zb": 101.15 }),
    );
    o.dimension(
        "parsel",
        Some("ordinate"),
        [[2.0, 30.0], [2.0, 40.0]],
        None,
        0.0,
        json!({ "angle": 0.0 }),
    );
    o.dimension(
        "parsel",
        Some("ordinate"),
        [[0.0, 0.0], [-12.0, 0.0]],
        None,
        0.0,
        json!({ "angle": 90.0, "mask": true }),
    );
    o
}

/// Öznitelikler over `slots` of `measured`, the layers' dock folded and Genel closed.
fn props(app: &mut App, slots: &[u32]) {
    open(app, measured());
    app.selection
        .set(slots.iter().map(|&s| kentos_domain::Slot(s)));
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
}

/// Hızlı ölçü's ground (as `fixtures/interaction/v1/quick-dimension.kcad`,
/// the road 6 m further south so that its dimensions stand clear of the
/// parcels'): two parcels side by side sharing their 30 m edge, a road edge
/// 10 m straight then a clockwise quarter arc, all selected, Hızlı ölçü running.
fn quick(app: &mut App) {
    let mut o = Objects::new();
    let west = o.path(
        "parsel",
        &[[0.0, 0.0], [20.0, 0.0], [20.0, 30.0], [0.0, 30.0]],
        true,
    );
    let east = o.path(
        "parsel",
        &[[20.0, 0.0], [40.0, 0.0], [40.0, 30.0], [20.0, 30.0]],
        true,
    );
    let road = o.bulged(
        "yol",
        &[[0.0, -16.0], [10.0, -16.0], [20.0, -26.0]],
        &[0.0, -(PI / 8.0).tan()],
    );
    open(app, o);
    app.selection
        .set([west, east, road].map(kentos_domain::Slot));
    crate::tools_scenes::run(app, "tool.quickDimension");
    forget(app);
    // 4 m over the parcels' north edge.
    hover(app, [10.0, 34.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("olcu-dugme-aci", chosen),
        ("olcu-hizli", quick),
        ("olcu-hizli-sonuc", |app| {
            quick(app);
            click(app, [10.0, 34.0]);
            app.selection.clear();
            // Drawn back so that the dimensions over the parcels show whole.
            let centre = crate::tools_scenes::at(app, [20.0, 4.0]);
            let _ = app.update(Message::Viewport(crate::viewport::Event::Zoomed {
                factor: 0.85,
                at: centre,
            }));
            hover(app, [60.0, -30.0]);
        }),
        ("olcu-aci-yaydan", angle_arc),
        ("olcu-aci-daireden", angle_circle),
        ("olcu-dogrusal-aci", linear_angle),
        // The slope is the drawing's 6th object, the ordinates the 7th and 8th.
        ("olcu-oznitelikler", |app| props(app, &[6])),
        ("olcu-oznitelikler-coklu", |app| props(app, &[7, 8])),
        ("olcu-koordinat", ordinate),
        ("olcu-yay-uzunlugu", arc_length),
        ("olcu-kismi", partial),
        ("olcu-kirikli", jogged),
        ("olcu-semt", azimuth),
        ("olcu-egim", slope),
        ("olcu-egim-kot", slope_ask),
    ]
}

/// Ölçülendirme ▾: its methods, each with its own icon, and the family's
/// tools; the button large, its panel's lead (DESIGN.md §7.3.1).
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![(
        "olcu-yontemleri",
        |app| app.tab = "draw",
        |s, app| open_split(s, app, "dimension"),
    )]
}

/// Ölçülendirme with Açı chosen from its list: the button's face shows
/// Açı's icon; the tool stopped again.
pub(crate) fn chosen(app: &mut App) {
    app.tab = "draw";
    let _ = app.update(Message::SplitChosen {
        key: "dimension",
        id: "tool.dimension",
        option: Some("A"),
        label: "Açı",
    });
    let _ = app.update(Message::Run("tool.cancel"));
    forget(app);
}
