//! Aks ağı (docs/adr/0707): a click puts a whole grid system — its grids across x and y, labelled 1… and A…, and
//! with Kesişimlere kolon a column at every crossing tied to its grids — in one undo step; the preview is the
//! command's plan and writes nothing. İki nokta turns it to the pointer's direction; spacings are typed as “3*6”.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench() -> Bench {
    bench_for("gridSystem")
}

/// A metre CAD plan with `tool` running.
fn bench_for(tool: &str) -> Bench {
    let mut b = Bench::new(tool);
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start(tool);
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

/// Eğrisel (docs/adr/0725 §1): radial grids 1… and arcs A… round the click, a column at every crossing along its radial
/// grid; clockwise turns the other way.
#[test]
fn the_curved_system_puts_radial_grids_arcs_and_columns() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("F", "radial", cx)));
    assert!(b.run(|s, cx| s.configure("X", "90/3", cx)), "three turns of 30°");
    assert!(b.run(|s, cx| s.configure("Y", "4", cx)));
    assert!(b.run(|s, cx| s.configure("R", "6", cx)));
    assert!(b.run(|s, cx| s.configure("N", "cw", cx)));
    assert!(b.run(|s, cx| s.configure("K", "yes", cx)));
    assert!(!b.run(|s, cx| s.configure("X", "4*100", cx)), "a whole turn or more is refused");
    b.click(0., 0.);
    assert_eq!(labels(&b), ["1", "2", "3", "4", "A", "B"]);
    let columns: Vec<_> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Column(c) => Some(c.column.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(columns.len(), 8, "4 radial × 2 arcs");
    // Clockwise: grid 2 at −30°, its column on arc A 6 m out, turned along it.
    let a2 = columns.iter().find(|c| c.place.as_deref() == Some("A-2")).expect("A-2");
    let t = (-30f64).to_radians();
    assert!((a2.at.x - (E + 6.0 * t.cos())).abs() < 1e-6 && (a2.at.y - (N + 6.0 * t.sin())).abs() < 1e-6, "{:?}", a2.at);
    assert!((a2.rotation - t).abs() < 1e-12);
    let arc = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Grid(g) if g.grid.label == "A" => g.grid.bulges.clone(),
            _ => None,
        })
        .expect("arc A");
    assert!(arc[0] < 0.0, "a clockwise arc bends the other way: {arc:?}");
}

/// Eşit böl (docs/adr/0725 §4): with İki nokta the x gaps split the second click's distance; Kesik (§5) writes every
/// grid's ends.
#[test]
fn equal_division_splits_the_second_click_and_gapped_grids_keep_their_ends() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("P", "two", cx)));
    assert!(b.run(|s, cx| s.configure("Q", "yes", cx)));
    assert!(b.run(|s, cx| s.configure("C", "12", cx)));
    assert!(!b.run(|s, cx| s.configure("C", "1200", cx)), "more than 1000 mm is refused");
    b.click(0., 0.);
    b.move_to(20.0, 0.0);
    b.click(20.0, 0.0);
    let xs: Vec<f64> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Grid(g) if g.grid.label.parse::<u32>().is_ok() => Some(g.grid.pts[0].x - E),
            _ => None,
        })
        .collect();
    assert_eq!(xs.len(), 4);
    for (k, x) in xs.iter().enumerate() {
        assert!((x - 20.0 / 3.0 * k as f64).abs() < 1e-9, "{xs:?}");
    }
    assert!(b.doc.entities().all(|e| !matches!(e, Entity::Grid(g) if g.grid.ends != Some(12.0))));
}

/// Kolon (docs/adr/0725 §2, §3): L with its legs, put by its lower left; the legs half its sides until typed.
#[test]
fn the_column_tool_puts_an_l_by_its_lower_left() {
    let mut b = bench_for("column");
    assert!(b.run(|s, cx| s.configure("B", "lShape", cx)));
    assert!(b.run(|s, cx| s.configure("E", "0.6", cx)));
    assert!(b.run(|s, cx| s.configure("Y", "0.5", cx)));
    assert!(b.run(|s, cx| s.configure("H", "bottomLeft", cx)));
    assert!(!b.run(|s, cx| s.configure("L", "0.2 -1", cx)), "a leg below 0 is refused");
    b.click(1.0, 2.0);
    assert!(b.run(|s, cx| s.configure("L", "0.2 0.3", cx)));
    b.click(4.0, 2.0);
    let columns: Vec<_> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Column(c) => Some(c.column.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].legs, Some([0.25, 0.3]), "half its depth and width");
    assert_eq!(columns[1].legs, Some([0.2, 0.3]));
    assert!(columns.iter().all(|c| c.shape == kentos_commands::ColumnShape::LShape
        && c.anchor == kentos_commands::ColumnAnchor::BottomLeft));
}
