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
    s.set_wall_view(View::new(Detail::Detailed, 20.0));
    // The two lines between brick and plaster too.
    assert_eq!(count(&s), 7);
    s.set_wall_view(View::new(Detail::Schematic, 100.0));
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

/// Another view's pieces (docs/adr/0696's 29 October addendum): what a store set to that view holds, pieced from the
/// same joined drawing, while the store keeps its own view and pieces. An L of two three-layer walls with a door in
/// the first, a space and a line, at every detail and two scales.
#[test]
fn another_views_pieces_are_those_of_a_store_at_that_view() {
    let json = r##"[
        {"id":1,"layerId":"a","kind":"wall","pts":[{"x":0,"y":0},{"x":4,"y":0}],"width":0.2,"justify":"center",
         "components":[{"thickness":0.02,"detail":"detailed"},{"detail":"standard","hatch":{"pattern":"ANSI31","scale":1,"angle":0}},
                       {"thickness":0.02,"detail":"detailed"}],"fill":"#5A5A5A"},
        {"id":2,"layerId":"a","kind":"wall","pts":[{"x":4,"y":0},{"x":4,"y":3}],"width":0.2,"justify":"center",
         "components":[{"thickness":0.02,"detail":"detailed"},{"detail":"standard","hatch":{"pattern":"ANSI31","scale":1,"angle":0}},
                       {"thickness":0.02,"detail":"detailed"}],"fill":"#5A5A5A"},
        {"id":3,"layerId":"a","kind":"opening","at":2,"width":0.9,"form":"door",
         "place":[{"x":1.55,"y":0.1},{"x":1.55,"y":-0.1},{"x":2.45,"y":0.1},{"x":2.45,"y":-0.1}],"frame":0.05},
        {"id":4,"layerId":"a","kind":"line","a":{"x":0,"y":-2},"b":{"x":4,"y":-2}}
    ]"##;
    let mut a = Store::new();
    a.put_json(json).unwrap();
    a.set_hosts([(3.0, Some(1.0))]);
    let own = a.wall_view();
    let before: Vec<_> = (1..=4)
        .map(|id| a.get(f64::from(id)).unwrap().expanded.clone())
        .collect();
    for detail in [Detail::Schematic, Detail::Standard, Detail::Detailed] {
        for scale in [50.0, 200.0] {
            let view = View::new(detail, scale);
            let mut b = Store::new();
            b.put_json(json).unwrap();
            b.set_hosts([(3.0, Some(1.0))]);
            b.set_wall_view(view);
            for id in 1..=4 {
                let id = f64::from(id);
                assert_eq!(
                    a.expanded_at(id, view).as_deref(),
                    b.get(id).unwrap().expanded.as_deref(),
                    "{id} at {detail:?} 1:{scale}"
                );
            }
        }
    }
    // Detailed draws more of the wall than the store's own view, and the store itself was not touched.
    let detailed = View::new(Detail::Detailed, 50.0);
    assert!(
        a.expanded_at(1.0, detailed).unwrap().shapes.len()
            > before[0].as_ref().unwrap().shapes.len()
    );
    assert_eq!(a.wall_view(), own);
    let after: Vec<_> = (1..=4)
        .map(|id| a.get(f64::from(id)).unwrap().expanded.clone())
        .collect();
    assert_eq!(after, before);
    assert_eq!(a.expanded_at(4.0, detailed), None, "a line has no pieces");
    assert_eq!(a.expanded_at(9.0, detailed), None, "no such object");
}

/// A door and a window follow the store's view as its walls do (docs/adr/3010): the screen's zoom and a sheet's scale
/// redraw them. At 1:200 schematic the door is its leaf and the window its glass; at 1:100 the swing and the window's
/// faces too; at 1:50 the frame's sides as well. Another view's pieces (`expanded_at`) are the same as the store's at it.
#[test]
fn a_door_and_a_window_follow_the_stores_view() {
    let json = r##"[
        {"id":1,"layerId":"a","kind":"wall","pts":[{"x":0,"y":0},{"x":6,"y":0}],"width":0.25,"justify":"center"},
        {"id":2,"layerId":"a","kind":"opening","at":1.5,"width":0.9,"form":"door",
         "place":[{"x":1.05,"y":0.125},{"x":1.05,"y":-0.125},{"x":1.95,"y":0.125},{"x":1.95,"y":-0.125}],"frame":0.05},
        {"id":3,"layerId":"a","kind":"opening","at":4,"width":1.2,"form":"window",
         "place":[{"x":3.4,"y":0.125},{"x":3.4,"y":-0.125},{"x":4.6,"y":0.125},{"x":4.6,"y":-0.125}],"frame":0.06}
    ]"##;
    let mut s = Store::new();
    s.put_json(json).unwrap();
    s.set_hosts([(2.0, Some(1.0)), (3.0, Some(1.0))]);
    let count = |s: &Store, id: f64| {
        s.get(id)
            .unwrap()
            .expanded
            .as_ref()
            .map_or(0, |x| x.shapes.len())
    };
    let standard = View::new(Detail::Standard, 50.0);
    for (view, door, window) in [
        (View::new(Detail::Schematic, 200.0), 1, 1),
        (View::new(Detail::Schematic, 100.0), 2, 3),
        (standard, 4, 5),
    ] {
        s.set_wall_view(view);
        assert_eq!((count(&s, 2.0), count(&s, 3.0)), (door, window), "{view:?}");
    }
    // The store at 1:50 gives another view its own pieces: the schematic window without its frame's sides.
    let schematic = View::new(Detail::Schematic, 100.0);
    assert_eq!(s.wall_view(), standard);
    assert_eq!(s.expanded_at(3.0, schematic).unwrap().shapes.len(), 3);
    assert_eq!(count(&s, 3.0), 5, "its own pieces untouched");
}

