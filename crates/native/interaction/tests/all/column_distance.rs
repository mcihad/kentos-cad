//! Kolonun geçici ölçüleri (docs/adr/0712): a selected column shows the clear distance from each of its sides to the
//! nearest parallel face (a wall's or another column's); a click on a number opens its text field, the value typed
//! moves the column square to that side in one undo step, the face across fixed; Esc and a bad value write nothing; a
//! column tied to grids says why it does not move at the click, before anything is typed.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};
use kentos_domain::Slot;
use kentos_interaction::{Level, ViewChange};
use serde_json::json;

fn bench() -> Bench {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b
}

fn add(b: &mut Bench, objects: Vec<serde_json::Value>) -> Vec<Slot> {
    let objects: Vec<_> = objects
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
    b.doc.add_many(objects, "Kaynaklar").unwrap()
}

/// A room 0..6 × 0..4 of 20 cm walls on their axes, a 40 × 40 cm column at (2, 1.5) selected, the pointer away.
fn room() -> (Bench, Slot) {
    let mut b = bench();
    let layer = b.doc.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    let wall = |a: (f64, f64), z: (f64, f64)| {
        json!({"kind":"wall","id":0,"layerId":layer,"attrs":{},"width":0.2,"pts":[p(a.0,a.1),p(z.0,z.1)]})
    };
    add(
        &mut b,
        vec![
            wall((0., 0.), (6., 0.)),
            wall((6., 0.), (6., 4.)),
            wall((6., 4.), (0., 4.)),
            wall((0., 4.), (0., 0.)),
        ],
    );
    let column = add(
        &mut b,
        vec![json!({"kind":"column","id":0,"layerId":layer,"attrs":{},"width":0.4,"depth":0.4,"at":p(2., 1.5)})],
    )[0];
    b.session.exit();
    b.selection.set([column]);
    b.move_to(4.5, 3.);
    (b, column)
}

fn at(b: &Bench, s: Slot) -> (f64, f64) {
    match b.doc.get(s) {
        Some(Entity::Column(c)) => (c.column.at.x - E, c.column.at.y - N),
        _ => panic!("a column"),
    }
}

#[test]
fn a_selected_column_shows_its_four_gaps_and_writes_nothing() {
    let (b, _) = room();
    let before = b.doc.to_snapshot_v2();
    let numbers = b.session.temporary_numbers();
    assert_eq!(numbers.len(), 4, "east, north, west, south: {numbers:?}");
    let preview = b.session.preview(&Default::default()).expect("the dimensions");
    // West: 1.8 − 0.1 = 1.700; east 6 − 0.1 − 2.2 = 3.700; south 1.3 − 0.1 = 1.200; north 3.9 − 1.7 = 2.200.
    for value in ["1.700", "3.700", "1.200", "2.200"] {
        assert!(
            preview.texts.iter().any(|t| t.text.contains(value)),
            "{value}: {:?}",
            preview.texts.iter().map(|t| &t.text).collect::<Vec<_>>()
        );
    }
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn a_number_typed_moves_the_column_in_one_step() {
    let (mut b, column) = room();
    let before = b.doc.to_snapshot_v2();
    // The west number: the leftmost.
    let west = b
        .session
        .temporary_numbers()
        .into_iter()
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    b.views.clear();
    b.click(west.x - E, west.y - N);
    assert!(b.session.is_running(), "the number's text field");
    assert!(
        b.views.iter().any(|v| matches!(v, ViewChange::Text(_))),
        "{:?}",
        b.views
    );
    assert_eq!(b.doc.to_snapshot_v2(), before, "the field writes nothing");
    b.run(|session, cx| session.text_typed(Some("1.00"), cx));
    assert!(!b.session.is_running());
    let (x, y) = at(&b, column);
    assert!((x - 1.3).abs() < 1e-9 && (y - 1.5).abs() < 1e-9, "({x}, {y})");
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.selection.ids(), [column], "the column stays selected");
    assert_eq!(b.doc.undo().as_deref(), Some("Mimari nesneyi düzenle"));
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities, "one undo takes it back");
}

