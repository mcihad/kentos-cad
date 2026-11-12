//! Constraints copied with their objects (docs/adr/0337 §3): a copy (`cad.entities.transform`'s copy,
//! `cad.entities.array`) takes the constraints among the objects it copies, to each copy, in the same step.
//!
//! - A constraint is copied when every object it names is among a copy's sources; one naming an object not copied
//!   is not, and is counted.
//! - The copy is a new constraint (a new id); a dimension's copy a new name (the next free d…, ang…, rad…, dia…) and,
//!   driving, the value its copy measures (a scaled copy's scaled): the copies are each their own.
//! - A copy the copied objects do not meet (a turned copy's horizontal, a mirrored copy's side) is not copied, and is
//!   counted. A fix's copy holds its copy where it is.

use std::collections::HashMap;

use kentos_commands::{
    CadParameter, CommandWarning, ConstraintId, ConstraintRule, EntityId, ParamUnits,
};
use kentos_domain::{Document, Uuid};
use serde_json::Value;

use crate::checks::{self, Stop};
use crate::codes;
use crate::constraint_solve::{kind_of, measured_now, next_name, unmet};

/// The rule with its objects' ids read through `map`.
fn mapped(rule: &ConstraintRule, map: &HashMap<EntityId, EntityId>) -> Option<ConstraintRule> {
    fn walk(v: &mut Value, map: &HashMap<String, String>) {
        match v {
            Value::Object(fields) => {
                for (key, field) in fields.iter_mut() {
                    match field {
                        Value::String(text) if key == "uid" => {
                            if let Some(to) = map.get(text.as_str()) {
                                to.clone_into(text);
                            }
                        }
                        other => walk(other, map),
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|i| walk(i, map)),
            _ => {}
        }
    }
    let texts: HashMap<String, String> = map
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let mut value = serde_json::to_value(rule).ok()?;
    walk(&mut value, &texts);
    serde_json::from_value(value).ok()
}

/// Copies the constraints among each group's sources (source id → copy id) to its copies; the warning when some were
/// not copied.
pub(crate) fn copy_constraints(
    doc: &mut Document,
    groups: &[HashMap<EntityId, EntityId>],
    label: &str,
) -> Result<Option<CommandWarning>, Stop> {
    let original = doc.constraints().clone();
    if original.constraints.is_empty() || groups.is_empty() {
        return Ok(None);
    }
    let units = ParamUnits::of(doc.settings());
    let mut next = original.clone();
    let mut crossing = 0usize;
    for group in groups {
        for c in &original.constraints {
            let uids: Vec<EntityId> = c.rule.elements().into_iter().map(|e| e.uid).collect();
            let inside = uids.iter().filter(|u| group.contains_key(u)).count();
            if inside == 0 {
                continue;
            }
            if inside < uids.len() {
                crossing += 1;
                continue;
            }
            let Some(mut rule) = mapped(&c.rule, group) else {
                crossing += 1;
                continue;
            };
            if let ConstraintRule::Distance { parameter, .. }
            | ConstraintRule::Angle { parameter, .. }
            | ConstraintRule::Radius { parameter, .. }
            | ConstraintRule::Diameter { parameter, .. } = &mut rule
            {
                *parameter = next_name(&next.parameters, &c.rule);
            }
            if let Some(name) = rule.parameter() {
                let kind = kind_of(&rule);
                let expression = if c.reference {
                    String::new()
                } else {
                    measured_now(doc, &rule)
                        .map(|v| {
                            kentos_domain::constraints::value_text(
                                kentos_domain::constraints::in_parameter_unit(units, kind, v),
                            )
                        })
                        .unwrap_or_default()
                };
                next.parameters.push(CadParameter {
                    name: name.to_owned(),
                    kind,
                    expression,
                    min: None,
                    max: None,
                    description: String::new(),
                });
            }
            let mut copy = c.clone();
            copy.id = ConstraintId(Uuid::now_v7().into_bytes());
            copy.rule = rule;
            next.constraints.push(copy);
        }
    }
    let added: Vec<usize> = (original.constraints.len()..next.constraints.len()).collect();
    let bad = unmet(doc, &next, Some(&added), None);
    let next = if bad.is_empty() {
        next
    } else {
        doc.constraints_without(&next, &bad)
    };
    if next != original {
        doc.set_constraints(next, label).map_err(|why| {
            Stop::failed(checks::error(
                codes::INVALID_CONSTRAINT,
                why.to_string(),
                None,
            ))
        })?;
    }
    let missed = crossing + bad.len();
    Ok((missed > 0).then(|| not_copied(missed)))
}

/// The warning of constraints a copy does not take.
pub(crate) fn not_copied(n: usize) -> CommandWarning {
    CommandWarning {
        code: codes::CONSTRAINTS_NOT_COPIED.into(),
        message: format!(
            "{n} kısıt kopyalanmadı: kopyalanmayan bir nesneyi anıyor ya da kopya ona uymuyor."
        ),
        path: None,
    }
}

/// A copy's group: each source's id onto its copy's, in the order both were written.
pub(crate) fn group_of(
    doc: &Document,
    sources: impl Iterator<Item = kentos_domain::Slot>,
    copies: impl Iterator<Item = kentos_domain::Slot>,
) -> HashMap<EntityId, EntityId> {
    let id = |slot| doc.uid(slot).map(|u| EntityId(u.into_bytes()));
    sources
        .zip(copies)
        .filter_map(|(s, c)| Some((id(s)?, id(c)?)))
        .collect()
}

/// A copy's group by the sources' ids.
pub(crate) fn group_by_ids(
    doc: &Document,
    sources: impl Iterator<Item = EntityId>,
    copies: impl Iterator<Item = kentos_domain::Slot>,
) -> HashMap<EntityId, EntityId> {
    sources
        .zip(copies)
        .filter_map(|(s, c)| Some((s, EntityId(doc.uid(c)?.into_bytes()))))
        .collect()
}

/// Whether any of these objects is named by a constraint (a copy that takes none says so).
pub(crate) fn any_constrained(doc: &Document, uids: &[EntityId]) -> usize {
    doc.constraints()
        .constraints
        .iter()
        .filter(|c| c.rule.elements().iter().any(|e| uids.contains(&e.uid)))
        .count()
}
