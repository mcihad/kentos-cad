//! `cad.constraints.query` v1 (docs/adr/0335 §1; AutoCAD's constraint bar and parameter manager, read only): the
//! drawing's constraints with their Turkish names, what each dimension measures now and whether each holds, the
//! parameters and the solver's diagnosis of the drawing as it is; optionally only the constraints naming some objects,
//! only some constraints by id or only the architectural bonds, each with the value it keeps (docs/adr/0780 §2).
//!
//! Whether a constraint holds is the solver's own test with every object held: a held constraint is only checked, so
//! the ones that do not hold come back as conflicting (`constraint_solve::holding`). The diagnosis is of the whole set,
//! solved without writing anything.

use std::collections::HashSet;

use kentos_commands::{
    CommandResult, ConstraintId, ConstraintsListed, ConstraintsQuery, EntityId, ListedConstraint,
    rule_is_architectural, rule_label,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::constraint_solve::{Held, measured_now, metres, solve_set};

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

/// What a query asks for: the objects, the constraints by id.
struct Asked {
    objects: HashSet<EntityId>,
    ids: HashSet<ConstraintId>,
}

fn check(input: &ConstraintsQuery) -> Result<Asked, Stop> {
    let uid = |field: &str, i: usize, text: &str| {
        Uuid::parse_str(text)
            .ok()
            .filter(|_| checks::is_uid_text(text))
            .ok_or_else(|| {
                Stop::failed(checks::error(
                    codes::INVALID_UID,
                    format!("“{text}” geçerli bir kimlik değil; kimlik küçük harfli, tireli bir UUID'dir."),
                    Some(format!("{field}[{i}]")),
                ))
            })
    };
    Ok(Asked {
        objects: input
            .uids
            .iter()
            .enumerate()
            .map(|(i, t)| uid("uids", i, t).map(|u| EntityId(u.into_bytes())))
            .collect::<Result<_, _>>()?,
        ids: input
            .ids
            .iter()
            .enumerate()
            .map(|(i, t)| uid("ids", i, t).map(|u| ConstraintId(u.into_bytes())))
            .collect::<Result<_, _>>()?,
    })
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
    let asked = match check(input) {
        Ok(asked) => asked,
        Err(stop) => return stop.into(),
    };
    let set = doc.constraints();
    // Every asked id is a constraint of the drawing (docs/adr/0780 §2).
    if let Some((i, text)) = input.ids.iter().enumerate().find(|(_, t)| {
        !set.constraints
            .iter()
            .any(|c| c.id.to_string() == **t)
    }) {
        return CommandResult::Failed {
            error: checks::error(
                codes::CONSTRAINT_NOT_FOUND,
                format!("“{text}” kimlikli kısıt çizimde yok; kaldırılmış olabilir. Kısıtları kimliksiz sorun."),
                Some(format!("ids[{i}]")),
            ),
        };
    }
    let kept = metres(doc, set);
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
    // An architectural bond is measured on its own (docs/adr/0780 §2): every one that does not hold now, not only the
    // first row the held solve finds contradicting.
    let unmet: HashSet<ConstraintId> = crate::arch_bonds::unmet(doc, set, &kept)
        .into_iter()
        .filter_map(|k| set.constraints.get(k).map(|c| c.id))
        .collect();
    let constraints = set
        .constraints
        .iter()
        .filter(|c| {
            asked.objects.is_empty()
                || c.rule.elements().iter().any(|e| asked.objects.contains(&e.uid))
        })
        .filter(|c| asked.ids.is_empty() || asked.ids.contains(&c.id))
        .filter(|c| !input.architectural || rule_is_architectural(&c.rule))
        .map(|c| ListedConstraint {
            label: rule_label(&c.rule).into(),
            measured: c.rule.parameter().and_then(|_| measured_now(doc, &c.rule)),
            kept: c
                .rule
                .parameter()
                .filter(|_| !c.reference)
                .and_then(|p| kept.get(p).copied()),
            holds: if rule_is_architectural(&c.rule) {
                !unmet.contains(&c.id)
            } else {
                !failing.contains(&c.id)
            },
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