/// The plan's cut height is the view's (docs/adr/3012): a window with its sill at 1.6 m leaves its wall whole and is
/// drawn dashed while the plan is cut at 1.10 or 1.20; cut at 1.70 it takes its hole out of its wall, every wall
/// joined again, and is drawn solid.
#[test]
fn a_higher_cut_cuts_a_high_window() {
    let json = r##"[
        {"id":1,"layerId":"a","kind":"wall","pts":[{"x":0,"y":0},{"x":6,"y":0}],"width":0.25,"justify":"center"},
        {"id":2,"layerId":"a","kind":"opening","at":3,"width":1.0,"form":"window","sill":1.6,"head":2.2,
         "place":[{"x":2.5,"y":0.125},{"x":2.5,"y":-0.125},{"x":3.5,"y":0.125},{"x":3.5,"y":-0.125}],"frame":0.06}
    ]"##;
    let mut s = Store::new();
    s.put_json(json).unwrap();
    s.set_hosts([(2.0, Some(1.0))]);
    let faces = |s: &Store| {
        s.get(1.0)
            .unwrap()
            .expanded
            .as_ref()
            .map_or(0, |x| x.shapes.len())
    };
    let dashed = |s: &Store| {
        let x = s.get(2.0).unwrap().expanded.clone().expect("its pieces");
        x.strokes
            .iter()
            .filter(|k| k.line_type.as_deref() == Some("dashed"))
            .count()
            == x.shapes.len()
    };
    let at = |cut: f64| View {
        cut,
        ..View::new(Detail::Standard, 50.0)
    };
    s.set_wall_view(at(1.2));
    let whole = faces(&s);
    assert!(dashed(&s), "over the cut: dashed");
    s.set_wall_view(at(1.7));
    assert!(
        faces(&s) > whole,
        "cut at 1.70 its hole splits its wall: {} > {whole}",
        faces(&s)
    );
    assert!(!dashed(&s), "cut: solid");
    s.set_wall_view(at(1.1));
    assert_eq!(faces(&s), whole, "back over the cut");
}

/// docs/adr/3013: at a line group's share (0.5: a 1:200 plan) a wall's and an opening's pieces take it on their own
/// share of their pen, a window's glass half of half; a line keeps its own. At a share of 1 nothing is written.
#[test]
fn a_line_group_weighs_the_models_lines_only() {
    let json = r##"[
        {"id":1,"layerId":"a","kind":"wall","pts":[{"x":0,"y":0},{"x":6,"y":0}],"width":0.25,"justify":"center"},
        {"id":2,"layerId":"a","kind":"opening","at":3,"width":1.0,"form":"window",
         "place":[{"x":2.5,"y":0.125},{"x":2.5,"y":-0.125},{"x":3.5,"y":0.125},{"x":3.5,"y":-0.125}],"frame":0.06},
        {"id":3,"layerId":"a","kind":"line","a":{"x":0,"y":5},"b":{"x":6,"y":5}}
    ]"##;
    let mut s = Store::new();
    s.put_json(json).unwrap();
    s.set_hosts([(2.0, Some(1.0))]);
    let shares = |s: &Store, id: f64| -> Vec<Option<f64>> {
        s.get(id)
            .and_then(|it| it.expanded.clone())
            .map(|x| x.strokes.iter().map(|k| k.weight_scale).collect())
            .unwrap_or_default()
    };
    let standard = View::new(Detail::Standard, 50.0);
    s.set_wall_view(standard);
    let (wall, window) = (shares(&s, 1.0), shares(&s, 2.0));
    assert!(!wall.is_empty() && !window.is_empty());
    s.set_wall_view(View {
        pens: 0.5,
        ..standard
    });
    let halved = |own: &[Option<f64>]| -> Vec<Option<f64>> {
        own.iter().map(|w| Some(w.unwrap_or(1.0) * 0.5)).collect()
    };
    assert_eq!(shares(&s, 1.0), halved(&wall), "the wall's pieces");
    assert_eq!(
        shares(&s, 2.0),
        halved(&window),
        "the window's pieces, its glass a quarter"
    );
    assert!(
        shares(&s, 2.0).contains(&Some(0.25)),
        "{:?}",
        shares(&s, 2.0)
    );
    assert!(
        s.get(3.0).unwrap().expanded.is_none(),
        "a line has no pieces to weigh"
    );
    // Back to each object's own pen.
    s.set_wall_view(standard);
    assert_eq!((shares(&s, 1.0), shares(&s, 2.0)), (wall, window));
}
