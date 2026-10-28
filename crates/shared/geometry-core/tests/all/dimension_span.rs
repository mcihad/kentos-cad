//! Façade and space dimension rows (docs/adr/0729 §2): a house of four walls measured from its south wall alone (the
//! corner walls' outer faces end it, the partition meets it, the north wall's window is hidden), its core, its grids,
//! a room's outline and door, a removed span, an oblique or curved wall refused, an oblique partition skipped, and
//! the same lengths far from the origin, turned and run the other way. The shared command cases
//! (fixtures/commands/v1/cad.dimensionsets.create.span.json) hold the rest against an independent reference.

use kentos_geometry_core::geom::wall::{Component, Justify, Spec};
use kentos_geometry_core::jsmath::{cos, js_round, sin};
use kentos_geometry_core::ops::dimension_set::{Openings, Point, Row};
use kentos_geometry_core::ops::dimension_span::{
    Fault, GridIn, OpeningIn, Recipe, Reference, SpaceIn, WallIn, Why, plan,
};
use kentos_geometry_core::vec2::Vec2;

fn spec(width: f64) -> Spec {
    Spec {
        width,
        justify: Justify::Center,
        ..Spec::default()
    }
}

/// A plaster, brick (the core), plaster wall 25 cm wide.
fn layered() -> Spec {
    let c = |t: f64, priority: u8| Component {
        thickness: Some(t),
        priority: Some(priority),
        ..Component::default()
    };
    Spec {
        width: 0.25,
        components: vec![c(0.02, 5), c(0.21, 1), c(0.02, 5)],
        ..Spec::default()
    }
}

/// A 10 × 8 house on its walls' axes, the origin moved to `o` and turned by `turn` (radians).
struct House {
    walls: Vec<Vec<Vec2>>,
    o: Vec2,
    c: f64,
    s: f64,
}

impl House {
    fn new(o: Vec2, turn: f64) -> Self {
        let mut h = House {
            walls: Vec::new(),
            o,
            c: cos(turn),
            s: sin(turn),
        };
        for path in [
            [(0., 0.), (10., 0.)],
            [(10., 0.), (10., 8.)],
            [(10., 8.), (0., 8.)],
            [(0., 8.), (0., 0.)],
            [(4., 0.), (4., 8.)],
        ] {
            let w = path.iter().map(|&(x, y)| h.p(x, y)).collect();
            h.walls.push(w);
        }
        h
    }
    fn p(&self, x: f64, y: f64) -> Vec2 {
        Vec2::new(self.o.x + self.c * x - self.s * y, self.o.y + self.s * x + self.c * y)
    }
    fn d(&self) -> Vec2 {
        Vec2::new(self.c, self.s)
    }
    /// A point's place along the direction from the house's origin.
    fn along(&self, p: Vec2) -> f64 {
        (p.x - self.o.x) * self.c + (p.y - self.o.y) * self.s
    }
}

fn lengths(spans: &[kentos_geometry_core::ops::dimension_set::Span], row: Row, h: &House) -> Vec<f64> {
    spans
        .iter()
        .filter(|s| s.row == row)
        .map(|s| js_round((h.along(s.b.at) - h.along(s.a.at)) * 1e6) / 1e6)
        .collect()
}

fn keep(_: Row, _: &Point, _: &Point) -> bool {
    false
}

/// The south wall's window (6 to 7.2) and the north wall's (3 to 4.2, along its path running west).
fn openings(h: &House) -> Vec<OpeningIn> {
    vec![
        OpeningIn {
            source: 10,
            host: Some(0),
            jambs: [h.p(6., 0.125), h.p(6., -0.125), h.p(7.2, 0.125), h.p(7.2, -0.125)],
            flue: false,
            niche_right: None,
        },
        OpeningIn {
            source: 11,
            host: Some(2),
            jambs: [h.p(4.2, 7.875), h.p(4.2, 8.125), h.p(3., 7.875), h.p(3., 8.125)],
            flue: false,
            niche_right: None,
        },
    ]
}

fn grids(h: &House) -> Vec<Vec<Vec2>> {
    [0., 4., 10.]
        .iter()
        .map(|&x| vec![h.p(x, -2.), h.p(x, 10.)])
        .collect()
}

fn run(
    h: &House,
    chosen: &[usize],
    s: &Spec,
    partition: &Spec,
    reference: Reference,
    rows: &[Row],
) -> Result<kentos_geometry_core::ops::dimension_span::Planned, Fault> {
    run_along(h, h.d(), chosen, s, partition, reference, rows)
}

