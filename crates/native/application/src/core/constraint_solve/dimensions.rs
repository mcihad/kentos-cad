//! New expressions are dimension checked; old file values are audited, never silently migrated (ADR 0840).
use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{ConstraintSet, check_param_expr_dimension, parse_param_expr};
use kentos_domain::Document;

use super::{Unsolvable, parameter_values, units};

/// Read-only, per-parameter physical-unit issues. Values remain in the existing project units.
pub fn parameter_dimension_issues(
    doc: &Document,
    set: &ConstraintSet,
    values: &BTreeMap<String, f64>,
) -> BTreeMap<String, String> {
    let kinds: BTreeMap<_, _> = set
        .parameters
        .iter()
        .map(|p| (p.name.as_str(), p.kind))
        .collect();
    set.parameters
        .iter()
        .filter(|p| !p.expression.is_empty())
        .filter_map(|p| {
            let expr = parse_param_expr(&p.expression).ok()?;
            let error = check_param_expr_dimension(&expr, p.kind, units(doc), &|n| {
                Some((*kinds.get(n)?, *values.get(n)?))
            })
            .err()?;
            Some((p.name.clone(), error))
        })
        .collect()
}

/// Refuse authored errors and newly introduced errors in dependent expressions (e.g. a changed power).
/// `origins` follows rename/remove/add identity within this edit, so renaming an old expression never becomes
/// an accidental migration. Already invalid, untouched legacy expressions stay readable and remain in the audit.
pub(crate) fn check_dimension_writes(
    doc: &Document,
    set: &ConstraintSet,
    authored: &BTreeSet<String>,
    origins: &BTreeMap<String, String>,
    stop: &dyn Fn() -> bool,
) -> Result<(), crate::checks::Stop> {
    use crate::checks::Stop;
    if stop() {
        return Err(Stop::Cancelled);
    }
    let values = parameter_values(doc, set).map_err(crate::constraints_edit::unsolvable)?;
    let issues = parameter_dimension_issues(doc, set, &values);
    if issues.is_empty() {
        return Ok(());
    }
    let old = doc.constraints();
    let old_values = parameter_values(doc, old).unwrap_or_default();
    let old_issues = parameter_dimension_issues(doc, old, &old_values);
    for (i, p) in set.parameters.iter().enumerate() {
        if stop() {
            return Err(Stop::Cancelled);
        }
        let Some(words) = issues.get(&p.name) else {
            continue;
        };
        let legacy = origins
            .get(&p.name)
            .and_then(|n| old.parameter(n).map(|p| (n, p)))
            .is_some_and(|(n, was)| was.kind == p.kind && old_issues.contains_key(n));
        if !authored.contains(&p.name) && legacy {
            continue;
        }
        return Err(crate::constraints_edit::unsolvable(Unsolvable {
            code: "parameter_kind",
            words: format!("“{}” ifadesinde birim uyuşmazlığı: {words}", p.name),
            path: Some(format!("parameters[{i}].expression")),
        }));
    }
    Ok(())
}
