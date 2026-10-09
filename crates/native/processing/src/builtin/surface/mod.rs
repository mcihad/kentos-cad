//! Yüzey analizi (docs/adr/0231): eight tools over a DEM, each a job of the
//! raster core (`kentos_raster`) driven over the host's files. The raster is
//! read where it is, its blocks decoded on the job's threads; the result is a
//! tiled GeoTIFF written beside it (`.yaziliyor`, renamed when whole) and a
//! raster object on a new layer, or lines on a new layer, in the run's one
//! step.

mod tools;

use std::collections::BTreeMap;

use kentos_contracts::{
    Entity, EntityBase, PathEntity, RasterEntity, RasterFields, RasterSample, RasterStyle, Vec2,
};
use kentos_raster::contours::Line;
use kentos_raster::job::{Finished, Job, Spec};
use serde_json::{Value, json};

use super::pointcloud::{STOPPED, broke, count_words};
use crate::files::{Beside, Files, NO_RASTER_FILES};
use crate::types::{
    ChangeSet, EnumOption, Feedback, ParamDef, ParamKind, Resolved, RunContext, RunResult,
    ScopeKind,
};

pub use tools::{
    aspect, color_relief, contours, curvature, hillshade, insolation, ruggedness, slope,
};

pub const CATEGORY: &str = "surface";

/// The raster parameter: one raster object.
pub fn raster_param() -> ParamDef {
    ParamDef::new(
        "input",
        "Raster",
        ParamKind::Features {
            kinds: Some(vec!["raster".to_owned()]),
            scopes: Some(vec![ScopeKind::Selection, ScopeKind::Layer]),
            writes: false,
        },
    )
    .describe("Çözümlenecek yükseklik rasteri (DEM): tek bir raster nesnesi.")
}

/// The band parameter.
pub fn band_param() -> ParamDef {
    whole("band", "Bant", 1, 1, 255).describe("Yüksekliklerin okunduğu bant (1'den).")
}

/// The z factor.
pub fn z_param() -> ParamDef {
    number("zFactor", "Z çarpanı", 1.0, 1e-6, 1e6, "").describe(
        "Yükseklikler bununla çarpılır: yüksekliklerin birimi metre değilse ya da düşey abartı için.",
    )
}

/// The gradient's method.
pub fn method_param() -> ParamDef {
    choice(
        "method",
        "Yöntem",
        &[
            ("horn", "Horn"),
            ("zevenbergenThorne", "Zevenbergen-Thorne"),
        ],
    )
    .describe(
        "Horn: sekiz komşu, gdaldem'in ve ArcGIS'in varsayılanı; Zevenbergen-Thorne: dört komşu.",
    )
}

/// A choice, its first option the default.
pub fn choice(name: &str, label: &str, options: &[(&str, &str)]) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Choice {
            options: options.iter().map(|(v, l)| EnumOption::new(v, l)).collect(),
        },
    )
    .default_value(json!(options[0].0))
}

/// A number parameter.
pub fn number(name: &str, label: &str, default: f64, min: f64, max: f64, unit: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min: Some(min),
            max: Some(max),
            integer: false,
            unit: unit.into(),
            placeholder: None,
        },
    )
    .default_value(json!(default))
}

/// A whole number parameter.
pub fn whole(name: &str, label: &str, default: u32, min: u32, max: u32) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min: Some(f64::from(min)),
            max: Some(f64::from(max)),
            integer: true,
            unit: String::new(),
            placeholder: None,
        },
    )
    .default_value(json!(default))
}

/// The result's layer (Çıktı katmanı): a new one goes right above the raster's (under it the
/// raster would hide it, docs/adr/0231 §2).
pub fn result_layer(name: &str, color: &str) -> ParamDef {
    let mut p = crate::builtin::pointcloud::result_layer_param(name, color);
    if let ParamKind::Layer { above, .. } = &mut p.kind {
        *above = Some("input".to_owned());
    }
    p
}

/// The run's raster: the one object of the input, or why not.
pub fn one_raster<'a>(r: &Resolved<'a>) -> Result<&'a RasterEntity, Box<RunResult>> {
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
        [one] => Ok(one),
        [] => Err(Box::new(RunResult::refused(
            "Raster seçin: çözümleme bir yükseklik rasteri (DEM) ister.".into(),
        ))),
        _ => Err(Box::new(RunResult::refused(format!(
            "{} raster seçili; çözümleme tek raster ister.",
            rasters.len()
        )))),
    }
}

