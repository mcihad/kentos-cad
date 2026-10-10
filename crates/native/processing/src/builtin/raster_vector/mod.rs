//! Raster ve vektör and Taranmış harita (docs/adr/0234): Rasterleştir (the
//! raster core's point job burning the objects onto a grid), Rasterden
//! alan, çizgi and nokta, Çizgi yakala and Alan kapat (its operation job's
//! vectorizing work: the features on a new layer right above the raster's,
//! in the run's one step) and Eğrilere kot ver (the geometry core's
//! ordering; the curves' vertices given their elevations in one step).

mod tools;

use std::collections::BTreeMap;

use kentos_contracts::{Entity, RasterEntity, RasterFields, RasterSample};
use kentos_domain::Slot;
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::ops::contour_elevations::contour_elevations;
use kentos_geometry_core::vec2::Vec2;
use kentos_native_application::elevation;
use kentos_native_application::geometry::shape;
use kentos_raster::from_points::{PointInput, PointJob, PointSpec};
use kentos_raster::vector::{FeatureKind, Features};
use serde_json::{Value, json};

use super::geometry::new_object;
use super::interpolation::grid_of;
use super::pointcloud::{STOPPED, broke, count_words};
use super::queries::attr;
use super::raster_ops::{drive, names_of, rasters_for, spec_of};
use super::surface::{base, files_of, system_of};
use crate::files::Beside;
use crate::types::{ChangeSet, Feedback, Patch, Resolved, RunContext, RunResult};

pub use tools::{
    capture_line, close_area, contour_elevations_tool, rasterize, to_lines, to_points, to_polygons,
};

pub const RASTER_VECTOR: &str = "rasterVector";
pub const SCANNED: &str = "scannedMap";

/// The kinds Rasterleştir burns: areas (by their cells' centres), lines (every cell they touch) and points.
pub const BURN_KINDS: [&str; 9] = [
    "polygon", "circle", "ellipse", "spline", "hatch", "line", "polyline", "arc", "point",
];

/// An elevation or a value as a summary writes it: at most three decimals, trailing zeros dropped.
pub fn short(v: f64) -> String {
    let t = fixed(v, 3);
    if t.contains('.') {
        t.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        t
    }
}

/// The features as objects on `layer`: areas with their holes, polylines,
/// points (with their elevation when `z` gives one).
pub fn objects_of(
    f: &Features,
    layer: &str,
    attrs: &dyn Fn(usize) -> BTreeMap<String, String>,
    z: Option<&dyn Fn(usize) -> f64>,
) -> Vec<Entity> {
    let mut out = Vec::with_capacity(f.len());
    let (mut at, mut ring) = (0usize, 0usize);
    let mut path = |n: usize| -> Vec<Vec2> {
        let pts = (at..at + n)
            .map(|q| Vec2::new(f.xy[2 * q], f.xy[2 * q + 1]))
            .collect();
        at += n;
        pts
    };
    for k in 0..f.len() {
        let zk = z.map(|z| z(k));
        let made = match f.kind {
            FeatureKind::Points => {
                let p = path(1)[0];
                new_object(
                    Shape::Point {
                        p,
                        z: zk,
                        parts: None,
                    },
                    layer,
                    attrs(k),
                )
            }
            FeatureKind::Lines => {
                let pts = path(f.sizes[k] as usize);
                let n = pts.len();
                new_object(
                    Shape::Polyline {
                        pts,
                        bulges: None,
                        holes: None,
                        parts: None,
                    },
                    layer,
                    attrs(k),
                )
                .map(|mut e| {
                    if let Some(z) = zk {
                        elevation::assign(&mut e, &[vec![Some(z); n]]);
                    }
                    e
                })
            }
            FeatureKind::Polygons => {
                let count = f.rings[k] as usize;
                let mut rings: Vec<Vec<Vec2>> = (0..count)
                    .map(|r| path(f.sizes[ring + r] as usize))
                    .collect();
                ring += count;
                let outline = rings.remove(0);
                new_object(
                    Shape::Polygon {
                        pts: outline,
                        bulges: None,
                        holes: (!rings.is_empty()).then(|| {
                            rings
                                .into_iter()
                                .map(|pts| Ring { pts, bulges: None })
                                .collect()
                        }),
                        parts: None,
                    },
                    layer,
                    attrs(k),
                )
            }
        };
        out.extend(made);
    }
    out
}

