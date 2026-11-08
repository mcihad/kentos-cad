//! `road.terrains.contours` v1 (plan/road.md Y2.5, docs/adr/0900 §13): a road project's surface's contour lines written
//! as 3D polylines on a layer, the major ones thicker, in one undo step “Eş yükselti eğrileri” through
//! `cad.entities.create`'s write. The shared cases are `fixtures/commands/v1/road.terrains.contours.json`.
//!
//! The checks, in the contract's order: a road project; the interval; the layer; the expected revision; the surface;
//! a surface is made; the levels; a line at all.

use kentos_commands::{
    CommandResult, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, TerrainContour, TerrainsContoured,
    TerrainsContours, Vec2,
};
use kentos_road::surface::ContourError;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::{self as common, fail};

/// A major line's weight (mm); a minor one takes its layer's.
const MAJOR_WEIGHT: f64 = 0.35;

struct Checked {
    lines: Vec<TerrainContour>,
    warnings: Vec<kentos_commands::CommandWarning>,
}

fn check(doc: &kentos_domain::Document, input: &TerrainsContours) -> Result<Checked, Stop> {
    common::terrain_project(doc)?;
    let bad = || {
        fail(
            codes::INVALID_CONTOURS,
            "Eş yükselti aralığı 0'dan büyük bir sayı, taban kotu sonlu bir sayı olmalı.".into(),
            "interval",
        )
    };
    let base = input.base.unwrap_or(0.0);
    if !(input.interval.is_finite() && input.interval > 0.0 && base.is_finite()) {
        return Err(bad());
    }
    let warnings = checks::layer(doc, &input.layer_id)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let own = common::surfaces(doc);
    let name = input.surface.as_str();
    let Some(def) = own
        .iter()
        .find(|s| s.id == name)
        .or_else(|| own.iter().find(|s| s.name.to_lowercase() == name.to_lowercase()))
    else {
        return Err(fail(
            codes::UNKNOWN_TERRAIN,
            format!("“{name}” kimlikli ya da adlı yüzey projede yok."),
            "surface",
        ));
    };
    let made = common::make(doc, def);
    let surface = made.built()?;
    let lines = surface
        .contours(input.interval, base, input.major_every.unwrap_or(5))
        .map_err(|e| match e {
            ContourError::Interval => bad(),
            ContourError::TooMany => fail(
                codes::INVALID_CONTOURS,
                "Bu aralıkla yüzeyi 10 000'den çok düzey keser; aralığı büyütün.".into(),
                "interval",
            ),
        })?;
    if lines.is_empty() {
        return Err(fail(
            codes::NO_CONTOURS,
            "Bu aralıkla yüzeyi kesen eş yükselti eğrisi yok; aralığı ya da taban kotunu değiştirin.".into(),
            "interval",
        ));
    }
    Ok(Checked {
        lines: lines
            .into_iter()
            .map(|l| TerrainContour {
                z: l.z,
                major: l.major,
                closed: l.closed,
                points: l.points,
            })
            .collect(),
        warnings,
    })
}

fn objects(lines: &[TerrainContour]) -> Vec<NewObject> {
    lines
        .iter()
        .map(|l| NewObject {
            geometry: EntityGeometry::Polyline {
                pts: l.points.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
                bulges: None,
                zs: Some(vec![Some(l.z); l.points.len()]),
                ms: None,
                parts: None,
                closed: l.closed,
                widths: None,
            },
            color: None,
            line_weight: l.major.then_some(MAJOR_WEIGHT),
            line_type: None,
            line_pattern: None,
            attrs: None,
            label: None,
            label_of: None,
            label_scale: None,
            symbol: None,
            order: None,
            transparency: None,
        })
        .collect()
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsContours) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The lines execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsContours) -> CommandResult<TerrainsContoured> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: TerrainsContoured {
                lines: c.lines,
                uids: Vec::new(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the lines, one undo step.
/// Draws `input` now, checked and written in the step it runs in, its drawing not kept (`road.project.rebuild` keeps it,
/// docs/adr/0911 §3).
pub(crate) fn draw(cx: &mut ExecutionContext<'_>, input: &kentos_commands::TerrainsContours) -> CommandResult<kentos_commands::TerrainsContoured> {
    write_execute(cx, input.clone())
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsContours) -> CommandResult<TerrainsContoured> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TERRAINS_CONTOURS,
        kentos_commands::ROAD_TERRAINS_CONTOURS_VERSION,
        input,
        |cx, input| {
            let r = write_execute(cx, input.clone());
            crate::road_drawings::kept(cx, kentos_commands::ROAD_TERRAINS_CONTOURS, &input, (kentos_commands::RoadSourceKind::Surface, "surface"), r, |o| &o.uids)
        },
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: TerrainsContours) -> CommandResult<TerrainsContoured> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: objects(&c.lines),
        operation: Some(CreateOperation::TerrainContours),
        expected_revision: None,
    };
    match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: TerrainsContoured {
                lines: c.lines,
                uids: output.created,
                revision: output.revision,
            },
            warnings,
        },
        other => crate::trim::relay(other),
    }
}
