//! Raster işlemleri and Raster istatistiği (docs/adr/0233): nine tools over
//! one or more rasters, each a run of the raster core's operation job
//! (`kentos_raster::ops`). The inputs are put in the run's order (the
//! layers from the top of the panel down, on a layer the later first), each
//! opened through the host's files and read where it is; a raster result is
//! a tiled GeoTIFF written beside the first input (or where asked) and a
//! raster object right above the first input's layer, in the run's one
//! step; Bölgesel istatistik writes its statistic into the zones' field and
//! gives a table, Histogram a table.

mod tools;

use std::collections::BTreeMap;

use kentos_contracts::{Entity, RasterEntity, RasterFields};
use kentos_domain::Slot;
use kentos_geometry_core::display::fixed;
use kentos_native_application::geometry::shape;
use kentos_raster::inputs::Input;
use kentos_raster::ops::{Histogram, Notes, OpsFinished, OpsJob, OpsSpec, ZoneFigures};
use kentos_raster::stats::Stat;
use serde_json::{Value, json};

use super::pointcloud::{STOPPED, broke, count_words};
use super::queries::{mean_scale, with_attr};
use super::surface::{base, files_of};
use crate::features::{raster_names, raster_order};
use crate::files::{Beside, Files};
use crate::types::{ChangeSet, Feedback, Patch, Resolved, RunContext, RunResult};

pub use tools::{
    calculator, cell_statistics, clip_by_mask, focal_statistics, histogram, mosaic, reclassify,
    resample, zonal_statistics,
};

pub const OPS: &str = "rasterOps";
pub const STATS: &str = "rasterStats";

/// The input's rasters in the run's order (§2): the layers from the top
/// of the panel down; on a layer the one drawn later first.
pub fn rasters_in_order<'a>(r: &Resolved<'a>, ctx: &RunContext<'_>) -> Vec<&'a RasterEntity> {
    let mut out: Vec<&RasterEntity> = r
        .features("input")
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Raster(x) => Some(x),
            _ => None,
        })
        .collect();
    raster_order(&mut out, ctx.doc);
    out
}

/// The rasters' names in an expression (§3): the layer's; a second raster
/// on a layer `Ad (2)`, and so on, in the run's order.
pub fn names_of(rasters: &[&RasterEntity], ctx: &RunContext<'_>) -> Vec<String> {
    raster_names(rasters, ctx.doc)
}

/// What a run gave.
pub struct Ran {
    /// A raster result: its bands, samples, look and grid.
    pub raster: Option<(
        u32,
        kentos_contracts::RasterSample,
        kentos_contracts::RasterStyle,
        kentos_raster::grid::Grid,
    )>,
    pub zones: Vec<ZoneFigures>,
    pub histogram: Option<Histogram>,
    /// A vectorizing run's features (docs/adr/0234).
    pub features: Option<kentos_raster::vector::Features>,
    /// ROC ile doğrulama's figures (docs/adr/0237 §8).
    pub roc: Option<kentos_raster::suitability::roc::Roc>,
    pub notes: Notes,
}

/// The run's settings: the tool's, and each input's place, nodata, name and look.
pub fn spec_of(
    ctx: &RunContext<'_>,
    rasters: &[&RasterEntity],
    names: &[String],
    tool: Value,
) -> Result<OpsSpec, String> {
    let srid = ctx.doc.settings().srid;
    let geographic = kentos_project::systems::own(ctx.doc.settings())
        .and_then(|s| s.system)
        .is_some_and(|s| s.is_geographic());
    serde_json::from_value(json!({
        "tool": tool,
        "inputs": rasters.iter().zip(names).map(|(x, name)| json!({
            "affine": x.raster.affine,
            "nodata": x.raster.style.nodata,
            "name": name,
            "style": x.raster.style,
        })).collect::<Vec<_>>(),
        "epsg": (srid > 0).then_some(srid),
        "geographic": geographic,
    }))
    .map_err(|e| format!("Çözümlemenin ayarları okunamadı: {e}"))
}