/// The project's coordinate system as the raster core reads it; none for a local project.
pub(crate) fn system_of(ctx: &RunContext<'_>) -> Option<Value> {
    let system = kentos_project::systems::own(ctx.doc.settings())?.system?;
    serde_json::from_str(&kentos_geometry_core::api::json::to_string(&system)).ok()
}

/// The job's settings: the tool's, the raster's place, band, nodata and system.
fn spec_of(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    raster: &RasterFields,
    tool: Value,
) -> Result<Spec, String> {
    serde_json::from_value(json!({
        "tool": tool,
        "band": r.number("band").unwrap_or(1.0) as u32,
        "affine": raster.affine,
        "nodata": raster.style.nodata,
        "epsg": (raster.srid > 0).then_some(raster.srid),
        "system": system_of(ctx),
    }))
    .map_err(|e| format!("Çözümlemenin ayarları okunamadı: {e}"))
}

/// What a job gave: a raster result's bands, samples and look, or the lines and their Kot texts.
enum Ran {
    Raster {
        bands: u32,
        sample: RasterSample,
        style: RasterStyle,
    },
    Lines(Vec<Line>, Vec<String>),
}

/// Runs `spec` over the raster, a raster result written at `path`.
fn drive(
    files: &dyn Files,
    raster: &RasterFields,
    spec: &Spec,
    path: Option<&str>,
    feedback: &mut dyn Feedback,
    label: &str,
) -> Result<Ran, String> {
    let open = files.open_raster(raster)?;
    let (mut job, header) = Job::new(open.reader, spec, kentos_raster::par::threads())?;
    let result = job.result().zip(job.style());
    let levels = job.contours().copied();
    let mut sink = match path {
        Some(p) => Some(files.create(p)?),
        None => None,
    };
    if let Some(s) = sink.as_mut() {
        s.write(&header)?;
    }
    while !job.done() {
        if feedback.canceled() {
            return Err(STOPPED.to_owned());
        }
        let mut blocks = Vec::new();
        for need in job.needs() {
            blocks.push((need, (open.block)(&need)?));
        }
        for (need, stream) in job.put_all(blocks)? {
            let (pixels, components) = (open.jpeg)(&stream)?;
            job.put_pixels(&need, pixels, components)?;
        }
        let bytes = job.step()?;
        if let Some(s) = sink.as_mut() {
            s.write(&bytes)?;
        }
        feedback.progress(0.97 * job.share(), label);
    }
    match (job.finish()?, sink, result, levels) {
        (Finished::Raster { tail, header }, Some(mut s), Some(((bands, sample), style)), _) => {
            s.write(&tail)?;
            s.patch(0, &header)?;
            s.finish()?;
            Ok(Ran::Raster {
                bands,
                sample,
                style,
            })
        }
        (Finished::Lines(lines), None, None, Some(levels)) => {
            let texts = lines.iter().map(|l| levels.level_text(l.value)).collect();
            Ok(Ran::Lines(lines, texts))
        }
        _ => Err("Çözümlemenin sonucu beklenen türde değil.".into()),
    }
}

/// The result raster's object: the source's place and size, the result's kind and look.
fn raster_object(
    f: &RasterFields,
    path: &str,
    (bands, sample, style): (u32, RasterSample, RasterStyle),
    layer: &str,
) -> Entity {
    Entity::Raster(RasterEntity {
        base: base(layer, BTreeMap::new(), None),
        raster: RasterFields {
            affine: f.affine,
            width: f.width,
            height: f.height,
            bands,
            sample,
            asset: None,
            file: Some(path.to_owned()),
            url: None,
            srid: f.srid,
            style,
            opacity: None,
        },
    })
}

