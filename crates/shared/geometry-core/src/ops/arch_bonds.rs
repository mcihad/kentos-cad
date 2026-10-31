//! Architectural bonds (docs/adr/0770 §4): a wall line's or a jamb's lasting relation to another, solved as walls move
//! in plan. A wall's straight edge moves square to itself (its shift `Δ` along its left normal, every line of the edge
//! with it) and an opening along its edge (its shift `δ`), so every rule is linear in the shifts:
//!
//! - **distance** of two parallel lines: `ε·Δ_b − Δ_a = σd − g₀` (`ε = n_a·n_b`, `σ` +1 when `b` is on `a`'s left,
//!   `g₀ = n_a·(P_b − P_a)` now);
//! - **distance** of a jamb from a line crossing its wall: `(n_b·h)·δ_a + (n_b·n_h)·Δ_host − Δ_b = σd − g₀`;
//! - **distance** of two jambs of one edge: `δ_b − δ_a = d − g₀` along the edge;
//! - **symmetric**: `ε_a·Δ_a + ε_b·Δ_b − 2·Δ_axis = −(s_a + s_b)`, `s` along the axis's normal from the axis;
//! - **fix**: the shift is nothing; **parallel**: no equation, the lines must be parallel.
//!
//! The shifts a caller holds (what an edit moved) are nothing. The rest is the least change (smallest `Σx²`) that
//! meets every equation: rows eliminated in the bonds' order, columns in the shifts' order, the first row with a
//! coefficient past [`COEF`] the pivot; a row left with no coefficient and a right side past [`TIE`] is a conflict
//! of the bonds it was made from. `scripts/fixtures/arch_bond_cases.py` writes the same without this code.

use crate::geom::wall::{Spec, boundaries};
use crate::vec2::Vec2;

/// A shift this small is no shift, and a right side this small meets its row, metres.
pub const TIE: f64 = 1e-9;
/// A coefficient this small is none.
pub const COEF: f64 = 1e-9;
/// Two directions this near are parallel: the sine of 0.001° (as Kaydır, docs/adr/0710).
pub const PARALLEL: f64 = 1.745_329_251_994_33e-5;
/// A jamb's bond crosses its wall at 30° at least: the cosine between the line's normal and the wall.
pub const CROSSING: f64 = 0.5;

fn left(u: Vec2) -> Vec2 {
    Vec2::new(-u.y, u.x)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

/// Which line of a wall's edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    LeftFace,
    RightFace,
    Axis,
    LeftCore,
    RightCore,
}

/// The line's distance from the wall's path to the edge's left: the faces are the first and last of the wall's lines,
/// the axis halfway, the core the boundaries of the strongest components' band (docs/adr/0729 §1; the faces when the
/// components do not fit).
pub fn line_offset(spec: &Spec, kind: LineKind) -> f64 {
    let b = boundaries(spec);
    let last = b.len() - 1;
    let core = || {
        if spec.components.len() + 1 != b.len() {
            return [0, last];
        }
        let ranks: Vec<u8> = spec.components.iter().map(|c| c.rank()).collect();
        let Some(&least) = ranks.iter().min() else {
            return [0, last];
        };
        let first = ranks.iter().position(|r| *r == least).unwrap_or(0);
        let end = ranks.iter().rposition(|r| *r == least).unwrap_or(last - 1);
        [first, end + 1]
    };
    match kind {
        LineKind::LeftFace => b[0],
        LineKind::RightFace => b[last],
        LineKind::Axis => (b[0] + b[last]) / 2.0,
        LineKind::LeftCore => b[core()[0]],
        LineKind::RightCore => b[core()[1]],
    }
}

/// A line of an edge: the edge's shift, a point on the line and the edge's direction (unit).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub var: usize,
    pub at: Vec2,
    pub dir: Vec2,
}

/// A jamb: the opening's shift along its edge, its point, its edge's direction (unit) and the edge's shift when the
/// edge is one of the problem's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Jamb {
    pub var: usize,
    pub at: Vec2,
    pub dir: Vec2,
    pub host: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Line(Line),
    Jamb(Jamb),
}

impl Item {
    fn var(&self) -> usize {
        match self {
            Item::Line(l) => l.var,
            Item::Jamb(j) => j.var,
        }
    }
}

