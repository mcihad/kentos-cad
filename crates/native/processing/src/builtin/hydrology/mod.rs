//! Hidroloji (docs/adr/0235): Çukur doldur, Akış yönü, Akış birikimi and
//! Topografik nemlilik indisi (the raster core's operation job writing a
//! raster beside the DEM, its object right above the DEM's layer), Döküm
//! noktası, Noktadan havza, Havzalar and Dere ağı (its objects on a new layer
//! right above the DEM's), each in the run's one step.

mod tools;

use std::collections::BTreeMap;

use kentos_contracts::{Entity, RasterEntity, RasterFields};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::Shape;
use kentos_native_application::geometry::shape;
use kentos_raster::ops::Notes;
use kentos_raster::vector::Features;
use serde_json::{Value, json};

use super::pointcloud::{broke, count_words};
use super::raster_ops::{drive, names_of, rasters_for, spec_of};
use super::raster_vector::objects_of;
use super::surface::{base, files_of};
use crate::files::Beside;
use crate::types::{ChangeSet, Feedback, Resolved, RunContext, RunResult};

pub use tools::{
    accumulation, basins, fill, flow_direction, pour_point, streams, watershed, wetness,
};

pub const HYDROLOGY: &str = "hydrology";

/// A number at most `decimals` decimals, trailing zeros dropped.
pub fn trimmed(v: f64, decimals: usize) -> String {
    let t = fixed(v, decimals);
    if t.contains('.') {
        t.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        t
    }
}

/// An attribute's text: counts and numbers whole, areas and lengths to the millimetre, slopes to five decimals.
pub fn attr_text(field: &str, v: f64) -> String {
    match field {
        "Eğim" => trimmed(v, 5),
        "Alan" | "Uzunluk" | "Düşü" | "Km" | "Uzaklık" => trimmed(v, 3),
        _ => trimmed(v, 0),
    }
}

/// Feature `k`'s attributes: its named numbers' texts.
pub fn attrs_of(f: &Features, k: usize) -> BTreeMap<String, String> {
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

/// The shapes of a features parameter as the core reads them.
fn shapes_of(r: &Resolved<'_>, name: &str) -> Vec<Shape> {
    r.features(name).entities.iter().map(|e| shape(e)).collect()
}

/// Runs a tool whose result is a raster: the file beside the DEM (or at the
/// path given), its object, the summary `said` writes from the notes.
pub fn run_raster(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    (suffix, label): (&str, &str),
    said: &dyn Fn(&Notes) -> String,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = || -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, true)?;
        let names = names_of(&rasters, ctx);
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let first = &rasters[0].raster;
        let path =
            files.output_path(r.text("output"), Some(Beside::from(first)), suffix, ".tif")?;
        let ran = drive(
            &*files,
            &rasters,
            &spec,
            Vec::new(),
            Some(&path),
            feedback,
            label,
        )?;
        feedback.progress(1.0, label);
        let Some((bands, sample, style, grid)) = ran.raster else {
            return Err("Çözümleme raster vermedi.".into());
        };
        let add: Vec<Entity> = r
            .flag("add")
            .then(|| {
                Entity::Raster(RasterEntity {
                    base: base(&r.layer("layer").id, BTreeMap::new(), None),
                    raster: RasterFields {
                        affine: grid.affine,
                        width: grid.width,
                        height: grid.height,
                        bands,
                        sample,
                        asset: None,
                        file: Some(path.clone()),
                        url: None,
                        srid: first.srid,
                        style,
                        opacity: None,
                    },
                })
            })
            .into_iter()
            .collect();
        let summary = format!(
            "{} × {} hücrelik raster; “{path}” yazıldı.{}",
            count_words(u64::from(grid.width)),
            count_words(u64::from(grid.height)),
            said(&ran.notes)
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
    go().unwrap_or_else(broke)
}

/// What an object tool makes of its features and the notes: the summary (the warnings said through the feedback).
pub type Said<'a> = dyn Fn(&Features, &Notes, &mut dyn Feedback) -> String + 'a;

/// Runs a tool whose result is objects (the points or routes of `shapes`
/// handed over): on a new layer right above the DEM's, with their numbers as
/// attributes; points with their pour cell's height when `z` says so.
pub fn run_objects(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    shapes: Option<&str>,
    label: &str,
    said: &Said<'_>,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = || -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, true)?;
        let names = names_of(&rasters, ctx);
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let shapes = shapes.map(|s| shapes_of(r, s)).unwrap_or_default();
        let ran = drive(&*files, &rasters, &spec, shapes, None, feedback, label)?;
        let f = ran.features.ok_or("Çözümleme nesne vermedi.")?;
        feedback.progress(1.0, label);
        let layer = r.layer("layer").id.clone();
        let add = objects_of(&f, &layer, &|k| attrs_of(&f, k), None);
        let summary = said(&f, &ran.notes, feedback);
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

/// The points of the input that no cell took, said.
pub fn warn_skipped(n: &Notes, feedback: &mut dyn Feedback) {
    if !n.hydro.skipped.is_empty() {
        feedback.warn(format!(
            "{} nokta rasterin dışında ya da değersiz hücrede kaldığı için atlandı.",
            count_words(n.hydro.skipped.len() as u64)
        ));
    }
}
