//! The drawing's constraints solved (docs/adr/0334, 0335): the document's `ConstraintSet` read as the core solver's
//! problem, its parameters' values worked out from their expressions, the outcome read back by the constraints' and
//! the objects' ids, and the shapes the solve moved as the objects to write. Every command and tool that changes the
//! drawing with its constraints goes through here, so they all diagnose alike.
//!
//! - Units: a length parameter's expression is read in the drawing's unit and an angle's in the project's angle unit
//!   (`ParamUnits`); the solver works in metres and radians.
//! - A reference dimension's parameter is the value it measures now; it drives nothing.
//! - Objects on locked layers are held: the solve does not move them (a constraint that would need them moved
//!   conflicts).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_commands::{
    CadElement, CadParameter, ConstraintDiagnosis, ConstraintId, ConstraintRule, ConstraintSet,
    Entity, EntityId, ParamUnits, ParameterKind, SolveStatus, eval_param_expr, param_expr_names,
    param_order, parse_param_expr,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::constraints::{
    Constraint, Hold, Outcome, Problem, Ref, Rule, Status, measure, solve,
};

use crate::geometry::{shape, with_shape};

/// Why the set cannot be solved before the solver is asked: a code of `codes.rs`, the words, and the parameter or
/// constraint it is about.
#[derive(Clone, Debug, PartialEq)]
pub struct Unsolvable {
    pub code: &'static str,
    pub words: String,
    pub path: Option<String>,
}

/// The units a project's parameters are written in.
pub fn units(doc: &Document) -> ParamUnits {
    ParamUnits::of(doc.settings())
}

/// A rule's parts as the solver's: each object in the order first named, its slot.
pub struct Mapped {
    pub slots: Vec<Option<Slot>>,
    pub shapes: Vec<Shape>,
    index: HashMap<EntityId, usize>,
}

impl Mapped {
    fn object(&mut self, doc: &Document, uid: EntityId) -> usize {
        if let Some(&i) = self.index.get(&uid) {
            return i;
        }
        let slot = doc.slot_of(Uuid::from_bytes(uid.0));
        let shape = slot
            .and_then(|s| doc.get(s))
            .map(shape)
            .unwrap_or(Shape::Point {
                p: Default::default(),
                z: None,
                parts: None,
            });
        self.slots.push(slot);
        self.shapes.push(shape);
        self.index.insert(uid, self.slots.len() - 1);
        self.slots.len() - 1
    }

    /// The solver's index of an object, if it was named.
    pub fn index_of(&self, uid: EntityId) -> Option<usize> {
        self.index.get(&uid).copied()
    }
}

/// The solver's rule for a constraint's, its objects numbered as `mapped` first names them.
pub fn core_rule(doc: &Document, mapped: &mut Mapped, rule: &ConstraintRule) -> Rule {
    kentos_domain::constraints::core_rule(rule, &mut |uid| mapped.object(doc, uid))
}

/// What a dimension measures now in the drawing, in metres or radians.
pub fn measured_now(doc: &Document, rule: &ConstraintRule) -> Option<f64> {
    let mut mapped = Mapped {
        slots: Vec::new(),
        shapes: Vec::new(),
        index: HashMap::new(),
    };
    let core = core_rule(doc, &mut mapped, rule);
    if mapped.slots.iter().any(Option::is_none) {
        return None;
    }
    measure(&mapped.shapes, &core)
}

/// A parameter's value in metres or radians from its value in its own unit.
fn to_base(units: ParamUnits, kind: ParameterKind, v: f64) -> f64 {
    match kind {
        ParameterKind::Length => v * units.metres_per_unit,
        ParameterKind::Angle => v * units.radians_per_unit,
        ParameterKind::Number => v,
    }
}

/// A parameter's value in its own unit from metres or radians.
pub fn from_base(units: ParamUnits, kind: ParameterKind, v: f64) -> f64 {
    match kind {
        ParameterKind::Length => v / units.metres_per_unit,
        ParameterKind::Angle => v / units.radians_per_unit,
        ParameterKind::Number => v,
    }
}

