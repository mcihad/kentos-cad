//! The drawing's geometric and dimensional constraints and its parameter table (docs/adr/0334): one value the
//! document keeps in its undo history. An edit replaces it whole (`set_constraints`), into the open transaction or
//! as an undo step of its own, so the constraints a command adds and the objects its solve moves are one step.
//!
//! The rules (`constraints_problem`) hold after every edit: a set whose ids repeat, whose rules name parts that do
//! not fit them or objects the drawing does not hold, or whose parameters do not parse or read each other round is
//! refused with nothing changed. A step that deletes a constrained object takes its constraints with it (AutoCAD's
//! rule; `follow_constraints`), and a dimension's parameter with its constraint unless another parameter reads it.
//!
//! The contract's rules read as the core solver's (`core_rule`) here, once, for the commands and the document alike.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use kentos_contracts::{
    CadElement, ConstraintRule, ConstraintSet, DistanceAxis, ElementRef, Entity, EntityId,
    ParamUnits, ParameterKind, SubPlace, constraints_problem, param_expr_names, parse_param_expr,
};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::ops::constraints::{Axis, Element, PointAt, Ref, Rule, measure};

use crate::document::Document;
use crate::edit::Refusal;
use crate::history::Op;
use crate::identity::Uuid;

/// The solver's point of a contract's place; none for a place it does not take (an edge's middle, a place along).
pub fn core_point(place: &SubPlace) -> Option<PointAt> {
    Some(match *place {
        SubPlace::Start => PointAt::Start,
        SubPlace::End => PointAt::End,
        SubPlace::Center => PointAt::Center,
        SubPlace::Node { part } => PointAt::Node(part),
        SubPlace::Vertex { part, ring, index } => PointAt::Vertex { part, ring, index },
        _ => return None,
    })
}

/// The solver's part of a contract's; a place it does not take reads as a part no object has (the constraint is
/// broken).
pub fn core_element(e: &CadElement) -> Element {
    match e {
        CadElement::Point { place } => match core_point(place) {
            Some(p) => Element::Point(p),
            None => Element::Segment {
                part: u32::MAX,
                ring: u32::MAX,
                index: u32::MAX,
            },
        },
        CadElement::Segment { part, ring, index } => Element::Segment {
            part: *part,
            ring: *ring,
            index: *index,
        },
        CadElement::Circle => Element::Circle,
    }
}

/// The solver's rule for a constraint's, its objects numbered by `object`.
pub fn core_rule(rule: &ConstraintRule, object: &mut dyn FnMut(EntityId) -> usize) -> Rule {
    let mut r = |e: &ElementRef| Ref {
        object: object(e.uid),
        element: core_element(&e.element),
    };
    match rule {
        ConstraintRule::Coincident { a, b } => Rule::Coincident { a: r(a), b: r(b) },
        ConstraintRule::Horizontal { a, b } => Rule::Horizontal {
            a: r(a),
            b: b.as_ref().map(&mut r),
        },
        ConstraintRule::Vertical { a, b } => Rule::Vertical {
            a: r(a),
            b: b.as_ref().map(&mut r),
        },
        ConstraintRule::Parallel { a, b } => Rule::Parallel { a: r(a), b: r(b) },
        ConstraintRule::Perpendicular { a, b } => Rule::Perpendicular { a: r(a), b: r(b) },
        ConstraintRule::Tangent { a, b, inside, left } => Rule::Tangent {
            a: r(a),
            b: r(b),
            inside: *inside,
            left: *left,
        },
        ConstraintRule::Concentric { a, b } => Rule::Concentric { a: r(a), b: r(b) },
        ConstraintRule::Collinear { a, b } => Rule::Collinear { a: r(a), b: r(b) },
        ConstraintRule::Equal { a, b } => Rule::Equal { a: r(a), b: r(b) },
        ConstraintRule::Symmetric { a, b, axis } => Rule::Symmetric {
            a: r(a),
            b: r(b),
            axis: r(axis),
        },
        ConstraintRule::Fix { a } => Rule::Fix { a: r(a) },
        ConstraintRule::Distance {
            a, b, axis, left, ..
        } => Rule::Distance {
            a: r(a),
            b: r(b),
            axis: match axis {
                DistanceAxis::Aligned => Axis::Aligned,
                DistanceAxis::Horizontal => Axis::Horizontal,
                DistanceAxis::Vertical => Axis::Vertical,
            },
            left: *left,
        },
        ConstraintRule::Angle {
            a,
            b,
            reverse_a,
            reverse_b,
            ..
        } => Rule::Angle {
            a: r(a),
            b: r(b),
            reverse_a: *reverse_a,
            reverse_b: *reverse_b,
        },
        ConstraintRule::Radius { a, .. } => Rule::Radius { a: r(a) },
        ConstraintRule::Diameter { a, .. } => Rule::Diameter { a: r(a) },
    }
}

/// The geometry core's shape of an object (the contract's own JSON).
pub fn core_shape(e: &Entity) -> Option<Shape> {
    let json = serde_json::to_string(e).ok()?;
    CoreEntity::from_json(&Json::parse(&json).ok()?)
        .ok()
        .map(|c| c.shape)
}

