//! Read-only world-X/Y grids, using observed intervals and the existing adoption validator.
use super::*;

// Preserve room in the shared work budget to actually inspect proposals. Reaching either
// discovery bound is explicitly partial, never a claim that every pattern was examined.
const PAIR_WORK: u64 = WORK_MAX / 4;
const AXIS_STEPS: usize = 128;

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
    // Subtract the seed before comparing or dividing: no world-space FP32 or relative epsilon.
    let local: Vec<_> = points
        .iter()
        .map(|p| Point {
            x: p.x - seed.x,
            y: p.y - seed.y,
            ..*p
        })
        .collect();
    let mut xs = BTreeMap::<u64, f64>::new();
    let mut ys = BTreeMap::<u64, f64>::new();
    'pairs: for (k, a) in local.iter().enumerate() {
        for b in &local[k + 1..] {
            if budget.work >= PAIR_WORK {
                budget.cut = true;
                break 'pairs;
            }
            if !budget.tick(stopped)? {
                break 'pairs;
            }
            let x = (b.x - a.x).abs();
            let y = (b.y - a.y).abs();
            if x.is_finite() && x > tolerance * 2. && x > 1e-9 && y <= tolerance {
                xs.insert(x.to_bits(), x);
            }
            if y.is_finite() && y > tolerance * 2. && y > 1e-9 && x <= tolerance {
                ys.insert(y.to_bits(), y);
            }
            if xs.len() > AXIS_STEPS || ys.len() > AXIS_STEPS {
                budget.cut = true;
                break 'pairs;
            }
        }
    }
    // Positive finite float bit order is numeric order; prefer the smallest observed spacing.
    let mut matches = Vec::new();
    let mut seen = HashSet::new();
    'proposals: for &dx in xs.values() {
        for &dy in ys.values() {
            let mut places = BTreeMap::<(i32, i32), EntityId>::new();
            let mut collision = false;
            for p in &local {
                if !budget.tick(stopped)? {
                    break 'proposals;
                }
                let col = (p.x / dx).round();
                let row = (p.y / dy).round();
                if !(col.is_finite() && row.is_finite()) || col.abs() > 9999. || row.abs() > 9999. {
                    continue;
                }
                if (p.x - col * dx).hypot(p.y - row * dy) > tolerance {
                    continue;
                }
                if places.insert((row as i32, col as i32), p.uid).is_some() {
                    collision = true;
                    break;
                }
            }
            if collision || places.len() < 4 || !places.values().any(|u| *u == seed.uid) {
                continue;
            }
            let min_row = places.keys().map(|(r, _)| *r).min();
            let max_row = places.keys().map(|(r, _)| *r).max();
            let min_col = places.keys().map(|(_, c)| *c).min();
            let max_col = places.keys().map(|(_, c)| *c).max();
            let (Some(r0), Some(r1), Some(c0), Some(c1)) = (min_row, max_row, min_col, max_col)
            else {
                continue;
            };
            if r0 == r1 || c0 == c1 {
                continue;
            }
            let (rows, cols) = ((r1 - r0 + 1) as u32, (c1 - c0 + 1) as u32);
            if u64::from(rows) * u64::from(cols) > 10_000 {
                continue;
            }
            // Actual neighbours in the same row/column are required; never divide a gap to
            // invent unobserved empty rows or columns.
            let across = places
                .keys()
                .any(|&(r, c)| places.contains_key(&(r, c + 1)));
            let along = places
                .keys()
                .any(|&(r, c)| places.contains_key(&(r + 1, c)));
            if !across || !along {
                continue;
            }
            // AssocArray's first item is the source insert. An empty origin is not silently
            // filled, and the layout is not shifted: use an occupied bounding corner or omit.
            let corner = [
                (r0, c0, 1, 1),
                (r0, c1, 1, -1),
                (r1, c0, -1, 1),
                (r1, c1, -1, -1),
            ]
            .into_iter()
            .find(|&(r, c, _, _)| places.contains_key(&(r, c)));
            let Some((r, c, row_sign, col_sign)) = corner else {
                continue;
            };
            let Some(&first) = places.get(&(r, c)) else {
                continue;
            };
            let mut items = Vec::with_capacity((rows * cols) as usize);
            for i in 0..rows {
                for j in 0..cols {
                    if !budget.tick(stopped)? {
                        break 'proposals;
                    }
                    items.push(ArrayItem {
                        uid: places
                            .get(&(r + i as i32 * row_sign, c + j as i32 * col_sign))
                            .copied(),
                        kept: false,
                    });
                }
            }
            let key: Vec<_> = items.iter().map(|i| i.uid).collect();
            if !seen.insert(key) {
                continue;
            }
            let adoption = ArraysAdopt {
                source: first.to_text(),
                layout: ArrayLayout::Grid {
                    rows,
                    cols,
                    dx: dx * f64::from(col_sign),
                    dy: dy * f64::from(row_sign),
                },
                items,
                tolerance: Some(tolerance),
                expected_revision: Some(doc.revision().to_string()),
            };
            let plan = match arrays_adopt::inspect(doc, &adoption, &mut *stopped) {
                CommandResult::Completed { output, .. } => output,
                CommandResult::Cancelled => return Err(Stop::Cancelled),
                _ => continue,
            };
            matches.push(ArrayPattern {
                present: places.len() as u32,
                missing: plan.missing,
                deviation: plan.deviation,
                adoption,
                overlaps: false,
            });
            if matches.len() > limit {
                budget.cut = true;
                break 'proposals;
            }
        }
    }
    Ok(matches)
}
