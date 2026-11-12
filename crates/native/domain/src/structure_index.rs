//! The slabs and the beams as a room's and a wall's height look for them (docs/adr/8051 §2): derived, per store, in
//! document order, each with its outline's rings (a beam's band) and their box. A room's floor and ceiling (docs/adr/8040
//! §3), a wall's foot and top (8040 §1) and a beam's top (8041 §1) asked every object of the drawing whether it is a
//! slab: a room schedule of 700 rooms did that 9 800 times a step, 43 million objects looked at (215 ms of a design
//! change at 100 flats).
//!
//! Nothing is decided here: the callers still read each slab and beam as the store holds it and apply their own rules.
//! A point outside a ring's box is never inside it (`point_in_polygon` is an even-odd test with exact orientation), so
//! the box only spares the test. Store writes of a slab or a beam drop the index; reading copies share it.

use std::sync::Arc;

use kentos_contracts::Entity;
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::geometry::point_in_polygon;

use crate::{Document, Slot, store::Store};

/// A slab's or a beam's outline: its rings (outer, then the holes) and their box.
struct Outline {
    slot: Slot,
    /// Min x, min y, max x, max y of every ring.
    bounds: [f64; 4],
    /// Each part's outer ring and its holes (a beam: its band, no hole).
    parts: Vec<(Vec<V>, Vec<Vec<V>>)>,
}

impl Outline {
    fn new(slot: Slot, parts: Vec<(Vec<V>, Vec<Vec<V>>)>) -> Self {
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in parts
            .iter()
            .flat_map(|(outer, holes)| outer.iter().chain(holes.iter().flatten()))
        {
            bounds = [
                bounds[0].min(p.x),
                bounds[1].min(p.y),
                bounds[2].max(p.x),
                bounds[3].max(p.y),
            ];
        }
        Self {
            slot,
            bounds,
            parts,
        }
    }

    /// Whether one of its parts holds `at`: inside its outer ring and none of its holes. A point outside the box is
    /// in none of them; one whose coordinate is not a number is left to the rings' test, as before.
    fn holds(&self, at: V) -> bool {
        let [x0, y0, x1, y1] = self.bounds;
        if at.x < x0 || at.x > x1 || at.y < y0 || at.y > y1 {
            return false;
        }
        self.parts.iter().any(|(outer, holes)| {
            point_in_polygon(at, outer) && !holes.iter().any(|h| point_in_polygon(at, h))
        })
    }
}

pub(crate) struct StructureIndex {
    slabs: Vec<Outline>,
    beams: Vec<Outline>,
}

impl std::fmt::Debug for StructureIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructureIndex")
            .field("slabs", &self.slabs.len())
            .field("beams", &self.beams.len())
            .finish()
    }
}

fn ring(pts: &[kentos_contracts::Vec2]) -> Vec<V> {
    pts.iter().map(|q| V::new(q.x, q.y)).collect()
}

impl StructureIndex {
    fn build(store: &Store) -> Self {
        let mut slabs = Vec::new();
        let mut beams = Vec::new();
        for stored in store.iter() {
            match &*stored.entity {
                Entity::Slab(s) => slabs.push(Outline::new(
                    stored.slot(),
                    s.slab
                        .parts
                        .iter()
                        .map(|p| {
                            (
                                ring(&p.outer.pts),
                                p.holes.iter().map(|h| ring(&h.pts)).collect(),
                            )
                        })
                        .collect(),
                )),
                Entity::Beam(b) => {
                    let band = match kentos_formats::arch_elements::core_beam(&b.beam) {
                        kentos_geometry_core::entity::Shape::Beam { pts, beam } => {
                            kentos_geometry_core::geom::beam::band(&pts, &beam)
                        }
                        _ => Vec::new(),
                    };
                    beams.push(Outline::new(stored.slot(), vec![(band, Vec::new())]));
                }
                _ => {}
            }
        }
        Self { slabs, beams }
    }
}

/// Whether a stored object is one the index lists: what a store write of it drops the index for.
pub(crate) fn indexed(entity: &Entity) -> bool {
    matches!(entity, Entity::Slab(_) | Entity::Beam(_))
}

impl Document {
    fn structure(&self) -> &StructureIndex {
        self.store
            .structure
            .get_or_init(|| Arc::new(StructureIndex::build(&self.store)))
    }