/// Opens the rasters through the host's files and runs `spec` over them
/// and `shapes`; a raster result written at `path`.
pub fn drive(
    files: &dyn Files,
    rasters: &[&RasterEntity],
    spec: &OpsSpec,
    shapes: Vec<kentos_geometry_core::entity::Shape>,
    path: Option<&str>,
    feedback: &mut dyn Feedback,
    label: &str,
) -> Result<Ran, String> {
    let mut readers = Vec::with_capacity(rasters.len());
    let mut blocks = Vec::with_capacity(rasters.len());
    let mut jpegs = Vec::with_capacity(rasters.len());
    for x in rasters {
        let open = files.open_raster(&x.raster)?;
        readers.push(Input::new(
            open.reader,
            x.raster.affine,
            x.raster.style.nodata,
        )?);
        blocks.push(open.block);
        jpegs.push(open.jpeg);
    }
    let (mut job, header) = OpsJob::new(readers, spec, shapes, kentos_raster::par::threads())?;
    let raster = job
        .result()
        .zip(job.style())
        .map(|((bands, sample), style)| (bands, sample, style, job.grid()));
    let mut sink = match (path, &raster) {
        (Some(p), Some(_)) => Some(files.create(p)?),
        _ => None,
    };
    if let Some(s) = sink.as_mut() {
        s.write(&header)?;
    }
    while !job.done() {
        if feedback.canceled() {
            return Err(STOPPED.to_owned());
        }
        let mut got = Vec::new();
        for (k, need) in job.needs() {
            got.push((k, need, (blocks[k as usize])(&need)?));
        }
        for (k, need, stream) in job.put_all(got)? {
            let (pixels, components) = (jpegs[k as usize])(&stream)?;
            job.put_pixels(k, &need, pixels, components)?;
        }
        let bytes = job.step()?;
        if let Some(s) = sink.as_mut() {
            s.write(&bytes)?;
        }
        feedback.progress(0.97 * job.share(), label);
    }
    let notes = job.notes().clone();
    // The look as the run left it (Değişim tespiti's is stretched by its values).
    let raster = raster
        .map(|(bands, sample, style, grid)| (bands, sample, job.style().unwrap_or(style), grid));
    let mut ran = Ran {
        raster,
        zones: Vec::new(),
        histogram: None,
        features: None,
        roc: None,
        notes,
    };
    match (job.finish()?, sink) {
        (OpsFinished::Raster { tail, header }, Some(mut s)) => {
            s.write(&tail)?;
            s.patch(0, &header)?;
            s.finish()?;
        }
        (OpsFinished::Zones(z), _) => ran.zones = z,
        (OpsFinished::Histogram(h), _) => ran.histogram = Some(h),
        (OpsFinished::Features(f), _) => ran.features = Some(f),
        (OpsFinished::Roc(r), _) => ran.roc = Some(r),
        (OpsFinished::Weights | OpsFinished::Report, _) => {}
        _ => return Err("Çözümlemenin sonucu beklenen türde değil.".into()),
    }
    Ok(ran)
}

/// The area objects of a features parameter as the core reads them.
fn shapes_of(r: &Resolved<'_>, name: &str) -> Vec<kentos_geometry_core::entity::Shape> {
    r.features(name).entities.iter().map(|e| shape(e)).collect()
}

/// The rasters of the input, or why the tool does not run.
pub(crate) fn rasters_for<'a>(
    r: &Resolved<'a>,
    ctx: &RunContext<'_>,
    one: bool,
) -> Result<Vec<&'a RasterEntity>, String> {
    let rasters = rasters_in_order(r, ctx);
    match (rasters.len(), one) {
        (0, _) => Err("Raster seçin: bu araç raster ister.".into()),
        (1, true) => Ok(rasters),
        (n, true) => Err(format!("{n} raster seçili; bu araç tek raster ister.")),
        _ => Ok(rasters),
    }
}

