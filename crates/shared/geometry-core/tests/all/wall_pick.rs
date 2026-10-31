//! A wall is picked inside its body (docs/adr/0776 §3): the 2+1 acceptance trace clicked the middle of a 25 cm
//! shell wall, 12.5 cm from both faces, and nothing was found at an aperture of 4 px at 2 cm a pixel.

use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

const E: f64 = 487000.0;
const N: f64 = 4420000.0;

fn store() -> Store {
    let mut s = Store::new();
    let walls = format!(
        r#"[
        {{ "kind": "wall", "id": 1, "layerId": "0", "attrs": {{}}, "closed": true, "justify": "left", "width": 0.25,
          "pts": [{{ "x": {e0}, "y": {n0} }}, {{ "x": {e1}, "y": {n0} }}, {{ "x": {e1}, "y": {n1} }}, {{ "x": {e0}, "y": {n1} }}] }},
        {{ "kind": "wall", "id": 2, "layerId": "0", "attrs": {{}}, "justify": "center", "width": 0.15,
          "pts": [{{ "x": {e2}, "y": {n1} }}, {{ "x": {e2}, "y": {n0} }}] }}
    ]"#,
        e0 = E,
        e1 = E + 11.0,
        e2 = E + 4.575,
        n0 = N,
        n1 = N + 8.0
    );
    s.put_json(&walls).expect("the walls");
    s
}

#[test]
fn a_click_between_a_walls_faces_finds_it_and_nothing_else() {
    let s = store();
    let tol = 0.08;
    // The shell's middle, 12.5 cm from each face: a hit inside its body.
    for p in [
        Vec2::new(E + 11.125, N + 3.0),
        Vec2::new(E + 5.0, N - 0.125),
        Vec2::new(E - 0.125, N + 4.0),
    ] {
        assert_eq!(s.hits(p, tol), vec![1.0], "{p:?}");
    }
    // The partition's faces within reach: picked by its edge.
    assert_eq!(s.hits(Vec2::new(E + 4.575, N + 3.0), tol), vec![2.0]);
    // Inside the room, away from every face: nothing (the shell's hole is not its body).
    assert!(s.hits(Vec2::new(E + 2.0, N + 3.0), tol).is_empty());
    assert!(s.hits(Vec2::new(E - 1.0, N + 3.0), tol).is_empty());
}

/// docs/adr/0776 §3 (B4): a room's outline lies on the partition's face; a click in the partition's body with both
/// faces in reach picks the wall first and the room after it, where Sıradakini seç finds it. Outside the body the
/// room's edge keeps its place among the edges.
#[test]
fn a_rooms_outline_on_a_walls_face_does_not_take_a_click_in_the_wall() {
    let mut s = store();
    let room = format!(
        r#"[{{ "kind": "space", "id": 3, "layerId": "0", "attrs": {{}},
            "parts": [{{ "outer": {{ "pts": [{{ "x": {a}, "y": {n0} }}, {{ "x": {b}, "y": {n0} }}, {{ "x": {b}, "y": {n1} }}, {{ "x": {a}, "y": {n1} }}] }}, "holes": [] }}] }}]"#,
        a = E + 4.65,
        b = E + 11.0,
        n0 = N,
        n1 = N + 8.0
    );
    s.put_json(&room).expect("the room");
    let tol = 0.08;
    assert_eq!(s.hits(Vec2::new(E + 4.575, N + 3.0), tol), vec![2.0, 3.0]);
    // In the room by its face, outside the wall: both by their edges, as before (equals in the drawing's order).
    assert_eq!(s.hits(Vec2::new(E + 4.70, N + 3.0), tol), vec![2.0, 3.0]);
}

/// docs/adr/0798 §3: the wall's body runs on through its openings as an area; a click in a door's gap, between its
/// jambs, picks the door (the smaller area) and the wall after it, where Sıradakini seç finds it.
#[test]
fn a_click_in_an_openings_gap_picks_the_opening_before_its_wall() {
    let mut s = store();
    let door = format!(
        r#"[{{ "kind": "opening", "id": 4, "layerId": "0", "attrs": {{}}, "at": 2.25, "width": 1.2, "form": "window",
            "place": [{{ "x": {a}, "y": {n0} }}, {{ "x": {a}, "y": {n1} }}, {{ "x": {b}, "y": {n0} }}, {{ "x": {b}, "y": {n1} }}] }}]"#,
        a = E + 1.65,
        b = E + 2.85,
        n0 = N,
        n1 = N - 0.25
    );
    s.put_json(&door).expect("the window");
    let tol = 0.08;
    let hits = s.hits(Vec2::new(E + 2.25, N - 0.125), tol);
    assert_eq!(hits.first(), Some(&4.0), "{hits:?}");
    assert!(hits.contains(&1.0), "the wall after it: {hits:?}");
    // Beside the gap the wall's body is the wall's.
    assert_eq!(s.hits(Vec2::new(E + 5.0, N - 0.125), tol), vec![1.0]);
}
