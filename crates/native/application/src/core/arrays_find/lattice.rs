//! Observed, independently directed 2D grid axes. The existing adoption gate decides acceptance.
use super::*;

const PAIRS_MAX: usize = 32_768;
const VECTORS_MAX: usize = 96;
type Vector = (f64, f64);
struct Ranked {
    key: Vec<EntityId>,
    pattern: ArrayPattern,
}
struct Proposal {
    first: EntityId,
    rows: u32,
    cols: u32,
    items: Vec<ArrayItem>,
    column: Vector,
    row: Vector,
}
fn layout(p: &Proposal, column: Vector, row: Vector) -> ArrayLayout {
    ArrayLayout::Lattice {
        rows: p.rows,
        cols: p.cols,
        dx: column.0.hypot(column.1),
        dy: row.0.hypot(row.1),
        column_angle: column.1.atan2(column.0).to_degrees(),
        row_angle: row.1.atan2(row.0).to_degrees(),
    }
}
#[allow(clippy::too_many_arguments)]
fn rectangle(
    points: &[Point],
    seed: Point,
    column: Vector,
    row: Vector,
    tolerance: f64,
    span_refinement: bool,
    budget: &mut Budget,
    stopped: &mut impl FnMut() -> bool,
) -> Result<Option<Proposal>, Stop> {
    let det = column.0 * row.1 - column.1 * row.0;
    if !det.is_finite() || det.abs() / column.0.hypot(column.1) / row.0.hypot(row.1) < 1e-6 {
        return Ok(None);
    }
    let mut places = BTreeMap::new();
    for p in points {
        if !budget.tick(stopped)? {
            return Ok(None);
        }
        let c = ((p.x * row.1 - p.y * row.0) / det).round();
        let r = ((column.0 * p.y - column.1 * p.x) / det).round();
        if !c.is_finite() || !r.is_finite() || c.abs() > 9999. || r.abs() > 9999. {
            continue;
        }
        // Candidate generation may accumulate uncertainty from an observed short interval.
        // Only the unchanged absolute tolerance in review_layouts can accept a placement.
        let bound = if span_refinement {
            tolerance * (1. + c.abs() + r.abs())
        } else {
            tolerance
        };
        if (p.x - c * column.0 - r * row.0).hypot(p.y - c * column.1 - r * row.1) > bound {
            continue;
        }
        if places.insert((r as i32, c as i32), p.uid).is_some() {
            return Ok(None);
        }
    }
    if places.len() < 4 || !places.values().any(|u| *u == seed.uid) {
        return Ok(None);
    }
    let r0 = places.keys().map(|p| p.0).min().unwrap_or(0);
    let r1 = places.keys().map(|p| p.0).max().unwrap_or(0);
    let c0 = places.keys().map(|p| p.1).min().unwrap_or(0);
    let c1 = places.keys().map(|p| p.1).max().unwrap_or(0);
    let rows = (r1 - r0 + 1) as u32;
    let cols = (c1 - c0 + 1) as u32;
    if rows < 2 || cols < 2 || u64::from(rows) * u64::from(cols) > 10_000 {
        return Ok(None);
    }
    // Never invent half-steps: each axis requires a genuinely occupied adjacent pair.
    if !places
        .keys()
        .any(|&(r, c)| places.contains_key(&(r, c + 1)))
        || !places
            .keys()
            .any(|&(r, c)| places.contains_key(&(r + 1, c)))
    {
        return Ok(None);
    }
    let corners = [
        (r0, c0, 1, 1),
        (r0, c1, 1, -1),
        (r1, c0, -1, 1),
        (r1, c1, -1, -1),
    ];
    let corner = corners
        .iter()
        .find(|&&(r, c, _, _)| places.get(&(r, c)) == Some(&seed.uid))
        .or_else(|| {
            corners
                .iter()
                .find(|&&(r, c, _, _)| places.contains_key(&(r, c)))
        });
    let Some(&(r, c, rs, cs)) = corner else {
        return Ok(None);
    };
    let first = places[&(r, c)];
    let mut items = Vec::with_capacity((rows * cols) as usize);
    for i in 0..rows {
        for j in 0..cols {
            if !budget.tick(stopped)? {
                return Ok(None);
            }
            items.push(ArrayItem {
                uid: places.get(&(r + i as i32 * rs, c + j as i32 * cs)).copied(),
                kept: false,
            });
        }
    }
    Ok(Some(Proposal {
        first,
        rows,
        cols,
        items,
        column: (column.0 * f64::from(cs), column.1 * f64::from(cs)),
        row: (row.0 * f64::from(rs), row.1 * f64::from(rs)),
    }))
}
// Fit the two displacements from the entire observed span, relative to the actual first insert.
// Rounding one short WCS interval must not accumulate across all columns or rows.
fn refined(
    p: &Proposal,
    points: &BTreeMap<EntityId, Point>,
    budget: &mut Budget,
    stopped: &mut impl FnMut() -> bool,
) -> Result<Option<ArrayLayout>, Stop> {
    let Some(first) = points.get(&p.first) else {
        return Ok(None);
    };
    let (mut cc, mut rr, mut cr, mut xc, mut xr, mut yc, mut yr) = (0., 0., 0., 0., 0., 0., 0.);
    for (k, item) in p.items.iter().enumerate() {
        if !budget.tick(stopped)? {
            return Ok(None);
        }
        let Some(at) = item.uid.and_then(|u| points.get(&u)) else {
            continue;
        };
        let c = (k as u32 % p.cols) as f64;
        let r = (k as u32 / p.cols) as f64;
        let x = at.x - first.x;
        let y = at.y - first.y;
        cc += c * c;
        rr += r * r;
        cr += c * r;
        xc += x * c;
        xr += x * r;
        yc += y * c;
        yr += y * r;
    }
    let det = cc * rr - cr * cr;
    if !det.is_finite() || det <= 0. {
        return Ok(None);
    }
    let column = ((xc * rr - xr * cr) / det, (yc * rr - yr * cr) / det);
    let row = ((xr * cc - xc * cr) / det, (yr * cc - yc * cr) / det);
    Ok(Some(layout(p, column, row)))
}
fn key(pattern: &ArrayPattern) -> Vec<EntityId> {
    let mut ids: Vec<_> = pattern
        .adoption
        .items
        .iter()
        .filter_map(|i| i.uid)
        .collect();
    ids.sort_unstable();
    ids
}
fn order(a: &ArrayPattern, b: &ArrayPattern) -> std::cmp::Ordering {
    b.present
        .cmp(&a.present)
        .then(a.missing.cmp(&b.missing))
        .then(a.deviation.total_cmp(&b.deviation))
}

