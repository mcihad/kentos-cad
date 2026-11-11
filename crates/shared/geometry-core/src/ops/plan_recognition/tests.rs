//! An office's plan drawn as lines, as AutoCAD's EXPLODE leaves it, read back.

use super::*;

/// A plan's line work, each call one source.
struct Draw {
    lines: Lines,
    sources: usize,
}

impl Draw {
    fn new() -> Self {
        Self {
            lines: Lines::default(),
            sources: 0,
        }
    }

    fn next(&mut self) -> usize {
        self.sources += 1;
        self.sources - 1
    }

    fn line(&mut self, a: (f64, f64), b: (f64, f64), role: Role) -> usize {
        let source = self.next();
        self.lines.segments.push(Segment {
            a: Vec2::new(a.0, a.1),
            b: Vec2::new(b.0, b.1),
            role,
            source,
        });
        source
    }

    /// An arc about `c` from `a0` over `sweep` degrees.
    fn arc(&mut self, c: (f64, f64), r: f64, a0: f64, sweep: f64, role: Role) -> usize {
        let source = self.next();
        self.lines.arcs.push(Arc {
            c: Vec2::new(c.0, c.1),
            r,
            a0: a0 * PI / 180.0,
            sweep: sweep * PI / 180.0,
            role,
            source,
        });
        source
    }

    /// A column's square about `c`, `size` wide.
    fn column(&mut self, c: (f64, f64), size: f64) -> usize {
        let source = self.next();
        let h = size / 2.0;
        self.lines.rings.push(Ring {
            pts: vec![
                Vec2::new(c.0 - h, c.1 - h),
                Vec2::new(c.0 + h, c.1 - h),
                Vec2::new(c.0 + h, c.1 + h),
                Vec2::new(c.0 - h, c.1 + h),
            ],
            role: Role::Column,
            source,
        });
        source
    }

    /// A line along x at `y` from `x0` to `x1`, broken at the gaps (each its own line).
    fn along_x(&mut self, y: f64, x0: f64, x1: f64, gaps: &[(f64, f64)]) {
        let mut x = x0;
        for &(g0, g1) in gaps {
            self.line((x, y), (g0, y), Role::Wall);
            x = g1;
        }
        self.line((x, y), (x1, y), Role::Wall);
    }

    /// A line along y at `x` from `y0` to `y1`, broken at the gaps.
    fn along_y(&mut self, x: f64, y0: f64, y1: f64, gaps: &[(f64, f64)]) {
        let mut y = y0;
        for &(g0, g1) in gaps {
            self.line((x, y), (x, g0), Role::Wall);
            y = g1;
        }
        self.line((x, y), (x, y1), Role::Wall);
    }

    /// Jambs across a wall along x at `x`, its faces at `y0` and `y1`.
    fn jamb_x(&mut self, x: f64, y0: f64, y1: f64) {
        self.line((x, y0), (x, y1), Role::Wall);
    }
}

/// A room of 4 × 3 m between its walls' centre lines, 0.2 m walls, its faces broken at the gaps given per side
/// (outer, inner): south, north, west, east.
fn room(
    d: &mut Draw,
    south: &[(f64, f64)],
    north: &[(f64, f64)],
    inner_south: &[(f64, f64)],
    inner_north: &[(f64, f64)],
) {
    d.along_x(-0.1, -0.1, 4.1, south);
    d.along_x(0.1, 0.1, 3.9, inner_south);
    d.along_x(3.1, -0.1, 4.1, north);
    d.along_x(2.9, 0.1, 3.9, inner_north);
    d.along_y(-0.1, -0.1, 3.1, &[]);
    d.along_y(0.1, 0.1, 2.9, &[]);
    d.along_y(4.1, -0.1, 3.1, &[]);
    d.along_y(3.9, 0.1, 2.9, &[]);
}

fn walls(r: &Recognized) -> Vec<(f64, f64, f64, f64, f64)> {
    let mut w: Vec<(f64, f64, f64, f64, f64)> = r
        .walls
        .iter()
        .map(|w| (w.a.x, w.a.y, w.b.x, w.b.y, w.width))
        .collect();
    w.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.total_cmp(&b.2))
            .then(a.3.total_cmp(&b.3))
    });
    w
}

fn near(a: &[(f64, f64, f64, f64, f64)], b: &[(f64, f64, f64, f64, f64)]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(p, q)| {
            (p.0 - q.0).abs() < 1e-9
                && (p.1 - q.1).abs() < 1e-9
                && (p.2 - q.2).abs() < 1e-9
                && (p.3 - q.3).abs() < 1e-9
                && (p.4 - q.4).abs() < 1e-9
        })
}

