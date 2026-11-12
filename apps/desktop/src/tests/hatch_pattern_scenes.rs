//! The pictures of docs/adr/0331 (taramanın desenleri, başlangıcı ve ölçekliliği) in a CAD project at 1:100: Tarama
//! desenleri with an office's PAT file taken in, two rooms hatched from their own corners and Hizala making the
//! second's lines continue the first's, Tarama's options with Başlangıç and Ölçekli, an annotative hatch's
//! Öznitelikler and Desen's menu with the project's patterns after the library's. `tools_screens` takes them in the
//! dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tarama-desenleri,tarama-hizala,tarama-baslangic,tarama-olcekli,tarama-desen-proje cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::Point;
use kentos_domain::Slot;
use kentos_ui::snapshot::Input;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::hatch_patterns_window::Event;
use crate::tools_scenes::{E, N, Objects, click, hover, layer, open_edited, run};
use crate::tools_screens::{Pointed, Scene, caption_at, press_caption};

/// An office's PAT file: a brick bond, a lawn and a parquet in the drawing's metres.
pub(crate) const OFFICE_PAT: &str = ";; Ofis desenleri\n\
*TUGLA, Tuğla örgü\n0, 0,0, 0,6.35\n90, 0,0, 6.35,6.35, 6.35,-6.35\n\
*CIM, Çim alan\n60, 0,0, 2,4, 1,-3\n120, 0,0, 2,4, 1,-3\n\
*KAROLAJ, Kare karo 30 × 30\n0, 0,0, 0,6\n90, 0,0, 0,6\n";

const LEFT: [[f64; 2]; 4] = [[0.0, 0.0], [9.0, 0.0], [9.0, 7.0], [0.0, 7.0]];
const RIGHT: [[f64; 2]; 4] = [[9.0, 0.0], [17.5, 0.0], [17.5, 7.0], [9.0, 7.0]];

/// Two rooms side by side (1, 2) in a CAD project at 1:100, Açıklama's tab open, the view on them.
fn rooms(app: &mut App) {
    let mut o = Objects::new();
    o.path("duvar", &LEFT, true);
    o.path("duvar", &RIGHT, true);
    open_edited(app, o, |d| {
        d["layers"] = serde_json::json!([
            layer("duvar", "Duvarlar", "fg", 0.5),
            layer("tarama", "Tarama", "#3E63DD", 0.18)
        ]);
        d["activeLayer"] = serde_json::json!("tarama");
    });
    make_cad(app);
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 100.0;
        doc.model.set_settings(settings);
    }
    app.tab = "annotate";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 2.0,
            max_x: E + 19.5,
            max_y: N + 9.0,
        },
        24.0,
    );
}

/// The office's patterns taken in through Tarama desenleri.
fn take_in(app: &mut App) {
    run(app, "hatchPatterns.manage");
    let _ = app.update(Message::HatchPatterns(Event::Loaded(Some((
        "ofis.pat".to_owned(),
        OFFICE_PAT.to_owned(),
    )))));
}

/// Tarama desenleri: the office's PAT file taken in, Çim chosen.
fn window(app: &mut App) {
    rooms(app);
    take_in(app);
    let _ = app.update(Message::HatchPatterns(Event::Choose("CIM".into())));
}

/// ANSI31 at Ölçek 6 from each room's own bottom left corner: their lines break at the shared wall.
fn hatched_from_corners(app: &mut App) {
    app.memory.hatch_by_lines = false;
    app.memory.hatch_choice =
        kentos_geometry_core::tools::hatch::choice_named("ANSI31").expect("ANSI31");
    app.memory.hatch_scale = 6.0;
    app.memory.hatch_origin = 1;
    run(app, "tool.hatch");
    click(app, [2.0, 2.0]);
    click(app, [12.0, 2.0]);
    let _ = app.update(Message::Run("tool.cancel"));
}

/// Taramayı düzenle's Hizala: the right room's lines continue the left's.
fn align(app: &mut App) {
    rooms(app);
    hatched_from_corners(app);
    app.memory.hatch_edit_mode = 5;
    run(app, "tool.hatchEdit");
    click(app, [4.5, 0.0]);
    click(app, [13.0, 0.0]);
    hover(app, [18.5, 8.5]);
}

/// Tarama with Başlangıç sol alt over the right room: its preview from the room's corner, the options in the prompt.
fn origin(app: &mut App) {
    rooms(app);
    app.memory.hatch_choice =
        kentos_geometry_core::tools::hatch::choice_named("ANSI31").expect("ANSI31");
    app.memory.hatch_scale = 6.0;
    app.memory.hatch_origin = 1;
    app.command_expanded = true;
    run(app, "tool.hatch");
    hover(app, [13.0, 3.5]);
}

/// An annotative hatch of the project's TUGLA selected: Öznitelikler's Ölçekli rows and its Başlangıç.
fn annotative(app: &mut App) {
    rooms(app);
    take_in(app);
    let _ = app.update(Message::HatchPatterns(Event::Close));
    app.memory.hatch_user = Some(0);
    app.memory.hatch_scale = 4.0;
    app.memory.hatch_annotative = true;
    app.memory.hatch_origin = 1;
    run(app, "tool.hatch");
    click(app, [2.0, 2.0]);
    let _ = app.update(Message::Run("tool.cancel"));
    app.memory.hatch_annotative = false;
    app.memory.hatch_user = None;
    app.selection.set([Slot(3)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    hover(app, [18.5, 8.5]);
}

/// Tarama with the office's patterns taken in, before its Desen menu opens.
fn with_patterns(app: &mut App) {
    rooms(app);
    take_in(app);
    let _ = app.update(Message::HatchPatterns(Event::Close));
    app.memory.hatch_user = Some(1);
    app.command_expanded = true;
    run(app, "tool.hatch");
    hover(app, [4.0, 3.5]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("tarama-desenleri", window),
        ("tarama-hizala", align),
        ("tarama-baslangic", origin),
        ("tarama-olcekli", annotative),
    ]
}

/// Desen's menu scrolled to its end: the project's patterns after the library's.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![("tarama-desen-proje", with_patterns, |s, app| {
        let menu = caption_at(s, app, "Desen: CIM", false);
        press_caption(s, app, "Desen: CIM", false, false);
        let mut update = |app: &mut App, message: Message| {
            let _ = app.update(message);
        };
        s.settle(app, App::view, &mut update);
        let at = Point::new(menu.x + 160.0, menu.y - 200.0);
        s.input(app, App::view, &mut update, Input::Scroll(at, -60.0));
        s.input(
            app,
            App::view,
            &mut update,
            Input::Move(Point::new(at.x, at.y + 150.0)),
        );
    })]
}
