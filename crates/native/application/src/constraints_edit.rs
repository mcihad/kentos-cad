//! `cad.constraints.edit` v1 (docs/adr/0335; AutoCAD's GEOMCONSTRAINT, DIMCONSTRAINT, DELCONSTRAINT, the reference
//! parameter and the flip of a constraint's solution): constraints added to or taken from the drawing, a dimension
//! made to drive or only to measure, a constraint's other solution, or the drawing solved again; every object the
//! solve moves and the constraints themselves in one undo step “Kısıtlar”. Objects on locked layers do not move.
//!
//! A new constraint's sense is read from the drawing unless the input gives it (`sense`): a tangent's side, a
//! distance's side of its segment, the angle under 180°, a horizontal or vertical distance from the lower to the
//! higher. The solve keeps it. A solution that turns a shape inside out is refused unless `allowFlip`.
//!
//! The checks, in order (the first that fails answers): the edit's own shape (an empty list, ids that are not
//! uuids); the rules' parts and kinds; parameter names and expressions as written; the expected revision; the
//! objects, their parts and the constraints named; the set's rules (`constraints_problem`); the parameters' values
//! (names, rounds, ranges); the solve: conflicting, singular, not converged, a constraint added that repeats the
//! others, a shape turned.

use std::collections::{BTreeSet, HashSet};

