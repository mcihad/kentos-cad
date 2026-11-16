//! Whether a body is a closed solid (docs/adr/8071 §1; plan/bim-plan.md BIM-10: "her gövde kapalı ve kendini kesmiyor"):
//! what keeps a shape the file writes from bounding one volume. A prism is closed when each ring of its profile is a
//! simple closed curve (three corners at least, an area, no edge crossing or touching another but its neighbours at
//! their shared corner, arcs measured as arcs) and its holes lie in its outer rings, as the file nests them; its depth
//! above zero. A set of faces is closed when every edge is met by exactly two faces, once each way (a closed, oriented
//! surface), and no face has fewer than three corners.

use std::collections::BTreeMap;

use kentos_geometry_core::Vec2 as P;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::bulge::bulge_path_edges;
use kentos_geometry_core::geom::intersect::intersect_edges;
use kentos_geometry_core::geom::region::ring_area;

use crate::building::{Building, Opening, Shape, View, flat_rings};

/// What keeps `shape` from being a closed solid, in words; empty when it is one.
pub fn faults(shape: &Shape) -> Vec<String> {
    match shape {
        Shape::Prism {
            rings,
            bulges,
            bottom,
            top,
        } => prism(rings, bulges, *bottom, *top),
        Shape::Faces { vertices, faces } => face_set(vertices.len(), faces),
    }
}

/// The bodies of `b` as its file writes them (a Reference View's cut, docs/adr/8071 §4) that are not closed solids,
/// its elements' and its spaces': each one's name and the first thing wrong with it.
pub fn open_bodies(b: &Building) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, e) in b.elements.iter().enumerate() {
        let shapes = match b.view {
            View::DesignTransfer => e.shapes.clone(),
            View::Reference => {
                let hosted: Vec<&Opening> = b
                    .openings
                    .iter()
                    .filter(|o| o.host == i && o.ring.len() >= 2)
                    .collect();
                crate::quantities::cut(&e.shapes, &hosted)
            }
        };
        if let Some(f) = shapes.iter().flat_map(faults).next() {
            out.push((e.name.clone(), f));
        }
    }
    for s in &b.spaces {
        if let Some(f) = s.shapes().iter().flat_map(faults).next() {
            out.push((s.name.clone(), f));
        }
    }
    out
}

fn prism(rings: &[Vec<[f64; 2]>], bulges: &[Vec<f64>], bottom: f64, top: f64) -> Vec<String> {
    let mut out = Vec::new();
    if top - bottom <= 0.0 {
        out.push(format!("derinliği yok ({bottom} → {top})"));
    }
    let flat = flat_rings(rings, bulges);
    for (i, r) in rings.iter().enumerate() {
        if flat[i].len() < 3 {
            out.push(format!("{}. halkanın üç köşesi yok", i + 1));
            continue;
        }
        let pts: Vec<P> = r.iter().map(|p| P::new(p[0], p[1])).collect();
        let b = bulges.get(i).cloned().unwrap_or_default();
        let ring = Ring {
            pts: pts.clone(),
            bulges: Some(b.clone()),
        };
        if ring_area(&ring).abs() <= 1e-12 {
            out.push(format!("{}. halkanın alanı yok", i + 1));
            continue;
        }
        let edges = bulge_path_edges(&pts, Some(&b), true);
        let n = edges.len();
        for j in 0..n {
            for k in j + 1..n {
                // Neighbours meet at their shared corner, and only there.
                let corner = if k == j + 1 {
                    Some(pts[k % pts.len()])
                } else if j == 0 && k == n - 1 {
                    Some(pts[0])
                } else {
                    None
                };
                for h in intersect_edges(&edges[j], &edges[k]) {
                    let at_corner = corner.is_some_and(|c| {
                        (h.p.x - c.x).abs().max((h.p.y - c.y).abs())
                            <= 1e-9 * c.x.abs().max(c.y.abs()).max(1.0)
                    });
                    if !at_corner {
                        out.push(format!(
                            "{}. halkanın {}. ve {}. kenarı ({:.6}, {:.6}) noktasında kesişiyor",
                            i + 1,
                            j + 1,
                            k + 1,
                            h.p.x,
                            h.p.y
                        ));
                    }
                }
            }
        }
    }
    // The file nests the rings by the even-odd rule (a hole in an outer ring, a ring no other holds another outer): no
    // two rings may cross or touch.
    let kept: Vec<usize> = (0..rings.len()).filter(|&i| flat[i].len() >= 3).collect();
    for (x, &i) in kept.iter().enumerate() {
        for &j in &kept[x + 1..] {
            let e = |k: usize| {
                let pts: Vec<P> = rings[k].iter().map(|p| P::new(p[0], p[1])).collect();
                bulge_path_edges(
                    &pts,
                    Some(&bulges.get(k).cloned().unwrap_or_default()),
                    true,
                )
            };
            let (a, b) = (e(i), e(j));
            if a.iter()
                .any(|ea| b.iter().any(|eb| !intersect_edges(ea, eb).is_empty()))
            {
                out.push(format!("{}. ve {}. halka kesişiyor", i + 1, j + 1));
            }
        }
    }
    out
}

