//! The pictures of docs/adr/0186 (Tarama ekleri) in a CAD project at 1:500:
//! three parcels, a building and a pool as islands and a parcel's number;
//! Tarama over the first with ANSI31 and Yazılar on (the region and the
//! pattern before the click), Desen's menu from its chip, the library's
//! patterns and the gradients side by side, Çoklu tara over the three
//! parcels before Enter, a tied hatch following its building moved, and
//! Öznitelikler's rows of a hatch with its Desen menu. The web's are
//! `shots.mjs hatches`. `tools_screens` takes them in the dark and the light
//! theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tarama-araci,tarama-desenler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::keyboard::key::Named;
use kentos_domain::Slot;
use kentos_geometry_core::tools::hatch::{Options, choice_named, pattern_of};
use kentos_ui::snapshot::Input;
use kentos_ui::widget::docking;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::{find_text, make_cad};
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::{Pointed, Scene, press_caption};

const A: [[f64; 2]; 4] = [[0.0, 0.0], [30.0, 0.0], [30.0, 22.0], [0.0, 22.0]];
const B: [[f64; 2]; 4] = [[30.0, 0.0], [56.0, 0.0], [56.0, 22.0], [30.0, 22.0]];
const C: [[f64; 2]; 4] = [[56.0, 0.0], [80.0, 0.0], [80.0, 22.0], [56.0, 22.0]];
const BUILDING: [[f64; 2]; 4] = [[5.0, 5.0], [15.0, 5.0], [15.0, 13.0], [5.0, 13.0]];

/// The parcels (1–3), the building in the first (4), its number (5) and the pool in the second (6).
fn ground() -> Objects {
    let mut o = Objects::new();
    for ring in [A, B, C] {
        o.path("parsel", &ring, true);
    }
    o.path("cizim", &BUILDING, true);
    o.text("cizim", [20.0, 16.0], "101", 2.0, 0.0);
    o.circle("cizim", [43.0, 11.0], 3.0);
    o
}

/// The view on a box of the drawing (east and north from its origin).
fn view(app: &mut App, min: [f64; 2], max: [f64; 2]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + min[0],
            min_y: N + min[1],
            max_x: E + max[0],
            max_y: N + max[1],
        },
        24.0,
    );
}

/// A drawing in a CAD project at 1:500, Açıklama's tab open.
fn cad(app: &mut App, objects: Objects, min: [f64; 2], max: [f64; 2]) {
    open(app, objects);
    make_cad(app);
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 500.0;
        doc.model.set_settings(settings);
    }
    app.tab = "annotate";
    app.command_expanded = false;
    view(app, min, max);
}

fn drawn(app: &mut App) {
    cad(app, ground(), [-6.0, -8.0], [86.0, 30.0]);
}

/// Tarama with Yazılar on, the pointer in the first parcel: the region
/// (the building an island, the number's box left open) and the pattern.
fn tool(app: &mut App) {
    drawn(app);
    app.memory.hatch_texts = true;
    run(app, "tool.hatch");
    hover(app, [24.0, 4.0]);
}

/// A choice's pattern at 1:500, its own scale and turn.
fn pattern(name: &str, color2: &str) -> Value {
    let choice = choice_named(name).unwrap_or_else(|| panic!("{name} is a choice"));
    let p = pattern_of(&Options {
        choice,
        scale: 1.0,
        angle: 0.0,
        color2: color2.to_owned(),
        inverted: false,
        plot_scale: 500.0,
    });
    serde_json::from_str(&kentos_geometry_core::api::json::to_string(&p))
        .expect("the pattern is JSON")
}

/// Every kind of pattern side by side: 14 × 10 m squares, each hatched and
/// named beneath, four a row.
const SHOWN: [(&str, &str); 12] = [
    ("ANSI31", "ANSI31"),
    ("ANSI33", "ANSI33"),
    ("ANSI36", "ANSI36"),
    ("ANSI37", "ANSI37"),
    ("ISO04W100", "ISO04W100"),
    ("NET3", "NET3"),
    ("BRICK", "BRICK"),
    ("DOTS", "DOTS"),
    ("CROSS", "CROSS"),
    ("Degrade doğrusal", "doğrusal"),
    ("Degrade silindir", "silindir"),
    ("Degrade küre", "küre"),
];

fn patterns(app: &mut App) {
    let mut o = Objects::new();
    for (k, (name, label)) in SHOWN.iter().enumerate() {
        let (x, y) = ((k % 4) as f64 * 18.0, -((k / 4) as f64) * 15.0);
        let ring = [[x, y], [x + 14.0, y], [x + 14.0, y + 10.0], [x, y + 10.0]];
        o.path("parsel", &ring, true);
        let id = o.hatch("cizim", &ring, pattern(name, "#FFFFFF"));
        if name.starts_with("Degrade") {
            o.fields(id, json!({ "color": "#3E63DD" }));
        }
        o.text("cizim", [x, y - 2.6], label, 1.6, 0.0);
    }
    cad(app, o, [-4.0, -36.0], [76.0, 12.0]);
    hover(app, [-3.0, 11.0]);
}

/// Çoklu tara over the three parcels, before Enter: their regions outlined.
fn many(app: &mut App) {
    drawn(app);
    app.selection.set([Slot(1), Slot(2), Slot(3)]);
    run(app, "tool.hatchSelected");
    hover(app, [40.0, 27.0]);
}

/// A tied hatch in the first parcel, its building then moved 8 m east: the
/// island moved with it, in the same step.
fn followed(app: &mut App) {
    drawn(app);
    run(app, "tool.hatch");
    click(app, [24.0, 4.0]);
    let _ = app.update(Message::Run("tool.cancel"));
    app.selection.set([Slot(4)]);
    run(app, "tool.move");
    click(app, BUILDING[0]);
    typed(app, "@8,0");
    app.selection.set([]);
    hover(app, [40.0, 27.0]);
}

/// The tied hatch selected: Öznitelikler's rows (Desen, Açı, Ölçek, İlişkili),
/// the layers folded away and Genel closed so that Geometri shows whole.
fn props(app: &mut App) {
    drawn(app);
    app.memory.hatch_texts = true;
    run(app, "tool.hatch");
    click(app, [24.0, 4.0]);
    let _ = app.update(Message::Run("tool.cancel"));
    app.memory.hatch_texts = false;
    app.selection.set([Slot(7)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    hover(app, [40.0, 27.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("tarama-araci", tool),
        ("tarama-desenler", patterns),
        ("tarama-coklu", many),
        ("tarama-iliskili", followed),
        ("tarama-oznitelikler", props),
    ]
}

/// Desen's menu from the command line's chip (in a narrow window, Diğer's
/// menu with Desen's open over it), and Öznitelikler's Desen drop-down.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        ("tarama-desen-menusu", tool, |s, app| {
            if find_text(s, app, "Desen: ANSI31").is_some() {
                press_caption(s, app, "Desen: ANSI31", false, false);
                return;
            }
            press_caption(s, app, "Diğer", false, false);
            let mut update = |app: &mut App, message: Message| {
                let _ = app.update(message);
            };
            s.settle(app, App::view, &mut update);
            for key in [Named::ArrowDown, Named::ArrowRight] {
                s.input(app, App::view, &mut update, Input::Key(key));
            }
        }),
        ("tarama-oznitelikler-desen", props, |s, app| {
            press_caption(s, app, "ANSI31", false, false)
        }),
    ]
}
