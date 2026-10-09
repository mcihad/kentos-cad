//! Uzaklık ve maliyet (docs/adr/0236): Uzaklık yüzeyi (from objects the
//! raster core's point job on their box or a raster's grid, from a raster's
//! cells its operation job), Birikimli maliyet and Maliyet koridoru (a raster
//! beside the cost raster, its object right above the cost raster's layer)
//! and En düşük maliyetli yol (polylines on a new layer right above it),
//! each in the run's one step. The sources and destinations are objects
//! burnt onto the cost raster's cells; a surface is the second raster read.

mod tools;

use std::collections::BTreeMap;

use kentos_contracts::{Entity, RasterEntity, RasterFields};
use kentos_native_application::geometry::shape;
use kentos_raster::from_points::{PointInput, PointJob, PointSpec};
use kentos_raster::ops::Notes;
use kentos_raster::vector::Features;
use serde_json::{Value, json};

use super::hydrology::trimmed;
use super::interpolation::grid_of;
use super::pointcloud::{STOPPED, broke, count_words};
use super::raster_ops::{drive, names_of, rasters_for, spec_of};
use super::raster_vector::objects_of;
use super::surface::{base, files_of, system_of};
use crate::files::Beside;
use crate::types::{ChangeSet, Feedback, Resolved, RunContext, RunResult};

pub use tools::{corridor, cost, euclidean, path};

pub const DISTANCE: &str = "distance";

/// An attribute's text: the numbers whole, costs, lengths and grades to three decimals.
pub fn attr_text(field: &str, v: f64) -> String {
    match field {
        "Yol" | "Kaynak" => trimmed(v, 0),
        _ => trimmed(v, 3),
    }
}

fn attrs_of(f: &Features, k: usize) -> BTreeMap<String, String> {
    let stride = f.fields.len();
    f.fields
        .iter()
        .enumerate()
        .map(|(x, name)| {
            (
                (*name).to_owned(),
                attr_text(name, f.numbers[k * stride + x]),
            )
        })
        .collect()
}

/// The objects of a features parameter as the core reads them.
fn shapes_of(r: &Resolved<'_>, name: &str) -> Vec<kentos_geometry_core::entity::Shape> {
    r.features(name).entities.iter().map(|e| shape(e)).collect()
}

/// The cost raster (the one of the input) and, while Yükseklik modeliyle is
/// on, the surface: the run's rasters in that order.
fn rasters_of<'a>(r: &Resolved<'a>, ctx: &RunContext<'_>) -> Result<Vec<&'a RasterEntity>, String> {
    let mut rasters = rasters_for(r, ctx, true)?;
    if !r.flag("useSurface") {
        return Ok(rasters);
    }
    let surface: Vec<&RasterEntity> = r
        .features("surface")
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Raster(x) => Some(x),
            _ => None,
        })
        .collect();
    match surface.as_slice() {
        [] => return Err("Yükseklik modelini seçin: Yükseklik modeliyle açık.".into()),
        [one] => rasters.push(one),
        many => {
            return Err(format!(
                "{} raster seçili; yükseklik modeli için tek raster seçin.",
                many.len()
            ));
        }
    }
    Ok(rasters)
}

/// The sources and then the second objects (destinations, second ends): the shapes and how many are the first.
fn ends_of(r: &Resolved<'_>, second: &str) -> (Vec<kentos_geometry_core::entity::Shape>, usize) {
    let mut shapes = shapes_of(r, "sources");
    let first = shapes.len();
    shapes.extend(shapes_of(r, second));
    (shapes, first)
}

fn raster_entity(
    grid: &kentos_raster::grid::Grid,
    (bands, sample): (u32, kentos_contracts::RasterSample),
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
        },
    })
}

/// Says the objects that fell on no cell.
fn warn_outside(n: u64, what: &str, feedback: &mut dyn Feedback) {
    if n > 0 {
        feedback.warn(format!(
            "{} {what} rasterin hücrelerine düşmediği için alınmadı.",
            count_words(n)
        ));
    }
}

