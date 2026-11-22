//! A room under a roof (docs/adr/8071 §11; EK-9 satır 14: "Mahaller kapalı olarak ve üst sınırları doğru tanımlandı
//! mı?"): with no slab, no asma tavan and no height of its own over it, a room on the storey a roof stands on rises to
//! the roof's underside. Its outline is cut by the roof's faces in plan; each piece stands from the room's floor up to
//! its face's underside, a plane over the whole piece. Its volume is each piece's area by that plane's height over the
//! floor at the piece's centroid (a plane's mean over an area is its value at the centroid: exact, arcs and all); its
//! body each piece a closed face set, its arcs flattened at the drawing's step.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::centroid::area_centroid;
use kentos_geometry_core::geom::region::{intersect_area_sets, net_area};

use crate::building::{Shape, flat_rings};

/// A plane `z = c + dx·x + dy·y`.
pub type Plane = (f64, f64, f64);

/// A roof face over a room: its outline in plan and its underside's plane.
pub type UnderFace = (Vec<[f64; 2]>, Plane);

fn height(p: Plane, at: Vec2) -> f64 {
    p.0 + p.1 * at.x + p.2 * at.y
}

/// The room's pieces under the faces, each its area (arcs kept) and its face's underside; none when some of the room is
/// under no face, or a piece has a hole (a face set here has no inner loops).
pub fn pieces(rings: &[Vec<[f64; 2]>], bulges: &[Vec<f64>], faces: &[UnderFace]) -> Option<Vec<(Area, Plane)>> {
    let room = crate::quantities::areas(rings, bulges);
    let whole: f64 = room.iter().map(net_area).sum();
    if whole.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    let mut out = Vec::new();
    let mut covered = 0.0;
    for (ring, plane) in faces {
        let face = Area {
            outer: Ring { pts: ring.iter().map(|p| Vec2::new(p[0], p[1])).collect(), bulges: None },
            holes: Vec::new(),
        };
        for a in intersect_area_sets(&[room.clone(), vec![face]]) {
            let s = net_area(&a);
            if s <= 1e-12 * whole {
                continue;
            }
            if !a.holes.is_empty() {
                return None;
            }
            covered += s;
            out.push((a, *plane));
        }
    }
    ((whole - covered).abs() <= 1e-9 * whole.max(1.0)).then_some(out)
}

/// The pieces' volume from `floor` up to their planes.
pub fn volume(pieces: &[(Area, Plane)], floor: f64) -> f64 {
    pieces
        .iter()
        .filter_map(|(a, p)| {
            let c = area_centroid((&a.outer.pts, a.outer.bulges.as_deref()), std::iter::empty())?;
            Some(net_area(a) * (height(*p, c) - floor))
        })
        .sum()
}

/// The highest point of the pieces' tops.
pub fn top(pieces: &[(Area, Plane)]) -> f64 {
    pieces
        .iter()
        .flat_map(|(a, p)| a.outer.pts.iter().map(move |q| height(*p, *q)))
        .fold(f64::NEG_INFINITY, f64::max)
}

/// Each piece as a closed face set, outward: its outline (counter-clockwise, arcs flattened) at `floor`, the same up on
/// its plane, and an upright side under each edge.
pub fn shapes(pieces: &[(Area, Plane)], floor: f64) -> Vec<Shape> {
    pieces
        .iter()
        .filter_map(|(a, p)| {
            let ring: Vec<[f64; 2]> = a.outer.pts.iter().map(|q| [q.x, q.y]).collect();
            let bulges = a.outer.bulges.clone().unwrap_or_default();
            let mut flat = flat_rings(&[ring], &[bulges]).pop()?;
            if flat.len() > 1 && flat.first() == flat.last() {
                flat.pop();
            }
            let n = flat.len();
            if n < 3 {
                return None;
            }
            let twice: f64 = (0..n).map(|i| flat[i][0] * flat[(i + 1) % n][1] - flat[(i + 1) % n][0] * flat[i][1]).sum();
            if twice < 0.0 {
                flat.reverse();
            }
            let mut vertices: Vec<[f64; 3]> = flat.iter().map(|q| [q[0], q[1], floor]).collect();
            vertices.extend(flat.iter().map(|q| [q[0], q[1], height(*p, Vec2::new(q[0], q[1]))]));
            let mut faces: Vec<Vec<usize>> = vec![(0..n).rev().collect(), (n..2 * n).collect()];
            faces.extend((0..n).map(|i| vec![i, (i + 1) % n, n + (i + 1) % n, n + i]));
            Some(Shape::Faces { vertices, faces })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4 × 3 room under a shed whose underside rises from 2.5 at y 0 to 4 at y 3 (z = 2.5 + 0.5·y), cut by the face
    /// line y 1 into two faces of the same plane: its volume 4 · 3 · (2.5 + 0.5 · 1.5) = 39, its pieces closed, its
    /// highest point 4.
    #[test]
    fn a_room_rises_to_the_roof_over_it() {
        let rings = vec![vec![[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]]];
        let plane = (2.5, 0.0, 0.5);
        let faces = vec![
            (vec![[-1.0, -1.0], [5.0, -1.0], [5.0, 1.0], [-1.0, 1.0]], plane),
            (vec![[-1.0, 1.0], [5.0, 1.0], [5.0, 4.0], [-1.0, 4.0]], plane),
        ];
        let p = pieces(&rings, &[vec![]], &faces).expect("wholly under the roof");
        assert_eq!(p.len(), 2);
        assert!((volume(&p, 0.0) - 39.0).abs() < 1e-9, "{}", volume(&p, 0.0));
        assert!((top(&p) - 4.0).abs() < 1e-12);
        let body = shapes(&p, 0.0);
        let v: f64 = body.iter().map(crate::quantities::volume).sum();
        assert!((v - 39.0).abs() < 1e-9, "{v}");
        for s in &body {
            assert_eq!(crate::closed::faults(s), Vec::<String>::new());
        }
        // A room reaching past the roof's faces is not under it.
        let short = vec![(vec![[-1.0, -1.0], [5.0, -1.0], [5.0, 2.0], [-1.0, 2.0]], plane)];
        assert!(pieces(&rings, &[vec![]], &short).is_none());
    }

    /// A room with an arc (a half disc of radius 2 on its straight side) under a level underside 3 m over its floor:
    /// its volume the half disc's area by 3, exactly.
    #[test]
    fn an_arc_is_measured_as_an_arc() {
        let rings = vec![vec![[2.0, 0.0], [-2.0, 0.0]]];
        let bulges = vec![vec![1.0, 0.0]];
        let faces = vec![(vec![[-3.0, -3.0], [3.0, -3.0], [3.0, 3.0], [-3.0, 3.0]], (3.0, 0.0, 0.0))];
        let p = pieces(&rings, &bulges, &faces).expect("under the roof");
        let want = std::f64::consts::PI * 4.0 / 2.0 * 3.0;
        assert!((volume(&p, 0.0) - want).abs() < 1e-9, "{} {want}", volume(&p, 0.0));
    }
}