/// What a dimension measures over the objects `shape_of` gives, in metres or radians.
pub fn measure_with(
    rule: &ConstraintRule,
    shape_of: &dyn Fn(EntityId) -> Option<Shape>,
) -> Option<f64> {
    let mut uids: Vec<EntityId> = Vec::new();
    let core = core_rule(rule, &mut |uid| match uids.iter().position(|u| *u == uid) {
        Some(i) => i,
        None => {
            uids.push(uid);
            uids.len() - 1
        }
    });
    let shapes: Option<Vec<Shape>> = uids.iter().map(|u| shape_of(*u)).collect();
    measure(&shapes?, &core)
}

/// A parameter's value in its own unit from metres or radians.
pub fn in_parameter_unit(units: ParamUnits, kind: ParameterKind, v: f64) -> f64 {
    match kind {
        ParameterKind::Length => v / units.metres_per_unit,
        ParameterKind::Angle => v / units.radians_per_unit,
        ParameterKind::Number => v,
    }
}

/// A value written as a parameter's expression: twelve decimals at most (below the solver's tolerance, so a
/// dimension added at the value the drawing measures moves nothing), no trailing zeros.
pub fn value_text(v: f64) -> String {
    let t = format!("{v:.12}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" { "0".into() } else { t.into() }
}

impl Document {
    /// The constraints and the parameter table.
    pub fn constraints(&self) -> &ConstraintSet {
        &self.constraints
    }

    /// Why `set` cannot be the drawing's: its problem, with where.
    pub(crate) fn constraints_refused(&self, set: &ConstraintSet) -> Option<String> {
        let uids: HashSet<EntityId> = self
            .store
            .iter()
            .map(|s| EntityId(s.uid.into_bytes()))
            .collect();
        constraints_problem(set, Some(&uids))
            .map(|(at, words)| format!("constraints/{at}: {words}"))
    }

    /// Gives the drawing the constraints `next` as one change named `label` (into the open transaction, else an
    /// undo step of its own). Refused with nothing changed when `next` breaks the rules. Whether anything changed.
    pub fn set_constraints(&mut self, next: ConstraintSet, label: &str) -> Result<bool, Refusal> {
        if *self.constraints == next {
            return Ok(false);
        }
        if let Some(why) = self.constraints_refused(&next) {
            return Err(Refusal(why));
        }
        let before = self.constraints.clone();
        let after = Arc::new(next);
        self.record(vec![Op::Constraints { before, after }], label);
        Ok(true)
    }

    /// The constraints naming objects the step deleted, taken away in the same step (docs/adr/0334 §1), with their
    /// dimensions' parameters unless another parameter reads them: then the parameter stays a user parameter, its
    /// value the one it had (a reference dimension's measured from the deleted object).
    pub(crate) fn follow_constraints(&mut self, ops: &[Op]) -> Vec<Op> {
        if self.constraints.constraints.is_empty() {
            return Vec::new();
        }
        let here = |uid: &EntityId| self.store.slot_of(Uuid::from_bytes(uid.0)).is_some();
        let gone: HashSet<EntityId> = self
            .constraints
            .constraints
            .iter()
            .flat_map(|c| c.rule.elements().into_iter().map(|e| e.uid))
            .filter(|u| !here(u))
            .collect();
        if gone.is_empty() {
            return Vec::new();
        }
        // The deleted objects as the step had them, for a reference value read by another parameter.
        let removed: HashMap<EntityId, Arc<Entity>> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Remove(s) => Some((EntityId(s.uid.into_bytes()), s.entity.clone())),
                _ => None,
            })
            .collect();
        let mut next = (*self.constraints).clone();
        let (dropped, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut next.constraints)
            .into_iter()
            .partition(|c| c.rule.elements().iter().any(|e| gone.contains(&e.uid)));
        next.constraints = kept;
        let read: BTreeSet<String> = next
            .parameters
            .iter()
            .filter_map(|p| parse_param_expr(&p.expression).ok())
            .flat_map(|e| param_expr_names(&e))
            .collect();
        let units = ParamUnits::of(self.settings());
        for c in &dropped {
            let Some(name) = c.rule.parameter() else {
                continue;
            };
            if !read.contains(name) {
                next.parameters.retain(|p| p.name != name);
                continue;
            }
            let Some(p) = next.parameters.iter_mut().find(|p| p.name == name) else {
                continue;
            };
            if p.expression.is_empty() {
                let shape_of = |uid: EntityId| {
                    let e = match self
                        .store
                        .slot_of(Uuid::from_bytes(uid.0))
                        .and_then(|s| self.store.get(s))
                    {
                        Some(s) => s.entity.clone(),
                        None => removed.get(&uid)?.clone(),
                    };
                    core_shape(&e)
                };
                let v = measure_with(&c.rule, &shape_of)
                    .map_or(0.0, |v| in_parameter_unit(units, p.kind, v));
                p.expression = value_text(v);
            }
        }
        // What is left must still keep the rules; when not (it cannot be by the steps above), the constraints go
        // whole rather than leave the drawing unsaveable.
        if self.constraints_refused(&next).is_some() {
            next = ConstraintSet::default();
        }
        let op = Op::Constraints {
            before: self.constraints.clone(),
            after: Arc::new(next),
        };
        self.apply_op(&op);
        vec![op]
    }
}
