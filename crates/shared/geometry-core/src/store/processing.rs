//! What the processing tools ask the store (docs/adr/0008, S4): the
//! "visible" scope's box test, corner numbering and edge-length labels of
//! objects by id. A run builds a store of the objects it reads, in the page
//! and in the processing worker alike (`apps/web/src/processing/geometry.ts`), so
//! both run this same code; the viewport's store answers the box test.

use std::collections::HashMap;

use super::{Store, padded};
use crate::entity::Shape;
use crate::geometry::Bounds;
use crate::ops::spatial_query::{Geometry, Relation, geometry_of, reach, relate};
use crate::processing::edge_lengths::{edge_lengths, edge_paths};
use crate::processing::numbering::{CornerRing, CornerWalk, number_corners};
use crate::vec2::Vec2;

/// Numbers per `number_corners` record: x, y, outward x and y, and whose number it takes (`Corner::refers`).
pub const CORNER_STRIDE: usize = 5;
/// Numbers per `edge_lengths` label, after the leading count of skipped shared edges: id, x, y, rotation, length.
pub const EDGE_LABEL_STRIDE: usize = 5;

impl Store {
    /// The pairs of an input and a reference in `relation` (docs/adr/0200
    /// §1), as positions in the two lists, flat (`i, j, i, j …`), inputs in
    /// their order and each one's references in theirs: Konuma göre seç and
    /// Bilgi al. References are looked for by their boxes first; an object is
    /// never paired with itself; unknown ids and construction lines take no
    /// part. Ayrık is not asked: the caller keeps the inputs Kesişen pairs with none.
    pub fn relate_pairs(
        &self,
        inputs: &[f64],
        references: &[f64],
        relation: Relation,
        within: f64,
    ) -> Vec<f64> {
        let refs: Vec<Option<Geometry>> = references
            .iter()
            .map(|&id| self.get(id).and_then(|it| geometry_of(&it.shape)))
            .collect();
        let mut place: HashMap<u64, usize> = HashMap::new();
        for (j, id) in references.iter().enumerate() {
            place.entry(id.to_bits()).or_insert(j);
        }
        let pad = reach(relation, within);
        let mut out = Vec::new();
        for (i, &id) in inputs.iter().enumerate() {
            let Some(g) = self.get(id).and_then(|it| geometry_of(&it.shape)) else {
                continue;
            };
            let mut near: Vec<usize> = self
                .candidates(&padded(g.bounds, pad))
                .into_iter()
                .filter_map(|it| place.get(&it.id.to_bits()).copied())
                .collect();
            near.sort_unstable();
            near.dedup();
            for j in near {
                if references[j] == id {
                    continue;
                }
                if refs[j]
                    .as_ref()
                    .is_some_and(|r| relate(&g, r, relation, within))
                {
                    out.push(i as f64);
                    out.push(j as f64);
                }
            }
        }
        out
    }

    /// Ids of objects on every layer whose box overlaps `r`, in the
    /// document's order: the "visible" scope's box test
    /// (`overlaps(entityBounds(e), view)`). A box with NaN never overlaps;
    /// a construction line's box is its base point.
    pub fn in_box(&self, r: &Bounds) -> Vec<f64> {
        self.candidates(&padded(*r, 0.0))
            .into_iter()
            .filter(|it| {
                let b = &it.bounds;
                b.min_x <= r.max_x && b.max_x >= r.min_x && b.min_y <= r.max_y && b.max_y >= r.min_y
            })
            .map(|it| it.id)
            .collect()
    }

    /// Corner numbering of these objects in the given order (unknown ids and
    /// kinds other than polygons and polylines left out): a polygon's outer
    /// ring then its holes, a polyline as an open path. `CORNER_STRIDE`
    /// numbers per corner, in numbering order (`number_corners`).
    pub fn number_corners(&self, ids: &[f64], walk: &CornerWalk, existing: &[Vec2]) -> Vec<f64> {
        let mut inputs = Vec::new();
        for &id in ids {
            match self.get(id).map(|it| &it.shape) {
                Some(Shape::Polygon {
                    pts, holes, parts, ..
                }) => {
                    let mut rings = vec![CornerRing { pts, closed: true }];
                    rings.extend(holes.iter().flatten().map(|h| CornerRing {
                        pts: &h.pts,
                        closed: true,
                    }));
                    // A multi-part area's other parts, each ring then its holes (docs/adr/0143).
                    for part in parts.iter().flatten() {
                        rings.push(CornerRing {
                            pts: &part.pts,
                            closed: true,
                        });
                        rings.extend(part.holes.iter().flatten().map(|h| CornerRing {
                            pts: &h.pts,
                            closed: true,
                        }));
                    }
                    inputs.push(rings);
                }
                Some(Shape::Polyline { pts, .. }) => {
                    inputs.push(vec![CornerRing { pts, closed: false }]);
                }
                _ => {}
            }
        }
        let corners = number_corners(&inputs, walk, existing);
        let mut out = Vec::with_capacity(corners.len() * CORNER_STRIDE);
        for c in corners {
            out.extend([c.p.x, c.p.y, c.out.x, c.out.y, c.refers]);
        }
        out
    }

