//! Constraints in other commands' edits (docs/adr/0337 §1, §2): run by `call::recorded` after a command's write, in
//! the same step, so what a write does to constrained objects and what the constraints make of it are one undo step.
//!
//! - First the constraints the write left without their parts go (the domain's rule: a deleted object, an indexed
//!   part of an object whose layout changed, a part an object no longer has).
//! - Then, when the write changed objects the constraints name: a cutting command (trim, break, join …) never solves,
//!   the constraints it leaves unmet go; a relaxed call does the same with its own word; any other write is followed
//!   by a solve that holds the parts the write moved and a fix where it was, writing what the solve moves, or nothing
//!   at all when it cannot hold (`constraint_conflict` …).
//! - A write that changed the constraints itself (the constraint and parameter commands, a copy) is left as it is.

use std::collections::{HashMap, HashSet};

use kentos_commands::{CommandWarning, EntityId, SolveStatus};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::constraints::{Status, moved_parts};

use crate::checks::{self, Stop};
use crate::codes;
use crate::constraint_solve::{Held, solve_set, unmet};
use crate::geometry::shape;

/// The commands that cut, join or turn objects round: their parts are not the ones the constraints were written for,
/// so they never solve; the constraints they leave unmet go.
pub const CUTTING: &[&str] = &[
    kentos_commands::CAD_ENTITIES_TRIM,
    kentos_commands::CAD_ENTITIES_EXTEND,
    kentos_commands::CAD_ENTITIES_BREAK,
    kentos_commands::CAD_ENTITIES_JOIN,
    kentos_commands::CAD_ENTITIES_DIVIDE,
    kentos_commands::CAD_ENTITIES_EXPLODE,
    kentos_commands::CAD_ENTITIES_FILLET,
    kentos_commands::CAD_ENTITIES_CHAMFER,
    kentos_commands::CAD_ENTITIES_CLEANUP,
    kentos_commands::CAD_ENTITIES_REVERSE,
    kentos_commands::CAD_UNITS_CHANGE,
];

/// What following a write did.
#[derive(Debug, Default)]
pub struct Followed {
    pub warnings: Vec<CommandWarning>,
    /// Whether it relaxed the constraints (the call's journal row says so).
    pub relaxed: bool,
}

fn warning(code: &str, message: String) -> CommandWarning {
    CommandWarning {
        code: code.into(),
        message,
        path: None,
    }
}

fn refused(code: &str, message: &str, ids: Vec<String>) -> Stop {
    let mut error = checks::error(code, message.into(), None);
    error.uids = ids;
    Stop::failed(error)
}

