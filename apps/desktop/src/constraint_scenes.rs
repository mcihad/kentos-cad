//! The pictures of docs/adr/0335 (Kısıt): a room drawn by hand, its four lines not quite square, a circle and a
//! ground line beside it, in a CAD project. Paralel with the top edge picked and the pointer on the bottom one: the
//! room as the constraint would solve it follows, dashed, with the solver's word. Then the room made a rectangle by
//! `cad.constraints.edit` (corners, level bottom, parallel sides, a square corner, a held corner, its width and
//! height); Uzaklık on its bottom edge asking its value; 16 written: the rectangle widens and stays a rectangle. A
//! width the other dimensions contradict is refused, nothing written. Teğet with the ground line picked and the
//! pointer on the circle. `tools_screens` takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kisit-paralel,kisit-deger,kisit-genislik,kisit-celiski,kisit-teget cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! The Kısıtlar panel's list (its seventeen kinds, each its own icon) is the icon tour's `kisit`.
//!
//! Test code only.

use kentos_commands::{
    CadElement, CommandResult, ConstraintEdit, ConstraintRule, ConstraintSense, ConstraintsEdit,
    DistanceAxis, ElementRef, EntityId, NewConstraint, SubPlace,
};
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, constraints_edit};

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, method, open, typed};
use crate::tools_screens::Scene;

/// The room (1–4: bottom, right, top, left), a circle (5) and a ground line (6).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.line("cizim", [0.0, 0.0], [12.0, 0.6]);
    o.line("cizim", [12.0, 0.6], [12.4, 8.0]);
    o.line("cizim", [12.4, 8.0], [0.0, 8.2]);
    o.line("cizim", [0.0, 8.2], [0.0, 0.0]);
    o.circle("cizim", [22.0, 6.0], 2.5);
    o.line("cizim", [17.0, 0.0], [29.0, 0.0]);
    o
}

pub(crate) fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 3.0,
            min_y: N - 4.0,
            max_x: E + 31.0,
            max_y: N + 12.0,
        },
        24.0,
    );
}

fn part(app: &App, slot: u32, element: CadElement) -> ElementRef {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let uid = doc.uid(Slot(slot)).expect("an object");
    ElementRef {
        uid: EntityId(uid.into_bytes()),
        element,
    }
}

pub(crate) fn point(app: &App, slot: u32, place: SubPlace) -> ElementRef {
    part(app, slot, CadElement::Point { place })
}

pub(crate) fn segment(app: &App, slot: u32) -> ElementRef {
    part(
        app,
        slot,
        CadElement::Segment {
            part: 0,
            ring: 0,
            index: 0,
        },
    )
}

pub(crate) fn add(rule: ConstraintRule, value: Option<&str>) -> NewConstraint {
    NewConstraint {
        rule,
        sense: ConstraintSense::Drawing,
        value: value.map(str::to_owned),
        reference: false,
    }
}

/// The room made a rectangle 12 by 8 by the command.
pub(crate) fn rectangle(app: &mut App) {
    let mut constraints: Vec<NewConstraint> = (1..=4)
        .map(|i| {
            add(
                ConstraintRule::Coincident {
                    a: point(app, i, SubPlace::End),
                    b: point(app, i % 4 + 1, SubPlace::Start),
                },
                None,
            )
        })
        .collect();
    constraints.push(add(
        ConstraintRule::Horizontal {
            a: segment(app, 1),
            b: None,
        },
        None,
    ));
    constraints.push(add(
        ConstraintRule::Parallel {
            a: segment(app, 1),
            b: segment(app, 3),
        },
        None,
    ));
    constraints.push(add(
        ConstraintRule::Parallel {
            a: segment(app, 2),
            b: segment(app, 4),
        },
        None,
    ));
    constraints.push(add(
        ConstraintRule::Perpendicular {
            a: segment(app, 1),
            b: segment(app, 2),
        },
        None,
    ));
    constraints.push(add(
        ConstraintRule::Fix {
            a: point(app, 1, SubPlace::Start),
        },
        None,
    ));
    constraints.push(add(
        ConstraintRule::Distance {
            a: point(app, 2, SubPlace::Start),
            b: point(app, 2, SubPlace::End),
            axis: DistanceAxis::Aligned,
            left: false,
            parameter: "yukseklik".into(),
        },
        Some("8"),
    ));
    let input = ConstraintsEdit {
        edit: ConstraintEdit::Add { constraints },
        hold: Vec::new(),
        allow_flip: false,
        expected_revision: None,
    };
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    match constraints_edit::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { .. } => {}
        other => panic!("not written: {other:?}"),
    }
}

/// Paralel: the top edge picked, the pointer on the bottom one; the ribbon on Değiştir, its Kısıtlar panel.
fn parallel(app: &mut App) {
    drawn(app);
    let _ = app.update(crate::app::Message::RibbonTab("modify"));
    method(app, "tool.constraint", "P");
    click(app, [6.0, 8.1]);
    hover(app, [6.0, 0.3]);
}

/// The rectangle; Uzaklık on its bottom edge, its value asked.
fn value_asked(app: &mut App) {
    drawn(app);
    rectangle(app);
    method(app, "tool.constraint", "U");
    click(app, [0.0, 0.0]);
    click(app, [12.0, 0.0]);
    hover(app, [8.0, -2.0]);
}

/// 16 written: the rectangle widens.
fn widened(app: &mut App) {
    value_asked(app);
    typed(app, "16");
    hover(app, [20.0, -2.0]);
}

/// A width the top edge's dimension contradicts: refused, nothing written.
fn conflict(app: &mut App) {
    widened(app);
    method(app, "tool.constraint", "U");
    click(app, [16.0, 8.0]);
    click(app, [0.0, 8.0]);
    typed(app, "10");
    hover(app, [20.0, -2.0]);
}

/// Teğet: the ground line picked, the pointer on the circle.
fn tangent(app: &mut App) {
    drawn(app);
    method(app, "tool.constraint", "T");
    click(app, [25.0, 0.0]);
    hover(app, [24.5, 6.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kisit-paralel", parallel),
        ("kisit-deger", value_asked),
        ("kisit-genislik", widened),
        ("kisit-celiski", conflict),
        ("kisit-teget", tangent),
    ]
}