/// A bond with its value in metres. A reference distance only measures: no equation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bond {
    Distance {
        a: Item,
        b: Item,
        value: f64,
        left: bool,
    },
    Parallel {
        a: Line,
        b: Line,
    },
    Symmetric {
        a: Line,
        b: Line,
        axis: Line,
    },
    Fix {
        a: Item,
    },
    Reference,
}

/// What a bond measures now: a distance its signed gap (`g` of the module's rules: positive on `a`'s left for two
/// lines, on the line's left for a jamb, along the edge for two jambs); a symmetric its miss (`s_a + s_b`); none for the
/// rest. None when its parts do not fit (not parallel, a jamb's line too near its wall).
pub fn measure(bond: &Bond) -> Option<f64> {
    match *bond {
        Bond::Distance { a, b, .. } => match (a, b) {
            (Item::Line(a), Item::Line(b)) => {
                (cross(a.dir, b.dir).abs() <= PARALLEL).then(|| dot(left(a.dir), sub(b.at, a.at)))
            }
            (Item::Jamb(j), Item::Line(l)) => {
                let n = left(l.dir);
                (dot(n, j.dir).abs() >= CROSSING).then(|| dot(n, sub(j.at, l.at)))
            }
            (Item::Jamb(a), Item::Jamb(b)) => Some(dot(a.dir, sub(b.at, a.at))),
            (Item::Line(_), Item::Jamb(_)) => None,
        },
        Bond::Symmetric { a, b, axis } => {
            let n = left(axis.dir);
            (cross(a.dir, axis.dir).abs() <= PARALLEL && cross(b.dir, axis.dir).abs() <= PARALLEL)
                .then(|| dot(n, sub(a.at, axis.at)) + dot(n, sub(b.at, axis.at)))
        }
        Bond::Parallel { a, b } => (cross(a.dir, b.dir).abs() <= PARALLEL).then_some(0.0),
        Bond::Fix { .. } | Bond::Reference => Some(0.0),
    }
}

/// How far a bond is from holding now (metres); none when its parts do not fit it.
pub fn miss(bond: &Bond) -> Option<f64> {
    let m = measure(bond)?;
    Some(match *bond {
        Bond::Distance { a, b, value, left } => match (a, b) {
            (Item::Jamb(_), Item::Jamb(_)) => m - value,
            _ => m - if left { value } else { -value },
        },
        Bond::Symmetric { .. } => m,
        _ => 0.0,
    })
}

/// Why the bonds do not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// Lines a bond wants parallel are not (a wall turned).
    NotParallel,
    /// A jamb's line no longer crosses its wall at 30° at least.
    Crossing,
    /// Their equations contradict one another (or a held shift).
    Inconsistent,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Every shift (nothing for a held one) and the problem's freedom (its free shifts less its rank).
    Solved { shifts: Vec<f64>, dof: usize },
    /// The bonds (by index) that cannot hold together.
    Conflict { bonds: Vec<usize>, why: Why },
}

/// One equation: its coefficients by shift, its right side and the bonds it was made from.
#[derive(Clone, Debug)]
struct Row {
    coef: Vec<f64>,
    rhs: f64,
    from: Vec<bool>,
}

