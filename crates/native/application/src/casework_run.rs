//! `cad.casework.run` v1 (“Mutfak dizisi”, docs/adr/0706, 0727): base cabinets along a straight, L or U path, their
//! sink, cooktop and appliances, a countertop cut for the sink and the cooktop and, if asked, wall cabinets above, as
//! one undo step, kept as the drawing's run. The tool previews the plan and writes through `execute`; Python and an
//! agent give the same path and values. How a recipe lays its objects is the document's one rule
//! (`Document::lay_casework`), which the run's edits and the walls' following use too. The shared cases are
//! `fixtures/commands/v1/cad.casework.run.json` (`scripts/fixtures/casework_cases.py`).
//!
//! The checks, in order (the first that fails answers): 1. the numbers (`not_finite`); 2. the run
//! (`invalid_casework`, the document's rule); 3. the line weight; 4. the expected revision; 5. the layer.

use kentos_commands::{
    BlockDefinition, BlockId, CaseworkBuilt, CaseworkPlan, CaseworkRun, CommandResult, CommandWarning,
};
use kentos_domain::casework::{CaseworkStop, CaseworkWarning, Laid, labels};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = labels::CASEWORK_RUN;

/// A run's refusal as a command's.
pub(crate) fn stop_of(s: CaseworkStop) -> Stop {
    Stop::failed(error(s.code, s.message, Some(s.path)))
}

/// A run's warning as a command's.
pub(crate) fn warning_of(w: CaseworkWarning) -> CommandWarning {
    CommandWarning {
        code: w.code.into(),
        message: w.message,
        path: Some(w.path),
    }
}

/// KentOS's library's block of that name: what a run brings when the drawing has none.
pub(crate) fn library(name: &str) -> Option<BlockDefinition> {
    crate::library_symbols::library()
        .iter()
        .find(|b| b.name == name)
        .cloned()
}

pub(crate) fn fresh_id() -> BlockId {
    BlockId(*Uuid::now_v7().as_bytes())
}

/// A laid run and its warnings, the layer's first.
pub(crate) struct Checked {
    pub laid: Laid,
    pub warnings: Vec<CommandWarning>,
}

/// The run `recipe` lays and the command's own checks after the document's rule (`path` before a recipe's paths:
/// none for the run itself).
pub(crate) fn check_recipe(doc: &Document, recipe: &CaseworkRun, prefix: &str) -> Result<Checked, Stop> {
    let mut fresh = fresh_id;
    let laid = doc
        .lay_casework(recipe, &library, &mut fresh)
        .map_err(|s| {
            stop_of(CaseworkStop {
                path: format!("{prefix}{}", s.path),
                ..s
            })
        })?;
    let at = |stop: Stop| match stop {
        Stop::Failed(mut e) => {
            e.path = e.path.map(|p| format!("{prefix}{p}"));
            Stop::Failed(e)
        }
        other => other,
    };
    checks::line_weight(recipe.line_weight, "lineWeight").map_err(at)?;
    let mut warnings = checks::layer(doc, &recipe.layer_id).map_err(at)?;
    warnings.extend(laid.warnings.iter().cloned().map(warning_of));
    for w in &mut warnings {
        w.path = w.path.take().map(|p| format!("{prefix}{p}"));
    }
    Ok(Checked { laid, warnings })
}

fn check(doc: &Document, input: &CaseworkRun) -> Result<Checked, Stop> {
    let mut fresh = fresh_id;
    let laid = doc
        .lay_casework(input, &library, &mut fresh)
        .map_err(stop_of)?;
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    warnings.extend(laid.warnings.iter().cloned().map(warning_of));
    Ok(Checked { laid, warnings })
}

/// A laid run as a plan: its objects (`id` 0) in order, the blocks it brings, its modules, legs, fridge and fillers.
pub(crate) fn plan_of(doc: &Document, laid: Laid) -> CaseworkPlan {
    CaseworkPlan {
        entities: laid.members.into_iter().map(|m| m.entity).collect(),
        blocks: laid.added.iter().map(|b| b.name.clone()).collect(),
        modules: laid.modules,
        legs: laid.legs,
        fridge: laid.fridge,
        filler: laid.filler,
        revision: doc.revision().to_string(),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &CaseworkRun) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &CaseworkRun) -> CommandResult<CaseworkPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document: what the tool draws as its preview.
