//! Aks ağı (docs/adr/0707): a click puts a whole grid system — its grids across x and y, labelled 1… and A…, and
//! with Kesişimlere kolon a column at every crossing tied to its grids — in one undo step; the preview is the
//! command's plan and writes nothing. İki nokta turns it to the pointer's direction; spacings are typed as “3*6”.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench() -> Bench {
    let mut b = Bench::new("gridSystem");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("gridSystem");
    b
}

fn labels(b: &Bench) -> Vec<String> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Grid(g) => Some(g.grid.label.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_click_puts_the_whole_system_in_one_step() {
    let mut b = bench();
    b.move_to(0., 0.);
    let before = b.doc.to_snapshot_v2();
    let preview = b.session.preview(&Default::default()).expect("the plan drawn");
    assert!(preview.strokes.len() >= 7, "the grids drawn: {}", preview.strokes.len());
    assert!(preview.texts.iter().any(|t| t.text == "A"), "the labels in their bubbles");
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(0., 0.);
    assert_eq!(labels(&b), ["1", "2", "3", "4", "A", "B", "C"]);
    let first = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Grid(g) => Some(g.grid.pts.clone()),
            _ => None,
        })
        .expect("grid 1");
    // Grid 1 runs up from 1.5 m below the first grid across y to 1.5 m past the last (10 m up).
    assert!((first[0].x - E).abs() < 1e-9 && (first[0].y - (N - 1.5)).abs() < 1e-9, "{first:?}");
    assert!((first[1].y - (N + 11.5)).abs() < 1e-9, "{first:?}");
    let written = b.doc.entities().count();
    b.confirm();
    assert_eq!(b.doc.entities().count(), written, "Enter after a click ends the tool, no second system");
    assert_eq!(b.doc.undo().as_deref(), Some("Aks"));
    assert_eq!(b.doc.entities().count(), 0, "one undo takes the system back");
}

#[test]
fn columns_stand_at_every_crossing_tied_to_their_grids() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("X", "2*4", cx)));
    assert!(b.run(|s, cx| s.configure("Y", "3", cx)));
    assert!(b.run(|s, cx| s.configure("K", "yes", cx)));
    assert!(b.run(|s, cx| s.configure("E", "40 cm", cx)));
    assert!(b.run(|s, cx| s.configure("S", "S1", cx)));
    assert!(!b.run(|s, cx| s.configure("X", "4 -2", cx)), "a gap below 0 is refused");
    b.click(10., 10.);
    let columns: Vec<_> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Column(c) => Some(c.column.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(columns.len(), 6, "3 × 2 crossings");
    assert_eq!(columns[0].place.as_deref(), Some("A-1"));
    assert_eq!(columns[5].place.as_deref(), Some("B-3"));
    assert_eq!(columns[0].tag.as_deref(), Some("S1"));
    assert!((columns[0].width - 0.4).abs() < 1e-12 && (columns[0].depth() - 0.5).abs() < 1e-12);
    let grids = columns[0].grids.clone().expect("tied to its grids");
    for uid in grids {
        let slot = b.doc.slot_of(uid.parse().expect("a uid")).expect("the grid is in the drawing");
        assert!(matches!(b.doc.get(slot), Some(Entity::Grid(_))));
    }
    assert_eq!(b.memory.grid_system_x.as_slice(), [4.0, 4.0], "the spacings stay for the next system");
}

#[test]
fn two_points_turn_the_system_to_the_pointer() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("P", "two", cx)));
    b.click(0., 0.);
    b.move_to(3.0, 3.0_f64.sqrt());
    b.click(3.0, 3.0_f64.sqrt());
    let across_y = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Grid(g) if g.grid.label == "A" => Some(g.grid.pts.clone()),
            _ => None,
        })
        .expect("grid A");
    let (dx, dy) = (across_y[1].x - across_y[0].x, across_y[1].y - across_y[0].y);
    // Far from the origin the stored points carry ~1e-9 m of rounding.
    assert!((dy.atan2(dx).to_degrees() - 30.0).abs() < 1e-6, "turned 30°: {across_y:?}");
}