    /// The slabs whose outline holds `at`, in document order.
    pub(crate) fn slabs_at(
        &self,
        at: kentos_contracts::Vec2,
    ) -> impl Iterator<Item = &kentos_contracts::SlabFields> {
        let p = V::new(at.x, at.y);
        self.structure()
            .slabs
            .iter()
            .filter(move |o| o.holds(p))
            .filter_map(|o| match self.store.get(o.slot).map(|s| &*s.entity) {
                Some(Entity::Slab(s)) => Some(&s.slab),
                _ => None,
            })
    }

    /// The beams whose band holds `at`, in document order.
    pub(crate) fn beams_at(
        &self,
        at: kentos_contracts::Vec2,
    ) -> impl Iterator<Item = &kentos_contracts::BeamFields> {
        let p = V::new(at.x, at.y);
        self.structure()
            .beams
            .iter()
            .filter(move |o| o.holds(p))
            .filter_map(|o| match self.store.get(o.slot).map(|s| &*s.entity) {
                Some(Entity::Beam(b)) => Some(&b.beam),
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wall_height::tests::{document, slab};
    use kentos_contracts::{BeamFields, SlabFields, Vec2};

    /// A small generator (xorshift): the same drawing and points every run.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }

        /// On a 0.25 m grid now and then: points on the rings' corners, edges and boxes.
        fn coordinate(&mut self, lo: f64, hi: f64) -> f64 {
            let v = lo + (hi - lo) * self.next();
            if self.next() < 0.3 {
                (v * 4.0).round() / 4.0
            } else {
                v
            }
        }
    }

    /// Every object looked at, as the heights looked before the index: a slab whose rings hold `at`.
    fn slabs_scanned(doc: &Document, at: Vec2) -> Vec<*const SlabFields> {
        let p = V::new(at.x, at.y);
        doc.entities()
            .filter_map(|e| match e {
                Entity::Slab(s) => Some(&s.slab),
                _ => None,
            })
            .filter(|s| {
                s.parts.iter().any(|part| {
                    point_in_polygon(p, &ring(&part.outer.pts))
                        && !part
                            .holes
                            .iter()
                            .any(|h| point_in_polygon(p, &ring(&h.pts)))
                })
            })
            .map(|s| s as *const SlabFields)
            .collect()
    }

    /// A beam whose band holds `at`, every object looked at.
    fn beams_scanned(doc: &Document, at: Vec2) -> Vec<*const BeamFields> {
        let p = V::new(at.x, at.y);
        doc.entities()
            .filter_map(|e| match e {
                Entity::Beam(b) => Some(&b.beam),
                _ => None,
            })
            .filter(|b| match kentos_formats::arch_elements::core_beam(b) {
                kentos_geometry_core::entity::Shape::Beam { pts, beam } => {
                    point_in_polygon(p, &kentos_geometry_core::geom::beam::band(&pts, &beam))
                }
                _ => false,
            })
            .map(|b| b as *const BeamFields)
            .collect()
    }

    fn indexed_slabs(doc: &Document, at: Vec2) -> Vec<*const SlabFields> {
        doc.slabs_at(at).map(|s| s as *const SlabFields).collect()
    }

    fn indexed_beams(doc: &Document, at: Vec2) -> Vec<*const BeamFields> {
        doc.beams_at(at).map(|b| b as *const BeamFields).collect()
    }

    /// 60 slabs (some with a hole, some of two parts, some on top of each other) and 40 beams of one to three
    /// stretches, 4 000 points: the index finds the slabs and the beams every object looked at finds, in the same
    /// order.
    #[test]
    fn the_index_finds_what_every_object_looked_at_finds() {
        let mut r = Rng(0x9e37_79b9_7f4a_7c15);
        let mut objects = Vec::new();
        for i in 0..60 {
            let (x0, y0) = (r.coordinate(-20.0, 20.0), r.coordinate(-20.0, 20.0));
            let (w, h) = (0.5 + 8.0 * r.next(), 0.5 + 8.0 * r.next());
            let rect = |x0: f64, y0: f64, w: f64, h: f64| {
                serde_json::json!({"pts": [
                    {"x": x0, "y": y0}, {"x": x0 + w, "y": y0}, {"x": x0 + w, "y": y0 + h}, {"x": x0, "y": y0 + h}
                ]})
            };
            let mut part = serde_json::json!({"outer": rect(x0, y0, w, h)});
            if i % 3 == 0 {
                part["holes"] =
                    serde_json::json!([rect(x0 + w / 4.0, y0 + h / 4.0, w / 2.0, h / 2.0)]);
            }
            let mut parts = vec![part];
            if i % 5 == 0 {
                parts.push(serde_json::json!({"outer": rect(x0 + w + 1.0, y0, w / 2.0, h)}));
            }
            objects.push(serde_json::json!({
                "parts": parts, "thickness": 0.15, "offset": 0.0, "arch": {"level": if i % 2 == 0 { "zemin" } else { "k1" }}
            }));
        }
        for i in 0..40 {
            let n = 2 + i % 3;
            let pts: Vec<serde_json::Value> = (0..n)
                .map(|_| serde_json::json!({"x": r.coordinate(-20.0, 20.0), "y": r.coordinate(-20.0, 20.0)}))
                .collect();
            let justify = ["left", "center", "right"][i % 3];
            objects.push(serde_json::json!({
                "kind": "beam", "pts": pts, "width": 0.2 + r.next(), "height": 0.5, "justify": justify,
                "arch": {"level": "zemin"}
            }));
        }
        let doc = document(serde_json::Value::Array(objects));
        let (mut slabs, mut beams) = (0, 0);
        for _ in 0..4000 {
            let at = Vec2 {
                x: r.coordinate(-22.0, 32.0),
                y: r.coordinate(-22.0, 32.0),
            };
            let scanned = slabs_scanned(&doc, at);
            assert_eq!(indexed_slabs(&doc, at), scanned, "{at:?}");
            let scanned_beams = beams_scanned(&doc, at);
            assert_eq!(indexed_beams(&doc, at), scanned_beams, "{at:?}");
            slabs += scanned.len();
            beams += scanned_beams.len();
        }
        // The points found many slabs and beams, not only none.
        assert!(slabs > 400 && beams > 20, "{slabs} {beams}");
    }

    /// The index follows the drawing: a slab added, moved, its step undone and redone, removed; a reading copy shares
    /// the index and its edits leave the drawing's as it was.
    #[test]
    fn the_index_follows_the_drawing() {
        let mut doc = document(serde_json::json!([slab("zemin", 0.0, 4.0, 0.15, 0.0)]));
        let at = Vec2 { x: 6.0, y: 2.0 };
        assert!(indexed_slabs(&doc, at).is_empty());
        let added: Entity = serde_json::from_value({
            let mut s = slab("zemin", 5.0, 8.0, 0.2, 0.0);
            s["kind"] = serde_json::json!("slab");
            s["id"] = serde_json::json!(0);
            s["layerId"] = serde_json::json!("0");
            s["attrs"] = serde_json::json!({});
            s
        })
        .unwrap();
        let slot = doc.add(added.clone()).unwrap();
        assert_eq!(indexed_slabs(&doc, at), slabs_scanned(&doc, at));
        assert_eq!(indexed_slabs(&doc, at).len(), 1);
        // Moved 10 m east: no longer over the point.
        let mut moved = added.clone();
        if let Entity::Slab(s) = &mut moved {
            for p in s.slab.parts.iter_mut().flat_map(|p| p.outer.pts.iter_mut()) {
                p.x += 10.0;
            }
        }
        assert!(doc.update(slot, moved));
        assert!(indexed_slabs(&doc, at).is_empty());
        assert_eq!(indexed_slabs(&doc, Vec2 { x: 16.0, y: 2.0 }).len(), 1);
        doc.undo();
        assert_eq!(indexed_slabs(&doc, at), slabs_scanned(&doc, at));
        assert_eq!(indexed_slabs(&doc, at).len(), 1);
        doc.redo();
        assert!(indexed_slabs(&doc, at).is_empty());
        // A reading copy: its own edits are its own.
        let mut copy = doc.reading_copy();
        assert_eq!(indexed_slabs(&copy, Vec2 { x: 16.0, y: 2.0 }).len(), 1);
        assert_eq!(copy.remove(&[slot]), 1);
        assert!(indexed_slabs(&copy, Vec2 { x: 16.0, y: 2.0 }).is_empty());
        assert_eq!(indexed_slabs(&doc, Vec2 { x: 16.0, y: 2.0 }).len(), 1);
        assert_eq!(doc.remove(&[slot]), 1);
        assert!(indexed_slabs(&doc, Vec2 { x: 16.0, y: 2.0 }).is_empty());
        assert_eq!(indexed_slabs(&doc, Vec2 { x: 2.0, y: 2.0 }).len(), 1);
    }
}