const ROOM: [(f64, f64, f64, f64, f64); 4] = [
    (0.0, 0.0, 0.0, 3.0, 0.2),
    (0.0, 0.0, 4.0, 0.0, 0.2),
    (0.0, 3.0, 4.0, 3.0, 0.2),
    (4.0, 0.0, 4.0, 3.0, 0.2),
];

fn wall_at(r: &Recognized, y: f64) -> usize {
    r.walls
        .iter()
        .position(|w| (w.a.y - y).abs() < 1e-9 && (w.b.y - y).abs() < 1e-9)
        .unwrap()
}

#[test]
fn a_room_s_outlines_are_four_walls_meeting_at_their_corners() {
    let mut d = Draw::new();
    room(&mut d, &[], &[], &[], &[]);
    let r = recognize(&d.lines, &Options::default());
    assert!(near(&walls(&r), &ROOM), "{:#?}", walls(&r));
    assert!(r.openings.is_empty());
    assert_eq!(r.used, (0..d.sources).collect::<Vec<_>>());
}

#[test]
fn a_door_s_swing_makes_its_gap_a_door_hinged_where_the_arc_turns() {
    let mut d = Draw::new();
    room(&mut d, &[(1.0, 1.9)], &[], &[(1.0, 1.9)], &[]);
    d.jamb_x(1.0, -0.1, 0.1);
    d.jamb_x(1.9, -0.1, 0.1);
    // The leaf and its swing into the room: hinged at the start jamb on the inner face.
    let leaf = d.line((1.0, 0.1), (1.0, 1.0), Role::Door);
    let swing = d.arc((1.0, 0.1), 0.9, 0.0, 90.0, Role::Door);
    let r = recognize(&d.lines, &Options::default());
    assert!(near(&walls(&r), &ROOM), "{:#?}", walls(&r));
    let south = wall_at(&r, 0.0);
    assert_eq!(
        r.openings,
        vec![Opening {
            wall: south,
            from: 1.0,
            to: 1.9,
            form: Form::Door,
            leaves: 1,
            hand: End::Start,
            side: Side::Left,
            swing: None,
        }]
    );
    assert!(r.used.contains(&leaf) && r.used.contains(&swing));
    assert_eq!(r.used.len(), d.sources);
}

#[test]
fn a_door_hinged_at_its_far_jamb_opening_out_is_said_so() {
    let mut d = Draw::new();
    room(&mut d, &[(1.0, 1.9)], &[], &[(1.0, 1.9)], &[]);
    // Hinged at x 1.9 on the outer face, swinging outside (−y) by 100°.
    d.arc((1.9, -0.1), 0.85, 180.0, 100.0, Role::Door);
    let r = recognize(&d.lines, &Options::default());
    let o = r.openings[0];
    assert_eq!(
        (o.form, o.hand, o.side, o.swing),
        (Form::Door, End::End, Side::Right, Some(100.0))
    );
}

#[test]
fn two_swings_from_both_jambs_are_a_door_of_two_leaves() {
    let mut d = Draw::new();
    room(&mut d, &[(1.0, 2.4)], &[], &[(1.0, 2.4)], &[]);
    d.arc((1.0, 0.1), 0.7, 0.0, 90.0, Role::Door);
    d.arc((2.4, 0.1), 0.7, 90.0, 90.0, Role::Door);
    let r = recognize(&d.lines, &Options::default());
    assert_eq!(r.openings.len(), 1);
    assert_eq!((r.openings[0].form, r.openings[0].leaves), (Form::Door, 2));
    assert_eq!((r.openings[0].from, r.openings[0].to), (1.0, 2.4));
}

#[test]
fn a_window_s_lines_along_its_gap_make_it_a_window() {
    let mut d = Draw::new();
    room(&mut d, &[], &[(2.0, 3.2)], &[], &[(2.0, 3.2)]);
    d.jamb_x(2.0, 2.9, 3.1);
    d.jamb_x(3.2, 2.9, 3.1);
    let glass: Vec<usize> = [2.9, 2.98, 3.02, 3.1]
        .iter()
        .map(|&y| d.line((2.0, y), (3.2, y), Role::Window))
        .collect();
    // Its sill outside, past its jambs.
    let sill = d.line((1.95, 3.15), (3.25, 3.15), Role::Window);
    let r = recognize(&d.lines, &Options::default());
    assert!(near(&walls(&r), &ROOM), "{:#?}", walls(&r));
    let north = wall_at(&r, 3.0);
    assert_eq!(r.openings.len(), 1);
    let o = r.openings[0];
    assert_eq!(
        (o.wall, o.from, o.to, o.form),
        (north, 2.0, 3.2, Form::Window)
    );
    assert!(glass.iter().all(|g| r.used.contains(g)) && r.used.contains(&sill));
}

