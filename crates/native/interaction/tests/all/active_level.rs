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
