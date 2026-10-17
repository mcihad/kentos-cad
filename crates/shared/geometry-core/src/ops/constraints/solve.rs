//! The solve itself (docs/adr/0334 §4): least-change Gauss–Newton over each independent group of equations.
//!
//! Each step builds the sparse Jacobian `J` of the group's rows at `x`, weighs the unknowns (`w`: one for lengths, an
//! arc's radius for its angles), and factors `J·W⁻¹`'s Gram matrix in the constraints' order (`sparse`): a row in the
//! span of the ones before is dependent, redundant while it holds and conflicting when it does not. The step is the
//! least-norm solution of the kept rows, `dz = J_Iᵀ·(J_I·J_Iᵀ)⁻¹·(−r_I)`, `dx = W⁻¹·dz`, halved while it does not
//! lower the sum of squares. The rank found at the end gives the freedoms left (`dof`) and which objects can still
//! move (an unknown no allowed motion moves is fixed).

use std::collections::{BTreeSet, HashMap};

use crate::entity::Shape;

use super::dual::D;
use super::model::{Built, Model, VarKind, build, write_back};
use super::rules::{measured, rows};
use super::sparse::{Factor, Row};
use super::{Element, Outcome, PointAt, Problem, Ref, Rule, Status};

/// The most steps one group takes.
const STEPS: u32 = 64;

/// How far a generic motion the rows allow moves an unknown for it to count as free.
const FIXED: f64 = 1e-8;

/// One constraint's equations: the unknowns its parts are built from (each part's slice), and its rule.
struct Eq {
    constraint: usize,
    rule: Rule,
    value: f64,
    /// Every unknown it reads, each once, in first-seen order: the dual numbers' slots.
    vars: Vec<usize>,
    /// Per part: its unknowns as positions in `vars`.
    parts: Vec<Vec<usize>>,
    /// A fix: its parts' place to hold, as built from the fix shapes.
    at: Vec<Built>,
    /// A tangent at an end (docs/adr/0335 §2): the arc's end that a coincident ties to the segment's end, built
    /// after the rule's own parts.
    end: Option<Ref>,
}

fn element_of(r: &Ref) -> &Element {
    &r.element
}

/// The residuals of `eq` at `x`, with derivatives by its slots when `exact`; unknowns `free` says are not are constant.
fn eval(eq: &Eq, shapes: &[Shape], x: &[f64], free: &[bool], s: f64) -> Vec<D> {
    let duals: Vec<D> = eq
        .vars
        .iter()
        .enumerate()
        .map(|(slot, &j)| {
            if free[j] {
                D::var(x[j], slot)
            } else {
                D::c(x[j])
            }
        })
        .collect();
    let refs: Vec<Ref> = eq.rule.refs().into_iter().chain(eq.end).collect();
    let parts: Vec<Built> = refs
        .iter()
        .zip(&eq.parts)
        .map(|(r, slots)| {
            let v: Vec<D> = slots.iter().map(|&k| duals[k]).collect();
            build(&shapes[r.object], element_of(r), &v)
        })
        .collect();
    rows(&eq.rule, &parts, eq.value, &eq.at, s)
}

/// Builds the parts of `rule` in `shapes` as constants (for a fix's place and a measure); none when a part is missing.
fn built_parts(model: &Model, shapes: &[Shape], x: &[f64], rule: &Rule) -> Option<Vec<Built>> {
    rule.refs()
        .iter()
        .map(|r| {
            let vars = model.element_vars(shapes, r.object, element_of(r))?;
            let v: Vec<D> = vars.iter().map(|&j| D::c(x[j])).collect();
            Some(build(&shapes[r.object], element_of(r), &v))
        })
        .collect()
}

pub(super) fn measure(shapes: &[Shape], rule: &Rule) -> Option<f64> {
    let mut used = vec![false; shapes.len()];
    for r in rule.refs() {
        *used.get_mut(r.object)? = true;
    }
    let model = Model::new(shapes, &used);
    let parts = built_parts(&model, shapes, &model.x, rule)?;
    measured(rule, &parts)
}

