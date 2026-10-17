//! `cad.constraints.query` v1 (docs/adr/0335 §1; AutoCAD's constraint bar and parameter manager, read only): the
//! drawing's constraints with their Turkish names, what each dimension measures now and whether each holds, the
//! parameters and the solver's diagnosis of the drawing as it is; optionally only the constraints naming some objects.
//!
//! Whether a constraint holds is the solver's own test with every object held: a held constraint is only checked, so
//! the ones that do not hold come back as conflicting (`constraint_solve::holding`). The diagnosis is of the whole set,
//! solved without writing anything.

use std::collections::HashSet;

use kentos_contracts::{
    CommandResult, ConstraintsListed, ConstraintsQuery, EntityId, ListedConstraint, rule_label,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::constraint_solve::{Held, measured_now, solve_set};

/// Checks `input`, writing nothing.
pub fn validate(_cx: &ExecutionContext<'_>, input: &ConstraintsQuery) -> CommandResult<()> {
    match check(input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

fn check(input: &ConstraintsQuery) -> Result<HashSet<EntityId>, Stop> {
    input
        .uids
        .iter()
        .enumerate()
        .map(|(i, text)| {
            Uuid::parse_str(text).ok().filter(|_| checks::is_uid_text(text)).map(|u| EntityId(u.into_bytes())).ok_or_else(|| {
                Stop::failed(checks::error(
                    codes::INVALID_UID,
                    format!("“{text}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
                    Some(format!("uids[{i}]")),
                ))
            })
        })
        .collect()
}

/// The same as [`execute`]: a query writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ConstraintsQuery,
) -> CommandResult<ConstraintsListed> {
    read(cx.doc, input)
}

/// The constraints, the parameters and the diagnosis; writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ConstraintsQuery,
) -> CommandResult<ConstraintsListed> {
    read(cx.doc, &input)
}

/// [`execute`] over a document read in place.
pub fn read(doc: &Document, input: &ConstraintsQuery) -> CommandResult<ConstraintsListed> {
    let only = match check(input) {
        Ok(only) => only,
        Err(stop) => return stop.into(),
    };
    let set = doc.constraints();
    // Every object held: the constraints that do not hold now come back as conflicting.
    let held: Vec<Held> = set
        .constraints
        .iter()
        .flat_map(|c| {
            c.rule.elements().into_iter().map(|e| Held {
                uid: e.uid,
                element: None,
            })
        })
        .collect();
    let now = solve_set(doc, set, &held, None, &|| false);
    let solved = solve_set(doc, set, &[], None, &|| false);
    let (now, solved) = match (now, solved) {
        (Ok(now), Ok(solved)) => (now, solved),
        (Err(u), _) | (_, Err(u)) => {
            return CommandResult::Failed {
                error: checks::error(u.code, u.words, u.path),
            };
        }
    };
    let failing: HashSet<_> = now
        .diagnosis
        .conflicting
        .iter()
        .chain(&now.diagnosis.broken)
        .chain(&now.diagnosis.singular)
        .copied()
        .collect();
    let constraints = set
        .constraints
        .iter()
        .filter(|c| only.is_empty() || c.rule.elements().iter().any(|e| only.contains(&e.uid)))
        .map(|c| ListedConstraint {
            label: rule_label(&c.rule).into(),
            measured: c.rule.parameter().and_then(|_| measured_now(doc, &c.rule)),
            holds: !failing.contains(&c.id),
            constraint: c.clone(),
        })
        .collect();
    CommandResult::Completed {
        output: ConstraintsListed {
            constraints,
            parameters: set.parameters.clone(),
            diagnosis: solved.diagnosis,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