fn face_set(count: usize, faces: &[Vec<usize>]) -> Vec<String> {
    let mut out = Vec::new();
    // Each directed edge's count: a closed oriented surface meets every edge once each way.
    let mut edges: BTreeMap<(usize, usize), i32> = BTreeMap::new();
    for (k, f) in faces.iter().enumerate() {
        if f.len() < 3 {
            out.push(format!("{}. yüzün üç köşesi yok", k + 1));
            continue;
        }
        if f.iter().any(|&v| v >= count) {
            out.push(format!("{}. yüz olmayan bir köşeyi anıyor", k + 1));
            continue;
        }
        for i in 0..f.len() {
            let (a, b) = (f[i], f[(i + 1) % f.len()]);
            *edges.entry((a, b)).or_insert(0) += 1;
        }
    }
    for (&(a, b), &n) in &edges {
        let back = edges.get(&(b, a)).copied().unwrap_or(0);
        if n != 1 || back != 1 {
            out.push(format!(
                "{a}–{b} kenarı {n} kez bu yönde, {back} kez ters yönde"
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ring_with_arcs_is_closed_and_a_bow_tie_is_not() {
        let quarter = (std::f64::consts::PI / 8.0).tan();
        let curved = Shape::Prism {
            rings: vec![vec![[6.15, 0.0], [0.0, 6.15], [0.0, 5.85], [5.85, 0.0]]],
            bulges: vec![vec![quarter, 0.0, -quarter, 0.0]],
            bottom: 0.0,
            top: 3.0,
        };
        assert_eq!(faults(&curved), Vec::<String>::new());
        let bow = Shape::Prism {
            rings: vec![vec![[0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 1.0]]],
            bulges: vec![],
            bottom: 0.0,
            top: 1.0,
        };
        assert_eq!(faults(&bow).len(), 1, "{:?}", faults(&bow));
        // An arc bowing 0.40 m across its own ring's far side 0.20 m off: it crosses it twice.
        let bulging = Shape::Prism {
            rings: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 0.2], [0.0, 0.2]]],
            bulges: vec![vec![-0.8, 0.0, 0.0, 0.0]],
            bottom: 0.0,
            top: 1.0,
        };
        assert_eq!(faults(&bulging).len(), 2, "{:?}", faults(&bulging));
    }

    #[test]
    fn a_box_of_faces_is_closed_and_one_missing_face_is_not() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let mut faces = vec![vec![4, 5, 6, 7], vec![3, 2, 1, 0]];
        for i in 0..4 {
            let j = (i + 1) % 4;
            faces.push(vec![i, j, j + 4, i + 4]);
        }
        assert_eq!(
            faults(&Shape::Faces {
                vertices: vertices.clone(),
                faces: faces.clone()
            }),
            Vec::<String>::new()
        );
        faces.pop();
        assert!(!faults(&Shape::Faces { vertices, faces }).is_empty());
    }
}
