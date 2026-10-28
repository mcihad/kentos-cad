//! Geçici ölçünün kalanı (docs/adr/0728): a side with a wall's face and a grid shows the face, a square at its
//! reference switching it to the grid (measured from the wall's axis) and back, the choice kept for the session; the
//! value typed moves the wall from the reference shown. An edge whose end is its path's shows its length, the end
//! that stays solid; typed, the other end moves, in one undo step; the square at the other end makes it the one that
//! stays. A reference on a locked layer still measures; a turned UCS measures and moves the same; Esc writes nothing.

use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, DrawingUnit, Entity, Workspace};
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn bench() -> Bench {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.session.exit();
    b
}

fn run(b: &mut Bench, name: &str, input: Value) -> Value {
    let answer = registry::run(&mut ExecutionContext::new(&mut b.doc), name, CallOp::Execute, input).expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn p(x: f64, y: f64) -> Value {
    json!({"x": E + x, "y": N + y})
}

/// A layer added, its id.
fn layer(b: &mut Bench, name: &str) -> String {
    let added = run(b, "cad.layers.edit", json!({"operation": "add", "kind": "layer", "name": name}));
    added["output"]["layer"].as_str().expect("its id").to_owned()
}

/// A 20 cm wall from (0, 0) to (5, 0), a wall 3 m north of its axis, a grid 2 m north of it; the wall's slot. With
/// `locked`, the wall across and the grid on a locked layer.
fn site(b: &mut Bench, locked: bool) -> Slot {
    let made = run(
        b,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [
            {"pts": [p(0., 0.), p(5., 0.)], "width": 0.2, "justify": "center"},
            {"pts": [p(0., 3.), p(5., 3.)], "width": 0.2, "justify": "center"},
        ]}),
    );
    let grids = if locked { layer(b, "Kilitli") } else { "cizim".to_owned() };
    run(b, "cad.grids.create", json!({"layerId": grids, "grids": [{"pts": [p(-2., 2.), p(9., 2.)], "label": "A"}]}));
    if locked {
        // The references on a locked layer: they measure, they do not move.
        let other = made["output"]["uids"][1].clone();
        run(b, "cad.entities.set", json!({"uids": [other], "layerId": grids, "operation": "layer"}));
        run(b, "cad.layers.set", json!({"operation": "locked", "layer": grids, "value": true}));
    }
    Slot(made["output"]["ids"][0].as_u64().expect("its slot") as u32)
}

fn wall_pts(b: &Bench, s: Slot) -> Vec<(f64, f64)> {
    match b.doc.get(s) {
        Some(Entity::Wall(w)) => w.wall.pts.iter().map(|p| (p.x - E, p.y - N)).collect(),
        _ => panic!("a wall"),
    }
}

fn texts(b: &Bench) -> Vec<String> {
    b.session
        .preview(&Default::default())
        .map(|p| p.texts.into_iter().map(|t| t.text).collect())
        .unwrap_or_default()
}

fn close(a: (f64, f64), x: f64, y: f64) -> bool {
    (a.0 - x).abs() < 1e-9 && (a.1 - y).abs() < 1e-9
}

#[test]
fn a_side_switches_to_its_grid_keeps_the_choice_and_moves_from_it() {
    let mut b = bench();
    let wall = site(&mut b, false);
    b.selection.set([wall]);
    b.move_to(2.5, 0.5);
    assert!(texts(&b).iter().any(|t| t.contains("2.800")), "the face first: {:?}", texts(&b));
    let squares = b.session.temporary_squares();
    let grid_side = squares
        .iter()
        .find(|q| (q.y - (N + 2.9)).abs() < 1e-6)
        .copied()
        .expect("a square at the face across, which has a grid too");
    b.click(grid_side.x - E, grid_side.y - N);
    assert!(!b.session.is_running(), "a square switches, it opens no field");
    assert!(texts(&b).iter().any(|t| t.contains("2.000")), "the grid now: {:?}", texts(&b));
    // Selected again: the choice stays for the session.
    b.selection.set([]);
    b.move_to(2.5, 0.5);
    b.selection.set([wall]);
    b.move_to(2.5, 0.5);
    assert!(texts(&b).iter().any(|t| t.contains("2.000")), "kept: {:?}", texts(&b));
    // Typed: the wall's axis 1 m from the grid, the grid where it is.
    let at = b.session.temporary_numbers()[0];
    b.click(at.x - E, at.y - N);
    assert!(b.session.is_running(), "the number's field");
    b.run(|s, cx| s.text_typed(Some("1"), cx));
    assert!(close(wall_pts(&b, wall)[0], 0., 1.), "{:?}", wall_pts(&b, wall));
    // The number's field ran and ended: the side still measures to the grid.
    b.move_to(2.5, 0.5);
    assert!(texts(&b).iter().any(|t| t.contains("1.000")), "the grid kept after the edit: {:?}", texts(&b));
    assert_eq!(b.doc.undo().as_deref(), Some("Duvarı düzenle"), "one step");
}