#[test]
fn escape_and_a_bad_value_write_nothing() {
    let (mut b, _) = room();
    let before = b.doc.to_snapshot_v2();
    let n = b.session.temporary_numbers()[0];
    b.click(n.x - E, n.y - N);
    b.run(|session, cx| session.text_typed(None, cx));
    assert!(!b.session.is_running());
    let n = b.session.temporary_numbers()[0];
    b.click(n.x - E, n.y - N);
    b.run(|session, cx| session.text_typed(Some("uzak"), cx));
    assert!(b.session.is_running(), "a value that does not read keeps the field");
    b.run(|session, cx| session.text_typed(None, cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn a_column_on_grids_says_why_it_does_not_move() {
    let (mut b, _) = room();
    let layer = b.doc.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    let grids = add(
        &mut b,
        vec![
            json!({"kind":"grid","id":0,"layerId":layer,"attrs":{},"pts":[p(-1., 3.),p(7., 3.)],"name":"1"}),
            json!({"kind":"grid","id":0,"layerId":layer,"attrs":{},"pts":[p(4., -1.),p(4., 5.)],"name":"A"}),
        ],
    );
    let uids: Vec<String> = grids.iter().map(|&s| b.doc.uid(s).unwrap().to_string()).collect();
    let tied = add(
        &mut b,
        vec![json!({"kind":"column","id":0,"layerId":layer,"attrs":{},"width":0.4,"depth":0.4,"at":p(4., 3.),"grids":[uids[1],uids[0]],"place":"A-1"})],
    )[0];
    b.selection.set([tied]);
    b.move_to(5.5, 1.);
    let before = b.doc.to_snapshot_v2();
    let n = b.session.temporary_numbers();
    assert_eq!(n.len(), 4, "its gaps are shown all the same");
    b.click(n[0].x - E, n[0].y - N);
    assert!(!b.session.is_running(), "no field opens: nothing to type");
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text().is_some_and(|t| t.contains("akslara bağlı (A-1)")),
        "{:?}",
        b.last_text()
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

/// docs/adr/8049 §3: the south side's padlock keeps the column's 1.20 m from the south wall as a bond, nothing moving;
/// the wall shifted 0.5 m out, the column follows; the padlock again lets the bond go.
#[test]
fn the_south_padlock_bonds_the_column_to_its_wall() {
    let (mut b, column) = room();
    let south = b
        .session
        .temporary_numbers()
        .into_iter()
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap();
    let near = |b: &crate::common::Bench| {
        b.session
            .temporary_locks()
            .into_iter()
            .min_by(|p, q| {
                (p.x - south.x)
                    .hypot(p.y - south.y)
                    .total_cmp(&(q.x - south.x).hypot(q.y - south.y))
            })
            .expect("a padlock")
    };
    assert_eq!(
        b.session.temporary_locks().len(),
        4,
        "a padlock on each side facing a wall"
    );
    let lock = near(&b);
    b.click(lock.x - E, lock.y - N);
    let set = b.doc.constraints().clone();
    assert_eq!(set.constraints.len(), 1, "one bond");
    assert_eq!(set.parameters[0].expression, "1.2");
    let (x, y) = at(&b, column);
    assert!(
        (x - 2.0).abs() < 1e-9 && (y - 1.5).abs() < 1e-9,
        "nothing moves: ({x}, {y})"
    );
    assert!(
        b.last_text().is_some_and(|t| t.contains(
            "Mesafe bağlandı (d1 = 1.200 m): karşıdaki yüz kayınca kolon, kolon kayınca yüz izler."
        )),
        "{:?}",
        b.last_text()
    );
    // The south wall 0.5 m out: the column follows, 1.20 m from it.
    let wall = b
        .doc
        .entities()
        .find(|e| matches!(e, Entity::Wall(w) if w.wall.pts.iter().all(|p| (p.y - N).abs() < 1e-9)))
        .map(|e| b.doc.uid(Slot(e.base().id)).unwrap().to_string())
        .expect("the south wall");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut b.doc),
        "cad.walls.edit",
        kentos_commands::CallOp::Execute,
        json!({"uids": [wall], "operation": {"kind": "shift", "by": -0.5}}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let (x, y) = at(&b, column);
    assert!(
        (x - 2.0).abs() < 1e-9 && (y - 1.0).abs() < 1e-9,
        "({x}, {y})"
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Duvarı düzenle"));
    // The padlock, filled now, lets the bond go.
    b.selection.set([]);
    b.move_to(4.5, 3.);
    b.selection.set([column]);
    b.move_to(4.5, 3.);
    let lock = near(&b);
    b.click(lock.x - E, lock.y - N);
    assert!(
        b.doc.constraints().constraints.is_empty(),
        "the bond let go"
    );
}