/// Every parameter's value in its own unit: a reference dimension's as the drawing measures it now, the others by
/// their expressions in dependency order. Refused when an expression does not read, a name is missing, names read
/// each other round, a value is not finite or outside its range.
pub fn parameter_values(
    doc: &Document,
    set: &ConstraintSet,
) -> Result<BTreeMap<String, f64>, Unsolvable> {
    let units = units(doc);
    let at = |i: usize| Some(format!("parameters[{i}]"));
    let mut values: BTreeMap<String, f64> = BTreeMap::new();
    let mut parsed = BTreeMap::new();
    let mut reads: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let index: HashMap<&str, usize> = set
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();
    for (i, p) in set.parameters.iter().enumerate() {
        if p.expression.is_empty() {
            // A reference dimension's: measured.
            let rule = set
                .constraints
                .iter()
                .find(|c| c.rule.parameter() == Some(p.name.as_str()))
                .map(|c| &c.rule);
            let v = rule
                .and_then(|r| measured_now(doc, r))
                .map(|v| from_base(units, p.kind, v))
                .unwrap_or(f64::NAN);
            values.insert(p.name.clone(), v);
            reads.insert(p.name.clone(), BTreeSet::new());
            continue;
        }
        let expr = parse_param_expr(&p.expression).map_err(|words| Unsolvable {
            code: "invalid_expression",
            words: format!("“{}” okunamadı: {words}", p.name),
            path: at(i),
        })?;
        let names = param_expr_names(&expr);
        if let Some(missing) = names.iter().find(|n| !index.contains_key(n.as_str())) {
            return Err(Unsolvable {
                code: "unknown_parameter",
                words: format!(
                    "“{}” “{missing}” diye bir parametreyi okuyor; böyle bir parametre yok",
                    p.name
                ),
                path: at(i),
            });
        }
        reads.insert(p.name.clone(), names);
        parsed.insert(p.name.clone(), expr);
    }
    let order = param_order(&reads).map_err(|circle| Unsolvable {
        code: "parameter_cycle",
        words: format!("parametreler birbirini okuyor: {}", circle.join(" → ")),
        path: circle
            .first()
            .and_then(|n| index.get(n.as_str()))
            .and_then(|&i| at(i)),
    })?;
    for name in order {
        let Some(expr) = parsed.get(&name) else {
            continue;
        };
        let v = eval_param_expr(expr, units, &|n| values.get(n).copied());
        let i = index[name.as_str()];
        let p = &set.parameters[i];
        let v = v.map_err(|words| Unsolvable {
            code: "invalid_expression",
            words: format!("“{name}” hesaplanamadı: {words}"),
            path: at(i),
        })?;
        if !v.is_finite() {
            return Err(Unsolvable {
                code: "not_finite",
                words: format!("“{name}” sonlu bir sayı değil"),
                path: at(i),
            });
        }
        if p.min.is_some_and(|lo| v < lo) || p.max.is_some_and(|hi| v > hi) {
            return Err(Unsolvable {
                code: "invalid_value",
                words: format!("“{name}” = {v}, aralığının dışında"),
                path: at(i),
            });
        }
        values.insert(name, v);
    }
    Ok(values)
}

/// A solve of the document's constraints, read back by ids.
pub struct Solved {
    pub outcome: Outcome,
    pub mapped: Mapped,
    pub diagnosis: ConstraintDiagnosis,
    /// The objects the solve moved, as they would be written (their slots).
    pub changed: Vec<(Slot, Entity)>,
    /// The objects that would turn inside out (docs/adr/0335 §4).
    pub flipped: Vec<Slot>,
}

/// The constraints of `set` (those at `only`, or all) the drawing as it is does not meet (docs/adr/0337): every
/// object held, so each equation only checks, to the solver's tolerance; a fix at its place in `fix_from` (an edit's
/// before), else where it is; a constraint whose parts are gone too. None when the parameters cannot be worked out.
pub fn unmet(
    doc: &Document,
    set: &ConstraintSet,
    only: Option<&[usize]>,
    fix_from: Option<&HashMap<EntityId, Shape>>,
) -> Vec<usize> {
    let held: Vec<Held> = set
        .constraints
        .iter()
        .flat_map(|c| c.rule.elements().into_iter().map(|e| e.uid))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|uid| Held { uid, element: None })
        .collect();
    let Ok(solved) = solve_set(doc, set, &held, fix_from, &|| false) else {
        return Vec::new();
    };
    let o = &solved.outcome;
    let mut out: Vec<usize> = o
        .conflicting
        .iter()
        .chain(&o.singular)
        .chain(&o.broken)
        .copied()
        .filter(|i| only.is_none_or(|s| s.contains(i)))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Extra unknowns a caller holds (a dragged grip, docs/adr/0337): by object id and part.