fn run_along(
    h: &House,
    direction: Vec2,
    chosen: &[usize],
    s: &Spec,
    partition: &Spec,
    reference: Reference,
    rows: &[Row],
) -> Result<kentos_geometry_core::ops::dimension_span::Planned, Fault> {
    let walls: Vec<WallIn<'_>> = h
        .walls
        .iter()
        .enumerate()
        .map(|(i, p)| WallIn {
            source: i,
            pts: p,
            bulges: None,
            spec: if i == 4 { partition } else { s },
            chosen: chosen.contains(&i),
        })
        .collect();
    let g = grids(h);
    let gs: Vec<GridIn<'_>> = g
        .iter()
        .enumerate()
        .map(|(i, p)| GridIn {
            source: 20 + i,
            pts: p,
            bulges: None,
        })
        .collect();
    plan(
        &Recipe {
            direction,
            reference,
            openings: Openings::Ends,
            rows,
            at: h.p(5., -1.5),
            gap: 0.5,
            row_places: &[],
        },
        &walls,
        &openings(h),
        &gs,
        None,
        &keep,
    )
}

const ALL: [Row; 4] = [Row::Openings, Row::Walls, Row::Grids, Row::Overall];

#[test]
fn the_south_wall_alone_reaches_the_corners_and_meets_the_partition() {
    for (o, turn) in [
        (Vec2::new(0., 0.), 0.0),
        (Vec2::new(500_000., 4_400_000.), 0.0),
        (Vec2::new(500_000., 4_400_000.), std::f64::consts::PI / 6.0),
    ] {
        let h = House::new(o, turn);
        let got = run(&h, &[0], &spec(0.25), &spec(0.1), Reference::Faces, &ALL).unwrap();
        assert_eq!(lengths(&got.spans, Row::Openings, &h), [6.125, 1.2, 2.925], "{o:?} {turn}");
        assert_eq!(lengths(&got.spans, Row::Walls, &h), [0.25, 3.825, 0.1, 5.825, 0.25]);
        assert_eq!(lengths(&got.spans, Row::Grids, &h), [0.125, 4.0, 6.0, 0.125]);
        assert_eq!(lengths(&got.spans, Row::Overall, &h), [10.25]);
        assert_eq!(got.side, -1.0);
        assert!(got.skipped.is_empty());
        // Each row a gap farther out: the first through `at`, 1.5 m south of the axis.
        for (row, k) in ALL.iter().zip(0..) {
            let s = got.spans.iter().find(|s| s.row == *row).unwrap();
            let across = |p: Vec2| -(p.x - h.o.x) * h.s + (p.y - h.o.y) * h.c;
            assert!((across(s.at) - (-1.5 - 0.5 * f64::from(k))).abs() < 1e-6, "{row:?}");
        }
    }
}

#[test]
fn the_core_is_the_strongest_band_and_hidden_windows_stay_out() {
    let h = House::new(Vec2::new(0., 0.), 0.0);
    let got = run(&h, &[0], &layered(), &spec(0.1), Reference::Core, &[Row::Overall]).unwrap();
    assert_eq!(lengths(&got.spans, Row::Overall, &h), [10.21]);
    // All four walls chosen: from the south the north wall's window is behind the south wall.
    let got = run(&h, &[0, 1, 2, 3], &spec(0.25), &spec(0.1), Reference::Faces, &[Row::Openings]).unwrap();
    assert_eq!(lengths(&got.spans, Row::Openings, &h), [6.125, 1.2, 2.925]);
    // Run west, the same lengths in the other order, the rows still south.
    let back = run_along(&h, Vec2::new(-1., 0.), &[0], &spec(0.25), &spec(0.1), Reference::Faces, &[Row::Openings])
        .unwrap();
    let l: Vec<f64> = back
        .spans
        .iter()
        .map(|s| js_round((s.a.at.x - s.b.at.x) * 1e6) / 1e6)
        .collect();
    assert_eq!(l, [2.925, 1.2, 6.125]);
    assert_eq!(back.side, 1.0);
    assert!(back.spans.iter().all(|s| (s.at.y - -1.5).abs() < 1e-9));
}