/// Runs a tool whose result is a raster: the file, its object, the summary.
pub fn run_raster_op(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    (suffix, label): (&str, &str),
    (one, shapes): (bool, Option<&str>),
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = || -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, one)?;
        let names = names_of(&rasters, ctx);
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        // Only the rasters the run reads are opened (Raster hesaplayıcı's those its expression names, the grid's
        // first): the first names the file, gives the system, and its layer has the result's right above it.
        let keep = spec.reads()?;
        let rasters: Vec<&RasterEntity> = keep.iter().map(|&k| rasters[k]).collect();
        let spec = spec.reading(&keep);
        let first = &rasters[0].raster;
        let path =
            files.output_path(r.text("output"), Some(Beside::from(first)), suffix, ".tif")?;
        let shapes = shapes.map(|s| shapes_of(r, s)).unwrap_or_default();
        let ran = drive(
            &*files,
            &rasters,
            &spec,
            shapes,
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
                        dataset: None,
                    },
                })
            })
            .into_iter()
            .collect();
        let kind = spec_kind(&spec);
        let n = &ran.notes;
        let empty_all = n.empty_cells == u64::from(grid.width) * u64::from(grid.height);
        if empty_all {
            feedback.warn("Sonucun hiçbir hücresinde değer yok.".to_owned());
        }
        let mut summary = format!(
            "{} × {} hücrelik raster; “{path}” yazıldı.",
            count_words(u64::from(grid.width)),
            count_words(u64::from(grid.height)),
        );
        match kind {
            "clipByMask" => {
                summary.push_str(&format!(" Maskenin içinde {} hücre.", count_words(n.cells)))
            }
            "reclassify" if n.cells > 0 => summary.push_str(&format!(
                " Tablonun hiçbir kuralının tutmadığı {} hücre.",
                count_words(n.cells)
            )),
            "mosaic" | "cellStatistics" => summary.push_str(&format!(
                " {} raster birleşti.",
                count_words(rasters.len() as u64)
            )),
            _ => {}
        }
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

fn spec_kind(spec: &OpsSpec) -> &'static str {
    use kentos_raster::ops::OpsTool;
    match spec.tool {
        OpsTool::Calculator { .. } => "calculator",
        OpsTool::Reclassify { .. } => "reclassify",
        OpsTool::ClipByMask { .. } => "clipByMask",
        OpsTool::Mosaic { .. } => "mosaic",
        OpsTool::Resample { .. } => "resample",
        OpsTool::ZonalStatistics { .. } => "zonalStatistics",
        OpsTool::Histogram { .. } => "histogram",
        OpsTool::FocalStatistics { .. } => "focalStatistics",
        OpsTool::CellStatistics { .. } => "cellStatistics",
        OpsTool::ToPolygons { .. } => "toPolygons",
        OpsTool::ToLines { .. } => "toLines",
        OpsTool::ToPoints { .. } => "toPoints",
        OpsTool::CaptureLine { .. } => "captureLine",
        OpsTool::CloseArea { .. } => "closeArea",
        OpsTool::Fill { .. } => "fill",
        OpsTool::FlowDirection { .. } => "flowDirection",
        OpsTool::FlowAccumulation { .. } => "flowAccumulation",
        OpsTool::Wetness { .. } => "wetness",
        OpsTool::PourPoint { .. } => "pourPoint",
        OpsTool::Watershed { .. } => "watershed",
        OpsTool::Basins { .. } => "basins",
        OpsTool::Streams { .. } => "streams",
        OpsTool::Distance { .. } => "distance",
        OpsTool::CostDistance { .. } => "costDistance",
        OpsTool::CostPath { .. } => "costPath",
        OpsTool::CostCorridor { .. } => "costCorridor",
        OpsTool::FuzzyMembership { .. } => "fuzzyMembership",
        OpsTool::FuzzyOverlay { .. } => "fuzzyOverlay",
        OpsTool::WeightedSum { .. } => "weightedSum",
        OpsTool::WeightedOverlay { .. } => "weightedOverlay",
        OpsTool::Pairwise { .. } => "pairwise",
        OpsTool::Roc { .. } => "roc",
        OpsTool::Composite { .. } => "composite",
        OpsTool::Band { .. } => "band",
        OpsTool::Index { .. } => "index",
        OpsTool::Supervised { .. } => "supervised",
        OpsTool::Unsupervised { .. } => "unsupervised",
        OpsTool::Accuracy { .. } => "accuracy",
        OpsTool::Change { .. } => "change",
        OpsTool::Pansharpen { .. } => "pansharpen",
    }
}