/// The equations of `bonds` over `vars` shifts (docs/adr/0770 §4), or the bond whose parts do not fit.
fn rows(vars: usize, bonds: &[Bond]) -> Result<Vec<Row>, (usize, Why)> {
    let mut out = Vec::new();
    for (k, bond) in bonds.iter().enumerate() {
        let mut row = Row {
            coef: vec![0.0; vars],
            rhs: 0.0,
            from: vec![false; bonds.len()],
        };
        row.from[k] = true;
        match *bond {
            Bond::Reference => continue,
            Bond::Parallel { a, b } => {
                if cross(a.dir, b.dir).abs() > PARALLEL {
                    return Err((k, Why::NotParallel));
                }
                continue;
            }
            Bond::Fix { a } => row.coef[a.var()] += 1.0,
            Bond::Symmetric { a, b, axis } => {
                if cross(a.dir, axis.dir).abs() > PARALLEL
                    || cross(b.dir, axis.dir).abs() > PARALLEL
                {
                    return Err((k, Why::NotParallel));
                }
                let n = left(axis.dir);
                row.coef[a.var] += dot(n, left(a.dir));
                row.coef[b.var] += dot(n, left(b.dir));
                row.coef[axis.var] -= 2.0;
                row.rhs = -(dot(n, sub(a.at, axis.at)) + dot(n, sub(b.at, axis.at)));
            }
            Bond::Distance {
                a,
                b,
                value,
                left: on_left,
            } => {
                let sigma = if on_left { 1.0 } else { -1.0 };
                match (a, b) {
                    (Item::Line(a), Item::Line(b)) => {
                        if cross(a.dir, b.dir).abs() > PARALLEL {
                            return Err((k, Why::NotParallel));
                        }
                        let n = left(a.dir);
                        row.coef[b.var] += dot(n, left(b.dir));
                        row.coef[a.var] -= 1.0;
                        row.rhs = sigma * value - dot(n, sub(b.at, a.at));
                    }
                    (Item::Jamb(j), Item::Line(l)) => {
                        let n = left(l.dir);
                        if dot(n, j.dir).abs() < CROSSING {
                            return Err((k, Why::Crossing));
                        }
                        row.coef[j.var] += dot(n, j.dir);
                        if let Some(h) = j.host {
                            row.coef[h] += dot(n, left(j.dir));
                        }
                        row.coef[l.var] -= 1.0;
                        row.rhs = sigma * value - dot(n, sub(j.at, l.at));
                    }
                    (Item::Jamb(a), Item::Jamb(b)) => {
                        row.coef[b.var] += dot(a.dir, b.dir);
                        row.coef[a.var] -= 1.0;
                        row.rhs = value - dot(a.dir, sub(b.at, a.at));
                    }
                    (Item::Line(_), Item::Jamb(_)) => return Err((k, Why::Crossing)),
                }
            }
        }
        out.push(row);
    }
    Ok(out)
}

/// Whether `bonds` hold together over `held.len()` shifts, the held ones nothing, without working the shifts out (a
/// refusal's bonds that free it, docs/adr/0780 §1): none, or the conflict [`solve`] finds.
pub fn conflict(held: &[bool], bonds: &[Bond]) -> Option<Outcome> {
    eliminate(held, bonds).err()
}

/// The equations eliminated column by column (the pivot rows), or the conflict.
fn eliminate(held: &[bool], bonds: &[Bond]) -> Result<Vec<Row>, Outcome> {
    let vars = held.len();
    let mut rows = match rows(vars, bonds) {
        Ok(r) => r,
        Err((k, why)) => {
            return Err(Outcome::Conflict {
                bonds: vec![k],
                why,
            });
        }
    };
    for r in &mut rows {
        for (v, h) in held.iter().enumerate() {
            if *h {
                r.coef[v] = 0.0;
            }
        }
    }
    // Eliminate column by column; the first remaining row with a coefficient is the pivot.
    let mut pivots: Vec<Row> = Vec::new();
    for col in 0..vars {
        if held[col] {
            continue;
        }
        let Some(p) = rows.iter().position(|r| r.coef[col].abs() > COEF) else {
            continue;
        };
        let pivot = rows.remove(p);
        for r in &mut rows {
            let f = r.coef[col] / pivot.coef[col];
            if f != 0.0 {
                for (x, y) in r.coef.iter_mut().zip(&pivot.coef) {
                    *x -= f * y;
                }
                r.rhs -= f * pivot.rhs;
                for (x, y) in r.from.iter_mut().zip(&pivot.from) {
                    *x |= *y;
                }
                r.coef[col] = 0.0;
            }
        }
        pivots.push(pivot);
    }
    if let Some(bad) = rows.iter().find(|r| r.rhs.abs() > TIE) {
        return Err(Outcome::Conflict {
            bonds: bad
                .from
                .iter()
                .enumerate()
                .filter_map(|(k, f)| f.then_some(k))
                .collect(),
            why: Why::Inconsistent,
        });
    }
    Ok(pivots)
}

