//! The pictures of docs/adr/0320: associative dimensions of every kind made with `cad.dimensions.create` (a radius, a
//! diameter past its arc's end, an arc length, an angle between two edges, a room's side with a Zincir and a Baz
//! ölçü from it), what a selected one is bound to (olcu-bag), the same after their objects moved, turned and grew
//! (olcu-izleme), the chain after the room moved (olcu-zincir), and the bonds near on paper (olcu-yakin).
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=olcu-bag,olcu-yakin,olcu-izleme,olcu-zincir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::{
    CommandResult, DimensionMeasure, DimensionsCreate, EdgeNear, EntitiesTransform, MeasuredPoint, NewLook, SubPlace,
    Transform, Vec2 as Point,
};
use kentos_domain::Slot;
use kentos_native_application::ExecutionContext;
use kentos_native_application::dimension_create::dimensions_create;
use kentos_native_application::transform;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited};
use crate::tools_screens::Scene;

fn p(x: f64, y: f64) -> Point {
    Point { x: E + x, y: N + y }
}

/// The drawing: a circle, an arc, two walls meeting at an angle, a room.
fn drawing(app: &mut App) {
    let mut o = Objects::new();
    o.circle("duvar", [2.0, 2.0], 1.5);
    o.arc("duvar", [7.0, 1.0], 2.0, 0.0, std::f64::consts::FRAC_PI_2);
    o.line("duvar", [11.0, 1.0], [17.0, 1.0]);
    o.line("duvar", [12.0, 0.0], [12.0, 5.0]);
    o.path("duvar", &[[0.0, 7.0], [6.0, 7.0], [6.0, 11.0], [0.0, 11.0]], true);
    open_edited(app, o, |d| {
        d["layers"] = serde_json::json!([
            layer("duvar", "Duvar", "#3E63DD", 0.35),
            layer("olcu", "Ölçü", "#E5484D", 0.18),
        ]);
        d["settings"]["plotScale"] = serde_json::json!(50);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.5,
            min_y: N - 1.5,
            max_x: E + 22.0,
            max_y: N + 13.0,
        },
        24.0,
    );
}

fn uid(app: &App, slot: u32) -> String {
    app.document.as_ref().expect("a drawing").model.uid(Slot(slot)).expect("an object").to_string()
}

fn edge(app: &App, slot: u32, x: f64, y: f64) -> EdgeNear {
    EdgeNear { uid: uid(app, slot), near: p(x, y) }
}

fn place(app: &App, slot: u32, place: SubPlace) -> MeasuredPoint {
    MeasuredPoint::Place { uid: uid(app, slot), place, path: Vec::new() }
}

fn corner(app: &App, index: u32) -> MeasuredPoint {
    place(app, 5, SubPlace::Vertex { part: 0, ring: 0, index })
}

/// Writes a dimension; its persistent id.
fn make(app: &mut App, measure: DimensionMeasure, at: Point) -> String {
    let input = DimensionsCreate {
        measure,
        at,
        offset: None,
        layer_id: "olcu".into(),
        dimension_style: None,
        height: None,
        mask: None,
        look: NewLook::default(),
        expected_revision: None,
    };
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    match dimensions_create::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, .. } => output.created[0].clone(),
        other => panic!("not written: {other:?}"),
    }
}

fn change(app: &mut App, slots: &[u32], transform: Transform) {
    let uids = slots.iter().map(|s| uid(app, *s)).collect();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let input = EntitiesTransform { uids, transform, copy: None, layer_id: None, expected_revision: None };
    assert!(matches!(
        transform::execute(&mut ExecutionContext::new(doc), input),
        CommandResult::Completed { .. }
    ));
}

/// Every kind made; the slots of the radius, the angle and the room's side.
fn dimensions(app: &mut App) {
    let m = DimensionMeasure::Radius { of: edge(app, 1, 3.5, 2.0) };
    make(app, m, p(3.6, 3.4));
    let m = DimensionMeasure::Diameter { of: edge(app, 2, 9.0, 1.0) };
    make(app, m, p(4.6, 3.6));
    let m = DimensionMeasure::ArcLength { of: edge(app, 2, 9.0, 1.0), between: None };
    make(app, m, p(9.0, 3.6));
    let m = DimensionMeasure::Angle { first: edge(app, 3, 16.0, 1.0), second: edge(app, 4, 12.0, 4.0) };
    make(app, m, p(13.6, 2.4));
    let m = DimensionMeasure::Aligned { a: corner(app, 0), b: corner(app, 1) };
    let side = make(app, m, p(3.0, 6.0));
    let m = DimensionMeasure::Continue { base: side.clone(), to: MeasuredPoint::At { at: p(10.0, 7.0) } };
    make(app, m, p(0.0, 0.0));
    let m = DimensionMeasure::Baseline { base: side, to: MeasuredPoint::At { at: p(10.0, 7.0) } };
    make(app, m, p(0.0, 0.0));
}

/// The radius and the angle selected: what they are bound to dashed, their places marked.
fn bonds(app: &mut App) {
    drawing(app);
    dimensions(app);
    app.selection.set([Slot(6), Slot(9)]);
    hover(app, [20.0, 12.0]);
}

/// The circle moved, the arc turned a quarter, the second wall moved right, the room grown: every dimension where its
/// objects are.
fn followed(app: &mut App) {
    bonds(app);
    change(app, &[1], Transform::Move { dx: 0.5, dy: 0.5 });
    change(app, &[2], Transform::Rotate { center: p(7.0, 1.0), angle: std::f64::consts::FRAC_PI_2 });
    change(app, &[4], Transform::Move { dx: 1.5, dy: 0.0 });
    change(app, &[5], Transform::Scale { center: p(0.0, 7.0), factor: 1.25 });
    app.selection.set([Slot(6), Slot(9)]);
    hover(app, [20.0, 12.0]);
}

/// The room moved: its side and the Zincir and Baz from it go with it, their free ends stay.
fn chained(app: &mut App) {
    drawing(app);
    dimensions(app);
    change(app, &[5], Transform::Move { dx: 0.0, dy: 0.75 });
    app.selection.set([Slot(10), Slot(11), Slot(12)]);
    hover(app, [20.0, 12.0]);
}

/// The same near on paper, the text at its true size and the pointer on the radius's value (docs/adr/0055's and
/// 0320's 17 Ekim ekleri): the turned values' letters and halos, and the dashes on the circle itself.
fn near(app: &mut App) {
    bonds(app);
    let _ = app.settings.choose(&[
        ("graphics.annotationSize", serde_json::Value::from("true")),
        ("appearance.drawingBackground", serde_json::Value::from("paper")),
    ]);
    app.apply_settings();
    app.backdrop = crate::appearance::Backdrop::Paper;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + 2.5, min_y: N + 2.5, max_x: E + 5.0, max_y: N + 4.5 },
        24.0,
    );
    hover(app, [3.42, 3.3]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("olcu-bag", bonds), ("olcu-yakin", near), ("olcu-izleme", followed), ("olcu-zincir", chained)]
}
