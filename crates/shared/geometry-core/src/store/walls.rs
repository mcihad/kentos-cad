//! Walls in the store (docs/adr/0339 §1, §3): a wall is drawn, picked and snapped by its pieces at the store's wall
//! view, its path's vertices are snap points, and a change of the view expands every wall again. Worked by hand on a
//! 4 m wall 0.2 m wide, centred on the x axis.

use super::Store;
use super::snap::SnapKind;
use crate::entity::Shape;
use crate::geom::wall::{Detail, View};
use crate::vec2::Vec2;

fn store() -> Store {
    let mut s = Store::new();
    s.put_json(
        r##"[{"id":1,"layerId":"a","kind":"wall","pts":[{"x":0,"y":0},{"x":4,"y":0}],"width":0.2,"justify":"center",
              "components":[{"thickness":0.02,"detail":"detailed"},{"detail":"standard","hatch":{"pattern":"ANSI31","scale":1,"angle":0}},
                            {"thickness":0.02,"detail":"detailed"}],"fill":"#5A5A5A"}]"##,
    )
    .unwrap();
    s
}

#[test]
fn a_wall_is_its_faces_box_and_is_picked_by_them_and_inside() {
    let s = store();
    let e = s.extent(Some(&[1.0])).unwrap();
    assert_eq!((e.min_x, e.min_y, e.max_x, e.max_y), (0.0, -0.1, 4.0, 0.1));
    // On its top face, and inside its hatched brick.
    assert_eq!(s.hit(Vec2::new(2.0, 0.105), 0.01), Some(1.0));
    assert_eq!(s.hit(Vec2::new(2.0, 0.0), 0.01), Some(1.0));
    // Edge tools do not act on it.
    assert!(s.hit_edge(Vec2::new(2.0, 0.1), 0.05).is_empty());
}

#[test]
fn a_wall_snaps_to_its_corners_and_its_path() {
    let s = store();
    let all = SnapKind::ALL.iter().fold(0, |k, s| k | s.bit());
    let at = |x: f64, y: f64| s.snap(Vec2::new(x, y), 0.03, all, None);
    assert_eq!(
        at(4.01, 0.11).map(|h| (h.point, h.kind)),
        Some((Vec2::new(4.0, 0.1), SnapKind::Endpoint))
    );
    assert_eq!(
        at(4.01, 0.0).map(|h| (h.point, h.kind)),
        Some((Vec2::new(4.0, 0.0), SnapKind::Endpoint))
    );
}

#[test]
fn the_view_decides_its_pieces() {
    let mut s = store();
    let count = |s: &Store| s.get(1.0).unwrap().shapes().count();
    // Standard: the brick's hatch, two faces, two caps.
    assert_eq!(count(&s), 5);
    s.set_wall_view(View {
        detail: Detail::Detailed,
        scale: 20.0,
    });
    // The two lines between brick and plaster too.
    assert_eq!(count(&s), 7);
    s.set_wall_view(View {
        detail: Detail::Schematic,
        scale: 100.0,
    });
    let it = s.get(1.0).unwrap();
    assert_eq!(it.shapes().count(), 5);
    assert!(
        matches!(it.shapes().next(), Some(Shape::Hatch { .. })),
        "its fill first"
    );
}

/// Two walls of an L, 4 m and 3 m, 0.2 m wide, put one by one, then the second removed.
#[test]
fn walls_that_meet_are_joined_and_part_when_one_goes() {
    let mut s = Store::new();
    let wall = |id: u32, a: (f64, f64), b: (f64, f64)| {
        format!(
            r#"[{{"id":{id},"layerId":"a","kind":"wall","pts":[{{"x":{},"y":{}}},{{"x":{},"y":{}}}],"width":0.2,"justify":"center"}}]"#,
            a.0, a.1, b.0, b.1
        )
    };
    s.put_json(&wall(1, (0.0, 0.0), (4.0, 0.0))).unwrap();
    // On its own: two faces and two caps.
    assert_eq!(s.get(1.0).unwrap().shapes().count(), 4);
    s.put_json(&wall(2, (4.0, 0.0), (4.0, 3.0))).unwrap();
    // Joined: the corner's caps go, the outer face reaches the corner.
    assert_eq!(s.get(1.0).unwrap().shapes().count(), 3);
    assert_eq!(s.get(2.0).unwrap().shapes().count(), 3);
    let e = s.extent(Some(&[1.0])).unwrap();
    assert!((e.max_x - 4.1).abs() < 1e-12);
    assert!(s.wall_issues().is_empty());
    s.remove(&[2.0]);
    assert_eq!(s.get(1.0).unwrap().shapes().count(), 4);
}

#[test]
fn walls_meeting_the_same_way_are_said() {
    let mut s = Store::new();
    s.put_json(
        r#"[{"id":1,"layerId":"a","kind":"wall","pts":[{"x":0,"y":0},{"x":4,"y":0}],"width":0.2,"justify":"center"},
            {"id":2,"layerId":"a","kind":"wall","pts":[{"x":4,"y":0},{"x":1,"y":0.01}],"width":0.2,"justify":"center"}]"#,
    )
    .unwrap();
    let issues = s.wall_issues();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::geom::wall_join::IssueCode::Overlap);
}

#[test]
fn changing_a_wall_level_rebuilds_both_sides_of_the_previous_join() {
    let mut s = Store::new();
    s.put_json(r#"[{"id":1,"layerId":"a","kind":"wall","pts":[{"x":0,"y":0},{"x":4,"y":0}],"width":0.2,"justify":"center","arch":{"level":"zemin"}}]"#).unwrap();
    let second = |level: &str| {
        format!(
            r#"[{{"id":2,"layerId":"a","kind":"wall","pts":[{{"x":4,"y":0}},{{"x":4,"y":3}}],"width":0.2,"justify":"center","arch":{{"level":"{level}"}}}}]"#
        )
    };
    for (level, count, max_x) in [("zemin", 3, 4.1), ("kat-1", 4, 4.0), ("zemin", 3, 4.1)] {
        s.put_json(&second(level)).unwrap();
        assert_eq!(
            s.get(1.0).unwrap().shapes().count(),
            count,
            "{level}: old neighbour refreshed"
        );
        assert_eq!(
            s.get(2.0).unwrap().shapes().count(),
            count,
            "{level}: changed wall refreshed"
        );
        assert!((s.extent(Some(&[1.0])).unwrap().max_x - max_x).abs() < 1e-12);
        assert!(s.wall_issues().is_empty());
    }
}