/// Runs a tool whose result is a raster over the input raster (and the
/// surface): the file beside it (or at the path given), its object right
/// above its layer, the summary `said` writes from the notes.
pub fn run_raster(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    shapes: (Vec<kentos_geometry_core::entity::Shape>, &str),
    (suffix, label): (&str, &str),
    said: &dyn Fn(&Notes, u64, &mut dyn Feedback) -> String,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = |feedback: &mut dyn Feedback| -> Result<RunResult, String> {
        let rasters = rasters_of(r, ctx)?;
        let names = names_of(&rasters, ctx);
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let first = &rasters[0].raster;
        let path =
            files.output_path(r.text("output"), Some(Beside::from(first)), suffix, ".tif")?;
        let ran = drive(
            &*files,
            &rasters,
            &spec,
            shapes.0,
            Some(&path),
            feedback,
            label,
        )?;
        feedback.progress(1.0, label);
        let Some((bands, sample, style, grid)) = ran.raster else {
            return Err("Çözümleme raster vermedi.".into());
        };
        warn_outside(ran.notes.distance.outside, shapes.1, feedback);
        let add: Vec<Entity> = r
            .flag("add")
            .then(|| {
                raster_entity(
                    &grid,
                    (bands, sample),
                    &path,
                    first.srid,
                    style,
                    &r.layer("layer").id,
                )
            })
            .into_iter()
            .collect();
        let cells = u64::from(grid.width) * u64::from(grid.height);
        let summary = format!(
            "{} × {} hücrelik raster; “{path}” yazıldı.{}",
            count_words(u64::from(grid.width)),
            count_words(u64::from(grid.height)),
            said(&ran.notes, cells, feedback)
        );
        let mut result = RunResult {
            changes: (!add.is_empty()).then(|| ChangeSet {
                add,
                ..ChangeSet::default()
            }),
            summary: Some(summary),
            above: Some(rasters[0].base.layer_id.clone()),
            ..RunResult::default()
        };
        result.outputs.insert("file".to_owned(), json!(path));
        Ok(result)
    };
    go(feedback).unwrap_or_else(broke)
}

/// En düşük maliyetli yol: the paths on a new layer right above the cost raster's, their numbers as attributes.
pub fn run_paths(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    shapes: Vec<kentos_geometry_core::entity::Shape>,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let label = "En düşük maliyetli yollar bulunuyor";
    let go = |feedback: &mut dyn Feedback| -> Result<RunResult, String> {
        let rasters = rasters_of(r, ctx)?;
        let names = names_of(&rasters, ctx);
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let ran = drive(&*files, &rasters, &spec, shapes, None, feedback, label)?;
        let f = ran.features.ok_or("Çözümleme nesne vermedi.")?;
        feedback.progress(1.0, label);
        let d = &ran.notes.distance;
        warn_outside(d.outside, "başlangıç nesnesi", feedback);
        if !d.unreached.is_empty() {
            let list: Vec<String> = d.unreached.iter().map(u32::to_string).collect();
            feedback.warn(format!(
                "{} varışa yol yok (erişilemiyor ya da rasterin değerli hücrelerine düşmüyor): {}.",
                count_words(d.unreached.len() as u64),
                list.join(", ")
            ));
        }
        let layer = r.layer("layer").id.clone();
        let add = objects_of(&f, &layer, &|k| attrs_of(&f, k), None);
        let summary = if f.is_empty() {
            "Yol bulunamadı.".to_owned()
        } else {
            format!("{} yol yazıldı.", count_words(f.len() as u64))
        };
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
    go(feedback).unwrap_or_else(broke)
}

/// Uzaklık yüzeyi from objects: the point job over the sources, on their
/// box with its margin or a raster's grid; the raster right below the
/// sources' layer.
pub fn run_from_objects(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = |feedback: &mut dyn Feedback| -> Result<RunResult, String> {
        let entities = &r.features("sources").entities;
        let Some(first) = entities.first() else {
            return Err("Kaynakları seçin: nokta, çizgi ya da alan.".into());
        };
        let input = PointInput::Lines {
            shapes: entities.iter().map(|e| shape(e)).collect(),
            weights: None,
        };
        let srid = ctx.doc.settings().srid;
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
            Some(Beside::named(layer_name, "uzaklik")),
            "-uzaklik",
            ".tif",
        )?;
        let label = "Uzaklık yüzeyi hesaplanıyor";
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
        warn_outside(notes.outside as u64, "kaynak", feedback);
        let add: Vec<Entity> = r
            .flag("add")
            .then(|| {
                raster_entity(
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
            "{} kaynaktan {} × {} hücrelik raster; “{path}” yazıldı.",
            count_words(notes.taken as u64),
            count_words(u64::from(grid.width)),
            count_words(u64::from(grid.height))
        );
        if notes.empty > 0 {
            summary.push_str(&format!(
                " En büyük uzaklığın ötesinde {} hücre değersiz.",
                count_words(notes.empty)
            ));
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
    go(feedback).unwrap_or_else(broke)
}