use kentos_contracts::{
    CadElement, CadParameter, CommandResult, Constraint, ConstraintEdit, ConstraintId,
    ConstraintRule, ConstraintSense, ConstraintSet, ConstraintsEdit, ConstraintsEditPlan,
    ConstraintsEdited, DistanceAxis, ElementRef, EntityId, ParameterKind, SolveStatus,
    constraints_problem, param_expr_names, parameter_name_valid, parse_param_expr,
};
use kentos_domain::constraints::{
    core_element, core_shape, in_parameter_unit, measure_with, value_text,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::constraints::has_part;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::constraint_solve::{
    Held, Solved, Unsolvable, kind_of, next_name, parameter_values, solve_set, units,
};

/// The undo step's name.
pub const LABEL: &str = "Kısıtlar";

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ConstraintsEdit) -> CommandResult<()> {
    match check(cx.doc, input, &|| false) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: the constraints and parameters as they would be, the objects the solve moves and
/// the diagnosis; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ConstraintsEdit,
) -> CommandResult<ConstraintsEditPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document read in place (a tool's preview).
pub fn plan_in(doc: &Document, input: &ConstraintsEdit) -> CommandResult<ConstraintsEditPlan> {
    match check(doc, input, &|| false) {
        Ok(planned) => CommandResult::Completed {
            output: ConstraintsEditPlan {
                constraints: planned.set.constraints.clone(),
                parameters: planned.set.parameters.clone(),
                changed: planned
                    .solved
                    .changed
                    .iter()
                    .map(|(_, e)| e.clone())
                    .collect(),
                diagnosis: planned.solved.diagnosis.clone(),
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the constraints and the moved objects as one undo step “Kısıtlar”, into the open
/// transaction or group if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ConstraintsEdit,
) -> CommandResult<ConstraintsEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_CONSTRAINTS_EDIT,
        kentos_contracts::CAD_CONSTRAINTS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ConstraintsEdit,
) -> CommandResult<ConstraintsEdited> {
    let planned = match check(cx.doc, &input, &|| false) {
        Ok(planned) => planned,
        Err(stop) => return stop.into(),
    };
    let Planned {
        set,
        solved,
        added,
        removed,
    } = planned;
    let changed: Vec<String> = solved
        .changed
        .iter()
        .filter_map(|(slot, _)| cx.doc.uid(*slot))
        .map(|u| u.to_string())
        .collect();
    let diagnosis = solved.diagnosis.clone();
    let written = cx.doc.transact(LABEL, |doc| {
        for (slot, entity) in solved.changed {
            doc.update(slot, entity);
        }
        doc.set_constraints(set, LABEL).map(|_| ())
    });
    match written {
        Ok(()) => CommandResult::Completed {
            output: ConstraintsEdited {
                added: added.iter().map(|id| id.to_string()).collect(),
                removed: removed.iter().map(|id| id.to_string()).collect(),
                changed,
                diagnosis,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => CommandResult::Failed {
            error: checks::error(
                codes::INVALID_CONSTRAINT,
                refusal.to_string(),
                Some("edit".into()),
            ),
        },
    }
}

/// What the edit writes.
pub(crate) struct Planned {
    pub set: ConstraintSet,
    pub solved: Solved,
    pub added: Vec<ConstraintId>,
    pub removed: Vec<ConstraintId>,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

fn refuse_ids(code: &str, message: String, path: &str, uids: Vec<String>) -> Stop {
    let mut error = checks::error(code, message, Some(path.into()));
    error.uids = uids;
    Stop::failed(error)
}

/// A constraint id from its text.
fn id_of(text: &str) -> Option<ConstraintId> {
    if !checks::is_uid_text(text) {
        return None;
    }
    Uuid::parse_str(text)
        .ok()
        .map(|u| ConstraintId(u.into_bytes()))
}

/// What a rule's parts must be, by its kind: point, segment, circle; for two the first's options.
fn fits(rule: &ConstraintRule) -> bool {
    use CadElement as E;
    let point = |e: &ElementRef| matches!(e.element, E::Point { .. });
    let seg = |e: &ElementRef| matches!(e.element, E::Segment { .. });
    let circle = |e: &ElementRef| matches!(e.element, E::Circle);
    match rule {
        ConstraintRule::Coincident { a, b } => {
            point(a) && !matches!(b.element, E::Point { .. } if false)
        }
        ConstraintRule::Horizontal { a, b } | ConstraintRule::Vertical { a, b } => match b {
            None => seg(a),
            Some(b) => point(a) && point(b),
        },
        ConstraintRule::Parallel { a, b }
        | ConstraintRule::Perpendicular { a, b }
        | ConstraintRule::Collinear { a, b }
        | ConstraintRule::Angle { a, b, .. } => seg(a) && seg(b),
        ConstraintRule::Tangent { a, b, .. } => (seg(a) || circle(a)) && circle(b),
        ConstraintRule::Concentric { a, b } => circle(a) && circle(b),
        ConstraintRule::Equal { a, b } => (seg(a) && seg(b)) || (circle(a) && circle(b)),
        ConstraintRule::Symmetric { a, b, axis } => point(a) && point(b) && seg(axis),
        ConstraintRule::Fix { .. } => true,
        ConstraintRule::Distance { a, b, axis, .. } => {
            point(a) && (point(b) || (seg(b) && *axis == DistanceAxis::Aligned))
        }
        ConstraintRule::Radius { a, .. } | ConstraintRule::Diameter { a, .. } => circle(a),
    }
}

/// The parameter name a dimension gives, mutably.
fn parameter_mut(rule: &mut ConstraintRule) -> Option<&mut String> {
    match rule {
        ConstraintRule::Distance { parameter, .. }
        | ConstraintRule::Angle { parameter, .. }
        | ConstraintRule::Radius { parameter, .. }
        | ConstraintRule::Diameter { parameter, .. } => Some(parameter),
        _ => None,
    }
}

/// The objects' shapes for measuring, as the document has them now.
fn shape_of(
    doc: &Document,
) -> impl Fn(EntityId) -> Option<kentos_geometry_core::entity::Shape> + '_ {
    move |uid| core_shape(doc.get(doc.slot_of(Uuid::from_bytes(uid.0))?)?)
}

/// A new constraint's sense read from the drawing (docs/adr/0335 §2).
fn sensed(doc: &Document, mut rule: ConstraintRule) -> ConstraintRule {
    let shapes = shape_of(doc);
    let measure = |r: &ConstraintRule| measure_with(r, &shapes);
    let probe = |a: ElementRef, b: ElementRef| ConstraintRule::Distance {
        a,
        b,
        axis: DistanceAxis::Aligned,
        left: true,
        parameter: String::new(),
    };
    match &mut rule {
        ConstraintRule::Tangent { a, b, inside, left } => match a.element {
            CadElement::Segment { .. } => {
                let centre = ElementRef {
                    uid: b.uid,
                    element: CadElement::Point {
                        place: kentos_contracts::SubPlace::Center,
                    },
                };
                *left = measure(&probe(centre, *a)).is_none_or(|d| d >= 0.0);
                *inside = false;
            }
            _ => {
                let (ca, cb) = (
                    ElementRef {
                        uid: a.uid,
                        element: CadElement::Point {
                            place: kentos_contracts::SubPlace::Center,
                        },
                    },
                    ElementRef {
                        uid: b.uid,
                        element: CadElement::Point {
                            place: kentos_contracts::SubPlace::Center,
                        },
                    },
                );
                let d = measure(&ConstraintRule::Distance {
                    a: ca,
                    b: cb,
                    axis: DistanceAxis::Aligned,
                    left: false,
                    parameter: String::new(),
                });
                let ra = measure(&ConstraintRule::Radius {
                    a: *a,
                    parameter: String::new(),
                });
                let rb = measure(&ConstraintRule::Radius {
                    a: *b,
                    parameter: String::new(),
                });
                if let (Some(d), Some(ra), Some(rb)) = (d, ra, rb) {
                    *inside = (d - (ra - rb).abs()).abs() < (d - (ra + rb)).abs();
                }
                *left = false;
            }
        },
        ConstraintRule::Distance {
            a, b, axis, left, ..
        } => match (b.element, *axis) {
            (CadElement::Segment { .. }, _) => {
                *left = measure(&probe(*a, *b)).is_none_or(|d| d >= 0.0);
            }
            (_, DistanceAxis::Horizontal | DistanceAxis::Vertical) => {
                let m = measure(&ConstraintRule::Distance {
                    a: *a,
                    b: *b,
                    axis: *axis,
                    left: false,
                    parameter: String::new(),
                });
                if m.is_some_and(|m| m < 0.0) {
                    std::mem::swap(a, b);
                }
                *left = false;
            }
            _ => *left = false,
        },
        ConstraintRule::Angle {
            a,
            b,
            reverse_a,
            reverse_b,
            ..
        } => {
            *reverse_a = false;
            *reverse_b = false;
            let m = measure(&ConstraintRule::Angle {
                a: *a,
                b: *b,
                reverse_a: false,
                reverse_b: false,
                parameter: String::new(),
            });
            if m.is_some_and(|m| m > std::f64::consts::PI) {
                *reverse_b = true;
            }
        }
        _ => {}
    }
    rule
}

/// The constraint's other solution (docs/adr/0335 §3); none for a rule that has one solution.
fn flipped(rule: &ConstraintRule) -> Option<ConstraintRule> {
    let mut r = rule.clone();
    match &mut r {
        ConstraintRule::Tangent {
            a, inside, left, ..
        } => match a.element {
            CadElement::Segment { .. } => *left = !*left,
            _ => *inside = !*inside,
        },
        ConstraintRule::Distance {
            a, b, axis, left, ..
        } => match (b.element, *axis) {
            (CadElement::Segment { .. }, _) => *left = !*left,
            (_, DistanceAxis::Horizontal | DistanceAxis::Vertical) => std::mem::swap(a, b),
            _ => return None,
        },
        // The other quadrant: the angle measured from b to a.
        ConstraintRule::Angle {
            a,
            b,
            reverse_a,
            reverse_b,
            ..
        } => {
            std::mem::swap(a, b);
            std::mem::swap(reverse_a, reverse_b);
        }
        _ => return None,
    }
    Some(r)
}

/// The edit's own shape, before the drawing is read.
fn check_shape(edit: &ConstraintEdit) -> Result<(), Stop> {
    let ids = |ids: &[String], path: &str| -> Result<(), Stop> {
        if ids.is_empty() {
            return Err(refuse(
                codes::NO_CONSTRAINTS,
                "Kısıtların kimliklerini verin.".into(),
                path,
            ));
        }
        for (i, id) in ids.iter().enumerate() {
            if id_of(id).is_none() {
                return Err(refuse(
                    codes::INVALID_UID,
                    format!(
                        "“{id}” geçerli bir kısıt kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."
                    ),
                    &format!("{path}[{i}]"),
                ));
            }
        }
        Ok(())
    };
    match edit {
        ConstraintEdit::Add { constraints } => {
            if constraints.is_empty() {
                return Err(refuse(
                    codes::NO_CONSTRAINTS,
                    "Eklenecek kısıtları verin.".into(),
                    "edit.constraints",
                ));
            }
            for (i, c) in constraints.iter().enumerate() {
                let at = format!("edit.constraints[{i}]");
                if !fits(&c.rule) {
                    return Err(refuse(
                        codes::INVALID_CONSTRAINT,
                        format!(
                            "{} kısıtının parçaları kuralına uymuyor: paralel, dik, doğrudaş ve açı iki kenar; eş merkezli iki daire; yarıçap ve çap daire; simetrik iki nokta ve bir kenar ister.",
                            kentos_contracts::rule_label(&c.rule)
                        ),
                        &format!("{at}.rule"),
                    ));
                }
                if c.reference && c.rule.parameter().is_none() {
                    return Err(refuse(
                        codes::INVALID_CONSTRAINT,
                        "Yalnız ölçü kısıtı (uzaklık, açı, yarıçap, çap) başvuru olur.".into(),
                        &format!("{at}.reference"),
                    ));
                }
                if let Some(name) = c.rule.parameter()
                    && !name.is_empty()
                    && !parameter_name_valid(name)
                {
                    return Err(refuse(
                        codes::INVALID_PARAMETER,
                        format!(
                            "“{name}” bir parametre adı değil: harfle başlar; harf, rakam ve _ içerir, işlev adı olmaz."
                        ),
                        &format!("{at}.rule.parameter"),
                    ));
                }
                if let Some(value) = &c.value {
                    if c.reference || c.rule.parameter().is_none() {
                        return Err(refuse(
                            codes::PARAMETER_EXPRESSION,
                            "Değer yalnız sürücü ölçü kısıtına verilir; başvuru kısıtı değerini çizimden ölçer.".into(),
                            &format!("{at}.value"),
                        ));
                    }
                    if let Err(words) = parse_param_expr(value) {
                        return Err(refuse(
                            codes::PARAMETER_EXPRESSION,
                            format!("“{value}” okunamadı: {words}"),
                            &format!("{at}.value"),
                        ));
                    }
                }
            }
            Ok(())
        }
        ConstraintEdit::Remove { ids: list } => ids(list, "edit.ids"),
        ConstraintEdit::Mode { ids: list, .. } => ids(list, "edit.ids"),
        ConstraintEdit::Flip { id } => ids(std::slice::from_ref(id), "edit.id"),
        ConstraintEdit::Solve => Ok(()),
    }
}

/// The objects and parts a rule names, found in the drawing.
fn check_parts(doc: &Document, rule: &ConstraintRule, at: &str) -> Result<(), Stop> {
    for e in rule.elements() {
        let uid = Uuid::from_bytes(e.uid.0);
        let Some(entity) = doc.slot_of(uid).and_then(|s| doc.get(s)) else {
            return Err(refuse_ids(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "{uid} kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir."
                ),
                at,
                vec![uid.to_string()],
            ));
        };
        let has = core_shape(entity).is_some_and(|s| has_part(&s, &core_element(&e.element)));
        if !has {
            return Err(refuse_ids(
                codes::ELEMENT_NOT_FOUND,
                format!(
                    "{uid} kimlikli nesnede kısıtın andığı parça yok: köşe numarası, düz kenar (yaylı kenar kısıt almaz) ya da daire olarak yeniden seçin."
                ),
                at,
                vec![uid.to_string()],
            ));
        }
    }
    Ok(())
}

/// Every constraint whose id the list names, by its position; refused when one is missing.
fn positions(set: &ConstraintSet, ids: &[String], path: &str) -> Result<Vec<usize>, Stop> {
    ids.iter()
        .enumerate()
        .map(|(i, text)| {
            let id = id_of(text);
            set.constraints
                .iter()
                .position(|c| Some(c.id) == id)
                .ok_or_else(|| {
                    refuse(
                        codes::CONSTRAINT_NOT_FOUND,
                        format!("“{text}” kimlikli kısıt çizimde yok: kaldırılmış olabilir."),
                        &format!("{path}[{i}]"),
                    )
                })
        })
        .collect()
}

fn unsolvable(u: Unsolvable) -> Stop {
    let code = match u.code {
        "invalid_expression" | "not_finite" => codes::PARAMETER_EXPRESSION,
        "unknown_parameter" => codes::PARAMETER_MISSING,
        "parameter_cycle" => codes::PARAMETER_CYCLE,
        _ => codes::PARAMETER_RANGE,
    };
    Stop::failed(checks::error(code, u.words, u.path))
}

/// The value a dimension measures now as its parameter's expression.
fn measured_text(doc: &Document, rule: &ConstraintRule, kind: ParameterKind) -> String {
    let v = measure_with(rule, &shape_of(doc)).unwrap_or(0.0);
    value_text(in_parameter_unit(units(doc), kind, v))
}

/// Parameters only the removed dimensions had go; those another parameter reads stay with their value.
fn drop_parameters(doc: &Document, set: &mut ConstraintSet, gone: &[Constraint]) {
    let read: BTreeSet<String> = set
        .parameters
        .iter()
        .filter_map(|p| parse_param_expr(&p.expression).ok())
        .flat_map(|e| param_expr_names(&e))
        .collect();
    for c in gone {
        let Some(name) = c.rule.parameter() else {
            continue;
        };
        if !read.contains(name) {
            set.parameters.retain(|p| p.name != name);
        } else if let Some(p) = set.parameters.iter_mut().find(|p| p.name == name)
            && p.expression.is_empty()
        {
            p.expression = measured_text(doc, &c.rule, p.kind);
        }
    }
}

pub(crate) fn check(
    doc: &Document,
    input: &ConstraintsEdit,
    stop: &dyn Fn() -> bool,
) -> Result<Planned, Stop> {
    check_shape(&input.edit)?;
    let mut held = Vec::new();
    for (i, text) in input.hold.iter().enumerate() {
        let Some(u) = Uuid::parse_str(text)
            .ok()
            .filter(|_| checks::is_uid_text(text))
        else {
            return Err(refuse(
                codes::INVALID_UID,
                format!(
                    "“{text}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."
                ),
                &format!("hold[{i}]"),
            ));
        };
        held.push((i, u));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    for (i, u) in &held {
        if doc.slot_of(*u).is_none() {
            return Err(refuse_ids(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "{u} kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir."
                ),
                &format!("hold[{i}]"),
                vec![u.to_string()],
            ));
        }
    }
    let held: Vec<Held> = held
        .iter()
        .map(|(_, u)| Held {
            uid: EntityId(u.into_bytes()),
            element: None,
        })
        .collect();
    let mut set = doc.constraints().clone();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    match &input.edit {
        ConstraintEdit::Add { constraints } => {
            for (i, c) in constraints.iter().enumerate() {
                let at = format!("edit.constraints[{i}]");
                check_parts(doc, &c.rule, &format!("{at}.rule"))?;
                let mut rule = if c.sense == ConstraintSense::Drawing {
                    sensed(doc, c.rule.clone())
                } else {
                    c.rule.clone()
                };
                let kind = kind_of(&rule);
                if let Some(name) = parameter_mut(&mut rule) {
                    if name.is_empty() {
                        *name = next_name(&set.parameters, &c.rule);
                    }
                    let name = name.clone();
                    if set
                        .constraints
                        .iter()
                        .any(|k| k.rule.parameter() == Some(name.as_str()))
                    {
                        return Err(refuse(
                            codes::PARAMETER_IN_USE,
                            format!(
                                "“{name}” parametresi başka bir ölçü kısıtının; başka bir ad verin ya da adı boş bırakın."
                            ),
                            &format!("{at}.rule.parameter"),
                        ));
                    }
                    let expression = if c.reference {
                        String::new()
                    } else {
                        match &c.value {
                            Some(v) => v.clone(),
                            None => measured_text(doc, &rule, kind),
                        }
                    };
                    match set.parameters.iter_mut().find(|p| p.name == name) {
                        Some(p) if p.kind != kind => {
                            return Err(refuse(
                                codes::PARAMETER_KIND,
                                format!(
                                    "“{name}” parametresinin türü bu ölçü kısıtına uymuyor (açıya açı, öbürlerine uzunluk)."
                                ),
                                &format!("{at}.rule.parameter"),
                            ));
                        }
                        Some(p) => {
                            // A user parameter the dimension takes: its expression stays unless a value is given.
                            if c.reference {
                                p.expression.clear();
                            } else if c.value.is_some() {
                                p.expression = expression;
                            }
                        }
                        None => set.parameters.push(CadParameter {
                            name,
                            kind,
                            expression,
                            min: None,
                            max: None,
                            description: String::new(),
                        }),
                    }
                }
                let id = ConstraintId(Uuid::now_v7().into_bytes());
                set.constraints.push(Constraint {
                    id,
                    rule,
                    reference: c.reference,
                });
                added.push(id);
            }
        }
        ConstraintEdit::Remove { ids } => {
            let at: HashSet<usize> = positions(&set, ids, "edit.ids")?.into_iter().collect();
            let mut gone = Vec::new();
            let mut kept = Vec::new();
            for (i, c) in std::mem::take(&mut set.constraints).into_iter().enumerate() {
                if at.contains(&i) {
                    removed.push(c.id);
                    gone.push(c);
                } else {
                    kept.push(c);
                }
            }
            set.constraints = kept;
            drop_parameters(doc, &mut set, &gone);
        }
        ConstraintEdit::Mode { ids, reference } => {
            for (k, i) in positions(&set, ids, "edit.ids")?.into_iter().enumerate() {
                let rule = set.constraints[i].rule.clone();
                let Some(name) = rule.parameter().map(str::to_owned) else {
                    return Err(refuse(
                        codes::INVALID_CONSTRAINT,
                        "Yalnız ölçü kısıtı (uzaklık, açı, yarıçap, çap) sürücü ya da başvuru olur.".into(),
                        &format!("edit.ids[{k}]"),
                    ));
                };
                set.constraints[i].reference = *reference;
                if let Some(p) = set.parameters.iter_mut().find(|p| p.name == name) {
                    if *reference {
                        p.expression.clear();
                    } else if p.expression.is_empty() {
                        p.expression = measured_text(doc, &rule, p.kind);
                    }
                }
            }
        }
        ConstraintEdit::Flip { id } => {
            let i = positions(&set, std::slice::from_ref(id), "edit.id")?[0];
            let Some(other) = flipped(&set.constraints[i].rule) else {
                return Err(refuse(
                    codes::INVALID_CONSTRAINT,
                    format!(
                        "{} kısıtının tek çözümü var; çevrilen kısıtlar teğet, kenara uzaklık, yatay ya da düşey uzaklık ve açıdır.",
                        kentos_contracts::rule_label(&set.constraints[i].rule)
                    ),
                    "edit.id",
                ));
            };
            set.constraints[i].rule = other;
        }
        ConstraintEdit::Solve => {}
    }
    let solved = settle(doc, &set, &held, &added, input.allow_flip, stop)?;
    Ok(Planned {
        set,
        solved,
        added,
        removed,
    })
}

/// The set's parameters worked out and the drawing solved with it, refused as the ADR says (docs/adr/0335 §1, 0336 §1):
/// a parameter's value, the set's rules, a conflict (`layer_locked` when its objects are all locked), a singular
/// constraint, no convergence, a constraint `added` that repeats the others, a shape turned inside out unless
/// `allow_flip`. Shared by `cad.constraints.edit` and `cad.parameters.edit`.
pub(crate) fn settle(
    doc: &Document,
    set: &ConstraintSet,
    held: &[Held],
    added: &[ConstraintId],
    allow_flip: bool,
    stop: &dyn Fn() -> bool,
) -> Result<Solved, Stop> {
    // The parameters' values first: their own codes (expression, missing name, round, range).
    let values = parameter_values(doc, set).map_err(unsolvable)?;
    for c in &set.constraints {
        if let (
            ConstraintRule::Radius { parameter, .. } | ConstraintRule::Diameter { parameter, .. },
            false,
        ) = (&c.rule, c.reference)
            && values.get(parameter).is_some_and(|v| *v <= 0.0)
        {
            return Err(refuse(
                codes::PARAMETER_RANGE,
                format!(
                    "“{parameter}” = {}: yarıçap ve çap sıfırdan büyük olmalı.",
                    values[parameter]
                ),
                "edit",
            ));
        }
    }
    // The objects the constraints name that the drawing holds.
    let uids: HashSet<EntityId> = set
        .constraints
        .iter()
        .flat_map(|c| c.rule.elements().into_iter().map(|e| e.uid))
        .filter(|u| doc.slot_of(Uuid::from_bytes(u.0)).is_some())
        .collect();
    if let Some((at, words)) = constraints_problem(set, Some(&uids)) {
        return Err(refuse(
            codes::INVALID_CONSTRAINT,
            words,
            &format!("constraints.{at}"),
        ));
    }
    let solved = solve_set(doc, set, held, None, stop).map_err(unsolvable)?;
    let ids = |list: &[ConstraintId]| list.iter().map(|id| id.to_string()).collect::<Vec<_>>();
    let d = &solved.diagnosis;
    match d.status {
        SolveStatus::Inconsistent if all_locked(doc, set, &d.conflicting) => {
            return Err(refuse_ids(
                codes::LAYER_LOCKED,
                "Çelişen kısıtın nesneleri kilitli katmanlarda; kımıldamadan kısıta uyamıyorlar. Katmanın kilidini açın.".into(),
                "edit",
                ids(&d.conflicting),
            ));
        }
        SolveStatus::Inconsistent => {
            return Err(refuse_ids(
                codes::CONSTRAINT_CONFLICT,
                "Kısıtlar birlikte tutamıyor; hiçbir şey yazılmadı. Çelişen kısıtı kaldırın, değerini değiştirin ya da başvuru yapın.".into(),
                "edit",
                ids(&d.conflicting),
            ));
        }
        SolveStatus::Singular => {
            return Err(refuse_ids(
                codes::CONSTRAINT_SINGULAR,
                "Bir kısıt anlamını yitirdi (boysuz kenar, yarıçapsız daire); hiçbir şey yazılmadı.".into(),
                "edit",
                ids(&d.singular),
            ));
        }
        SolveStatus::NonConvergent => {
            return Err(refuse(
                codes::NOT_CONVERGED,
                "Çözücü adımları içinde bir çözüme ulaşamadı; hiçbir şey yazılmadı. Değeri adım adım değiştirin.".into(),
                "edit",
            ));
        }
        _ => {}
    }
    let repeats: Vec<ConstraintId> = d
        .redundant
        .iter()
        .copied()
        .filter(|id| added.contains(id))
        .collect();
    if !repeats.is_empty() {
        return Err(refuse_ids(
            codes::CONSTRAINT_REDUNDANT,
            "Eklenen kısıt ötekilerin zaten söylediğini tekrarlıyor; eklenmedi. Ölçü kısıtını başvuru olarak ekleyebilirsiniz.".into(),
            "edit.constraints",
            ids(&repeats),
        ));
    }
    if !solved.flipped.is_empty() && !allow_flip {
        let turned: Vec<String> = solved
            .flipped
            .iter()
            .filter_map(|s: &Slot| doc.uid(*s))
            .map(|u| u.to_string())
            .collect();
        return Err(refuse_ids(
            codes::SHAPE_FLIPPED,
            "Bu çözüm bir şekli ters çeviriyor (yönü, dönüşü ya da yayı); yazılmadı. Değeri adım adım değiştirin ya da allowFlip ile yine de yazın.".into(),
            "edit",
            turned,
        ));
    }
    Ok(solved)
}

/// Whether every object of every one of these constraints is on a locked layer.
fn all_locked(doc: &Document, set: &ConstraintSet, ids: &[ConstraintId]) -> bool {
    !ids.is_empty()
        && ids.iter().all(|id| {
            set.constraints
                .iter()
                .find(|c| c.id == *id)
                .is_some_and(|c| {
                    c.rule.elements().iter().all(|e| {
                        doc.slot_of(Uuid::from_bytes(e.uid.0))
                            .and_then(|s| doc.get(s))
                            .is_some_and(|x| doc.layers().is_locked(&x.base().layer_id))
                    })
                })
        })
}