/// Runs a raster tool: the result file beside the raster (or where asked) and its object.
pub fn run_raster(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    (suffix, label): (&str, &str),
) -> RunResult {
    let (files, raster) = match (files_of(feedback), one_raster(r)) {
        (Ok(f), Ok(x)) => (f, x),
        (Err(e), _) | (_, Err(e)) => return *e,
    };
    let go = || -> Result<RunResult, String> {
        let f = &raster.raster;
        let spec = spec_of(r, ctx, f, tool)?;
        let path = files.output_path(r.text("output"), Some(Beside::from(f)), suffix, ".tif")?;
        let Ran::Raster {
            bands,
            sample,
            style,
        } = drive(&*files, f, &spec, Some(&path), feedback, label)?
        else {
            return Err("Çözümleme raster vermedi.".into());
        };
        feedback.progress(1.0, label);
        let add: Vec<Entity> = r
            .flag("add")
            .then(|| raster_object(f, &path, (bands, sample, style), &r.layer("layer").id))
            .into_iter()
            .collect();
        let mut result = RunResult {
            changes: (!add.is_empty()).then(|| ChangeSet {
                add,
                ..ChangeSet::default()
            }),
            summary: Some(format!(
                "{} × {} hücrelik raster; “{path}” yazıldı.",
                count_words(u64::from(f.width)),
                count_words(u64::from(f.height)),
            )),
            ..RunResult::default()
        };
        result.outputs.insert("file".to_owned(), json!(path));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}

/// The host's files, or why a raster tool does not run here (boxed: a run's result is large).
pub(crate) fn files_of(
    feedback: &dyn Feedback,
) -> Result<std::sync::Arc<dyn Files>, Box<RunResult>> {
    feedback
        .files()
        .ok_or_else(|| Box::new(RunResult::refused(NO_RASTER_FILES.to_owned())))
}

pub(crate) fn base(
    layer: &str,
    attrs: BTreeMap<String, String>,
    line_weight: Option<f64>,
) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.to_owned(),
        color: None,
        attrs,
        label: None,
        symbol: None,
        line_weight,
    }
}

/// The contour lines as polylines on the layer: their vertices at their
/// level, Kot (its text) and Tür; a main line 0.35 mm.
pub fn line_objects(lines: &[Line], texts: &[String], layer: &str) -> Vec<Entity> {
    lines
        .iter()
        .zip(texts)
        .map(|(l, text)| {
            let mut attrs = BTreeMap::new();
            attrs.insert("Kot".to_owned(), text.clone());
            attrs.insert(
                "Tür".to_owned(),
                if l.index { "Ana" } else { "Ara" }.to_owned(),
            );
            Entity::Polyline(PathEntity {
                base: base(layer, attrs, l.index.then_some(0.35)),
                pts: l.pts.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
                bulges: None,
                holes: None,
                zs: Some(vec![Some(l.value); l.pts.len()]),
                parts: None,
            })
        })
        .collect()
}

/// Runs Eş yükselti eğrileri: the lines on the layer.
pub fn run_lines(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
) -> RunResult {
    let (files, raster) = match (files_of(feedback), one_raster(r)) {
        (Ok(f), Ok(x)) => (f, x),
        (Err(e), _) | (_, Err(e)) => return *e,
    };
    let go = || -> Result<RunResult, String> {
        let f = &raster.raster;
        let spec = spec_of(r, ctx, f, tool)?;
        let label = "Eğriler çıkarılıyor";
        let Ran::Lines(lines, texts) = drive(&*files, f, &spec, None, feedback, label)? else {
            return Err("Çözümleme eğri vermedi.".into());
        };
        feedback.progress(1.0, label);
        let main = lines.iter().filter(|l| l.index).count();
        let vertices: usize = lines.iter().map(|l| l.pts.len()).sum();
        let summary = match (texts.first(), texts.last()) {
            (Some(lo), Some(hi)) => format!(
                "{} eğri ({} ana, {} ara), {} köşe; kotlar {lo} ile {hi} arası.",
                count_words(lines.len() as u64),
                count_words(main as u64),
                count_words((lines.len() - main) as u64),
                count_words(vertices as u64),
            ),
            _ => "Bu aralıkla eğri çıkmadı: rasterin değerleri hiçbir düzeyi geçmiyor.".to_owned(),
        };
        let objects = line_objects(&lines, &texts, &r.layer("layer").id);
        let mut result = RunResult {
            changes: (!objects.is_empty()).then(|| ChangeSet {
                add: objects,
                ..ChangeSet::default()
            }),
            summary: Some(summary),
            ..RunResult::default()
        };
        result
            .outputs
            .insert("lines".to_owned(), json!(lines.len()));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}