/// A figure as the table and the fields write it: counts whole, the rest at `decimals`.
pub fn figure(v: Option<f64>, whole: bool, decimals: u32) -> String {
    match v {
        None => String::new(),
        Some(x) if whole => fixed(x, 0),
        Some(x) => fixed(x, decimals as usize),
    }
}

/// Runs Bölgesel istatistik: the statistic into each zone's field, the table.
pub fn run_zonal(r: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let mut go = || -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, true)?;
        let zones: Vec<&Entity> = r.features("zones").entities.clone();
        if zones.is_empty() {
            return Err("Bölgeleri seçin: kapalı alanlar.".into());
        }
        let stat = Stat::from_key(r.text("stat")).unwrap_or(Stat::Mean);
        let names = names_of(&rasters, ctx);
        let tool = json!({ "kind": "zonalStatistics", "band": r.number("band").unwrap_or(1.0) as u32, "stat": stat.key() });
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let label = "Bölgelerin istatistikleri hesaplanıyor";
        let shapes = zones.iter().map(|e| shape(e)).collect();
        let ran = drive(&*files, &rasters, &spec, shapes, None, feedback, label)?;
        feedback.progress(1.0, label);
        let output = r.text("output").trim().to_owned();
        let decimals = r.number("decimals").unwrap_or(3.0).clamp(0.0, 12.0) as u32;
        let mut update = Vec::new();
        let mut empty = 0usize;
        let mut rows = Vec::new();
        let shown_extra = !matches!(
            stat,
            Stat::Count | Stat::Sum | Stat::Mean | Stat::Min | Stat::Max | Stat::Std
        );
        for (z, f) in zones.iter().zip(&ran.zones) {
            if f.moments.n == 0 {
                empty += 1;
            }
            let scale = mean_scale(ctx, z, &output).unwrap_or(decimals);
            let text = figure(f.value, stat.whole(), scale);
            if !output.is_empty()
                && let Some(attrs) =
                    with_attr(ctx, z, &output, (!text.is_empty()).then_some(text.as_str()))
            {
                update.push(Patch {
                    id: Slot(z.base().id),
                    attrs: Some(attrs),
                    label: None,
                    zs: None,
                });
            }
            let m = &f.moments;
            let name = z
                .base()
                .label
                .clone()
                .unwrap_or_else(|| format!("#{}", z.base().id));
            let mut row = vec![
                name,
                fixed(m.n as f64, 0),
                figure(m.sum(), false, decimals),
                figure(m.mean(), false, decimals),
                figure(m.min(), false, decimals),
                figure(m.max(), false, decimals),
                figure(m.std(), false, decimals),
            ];
            if shown_extra {
                row.push(figure(f.value, stat.whole(), decimals));
            }
            rows.push(row);
        }
        if empty > 0 {
            feedback.warn(format!(
                "{} bölgenin içinde değeri olan hücre merkezi yok.",
                count_words(empty as u64)
            ));
        }
        let mut columns = vec![
            "Nesne",
            "Sayı",
            "Toplam",
            "Ortalama",
            "En küçük",
            "En büyük",
            "Standart sapma",
        ];
        if shown_extra {
            columns.push(stat.label());
        }
        let count = update.len();
        let changed: Vec<u32> = update.iter().map(|u: &Patch| u.id.0).collect();
        let mut result = RunResult {
            changes: (!update.is_empty()).then(|| ChangeSet {
                update,
                ..ChangeSet::default()
            }),
            summary: Some(if output.is_empty() {
                format!(
                    "{} bölgenin istatistikleri tabloda.",
                    count_words(zones.len() as u64)
                )
            } else {
                format!(
                    "{count} bölgeye “{output}” yazıldı ({}).",
                    stat.label().to_lowercase()
                )
            }),
            ..RunResult::default()
        };
        result.outputs.insert(
            "table".to_owned(),
            json!({ "columns": columns, "rows": rows }),
        );
        result.outputs.insert("changed".to_owned(), json!(changed));
        result.outputs.insert("count".to_owned(), json!(count));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}

