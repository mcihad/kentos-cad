//! Metraj's pictures over the spaces (docs/adr/0342 §5): the quantities of the rooms, of the walls and of a shaft. The
//! spaces are drawn by the core's tools (space_scenes.rs). `arch_screens` takes them.

use crate::arch;
use crate::app::{App, Message};
use crate::tools_scenes::run;
use crate::space_scenes::{shaft_written, spaces};

/// The pictures of this file, for the module's `screens` (arch_screens.rs).
pub(crate) fn scenes() -> Vec<crate::tools_screens::Scene> {
    vec![
        ("saft-metraj", shaft_quantities),
        ("mahal-metraj", quantities),
        ("mahal-metraj-duvar", quantities_walls),
    ]
}

fn quantities(app: &mut App) {
    spaces(app);
    run(app, "arch.quantities");
}

fn quantities_walls(app: &mut App) {
    spaces(app);
    run(app, "arch.quantities");
    let _ = app.update(Message::from(arch::Event::Quantities(
        crate::quantities::Event::Tab(crate::quantities::Tab::Walls),
    )));
}

fn shaft_quantities(app: &mut App) {
    shaft_written(app);
    run(app, "arch.quantities");
}
