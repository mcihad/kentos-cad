//! The pictures of docs/adr/0342 (spaces and quantities): the room of docs/adr/0341 with its doors and windows, its
//! two rooms made spaces with the Mahal tool (Z-01 Salon, Z-02 Mutfak); CAD's Mimari tab with Mahal; Mahal under the
//! cursor (the command's plan, its area); Mahal by the walls' outer faces (Brüt); a space's Öznitelikler; Mahali böl
//! with its line; Metraj's spaces and walls; an outer wall deleted, the space left with its warning. `tools_screens`
//! takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=mahal-serit,mahal-oda,mahal-arac,mahal-brut,mahal-oznitelikler,mahal-bol,mahal-metraj,mahal-metraj-duvar,mahal-kayip cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;

use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::tools_scenes::{click, hover, run, typed};
use crate::tools_screens::Scene;

/// The room's walls on a level, Zemin kat of 3 m: what gives the walls their faces and the spaces their volume.
fn on_level(app: &mut App) {
    let Some(doc) = app.document.as_mut() else {
        return;
    };
    let mut settings = doc.model.settings().clone();
    settings.arch.get_or_insert_with(Default::default).levels = vec![kentos_commands::ArchLevel {
        id: "zemin".into(),
        name: "Zemin kat".into(),
        elevation: 0.0,
        height: Some(3.0),
    }];
    doc.model.set_settings(settings);
    let walls: Vec<String> = doc
        .model
        .entities()
        .filter(|e| matches!(e, kentos_commands::Entity::Wall(_)))
        .filter_map(|e| doc.model.uid(Slot(e.base().id)))
        .map(|u| u.to_string())
        .collect();
    let _ = kentos_native_application::walls_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        kentos_commands::WallsEdit {
            uids: walls,
            operation: kentos_commands::WallsOperation::Set(kentos_commands::WallChange {
                level: Some(Some("zemin".into())),
                ..Default::default()
            }),
            expected_revision: None,
        },
    );
}

/// The room with its openings and its two spaces on Zemin kat: slots 12 (Z-01 Salon) and 13 (Z-02 Mutfak).
pub(crate) fn spaces(app: &mut App) {
    crate::opening_scenes::room(app, 50.0);
    on_level(app);
    run(app, "tool.space");
    typed(app, "N");
    typed(app, "Z-01");
    typed(app, "A");
    typed(app, "Salon");
    click(app, [2.0, 2.5]);
    typed(app, "A");
    typed(app, "Mutfak");
    click(app, [5.6, 2.5]);
    typed(app, "");
    if let Some(doc) = app.document.as_mut() {
        let uids = kentos_interaction::properties::uids_of(&doc.model, &[Slot(12), Slot(13)]);
        let _ = kentos_native_application::spaces_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::SpacesEdit {
                uids,
                operation: kentos_commands::SpacesOperation::Set(kentos_commands::SpaceChange {
                    level: Some(Some("zemin".into())),
                    ..Default::default()
                }),
                expected_revision: None,
            },
        );
    }
    hover(app, [9.0, -1.0]);
}

fn ribbon(app: &mut App) {
    spaces(app);
    app.tab = "arch";
}

fn written(app: &mut App) {
    spaces(app);
}

/// Mahal with the cursor in the right room: the space the command would write, its area.
fn tool(app: &mut App) {
    crate::opening_scenes::room(app, 50.0);
    app.command_expanded = true;
    run(app, "tool.space");
    typed(app, "N");
    typed(app, "Z-02");
    hover(app, [5.6, 2.5]);
}

/// Mahal by the walls' outer faces: the left room to the outside of the outer walls and the inner wall's far face.
fn gross(app: &mut App) {
    crate::opening_scenes::room(app, 50.0);
    app.command_expanded = true;
    run(app, "tool.space");
    typed(app, "Y");
    typed(app, "Y");
    typed(app, "Y");
    hover(app, [2.0, 2.5]);
}

/// Z-01 Salon selected: Öznitelikler's space rows.
fn props(app: &mut App) {
    spaces(app);
    app.selection.set([Slot(12)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Mahali böl: the salon picked, the line across it at 2 m.
fn split(app: &mut App) {
    spaces(app);
    app.command_expanded = true;
    run(app, "tool.spaceSplit");
    click(app, [2.0, 2.5]);
    click(app, [2.0, -0.5]);
    hover(app, [2.0, 5.5]);
}

fn quantities(app: &mut App) {
    spaces(app);
    run(app, "arch.quantities");
}

fn quantities_walls(app: &mut App) {
    spaces(app);
    run(app, "arch.quantities");
    let _ = app.update(Message::Quantities(crate::quantities::Event::Tab(
        crate::quantities::Tab::Walls,
    )));
}

/// The bottom outer wall deleted: the salon and the kitchen keep their last boundary, each said with a warning.
fn lost(app: &mut App) {
    spaces(app);
    run(app, "tool.erase");
    click(app, [3.0, 0.0]);
    typed(app, "");
    hover(app, [9.0, -1.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("mahal-serit", ribbon),
        ("mahal-oda", written),
        ("mahal-arac", tool),
        ("mahal-brut", gross),
        ("mahal-oznitelikler", props),
        ("mahal-bol", split),
        ("mahal-metraj", quantities),
        ("mahal-metraj-duvar", quantities_walls),
        ("mahal-kayip", lost),
    ]
}