    /// Edge-length labels of these objects in the given order (lines,
    /// polylines and polygons; `edge_lengths`): the number of shared edges
    /// skipped, then `EDGE_LABEL_STRIDE` numbers per label.
    pub fn edge_lengths(
        &self,
        ids: &[f64],
        height: f64,
        min_length: f64,
        inside: bool,
        shared: bool,
    ) -> Vec<f64> {
        let mut owners = Vec::new();
        let mut paths = Vec::new();
        for &id in ids {
            let Some(it) = self.get(id) else { continue };
            for p in edge_paths(&it.shape) {
                owners.push(id);
                paths.push(p);
            }
        }
        let (labels, skipped) = edge_lengths(&paths, height, min_length, inside, shared);
        let mut out = Vec::with_capacity(1 + labels.len() * EDGE_LABEL_STRIDE);
        out.push(skipped as f64);
        for (k, l) in labels {
            out.extend([owners[k], l.p.x, l.p.y, l.rotation, l.length]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::numbering::StartCorner;

    fn square(id: u32, layer: &str, x: f64, s: f64) -> String {
        format!(
            r#"{{"id":{id},"layerId":"{layer}","kind":"polygon","pts":[{{"x":{x},"y":0}},{{"x":{},"y":0}},{{"x":{},"y":{s}}},{{"x":{x},"y":{s}}}]}}"#,
            x + s,
            x + s
        )
    }

    fn store() -> Store {
        let mut s = Store::new();
        let xline =
            r#"{"id":4,"layerId":"b","kind":"xline","p":{"x":100,"y":100},"dir":{"x":1,"y":0}}"#;
        let bad = r#"{"id":5,"layerId":"a","kind":"line","a":{"x":null,"y":0},"b":{"x":1,"y":1}}"#;
        let line = r#"{"id":6,"layerId":"a","kind":"line","a":{"x":10,"y":10},"b":{"x":10,"y":0}}"#;
        s.put_json(&format!(
            "[{},{},{},{xline},{bad},{line}]",
            square(1, "a", 0.0, 10.0),
            square(2, "gizli", 10.0, 10.0),
            square(3, "a", 50.0, 10.0)
        ))
        .unwrap();
        s.set_layers_json(r#"[{"id":"gizli","visible":false,"locked":false,"pickInterior":true}]"#)
            .unwrap();
        s
    }

    #[test]
    fn the_box_test_takes_every_layer_but_never_a_nan_box() {
        let s = store();
        let r = Bounds {
            min_x: 5.0,
            min_y: 5.0,
            max_x: 15.0,
            max_y: 105.0,
        };
        // The hidden square too (the caller decides visibility), the line; not the NaN one.
        assert_eq!(s.in_box(&r), [1.0, 2.0, 6.0]);
        let base = Bounds {
            min_x: 99.0,
            min_y: 99.0,
            max_x: 101.0,
            max_y: 101.0,
        };
        assert_eq!(s.in_box(&base), [4.0]);
    }

    #[test]
    fn corners_and_edges_by_id_in_the_given_order() {
        let s = store();
        let walk = CornerWalk {
            ccw: false,
            start: StartCorner::First,
            point: None,
            tolerance: 0.001,
            shared: true,
        };
        let c = s.number_corners(&[2.0, 1.0, 6.0, 99.0], &walk, &[Vec2::new(20.0, 0.0)]);
        // Two squares of four corners; the line and the unknown id give none.
        assert_eq!(c.len(), 8 * CORNER_STRIDE);
        let refs: Vec<f64> = c.chunks(CORNER_STRIDE).map(|r| r[4]).collect();
        // Square 2 from its first vertex (10, 0), clockwise: (10,0), (10,10), (20,10), then (20,0) is existing point 0.
        assert_eq!(refs, [0.0, 1.0, 2.0, -1.0, 3.0, 4.0, 1.0, 0.0]);
        let e = s.edge_lengths(&[1.0, 2.0, 6.0, 3.0], 2.0, 0.0, false, true);
        // Twelve edges and the line; the shared edge twice more.
        assert_eq!(e[0], 2.0);
        assert_eq!(e.len(), 1 + 11 * EDGE_LABEL_STRIDE);
        assert_eq!(e[1], 1.0);
    }
}