/// Follows the open step's write: see the module. `relax`: the call asked for it.
pub(crate) fn follow(
    doc: &mut Document,
    command: &str,
    relax: bool,
    stop: &dyn Fn() -> bool,
) -> Result<Followed, Stop> {
    let mut out = Followed::default();
    if doc.constraints().constraints.is_empty() || doc.step_touches_constraints() {
        return Ok(out);
    }
    let befores = doc.step_befores();
    let mut removed = doc.settle_constraints_now();
    let set = doc.constraints().clone();
    let named: HashSet<EntityId> = set
        .constraints
        .iter()
        .flat_map(|c| c.rule.elements().into_iter().map(|e| e.uid))
        .collect();
    let edited: Vec<(EntityId, Shape)> = befores
        .iter()
        .filter_map(|(slot, before)| {
            let uid = EntityId(doc.uid(*slot)?.into_bytes());
            named.contains(&uid).then(|| (uid, shape(before)))
        })
        .collect();
    if !edited.is_empty() {
        let fix: HashMap<EntityId, Shape> = edited.iter().cloned().collect();
        let cutting = CUTTING.contains(&command);
        let relax = !cutting && (relax || doc.take_relax());
        if cutting || relax {
            let bad = unmet(doc, &set, None, Some(&fix));
            if !bad.is_empty() {
                let next = doc.constraints_without(&set, &bad);
                doc.set_constraints(next, "").map_err(|why| {
                    refused(codes::INVALID_CONSTRAINT, &why.to_string(), Vec::new())
                })?;
                if cutting {
                    removed += bad.len();
                } else {
                    out.relaxed = true;
                    out.warnings.push(warning(
                        codes::CONSTRAINTS_RELAXED,
                        format!(
                            "Kısıtlar gevşetildi: düzenlemenin bozduğu {} kısıt kaldırıldı.",
                            bad.len()
                        ),
                    ));
                }
            }
        } else {
            let mut held = Vec::new();
            for (uid, before) in &edited {
                let Some(now) = doc
                    .slot_of(kentos_domain::Uuid::from_bytes(uid.0))
                    .and_then(|s| doc.get(s))
                    .map(shape)
                else {
                    continue;
                };
                for part in moved_parts(before, &now).into_iter().flatten() {
                    held.push(Held {
                        uid: *uid,
                        element: Some(kentos_domain::cad_element(&part)),
                    });
                }
            }
            let solved = solve_set(doc, &set, &held, Some(&fix), stop).map_err(|u| {
                Stop::failed(checks::error(codes::PARAMETER_RANGE, u.words, u.path))
            })?;
            if solved.outcome.status == Status::Cancelled {
                return Err(Stop::Cancelled);
            }
            let ids = |list: &[kentos_commands::ConstraintId]| {
                list.iter().map(ToString::to_string).collect::<Vec<_>>()
            };
            let d = &solved.diagnosis;
            match d.status {
                SolveStatus::Inconsistent => {
                    let words = match &solved.arch_why {
                        // The architectural bonds' own reason first (docs/adr/0770 §4), then the bond whose
                        // taking away lets the edit through (docs/adr/0780 §1).
                        Some(why) => format!(
                            "{why}; hiçbir şey yazılmadı. {}",
                            crate::constraint_solve::bonds_words(&solved.arch_bonds)
                        ),
                        None => "Düzenleme kısıtlarla birlikte tutmuyor; hiçbir şey yazılmadı. Çelişen kısıtı kaldırın ya da kısıtları gevşeterek düzenleyin.".to_owned(),
                    };
                    let mut error = checks::error(codes::CONSTRAINT_CONFLICT, words, None);
                    error.uids = ids(&d.conflicting);
                    error.bonds = solved.arch_bonds.clone();
                    return Err(Stop::failed(error));
                }
                SolveStatus::Singular => {
                    return Err(refused(
                        codes::CONSTRAINT_SINGULAR,
                        "Düzenleme bir kısıtı anlamsız bırakıyor (boysuz kenar, yarıçapsız daire); hiçbir şey yazılmadı.",
                        ids(&d.singular),
                    ));
                }
                SolveStatus::NonConvergent => {
                    return Err(refused(
                        codes::NOT_CONVERGED,
                        "Kısıtlar düzenlemeyi izleyemedi; hiçbir şey yazılmadı. Daha küçük bir adımla deneyin ya da kısıtları gevşeterek düzenleyin.",
                        Vec::new(),
                    ));
                }
                _ => {}
            }
            for (slot, entity) in solved.changed {
                doc.update(slot, entity);
            }
        }
    }
    if removed > 0 {
        out.warnings.insert(
            0,
            warning(
                codes::CONSTRAINTS_REMOVED,
                format!(
                    "{removed} kısıt kaldırıldı: andıkları nesne silindi, biçimi değişti ya da düzenleme onları bozdu."
                ),
            ),
        );
    }
    Ok(out)
}

/// What execute would be refused with by the architectural bonds (docs/adr/0790 §1): the command run whole on a
/// reading copy of the drawing (its step, the constraints following in it), so that a plan says the refusal execute
/// would give, with its bonds in the way. Asked only when an architectural bond names one of `uids` (the objects the
/// write changes; the bonds follow only those); none when the copy takes the write, or refuses it for another reason
/// (execute says that one).
pub(crate) fn bonds_refusal<'u, T>(
    doc: &Document,
    uids: impl IntoIterator<Item = &'u str>,
    run: impl FnOnce(&mut crate::ExecutionContext<'_>) -> kentos_commands::CommandResult<T>,
) -> Option<kentos_commands::CommandError> {
    let named: HashSet<EntityId> = doc
        .constraints()
        .constraints
        .iter()
        .filter(|c| kentos_commands::rule_is_architectural(&c.rule))
        .flat_map(|c| c.rule.elements().into_iter().map(|e| e.uid))
        .collect();
    if named.is_empty()
        || !uids.into_iter().any(|u| {
            kentos_domain::Uuid::parse_str(u).is_ok_and(|u| named.contains(&EntityId(u.into_bytes())))
        })
    {
        return None;
    }
    let mut copy = doc.reading_copy();
    match run(&mut crate::ExecutionContext::new(&mut copy)) {
        kentos_commands::CommandResult::Failed { error }
            if error.code == codes::CONSTRAINT_CONFLICT && !error.bonds.is_empty() =>
        {
            Some(error)
        }
        _ => None,
    }
}
