//! The written file's bodies, measured for the model quality control form (feature `geometry`; EK-9 Tablo 9.1/13–16,
//! docs/adr/8074 §2): each body's lowest and highest point and its box in plan, with its float32 bound
//! ([`Body::rounding`]); each space's
//! footprint, whether it is closed, and the spaces of one storey that share ground, with how much. Measured on the
//! engine's triangles, which are display, not measure (docs/adr/8061 §5). A space is closed when its every edge two
//! triangles share at corners written alike; the other bodies are not asked: a body the engine cut its openings from
//! leaves seams its triangles do not share (the section building's walls and slabs), which says nothing of the file.
//!
//! What is a finding is the rule's (`dijital_proje.form`): two spaces overlap here when their footprints share more
//! than the geometry core's overlay gives as zero.

use std::collections::{BTreeMap, HashMap};

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region::{intersect_area_sets, net_area, union_areas};
use serde_json::{Value as Json, json};

use super::Model;
use super::bodies::{Bodies, Body};
use super::header;

/// A written file's facts with its bodies' under `bodies` ([`Model::facts`] and [`Model::body_facts`]): what the model
/// quality control form reads (docs/adr/8074 §2), for a file `name` read within `limits`; `cancel` is asked between
/// records and bodies.
pub fn file_facts(
    bytes: &[u8],
    name: &str,
    limits: &super::Limits,
    cancel: &dyn Fn() -> bool,
) -> Result<Json, super::ReadError> {
    let model = super::read(bytes, limits, cancel)?;
    let mut facts = model.facts(bytes, name)?;
    let bodies = model.bodies(bytes, cancel)?;
    facts["bodies"] = model.body_facts(&bodies);
    Ok(facts)
}

impl Model {
    /// The bodies' facts: `{"bodies": [...], "spaces": [...], "overlaps": [...], "notShown": [...]}`, each object by its
    /// GlobalId; the objects with a shape whose body could not be made with why (the form's 7th question cannot measure
    /// them, docs/adr/8133).
    pub fn body_facts(&self, bodies: &Bodies) -> Json {
        let schema = header::version(self.summary.family);
        let mut out_bodies = Vec::new();
        let mut spaces: Vec<(usize, Option<String>, Vec<Area>)> = Vec::new();
        let mut space_json = Vec::new();
        let mut zone_json = Vec::new();
        for b in &bodies.bodies {
            let o = &self.objects[b.object];
            let (lo, hi) = heights(b);
            out_bodies.push(json!({ "globalId": o.global_id, "zMin": lo, "zMax": hi, "box": plan_box(b), "rounding": b.rounding }));
            if schema.is_subtype_of(&o.class, "IfcSpace") {
                let closed = closed(b);
                let storey = self
                    .storey_of_object(b.object)
                    .map(|s| self.objects[s].global_id.clone());
                let ground = footprint(b, lo);
                let area: f64 = union_areas(&ground).iter().map(net_area).sum();
                space_json.push(json!({
                    "globalId": o.global_id,
                    "storey": storey,
                    "area": area,
                    "zMin": lo,
                    "zMax": hi,
                    "rounding": b.rounding,
                    "closed": closed,
                }));
                spaces.push((b.object, storey, ground));
            }
            // A zone's footprint and its storey (docs/adr/8125; EK-9/10): IFC 4.3 gives a zone no area quantity, so its
            // area is its body's footprint, within twice its outline's length times its float32 bound.
            if schema.is_subtype_of(&o.class, "IfcSpatialZone") {
                let storey = self
                    .storey_of_object(b.object)
                    .map(|s| self.objects[s].global_id.clone());
                let ground = union_areas(&footprint(b, lo));
                let area: f64 = ground.iter().map(net_area).sum();
                let outline: f64 = ground
                    .iter()
                    .flat_map(|a| std::iter::once(&a.outer).chain(&a.holes))
                    .map(ring_length)
                    .sum();
                zone_json.push(json!({
                    "globalId": o.global_id,
                    "storey": storey,
                    "area": area,
                    "areaBound": 2.0 * outline * b.rounding,
                }));
            }
        }
        let not_shown: Vec<Json> = bodies
            .not_shown
            .iter()
            .map(|n| json!({ "globalId": self.objects[n.object].global_id, "reason": n.reason }))
            .collect();
        json!({ "bodies": out_bodies, "spaces": space_json, "zones": zone_json, "overlaps": overlaps(self, &spaces), "notShown": not_shown })
    }

