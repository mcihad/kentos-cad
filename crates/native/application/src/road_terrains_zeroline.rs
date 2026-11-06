//! `road.terrains.zeroline` v1 (docs/adr/1620; plan/map-cad.md MAP-15): the cut and fill boundary of two terrain
//! surfaces, where the comparison meets the base (`kentos_road::volume::zero_lines`), written as 3D polylines at the
//! base's height on a layer in one undo step “Sıfır hattı” through `cad.entities.create`'s write. Harita and Yol
//! projects. The shared cases are `fixtures/commands/v1/road.terrains.zeroline.json`.
//!
//! The checks, in the contract's order: a map or road project; the layer; the expected revision; the surfaces, two
//! different ones; both made; a line at all.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, TerrainZeroLine,
    TerrainsZeroLine, TerrainsZeroLined, Vec2,
};
use kentos_road::surface::Surface;
use kentos_road::volume::{self, Triangle};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

struct Checked {
    out: TerrainsZeroLined,
    warnings: Vec<CommandWarning>,
}

fn triangles(s: &Surface) -> Vec<Triangle> {
    s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect()
}

fn check(doc: &kentos_domain::Document, input: &TerrainsZeroLine) -> Result<Checked, Stop> {
    terrains::terrain_project(doc)?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let base = crate::road_profiles::surface_id(doc, &input.base, "base")?;
    let comparison = crate::road_profiles::surface_id(doc, &input.comparison, "comparison")?;
    if base == comparison {
        return Err(fail(
            codes::INVALID_ANALYSIS,
            "Taban ile karşılaştırma aynı yüzey; iki ayrı yüzey verin.".into(),
            "comparison",
        ));
    }
    let def = |id: &str| terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled);
    let (b, c) = (def(&base)?, def(&comparison)?);
    let (made_b, made_c) = (terrains::make(doc, &b), terrains::make(doc, &c));
    let (sb, sc) = (made_b.built()?, made_c.built()?);
    let (found, coincident) = volume::zero_lines(&triangles(sc), &triangles(sb));
    if found.is_empty() {
        return Err(fail(
            codes::NO_ZERO_LINE,
            "İki yüzeyin farkı hiçbir yerde işaret değiştirmiyor: sıfır hattı yok (karşılaştırma tabanın hep altında ya \
             da hep üstünde, ya da örtüşmüyorlar)."
                .into(),
            "comparison",
        ));
    }
    let mut length = 0.0;
    let lines: Vec<TerrainZeroLine> = found
        .into_iter()
        .map(|l| {
            let mut run: f64 = l.points.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
            if l.closed && l.points.len() > 2 {
                let (a, z) = (l.points[0], l.points[l.points.len() - 1]);
                run += (a[0] - z[0]).hypot(a[1] - z[1]);
            }
            length += run;
            TerrainZeroLine {
                closed: l.closed,
                length: run,
                // The base's height there; the comparison's is the same on its zero line.
                points: l.points.iter().map(|p| [p[0], p[1], sb.height(p[0], p[1]).z.unwrap_or(f64::NAN)]).collect(),
            }
        })
        .collect();
    Ok(Checked {
        out: TerrainsZeroLined {
            base,
            comparison,
            lines,
            length,
            coincident,
            uids: Vec::new(),
            revision: doc.revision().to_string(),
        },
        warnings,
    })
}

fn objects(lines: &[TerrainZeroLine]) -> Vec<NewObject> {
    lines
        .iter()
        .map(|l| {
            let heights = l.points.iter().all(|p| p[2].is_finite());
            NewObject {
                geometry: EntityGeometry::Polyline {
                    pts: l.points.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
                    bulges: None,
                    zs: heights.then(|| l.points.iter().map(|p| Some(p[2])).collect()),
                    ms: None,
                    parts: None,
                    closed: l.closed,
                    widths: None,
                },
                color: None,
                line_weight: None,
                line_type: None,
                attrs: None,
                label: None,
                label_of: None,
                label_scale: None,
                symbol: None,
                order: None,
                transparency: None,
            }
        })
        .collect()
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsZeroLine) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// The lines execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsZeroLine) -> CommandResult<TerrainsZeroLined> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: c.out, warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// Writes the lines, one undo step “Sıfır hattı”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsZeroLine) -> CommandResult<TerrainsZeroLined> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TERRAINS_ZEROLINE,
        kentos_commands::ROAD_TERRAINS_ZEROLINE_VERSION,
        input,
        |cx, input| {
            let c = match check(cx.doc, &input) {
                Ok(c) => c,
                Err(stop) => return stop.into(),
            };
            let create = EntitiesCreate {
                layer_id: input.layer_id.clone(),
                objects: objects(&c.out.lines),
                operation: Some(CreateOperation::TerrainZeroLine),
                expected_revision: None,
            };
            match crate::create::write_in(cx, create) {
                CommandResult::Completed { output, warnings } => CommandResult::Completed {
                    output: TerrainsZeroLined { uids: output.created, revision: output.revision, ..c.out },
                    warnings,
                },
                other => crate::trim::relay(other),
            }
        },
    )
}
