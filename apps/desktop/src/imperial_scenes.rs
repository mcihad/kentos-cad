//! The pictures of docs/adr/0262 (feet and inches): Proje ayarları in an
//! inch project with Uzunluk biçimi; a wall written 12'-6 1/2" in
//! Öznitelikler and the status bar in feet and inches; a length typed
//! `5'6-1/2"` while a line is drawn. `tools_screens` takes them in both
//! themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=inc-ayarlar,inc-duvar,inc-yazilan cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::{DrawingUnit, LengthStyle};
use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::project::settings::Section;
use crate::project::{Event, SettingsEvent};
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// Metres in an inch.
const INCH: f64 = 0.0254;

/// A room 150.5 × 96 inches and a line of 66.5 inches.
fn ground() -> Objects {
    let mut o = Objects::new();
    let (w, d) = (150.5 * INCH, 96.0 * INCH);
    o.path("cizim", &[[0.0, 0.0], [w, 0.0], [w, d], [0.0, d]], true);
    o.line("yol", [0.0, -1.0], [66.5 * INCH, -1.0]);
    o
}

/// An inch drawing, architectural, sixteenths.
fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    s.srid = 0;
    s.second_srid = None;
    s.drawing_unit = Some(DrawingUnit::In);
    s.length_style = Some(LengthStyle::Architectural);
    s.length_decimals = 4;
    // 1/4" = 1'-0" (1:48), an inch drawing's usual plan scale.
    s.plot_scale = 48.0;
    doc.model.set_settings(s);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.6,
            min_y: N - 1.6,
            max_x: E + 4.4,
            max_y: N + 2.8,
        },
        24.0,
    );
}

/// Proje ayarları › Birimler: İnç, Mimari, Duyarlık.
fn settings(app: &mut App) {
    drawn(app);
    let _ = app.update(Message::Run("file.settings"));
    let _ = app.update(Message::Project(Box::new(Event::Settings(
        SettingsEvent::Section(Section::Units),
    ))));
}

/// The room selected: its measures in feet and inches; the pointer's place too.
fn wall(app: &mut App) {
    drawn(app);
    app.selection.set([Slot(1)]);
    hover(app, [150.5 * INCH, 96.0 * INCH]);
}

/// A line from the room's corner, 5'6-1/2" typed: the length taken in inches.
fn typed_length(app: &mut App) {
    drawn(app);
    run(app, "tool.line");
    click(app, [0.0, 96.0 * INCH]);
    typed(app, "@5'6-1/2\",0");
    hover(app, [3.0, 3.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("inc-ayarlar", settings),
        ("inc-duvar", wall),
        ("inc-yazilan", typed_length),
    ]
}