#[derive(Clone, Copy, Debug)]
pub struct Held {
    pub uid: EntityId,
    pub element: Option<CadElement>,
}

/// Solves `set` over the document as it is. `fix_from`: the shapes a fix holds its parts at, by object id (an
/// edit's before); none: where they are.
pub fn solve_set(
    doc: &Document,
    set: &ConstraintSet,
    held: &[Held],
    fix_from: Option<&HashMap<EntityId, Shape>>,
    stop: &dyn Fn() -> bool,
) -> Result<Solved, Unsolvable> {
    let values = parameter_values(doc, set)?;
    let units = units(doc);
    let mut mapped = Mapped {
        slots: Vec::new(),
        shapes: Vec::new(),
        index: HashMap::new(),
    };
    let mut constraints = Vec::with_capacity(set.constraints.len());
    for c in &set.constraints {
        let rule = core_rule(doc, &mut mapped, &c.rule);
        let value = match c.rule.parameter() {
            None => 0.0,
            Some(name) => {
                let kind = set
                    .parameter(name)
                    .map_or(ParameterKind::Length, |p| p.kind);
                values
                    .get(name)
                    .map_or(f64::NAN, |&v| to_base(units, kind, v))
            }
        };
        constraints.push(Constraint {
            rule,
            value,
            reference: c.reference,
        });
    }
    // A missing object is out of the solver's range: its constraints are broken.
    let missing: Vec<usize> = (0..mapped.slots.len())
        .filter(|&i| mapped.slots[i].is_none())
        .collect();
    if !missing.is_empty() {
        for c in &mut constraints {
            remap_missing(&mut c.rule, &missing, mapped.shapes.len());
        }
    }
    let mut holds: Vec<Hold> = Vec::new();
    for (i, slot) in mapped.slots.iter().enumerate() {
        if let Some(e) = slot.and_then(|s| doc.get(s))
            && doc.layers().is_locked(e.base().layer_id.as_str())
        {
            holds.push(Hold {
                object: i,
                element: None,
            });
        }
    }
    for h in held {
        if let Some(i) = mapped.index_of(h.uid) {
            holds.push(Hold {
                object: i,
                element: h
                    .element
                    .as_ref()
                    .map(kentos_domain::constraints::core_element),
            });
        }
    }
    let fix_shapes: Option<Vec<Shape>> = fix_from.map(|before| {
        mapped
            .index
            .iter()
            .fold(mapped.shapes.clone(), |mut out, (uid, &i)| {
                if let Some(s) = before.get(uid) {
                    out[i] = s.clone();
                }
                out
            })
    });
    let outcome = solve(
        &Problem {
            shapes: &mapped.shapes,
            constraints: &constraints,
            held: &holds,
            fix_from: fix_shapes.as_deref(),
        },
        stop,
    );
    let ids: Vec<ConstraintId> = set.constraints.iter().map(|c| c.id).collect();
    let uid_of = |i: usize| {
        mapped.slots[i]
            .and_then(|s| doc.uid(s))
            .map(|u| EntityId(u.into_bytes()))
    };
    let by = |list: &[usize]| list.iter().map(|&k| ids[k]).collect::<Vec<_>>();
    let diagnosis = ConstraintDiagnosis {
        status: match outcome.status {
            Status::Fully => SolveStatus::Fully,
            Status::Under => SolveStatus::Under,
            Status::Over => SolveStatus::Over,
            Status::Inconsistent => SolveStatus::Inconsistent,
            Status::Singular => SolveStatus::Singular,
            Status::NonConvergent | Status::Cancelled => SolveStatus::NonConvergent,
        },
        dof: outcome.dof as u32,
        redundant: by(&outcome.redundant),
        conflicting: by(&outcome.conflicting),
        singular: by(&outcome.singular),
        broken: by(&outcome.broken),
        free: outcome.free.iter().filter_map(|&i| uid_of(i)).collect(),
        iterations: outcome.iterations,
    };
    let mut changed = Vec::new();
    let mut flipped = Vec::new();
    for &i in &outcome.changed {
        let Some(slot) = mapped.slots[i] else {
            continue;
        };
        let Some(entity) = doc.get(slot) else {
            continue;
        };
        if turned(&mapped.shapes[i], &outcome.shapes[i]) {
            flipped.push(slot);
        }
        if let Some(next) = with_shape(entity, outcome.shapes[i].clone()) {
            changed.push((slot, next));
        }
    }
    Ok(Solved {
        outcome,
        mapped,
        diagnosis,
        changed,
        flipped,
    })
}

