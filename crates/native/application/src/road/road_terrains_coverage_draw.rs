//! `road.terrains.lowconfidence` v1 (plan/road.md Y19.2, docs/adr/0920 §5): a terrain surface's estimated areas, its
//! triangles longer than the user's limit, drawn as closed polylines on their outlines (`kentos_road::surface`'s
//! `outlines`), the low confidence areas' 2D polygons; with `outline` the surface's own outline, with `labels` each
//! area's area. One undo step “Düşük güvenli alanları çiz”. The shared cases are
//! `fixtures/commands/v1/road.terrains.lowconfidence.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, TerrainsCoverageDraw,
    TerrainsCoverageDrawn, TextAlign, TextFace, Vec2,
};
use kentos_domain::Document;
use kentos_road::surface::ring_area;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

/// A text's paper height (mm) at the drawing's plot scale.
const TEXT_MM: f64 = 2.5;

pub(crate) fn object(geometry: EntityGeometry) -> NewObject {
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

pub(crate) fn ring(pts: &[[f64; 2]]) -> NewObject {
    object(EntityGeometry::Polyline {
        pts: pts.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
        bulges: None,
        zs: None,
        ms: None,
        parts: None,
        closed: true,
        widths: None,
    })
}

/// Whether `p` lies inside `ring` (even-odd).
fn inside(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    let n = ring.len();
    let mut hit = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) {
            hit = !hit;
        }
    }
    hit
}

/// A ring's area-weighted centre.
fn centre(ring: &[[f64; 2]]) -> [f64; 2] {
    let n = ring.len();
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let c = p[0] * q[1] - q[0] * p[1];
        a += c;
        cx += (p[0] + q[0]) * c;
        cy += (p[1] + q[1]) * c;
    }
    [cx / (3.0 * a), cy / (3.0 * a)]
}

/// Outlines as closed polylines and, with `label`, each outer outline's area (less the holes it holds) at its centre:
/// the objects, how many outer outlines and holes, and their area.
pub(crate) fn outlined(loops: &[Vec<[f64; 2]>], plot_scale: f64, label: Option<&str>) -> (Vec<NewObject>, u32, u32, f64) {
    let outers: Vec<&Vec<[f64; 2]>> = loops.iter().filter(|r| ring_area(r) > 0.0).collect();
    let holes: Vec<&Vec<[f64; 2]>> = loops.iter().filter(|r| ring_area(r) <= 0.0).collect();
    let mut objects: Vec<NewObject> = loops.iter().map(|r| ring(r)).collect();
    let mut area = 0.0;
    let height = TEXT_MM * plot_scale / 1000.0;
    for outer in &outers {
        // Its holes: those whose first corner it holds, not held by a smaller outer outline also holding it.
        let own: f64 = holes
            .iter()
            .filter(|h| {
                inside(outer, h[0])
                    && !outers.iter().any(|o| !std::ptr::eq(*o, *outer) && inside(o, h[0]) && ring_area(o) < ring_area(outer))
            })
            .map(|h| -ring_area(h))
            .sum();
        let a = ring_area(outer) - own;
        area += a;
        if let Some(label) = label {
            let c = centre(outer);
            objects.push(object(EntityGeometry::Text {
                layout: None,
                annotative: None,
                p: Vec2 { x: c[0], y: c[1] },
                text: format!("{label} · {} m²", format!("{a:.1}").replace('.', ",")),
                height,
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
    }
    let n = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
    (objects, n(outers.len()), n(holes.len()), area)
}

struct Checked {
    objects: Vec<NewObject>,
    surface: String,
    areas: u32,
    holes: u32,
    area: f64,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &TerrainsCoverageDraw) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    if !(input.longest.is_finite() && input.longest > 0.0) {
        return Err(fail(codes::INVALID_COVERAGE, "Tahmin sayılan kenar boyu (longest) sıfırdan büyük olmalı.".into(), "longest"));
    }
    terrains::terrain_project(doc)?;
    let id = crate::road_profiles::surface_id(doc, &input.surface, "surface")?;
    let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
    let made = terrains::make(doc, &def);
    let surface = made.built()?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let loops = surface.outlines(|i| surface.longest_edge(i) > input.longest);
    let (mut objects, areas, holes, area) = outlined(&loops, doc.settings().plot_scale, input.labels.then_some("Düşük güven"));
    if input.outline {
        objects.extend(surface.outlines(|_| true).iter().map(|r| ring(r)));
    }
    Ok(Checked {
        objects,
        surface: id,
        areas,
        holes,
        area,
        warnings,
    })
}

fn write(cx: &mut ExecutionContext<'_>, input: &TerrainsCoverageDraw, c: Checked) -> CommandResult<TerrainsCoverageDrawn> {
    let uids = if c.objects.is_empty() {
        Vec::new()
    } else {
        let create = EntitiesCreate {
            layer_id: input.layer_id.clone(),
            objects: c.objects,
            operation: Some(CreateOperation::LowConfidence),
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
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsCoverageDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsCoverageDraw) -> CommandResult<TerrainsCoverageDrawn> {
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

/// Draws the estimated areas: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsCoverageDraw) -> CommandResult<TerrainsCoverageDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW,
        kentos_commands::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, &input, c),
            Err(stop) => stop.into(),
        },
    )
}
