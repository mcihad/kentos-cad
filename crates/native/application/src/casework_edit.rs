//! `cad.casework.edit` v1 (docs/adr/0727 §6): a kitchen run laid anew by one module's own values (its width, its
//! door, what it holds) or by a whole recipe, its objects kept by their parts in it, or released (its objects stay, no
//! longer a run); one undo step. `cad.casework.query` v1 reads the runs: their module table and what keeps them.
//!
//! The checks, in order: the run (`casework_not_found`, path `run`), the new recipe's (the document's rule and the
//! command's: a module edit's refusals and warnings at `edit`, a recipe's at `edit.recipe.` before the recipe's
//! path), the expected revision.

use kentos_commands::{
    CaseworkBuilt, CaseworkChange, CaseworkDoor, CaseworkEdit, CaseworkInfo, CaseworkModuleKind,
    CaseworkModuleSetting, CaseworkPlan, CaseworkQuery, CaseworkRecord, CaseworkRun, CaseworkRuns,
    CommandResult, CommandWarning,
};
use kentos_domain::casework::{Laid, labels};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::casework_run::{Checked, built, check_recipe, plan_of, refused};
use crate::checks::{self, Stop, error};
use crate::codes;

fn not_found(key: &str) -> Stop {
    Stop::failed(error(
        codes::CASEWORK_NOT_FOUND,
        format!("“{key}” çizimdeki bir mutfak dizisinin ya da bir nesnesinin kimliği değil. Dizinin kimliğini ya da bir nesnesinin kalıcı kimliğini verin."),
        Some("run".into()),
    ))
}

/// A module edit as a recipe (docs/adr/0727 §6): the module's own values become `width` and `hand`; `holds`, when
/// given, what it holds (the sink, the cooktop, the dishwasher, or a base cabinet: none of them).
pub fn edited_module(
    recipe: &CaseworkRun,
    module: u32,
    width: Option<f64>,
    hand: Option<CaseworkDoor>,
    holds: Option<CaseworkModuleKind>,
) -> CaseworkRun {
    let mut out = recipe.clone();
    let mut own: Vec<CaseworkModuleSetting> = recipe
        .per_module
        .iter()
        .flatten()
        .filter(|m| m.module != module)
        .cloned()
        .collect();
    if width.is_some() || hand.is_some() {
        own.push(CaseworkModuleSetting {
            module,
            width,
            hand,
        });
    }
    own.sort_by_key(|m| m.module);
    out.per_module = (!own.is_empty()).then_some(own);
    if let Some(kind) = holds {
        for (slot, this) in [
            (&mut out.sink, CaseworkModuleKind::Sink),
            (&mut out.cooktop, CaseworkModuleKind::Cooktop),
            (&mut out.dishwasher, CaseworkModuleKind::Dishwasher),
        ] {
            if *slot == Some(module) && kind != this {
                *slot = None;
            }
            if kind == this {
                *slot = Some(module);
            }
        }
        if out.cooktop.is_none() {
            out.oven = None;
        }
    }
    out
}

/// What an edit leaves: the run, and its new recipe laid (none: released).
struct Edited {
    record: CaseworkRecord,
    laid: Option<(CaseworkRun, Checked)>,
}

fn check(doc: &Document, input: &CaseworkEdit) -> Result<Edited, Stop> {
    let record = doc
        .casework_of(&input.run)
        .cloned()
        .ok_or_else(|| not_found(&input.run))?;
    let laid = match &input.edit {
        CaseworkChange::Release => None,
        CaseworkChange::Module {
            module,
            width,
            hand,
            holds,
        } => {
            let recipe = edited_module(&record.recipe, *module, *width, *hand, *holds);
            let mut c = check_recipe(doc, &recipe, "").map_err(at_edit)?;
            // What it leaves is the edit's: its warnings at it, as its refusals.
            for w in &mut c.warnings {
                w.path = Some("edit".into());
            }
            Some((recipe, c))
        }
        CaseworkChange::Recipe { recipe } => {
            let recipe = CaseworkRun {
                expected_revision: None,
                ..(**recipe).clone()
            };
            let c = check_recipe(doc, &recipe, "edit.recipe.")?;
            Some((recipe, c))
        }
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Edited { record, laid })
}

/// A module edit's refusal: its new recipe's, at the edit.
fn at_edit(stop: Stop) -> Stop {
    match stop {
        Stop::Failed(mut e) => {
            e.path = Some("edit".into());
            Stop::Failed(e)
        }
        other => other,
    }
}