/// Runs Histogram: the table of intervals.
pub fn run_histogram(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let mut go = || -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, true)?;
        let names = names_of(&rasters, ctx);
        let (lo, hi) = (r.number("min"), r.number("max"));
        let tool = json!({
            "kind": "histogram",
            "band": r.number("band").unwrap_or(1.0) as u32,
            "bins": r.number("bins").unwrap_or(20.0) as u32,
            "min": lo,
            "max": hi,
        });
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let label = "Histogram çıkarılıyor";
        let ran = drive(&*files, &rasters, &spec, Vec::new(), None, feedback, label)?;
        feedback.progress(1.0, label);
        let h = ran.histogram.ok_or("Histogram çıkmadı.")?;
        if h.valid == 0 {
            return Ok(RunResult {
                summary: Some("Bandın hiçbir hücresinde değer yok.".into()),
                ..RunResult::default()
            });
        }
        let rows = histogram_rows(&h);
        let mut summary = format!(
            "{} hücre, {} ile {} arası {} aralık.",
            count_words(h.valid),
            fixed(h.lo, 3),
            fixed(h.hi, 3),
            count_words(h.counts.len() as u64)
        );
        if h.empty > 0 {
            summary.push_str(&format!(" Değeri olmayan {} hücre.", count_words(h.empty)));
        }
        if h.below + h.above > 0 {
            summary.push_str(&format!(
                " Aralığın altında {}, üstünde {} hücre.",
                count_words(h.below),
                count_words(h.above)
            ));
        }
        let mut result = RunResult {
            summary: Some(summary),
            ..RunResult::default()
        };
        result.outputs.insert(
            "table".to_owned(),
            json!({ "columns": ["Aralık", "Alt sınır", "Üst sınır", "Sayı", "Oran (%)", "Birikimli (%)"], "rows": rows }),
        );
        Ok(result)
    };
    go().unwrap_or_else(broke)
}

/// The histogram's rows: each interval's bounds, count and shares (§11).
pub fn histogram_rows(h: &Histogram) -> Vec<Vec<String>> {
    let n = h.counts.len() as f64;
    let total = h.counts.iter().sum::<u64>().max(1) as f64;
    let mut so_far = 0u64;
    h.counts
        .iter()
        .enumerate()
        .map(|(k, &c)| {
            so_far += c;
            let (a, b) = if h.hi > h.lo {
                (
                    h.lo + (h.hi - h.lo) * k as f64 / n,
                    if k + 1 == h.counts.len() {
                        h.hi
                    } else {
                        h.lo + (h.hi - h.lo) * (k + 1) as f64 / n
                    },
                )
            } else {
                (h.lo, h.hi)
            };
            vec![
                (k + 1).to_string(),
                fixed(a, 3),
                fixed(b, 3),
                c.to_string(),
                fixed(100.0 * c as f64 / total, 2),
                fixed(100.0 * so_far as f64 / total, 2),
            ]
        })
        .collect()
}
