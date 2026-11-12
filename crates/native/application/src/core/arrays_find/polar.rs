//! Full-circle hypotheses only. Native associative placements and adoption decide every member.
use super::*;
use kentos_commands::Vec2 as ContractPoint;
use kentos_geometry_core::{
    Vec2,
    geom::delaunay::circumcenter,
    ops::assoc_array::{Layout, Placement, item_placements},
};
use std::{collections::HashMap, f64::consts::TAU};

struct Search<'a, 'b, F> {
    doc: &'a Document,
    points: &'a [Point],
    seed: Point,
    source: Placement,
    tolerance: f64,
    limit: usize,
    budget: &'b mut Budget,
    stopped: &'b mut F,
    centers: HashSet<(u64, u64)>,
    accepted: HashMap<(bool, Vec<Option<EntityId>>), usize>,
    matches: Vec<ArrayPattern>,
}
impl<F: FnMut() -> bool> Search<'_, '_, F> {
    /// False means the explicitly bounded search must stop, preserving partial candidates.
    fn center(&mut self, center: Vec2) -> Result<bool, Stop> {
        if !self.budget.tick(self.stopped)? {
            return Ok(false);
        }
        if !center.x.is_finite()
            || !center.y.is_finite()
            || !self
                .centers
                .insert((center.x.to_bits(), center.y.to_bits()))
        {
            return Ok(true);
        }
        let radius = (self.seed.x - center.x).hypot(self.seed.y - center.y);
        if !radius.is_finite() || radius <= self.tolerance * 2. {
            return Ok(true);
        }
        let start = (self.seed.y - center.y).atan2(self.seed.x - center.x);
        let mut ring = Vec::new();
        for p in self.points {
            if !self.budget.tick(self.stopped)? {
                return Ok(false);
            }
            let r = (p.x - center.x).hypot(p.y - center.y);
            if (r - radius).abs() <= self.tolerance {
                let angle = ((p.y - center.y).atan2(p.x - center.x) - start).rem_euclid(TAU);
                ring.push((angle, *p));
            }
        }
        if ring.len() < 4 {
            return Ok(true);
        }
        ring.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.uid.cmp(&b.1.uid)));
        let mut counts = BTreeSet::new();
        for k in 0..ring.len() {
            if !self.budget.tick(self.stopped)? {
                return Ok(false);
            }
            let next = if k + 1 == ring.len() {
                ring[0].0 + TAU
            } else {
                ring[k + 1].0
            };
            let gap = next - ring[k].0;
            let n = (TAU / gap).round();
            if gap > 0. && (4. ..=1000.).contains(&n) {
                counts.insert(n as u32);
            }
        }
        for count in counts {
            for rotate in [false, true] {
                for _ in 0..count {
                    if !self.budget.tick(self.stopped)? {
                        return Ok(false);
                    }
                }
                let Some(poses) = item_placements(
                    self.source,
                    &Layout::Polar {
                        center,
                        count,
                        fill: 360.,
                        rotate,
                    },
                ) else {
                    continue;
                };
                if poses
                    .iter()
                    .any(|p| !p.p.x.is_finite() || !p.p.y.is_finite())
                {
                    continue;
                }
                let mut items = vec![
                    ArrayItem {
                        uid: None,
                        kept: false
                    };
                    count as usize
                ];
                let mut collision = false;
                for &(angle, p) in &ring {
                    if !self.budget.tick(self.stopped)? {
                        return Ok(false);
                    }
                    let slot = (angle / TAU * f64::from(count)).round() as usize % count as usize;
                    let pose = &poses[slot];
                    if (p.x - pose.p.x).hypot(p.y - pose.p.y) > self.tolerance {
                        continue;
                    }
                    let Some(Entity::Insert(actual)) = self
                        .doc
                        .slot_of(Uuid::from_bytes(p.uid.0))
                        .and_then(|s| self.doc.get(s))
                    else {
                        continue;
                    };
                    let turn = (actual.rotation - pose.rotation).rem_euclid(TAU);
                    if turn.min(TAU - turn) > 1e-10 {
                        continue;
                    }
                    if items[slot].uid.is_some() {
                        collision = true;
                        break;
                    }
                    items[slot].uid = Some(p.uid);
                }
                let present = items.iter().filter(|i| i.uid.is_some()).count();
                if collision
                    || present < 4
                    || items[0].uid != Some(self.seed.uid)
                    || !(0..items.len()).any(|k| {
                        items[k].uid.is_some() && items[(k + 1) % items.len()].uid.is_some()
                    })
                {
                    continue;
                }
                let key = (rotate, items.iter().map(|i| i.uid).collect::<Vec<_>>());
                let layout = ArrayLayout::Polar {
                    center: ContractPoint {
                        x: center.x,
                        y: center.y,
                    },
                    count,
                    fill: 360.,
                    rotate,
                };
                let Some((adoption, plan)) = review_layouts(
                    self.doc,
                    self.seed.uid,
                    &items,
                    [layout],
                    self.tolerance,
                    self.budget,
                    self.stopped,
                )?
                else {
                    continue;
                };
                let candidate = ArrayPattern {
                    adoption,
                    present: present as u32,
                    missing: plan.missing,
                    deviation: plan.deviation,
                    overlaps: false,
                };
                if let Some(&at) = self.accepted.get(&key) {
                    // A second measured centre can explain the same members more accurately.
                    if candidate.deviation < self.matches[at].deviation {
                        self.matches[at] = candidate;
                    }
                } else {
                    self.accepted.insert(key, self.matches.len());
                    self.matches.push(candidate);
                    if self.matches.len() > self.limit {
                        self.budget.cut = true;
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }
}

pub(super) fn find(
    doc: &Document,
    points: &[Point],
    seed: Point,
    tolerance: f64,
    limit: usize,
    budget: &mut Budget,
    stopped: &mut impl FnMut() -> bool,
) -> Result<Vec<ArrayPattern>, Stop> {
    if points.len() < 4 {
        return Ok(Vec::new());
    }
    let insert = arrays_adopt::prototype(doc, Uuid::from_bytes(seed.uid.0), "source")?;
    let source = Placement {
        p: Vec2::new(seed.x, seed.y),
        scale: insert.scale,
        rotation: insert.rotation,
        mirror: insert.mirror,
    };
    let mut search = Search {
        doc,
        points,
        seed,
        source,
        tolerance,
        limit,
        budget,
        stopped,
        centers: HashSet::new(),
        accepted: HashMap::new(),
        matches: Vec::new(),
    };
    'pairs: for (k, a) in points.iter().enumerate() {
        for b in &points[k + 1..] {
            if !search.budget.tick(search.stopped)? {
                break 'pairs;
            }
            // Diametric midpoints improve even-circle centres without squaring world coordinates.
            let mid = Vec2::new(a.x + (b.x - a.x) / 2., a.y + (b.y - a.y) / 2.);
            if !search.center(mid)? {
                break 'pairs;
            }
            let (ax, ay, bx, by) = (a.x - seed.x, a.y - seed.y, b.x - seed.x, b.y - seed.y);
            let scale = ax.abs().max(ay.abs()).max(bx.abs()).max(by.abs());
            if !scale.is_finite() || scale == 0. {
                continue;
            }
            let local = circumcenter(
                Vec2::new(0., 0.),
                Vec2::new(ax / scale, ay / scale),
                Vec2::new(bx / scale, by / scale),
            );
            let center = Vec2::new(seed.x + local.x * scale, seed.y + local.y * scale);
            if !search.center(center)? {
                break 'pairs;
            }
        }
    }
    Ok(search.matches)
}
