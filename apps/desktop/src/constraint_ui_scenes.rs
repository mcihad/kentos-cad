//! The pictures of docs/adr/0338 §1: the room of `constraint_scenes` made a rectangle, its constraints marked by each
//! part (Kısıtları göster), the height's mark showing its name and value; the pointer on the bottom edge's Paralel
//! mark: both edges it names in the accent colour, its name above. Parametreler open on it; then the diagonal given a
//! dimension and set to 6 in the window, shorter than the height: refused, the constraints in the conflict marked.
//! `tools_screens` takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kisit-isaretler,kisit-isaret-vurgu,kisit-parametreler,kisit-parametre-celiski cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::Point;
use kentos_contracts::{
    CommandResult, ConstraintEdit, ConstraintRule, ConstraintsEdit, DistanceAxis, SubPlace,
};
use kentos_native_application::{ExecutionContext, constraints_edit};

use crate::app::{App, Message};
use crate::constraint_scenes::{add, drawn, point, rectangle};
use crate::parameters_window::Event as Parameters;
use crate::tools_screens::Scene;
use crate::viewport::Event;

/// The rectangle and its marks.
fn marked(app: &mut App) {
    drawn(app);
    rectangle(app);
    let _ = app.update(Message::RibbonTab("modify"));
}

/// The pointer on the first Paralel mark.
fn hovered(app: &mut App) {
    marked(app);
    let marks = app.constraint_marks().expect("marks");
    let g = marks
        .glyphs
        .iter()
        .find(|g| g.title == "Paralel")
        .expect("a Paralel mark");
    let [x, y] = app.viewport.camera.world_to_screen(g.at);
    // Its box stands 8 px right of the part and 26 px above it (constraint_view.rs); the pointer in its middle,
    // after the marks before it at the same place.
    let before = marks
        .glyphs
        .iter()
        .take_while(|o| !std::ptr::eq(*o, g))
        .filter(|o| {
            let [ox, oy] = app.viewport.camera.world_to_screen(o.at);
            ((ox - x).abs() < 2.0) && ((oy - y).abs() < 2.0)
        })
        .count() as f32;
    let at = Point::new(x as f32 + 8.0 + before * 20.0 + 9.0, y as f32 - 26.0 + 9.0);
    let _ = app.update(Message::Viewport(Event::Moved(at)));
}

/// Parametreler open on the rectangle.
fn window(app: &mut App) {
    marked(app);
    let _ = app.update(Message::Run("constraint.parameters"));
}

/// The diagonal given a dimension of its own (the width follows from it); set to 6, shorter than the height 8:
/// no rectangle has it, refused, the constraints in the conflict marked.
fn conflict(app: &mut App) {
    marked(app);
    let left = add(
        ConstraintRule::Distance {
            a: point(app, 1, SubPlace::Start),
            b: point(app, 2, SubPlace::End),
            axis: DistanceAxis::Aligned,
            left: false,
            parameter: "kosegen".into(),
        },
        None,
    );
    let input = ConstraintsEdit {
        edit: ConstraintEdit::Add {
            constraints: vec![left],
        },
        hold: Vec::new(),
        allow_flip: false,
        expected_revision: None,
    };
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    match constraints_edit::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { .. } => {}
        other => panic!("not written: {other:?}"),
    }
    let _ = app.update(Message::Run("constraint.parameters"));
    let _ = app.update(Message::Parameters(Parameters::Expression(
        "kosegen".into(),
        "6".into(),
    )));
    let _ = app.update(Message::Parameters(Parameters::Apply("kosegen".into())));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kisit-isaretler", marked),
        ("kisit-isaret-vurgu", hovered),
        ("kisit-parametreler", window),
        ("kisit-parametre-celiski", conflict),
    ]
}