#[test]
fn a_gap_with_nothing_in_it_is_a_bare_opening_and_its_jambs_no_wall() {
    let mut d = Draw::new();
    // 0.5 m: its two jambs are 0.5 m apart, a band no wider than a wall.
    room(&mut d, &[(3.0, 3.5)], &[], &[(3.0, 3.5)], &[]);
    d.jamb_x(3.0, -0.1, 0.1);
    d.jamb_x(3.5, -0.1, 0.1);
    let r = recognize(&d.lines, &Options::default());
    assert!(near(&walls(&r), &ROOM), "{:#?}", walls(&r));
    assert_eq!(r.openings.len(), 1);
    let o = r.openings[0];
    assert_eq!((o.from, o.to, o.form), (3.0, 3.5, Form::Opening));
    assert_eq!(r.used.len(), d.sources);
}

#[test]
fn a_slip_of_the_pen_is_no_opening_and_a_wide_gap_ends_the_wall() {
    let mut d = Draw::new();
    // 0.1 m without anything: the wall runs on.
    room(&mut d, &[(3.0, 3.1)], &[], &[(3.0, 3.1)], &[]);
    let r = recognize(&d.lines, &Options::default());
    assert!(near(&walls(&r), &ROOM), "{:#?}", walls(&r));
    assert!(r.openings.is_empty());
    // A wall 3.5 m away along the same line: two walls.
    let mut d = Draw::new();
    d.along_x(0.0, 0.0, 2.0, &[]);
    d.along_x(0.2, 0.0, 2.0, &[]);
    d.along_x(0.0, 5.5, 8.0, &[]);
    d.along_x(0.2, 5.5, 8.0, &[]);
    let r = recognize(&d.lines, &Options::default());
    assert!(
        near(
            &walls(&r),
            &[(0.0, 0.1, 2.0, 0.1, 0.2), (5.5, 0.1, 8.0, 0.1, 0.2)]
        ),
        "{:#?}",
        walls(&r)
    );
}

#[test]
fn a_partition_meeting_two_walls_runs_into_both_and_they_run_through() {
    let mut d = Draw::new();
    // A 0.1 m partition at x 2.5: the walls' inner faces break where it meets them.
    room(&mut d, &[], &[], &[(2.45, 2.55)], &[(2.45, 2.55)]);
    d.along_y(2.45, 0.1, 2.9, &[]);
    d.along_y(2.55, 0.1, 2.9, &[]);
    let r = recognize(&d.lines, &Options::default());
    let mut want = ROOM.to_vec();
    want.push((2.5, 0.0, 2.5, 3.0, 0.1));
    want.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    assert!(near(&walls(&r), &want), "{:#?}", walls(&r));
    assert!(r.openings.is_empty());
    assert_eq!(r.used.len(), d.sources);
}

#[test]
fn walls_stopping_at_columns_run_through_them_and_meet_at_the_corner() {
    let mut d = Draw::new();
    // 0.3 m columns at the south corners and in the middle of the south wall: its faces stop at them.
    d.along_x(-0.1, 0.15, 3.85, &[(1.85, 2.15)]);
    d.along_x(0.1, 0.15, 3.85, &[(1.85, 2.15)]);
    d.along_x(3.1, -0.1, 4.1, &[]);
    d.along_x(2.9, 0.1, 3.9, &[]);
    d.along_y(-0.1, 0.15, 3.1, &[]);
    d.along_y(0.1, 0.15, 2.9, &[]);
    d.along_y(4.1, 0.15, 3.1, &[]);
    d.along_y(3.9, 0.15, 2.9, &[]);
    let cols = [
        d.column((0.0, 0.0), 0.3),
        d.column((2.0, 0.0), 0.3),
        d.column((4.0, 0.0), 0.3),
    ];
    let r = recognize(&d.lines, &Options::default());
    assert!(near(&walls(&r), &ROOM), "{:#?}", walls(&r));
    assert!(r.openings.is_empty());
    // The columns are not made yet: they stay.
    assert!(cols.iter().all(|c| !r.used.contains(c)));
    assert_eq!(r.used.len(), d.sources - 3);
}