    /// The storey above object `i`: through its whole, its container, else the structure it is referenced in (a zone,
    /// docs/adr/8125).
    fn storey_of_object(&self, i: usize) -> Option<usize> {
        let schema = header::version(self.summary.family);
        let mut at = self
            .objects
            .get(i)
            .and_then(|o| o.parent.or(o.container).or(o.reference));
        for _ in 0..self.objects.len() {
            let p = at?;
            let o = self.objects.get(p)?;
            if schema.is_subtype_of(&o.class, "IfcBuildingStorey") {
                return Some(p);
            }
            at = o.parent.or(o.container);
        }
        None
    }
}

/// A ring's length, its last corner back to its first.
fn ring_length(r: &Ring) -> f64 {
    let n = r.pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (r.pts[i], r.pts[(i + 1) % n]);
            crate::math::hypot(b.x - a.x, b.y - a.y)
        })
        .sum()
}

/// A body's lowest and highest corner.
fn heights(b: &Body) -> (f64, f64) {
    b.positions
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p[2]), hi.max(p[2]))
        })
}

/// A body's box in plan: `[min east, min north, max east, max north]`.
fn plan_box(b: &Body) -> [f64; 4] {
    b.positions.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |m, p| {
            [
                m[0].min(p[0]),
                m[1].min(p[1]),
                m[2].max(p[0]),
                m[3].max(p[1]),
            ]
        },
    )
}

/// Whether every edge of the body is two triangles' (an edge as its two corners, corners written alike the same).
fn closed(b: &Body) -> bool {
    let key = |i: u32| b.positions.get(i as usize).map(|p| p.map(f64::to_bits));
    let mut edges: HashMap<([u64; 3], [u64; 3]), u32> = HashMap::new();
    for t in &b.triangles {
        for (a, c) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (Some(p), Some(q)) = (key(a), key(c)) else {
                return false;
            };
            if p == q {
                continue;
            }
            let e = if p < q { (p, q) } else { (q, p) };
            *edges.entry(e).or_default() += 1;
        }
    }
    !edges.is_empty() && edges.values().all(|&n| n == 2)
}

/// The body's footprint: its triangles lying at its lowest height (within its float32 bound), each a triangle in plan.
fn footprint(b: &Body, lowest: f64) -> Vec<Area> {
    let bound = 2.0 * b.rounding;
    b.triangles
        .iter()
        .filter_map(|t| {
            let p = t.map(|i| b.positions.get(i as usize).copied());
            let [Some(a), Some(c), Some(d)] = p else {
                return None;
            };
            if [a, c, d].iter().any(|q| (q[2] - lowest).abs() > bound) {
                return None;
            }
            let ring = Ring {
                pts: vec![
                    Vec2::new(a[0], a[1]),
                    Vec2::new(c[0], c[1]),
                    Vec2::new(d[0], d[1]),
                ],
                bulges: None,
            };
            Some(Area {
                outer: ring,
                holes: Vec::new(),
            })
        })
        .collect()
}

/// The spaces of one storey whose footprints share ground: the pair by GlobalId and the area shared.
fn overlaps(model: &Model, spaces: &[(usize, Option<String>, Vec<Area>)]) -> Vec<Json> {
    let boxes: Vec<Option<[f64; 4]>> = spaces
        .iter()
        .map(|(_, _, ground)| {
            ground
                .iter()
                .flat_map(|a| a.outer.pts.iter())
                .fold(None, |b: Option<[f64; 4]>, p| {
                    Some(b.map_or([p.x, p.y, p.x, p.y], |b| {
                        [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]
                    }))
                })
        })
        .collect();
    let mut by_storey: BTreeMap<Option<&String>, Vec<usize>> = BTreeMap::new();
    for (k, (_, storey, _)) in spaces.iter().enumerate() {
        by_storey.entry(storey.as_ref()).or_default().push(k);
    }
    let mut out = Vec::new();
    for members in by_storey.values() {
        for (n, &i) in members.iter().enumerate() {
            for &j in &members[n + 1..] {
                let (Some(p), Some(q)) = (boxes[i], boxes[j]) else {
                    continue;
                };
                if p[2] <= q[0] || q[2] <= p[0] || p[3] <= q[1] || q[3] <= p[1] {
                    continue;
                }
                let shared = intersect_area_sets(&[spaces[i].2.clone(), spaces[j].2.clone()]);
                let area: f64 = shared.iter().map(net_area).sum();
                if area > 0.0 {
                    out.push(json!({
                        "a": model.objects[spaces[i].0].global_id,
                        "b": model.objects[spaces[j].0].global_id,
                        "area": area,
                    }));
                }
            }
        }
    }
    out
}