fn released_plan(doc: &Document) -> CaseworkPlan {
    CaseworkPlan {
        entities: Vec::new(),
        blocks: Vec::new(),
        modules: Vec::new(),
        legs: Vec::new(),
        fridge: None,
        filler: 0.0,
        revision: doc.revision().to_string(),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &CaseworkEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(e) => CommandResult::Completed {
            output: (),
            warnings: e.laid.map(|(_, c)| c.warnings).unwrap_or_default(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: the run's objects laid anew (`id` 0); none for a release.
pub fn plan(cx: &ExecutionContext<'_>, input: &CaseworkEdit) -> CommandResult<CaseworkPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document: what the desktop previews.
pub fn plan_in(doc: &Document, input: &CaseworkEdit) -> CommandResult<CaseworkPlan> {
    match check(doc, input) {
        Ok(Edited {
            laid: Some((_, c)), ..
        }) => CommandResult::Completed {
            output: plan_of(doc, c.laid),
            warnings: c.warnings,
        },
        Ok(Edited { laid: None, .. }) => CommandResult::Completed {
            output: released_plan(doc),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes: the run laid anew (“Mutfak dizisini düzenle”) or released (“Mutfak dizisini
/// bırak”).
pub fn execute(cx: &mut ExecutionContext<'_>, input: CaseworkEdit) -> CommandResult<CaseworkBuilt> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_CASEWORK_EDIT,
        kentos_commands::CAD_CASEWORK_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: CaseworkEdit) -> CommandResult<CaseworkBuilt> {
    let edited = match check(cx.doc, &input) {
        Ok(e) => e,
        Err(stop) => return stop.into(),
    };
    let record = edited.record;
    let Some((recipe, c)) = edited.laid else {
        let written: Vec<(Uuid, kentos_domain::Slot)> = record
            .members
            .iter()
            .filter_map(|m| {
                let uid = Uuid::from_bytes(m.uid.0);
                cx.doc.slot_of(uid).map(|slot| (uid, slot))
            })
            .collect();
        cx.doc.release_casework(record.id);
        return CommandResult::Completed {
            output: CaseworkBuilt {
                run: record.id.to_text(),
                uids: written.iter().map(|(u, _)| u.to_string()).collect(),
                ids: written.iter().map(|(_, s)| s.0).collect(),
                blocks: Vec::new(),
                modules: Vec::new(),
                legs: Vec::new(),
                fridge: None,
                filler: 0.0,
                released: true,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        };
    };
    let Checked { laid, warnings } = c;
    let written = cx.doc.transact(labels::CASEWORK_EDIT, |doc| {
        doc.write_casework(Some(&record), recipe.clone(), &laid, labels::CASEWORK_EDIT)
            .map_err(refused)
    });
    let (run, written) = match written {
        Ok(w) => w,
        Err(stop) => return stop.into(),
    };
    CommandResult::Completed {
        output: built(cx.doc, run, &written, laid),
        warnings,
    }
}

/// `cad.casework.query` v1: the runs (or the one named), each laid from its recipe for its module table.
pub mod query {
    use super::*;

    fn info(doc: &Document, r: &CaseworkRecord) -> CaseworkInfo {
        let library = |_: &str| None;
        let mut fresh = crate::casework_run::fresh_id;
        let laid: Option<Laid> = doc.lay_casework(&r.recipe, &library, &mut fresh).ok();
        let missing = r
            .members
            .iter()
            .filter(|m| doc.slot_of(Uuid::from_bytes(m.uid.0)).is_none())
            .map(|m| m.uid.to_text())
            .collect();
        let (legs, modules, fridge, filler) = match laid {
            Some(l) => (l.legs, l.modules, l.fridge, l.filler),
            None => (Vec::new(), Vec::new(), None, 0.0),
        };
        CaseworkInfo {
            id: r.id.to_text(),
            recipe: r.recipe.clone(),
            legs,
            modules,
            fridge,
            filler,
            members: r.members.clone(),
            missing,
            broken: r.broken,
        }
    }

    fn read(doc: &Document, input: &CaseworkQuery) -> Result<CaseworkRuns, Stop> {
        let runs = match &input.run {
            Some(key) => vec![info(doc, doc.casework_of(key).ok_or_else(|| not_found(key))?)],
            None => doc.casework().iter().map(|r| info(doc, r)).collect(),
        };
        Ok(CaseworkRuns {
            runs,
            revision: doc.revision().to_string(),
        })
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &CaseworkQuery) -> CommandResult<()> {
        match read(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &CaseworkQuery) -> CommandResult<CaseworkRuns> {
        plan_in(cx.doc, input)
    }

    /// The answer without a context.
    pub fn plan_in(doc: &Document, input: &CaseworkQuery) -> CommandResult<CaseworkRuns> {
        match read(doc, input) {
            Ok(output) => CommandResult::Completed {
                output,
                warnings: Vec::<CommandWarning>::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: CaseworkQuery) -> CommandResult<CaseworkRuns> {
        plan_in(cx.doc, &input)
    }
}
