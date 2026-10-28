//! Duvar boyu (docs/adr/0728 §3): a wall's edge made a length, one end staying; a free end alone, a corner's wall
//! shifting with it (a room stretching), a wall end to end in one line shortening, a tee, a crowded end and a corner of
//! the path refused, both ends from the middle, and an edge that would vanish. The command's shared cases
//! (fixtures/commands/v1/cad.walls.edit.length.json) hold the rest against an independent reference.

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::geom::wall::Spec;
use kentos_geometry_core::geom::wall_join::WallIn;
use kentos_geometry_core::ops::wall_shift::{Anchor, edge_length, free_ends, lengthen};
use kentos_geometry_core::vec2::Vec2;

fn spec(json: &str) -> Spec {
    Spec::read_flat(&Json::parse(json).unwrap()).unwrap()
}

fn paths(walls: &[&[(f64, f64)]]) -> Vec<Vec<Vec2>> {
    walls
        .iter()
        .map(|w| w.iter().map(|&(x, y)| Vec2::new(x, y)).collect())
        .collect()
}

fn ins<'a>(paths: &'a [Vec<Vec2>], spec: &'a Spec) -> Vec<WallIn<'a>> {
    paths
        .iter()
        .map(|p| WallIn {
            pts: p,
            bulges: None,
            spec,
        })
        .collect()
}

fn near(a: Vec2, b: (f64, f64)) -> bool {
    (a.x - b.0).abs() < 1e-9 && (a.y - b.1).abs() < 1e-9
}

#[test]
fn a_free_end_moves_alone_and_the_start_stays() {
    let p = paths(&[&[(0., 0.), (4., 0.)]]);
    let s = spec(r#"{"width":0.2,"justify":"center"}"#);
    let w = ins(&p, &s);
    assert_eq!(free_ends(&w, 0, 0), [true, true]);
    assert!((edge_length(&w, 0, 0).unwrap() - 4.0).abs() < 1e-12);
    let m = lengthen(&w, 0, 0, 5.5, Anchor::Start).unwrap();
    assert_eq!(m.len(), 1);
    assert!(near(m[0].pts[0], (0., 0.)) && near(m[0].pts[1], (5.5, 0.)));
    let m = lengthen(&w, 0, 0, 3.0, Anchor::End).unwrap();
    assert!(near(m[0].pts[0], (1., 0.)) && near(m[0].pts[1], (4., 0.)), "the start moved");
    let m = lengthen(&w, 0, 0, 5.0, Anchor::Middle).unwrap();
    assert!(near(m[0].pts[0], (-0.5, 0.)) && near(m[0].pts[1], (4.5, 0.)), "half each way");
}

#[test]
fn a_corners_wall_shifts_and_the_room_stretches() {
    // A room of four walls, 6 × 4 on their axes: the south wall made 7 m long from its west end.
    let p = paths(&[
        &[(0., 0.), (6., 0.)],
        &[(6., 0.), (6., 4.)],
        &[(6., 4.), (0., 4.)],
        &[(0., 4.), (0., 0.)],
    ]);
    let s = spec(r#"{"width":0.2,"justify":"center"}"#);
    let w = ins(&p, &s);
    let m = lengthen(&w, 0, 0, 7.0, Anchor::Start).unwrap();
    assert_eq!(m[0].wall, 0, "the wall first");
    let at = |wall: usize| m.iter().find(|x| x.wall == wall).map(|x| x.pts.clone());
    assert!(near(at(0).unwrap()[1], (7., 0.)));
    let east = at(1).expect("the east wall shifted");
    assert!(near(east[0], (7., 0.)) && near(east[1], (7., 4.)));
    let north = at(2).expect("the north wall's end followed");
    assert!(near(north[0], (7., 4.)) && near(north[1], (0., 4.)));
    assert!(at(3).is_none(), "the west wall stays");
}

#[test]
fn a_wall_end_to_end_shortens_and_a_tee_or_a_path_corner_is_refused() {
    let s = spec(r#"{"width":0.2,"justify":"center"}"#);
    let p = paths(&[&[(0., 0.), (3., 0.)], &[(3., 0.), (6., 0.)]]);
    let w = ins(&p, &s);
    let m = lengthen(&w, 0, 0, 4.0, Anchor::Start).unwrap();
    assert!(near(m[0].pts[1], (4., 0.)));
    let next = m.iter().find(|x| x.wall == 1).expect("the next wall");
    assert!(near(next.pts[0], (4., 0.)) && near(next.pts[1], (6., 0.)), "it shortened");
    assert!(lengthen(&w, 0, 0, 6.0, Anchor::Start).unwrap_err().contains("yok ediyor"));

    // A wall standing in another one's body (a tee).
    let p = paths(&[&[(2., 3.), (2., 0.)], &[(0., 0.), (5., 0.)]]);
    let w = ins(&p, &s);
    assert_eq!(free_ends(&w, 0, 0), [true, false], "the end in the other wall's body cannot move");
    let why = lengthen(&w, 0, 0, 2.0, Anchor::Start).unwrap_err();
    assert!(why.contains("saplı"), "{why}");
    assert!(lengthen(&w, 0, 0, 2.0, Anchor::End).is_ok(), "its free end moves");

    // A wall of two edges: its corner does not move.
    let p = paths(&[&[(0., 0.), (4., 0.), (4., 3.)]]);
    let w = ins(&p, &s);
    assert_eq!(free_ends(&w, 0, 0), [true, false]);
    let why = lengthen(&w, 0, 0, 5.0, Anchor::Start).unwrap_err();
    assert!(why.contains("köşesi"), "{why}");
    let m = lengthen(&w, 0, 0, 5.0, Anchor::End).unwrap();
    assert!(near(m[0].pts[0], (-1., 0.)) && near(m[0].pts[1], (4., 0.)));
}

#[test]
fn a_crowded_end_and_a_bad_length_are_refused() {
    let s = spec(r#"{"width":0.2,"justify":"center"}"#);
    let p = paths(&[&[(0., 0.), (3., 0.)], &[(3., 0.), (3., 3.)], &[(3., 0.), (5., -2.)]]);
    let w = ins(&p, &s);
    assert_eq!(free_ends(&w, 0, 0), [true, false], "where three walls meet the end cannot move");
    let why = lengthen(&w, 0, 0, 4.0, Anchor::Start).unwrap_err();
    assert!(why.contains("birden çok"), "{why}");
    assert!(lengthen(&w, 0, 0, 0.0, Anchor::Start).is_err());
    assert!(lengthen(&w, 0, 0, f64::NAN, Anchor::Start).is_err());
    let p = paths(&[&[(0., 0.), (4., 0.), (4., 3.), (0., 3.)]]);
    let closed = spec(r#"{"width":0.2,"justify":"center","closed":true}"#);
    let w = ins(&p, &closed);
    assert_eq!(free_ends(&w, 0, 1), [false, false], "a closed wall has no ends");
}
