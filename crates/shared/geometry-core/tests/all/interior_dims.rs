//! Kesen çizgiyle iç ölçü zinciri (docs/adr/0711 §3): the faces a cut crosses, in order; a corner's hidden faces, a
//! column's sides or centre, a curved face counted, a short cut refused. The command's shared cases
//! (fixtures/commands/v1/cad.dimensions.interior.json) hold the rest against an independent reference.

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch::ColumnSpec;
use kentos_geometry_core::geom::wall::Spec;
use kentos_geometry_core::ops::interior_dims::{Columns, Faces, references};
use kentos_geometry_core::ops::subref::Place;
use kentos_geometry_core::vec2::Vec2;

const PLAIN: &str = r#"{"width":0.2,"justify":"center"}"#;

fn wall(pts: &[(f64, f64)], spec: &str) -> Shape {
    let spec = Spec::read_flat(&Json::parse(spec).unwrap()).unwrap();
    Shape::Wall {
        pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
        bulges: None,
        wall: spec,
    }
}

fn column(at: (f64, f64), json: &str) -> Shape {
    Shape::Column {
        at: Vec2::new(at.0, at.1),
        column: ColumnSpec::read_flat(&Json::parse(json).unwrap()).unwrap(),
    }
}

fn xs(refs: &[kentos_geometry_core::ops::interior_dims::Reference]) -> Vec<f64> {
    refs.iter().map(|r| (r.at.x * 1e9).round() / 1e9).collect()
}

#[test]
fn two_walls_give_their_four_faces_in_order() {
    let walls = [
        wall(&[(0., 0.), (0., 4.)], PLAIN),
        wall(&[(4., 0.), (4., 4.)], PLAIN),
    ];
    let sources: Vec<&Shape> = walls.iter().collect();
    let got = references(
        &sources,
        Vec2::new(-1., 2.),
        Vec2::new(5., 2.),
        Faces::Outer,
        Columns::Ends,
    )
    .unwrap();
    assert_eq!(xs(&got.refs), [-0.1, 0.1, 3.9, 4.1]);
    assert_eq!(
        got.refs[0].place,
        Place::Along {
            part: 0,
            ring: 0,
            index: 0,
            t: 0.5
        },
        "a wall running north: its left face west"
    );
    assert_eq!(
        got.refs[1].place,
        Place::Along {
            part: 1,
            ring: 0,
            index: 0,
            t: 0.5
        }
    );
    // Backwards, the same faces from the east.
    let back = references(
        &sources,
        Vec2::new(5., 2.),
        Vec2::new(-1., 2.),
        Faces::Outer,
        Columns::Ends,
    )
    .unwrap();
    assert_eq!(xs(&back.refs), [4.1, 3.9, 0.1, -0.1]);
}

#[test]
fn a_corners_hidden_faces_are_left_out() {
    // A room's south-east corner: a cut down through the east wall's body crosses the south wall's inner face there.
    let walls = [
        wall(&[(0., 0.), (5., 0.)], PLAIN),
        wall(&[(5., 0.), (5., 4.)], PLAIN),
    ];
    let sources: Vec<&Shape> = walls.iter().collect();
    let got = references(
        &sources,
        Vec2::new(4.95, -1.),
        Vec2::new(4.95, 1.),
        Faces::Outer,
        Columns::Ends,
    )
    .unwrap();
    assert_eq!(
        got.refs.len(),
        1,
        "only the south wall's outer face shows: {:?}",
        got.refs
    );
    assert!((got.refs[0].at.y + 0.1).abs() < 1e-12);
}

#[test]
fn a_columns_sides_its_centre_or_nothing() {
    let shapes = [
        wall(&[(0., 0.), (0., 4.)], PLAIN),
        column((2., 2.), r#"{"width":0.4}"#),
        column((3., 2.), r#"{"width":0.4,"shape":"round"}"#),
        wall(&[(5., 0.), (5., 4.)], PLAIN),
    ];
    let sources: Vec<&Shape> = shapes.iter().collect();
    let cut = |c| {
        references(
            &sources,
            Vec2::new(-1., 2.),
            Vec2::new(6., 2.),
            Faces::Outer,
            c,
        )
        .unwrap()
    };
    assert_eq!(
        xs(&cut(Columns::Ends).refs),
        [-0.1, 0.1, 1.8, 2.2, 3.0, 4.9, 5.1]
    );
    assert_eq!(
        xs(&cut(Columns::Centers).refs),
        [-0.1, 0.1, 2.0, 3.0, 4.9, 5.1]
    );
    assert_eq!(xs(&cut(Columns::None).refs), [-0.1, 0.1, 4.9, 5.1]);
}

#[test]
fn a_curved_face_is_counted_and_a_short_cut_refused() {
    let mut arc = wall(&[(0., 0.), (6., 0.)], PLAIN);
    if let Shape::Wall { bulges, .. } = &mut arc {
        *bulges = Some(vec![0.3, 0.0]);
    }
    let shapes = [
        arc,
        wall(&[(-2., -2.), (-2., 4.)], PLAIN),
        wall(&[(8., -2.), (8., 4.)], PLAIN),
    ];
    let sources: Vec<&Shape> = shapes.iter().collect();
    let got = references(
        &sources,
        Vec2::new(-3., -0.5),
        Vec2::new(9., -0.5),
        Faces::Outer,
        Columns::Ends,
    )
    .unwrap();
    assert_eq!(got.curved, 2, "both faces of the arc");
    assert_eq!(xs(&got.refs), [-2.1, -1.9, 7.9, 8.1]);
    assert!(
        references(
            &sources,
            Vec2::new(0., 0.),
            Vec2::new(0.0005, 0.),
            Faces::Outer,
            Columns::Ends
        )
        .is_err()
    );
}

#[test]
fn every_line_of_a_wall_of_components() {
    let walls = [wall(
        &[(0., 0.), (0., 4.)],
        r#"{"width":0.3,"justify":"center","components":[{"name":"Sıva","thickness":0.02},{"name":"Tuğla"},{"name":"Sıva","thickness":0.02}]}"#,
    )];
    let sources: Vec<&Shape> = walls.iter().collect();
    let outer = references(
        &sources,
        Vec2::new(-1., 2.),
        Vec2::new(1., 2.),
        Faces::Outer,
        Columns::Ends,
    )
    .unwrap();
    assert_eq!(xs(&outer.refs), [-0.15, 0.15]);
    let skins = references(
        &sources,
        Vec2::new(-1., 2.),
        Vec2::new(1., 2.),
        Faces::Skins,
        Columns::Ends,
    )
    .unwrap();
    assert_eq!(xs(&skins.refs), [-0.15, -0.13, 0.13, 0.15]);
}
