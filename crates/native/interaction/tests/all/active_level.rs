//! Etkin kat (docs/adr/0713): new walls, columns and stairs go on the host's active level, written in their commands'
//! input; without one, as before, on none. The room tools look for walls on it while it has walls (an older drawing's
//! walls have no level: they keep looking at every level), and a level chosen in a tool is kept.

use crate::common::Bench;
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench(tool: &str) -> Bench {
    let mut b = Bench::new(tool);
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
    b
}

fn level_of(e: &Entity) -> Option<&str> {
    match e {
        Entity::Wall(w) => w.wall.arch.level.as_deref(),
        Entity::Column(c) => c.column.arch.level.as_deref(),
        Entity::Stair(s) => s.stair.arch.level.as_deref(),
        Entity::Space(s) => s.space.arch.level.as_deref(),
        _ => panic!("an architectural object: {e:?}"),
    }
}

/// A wall drawn through `pts` with the Duvar tool.
fn wall(b: &mut Bench, pts: &[(f64, f64)]) {
    b.start("wall");
    for &(x, y) in pts {
        b.click(x, y);
    }
    b.confirm();
    b.confirm();
}

#[test]
fn walls_columns_and_stairs_go_on_the_active_level() {
    let mut b = bench("wall");
    b.session.exit();
    b.level = Some("kat-1".into());
    wall(&mut b, &[(0., 0.), (5., 0.)]);
    assert_eq!(level_of(b.newest()), Some("kat-1"));
    b.start("column");
    b.click(2., 2.);
    b.confirm();
    assert_eq!(level_of(b.newest()), Some("kat-1"));
    b.start("stair");
    b.click(0., 5.);
    b.click(4., 5.);
    b.confirm();
    assert!(matches!(b.newest(), Entity::Stair(_)));
    assert_eq!(level_of(b.newest()), Some("kat-1"));
}

#[test]
fn without_an_active_level_they_go_on_none() {
    let mut b = bench("wall");
    b.session.exit();
    wall(&mut b, &[(0., 0.), (5., 0.)]);
    assert_eq!(level_of(b.newest()), None, "as before Etkin kat");
}

#[test]
fn a_room_follows_the_active_level_only_while_it_has_walls() {
    // An older drawing: walls with no level; the room tool keeps looking at every level.
    let mut b = bench("wall");
    b.session.exit();
    wall(&mut b, &[(0., 0.), (4., 0.), (4., 3.), (0., 3.), (0., 0.)]);
    b.level = Some("zemin".into());
    b.start("space");
    let shown = |b: &Bench| {
        b.session
            .settings(&Default::default())
            .into_iter()
            .find(|r| r.key == "L")
            .map(|r| r.value)
            .unwrap()
    };
    assert_eq!(shown(&b), "Tüm katlar", "no wall on Zemin kat yet");
    b.click(2., 1.5);
    assert_eq!(level_of(b.newest()), None);
    b.session.exit();
    // Walls on Zemin kat now: the room tool takes it, and says it follows.
    wall(&mut b, &[(10., 0.), (14., 0.), (14., 3.), (10., 3.), (10., 0.)]);
    b.start("space");
    assert_eq!(shown(&b), "Zemin kat (etkin kat)");
    b.click(12., 1.5);
    assert_eq!(level_of(b.newest()), Some("zemin"));
    // A level chosen in the tool is kept.
    assert!(b.run(|s, cx| s.configure("L", "id:kat-1", cx)));
    assert_eq!(shown(&b), "1. kat");
}

/// docs/adr/0713's 27 October addendum: a stair rises Etkin kat's height until one is typed; empty or Kat follows it
/// again. A level with no height leaves the tool's own.
#[test]
fn a_stair_rises_the_active_levels_height() {
    let mut b = bench("wall");
    let mut s = b.doc.settings().clone();
    s.arch = Some(
        serde_json::from_value(serde_json::json!({
            "levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.06},
                {"id": "kat-1", "name": "1. kat", "elevation": 3.06}
            ]
        }))
        .unwrap(),
    );
    b.doc.set_settings(s);
    b.session.exit();
    b.level = Some("zemin".into());
    let shown = |b: &Bench| {
        b.session
            .settings(&Default::default())
            .into_iter()
            .find(|r| r.key == "Y")
            .map(|r| r.value)
            .unwrap()
    };
    let rise = |e: &Entity| match e {
        Entity::Stair(s) => f64::from(s.stair.risers) * s.stair.riser_height,
        other => panic!("a stair: {other:?}"),
    };
    b.start("stair");
    assert_eq!(shown(&b), "3.060 m · katın");
    b.click(0., 0.);
    b.click(6., 0.);
    b.confirm();
    let first = b.doc.revision();
    assert!((rise(b.newest()) - 3.06).abs() < 1e-9, "{}", rise(b.newest()));
    // No stair_rise_level: the stair's rise is its level's.
    // A typed height is kept; Kat follows the level again.
    assert!(b.run(|s, cx| s.configure("Y", "2.80", cx)));
    assert_eq!(shown(&b), "2.800 m");
    b.click(0., 4.);
    b.click(6., 4.);
    b.confirm();
    assert!(b.doc.revision() > first);
    assert!((rise(b.newest()) - 2.8).abs() < 1e-9);
    assert!(b.run(|s, cx| s.configure("Y", "kat", cx)));
    assert_eq!(shown(&b), "3.060 m · katın");
    // 1. kat has no height: the tool's own.
    b.session.exit();
    b.level = Some("kat-1".into());
    b.start("stair");
    assert_eq!(shown(&b), "3.000 m");
}

/// docs/adr/0720 §4: Etkin kat changed while a tool runs, the pointer resting: the stair's height row and its
/// preview's rise follow the new level at once; a space tool's Kat follows a level with walls.
#[test]
fn a_running_tool_follows_the_active_level_at_once() {
    let mut b = bench("wall");
    let mut s = b.doc.settings().clone();
    s.arch = Some(
        serde_json::from_value(serde_json::json!({
            "levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.06},
                {"id": "kat-1", "name": "1. kat", "elevation": 3.06, "height": 2.8}
            ]
        }))
        .unwrap(),
    );
    b.doc.set_settings(s);
    b.session.exit();
    b.level = Some("zemin".into());
    let row = |b: &Bench, key: &str| {
        b.session
            .settings(&Default::default())
            .into_iter()
            .find(|r| r.key == key)
            .map(|r| r.value)
            .unwrap()
    };
    b.start("stair");
    b.click(0., 0.);
    b.move_to(6., 0.);
    assert_eq!(row(&b, "Y"), "3.060 m · katın");
    let before = b.session.preview(&Default::default());
    // The level changes on the status bar: no pointer move.
    b.level = Some("kat-1".into());
    b.run(|s, cx| s.level_changed(cx));
    assert_eq!(row(&b, "Y"), "2.800 m · katın", "the row follows at once");
    assert_ne!(b.session.preview(&Default::default()), before, "and the preview's rise");
    b.click(6., 0.);
    b.confirm();
    let Entity::Stair(st) = b.newest() else { panic!("a stair") };
    assert_eq!(st.stair.arch.level.as_deref(), Some("kat-1"));
    assert!((f64::from(st.stair.risers) * st.stair.riser_height - 2.8).abs() < 1e-9);
}
