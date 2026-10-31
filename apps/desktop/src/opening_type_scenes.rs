//! The pictures of docs/adr/0772 (door and window types): the shared cases' corridor
//! (`fixtures/commands/v1/cad.openingtypes.edit.json` `setup`), twenty doors of the office type K01, one keeping its own
//! frame; Kapı ve pencere tipleri with K01 chosen and its threshold, casing and use zone typed (the plan counting what
//! follows); the change applied, near; the exception's Öznitelikler (its type, version, exception and four widths);
//! Aynı tiptekileri seç; and the door checks of `fixtures/commands/v1/cad.arch.audit.doors.json` on the Denetim tab.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tip-koridor,tip-pencere,tip-uygulandi,tip-yakin,tip-oznitelikler,tip-ayni,tip-denetim cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::OpeningTypeField;
use kentos_domain::Slot;
use kentos_ui::widget::docking;
use serde_json::Value;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::opening_types_window::Event;
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

const TYPES: &str = include_str!("../../../fixtures/commands/v1/cad.openingtypes.edit.json");
const DOORS: &str = include_str!("../../../fixtures/commands/v1/cad.arch.audit.doors.json");

fn open(app: &mut App, cases: &str, bounds: [f64; 4]) {
    let file: Value = serde_json::from_str(cases).expect("the cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.draft.snap = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: bounds[0],
            min_y: bounds[1],
            max_x: bounds[2],
            max_y: bounds[3],
        },
        24.0,
    );
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::OpeningTypes(e));
}

/// The corridor's first eight K01 doors and the exception (the sixth, its frame 7 cm).
fn corridor(app: &mut App) {
    open(app, TYPES, [-0.5, -3.0, 21.0, 4.0]);
}

/// Kapı ve pencere tipleri over K01: a 2 cm threshold, 7 cm casings and a 1.2 m use zone typed, the plan saying what
/// follows and which exception is kept.
fn window(app: &mut App) {
    corridor(app);
    run(app, "arch.openingTypes");
    for (field, text) in [
        (OpeningTypeField::Threshold, "0.02"),
        (OpeningTypeField::Casing, "0.07"),
        (OpeningTypeField::UseZone, "1.2"),
    ] {
        event(app, Event::Field(field, text.into()));
    }
}

/// Uygula: K01's version 2, every door but the one on the locked layer following, the exception keeping its frame.
fn applied(app: &mut App) {
    window(app);
    event(app, Event::Apply);
    event(app, Event::Close);
}

/// The change near at 1:20: the second door's threshold, casings and use zone, beside the exception.
fn near(app: &mut App) {
    applied(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: 3.0,
            min_y: -0.5,
            max_x: 5.2,
            max_y: 1.5,
        },
        24.0,
    );
}

/// The exception selected: Öznitelikler's Tip section, its version, the frame it keeps and its four widths.
fn props(app: &mut App) {
    applied(app);
    app.selection.set([Slot(10)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    // Genel and Geometri folded, so the whole Tip section shows.
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "geometry",
    )));
}

/// Aynı tiptekileri seç from the exception: every K01 door selected.
fn same(app: &mut App) {
    applied(app);
    app.selection.set([Slot(10)]);
    run(app, "arch.openingSameType");
}

/// Mimari denetim over the door cases: the leaves that meet a wall, a column or a cabinet, the narrow door.
fn audited(app: &mut App) {
    open(app, DOORS, [-0.5, -3.5, 8.5, 4.5]);
    run(app, "arch.audit");
    let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Group(Some(
        kentos_commands::ArchAuditCheck::Doors,
    ))));
    // The first finding pressed: its door and the wall it meets selected.
    let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Press(0)));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("tip-koridor", corridor),
        ("tip-pencere", window),
        ("tip-uygulandi", applied),
        ("tip-yakin", near),
        ("tip-oznitelikler", props),
        ("tip-ayni", same),
        ("tip-denetim", audited),
    ]
}
