//! A leader in the core (docs/adr/0146): its length, and its note's turn
//! under the transforms.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::{Shape, entity_area, entity_length};
use kentos_geometry_core::geom::affine::{mirror, rotation, scaling};
use kentos_geometry_core::ops::transform::transform_shape;

fn leader(pts: &[(f64, f64)], rotation: f64) -> Shape {
    Shape::Leader {
        pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
        text: Some("Mevcut bina".into()),
        height: 2.5,
        rotation,
        arrow: Some("open".into()),
        mask: Some(true),
    }
}

#[test]
fn a_leader_s_length_is_its_vertices_and_it_has_no_area() {
    // 5 m, then 6 m; the landing (2 × 2.5 m past the last vertex) does not count (§2).
    let e = leader(&[(0.0, 0.0), (3.0, 4.0), (9.0, 4.0)], 0.0);
    assert_eq!(entity_length(&e), Some(11.0));
    assert_eq!(entity_area(&e), None);
}

#[test]
fn its_note_turns_as_a_text_does() {
    let e = leader(&[(0.0, 0.0), (8.0, 6.0)], 30.0);
    let turn = |m| match transform_shape(&e, &m) {
        Shape::Leader {
            pts,
            height,
            rotation,
            text,
            arrow,
            mask,
        } => {
            assert_eq!(text.as_deref(), Some("Mevcut bina"));
            assert_eq!(arrow.as_deref(), Some("open"));
            assert_eq!(mask, Some(true));
            (pts, height, rotation)
        }
        other => panic!("a leader stays a leader, not {other:?}"),
    };
    let o = Vec2::new(0.0, 0.0);
    // A quarter turn: the vertices turn and the note with them.
    let (pts, height, rot) = turn(rotation(std::f64::consts::FRAC_PI_2, o));
    assert!((pts[1].x + 6.0).abs() < 1e-12 && (pts[1].y - 8.0).abs() < 1e-12);
    assert_eq!(height, 2.5);
    assert!((rot - 120.0).abs() < 1e-9, "{rot}");
    // Twice the size: the note's height too.
    let (_, height, rot) = turn(scaling(2.0, o));
    assert_eq!(height, 5.0);
    assert!((rot - 30.0).abs() < 1e-9, "{rot}");
    // Mirrored across the north axis the vertices mirror, the note stays readable
    // (−30° mirrored is 150°, a half turn more is 330°).
    let (pts, _, rot) = turn(mirror(o, Vec2::new(0.0, 1.0)));
    assert!((pts[1].x + 8.0).abs() < 1e-12 && (pts[1].y - 6.0).abs() < 1e-12);
    assert!((rot - 330.0).abs() < 1e-9, "{rot}");
}