pub fn plan_in(doc: &Document, input: &CaseworkRun) -> CommandResult<CaseworkPlan> {
    match check(doc, input) {
        Ok(c) => CommandResult::Completed {
            output: plan_of(doc, c.laid),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The run as the tool previews it: the plan and the shapes its objects draw in the drawing, its inserts opened
/// through their blocks (the drawing's, or the library's it would bring) with their widths; each with whether it
/// draws dashed (a wall cabinet's).
pub struct CaseworkDrawn {
    pub plan: CaseworkPlan,
    pub shapes: Vec<(kentos_geometry_core::entity::Shape, bool)>,
}

/// The shapes a laid run's objects draw, its inserts opened through their blocks.
pub(crate) fn shapes_of(doc: &Document, laid: &Laid) -> Vec<(kentos_geometry_core::entity::Shape, bool)> {
    let mut defs: Vec<std::sync::Arc<BlockDefinition>> = doc.blocks().to_vec();
    defs.extend(laid.added.iter().cloned().map(std::sync::Arc::new));
    let blocks = crate::blocks::core_blocks(&defs);
    let mut shapes = Vec::new();
    for m in &laid.members {
        let shape = crate::geometry::shape(&m.entity);
        if matches!(m.entity, kentos_commands::Entity::Insert(_)) {
            shapes.extend(
                blocks
                    .expand(&shape)
                    .into_iter()
                    .filter(|piece| !kentos_geometry_core::block::shows_nothing(&piece.shape))
                    .map(|piece| {
                        let dashed = piece.stroke.line_type.as_deref() == Some("dashed");
                        (piece.shape, dashed)
                    }),
            );
        } else {
            shapes.push((shape, false));
        }
    }
    shapes
}

/// [`plan_in`] with what it draws: the tool's preview; writes nothing.
pub fn drawn_in(doc: &Document, input: &CaseworkRun) -> CommandResult<CaseworkDrawn> {
    let c = match check(doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let shapes = shapes_of(doc, &c.laid);
    CommandResult::Completed {
        output: CaseworkDrawn {
            plan: plan_of(doc, c.laid),
            shapes,
        },
        warnings: c.warnings,
    }
}

/// A run's write's refusal: no slot left, or the document's rules.
pub(crate) fn refused(r: kentos_domain::Refusal) -> Stop {
    let words = r.to_string();
    let code = if words == kentos_domain::SlotsExhausted.to_string() {
        codes::SLOTS_EXHAUSTED
    } else {
        codes::INVALID_CASEWORK
    };
    Stop::failed(error(code, words, None))
}

/// The answer of a laid run written: its id, its objects' persistent ids and slots in the plan's order, what it
/// brought and laid.
pub(crate) fn built(
    doc: &Document,
    run: kentos_commands::CaseworkId,
    written: &[(Uuid, kentos_domain::Slot)],
    laid: Laid,
) -> CaseworkBuilt {
    CaseworkBuilt {
        run: run.to_text(),
        uids: written.iter().map(|(u, _)| u.to_string()).collect(),
        ids: written.iter().map(|(_, s)| s.0).collect(),
        blocks: laid.added.iter().map(|b| b.name.clone()).collect(),
        modules: laid.modules,
        legs: laid.legs,
        fridge: laid.fridge,
        filler: laid.filler,
        released: false,
        revision: doc.revision().to_string(),
    }
}

/// Checks `input` again and writes the run, with the library blocks it brings, and its record as one undo step
/// (“Mutfak dizisi”).
pub fn execute(cx: &mut ExecutionContext<'_>, input: CaseworkRun) -> CommandResult<CaseworkBuilt> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_CASEWORK_RUN,
        kentos_commands::CAD_CASEWORK_RUN_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: CaseworkRun) -> CommandResult<CaseworkBuilt> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let written = cx.doc.transact(STEP, |doc| {
        doc.write_casework(None, input.clone(), &c.laid, STEP)
            .map_err(refused)
    });
    let (run, written) = match written {
        Ok(w) => w,
        Err(stop) => return stop.into(),
    };
    CommandResult::Completed {
        output: built(cx.doc, run, &written, c.laid),
        warnings: c.warnings,
    }
}
