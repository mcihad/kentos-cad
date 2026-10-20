//! The pictures of docs/adr/0344 (tags and schedules): the room of docs/adr/0342 with its doors, windows and spaces, its
//! openings numbered (K1 …, P1 …, B1), every opening and space tagged with Hepsini etiketle, the door and window and the
//! room schedules placed beside it; CAD's Mimari tab with Etiket and Çizelge; Etiket on a door with the tag under the
//! cursor; a tag's Öznitelikler; Etiketleri denetle; Kapı ve pencere çizelgesi under the cursor; a door's number changed
//! in the schedule, its tag and row following. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=mimari-etiket-serit,mimari-etiket-plan,mimari-etiket-arac,mimari-etiket-oznitelikler,mimari-etiket-denetim,cizelge-arac,cizelge-izleme cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::{Entity, OpeningForm, OpeningsEdit, OpeningsOperation};
use kentos_domain::Slot;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::tools_scenes::{click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::fit;

/// The openings numbered by their form: K1 …, P1 …, B1.
fn numbered(app: &mut App) {
    let Some(doc) = app.document.as_mut() else {
        return;
    };
    for (prefix, form) in [
        ("K", OpeningForm::Door),
        ("P", OpeningForm::Window),
        ("B", OpeningForm::Opening),
    ] {
        let _ = kentos_native_application::openings_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            OpeningsEdit {
                uids: Vec::new(),
                operation: OpeningsOperation::Renumber {
                    prefix: prefix.into(),
                    start: 1,
                    form: Some(form),
                },
                expected_revision: None,
            },
        );
    }
}

/// The room numbered and tagged: every opening and space (Hepsini etiketle).
fn tagged(app: &mut App) {
    crate::space_scenes::spaces(app);
    numbered(app);
    fit(app, 12.8, 5.2);
    run(app, "tool.tagAll");
    typed(app, "");
    hover(app, [9.0, -1.0]);
}

/// The room tagged and its two schedules beside it.
fn scheduled(app: &mut App) {
    tagged(app);
    run(app, "tool.scheduleOpenings");
    click(app, [8.0, 5.0]);
    run(app, "tool.scheduleSpaces");
    click(app, [8.0, 2.4]);
    hover(app, [15.0, -1.0]);
}

fn ribbon(app: &mut App) {
    scheduled(app);
    app.tab = "arch";
}

fn plan(app: &mut App) {
    scheduled(app);
}

/// Etiket: the window on the top wall picked, its tag under the cursor with a leader, in a rectangle.
fn tag_tool(app: &mut App) {
    crate::space_scenes::spaces(app);
    numbered(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: crate::tools_scenes::E + 3.0,
            min_y: crate::tools_scenes::N + 3.0,
            max_x: crate::tools_scenes::E + 8.0,
            max_y: crate::tools_scenes::N + 6.8,
        },
        24.0,
    );
    app.command_expanded = true;
    run(app, "tool.tag");
    typed(app, "B");
    typed(app, "B");
    typed(app, "B");
    typed(app, "K");
    click(app, [5.6, 5.0]);
    hover(app, [6.6, 6.0]);
}

/// The first tag selected: Öznitelikler's tag rows.
fn props(app: &mut App) {
    tagged(app);
    let first = app.document.as_ref().and_then(|d| {
        d.model
            .entities()
            .find(|e| matches!(e, Entity::Tag(_)))
            .map(|e| Slot(e.base().id))
    });
    if let Some(s) = first {
        app.selection.set([s]);
    }
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Etiketleri denetle on the room: the spaces tagged, a window numbered P1 twice, the openings not yet tagged.
fn check(app: &mut App) {
    crate::space_scenes::spaces(app);
    numbered(app);
    if let Some(doc) = app.document.as_mut() {
        let p2: Vec<String> = doc
            .model
            .entities()
            .filter(
                |e| matches!(e, Entity::Opening(o) if o.opening.number.as_deref() == Some("P2")),
            )
            .filter_map(|e| doc.model.uid(Slot(e.base().id)).map(|u| u.to_string()))
            .collect();
        let _ = kentos_native_application::openings_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            OpeningsEdit {
                uids: p2,
                operation: OpeningsOperation::Set(kentos_commands::OpeningChange {
                    number: Some(Some("P1".into())),
                    ..Default::default()
                }),
                expected_revision: None,
            },
        );
    }
    run(app, "tool.tagAll");
    typed(app, "A");
    typed(app, "");
    run(app, "arch.tagsCheck");
}

/// Kapı ve pencere çizelgesi under the cursor, grouped by type with its total.
fn schedule_tool(app: &mut App) {
    tagged(app);
    app.command_expanded = true;
    run(app, "tool.scheduleOpenings");
    typed(app, "G");
    hover(app, [8.0, 5.0]);
}

/// The schedule's first row (the bare opening) renumbered B9 from its cell: the door's tag and the row follow, one step.
fn following(app: &mut App) {
    scheduled(app);
    let found = app.document.as_ref().and_then(|d| {
        d.model.entities().find_map(|e| match e {
            Entity::Table(t)
                if matches!(
                    t.source,
                    Some(kentos_commands::TableSource::Schedule {
                        of: kentos_commands::ScheduleOf::Openings,
                        ..
                    })
                ) =>
            {
                d.model
                    .uid(Slot(e.base().id))
                    .map(|u| (u.to_string(), Slot(e.base().id)))
            }
            _ => None,
        })
    });
    if let (Some((uid, slot)), Some(doc)) = (found, app.document.as_mut()) {
        let _ = kentos_native_application::schedules_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::SchedulesEdit {
                uid,
                operation: kentos_commands::ScheduleOperation::Cell {
                    row: 1,
                    column: 0,
                    value: "B9".into(),
                },
                expected_revision: None,
            },
        );
        app.selection.set([slot]);
    }
    hover(app, [15.0, -1.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("mimari-etiket-serit", ribbon),
        ("mimari-etiket-plan", plan),
        ("mimari-etiket-arac", tag_tool),
        ("mimari-etiket-oznitelikler", props),
        ("mimari-etiket-denetim", check),
        ("cizelge-arac", schedule_tool),
        ("cizelge-izleme", following),
    ]
}