/// The ends of a segment as points of its object: a line's start and end, a path edge's two vertices.
fn segment_ends(shape: &Shape, element: &Element) -> Vec<PointAt> {
    match (shape, element) {
        (Shape::Line { .. }, Element::Segment { .. }) => vec![PointAt::Start, PointAt::End],
        (
            Shape::Polygon { .. } | Shape::Polyline { .. },
            Element::Segment { part, ring, index },
        ) => vec![
            PointAt::Vertex {
                part: *part,
                ring: *ring,
                index: *index,
            },
            PointAt::Vertex {
                part: *part,
                ring: *ring,
                index: index + 1,
            },
            PointAt::Vertex {
                part: *part,
                ring: *ring,
                index: 0,
            },
        ],
        _ => Vec::new(),
    }
}

/// A segment–arc tangent whose arc's end a coincident ties to an end of the segment (docs/adr/0335 §2): that end of
/// the arc, so the tangent is written at it (its single root) rather than as the centre's distance (a double root
/// with the coincident).
fn tangent_end(rule: &Rule, all: &[super::Constraint], shapes: &[Shape]) -> Option<Ref> {
    let Rule::Tangent { a, b, .. } = rule else {
        return None;
    };
    if !matches!(a.element, Element::Segment { .. })
        || !matches!(shapes.get(b.object)?, Shape::Arc { .. })
    {
        return None;
    }
    let ends = segment_ends(shapes.get(a.object)?, &a.element);
    let on_segment = |r: &Ref| {
        r.object == a.object && matches!(r.element, Element::Point(p) if ends.contains(&p))
    };
    let arc_end = |r: &Ref| {
        r.object == b.object && matches!(r.element, Element::Point(PointAt::Start | PointAt::End))
    };
    all.iter().find_map(|c| match c.rule {
        Rule::Coincident { a: p, b: q } if !c.reference && arc_end(&p) && on_segment(&q) => Some(p),
        Rule::Coincident { a: p, b: q } if !c.reference && arc_end(&q) && on_segment(&p) => Some(q),
        _ => None,
    })
}

/// Union–find over the unknowns.
fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

struct Group {
    eqs: Vec<usize>,
    cols: Vec<usize>,
}

struct Rows {
    jac: Vec<Row>,
    res: Vec<f64>,
    /// Each row's equation (index into the group's `eqs`).
    of: Vec<usize>,
}

