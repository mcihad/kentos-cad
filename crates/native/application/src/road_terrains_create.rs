//! `road.terrains.create` v1 (plan/road.md Y2.3, docs/adr/0900 §11): a terrain surface made from the drawing's points,
//! breaklines, boundary and voids; its definition goes last in the road project's settings, accepted (its digest the
//! sources'). A setting, not an undo step. The shared cases are `fixtures/commands/v1/road.terrains.create.json`.
//!
//! The checks, in the contract's order: a road project; the definition and the surfaces with it; the expected
//! revision; a surface is made of what the sources give.

use kentos_commands::{CommandResult, TerrainSurfaceDef, TerrainsCreate, TerrainsWritten};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::road_terrains::{self as common, Made};

struct Checked {
    /// The project's surfaces with the new one last.
    list: Vec<TerrainSurfaceDef>,
    def: TerrainSurfaceDef,
    made: Made,
}

fn check(doc: &kentos_domain::Document, input: &TerrainsCreate) -> Result<Checked, Stop> {
    common::terrain_project(doc)?;
    let own = common::surfaces(doc);
    let id = input.id.clone().unwrap_or_else(|| {
        (1..)
            .map(|n| format!("t-{n}"))
            .find(|id| own.iter().all(|s| &s.id != id))
            .unwrap_or_default()
    });
    let mut def = TerrainSurfaceDef {
        id,
        name: input.name.clone(),
        points: input.points.clone(),
        breaklines: input.breaklines.clone(),
        boundary: input.boundary.clone(),
        voids: input.voids.clone(),
        tolerance: input.tolerance,
        max_edge: input.max_edge,
        built: None,
        clouds: Vec::new(),
        excluded: Vec::new(),
    };
    let mut list = own.to_vec();
    list.push(def.clone());
    common::judged(&list, list.len() - 1, "")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let made = common::make(doc, &def);
    made.built()?;
    def.built = Some(made.gathered.digest.clone());
    list.pop();
    list.push(def.clone());
    Ok(Checked { list, def, made })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.made.warnings(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsCreate) -> CommandResult<TerrainsWritten> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            warnings: c.made.warnings(),
            output: TerrainsWritten {
                surface: Some(c.made.report(&c.def)),
                surfaces: c.list,
                revision: cx.doc.revision().to_string(),
            },
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the project's surfaces; a setting, not an undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsCreate) -> CommandResult<TerrainsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TERRAINS_CREATE,
        kentos_commands::ROAD_TERRAINS_CREATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: TerrainsCreate) -> CommandResult<TerrainsWritten> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let surface = Some(c.made.report(&c.def));
    let warnings = c.made.warnings();
    let doc = &mut *cx.doc;
    doc.set_settings(common::with_surfaces(doc, c.list.clone()));
    CommandResult::Completed {
        output: TerrainsWritten {
            surfaces: c.list,
            surface,
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}
