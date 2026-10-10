//! Çok boyutlu veri (docs/adr/0243 §8–§10; the web's `builtin/multidim/`):
//! Kesit (a raster's values along lines: a table, and the lines with the
//! values as their vertices' elevations on a new layer), Zaman serisi
//! (values at points over a NetCDF dataset's time steps, or over a raster's
//! bands: a table) and Mesh hesaplayıcı (a new dataset on a mesh: a UGRID
//! file beside the source and a raster showing it right above the source's
//! layer). The jobs are the raster core's (`kentos_raster::multidim`); here
//! their runs of the file are read through the host's files.

pub mod tools;

use std::collections::BTreeMap;

use kentos_contracts::{
    DatasetDim, Entity, PathEntity, RasterDataset, RasterEntity, RasterFields, Workspace,
};
use kentos_geometry_core::entity::Shape;
use kentos_native_application::geometry::shape;
use kentos_raster::inputs::Input;
use kentos_raster::multidim::calc::{CalcSpec, MeshCalc, Summary};
use kentos_raster::multidim::points::PointsJob;
use kentos_raster::multidim::series::{NamedPoint, TimeJob, bands_job, named_points};
use kentos_raster::multidim::{Finished, Table, profile};
use serde_json::{Value, json};

use super::pointcloud::{STOPPED, broke};
use super::surface::{base, files_of};
use crate::files::{Beside, DecodeJpeg, Files, RasterOpen, ReadBlock};
use crate::types::{ChangeSet, Feedback, Resolved, RunContext, RunResult};

pub const MULTIDIM: &str = "multidim";

/// A table as the tools' table output.
fn table_json(t: &Table) -> Value {
    json!({ "columns": t.columns, "rows": t.rows })
}

/// The one raster of the input, or why not.
fn one_raster<'a>(r: &Resolved<'a>) -> Result<&'a RasterEntity, String> {
    let rasters: Vec<&RasterEntity> = r
        .features("input")
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Raster(x) => Some(x),
            _ => None,
        })
        .collect();
    match rasters.as_slice() {
        [] => Err("Raster seçin: bu araç raster ister.".into()),
        [one] => Ok(one),
        many => Err(format!(
            "{} raster seçili; bu araç tek raster ister.",
            many.len()
        )),
    }
}

/// The project's axes' names, east's first (a CAD project's X, Y; a CBS project's Y, X).
fn axes(ctx: &RunContext<'_>) -> [String; 2] {
    match ctx.doc.settings().project_type() {
        Some(Workspace::Cad) => ["X".into(), "Y".into()],
        _ => ["Y".into(), "X".into()],
    }
}

/// The finished job's warnings through the feedback.
fn warned(f: &Finished, feedback: &mut dyn Feedback) {
    for w in &f.warnings {
        feedback.warn(w.clone());
    }
}

/// How a raster's blocks are read and its JPEG blocks decoded.
struct Blocks<'f> {
    block: ReadBlock<'f>,
    jpeg: DecodeJpeg<'f>,
}

/// A points job run to its end over the raster's blocks.
fn drive_points(
    job: &mut PointsJob,
    open: &Blocks<'_>,
    feedback: &mut dyn Feedback,
    label: &str,
) -> Result<(), String> {
    while !job.done() {
        if feedback.canceled() {
            return Err(STOPPED.to_owned());
        }
        let needs = job.values.needs();
        for (k, need) in needs.iter().enumerate() {
            let bytes = (open.block)(need)?;
            if let Some(stream) = job.values.put(k, &bytes)? {
                let (pixels, components) = (open.jpeg)(&stream)?;
                job.values.put_pixels(k, pixels, components)?;
            }
        }
        job.values.step()?;
        feedback.progress(0.97 * job.values.share(), label);
    }
    Ok(())
}

/// The raster opened as an analysis input (a NetCDF slice as the raster shows it).
fn input_of<'f>(
    files: &'f dyn Files,
    raster: &RasterEntity,
) -> Result<(Input, Blocks<'f>), String> {
    let RasterOpen {
        reader,
        block,
        jpeg,
    } = files.open_raster(&raster.raster)?;
    let input = Input::new(reader, raster.raster.affine, raster.raster.style.nodata)?;
    Ok((input, Blocks { block, jpeg }))
}

