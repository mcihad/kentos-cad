use super::*;
use crate::building::{Element, Space, Storey};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn element(class: Class, shapes: Vec<Shape>, storey: usize) -> Element {
    Element {
        class,
        id: Some(7),
        name: String::new(),
        description: None,
        tag: None,
        predefined: "STANDARD",
        storey,
        shapes,
        overall: None,
        material: None,
        psets: Vec::new(),
        qtos: Vec::new(),
    }
}

/// The signed volume the triangles close (counterclockwise from outside: positive); faces met from both sides cancel.
fn volume(m: &Mesh) -> f64 {
    m.triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|k| m.vertices[k as usize]);
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.0
        })
        .sum()
}

fn building() -> Building {
    let storeys = vec![
        Storey { id: None, name: "Zemin".into(), long_name: None, elevation: 0.0, psets: vec![], qtos: vec![] },
        Storey { id: None, name: "1. kat".into(), long_name: None, elevation: 3.0, psets: vec![], qtos: vec![] },
    ];
    // A 5 × 0.25 m wall 3 m high on the first floor, a 1 m window from 0.90 to 2.10 through it.
    let wall = element(Class::Wall, vec![Shape::Prism { rings: vec![rect(0.0, 0.0, 5.0, 0.25)], bottom: 0.0, top: 3.0 }], 1);
    // A slab with a 1 × 1 m hole (even-odd rings), 0.2 m thick.
    let slab = element(
        Class::Slab,
        vec![Shape::Prism { rings: vec![rect(0.0, 0.0, 4.0, 4.0), rect(1.0, 1.0, 2.0, 2.0)], bottom: -0.2, top: 0.0 }],
        0,
    );
    // A roof face over 2 × 2 m rising 1 m to the north, 0.2 m thick, its faces as the writer gives them.
    let r = rect(0.0, 0.0, 2.0, 2.0);
    let z = |p: [f64; 2]| 3.0 + p[1] * 0.5;
    let mut vertices: Vec<[f64; 3]> = r.iter().map(|p| [p[0], p[1], z(*p)]).collect();
    vertices.extend(r.iter().map(|p| [p[0], p[1], z(*p) - 0.2]));
    let mut faces = vec![vec![0, 1, 2, 3], vec![7, 6, 5, 4]];
    for i in 0..4 {
        let j = (i + 1) % 4;
        faces.push(vec![i, i + 4, j + 4, j]);
    }
    let roof = element(Class::Roof, vec![Shape::Faces { vertices, faces }], 0);
    let empty = element(Class::Railing, vec![], 0);
    Building {
        storeys,
        elements: vec![wall, slab, roof, empty],
        openings: vec![Opening {
            id: None,
            name: String::new(),
            host: 0,
            ring: rect(2.0, -0.1, 3.0, 0.35),
            bottom: 0.9,
            top: 2.1,
            filler: None,
            predefined: "OPENING",
        }],
        spaces: vec![Space {
            id: Some(9),
            name: String::new(),
            long_name: None,
            storey: 0,
            rings: vec![rect(0.0, 0.0, 4.0, 3.0)],
            bottom: 0.0,
            top: 2.8,
            predefined: "INTERNAL",
            psets: vec![],
            qtos: vec![],
        }],
        ..Default::default()
    }
}

#[test]
fn every_element_with_a_body_and_every_space_is_a_closed_mesh_of_its_volume() {
    let b = building();
    let all = meshes(&b);
    let kinds: Vec<Kind> = all.iter().map(|m| m.kind).collect();
    assert_eq!(kinds, [Kind::Element(Class::Wall), Kind::Element(Class::Slab), Kind::Element(Class::Roof), Kind::Space]);
    // The wall less its window: 5 · 0.25 · 3 − 1 · 0.25 · 1.2.
    assert!((volume(&all[0]) - (3.75 - 0.3)).abs() < 1e-9, "{}", volume(&all[0]));
    assert!(all[0].vertices.iter().all(|v| v[2] >= 3.0 - 1e-9 && v[2] <= 6.0 + 1e-9), "its storey's floor added");
    // The slab less its hole: (16 − 1) · 0.2.
    assert!((volume(&all[1]) - 3.0).abs() < 1e-9, "{}", volume(&all[1]));
    // The roof face: a sheared box, 2 · 2 · 0.2.
    assert!((volume(&all[2]) - 0.8).abs() < 1e-9, "{}", volume(&all[2]));
    assert!((volume(&all[3]) - 4.0 * 3.0 * 2.8).abs() < 1e-9);
    assert_eq!((all[0].id, all[3].id, all[3].storey), (Some(7), Some(9), 0));
}

#[test]
fn a_window_is_a_hole_through_the_wall() {
    let all = meshes(&building());
    // A point in the window's middle (2.5, 0.125, 3 + 1.5) is outside the wall: a ray along x from it crosses an even
    // number of the wall's faces.
    let (o, wall) = ([2.5, 0.125, 4.5], &all[0]);
    let crossings = wall
        .triangles
        .iter()
        .filter(|t| {
            let [a, b, c] = t.map(|k| wall.vertices[k as usize]);
            // Möller–Trumbore along +x.
            let (e1, e2) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
            let d = [1.0, 0.0, 0.0];
            let p = [d[1] * e2[2] - d[2] * e2[1], d[2] * e2[0] - d[0] * e2[2], d[0] * e2[1] - d[1] * e2[0]];
            let det = e1[0] * p[0] + e1[1] * p[1] + e1[2] * p[2];
            if det.abs() < 1e-12 {
                return false;
            }
            let s = [o[0] - a[0], o[1] - a[1], o[2] - a[2]];
            let u = (s[0] * p[0] + s[1] * p[1] + s[2] * p[2]) / det;
            let q = [s[1] * e1[2] - s[2] * e1[1], s[2] * e1[0] - s[0] * e1[2], s[0] * e1[1] - s[1] * e1[0]];
            let v = (d[0] * q[0] + d[1] * q[1] + d[2] * q[2]) / det;
            let t = (e2[0] * q[0] + e2[1] * q[1] + e2[2] * q[2]) / det;
            u > 1e-9 && v > 1e-9 && u + v < 1.0 - 1e-9 && t > 1e-9
        })
        .count();
    assert_eq!(crossings % 2, 0, "{crossings}");
}