#[allow(clippy::too_many_arguments)]
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
    let local: Vec<_> = points
        .iter()
        .map(|p| Point {
            x: p.x - seed.x,
            y: p.y - seed.y,
            ..*p
        })
        .collect();
    let by_uid: BTreeMap<_, _> = points.iter().map(|p| (p.uid, *p)).collect();
    let mut sorted = local.clone();
    sorted.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.uid.cmp(&b.uid))
    });
    let mut deltas = Vec::new();
    'pairs: for (k, a) in sorted.iter().enumerate() {
        for b in &sorted[k + 1..] {
            if !budget.tick(stopped)? {
                break 'pairs;
            }
            if deltas.len() >= PAIRS_MAX {
                budget.cut = true;
                break 'pairs;
            }
            let (mut x, mut y) = (b.x - a.x, b.y - a.y);
            if x < -tolerance || (x.abs() <= tolerance && y < 0.) {
                x = -x;
                y = -y;
            }
            let length = x.hypot(y);
            if length.is_finite() && length > tolerance * 2. && length > 1e-9 {
                deltas.push((x, y));
            }
        }
    }
    deltas.sort_by(|a, b| {
        a.0.hypot(a.1)
            .total_cmp(&b.0.hypot(b.1))
            .then(a.0.total_cmp(&b.0))
            .then(a.1.total_cmp(&b.1))
    });
    let mut vectors: Vec<Vector> = Vec::new();
    'vectors: for d in deltas {
        for v in &vectors {
            if !budget.tick(stopped)? {
                break 'vectors;
            }
            if (v.0 - d.0).hypot(v.1 - d.1) <= tolerance * 2. {
                continue 'vectors;
            }
        }
        if vectors.len() >= VECTORS_MAX {
            budget.cut = true;
            break;
        }
        vectors.push(d);
    }
    let mut matches: Vec<Ranked> = Vec::new();
    'axes: for (i, &column) in vectors.iter().enumerate() {
        for &row in &vectors[i + 1..] {
            if !budget.tick(stopped)? {
                break 'axes;
            }
            for span_refinement in [false, true] {
                let Some(proposal) = rectangle(
                    &local,
                    seed,
                    column,
                    row,
                    tolerance,
                    span_refinement,
                    budget,
                    stopped,
                )?
                else {
                    continue;
                };
                let raw = layout(&proposal, proposal.column, proposal.row);
                let fit =
                    refined(&proposal, &by_uid, budget, stopped)?.unwrap_or_else(|| raw.clone());
                let Some((adoption, plan)) = review_layouts(
                    doc,
                    proposal.first,
                    &proposal.items,
                    [fit, raw],
                    tolerance,
                    budget,
                    stopped,
                )?
                else {
                    continue;
                };
                let candidate = ArrayPattern {
                    present: proposal.items.iter().filter(|i| i.uid.is_some()).count() as u32,
                    missing: plan.missing,
                    deviation: plan.deviation,
                    adoption,
                    overlaps: false,
                };
                let ids = key(&candidate);
                if let Some(at) = matches.iter().position(|p| p.key == ids) {
                    if order(&candidate, &matches[at].pattern).is_lt() {
                        matches[at].pattern = candidate;
                    }
                } else {
                    matches.push(Ranked {
                        key: ids,
                        pattern: candidate,
                    });
                }
                matches.sort_by(|a, b| order(&a.pattern, &b.pattern));
                if matches.len() > limit {
                    matches.pop();
                    budget.cut = true;
                }
            }
        }
    }
    Ok(matches.into_iter().map(|p| p.pattern).collect())
}