/// The line objects' shapes (the core walks a multi-part line part by part).
fn line_shapes(r: &Resolved<'_>) -> Vec<Shape> {
    r.features("lines")
        .entities
        .iter()
        .map(|e| shape(e))
        .collect()
}

/// Runs Kesit: the table, and the lines with values on the layer.
pub fn run_profile(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let mut go = || -> Result<RunResult, String> {
        let raster = one_raster(r)?;
        let lines = line_shapes(r);
        let [_, a, _, _, c, _] = raster.raster.affine;
        let step = r
            .number("step")
            .unwrap_or_else(|| kentos_geometry_core::jsmath::js_hypot(a, c));
        let band = r.number("band").unwrap_or(1.0).max(1.0) as usize - 1;
        let (input, open) = input_of(&*files, raster)?;
        let mut job = profile::job(input, &lines, step, band, axes(ctx))?;
        let label = "Kesit çıkarılıyor";
        drive_points(&mut job, &open, feedback, label)?;
        feedback.progress(1.0, label);
        let finished = job.finish();
        warned(&finished, feedback);
        let layer = r.layer("layer").id.clone();
        let add: Vec<Entity> = if r.flag("draw") {
            finished
                .pieces
                .iter()
                .map(|p| {
                    let mut attrs = BTreeMap::new();
                    attrs.insert("Çizgi".to_owned(), p.line.to_string());
                    Entity::Polyline(PathEntity {
                        base: base(&layer, attrs, None),
                        pts: p
                            .points
                            .iter()
                            .map(|q| kentos_contracts::Vec2 { x: q[0], y: q[1] })
                            .collect(),
                        bulges: None,
                        holes: None,
                        zs: Some(p.points.iter().map(|q| Some(q[2])).collect()),
                        parts: None,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut result = RunResult {
            changes: (!add.is_empty()).then(|| ChangeSet {
                add,
                ..ChangeSet::default()
            }),
            summary: Some(finished.summary.clone()),
            above: Some(raster.base.layer_id.clone()),
            ..RunResult::default()
        };
        if let Some(t) = &finished.table {
            result.outputs.insert("table".to_owned(), table_json(t));
        }
        Ok(result)
    };
    go().unwrap_or_else(broke)
}

/// The points of the input: every point of a multi-point object, its `ad`.
fn points_of(r: &Resolved<'_>) -> Vec<NamedPoint> {
    named_points(
        r.features("points")
            .entities
            .iter()
            .map(|e| (e.base().attrs.get("ad").cloned(), shape(e)))
            .collect(),
    )
}

/// Runs Zaman serisi: the table.
pub fn run_series(
    r: &Resolved<'_>,
    _ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let mut go = || -> Result<RunResult, String> {
        let raster = one_raster(r)?;
        let points = points_of(r);
        let label = "Zaman serisi okunuyor";
        let timed = raster
            .raster
            .dataset
            .as_ref()
            .is_some_and(|d| d.dims.iter().any(|x| x.time));
        let finished = if timed {
            let mut open = files.open_cube(&raster.raster)?;
            let xy: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
            let mut series = open
                .cube
                .series(
                    &open.part,
                    raster.raster.affine,
                    raster.raster.style.nodata,
                    &xy,
                )
                .map_err(|e| e.0)?;
            let total = series.runs.len().max(1);
            for (k, run) in series.runs.clone().iter().enumerate() {
                if feedback.canceled() {
                    return Err(STOPPED.to_owned());
                }
                series.put(run.offset, (open.read)(run.offset, run.len)?);
                if k % 256 == 0 {
                    feedback.progress(0.97 * k as f64 / total as f64, label);
                }
            }
            let mut job = TimeJob::new(series, &points)?;
            job.needs();
            job.step()?;
            job.finish()?
        } else {
            let (input, open) = input_of(&*files, raster)?;
            let mut job = bands_job(input, &points)?;
            drive_points(&mut job, &open, feedback, label)?;
            job.finish()
        };
        feedback.progress(1.0, label);
        warned(&finished, feedback);
        let mut result = RunResult {
            summary: Some(finished.summary.clone()),
            ..RunResult::default()
        };
        if let Some(t) = &finished.table {
            result.outputs.insert("table".to_owned(), table_json(t));
        }
        Ok(result)
    };
    go().unwrap_or_else(broke)
}

/// The summary's settings name.
fn summary_of(text: &str) -> Summary {
    match text {
        "max" => Summary::Max,
        "min" => Summary::Min,
        "mean" => Summary::Mean,
        "sum" => Summary::Sum,
        _ => Summary::None,
    }
}

/// Runs Mesh hesaplayıcı: the UGRID file, the raster showing it, the summary.
pub fn run_calc(r: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let mut go = || -> Result<RunResult, String> {
        let raster = one_raster(r)?;
        let Some(dataset) = raster.raster.dataset.as_ref().filter(|d| d.mesh.is_some()) else {
            return Err(
                "Mesh hesaplayıcı mesh raster ister: Mesh ekle ile eklenen bir raster seçin."
                    .into(),
            );
        };
        let spec = CalcSpec {
            expression: r.text("expression").to_owned(),
            summary: summary_of(r.text("summary")),
            name: r.text("name").to_owned(),
        };
        let mut open = files.open_cube(&raster.raster)?;
        let mut calc = MeshCalc::new(&mut open.cube, &open.part, &spec)?;
        let (variable, mesh, steps) = calc.output();
        let path = files.output_path(
            r.text("output"),
            Some(Beside::from(&raster.raster)),
            "-hesap",
            ".nc",
        )?;
        let label = "Mesh hesaplanıyor";
        while !calc.done() {
            if feedback.canceled() {
                return Err(STOPPED.to_owned());
            }
            for (k, (at, len)) in calc.needs()?.into_iter().enumerate() {
                calc.put(k, (open.read)(at, len)?)?;
            }
            calc.step()?;
            feedback.progress(0.9 * calc.share(), label);
        }
        let mut sink = files.create(&path)?;
        let finished = calc.finish(&mut |b: &[u8]| sink.write(b))?;
        sink.finish()?;
        feedback.progress(1.0, label);
        warned(&finished, feedback);
        // The raster showing the new dataset: the source's grid and look, its new file.
        let add: Vec<Entity> = r
            .flag("add")
            .then(|| {
                let dims: Vec<DatasetDim> = steps
                    .iter()
                    .map(|(values, time)| DatasetDim {
                        name: "time".to_owned(),
                        index: 0,
                        values: values.clone(),
                        time: *time,
                        units: None,
                    })
                    .collect();
                let follow = dataset.follow_time && dims.iter().any(|d| d.time);
                let mut style = raster.raster.style.clone();
                style.stretch = kentos_contracts::RasterStretch::MinMax;
                style.min = None;
                style.max = None;
                Entity::Raster(RasterEntity {
                    base: base(&r.layer("layer").id, BTreeMap::new(), None),
                    raster: RasterFields {
                        file: Some(path.clone()),
                        asset: None,
                        url: None,
                        sample: kentos_contracts::RasterSample::F32,
                        opacity: raster.raster.opacity,
                        style,
                        dataset: Some(RasterDataset {
                            variable: variable.clone(),
                            vector: None,
                            mesh: Some(mesh.to_owned()),
                            dims,
                            follow_time: follow,
                        }),
                        ..raster.raster.clone()
                    },
                })
            })
            .into_iter()
            .collect();
        let mut result = RunResult {
            changes: (!add.is_empty()).then(|| ChangeSet {
                add,
                ..ChangeSet::default()
            }),
            summary: Some(format!("{} “{path}” yazıldı.", finished.summary)),
            above: Some(raster.base.layer_id.clone()),
            ..RunResult::default()
        };
        result.outputs.insert("file".to_owned(), json!(path));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}
