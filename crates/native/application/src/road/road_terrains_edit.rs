//! `road.terrains.edit` v1 (plan/road.md Y2.3, docs/adr/0900 §11): a terrain surface's definition replaced (`set`: it
//! keeps the digest it was last accepted from), a surface made again from its sources accepted (`accept`), or one
//! removed. A setting, not an undo step. The shared cases are `fixtures/commands/v1/road.terrains.edit.json`.
//!
//! The checks, in the contract's order: a road project; what the operation needs (`surface`, `id`); set: the
//! definition without `built`, and the surfaces with it; the expected revision; the surface is the project's; accept: a
//! surface is made of what the sources give.

use kentos_commands::{CommandResult, CommandWarning, TerrainSurfaceDef, TerrainsEdit, TerrainsEditOperation, TerrainsWritten};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::{self as common, fail};

struct Checked {
    list: Vec<TerrainSurfaceDef>,
    surface: Option<kentos_commands::TerrainReport>,
    warnings: Vec<CommandWarning>,
}

fn unknown(id: &str, path: &str) -> Stop {
    fail(
        codes::UNKNOWN_TERRAIN,
        format!("“{id}” kimlikli yüzey projede yok. Var olan bir yüzeyin kimliğini verin."),
        path,
    )
}

fn check(doc: &kentos_domain::Document, input: &TerrainsEdit) -> Result<Checked, Stop> {
    common::terrain_project(doc)?;
    let own = common::surfaces(doc);
    let op = input.operation;
    if op == TerrainsEditOperation::Set && input.surface.is_none() {
        return Err(fail(codes::NO_TERRAIN, "Yazılacak yüzeyin tanımını (surface) verin.".into(), "surface"));
    }
    if op != TerrainsEditOperation::Set && input.id.as_deref().is_none_or(str::is_empty) {
        return Err(fail(codes::NO_ID, "Yüzeyin kimliğini (id) verin.".into(), "id"));
    }
    if let (TerrainsEditOperation::Set, Some(s)) = (op, &input.surface) {
        if s.built.is_some() {
            return Err(fail(
                codes::INVALID_TERRAIN,
                "Özet (built) yazılmaz; yüzeyi kabul etmek için accept kullanın.".into(),
                "surface.built",
            ));
        }
        let mut list = own.to_vec();
        let at = list.iter().position(|o| o.id == s.id);
        let mut new = s.clone();
        new.built = at.and_then(|i| list[i].built.clone());
        let index = match at {
            Some(i) => {
                list[i] = new.clone();
                i
            }
            None => list.len(),
        };
        if at.is_some() {
            common::judged(&list, index, "surface")?;
        } else {
            // Checked as it would stand, so its own rules are said before that it is no surface of the project.
            let mut with = list.clone();
            with.push(new.clone());
            common::judged(&with, index, "surface")?;
        }
        checks::revision(doc, input.expected_revision.as_deref())?;
        if at.is_none() {
            return Err(unknown(&s.id, "surface.id"));
        }
        let made = common::make(doc, &new);
        return Ok(Checked {
            surface: Some(made.report(&new)),
            warnings: made.warnings(),
            list,
        });
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let id = input.id.as_deref().unwrap_or_default();
    let Some(at) = own.iter().position(|o| o.id == id) else {
        return Err(unknown(id, "id"));
    };
    let mut list = own.to_vec();
    if op == TerrainsEditOperation::Remove {
        list.remove(at);
        return Ok(Checked {
            list,
            surface: None,
            warnings: vec![],
        });
    }
    let made = common::make(doc, &list[at]);
    made.built()?;
    list[at].built = Some(made.gathered.digest.clone());
    Ok(Checked {
        surface: Some(made.report(&list[at])),
        warnings: made.warnings(),
        list,
    })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsEdit) -> CommandResult<TerrainsWritten> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: TerrainsWritten {
                surfaces: c.list,
                surface: c.surface,
                revision: cx.doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the project's surfaces; a setting, not an undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsEdit) -> CommandResult<TerrainsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TERRAINS_EDIT,
        kentos_commands::ROAD_TERRAINS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: TerrainsEdit) -> CommandResult<TerrainsWritten> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    doc.set_settings(common::with_surfaces(doc, c.list.clone()));
    CommandResult::Completed {
        output: TerrainsWritten {
            surfaces: c.list,
            surface: c.surface,
            revision: doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}
