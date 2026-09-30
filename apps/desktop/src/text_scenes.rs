//! The drawing and the steps of the pictures of docs/adr/0145 §6 (Yazı's
//! options and Öznitelikler's text rows): the tool with Hiza sağ üst,
//! Genişlik 0.8, Zemin and Artır on, its box about the pointer, and the Hiza
//! menu from the command line's chip; one text's rows and their Hiza
//! drop-down; two texts that differ (Çeşitli). The web's are
//! `shots.mjs texts` (`text-options`, `text-props` …). `tools_screens` takes
//! them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=yazi-secenekleri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;

use iced::keyboard::key::Named;
use kentos_ui::snapshot::Input;

use crate::app::{App, Message};
use crate::files_testing::find_text;
use crate::find_replace::Event as FindEvent;
use crate::tools_scenes::{Objects, forget, hover, open, run, typed};
use crate::tools_screens::{Pointed, Scene, press_caption};

/// A square and a line across it, a centred, narrowed and masked text on
/// the line in the square, and a plain one turned 20°: slots 1 to 4.
fn texts_ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "cizim",
        &[[4.0, 4.0], [22.0, 4.0], [22.0, 20.0], [4.0, 20.0]],
        true,
    );
    o.line("cizim", [0.0, 12.0], [52.0, 12.0]);
    let a = o.text("cizim", [13.0, 12.0], "Ada 101", 1.6, 0.0);
    o.text_extras(a, Some("middleCenter"), Some(0.8), true);
    o.text("cizim", [34.0, 17.0], "Yol 12", 1.6, 20.0);
    o
}

const ADA: Slot = Slot(3);
const YOL: Slot = Slot(4);

/// Yazı with its options set as a user sets them: Hiza from its menu, the
/// others typed; the pointer where the next text goes.
fn tool(app: &mut App) {
    open(app, texts_ground());
    run(app, "tool.text");
    let _ = app.update(Message::PromptChoice("H", "sağ üst"));
    for option in ["G", "0.8", "Z", "R"] {
        typed(app, option);
    }
    forget(app);
    hover(app, [40.0, 6.0]);
}

/// The texts selected, Öznitelikler with the room to show its Geometri rows:
/// the dock's upper stack folded to its header and Genel shut.
fn props(app: &mut App, slots: &[Slot]) {
    open(app, texts_ground());
    app.selection.set(slots.iter().copied());
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
}

/// Okunur yap (4b): two texts upside down (centred, and on the baseline's
/// centre) and one that reads, all selected, Değiştir's tab open.
fn upside(app: &mut App) {
    let mut o = Objects::new();
    o.line("cizim", [0.0, 12.0], [52.0, 12.0]);
    let ada = o.text("cizim", [14.0, 14.0], "Ada 101", 1.6, 180.0);
    o.text_extras(ada, Some("middleCenter"), None, false);
    let yol = o.text("cizim", [34.0, 10.0], "Yol 12", 1.6, 200.0);
    o.text_extras(yol, Some("baselineCenter"), None, false);
    o.text("cizim", [24.0, 5.0], "Park", 1.6, 30.0);
    open(app, o);
    app.selection.set([Slot(2), Slot(3), Slot(4)]);
    app.tab = "modify";
    forget(app);
}

/// Bul ve değiştir (4c): four texts, one of them on a locked layer, and the
/// window looking for `Ada *` to write `Parsel *` with Joker on.
fn find_window(app: &mut App) {
    let mut o = Objects::new();
    for (words, y) in [
        ("Ada 101", 30.0),
        ("Ada 102", 26.0),
        ("Ada 103", 22.0),
        ("Yol 12", 18.0),
    ] {
        o.text("cizim", [10.0, y], words, 1.6, 0.0);
    }
    o.text("parsel", [30.0, 30.0], "Ada 9", 1.6, 0.0);
    open(app, o);
    let _ = app.update(Message::LayerLocked("parsel".into()));
    run(app, "text.findReplace");
    for event in [
        FindEvent::Find("Ada *".into()),
        FindEvent::Replace("Parsel *".into()),
        FindEvent::Wildcard(true),
    ] {
        let _ = app.update(Message::FindReplace(event));
    }
}

/// Metin dosyası yerleştir (4d): a file of parcel names read through the
/// picker, Yazı at 3 mm and orta sol; the lines' boxes about the pointer.
fn text_file(app: &mut App) {
    open(app, texts_ground());
    let file = std::env::temp_dir().join("kentos-parseller.txt");
    std::fs::write(
        &file,
        "Ada 101 Parsel 1\nAda 101 Parsel 2\nAda 101 Parsel 3\n\nAda 102 Parsel 1\nAda 102 Parsel 2\n",
    )
    .expect("the scene's file");
    app.picker = crate::app::Picker::File(file);
    run(app, "tool.text");
    typed(app, "Y");
    typed(app, "3");
    let _ = app.update(Message::PromptChoice("H", "sol orta"));
    let _ = app.update(Message::Run("tool.cancel"));
    let task = app.update(Message::Run("tool.placeTextFile"));
    crate::files_testing::drive(app, task);
    // The view drawn back so that the six lines, 3 m high, fit in it.
    let centre = crate::tools_scenes::at(app, [26.0, 12.0]);
    let _ = app.update(Message::Viewport(crate::viewport::Event::Zoomed {
        factor: 0.6,
        at: centre,
    }));
    forget(app);
    hover(app, TEXT_FILE_AT);
}

/// Where the first line goes: 1.6 of the web's `u` (10 m here) left of the
/// ground's centre and 3 × 1.5 × 3 m over Ada 101's line, so that the empty
/// fourth line falls on Ada 101, as the web's `FILE_AT` puts it.
const TEXT_FILE_AT: [f64; 2] = [10.0, 25.5];

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("yazi-secenekleri", tool),
        ("yazi-oznitelikler", |app| props(app, &[ADA])),
        ("yazi-oznitelikler-coklu", |app| props(app, &[ADA, YOL])),
        ("bul-degistir", find_window),
        ("metin-dosyasi", text_file),
        ("metin-dosyasi-sonuc", |app| {
            text_file(app);
            crate::tools_scenes::click(app, TEXT_FILE_AT);
        }),
        ("okunur-yap-secim", upside),
        ("okunur-yap-sonuc", |app| {
            upside(app);
            run(app, "tool.readable");
        }),
    ]
}

/// The Hiza menus: the command line's chip (in a narrow window, Diğer's
/// menu with Hiza's open over it), and Öznitelikler's drop-down.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        ("yazi-hiza-menusu", tool, |s, app| {
            if find_text(s, app, "Hiza: sağ üst").is_some() {
                press_caption(s, app, "Hiza: sağ üst", false, false);
                return;
            }
            press_caption(s, app, "Diğer", false, false);
            let mut update = |app: &mut App, message: Message| {
                let _ = app.update(message);
            };
            s.settle(app, App::view, &mut update);
            // Hiza, Diğer's first row, opened from the keyboard as a user may:
            // ↓ to it, → into its menu.
            for key in [Named::ArrowDown, Named::ArrowRight] {
                s.input(app, App::view, &mut update, Input::Key(key));
            }
        }),
        (
            "yazi-oznitelikler-hiza",
            |app| props(app, &[ADA]),
            |s, app| press_caption(s, app, "Orta", false, false),
        ),
    ]
}
