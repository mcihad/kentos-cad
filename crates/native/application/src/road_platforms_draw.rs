//! `road.platforms.draw` v1 (plan/road.md Y14, docs/adr/0927): a graded pad drawn in plan: its boundary and (when no
//! line is open) its daylight line as closed 3D polylines, each slope line that meets the terrain as a line; with
//! `labels` a text at the pad's middle saying its base elevation, cut and fill. One undo step “Platform çiz”. The shared
//! cases are `fixtures/commands/v1/road.platforms.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, PlatformsDraw, PlatformsDrawn,
    TextAlign, TextFace, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::road_platforms as common;

/// A text's paper height (mm) at the drawing's plot scale.
const TEXT_MM: f64 = 2.5;

struct Checked {
    id: String,
    objects: Vec<NewObject>,
    warnings: Vec<CommandWarning>,
}

fn object(geometry: EntityGeometry) -> NewObject {
    NewObject {
        geometry,
        color: None,
        line_weight: None,
        line_type: None,
        line_pattern: None,
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

/// A closed 3D polyline through `pts`.
fn ring(pts: &[[f64; 3]]) -> NewObject {
    object(EntityGeometry::Polyline {
        pts: pts.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
        bulges: None,
        zs: Some(pts.iter().map(|p| Some(p[2])).collect()),
        ms: None,
        parts: None,
        closed: true,
        widths: None,
    })
}

fn comma(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

fn check(doc: &Document, input: &PlatformsDraw, told: &dyn Fn(u64, u64) -> bool) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_platforms(), &input.platform, "platform")?;
    let read = common::read_told(doc, def, told)?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    warnings.extend(read.warnings);
    let r = &read.report;
    let mut objects = vec![ring(&r.top)];
    if !r.daylight.is_empty() {
        objects.push(ring(&r.daylight));
    }
    for l in &r.lines {
        if let Some(to) = l.to
            && (to[0] - l.from[0]).hypot(to[1] - l.from[1]) > 1e-9
        {
            objects.push(object(EntityGeometry::Line {
                a: Vec2 { x: l.from[0], y: l.from[1] },
                b: Vec2 { x: to[0], y: to[1] },
                zs: Some(vec![Some(l.from[2]), Some(to[2])]),
                ms: None,
            }));
        }
    }
    if input.labels {
        let h = TEXT_MM * doc.settings().plot_scale / 1000.0;
        let n = r.top.len() as f64;
        let (cx, cy) = r.top.iter().fold((0.0, 0.0), |(x, y), p| (x + p[0] / n, y + p[1] / n));
        let volumes = match (r.cut, r.fill) {
            (Some(c), Some(f)) => format!(" · Kazı {} m³ · Dolgu {} m³", comma(c, 1), comma(f, 1)),
            _ => String::new(),
        };
        objects.push(object(EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: Vec2 { x: cx, y: cy },
            text: format!("{} · Kot {}{volumes}", def.name, comma(def.base[2], 2)),
            height: h,
            rotation: 0.0,
            align: Some(TextAlign::MiddleCenter),
            width_factor: None,
            mask: false,
            box_width: None,
            line_spacing: None,
            runs: Vec::new(),
            face: TextFace::default(),
            path: None,
        }));
    }
    Ok(Checked { id: def.id.clone(), objects, warnings })
}

fn write(cx: &mut ExecutionContext<'_>, input: &PlatformsDraw, c: Checked) -> CommandResult<PlatformsDrawn> {
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: c.objects,
        operation: Some(CreateOperation::Platforms),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: PlatformsDrawn { platform: c.id, uids: created.created, revision: cx.doc.revision().to_string() },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &PlatformsDraw) -> CommandResult<()> {
    match check(cx.doc, input, &cx.stop_only()) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would draw now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &PlatformsDraw) -> CommandResult<PlatformsDrawn> {
    let c = match check(cx.doc, input, &cx.stop_only()) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), input, c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.uids.clear();
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Draws `input` now, checked and written in the step it runs in, its drawing not kept (`road.project.rebuild` keeps it,
/// docs/adr/0911 §3).
pub(crate) fn draw(cx: &mut ExecutionContext<'_>, input: &PlatformsDraw) -> CommandResult<PlatformsDrawn> {
    let checked = check(cx.doc, input, &cx.told());
    match checked {
        Ok(c) => write(cx, input, c),
        Err(stop) => stop.into(),
    }
}

/// Draws the pad: one undo step, kept in the road drawings.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PlatformsDraw) -> CommandResult<PlatformsDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_PLATFORMS_DRAW,
        kentos_commands::ROAD_PLATFORMS_DRAW_VERSION,
        input,
        |cx, input| {
            let checked = check(cx.doc, &input, &cx.told());
            let r = match checked {
                Ok(c) => write(cx, &input, c),
                Err(stop) => stop.into(),
            };
            crate::road_drawings::kept(cx, kentos_commands::ROAD_PLATFORMS_DRAW, &input, (kentos_commands::RoadSourceKind::Platform, "platform"), r, |o| &o.uids)
        },
    )
}

#[cfg(test)]
mod stop_tests {
    //! Docs/adr/4860: the corners' slope lines, the design surface, the volumes told and a stop heard within (`crate::told_tests`), on the shared cases' project.
    use super::*;

    const CASES: &str = include_str!("../../../../fixtures/commands/v1/road.platforms.draw.json");

    #[test]
    fn its_reckoning_is_told_and_hears_a_stop() {
        let (doc, input): (kentos_domain::Document, kentos_commands::PlatformsDraw) = crate::told_tests::project(CASES);
        crate::told_tests::told_and_stopped(|told| check(&doc, &input, told));
    }

    #[test]
    fn a_stopped_job_draws_nothing() {
        let (mut doc, input): (kentos_domain::Document, kentos_commands::PlatformsDraw) = crate::told_tests::project(CASES);
        crate::told_tests::stopped_job_draws_nothing(&mut doc, |cx| draw(cx, &input));
    }
}