pub(super) fn run(p: &Problem<'_>, stop: &dyn Fn() -> bool) -> Outcome {
    let shapes = p.shapes;
    let mut used = vec![false; shapes.len()];
    let mut broken = Vec::new();
    for (i, c) in p.constraints.iter().enumerate() {
        let refs = c.rule.refs();
        if refs.iter().any(|r| r.object >= shapes.len()) {
            broken.push(i);
            continue;
        }
        for r in refs {
            used[r.object] = true;
        }
    }
    let mut model = Model::new(shapes, &used);
    // A local origin: the coordinates' middle, so the steps work on small numbers.
    let (mut sx, mut sy, mut nx, mut ny) = (0.0, 0.0, 0usize, 0usize);
    for (j, k) in model.kinds.iter().enumerate() {
        match k {
            VarKind::East => {
                sx += model.x[j];
                nx += 1;
            }
            VarKind::North => {
                sy += model.x[j];
                ny += 1;
            }
            _ => {}
        }
    }
    let origin = (
        if nx > 0 { sx / nx as f64 } else { 0.0 },
        if ny > 0 { sy / ny as f64 } else { 0.0 },
    );
    let shift = |x: &mut [f64], kinds: &[VarKind], sign: f64| {
        for (v, k) in x.iter_mut().zip(kinds) {
            match k {
                VarKind::East => *v += sign * origin.0,
                VarKind::North => *v += sign * origin.1,
                _ => {}
            }
        }
    };
    let start_x = model.x.clone();
    shift(&mut model.x, &model.kinds, -1.0);
    // The drawing's scale: the extent of the coordinates (at least a metre).
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for (v, k) in model.x.iter().zip(&model.kinds) {
        let axis = match k {
            VarKind::East => 0,
            VarKind::North => 1,
            _ => continue,
        };
        lo[axis] = libm::fmin(lo[axis], *v);
        hi[axis] = libm::fmax(hi[axis], *v);
    }
    let extent = if lo[0].is_finite() {
        crate::jsmath::js_hypot(hi[0] - lo[0], hi[1] - lo[1])
    } else {
        0.0
    };
    let s = libm::fmax(extent, 1.0);
    let tol = 1e-11 * s;

    // The fix shapes' unknowns, in the same layout and origin.
    let fix_x = match p.fix_from {
        Some(fixed) if fixed.len() == shapes.len() => {
            let mut fm = Model::new(fixed, &used);
            if fm.x.len() == model.x.len() {
                shift(&mut fm.x, &fm.kinds, -1.0);
                Some(fm.x)
            } else {
                None
            }
        }
        _ => None,
    };

    // The unknowns held.
    let mut free = vec![true; model.x.len()];
    for h in p.held {
        let vars = match h.element {
            None => model
                .slots
                .get(h.object)
                .and_then(|s| s.as_ref())
                .map(|slot| {
                    (slot.start..model.x.len())
                        .take_while(|&j| model.owner[j] == h.object)
                        .collect::<Vec<_>>()
                }),
            Some(e) => model.element_vars(shapes, h.object, &e),
        };
        for j in vars.into_iter().flatten() {
            free[j] = false;
        }
    }

    // The equations.
    let mut eqs: Vec<Eq> = Vec::new();
    let mut reference: Vec<(usize, Rule)> = Vec::new();
    let mut out_of_range = vec![false; p.constraints.len()];
    for &i in &broken {
        out_of_range[i] = true;
    }
    // The objects of the constraints that are not broken: only theirs are counted.
    let mut live = vec![false; shapes.len()];
    for (i, c) in p.constraints.iter().enumerate() {
        if out_of_range[i] {
            continue;
        }
        let mut parts_vars = Vec::new();
        let mut ok = true;
        let end = tangent_end(&c.rule, p.constraints, shapes);
        for r in c.rule.refs().into_iter().chain(end) {
            match model.element_vars(shapes, r.object, &r.element) {
                Some(v) => parts_vars.push(v),
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok || !c.value.is_finite() {
            broken.push(i);
            continue;
        }
        for r in c.rule.refs() {
            live[r.object] = true;
        }
        if c.reference {
            reference.push((i, c.rule));
            continue;
        }
        let mut vars: Vec<usize> = Vec::new();
        let parts = parts_vars
            .iter()
            .map(|pv| {
                pv.iter()
                    .map(|j| match vars.iter().position(|v| v == j) {
                        Some(k) => k,
                        None => {
                            vars.push(*j);
                            vars.len() - 1
                        }
                    })
                    .collect()
            })
            .collect();
        if vars.len() > super::dual::SLOTS {
            broken.push(i);
            continue;
        }
        let at = if matches!(c.rule, Rule::Fix { .. }) {
            let from = fix_x.as_deref().unwrap_or(&model.x);
            built_parts(&model, shapes, from, &c.rule).unwrap_or_default()
        } else {
            Vec::new()
        };
        eqs.push(Eq {
            constraint: i,
            rule: c.rule,
            value: c.value,
            vars,
            parts,
            at,
            end,
        });
    }

    // The groups: equations sharing a free unknown, joined.
    let n = model.x.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for eq in &eqs {
        let mut first = None;
        for &j in eq.vars.iter().filter(|&&j| free[j]) {
            match first {
                None => first = Some(j),
                Some(f) => {
                    let (a, b) = (find(&mut parent, f), find(&mut parent, j));
                    parent[a] = b;
                }
            }
        }
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut group_of: HashMap<usize, usize> = HashMap::new();
    let mut lone: Vec<usize> = Vec::new();
    for (e, eq) in eqs.iter().enumerate() {
        match eq.vars.iter().find(|&&j| free[j]) {
            Some(&j) => {
                let root = find(&mut parent, j);
                let g = *group_of.entry(root).or_insert_with(|| {
                    groups.push(Group {
                        eqs: Vec::new(),
                        cols: Vec::new(),
                    });
                    groups.len() - 1
                });
                groups[g].eqs.push(e);
            }
            // Every unknown held: the equation only checks.
            None => lone.push(e),
        }
    }
    for g in &mut groups {
        let mut cols: BTreeSet<usize> = BTreeSet::new();
        for &e in &g.eqs {
            cols.extend(eqs[e].vars.iter().copied().filter(|&j| free[j]));
        }
        g.cols = cols.into_iter().collect();
    }

    // The unknowns' weights: an arc's angles by its radius.
    let weight = |x: &[f64], j: usize| match model.kinds[j] {
        VarKind::Angle => {
            // The radius is the arc's third number: two or three before the angle.
            let start = model.slots[model.owner[j]].as_ref().map_or(j, |s| s.start);
            libm::fmax(x[start + 2].abs(), 1e-9)
        }
        _ => 1.0,
    };

    let mut x = model.x.clone();
    let mut redundant = BTreeSet::new();
    let mut conflicting = BTreeSet::new();
    let mut singular = BTreeSet::new();
    let mut rank_total = 0usize;
    let mut determined = vec![false; n];
    let mut iterations = 0u32;
    let mut cancelled = false;
    // Constraints whose rows lost their meaning on a full step (a segment pulled to nothing): the solve can only
    // creep towards that, so it says them singular rather than the conflict it would stop short at.
    let mut blew: BTreeSet<usize> = BTreeSet::new();

    for g in &groups {
        let col_of: HashMap<usize, usize> =
            g.cols.iter().enumerate().map(|(k, &j)| (j, k)).collect();
        let width = g.cols.len();
        let build_rows = |x: &[f64]| -> Rows {
            let mut out = Rows {
                jac: Vec::new(),
                res: Vec::new(),
                of: Vec::new(),
            };
            for (gi, &e) in g.eqs.iter().enumerate() {
                let eq = &eqs[e];
                for d in eval(eq, shapes, x, &free, s) {
                    let mut row: Row = Vec::with_capacity(eq.vars.len());
                    for (slot, &j) in eq.vars.iter().enumerate() {
                        if let Some(&c) = col_of.get(&j) {
                            let v = d.d[slot] / weight(x, j);
                            if v != 0.0 || !v.is_finite() {
                                row.push((c, v));
                            }
                        }
                    }
                    row.sort_unstable_by_key(|e| e.0);
                    out.jac.push(row);
                    out.res.push(d.v);
                    out.of.push(gi);
                }
            }
            out
        };
        let merit = |res: &[f64]| -> f64 {
            res.iter()
                .map(|r| if r.is_finite() { r * r } else { f64::INFINITY })
                .sum()
        };
        let mut steps = 0;
        let mut now = build_rows(&x);
        loop {
            if stop() {
                cancelled = true;
                break;
            }
            if now.res.iter().all(|r| r.abs() <= tol) || steps >= STEPS {
                break;
            }
            steps += 1;
            iterations += 1;
            let f = Factor::new(&now.jac, width);
            // The least-norm step of the kept rows: J_I·dz = −r_I.
            let b: Vec<f64> = f.kept.iter().map(|&i| -now.res[i]).collect();
            let dz = f.least_norm(&now.jac, &b, width);
            let before = merit(&now.res);
            let start: Vec<f64> = g.cols.iter().map(|&j| x[j]).collect();
            let dx: Vec<f64> = g
                .cols
                .iter()
                .zip(&dz)
                .map(|(&j, d)| d / weight(&x, j))
                .collect();
            let mut alpha = 1.0;
            let mut taken = None;
            for _ in 0..30 {
                for ((&j, s0), d) in g.cols.iter().zip(&start).zip(&dx) {
                    x[j] = s0 + alpha * d;
                }
                let trial = build_rows(&x);
                if alpha == 1.0 {
                    for (i, r) in trial.res.iter().enumerate() {
                        if !r.is_finite() {
                            blew.insert(eqs[g.eqs[trial.of[i]]].constraint);
                        }
                    }
                }
                if merit(&trial.res) < before {
                    taken = Some(trial);
                    break;
                }
                alpha *= 0.5;
            }
            match taken {
                Some(trial) => now = trial,
                None => {
                    for (&j, s0) in g.cols.iter().zip(&start) {
                        x[j] = *s0;
                    }
                    break;
                }
            }
        }
        if cancelled {
            break;
        }
        // The rank and the rows at the end.
        let end = now;
        let f = Factor::new(&end.jac, width);
        rank_total += f.rank();
        for &i in &f.dropped {
            let c = eqs[g.eqs[end.of[i]]].constraint;
            if end.res[i].abs() <= tol {
                redundant.insert(c);
            } else {
                conflicting.insert(c);
            }
        }
        for &i in &f.zero {
            let c = eqs[g.eqs[end.of[i]]].constraint;
            // A row of no length that holds repeats nothing yet asks nothing more (a distance of nought); one that
            // does not cannot be met from here: its parts lost their meaning (a segment of no length).
            if end.res[i].is_finite()
                && end.res[i].abs() <= tol
                && end.jac[i].iter().all(|v| v.1.is_finite())
            {
                redundant.insert(c);
            } else {
                singular.insert(c);
            }
        }
        for (i, r) in end.res.iter().enumerate() {
            if !r.is_finite() {
                singular.insert(eqs[g.eqs[end.of[i]]].constraint);
            }
        }
        // An unknown is fixed when no motion the rows allow moves it: a generic vector's part in their null space
        // (projected twice, the second taking out the first's rounding) is nought there. Two vectors, so that a
        // freedom one of them happens to miss is still seen.
        if f.rank() >= width {
            for &j in &g.cols {
                determined[j] = true;
            }
        } else {
            let mut seed = 0x9e37_79b9_7f4a_7c15u64;
            let mut moves = vec![0.0f64; width];
            for _ in 0..2 {
                let v: Vec<f64> = (0..width)
                    .map(|_| {
                        seed ^= seed << 13;
                        seed ^= seed >> 7;
                        seed ^= seed << 17;
                        1.0 + (seed >> 11) as f64 / (1u64 << 53) as f64
                    })
                    .collect();
                let once = f.null_part(&end.jac, &v);
                let twice = f.null_part(&end.jac, &once);
                for (m, t) in moves.iter_mut().zip(twice) {
                    *m = libm::fmax(*m, t.abs());
                }
            }
            for (c, &j) in g.cols.iter().enumerate() {
                determined[j] = moves[c] <= FIXED;
            }
        }
    }
    let mut lone_unmet = false;
    for &e in &lone {
        let eq = &eqs[e];
        if eval(eq, shapes, &x, &free, s)
            .iter()
            .any(|d| !d.v.is_finite() || d.v.abs() > tol)
        {
            lone_unmet = true;
            conflicting.insert(eq.constraint);
        } else {
            redundant.insert(eq.constraint);
        }
    }
    // Held unknowns are fixed by the holding.
    for (j, f) in free.iter().enumerate() {
        if !f {
            determined[j] = true;
        }
    }
    let unknowns = (0..n).filter(|&j| free[j] && live[model.owner[j]]).count();
    let dof = unknowns.saturating_sub(rank_total);
    let all_hold = groups.iter().all(|g| {
        g.eqs.iter().all(|&e| {
            eval(&eqs[e], shapes, &x, &free, s)
                .iter()
                .all(|d| d.v.abs() <= tol)
        })
    });
    // Conflicts found as unmet dependent rows count only when the solve could not meet them.
    let all_hold = all_hold && !lone_unmet;
    if all_hold {
        conflicting.clear();
    } else if !blew.is_empty() && !lone_unmet {
        singular.extend(blew);
        conflicting.clear();
    }
    let status = if cancelled {
        Status::Cancelled
    } else if !conflicting.is_empty() {
        Status::Inconsistent
    } else if !singular.is_empty() {
        Status::Singular
    } else if !all_hold {
        Status::NonConvergent
    } else if !redundant.is_empty() {
        Status::Over
    } else if dof == 0 {
        Status::Fully
    } else {
        Status::Under
    };

    // The shapes, back at their origin; only the ones that moved are rewritten.
    let mut out_x = x.clone();
    shift(&mut out_x, &model.kinds, 1.0);
    let mut shapes_out = shapes.to_vec();
    // Per object: whether it moved (moves of rounding size are not edits) and whether it can still move.
    let mut moved = vec![false; shapes.len()];
    let mut loose = vec![false; shapes.len()];
    for j in 0..n {
        let i = model.owner[j];
        moved[i] |= (x[j] - model.x[j]).abs() > tol * 1e-3;
        loose[i] |= !determined[j] && live[i];
    }
    let mut changed = Vec::new();
    for (i, slot) in model.slots.iter().enumerate() {
        if let Some(slot) = slot
            && moved[i]
            && status.holds()
        {
            write_back(&mut shapes_out[i], slot, &out_x);
            changed.push(i);
        }
    }
    let free_objects: Vec<usize> = (0..shapes.len()).filter(|&i| loose[i]).collect();
    let final_x = if status.holds() { &out_x } else { &start_x };
    let measured_out = reference
        .iter()
        .filter_map(|(i, rule)| {
            let parts = built_parts(&model, shapes, final_x, rule)?;
            measured(rule, &parts).map(|v| (*i, v))
        })
        .collect();
    broken.sort_unstable();
    Outcome {
        shapes: shapes_out,
        status,
        dof,
        redundant: redundant.into_iter().collect(),
        conflicting: conflicting.into_iter().collect(),
        singular: singular.into_iter().collect(),
        broken,
        free: free_objects,
        changed,
        measured: measured_out,
        iterations,
    }
}