/// Points a rule's parts at missing objects past the solver's shapes, so they are broken.
fn remap_missing(rule: &mut Rule, missing: &[usize], len: usize) {
    let fix = |r: &mut Ref| {
        if missing.contains(&r.object) {
            r.object = len;
        }
    };
    match rule {
        Rule::Horizontal { a, b } | Rule::Vertical { a, b } => {
            fix(a);
            if let Some(b) = b {
                fix(b);
            }
        }
        Rule::Fix { a } | Rule::Radius { a } | Rule::Diameter { a } => fix(a),
        Rule::Symmetric { a, b, axis } => {
            fix(a);
            fix(b);
            fix(axis);
        }
        Rule::Coincident { a, b }
        | Rule::Parallel { a, b }
        | Rule::Perpendicular { a, b }
        | Rule::Tangent { a, b, .. }
        | Rule::Concentric { a, b }
        | Rule::Collinear { a, b }
        | Rule::Equal { a, b }
        | Rule::Distance { a, b, .. }
        | Rule::Angle { a, b, .. } => {
            fix(a);
            fix(b);
        }
    }
}

/// Twice the signed area of a ring.
fn ring_area(pts: &[kentos_geometry_core::Vec2]) -> f64 {
    let n = pts.len();
    (0..n)
        .map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y)
        .sum()
}

/// Whether `after` is `before` turned inside out (docs/adr/0335 §4): a ring's winding reversed, a line's direction
/// reversed, a circle's or an arc's radius not above nought, an arc's sweep reversed.
pub fn turned(before: &Shape, after: &Shape) -> bool {
    match (before, after) {
        (Shape::Line { a, b }, Shape::Line { a: c, b: d }) => {
            let (u, v) = ((b.x - a.x, b.y - a.y), (d.x - c.x, d.y - c.y));
            u.0 * v.0 + u.1 * v.1 < 0.0
        }
        (Shape::Circle { .. }, Shape::Circle { r, .. }) => *r <= 0.0,
        (
            Shape::Arc { a0, a1, .. },
            Shape::Arc {
                r, a0: b0, a1: b1, ..
            },
        ) => *r <= 0.0 || ((a1 - a0) > 0.0) != ((b1 - b0) > 0.0),
        (Shape::Polygon { pts: p, .. }, Shape::Polygon { pts: q, .. }) => {
            let (x, y) = (ring_area(p), ring_area(q));
            x != 0.0 && x * y <= 0.0
        }
        (
            Shape::Polyline {
                pts: p,
                closed: true,
                ..
            },
            Shape::Polyline { pts: q, .. },
        ) => {
            let (x, y) = (ring_area(p), ring_area(q));
            x != 0.0 && x * y <= 0.0
        }
        _ => false,
    }
}

/// The parameters' kind a dimension takes.
pub fn kind_of(rule: &ConstraintRule) -> ParameterKind {
    match rule {
        ConstraintRule::Angle { .. } => ParameterKind::Angle,
        _ => ParameterKind::Length,
    }
}

/// The next free parameter name for a dimension (AutoCAD's d1, ang1, rad1, dia1).
pub fn next_name(params: &[CadParameter], rule: &ConstraintRule) -> String {
    let stem = match rule {
        ConstraintRule::Angle { .. } => "ang",
        ConstraintRule::Radius { .. } => "rad",
        ConstraintRule::Diameter { .. } => "dia",
        _ => "d",
    };
    (1..)
        .map(|k| format!("{stem}{k}"))
        .find(|n| params.iter().all(|p| &p.name != n))
        .unwrap_or_default()
}
