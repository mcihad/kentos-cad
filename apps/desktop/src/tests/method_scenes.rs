//! The pictures of docs/adr/0287 (the drawing tools' methods after the comparison with AutoCAD): the
//! ribbon's menus of Yay, Dikdörtgen (with Düzgün çokgen), Elips and Revizyon bulutu in a CAD project;
//! Yay's Başlangıç, merkez, açı, Dikdörtgen's Alan (500 m², a side of 20 m), Yardımcı çizgi's Paralel (3 m
//! from a building's edges) and Revizyon bulutu's Nesne in use. `tools_screens` takes them in both themes at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=yontem-yay,yontem-dikdortgen,yontem-elips,yontem-bulut,yontem-yay-aci,yontem-alan,yontem-paralel,yontem-bulut-nesne cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{Objects, click, hover, method, open, typed};
use crate::tools_screens::{Pointed, Scene, open_split};

/// A building's outline and a parcel beside it, in a CAD project.
fn ground(app: &mut App) {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[0.0, 0.0], [30.0, 0.0], [30.0, 18.0], [0.0, 18.0]],
        true,
    );
    o.path(
        "parsel",
        &[[42.0, -6.0], [70.0, -6.0], [70.0, 26.0], [42.0, 26.0]],
        true,
    );
    open(app, o);
    make_cad(app);
    app.tab = "home";
}

fn arc_angle(app: &mut App) {
    ground(app);
    method(app, "tool.arc", "BMA");
    click(app, [-6.0, 0.0]);
    click(app, [-6.0, 12.0]);
    hover(app, [-16.0, 16.0]);
}

fn rectangle_area(app: &mut App) {
    ground(app);
    method(app, "tool.rectangle", "ALAN");
    click(app, [44.0, -4.0]);
    typed(app, "500");
    typed(app, "20");
    hover(app, [50.0, 4.0]);
}

fn parallel(app: &mut App) {
    ground(app);
    method(app, "tool.xline", "P");
    typed(app, "3");
    click(app, [30.0, 9.0]);
    click(app, [36.0, 9.0]);
    click(app, [15.0, 18.0]);
    hover(app, [15.0, 23.0]);
}

fn cloud_round(app: &mut App) {
    ground(app);
    method(app, "tool.revcloud", "N");
    click(app, [70.0, 10.0]);
    hover(app, [20.0, 30.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("yontem-yay-aci", arc_angle),
        ("yontem-alan", rectangle_area),
        ("yontem-paralel", parallel),
        ("yontem-bulut-nesne", cloud_round),
    ]
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        ("yontem-yay", ground, |s, app| open_split(s, app, "arc")),
        ("yontem-dikdortgen", ground, |s, app| {
            open_split(s, app, "rectangle")
        }),
        ("yontem-elips", ground, |s, app| {
            open_split(s, app, "ellipse")
        }),
        (
            "yontem-bulut",
            |app| {
                ground(app);
                app.tab = "annotate";
            },
            |s, app| open_split(s, app, "revcloud"),
        ),
    ]
}
