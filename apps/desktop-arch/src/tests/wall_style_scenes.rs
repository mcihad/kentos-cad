//! Mimari's wall pictures that need its windows (docs/adr/0339 §6, 0340 §6, 0699): Duvar stilleri with the section,
//! Birleşimleri denetle, a wall style renamed and its name given back by Geri al. The walls themselves are drawn by the
//! core's tools (wall_scenes.rs). `arch_screens` takes them:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=duvar-stilleri,duvar-birlesim-denetle,duvar-stili-adi-kaydedildi,duvar-stili-geri-alindi cargo test -p kentos-desktop-arch arch_screens -- --ignored --nocapture
//! ```

use crate::app::{App, Message};
use crate::arch;
use crate::tools_scenes::run;
use crate::wall_scenes::{joins_cross, room};
use kentos_domain::Slot;

/// The pictures of this file, for the module's `screens` (arch_screens.rs).
pub(crate) fn scenes() -> Vec<crate::tools_screens::Scene> {
    vec![
        ("duvar-stili-adi-kaydedildi", style_renamed),
        ("duvar-stili-geri-alindi", style_rename_undone),
        ("duvar-birlesim-denetle", joins_check),
        ("duvar-stilleri", styles),
    ]
}

/// Duvar stilleri over the room, the project's Dış duvar 25 chosen.
fn styles(app: &mut App) {
    room(app, 50.0);
    run(app, "arch.wallStyles");
}

/// The sharp corner's issue in Birleşimleri denetle.
fn joins_check(app: &mut App) {
    joins_cross(app);
    run(app, "arch.wallJoinCheck");
}

/// docs/adr/0699: the room's style renamed in Duvar stilleri and saved; the wall selected shows the new name.
fn style_renamed(app: &mut App) {
    room(app, 50.0);
    run(app, "arch.wallStyles");
    let _ = app.update(Message::from(arch::Event::WallStyles(
        crate::wall_styles::Event::Choose(Some("kentos-dis-duvar-25".into())),
    )));
    let _ = app.update(Message::from(arch::Event::WallStyles(
        crate::wall_styles::Event::Name("Dış duvar 25 QA".into()),
    )));
    let _ = app.update(Message::from(arch::Event::WallStyles(
        crate::wall_styles::Event::Save,
    )));
    app.selection.set([Slot(1)]);
}

/// One Geri al gives the old name back: the style table is in the step.
fn style_rename_undone(app: &mut App) {
    style_renamed(app);
    run(app, "edit.undo");
    app.selection.set([Slot(1)]);
}