#[test]
fn a_removed_span_keeps_its_row_and_an_empty_row_takes_no_place() {
    let h = House::new(Vec2::new(0., 0.), 0.0);
    let s = spec(0.25);
    let walls: Vec<WallIn<'_>> = h
        .walls
        .iter()
        .enumerate()
        .map(|(i, p)| WallIn {
            source: i,
            pts: p,
            bulges: None,
            spec: &s,
            chosen: i == 0,
        })
        .collect();
    let drop = |row: Row, a: &Point, b: &Point| {
        row == Row::Openings && a.source == 3 && b.source == 10
    };
    let got = plan(
        &Recipe {
            direction: h.d(),
            reference: Reference::Faces,
            openings: Openings::Ends,
            rows: &[Row::Openings, Row::Grids, Row::Overall],
            at: h.p(5., -1.5),
            gap: 0.5,
            row_places: &[],
        },
        &walls,
        &openings(&h),
        &[],
        None,
        &drop,
    )
    .unwrap();
    assert_eq!(lengths(&got.spans, Row::Openings, &h), [1.2, 2.925]);
    // No grid: its row measures what the overall does, first, in the second place; the overall is not drawn again.
    assert_eq!(lengths(&got.spans, Row::Grids, &h), [10.25]);
    assert!(lengths(&got.spans, Row::Overall, &h).is_empty());
    let grid = got.spans.iter().find(|s| s.row == Row::Grids).unwrap();
    assert!((grid.at.y - -2.0).abs() < 1e-9);
}

#[test]
fn a_room_measures_its_outline_and_its_door() {
    let h = House::new(Vec2::new(0., 0.), 0.0);
    let ring: Vec<Vec2> = [(0.125, 0.125), (3.95, 0.125), (3.95, 7.875), (0.125, 7.875)]
        .iter()
        .map(|&(x, y)| h.p(x, y))
        .collect();
    let space = SpaceIn {
        source: 30,
        parts: vec![vec![(ring, vec![0.0; 4])]],
    };
    let door = OpeningIn {
        source: 12,
        host: Some(0),
        jambs: [h.p(1., 0.125), h.p(1., -0.125), h.p(2., 0.125), h.p(2., -0.125)],
        flue: false,
        niche_right: None,
    };
    let got = plan(
        &Recipe {
            direction: h.d(),
            reference: Reference::Faces,
            openings: Openings::Ends,
            rows: &[Row::Openings, Row::Overall],
            at: h.p(2., 0.6),
            gap: 0.5,
            row_places: &[],
        },
        &[],
        &[door],
        &[],
        Some(&space),
        &keep,
    )
    .unwrap();
    assert_eq!(lengths(&got.spans, Row::Openings, &h), [0.875, 1.0, 1.95]);
    assert_eq!(lengths(&got.spans, Row::Overall, &h), [3.825]);
    assert_eq!(got.side, -1.0);
}

#[test]
fn an_oblique_or_curved_chosen_wall_is_refused_an_oblique_partition_skipped() {
    let mut h = House::new(Vec2::new(0., 0.), 0.0);
    h.walls[1] = vec![Vec2::new(10., 0.), Vec2::new(11., 8.)];
    let s = spec(0.25);
    assert_eq!(
        run(&h, &[0, 1], &s, &spec(0.1), Reference::Faces, &ALL).unwrap_err(),
        Fault::Unsupported(1, Why::Oblique)
    );
    // Not chosen, the oblique east wall at the south wall's end is skipped; the end is free.
    let got = run(&h, &[0], &s, &spec(0.1), Reference::Faces, &[Row::Overall]).unwrap();
    assert_eq!(got.skipped, [1]);
    assert_eq!(lengths(&got.spans, Row::Overall, &h), [10.125]);
    let curved = [Vec2::new(0., 0.), Vec2::new(10., 0.)];
    let bulges = [0.3, 0.0];
    let walls = [WallIn {
        source: 0,
        pts: &curved,
        bulges: Some(&bulges),
        spec: &s,
        chosen: true,
    }];
    let refused = plan(
        &Recipe {
            direction: Vec2::new(1., 0.),
            reference: Reference::Faces,
            openings: Openings::Ends,
            rows: &[Row::Overall],
            at: Vec2::new(5., -2.),
            gap: 0.5,
            row_places: &[],
        },
        &walls,
        &[],
        &[],
        None,
        &keep,
    );
    assert_eq!(refused.unwrap_err(), Fault::Unsupported(0, Why::Arc));
}
