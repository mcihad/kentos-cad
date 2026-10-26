//! A derived, per-store candidate index for the wall join command's existing reach boxes.
//! No geometry/join result or layer state is cached. Store writes invalidate only this
//! memoized index; reading copies clone its immutable Arc into an independent OnceLock.
use std::sync::Arc;

use crate::{Document, Slot, store::Store};
use kentos_contracts::Entity;
use kentos_geometry_core::{Vec2, geometry::Bounds, store::rtree::PackedTree};

pub(crate) struct WallCandidates {
    tree: PackedTree,
    unusual: Vec<(u32, Bounds)>,
    slots: Vec<Slot>,
}
impl std::fmt::Debug for WallCandidates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WallCandidates")
            .field("walls", &self.slots.len())
            .field("unusual", &self.unusual.len())
            .finish_non_exhaustive()
    }
}
impl WallCandidates {
    fn build(store: &Store) -> Self {
        let mut entries = Vec::new();
        let mut unusual = Vec::new();
        let mut slots = Vec::new();
        for stored in store.iter() {
            let Entity::Wall(w) = &*stored.entity else {
                continue;
            };
            let token = slots.len() as u32;
            slots.push(stored.slot());
            let b = kentos_geometry_core::geom::wall::join_reach_bounds(
                w.wall.pts.iter().map(|p| Vec2::new(p.x, p.y)),
                w.wall.width,
                w.wall.offset,
                0.,
            );
            if [b.min_x, b.min_y, b.max_x, b.max_y]
                .iter()
                .all(|v| v.is_finite())
                && b.min_x <= b.max_x
                && b.min_y <= b.max_y
            {
                entries.push((token, b));
            } else {
                // Defensive compatibility for programmatically constructed invalid data:
                // the old intersection predicate still decides, without invalid tree boxes.
                unusual.push((token, b));
            }
        }
        Self {
            tree: PackedTree::build(&entries),
            unusual,
            slots,
        }
    }
}
impl Document {
    /// Candidate wall slots in document order for the join command's conservative path boxes.
    /// Refines no geometry, ignores no hidden/locked/phased walls: the existing caller still
    /// reads each current wall and applies its own join rules. Empty queries allocate no index.
    pub fn wall_candidates(&self, boxes: &[[f64; 4]]) -> Vec<Slot> {
        if boxes.is_empty() {
            return Vec::new();
        }
        let index = self
            .store
            .wall_candidates
            .get_or_init(|| Arc::new(WallCandidates::build(&self.store)));
        let mut slots = Vec::new();
        for b in boxes {
            let bounds = Bounds {
                min_x: b[0],
                min_y: b[1],
                max_x: b[2],
                max_y: b[3],
            };
            index.tree.search(&bounds, &mut slots);
            for (id, unusual) in &index.unusual {
                if bounds.min_x <= unusual.max_x
                    && unusual.min_x <= bounds.max_x
                    && bounds.min_y <= unusual.max_y
                    && unusual.min_y <= bounds.max_y
                {
                    slots.push(*id);
                }
            }
        }
        slots.sort_unstable();
        slots.dedup();
        // Tokens were assigned in document order; sorting them avoids repeated
        // store/order lookups for a large query and restores the old scan order.
        slots
            .into_iter()
            .filter_map(|token| index.slots.get(token as usize).copied())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::External;
    use kentos_contracts::DocumentSnapshotV1;
    use serde_json::{Value, json};

    fn drawing() -> Document {
        let mut v: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/commands/v1/cad.walls.rectangle.json"
        ))
        .unwrap();
        v["setup"]["entities"] = json!([]);
        Document::from_snapshot(DocumentSnapshotV1::from_json(&v["setup"].to_string()).unwrap())
            .unwrap()
    }
    fn wall(x: f64, y: f64, width: f64, offset: f64) -> Entity {
        serde_json::from_value(json!({"kind":"wall","id":0,"layerId":"duvar","attrs":{},
            "pts":[{"x":x,"y":y},{"x":x+4.,"y":y+3.}],"width":width,"offset":offset}))
        .unwrap()
    }
    fn point() -> Entity {
        serde_json::from_value(json!({"kind":"point","id":0,"layerId":"duvar","attrs":{},
            "p":{"x":0.,"y":0.}}))
        .unwrap()
    }
    // Frozen pre-index algorithm. Do not call the shared bounds helper here:
    // differential tests must catch an accidental arithmetic/policy change too.
    fn scan(doc: &Document, boxes: &[[f64; 4]]) -> Vec<Slot> {
        doc.entities()
            .filter_map(|e| {
                let Entity::Wall(w) = e else { return None };
                let r = w.wall.width + w.wall.offset.abs();
                let mut b = [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ];
                for p in &w.wall.pts {
                    b[0] = b[0].min(p.x - r);
                    b[1] = b[1].min(p.y - r);
                    b[2] = b[2].max(p.x + r);
                    b[3] = b[3].max(p.y + r);
                }
                boxes
                    .iter()
                    .any(|a| a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3])
                    .then_some(Slot(e.base().id))
            })
            .collect()
    }
    fn check(doc: &Document, boxes: &[[f64; 4]]) -> Vec<Slot> {
        let expected = scan(doc, boxes);
        assert_eq!(doc.wall_candidates(boxes), expected);
        expected
    }
    const LOCAL: [[f64; 4]; 1] = [[-1., -1., 6., 5.]];

    #[test]
    fn candidates_match_the_old_scan_for_varied_paths_offsets_large_coordinates_and_order() {
        let mut doc = drawing();
        assert!(doc.wall_candidates(&[]).is_empty());
        assert!(
            doc.store.wall_candidates.get().is_none(),
            "empty query does not build"
        );
        let mut walls = Vec::new();
        for i in 0..640 {
            let x = (i % 32) as f64 * 7.;
            let y = (i / 32) as f64 * 9.;
            let mut e = wall(x, y, 0.1 + (i % 5) as f64 * 0.2, (i % 7) as f64 * 0.3 - 0.9);
            let Entity::Wall(w) = &mut e else {
                unreachable!()
            };
            if i % 3 == 0 {
                w.wall.pts.push(kentos_contracts::Vec2 {
                    x: x - 2.,
                    y: y + 6.,
                });
                w.wall.closed = true;
            }
            if i % 5 == 0 {
                w.wall.bulges = Some(vec![0.25, 0.]);
            }
            walls.push(e);
            if i % 6 == 0 {
                walls.push(point());
            }
        }
        walls.push(wall(500_000., 4_400_000., 1.3, -0.7));
        let ids = doc.add_many(walls, "seed").unwrap();
        // Delete/reinsert deliberately shuffled slots: order comes from the document,
        // never the R-tree's Hilbert traversal or a numeric slot sort.
        doc.remove(&[ids[0], ids[4]]);
        doc.undo();
        let before = doc.to_snapshot_v2();
        let revision = doc.revision();
        for i in 0..100 {
            let x = (i % 20) as f64 * 10. - 10.;
            let y = (i / 20) as f64 * 12. - 5.;
            let b = [x, y, x + 17., y + 14.];
            check(&doc, &[b, b, [500_000., 4_400_000., 500_005., 4_400_004.]]);
        }
        // Inclusive touching, no match, entire store and reversed/NaN queries.
        for b in [
            [-0.2, -0.2, -0.2, -0.2],
            [1e6, 1e6, 1e6 + 1., 1e6 + 1.],
            [
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::INFINITY,
            ],
            [6., 5., -1., -1.],
            [f64::NAN, 0., 10., 10.],
        ] {
            check(&doc, &[b]);
        }
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.revision(), revision);
    }

    #[test]
    fn imported_nonmonotonic_slots_keep_document_order_and_inclusive_touching() {
        let mut doc = drawing();
        let mut snapshot = doc.to_snapshot();
        let mut entities = vec![
            wall(0., 0., 0.2, 0.),
            wall(0., 0., 0.2, 0.),
            wall(20., 20., 0.2, 0.),
        ];
        for (e, id) in entities.iter_mut().zip([91, 3, 60]) {
            let Entity::Wall(w) = e else { unreachable!() };
            w.base.id = id;
        }
        snapshot.entities = entities;
        doc = Document::from_snapshot(snapshot).unwrap();
        assert_eq!(check(&doc, &LOCAL), vec![Slot(91), Slot(3)]);
        assert_eq!(
            check(&doc, &[[-0.2, -0.2, -0.2, -0.2]]),
            vec![Slot(91), Slot(3)]
        );
        assert!(check(&doc, &[[-0.2000001, -0.2, -0.2000001, -0.2]]).is_empty());
    }

    #[test]
    fn edits_type_changes_delete_undo_redo_and_rollbacks_invalidate_a_warm_index() {
        let mut doc = drawing();
        let a = doc.add(wall(0., 0., 0.2, 0.)).unwrap();
        assert_eq!(check(&doc, &LOCAL), vec![a]);
        doc.update(a, wall(100., 100., 0.2, 0.));
        assert!(check(&doc, &LOCAL).is_empty());
        doc.undo();
        assert_eq!(check(&doc, &LOCAL), vec![a]);
        doc.redo();
        assert!(check(&doc, &LOCAL).is_empty());
        // Width and offset alone can bring an otherwise distant wall into range.
        doc.update(a, wall(10., 0., 0.2, -10.));
        assert_eq!(check(&doc, &LOCAL), vec![a]);
        doc.update(a, point());
        assert!(check(&doc, &LOCAL).is_empty());
        doc.update(a, wall(0., 0., 0.2, 0.));
        assert_eq!(check(&doc, &LOCAL), vec![a]);
        doc.remove(&[a]);
        assert!(check(&doc, &LOCAL).is_empty());
        doc.undo();
        assert_eq!(check(&doc, &LOCAL), vec![a]);
        let before = doc.to_snapshot_v2();
        let group = doc.begin_group("cancel");
        doc.update(a, wall(100., 100., 0.2, 0.));
        assert!(check(&doc, &LOCAL).is_empty());
        let b = doc.add(wall(0., 0., 0.2, 0.)).unwrap();
        assert_eq!(check(&doc, &LOCAL), vec![b]);
        doc.cancel_group(group);
        assert_eq!(check(&doc, &LOCAL), vec![a]);
        assert_eq!(doc.to_snapshot_v2(), before);
        let refused: Result<(_, _), &str> = doc.run_step(
            |d| {
                d.update(a, wall(100., 100., 0.2, 0.));
                assert!(check(d, &LOCAL).is_empty());
                Err::<(), _>("refused")
            },
            |_, _, _| Ok(None),
        );
        assert_eq!(refused.err(), Some("refused"));
        assert_eq!(check(&doc, &LOCAL), vec![a]);
        assert_eq!(doc.to_snapshot_v2(), before);
    }

    #[test]
    fn reading_copies_share_immutable_boxes_but_diverge_safely_on_each_write() {
        let mut doc = drawing();
        let a = doc.add(wall(0., 0., 0.2, 0.)).unwrap();
        check(&doc, &LOCAL);
        let mut copy = doc.reading_copy();
        assert!(Arc::ptr_eq(
            doc.store.wall_candidates.get().unwrap(),
            copy.store.wall_candidates.get().unwrap()
        ));
        doc.update(a, wall(100., 100., 0.2, 0.));
        copy.update(a, wall(2., 2., 0.4, 0.));
        assert!(check(&doc, &LOCAL).is_empty());
        assert_eq!(check(&copy, &LOCAL), vec![a]);
        assert!(!Arc::ptr_eq(
            doc.store.wall_candidates.get().unwrap(),
            copy.store.wall_candidates.get().unwrap()
        ));
        copy.undo();
        assert_eq!(check(&copy, &LOCAL), vec![a]);
        assert!(check(&doc, &LOCAL).is_empty());
        doc.undo();
        assert_eq!(check(&doc, &LOCAL), vec![a]);
    }

    #[test]
    fn external_changes_and_layer_flags_cannot_leave_stale_or_filtered_candidates() {
        let mut doc = drawing();
        let a = doc.add(wall(0., 0., 0.2, 0.)).unwrap();
        check(&doc, &LOCAL);
        let shared = doc.store.wall_candidates.get().unwrap().clone();
        doc.set_layer_visible("duvar", false);
        doc.set_layer_locked("duvar", true);
        // The legacy join command considers these walls. Cache only boxes, not
        // edit permissions, layer visibility or a precomputed join result.
        assert_eq!(check(&doc, &LOCAL), vec![a]);
        assert!(Arc::ptr_eq(
            &shared,
            doc.store.wall_candidates.get().unwrap()
        ));
        doc.apply_external(External {
            put: vec![(doc.uid(a).unwrap(), wall(100., 100., 0.2, 0.))],
            ..External::default()
        })
        .unwrap();
        assert!(check(&doc, &LOCAL).is_empty());
        doc.apply_external(External {
            put: vec![(doc.uid(a).unwrap(), wall(0., 0., 0.2, 0.))],
            ..External::default()
        })
        .unwrap();
        assert_eq!(check(&doc, &LOCAL), vec![a]);
        doc.apply_external(External {
            remove: vec![doc.uid(a).unwrap()],
            ..External::default()
        })
        .unwrap();
        assert!(check(&doc, &LOCAL).is_empty());
    }

    #[test]
    fn defensive_nonfinite_or_empty_boxes_keep_the_legacy_intersection_policy() {
        let mut doc = drawing();
        let mut walls = Vec::new();
        for mode in 0..5 {
            let mut e = wall(0., 0., 0.2, 0.);
            let Entity::Wall(w) = &mut e else {
                unreachable!()
            };
            match mode {
                0 => w.wall.pts.clear(),
                1 => w.wall.width = f64::INFINITY,
                2 => w.wall.pts[0].x = f64::NAN,
                3 => w.wall.pts[0].y = f64::NEG_INFINITY,
                _ => w.wall.width = -1.,
            }
            walls.push(e);
        }
        doc.add_many(walls, "defensive").unwrap();
        for b in [
            LOCAL[0],
            [
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::INFINITY,
            ],
        ] {
            check(&doc, &[b]);
        }
        assert!(!doc.store.wall_candidates.get().unwrap().unusual.is_empty());
    }
}