/// Solves `bonds` over `held.len()` shifts, the held ones nothing (docs/adr/0770 §4).
pub fn solve(held: &[bool], bonds: &[Bond]) -> Outcome {
    let vars = held.len();
    let pivots = match eliminate(held, bonds) {
        Ok(p) => p,
        Err(conflict) => return conflict,
    };
    let free = held.iter().filter(|h| !**h).count();
    let rank = pivots.len();
    // The least change: x = Rᵀ·y with (R·Rᵀ)·y = d over the pivot rows (they span the equations' rows).
    let mut m: Vec<Vec<f64>> = (0..rank)
        .map(|i| {
            (0..rank)
                .map(|j| {
                    pivots[i]
                        .coef
                        .iter()
                        .zip(&pivots[j].coef)
                        .map(|(a, b)| a * b)
                        .sum()
                })
                .collect()
        })
        .collect();
    let mut y: Vec<f64> = pivots.iter().map(|r| r.rhs).collect();
    for c in 0..rank {
        let Some(p) = (c..rank).find(|&r| m[r][c].abs() > COEF) else {
            continue;
        };
        m.swap(c, p);
        y.swap(c, p);
        for r in 0..rank {
            if r != c {
                let f = m[r][c] / m[c][c];
                if f != 0.0 {
                    for k in 0..rank {
                        m[r][k] -= f * m[c][k];
                    }
                    y[r] -= f * y[c];
                }
            }
        }
    }
    for c in 0..rank {
        if m[c][c].abs() > COEF {
            y[c] /= m[c][c];
        }
    }
    let mut shifts = vec![0.0; vars];
    for (i, p) in pivots.iter().enumerate() {
        for (v, c) in p.coef.iter().enumerate() {
            shifts[v] += c * y[i];
        }
    }
    for (v, h) in held.iter().enumerate() {
        if *h || shifts[v].abs() <= TIE {
            shifts[v] = 0.0;
        }
    }
    Outcome::Solved {
        shifts,
        dof: free.saturating_sub(rank),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(var: usize, x: f64, y: f64, dx: f64, dy: f64) -> Line {
        Line {
            var,
            at: Vec2::new(x, y),
            dir: Vec2::new(dx, dy),
        }
    }

    #[test]
    fn a_held_wall_drags_the_bonded_one() {
        // Two upright lines 1.2 apart, the second on the first's right (direction north: left is west).
        let a = line(0, 0.5, 0.0, 0.0, 1.0);
        let b = line(1, 1.7, 0.0, 0.0, 1.0);
        let bond = Bond::Distance {
            a: Item::Line(a),
            b: Item::Line(Line {
                at: Vec2::new(2.0, 0.0),
                ..b
            }),
            value: 1.2,
            left: false,
        };
        let Outcome::Solved { shifts, dof } = solve(&[true, false], &[bond]) else {
            panic!()
        };
        // Their normal is west: b 1.5 west of a is a gap of −1.5; held at −1.2 it shifts 0.3 along its normal (west).
        assert!((shifts[1] - 0.3).abs() < 1e-12, "{shifts:?}");
        assert_eq!(dof, 0);
    }

    #[test]
    fn two_values_on_one_pair_conflict() {
        let a = line(0, 0.0, 0.0, 1.0, 0.0);
        let b = line(1, 0.0, 1.0, 1.0, 0.0);
        let d = |value| Bond::Distance {
            a: Item::Line(a),
            b: Item::Line(b),
            value,
            left: true,
        };
        assert_eq!(
            solve(&[true, false], &[d(1.2), d(1.5)]),
            Outcome::Conflict {
                bonds: vec![0, 1],
                why: Why::Inconsistent
            }
        );
        // Asked only whether they hold together (docs/adr/0780 §1): the same conflict; either one alone holds.
        assert_eq!(
            conflict(&[true, false], &[d(1.2), d(1.5)]),
            Some(solve(&[true, false], &[d(1.2), d(1.5)]))
        );
        assert_eq!(conflict(&[true, false], &[d(1.5)]), None);
    }

    #[test]
    fn the_least_change_splits_an_unheld_pair() {
        let a = line(0, 0.0, 0.0, 1.0, 0.0);
        let b = line(1, 0.0, 1.0, 1.0, 0.0);
        let bond = Bond::Distance {
            a: Item::Line(a),
            b: Item::Line(b),
            value: 2.0,
            left: true,
        };
        let Outcome::Solved { shifts, dof } = solve(&[false, false], &[bond]) else {
            panic!()
        };
        assert!(
            (shifts[0] + 0.5).abs() < 1e-12 && (shifts[1] - 0.5).abs() < 1e-12,
            "{shifts:?}"
        );
        assert_eq!(dof, 1);
    }
}
