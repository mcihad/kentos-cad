//! Kolonun geçici ölçüleri (docs/adr/0712): a column's four gaps to the walls' faces and the columns round it, a
//! turned column's to a wall turned with it, a round column's by its bounding square, and the move that makes a gap
//! what was typed. The command's shared cases (fixtures/commands/v1/cad.columns.distances.json and
//! cad.arch.edit.distance.json) hold the rest against an independent reference.

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::geom::arch::ColumnSpec;
use kentos_geometry_core::geom::wall::Spec;
use kentos_geometry_core::geom::wall_join::WallIn;
use kentos_geometry_core::jsmath::{cos, sin};
use kentos_geometry_core::ops::column_shift::{ColumnIn, Target, gaps, moved};
use kentos_geometry_core::vec2::Vec2;

const PLAIN: &str = r#"{"width":0.2,"justify":"center"}"#;

fn spec(json: &str) -> Spec {
    Spec::read_flat(&Json::parse(json).unwrap()).unwrap()
}

fn column(json: &str) -> ColumnSpec {
    ColumnSpec::read_flat(&Json::parse(json).unwrap()).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// A room 0..6 × 0..4 of 20 cm walls on their axes.
fn room() -> Vec<Vec<Vec2>> {
    [
        [(0., 0.), (6., 0.)],
        [(6., 0.), (6., 4.)],
        [(6., 4.), (0., 4.)],
        [(0., 4.), (0., 0.)],
    ]
    .iter()
    .map(|w| w.iter().map(|&(x, y)| Vec2::new(x, y)).collect())
    .collect()
}

#[test]
fn a_columns_four_gaps_go_to_the_walls_and_the_column_beside() {
    let paths = room();
    let wall = spec(PLAIN);
    let walls: Vec<WallIn> = paths
        .iter()
        .map(|p| WallIn {
            pts: p,
            bulges: None,
            spec: &wall,
        })
        .collect();
    let square = column(r#"{"width":0.4}"#);
    let columns = [
        ColumnIn {
            at: Vec2::new(2., 1.5),
            spec: &square,
            level: None,
        },
        ColumnIn {
            at: Vec2::new(4.5, 1.5),
            spec: &square,
            level: None,
        },
    ];
    let g = gaps(&walls, &columns, 0);
    let d: Vec<f64> = g.iter().map(|g| g.expect("all four").distance).collect();
    // Below 1.3 − 0.1; right to the other column's left side 4.3 − 2.2; above 3.9 − 1.7; left 1.8 − 0.1.
    for (got, want) in d.iter().zip([1.2, 2.1, 2.2, 1.7]) {
        assert!(close(*got, want), "{d:?}");
    }
    assert_eq!(
        g[1].unwrap().target,
        Target::Column { column: 1, side: 3 },
        "the other column's left side"
    );
    assert!(matches!(g[0].unwrap().target, Target::Wall { wall: 0, edge: 0, left_face: true }));
    // 1.00 from the west wall: the column 0.7 west.
    let at = moved(&walls, &columns, 0, 3, 1.0).unwrap();
    assert!(close(at.x, 1.3) && close(at.y, 1.5), "{at:?}");
    assert!(moved(&walls, &columns, 0, 4, 1.0).is_err(), "no fifth side");
}

#[test]
fn a_turned_column_measures_to_a_wall_turned_with_it_only() {
    let turn = 30f64.to_radians();
    let (c, s) = (cos(turn), sin(turn));
    let rot = |x: f64, y: f64| Vec2::new(x * c - y * s, x * s + y * c);
    // A wall 3 m north of the column in its own frame, turned 30°; another, square to the world, east.
    let turned = vec![rot(-4., 3.), rot(4., 3.)];
    let square = vec![Vec2::new(5., -5.), Vec2::new(5., 5.)];
    let wall = spec(PLAIN);
    let walls = [
        WallIn {
            pts: &turned,
            bulges: None,
            spec: &wall,
        },
        WallIn {
            pts: &square,
            bulges: None,
            spec: &wall,
        },
    ];
    // The core takes the turn in radians.
    let spec = column(&format!(r#"{{"width":0.4,"rotation":{turn}}}"#));
    let columns = [ColumnIn {
        at: Vec2::new(0., 0.),
        spec: &spec,
        level: None,
    }];
    let g = gaps(&walls, &columns, 0);
    let above = g[2].expect("the turned wall above");
    assert!(close(above.distance, 3.0 - 0.1 - 0.2), "{above:?}");
    assert!(g[1].is_none(), "the east wall is not parallel to the turned side: {:?}", g[1]);
}

#[test]
fn a_round_column_is_measured_by_its_bounding_square_and_levels_part() {
    let paths = room();
    let wall = spec(r#"{"width":0.2,"justify":"center","arch":{"level":"zemin"}}"#);
    let walls: Vec<WallIn> = paths
        .iter()
        .map(|p| WallIn {
            pts: p,
            bulges: None,
            spec: &wall,
        })
        .collect();
    let round = column(r#"{"width":0.5,"shape":"round"}"#);
    let on = [ColumnIn {
        at: Vec2::new(3., 2.),
        spec: &round,
        level: Some("zemin"),
    }];
    let d: Vec<f64> = gaps(&walls, &on, 0).iter().map(|g| g.unwrap().distance).collect();
    for (got, want) in d.iter().zip([1.65, 2.65, 1.65, 2.65]) {
        assert!(close(*got, want), "{d:?}");
    }
    let off = [ColumnIn {
        at: Vec2::new(3., 2.),
        spec: &round,
        level: Some("bodrum"),
    }];
    assert!(gaps(&walls, &off, 0).iter().all(Option::is_none), "another level's walls are not its");
}