/// What a vectorizing tool makes of its features: the objects and the summary.
pub type Made<'a> = dyn Fn(&Features, &str, &mut dyn Feedback) -> (Vec<Entity>, String) + 'a;

/// Runs a vectorizing job over the one raster of the input; `made` turns its features into objects and a summary.
pub fn run_vector(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    label: &str,
    made: &Made<'_>,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = || -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, true)?;
        let names = names_of(&rasters, ctx);
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let ran = drive(&*files, &rasters, &spec, Vec::new(), None, feedback, label)?;
        let f = ran.features.ok_or("Çözümleme nesne vermedi.")?;
        feedback.progress(1.0, label);
        let layer = r.layer("layer").id.clone();
        let (add, summary) = made(&f, &layer, feedback);
        let mut result = RunResult {
            changes: (!add.is_empty() && !layer.is_empty()).then(|| ChangeSet {
                add,
                ..ChangeSet::default()
            }),
            summary: Some(summary),
            above: Some(rasters[0].base.layer_id.clone()),
            ..RunResult::default()
        };
        let n = result.changes.as_ref().map_or(0, |c| c.add.len());
        result.outputs.insert("count".to_owned(), json!(n));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}

/// Rasterleştir (docs/adr/0234 §3): the objects burnt onto the grid, the
/// GeoTIFF written, its object right below the input's layer.
pub fn run_rasterize(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let mut go = || -> Result<RunResult, String> {
        let entities = &r.features("input").entities;
        let Some(first) = entities.first() else {
            return Err("Girdi boş: nesne seçin.".into());
        };
        let field = r.text("field").trim();
        let by_field = r.text("valueFrom") == "field";
        if by_field && field.is_empty() {
            return Err("Değer alanını seçin: Değer alandan okunur.".into());
        }
        let texts: Option<Vec<Option<String>>> = by_field.then(|| {
            entities
                .iter()
                .map(|e| attr(e, field).map(str::to_owned))
                .collect()
        });
        let input = PointInput::Lines {
            shapes: entities.iter().map(|e| shape(e)).collect(),
            weights: texts,
        };
        let srid = ctx.doc.settings().srid;
        let tool = json!({
            "kind": "rasterize",
            "value": r.number("value").unwrap_or(1.0),
            "overlap": match r.text("overlap") { "" => "last", o => o },
            "sample": match r.text("sample") { "" => "f32", s => s },
        });
        let spec = PointSpec {
            tool: serde_json::from_value(tool)
                .map_err(|e| format!("Çözümlemenin ayarları okunamadı: {e}"))?,
            cell: r.number("cellSize").unwrap_or(0.0),
            grid: grid_of(r)?,
            epsg: (srid > 0).then_some(srid),
            system: system_of(ctx),
            cross: false,
        };
        let layer_name = ctx.layer_name(&first.base().layer_id);
        let path = files.output_path(
            r.text("output"),
            Some(Beside::named(layer_name, "yuzey")),
            "-raster",
            ".tif",
        )?;
        let label = "Rasterleştiriliyor";
        let (mut job, header) = PointJob::new(input, &spec, kentos_raster::par::threads())?;
        let mut sink = files.create(&path)?;
        sink.write(&header)?;
        while !job.done() {
            if feedback.canceled() {
                return Err(STOPPED.to_owned());
            }
            let bytes = job.step()?;
            sink.write(&bytes)?;
            feedback.progress(0.97 * job.share(), label);
        }
        let (grid, bands, sample, style) = (job.grid(), job.bands(), job.sample(), job.style(1));
        let done = job.finish()?;
        sink.write(&done.tail)?;
        sink.patch(0, &done.header)?;
        sink.finish()?;
        feedback.progress(1.0, label);
        let notes = &done.notes;
        if notes.unread > 0 {
            feedback.warn(format!(
                "{} nesnenin değeri sayı olarak okunamadığı için alınmadı.",
                count_words(notes.unread as u64)
            ));
        }
        if notes.outside > 0 {
            feedback.warn(format!(
                "{} nesne ızgaranın dışında kaldı.",
                count_words(notes.outside as u64)
            ));
        }
        let add: Vec<Entity> = r
            .flag("add")
            .then(|| {
                raster_object(
                    &grid,
                    (bands, sample),
                    &path,
                    srid,
                    style,
                    &r.layer("layer").id,
                )
            })
            .into_iter()
            .collect();
        let mut summary = format!(
            "{} nesneden {} × {} hücrelik raster; “{path}” yazıldı.",
            count_words(notes.taken as u64),
            count_words(u64::from(grid.width)),
            count_words(u64::from(grid.height))
        );
        if notes.empty > 0 {
            summary.push_str(&format!(" Değersiz {} hücre.", count_words(notes.empty)));
        }
        let mut result = RunResult {
            changes: (!add.is_empty()).then(|| ChangeSet {
                add,
                ..ChangeSet::default()
            }),
            summary: Some(summary),
            ..RunResult::default()
        };
        result.outputs.insert("file".to_owned(), json!(path));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}

/// Rasterleştir's raster object: the grid, the file, its samples and look.
fn raster_object(
    grid: &kentos_raster::grid::Grid,
    (bands, sample): (u32, RasterSample),
    path: &str,
    srid: u32,
    style: kentos_contracts::RasterStyle,
    layer: &str,
) -> Entity {
    Entity::Raster(RasterEntity {
        base: base(layer, BTreeMap::new(), None),
        raster: RasterFields {
            affine: grid.affine,
            width: grid.width,
            height: grid.height,
            bands,
            sample,
            asset: None,
            file: Some(path.to_owned()),
            url: None,
            srid,
            style,
            opacity: None,
            dataset: None,
        },
    })
}

/// Eğrilere kot ver (docs/adr/0234 §9): each curve the cut crosses its
/// elevation at every vertex, in the run's one step.
pub fn run_contour_elevations(
    r: &Resolved<'_>,
    _ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let mut go = || -> Result<RunResult, String> {
        let curves = &r.features("curves").entities;
        if curves.is_empty() {
            return Err("Eğrileri seçin: çizgi, çoklu çizgi ya da kapalı alan.".into());
        }
        let (Some(start), Some(end)) = (r.point("start"), r.point("end")) else {
            return Err("Kesen çizginin başlangıç ve bitiş noktalarını seçin.".into());
        };
        if start.x == end.x && start.y == end.y {
            return Err(
                "Başlangıç ve bitiş aynı nokta: kesen çizgi eğrileri boydan boya geçmeli.".into(),
            );
        }
        let step = r.number("step").unwrap_or(1.0);
        if !(step != 0.0 && step.is_finite()) {
            return Err("Aralık 0 olamaz: eksi bir aralık kotları azaltır.".into());
        }
        let first = r.number("first").unwrap_or(0.0);
        let shapes: Vec<Shape> = curves.iter().map(|e| shape(e)).collect();
        let zs = contour_elevations(
            &shapes,
            Vec2::new(start.x, start.y),
            Vec2::new(end.x, end.y),
            first,
            step,
        );
        let mut update = Vec::new();
        let mut missed = 0usize;
        for (e, z) in curves.iter().zip(zs) {
            let Some(z) = z else {
                missed += 1;
                continue;
            };
            let paths = elevation::paths(e);
            update.push(Patch {
                id: Slot(e.base().id),
                attrs: None,
                label: None,
                zs: Some(paths.iter().map(|p| vec![Some(z); p.pts.len()]).collect()),
            });
        }
        if missed > 0 {
            feedback.warn(format!(
                "{} eğri kesen çizgiyle kesişmediği için değişmedi.",
                count_words(missed as u64)
            ));
        }
        if update.is_empty() {
            return Ok(RunResult {
                summary: Some("Kesen çizgi seçilen eğrilerin hiçbirini kesmiyor.".into()),
                ..RunResult::default()
            });
        }
        let n = update.len();
        let changed: Vec<u32> = update.iter().map(|u: &Patch| u.id.0).collect();
        let mut result = RunResult {
            changes: Some(ChangeSet {
                update,
                ..ChangeSet::default()
            }),
            summary: Some(format!(
                "{} eğriye kot verildi: {} ile {} arası.",
                count_words(n as u64),
                short(first),
                short(first + (n - 1) as f64 * step)
            )),
            ..RunResult::default()
        };
        result.outputs.insert("changed".to_owned(), json!(changed));
        result.outputs.insert("count".to_owned(), json!(n));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}