#[test]
fn the_length_moves_the_end_the_square_makes_the_other_stay_and_esc_writes_nothing() {
    let mut b = bench();
    let wall = site(&mut b, false);
    b.selection.set([wall]);
    b.move_to(2.5, 0.5);
    let length = b.session.temporary_length().expect("the length");
    assert!(texts(&b).iter().any(|t| t.contains("5.000")), "{:?}", texts(&b));
    // Esc: nothing.
    let before = b.doc.to_snapshot_v2();
    b.click(length.x - E, length.y - N);
    b.run(|s, cx| s.text_typed(None, cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    // 6 m: the start stays, the end goes to x 6.
    b.move_to(2.5, 0.5);
    let length = b.session.temporary_length().expect("the length");
    b.click(length.x - E, length.y - N);
    b.run(|s, cx| s.text_typed(Some("6"), cx));
    assert!(close(wall_pts(&b, wall)[1], 6., 0.), "{:?}", wall_pts(&b, wall));
    assert_eq!(b.doc.undo().as_deref(), Some("Duvarı düzenle"));
    // The square at the end: the end stays now, the start moves.
    b.move_to(2.5, 0.5);
    let end = b
        .session
        .temporary_squares()
        .into_iter()
        .find(|q| (q.x - (E + 5.0)).abs() < 1e-6 && q.y < N)
        .expect("a square at the end of the length's line, below the wall");
    b.click(end.x - E, end.y - N);
    let length = b.session.temporary_length().expect("the length");
    b.click(length.x - E, length.y - N);
    b.run(|s, cx| s.text_typed(Some("4"), cx));
    let got = wall_pts(&b, wall);
    assert!(close(got[0], 1., 0.) && close(got[1], 5., 0.), "{got:?}");
}

#[test]
fn references_on_a_locked_layer_measure_and_stay() {
    let mut b = bench();
    let wall = site(&mut b, true);
    b.selection.set([wall]);
    b.move_to(2.5, 0.5);
    assert!(texts(&b).iter().any(|t| t.contains("2.800")), "{:?}", texts(&b));
    let at = b.session.temporary_numbers()[0];
    b.click(at.x - E, at.y - N);
    b.run(|s, cx| s.text_typed(Some("2.5"), cx));
    assert!(close(wall_pts(&b, wall)[0], 0., 0.3), "{:?}", wall_pts(&b, wall));
}

#[test]
fn a_turned_ucs_measures_and_moves_the_same() {
    let mut b = bench();
    let wall = site(&mut b, false);
    let mut settings = b.doc.settings().clone();
    settings.ucs = Some(kentos_commands::Ucs {
        origin: kentos_commands::Vec2 { x: E + 1.0, y: N - 2.0 },
        angle: 0.6,
    });
    b.doc.set_settings(settings);
    b.selection.set([wall]);
    b.move_to(2.5, 0.5);
    assert!(texts(&b).iter().any(|t| t.contains("2.800")), "{:?}", texts(&b));
    let at = b.session.temporary_numbers()[0];
    b.click(at.x - E, at.y - N);
    b.run(|s, cx| s.text_typed(Some("2.5"), cx));
    assert!(close(wall_pts(&b, wall)[0], 0., 0.3), "{:?}", wall_pts(&b, wall));
}

#[test]
fn a_column_and_an_opening_switch_to_their_grids() {
    let mut b = bench();
    // A wall along y at x 0, a column 2 m east of it, a grid 1 m east of the wall: the column's left side has both.
    run(
        &mut b,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{"pts": [p(0., 3.), p(0., 7.)], "width": 0.2, "justify": "center"}]}),
    );
    let column = run(
        &mut b,
        "cad.columns.create",
        json!({"layerId": "cizim", "columns": [{"at": p(2., 5.), "width": 0.4}]}),
    );
    run(
        &mut b,
        "cad.grids.create",
        json!({"layerId": "cizim", "grids": [{"pts": [p(1., -5.), p(1., 15.)], "label": "1"}]}),
    );
    let column = Slot(column["output"]["ids"][0].as_u64().expect("its slot") as u32);
    b.selection.set([column]);
    b.move_to(2., 5.);
    assert!(texts(&b).iter().any(|t| t.contains("1.700")), "the wall's face: {:?}", texts(&b));
    let square = *b.session.temporary_squares().first().expect("a square on the left side");
    b.click(square.x - E, square.y - N);
    assert!(texts(&b).iter().any(|t| t.contains("1.000")), "the grid: {:?}", texts(&b));

    // A wall along x, a door 2 m along it (its middle at x 12), the grid at x 11 crossing its face.
    let wall = run(
        &mut b,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{"pts": [p(10., 20.), p(16., 20.)], "width": 0.2, "justify": "center"}]}),
    );
    run(
        &mut b,
        "cad.grids.create",
        json!({"layerId": "cizim", "grids": [{"pts": [p(11., 15.), p(11., 25.)], "label": "2"}]}),
    );
    let door = run(
        &mut b,
        "cad.openings.create",
        json!({"layerId": "cizim", "openings": [{"host": wall["output"]["uids"][0], "at": 2.0}]}),
    );
    let door = Slot(door["output"]["ids"][0].as_u64().expect("its slot") as u32);
    b.selection.set([door]);
    b.move_to(12., 20.5);
    let before = texts(&b);
    let square = b
        .session
        .temporary_squares()
        .into_iter()
        .find(|q| q.x < E + 11.5)
        .expect("a square on the start side");
    b.click(square.x - E, square.y - N);
    assert!(texts(&b).iter().any(|t| t.contains("1.000")), "the grid from its middle: {before:?} → {:?}", texts(&b));
    let at = b.session.temporary_numbers()[0];
    b.click(at.x - E, at.y - N);
    b.run(|s, cx| s.text_typed(Some("0"), cx));
    match b.doc.get(door) {
        Some(Entity::Opening(o)) => assert!((o.opening.at - 1.0).abs() < 1e-9, "its middle on the grid: {}", o.opening.at),
        _ => panic!("the door"),
    }
}
