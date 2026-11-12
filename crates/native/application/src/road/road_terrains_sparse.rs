//! `road.terrains.sparse` v1 (plan/road.md Y19.3, docs/adr/0920 §7): a terrain surface's density grid's cells with
//! fewer points than the user's least drawn as closed polylines on their merged outlines (`kentos_road::surface`'s
//! `Density::outlines`), with `labels` each area's area. One undo step “Seyrek alanları çiz”. The shared cases are
//! `fixtures/commands/v1/road.terrains.sparse.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, NewObject, TerrainsCoverageDrawn, TerrainsSparseDraw,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::road_terrains_coverage_draw::outlined;
use crate::road_terrains_density::{grid, numbers};

struct Checked {
    objects: Vec<NewObject>,
    surface: String,
    areas: u32,
    holes: u32,
    area: f64,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &TerrainsSparseDraw) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    numbers(input.cell, Some(input.least), None)?;
    let (id, _, d) = grid(doc, &input.surface, input.boundary.as_deref(), input.cell, Some(input.least))?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let loops = d.outlines(|n| n < input.least);
    let (objects, areas, holes, area) = outlined(&loops, doc.settings().plot_scale, input.labels.then_some("Seyrek"));
    Ok(Checked { objects, surface: id, areas, holes, area, warnings })
}

fn write(cx: &mut ExecutionContext<'_>, input: &TerrainsSparseDraw, c: Checked) -> CommandResult<TerrainsCoverageDrawn> {
    let uids = if c.objects.is_empty() {
        Vec::new()
    } else {
        let create = EntitiesCreate {
            layer_id: input.layer_id.clone(),
            objects: c.objects,
            operation: Some(CreateOperation::Sparse),
            expected_revision: None,
        };
        match crate::create::write_in(cx, create) {
            CommandResult::Completed { output, .. } => output.created,
            other => return crate::trim::relay(other),
        }
    };
    CommandResult::Completed {
        output: TerrainsCoverageDrawn {
            surface: c.surface,
            areas: c.areas,
            holes: c.holes,
            area: c.area,
            uids,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsSparseDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsSparseDraw) -> CommandResult<TerrainsCoverageDrawn> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), input, c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Draws the sparse cells: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsSparseDraw) -> CommandResult<TerrainsCoverageDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TERRAINS_SPARSE_DRAW,
        kentos_commands::ROAD_TERRAINS_SPARSE_DRAW_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, &input, c),
            Err(stop) => stop.into(),
        },
    )
}
