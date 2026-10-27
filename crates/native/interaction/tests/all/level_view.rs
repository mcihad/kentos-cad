//! Kat görünümü (docs/adr/0715): Yalnız etkin kat hides the other levels' objects (their openings with them) from the
//! drawing, picking and snapping; Etkin kat ve alt kat draws the level under the active one faint, snapped to but not
//! picked; objects on no level show on every level; Bütün katlar is as before.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};
use kentos_domain::Slot;
use kentos_interaction::level_view::{LevelLooks, LevelView, looks};
use kentos_interaction::Vec2;

/// Three levels 3 m apart; a 10 m wall on each (y 0, 4 and 8), a door on the 1. kat's, a line on none.
fn building() -> (Bench, [Slot; 5]) {
    let mut b = Bench::new("wall");
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.drawing_unit = Some(DrawingUnit::M);
    s.arch = Some(
        serde_json::from_value(serde_json::json!({
            "levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0},
                {"id": "kat-1", "name": "1. kat", "elevation": 3.0},
                {"id": "kat-2", "name": "2. kat", "elevation": 6.0}
            ]
        }))
        .unwrap(),
    );
    b.doc.set_settings(s);
    b.session.exit();
    let mut walls = Vec::new();
    for (level, y) in [("zemin", 0.0), ("kat-1", 4.0), ("kat-2", 8.0)] {
        b.level = Some(level.into());
        b.start("wall");
        b.click(0.0, y);
        b.click(10.0, y);
        b.confirm();
        b.confirm();
        walls.push(Slot(b.newest().base().id));
    }
    b.start("door");
    b.click(5.0, 4.3);
    b.session.exit();
    let door = Slot(b.newest().base().id);
    assert!(matches!(b.newest(), Entity::Opening(_)));
    b.start("line");
    b.click(0.0, 12.0);
    b.click(6.0, 12.0);
    b.confirm();
    b.session.exit();
    let line = Slot(b.newest().base().id);
    (b, [walls[0], walls[1], walls[2], door, line])
}

#[test]
fn a_view_hides_the_other_levels_and_fades_the_one_below() {
    let (b, [ground, first, second, door, _line]) = building();
    let ids = |v: &[Slot]| v.iter().map(|s| s.0).collect::<Vec<_>>();
    let seen = |view, active| {
        let l: LevelLooks = looks(&b.doc, view, active);
        (l.hidden, l.faded, l.elsewhere)
    };
    // Bütün katlar hides nothing; what is elsewhere is still known (a click takes the active level first).
    assert_eq!(
        seen(LevelView::All, Some("kat-1")),
        (vec![], vec![], ids(&[ground, second]))
    );
    assert_eq!(
        seen(LevelView::Active, Some("kat-1")),
        (ids(&[ground, second]), vec![], ids(&[ground, second])),
        "the door is on its wall's level; the line on none"
    );
    assert_eq!(
        seen(LevelView::Below, Some("kat-1")),
        (ids(&[second]), ids(&[ground]), ids(&[ground, second]))
    );
    assert_eq!(
        seen(LevelView::Below, Some("zemin")).0,
        ids(&[first, second, door]),
        "the lowest level has none below"
    );
    // Katsız: only what is on no level.
    assert_eq!(
        seen(LevelView::Active, None).0,
        ids(&[ground, first, second, door])
    );
}

#[test]
fn the_faint_level_is_snapped_to_not_picked_and_the_hidden_one_neither() {
    let (mut b, [ground, first, second, _door, line]) = building();
    b.spatial.sync(&b.doc);
    let at = |x: f64, y: f64| Vec2::new(E + x, N + y);
    let kinds = kentos_interaction::default_snap_kinds();
    let snapped = |b: &Bench, x: f64, y: f64| {
        b.spatial
            .snap(at(x, y), 0.3, kinds, None)
            .map(|h| Slot(h.id as u32))
    };
    // Bütün katlar: every wall picked and snapped to.
    assert_eq!(b.spatial.pick(at(3.0, 0.1), 0.2), Some(ground));
    assert_eq!(b.spatial.pick(at(3.0, 8.1), 0.2), Some(second));
    b.spatial
        .set_level_view(&b.doc, LevelView::Below, Some("kat-1".into()));
    assert_eq!(b.spatial.pick(at(3.0, 0.1), 0.2), None, "the faint level is not picked");
    assert_eq!(snapped(&b, 0.05, 0.05), Some(ground), "but snapped to");
    assert_eq!(b.spatial.pick(at(3.0, 8.1), 0.2), None, "the hidden level is not picked");
    assert_ne!(snapped(&b, 0.05, 8.05), Some(second), "nor snapped to");
    assert_eq!(b.spatial.pick(at(3.0, 4.1), 0.2), Some(first), "the active level is");
    assert_eq!(b.spatial.pick(at(3.0, 12.0), 0.2), Some(line), "and what is on no level");
    // A window round everything takes the active level's and the line only.
    let mut window = b.spatial.in_rect(at(-1.0, -1.0), at(11.0, 13.0), true);
    window.sort();
    assert!(!window.contains(&ground) && !window.contains(&second), "{window:?}");
    assert!(window.contains(&first) && window.contains(&line), "{window:?}");
    // Back to Bütün katlar: all of them again.
    b.spatial.set_level_view(&b.doc, LevelView::All, Some("kat-1".into()));
    assert_eq!(b.spatial.pick(at(3.0, 0.1), 0.2), Some(ground));
    assert!(b.spatial.level_looks().1.is_empty());
}

/// docs/adr/0715 §2: walls lying over each other on two levels, Bütün katlar: a click takes the active level's, and a
/// door goes in it; with the other level active, the other wall.
#[test]
fn a_click_takes_the_active_levels_wall_where_walls_lie_over_each_other() {
    let mut b = Bench::new("wall");
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.drawing_unit = Some(DrawingUnit::M);
    s.arch = Some(
        serde_json::from_value(serde_json::json!({
            "levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0},
                {"id": "kat-1", "name": "1. kat", "elevation": 3.0}
            ]
        }))
        .unwrap(),
    );
    b.doc.set_settings(s);
    b.session.exit();
    let mut walls = Vec::new();
    for level in ["zemin", "kat-1"] {
        b.level = Some(level.into());
        b.start("wall");
        b.click(0.0, 0.0);
        b.click(10.0, 0.0);
        b.confirm();
        b.confirm();
        walls.push(Slot(b.newest().base().id));
    }
    let at = Vec2::new(E + 3.0, N + 0.1);
    b.spatial.sync(&b.doc);
    b.spatial.set_level_view(&b.doc, LevelView::All, Some("kat-1".into()));
    assert_eq!(b.spatial.pick(at, 0.2), Some(walls[1]), "1. kat's first");
    assert_eq!(b.spatial.hits(at, 0.2), vec![walls[1], walls[0]]);
    b.start("door");
    b.click(5.0, 0.1);
    let Entity::Opening(door) = b.newest() else {
        panic!("a door")
    };
    let host = door.opening.host.clone();
    let wall_uid = |s: Slot| b.doc.uid(s).expect("a uid").to_string();
    assert_eq!(host, wall_uid(walls[1]), "the door is in 1. kat's wall");
    b.session.exit();
    b.spatial.set_level_view(&b.doc, LevelView::All, Some("zemin".into()));
    b.spatial.sync(&b.doc);
    assert_eq!(b.spatial.pick(at, 0.2), Some(walls[0]), "Zemin kat's now");
}