#[test]
fn a_door_beside_a_column_opens_from_the_column_s_face() {
    let mut d = Draw::new();
    d.along_x(-0.1, -0.1, 4.1, &[(1.85, 3.05)]);
    d.along_x(0.1, 0.1, 3.9, &[(1.85, 3.05)]);
    d.along_x(3.1, -0.1, 4.1, &[]);
    d.along_x(2.9, 0.1, 3.9, &[]);
    d.along_y(-0.1, -0.1, 3.1, &[]);
    d.along_y(0.1, 0.1, 2.9, &[]);
    d.along_y(4.1, -0.1, 3.1, &[]);
    d.along_y(3.9, 0.1, 2.9, &[]);
    d.column((2.0, 0.0), 0.3);
    d.arc((2.15, 0.1), 0.9, 0.0, 90.0, Role::Door);
    let r = recognize(&d.lines, &Options::default());
    assert!(near(&walls(&r), &ROOM), "{:#?}", walls(&r));
    let o = r.openings[0];
    assert!(
        (o.from - 2.15).abs() < 1e-9 && (o.to - 3.05).abs() < 1e-9,
        "{o:?}"
    );
    assert_eq!((o.form, o.hand), (Form::Door, End::Start));
}

#[test]
fn an_insulation_s_line_is_the_wall_s_and_not_a_wall_of_its_own() {
    let mut d = Draw::new();
    // A 0.25 m wall with 0.05 m of insulation outside it: three lines.
    d.along_x(0.0, 0.0, 5.0, &[]);
    d.along_x(0.05, 0.0, 5.0, &[]);
    d.along_x(0.3, 0.0, 5.0, &[]);
    let r = recognize(&d.lines, &Options::default());
    assert!(
        near(&walls(&r), &[(0.0, 0.15, 5.0, 0.15, 0.3)]),
        "{:#?}",
        walls(&r)
    );
    assert_eq!(r.used.len(), 3);
}

#[test]
fn walls_either_side_of_a_shaft_are_two_walls() {
    let mut d = Draw::new();
    // 0.15 m walls 0.4 m apart.
    for y in [0.0, 0.15, 0.55, 0.7] {
        d.along_x(y, 0.0, 3.0, &[]);
    }
    let r = recognize(&d.lines, &Options::default());
    assert!(
        near(
            &walls(&r),
            &[
                (0.0, 0.075, 3.0, 0.075, 0.15),
                (0.0, 0.625, 3.0, 0.625, 0.15)
            ]
        ),
        "{:#?}",
        walls(&r)
    );
}

#[test]
fn faces_drawn_in_pieces_twice_and_backwards_read_as_drawn_once() {
    let mut d = Draw::new();
    room(&mut d, &[], &[], &[], &[]);
    // The south face again in three overlapping pieces, one backwards, and the inner one 1 mm off.
    d.line((-0.1, -0.1), (1.5, -0.1), Role::Wall);
    d.line((2.5, -0.1), (1.2, -0.1), Role::Wall);
    d.line((2.4, -0.1), (4.1, -0.1), Role::Wall);
    d.line((0.1, 0.101), (3.9, 0.101), Role::Wall);
    let r = recognize(&d.lines, &Options::default());
    assert_eq!(r.walls.len(), 4, "{:#?}", walls(&r));
    // The inner face is the two lines' mean, 0.1005: the wall is 0.2005 wide about 0.00025.
    let south = r.walls[wall_at(&r, 0.00025)];
    assert!((south.width - 0.2005).abs() < 1e-9, "{south:?}");
    assert_eq!(r.used.len(), d.sources);
}

#[test]
fn a_wall_askew_reads_along_its_own_line() {
    let mut d = Draw::new();
    // A 0.2 m wall at 30°.
    let turn = 30.0 * PI / 180.0;
    let (c, s) = (crate::jsmath::cos(turn), crate::jsmath::sin(turn));
    let p = |t: f64, r: f64| (t * c - r * s, t * s + r * c);
    d.line(p(0.0, -0.1), p(4.0, -0.1), Role::Wall);
    d.line(p(0.0, 0.1), p(4.0, 0.1), Role::Wall);
    let r = recognize(&d.lines, &Options::default());
    assert_eq!(r.walls.len(), 1);
    let w = r.walls[0];
    assert!((w.width - 0.2).abs() < 1e-9);
    let b = p(4.0, 0.0);
    assert!(
        (w.b.x - b.0).abs() < 1e-9 && (w.b.y - b.1).abs() < 1e-9,
        "{w:?}"
    );
}

#[test]
fn lines_of_other_things_are_left() {
    let mut d = Draw::new();
    room(&mut d, &[], &[], &[], &[]);
    // A bed in the room, on the walls' layer, and a door's leaf where no door is.
    let bed = d.line((1.0, 1.0), (2.0, 1.0), Role::Wall);
    let leaf = d.line((0.5, 0.2), (0.5, 1.0), Role::Door);
    let r = recognize(&d.lines, &Options::default());
    assert_eq!(r.walls.len(), 4);
    assert!(!r.used.contains(&bed) && !r.used.contains(&leaf));
    assert_eq!(r.used.len(), d.sources - 2);
}
