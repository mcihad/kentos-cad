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

/// Checks `input`, `told` the levels drawn and answering whether to stop (docs/adr/4040).
fn check(doc: &kentos_domain::Document, input: &TerrainsContours, told: &dyn Fn(u64, u64) -> bool) -> Result<Checked, Stop> {
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
        .contours_told(input.interval, base, input.major_every.unwrap_or(5), told)
        .map_err(|e| match e {
            ContourError::Interval => bad(),
            ContourError::TooMany => fail(
                codes::INVALID_CONTOURS,
                "Bu aralıkla yüzeyi 10 000'den çok düzey keser; aralığı büyütün.".into(),
                "interval",
            ),
        })?
        .ok_or(Stop::Cancelled)?;
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
    match check(cx.doc, input, &cx.stop_only()) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The lines execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsContours) -> CommandResult<TerrainsContoured> {
    match check(cx.doc, input, &cx.stop_only()) {
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
    let c = match check(cx.doc, &input, &cx.told()) {
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

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use kentos_commands::DocumentSnapshotV2;
    use serde_json::Value;

    use super::*;

    /// The shared cases' project and their first case's input.
    fn project() -> (kentos_domain::Document, kentos_commands::TerrainsContours) {
        let cases: Value =
            serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.contours.json")).expect("cases");
        let snapshot: DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("a snapshot");
        let doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
        let input = serde_json::from_value(cases["cases"][0]["steps"][0]["input"].clone()).expect("an input");
        (doc, input)
    }

    /// docs/adr/4040: the levels are told one at a time, increasing within one whole; a stop asked at the fourth is heard there, cancelled.
    #[test]
    fn the_levels_are_told_and_a_stop_is_heard_between_them() {
        let (doc, input) = project();
        let seen = RefCell::new(Vec::new());
        let whole = check(&doc, &input, &|done, total| {
            seen.borrow_mut().push((done, total));
            false
        });
        assert!(whole.is_ok());
        let seen = seen.into_inner();
        assert!(seen.len() > 4, "{} ticks", seen.len());
        assert!(seen.windows(2).all(|w| w[0].0 <= w[1].0 && w[0].1 == w[1].1 && w[1].0 < w[1].1), "{seen:?}");
        assert_eq!(seen, (0..seen.len() as u64).map(|k| (k, seen.len() as u64)).collect::<Vec<_>>());
        let asked = Cell::new(0);
        let stopped = check(&doc, &input, &|_, _| {
            asked.set(asked.get() + 1);
            asked.get() > 3
        });
        assert!(matches!(stopped, Err(Stop::Cancelled)));
        assert_eq!(asked.get(), 4);
    }

    /// As a job's part (a rebuild's, Tariften üret's), a stop is heard in the levels: cancelled, nothing written.
    #[test]
    fn a_stopped_job_draws_nothing() {
        let (mut doc, input) = project();
        let before = doc.to_snapshot_v2();
        let job = crate::jobs::Progress::default();
        job.stop();
        let mut cx = ExecutionContext::new(&mut doc);
        cx.job = Some(&job);
        assert!(matches!(draw(&mut cx, &input), CommandResult::Cancelled));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}
