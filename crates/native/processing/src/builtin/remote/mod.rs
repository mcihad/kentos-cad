//! Uzaktan algılama (docs/adr/0242; the web's `builtin/remote/`): Bant
//! birleştir, Bantlara ayır, Spektral indis, Denetimli ve Denetimsiz
//! sınıflandırma, Değişim tespiti and Görüntü birleştirme (a raster beside
//! the first input, its object right above the first input's layer) and
//! Doğruluk analizi (a table), each a run of the raster core's operation job
//! in the run's one step (Bantlara ayır one run a band). The core writes the
//! tables, the summaries' tails and the warnings; here they are passed on.

pub mod tools;

use std::collections::BTreeMap;

use kentos_commands::{Entity, RasterEntity, RasterFields};
use kentos_native_application::geometry::shape;
use kentos_raster::remote::{RemoteNotes, Table};
use serde_json::{Value, json};

use super::pointcloud::{broke, count_words};
use super::raster_ops::{Ran, drive, names_of, rasters_for, spec_of};
use super::surface::{base, files_of};
use crate::files::Beside;
use crate::types::{ChangeSet, Feedback, Resolved, RunContext, RunResult};

pub const REMOTE: &str = "remoteSensing";

/// A table as the tools' table output.
pub fn table_json(t: &Table) -> Value {
    json!({ "columns": t.columns, "rows": t.rows })
}

/// The notes' warnings through the feedback; the tail with a space before it (none when empty).
fn said(n: &RemoteNotes, feedback: &mut dyn Feedback) -> String {
    for w in &n.warnings {
        feedback.warn(w.clone());
    }
    if n.tail.is_empty() {
        String::new()
    } else {
        format!(" {}", n.tail)
    }
}

/// The one raster of the features parameter `name`, or why not.
fn one_raster<'a>(r: &Resolved<'a>, name: &str, label: &str) -> Result<&'a RasterEntity, String> {
    let rasters: Vec<&RasterEntity> = r
        .features(name)
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Raster(x) => Some(x),
            _ => None,
        })
        .collect();
    match rasters.as_slice() {
        [] => Err(format!("{label} seçin: bu araç raster ister.")),
        [one] => Ok(one),
        many => Err(format!(
            "{} raster seçili; {label} için tek raster seçin.",
            many.len()
        )),
    }
}

/// The rasters of a run: the input's (one, or many in the run's order) and, after them, the second parameter's one.
fn rasters_of<'a>(
    r: &Resolved<'a>,
    ctx: &RunContext<'_>,
    one: bool,
    second: Option<(&str, &str)>,
) -> Result<Vec<&'a RasterEntity>, String> {
    let mut rasters = rasters_for(r, ctx, one)?;
    if let Some((name, label)) = second {
        rasters.push(one_raster(r, name, label)?);
    }
    Ok(rasters)
}

/// Kalite maskesi uygula's rule (docs/adr/0644 §2): the one written, else the quality raster's own (`stac.qa_mask`).
pub fn qa_rule(r: &Resolved<'_>) -> Result<String, String> {
    let written = r.text("rule").trim();
    if !written.is_empty() {
        return Ok(written.to_owned());
    }
    let qa = one_raster(r, "qa", "Kalite rasteri")?;
    qa.base
        .attrs
        .get(kentos_commands::stac_attrs::QA_MASK)
        .filter(|t| !t.trim().is_empty())
        .cloned()
        .ok_or_else(|| {
            "Kalite rasterinin maske kuralı yok (stac.qa_mask): kuralı yazın, values:0,1,3 ya da bits:1,3.".into()
        })
}

/// The first input raster's record (`input_record`), none when the run has no raster yet.
pub fn rasters_of_input(r: &Resolved<'_>, ctx: &RunContext<'_>) -> BTreeMap<String, String> {
    rasters_for(r, ctx, true)
        .ok()
        .and_then(|rs| rs.first().map(|x| input_record(x)))
        .unwrap_or_default()
}

/// The objects of a features parameter and the text of `field` on each (none where it is missing).
pub fn objects_with(
    r: &Resolved<'_>,
    name: &str,
    field: &str,
) -> (
    Vec<kentos_geometry_core::entity::Shape>,
    Vec<Option<String>>,
) {
    let entities = &r.features(name).entities;
    let shapes = entities.iter().map(|e| shape(e)).collect();
    let texts = entities
        .iter()
        .map(|e| e.base().attrs.get(field).cloned())
        .collect();
    (shapes, texts)
}

fn raster_entity(
    ran: &Ran,
    path: &str,
    srid: u32,
    layer: &str,
    attrs: BTreeMap<String, String>,
) -> Result<Entity, String> {
    let Some((bands, sample, style, grid)) = ran.raster.clone() else {
        return Err("Çözümleme raster vermedi.".into());
    };
    Ok(Entity::Raster(RasterEntity {
        base: base(layer, attrs, None),
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
            georef: None,
            tiles: None,
        },
    }))
}

/// What a raster run writes: the input (`one` raster or several) and the
/// second parameter's raster, the objects `shapes`; the file beside the
/// first input (or where asked), its object right above the first input's
/// layer; the summary's tail, the table and the warnings the core's.
#[allow(clippy::too_many_arguments)]
pub fn run_raster(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    inputs: (bool, Option<(&str, &str)>),
    shapes: Vec<kentos_geometry_core::entity::Shape>,
    names: (&str, &str),
) -> RunResult {
    run_raster_noted(r, ctx, feedback, tool, inputs, shapes, names, &|_| BTreeMap::new())
}

/// [`run_raster`] whose raster carries the attributes `noted` makes of the run's notes (a model's record,
/// docs/adr/0644 §4).
#[allow(clippy::too_many_arguments)]
pub fn run_raster_noted(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    (one, second): (bool, Option<(&str, &str)>),
    shapes: Vec<kentos_geometry_core::entity::Shape>,
    (suffix, label): (&str, &str),
    noted: &dyn Fn(&RemoteNotes) -> BTreeMap<String, String>,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = |feedback: &mut dyn Feedback| -> Result<RunResult, String> {
        let rasters = rasters_of(r, ctx, one, second)?;
        let names = names_of(&rasters, ctx);
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let first = &rasters[0].raster;
        let path =
            files.output_path(r.text("output"), Some(Beside::from(first)), suffix, ".tif")?;
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
        let Some((_, _, _, grid)) = &ran.raster else {
            return Err("Çözümleme raster vermedi.".into());
        };
        let (w, h) = (grid.width, grid.height);
        if ran.notes.empty_cells == u64::from(w) * u64::from(h) {
            feedback.warn("Sonucun hiçbir hücresinde değer yok.".to_owned());
        }
        let add = if r.flag("add") {
            vec![raster_entity(
                &ran,
                &path,
                first.srid,
                &r.layer("layer").id,
                noted(&ran.notes.remote),
            )?]
        } else {
            Vec::new()
        };
        let notes = &ran.notes.remote;
        let summary = format!(
            "{} × {} hücrelik raster; “{path}” yazıldı.{}",
            count_words(u64::from(w)),
            count_words(u64::from(h)),
            said(notes, feedback)
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
        if let Some(t) = &notes.table {
            result.outputs.insert("table".to_owned(), table_json(t));
        }
        Ok(result)
    };
    go(feedback).unwrap_or_else(broke)
}

/// Bantlara ayır (§4): each band its own GeoTIFF, `…-b1.tif`, `…-b2.tif` …,
/// beside the raster (or after the name asked); the objects in the bands'
/// order on a new layer right above the raster's.
pub fn run_split(r: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = |feedback: &mut dyn Feedback| -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, true)?;
        let names = names_of(&rasters, ctx);
        let first = &rasters[0].raster;
        let asked = r.text("output").trim();
        let base_path = files.output_path(asked, Some(Beside::from(first)), "-b", ".tif")?;
        let stem = base_path.strip_suffix(".tif").unwrap_or(&base_path);
        let stem = if asked.is_empty() {
            stem.to_owned()
        } else {
            format!("{stem}-b")
        };
        // The file's bands; an alpha band (the last) the core refuses, and the bands end there.
        let bands = first.bands;
        let label = "Bantlar ayrılıyor";
        let mut add = Vec::new();
        let mut rows = Vec::new();
        let mut size = (0, 0);
        let mut count = 0u32;
        for b in 1..=bands.max(1) {
            let tool = json!({ "kind": "band", "band": b });
            let spec = spec_of(ctx, &rasters, &names, tool)?;
            let path = format!("{stem}{b}.tif");
            let ran = match drive(
                &*files,
                &rasters,
                &spec,
                Vec::new(),
                Some(&path),
                feedback,
                label,
            ) {
                Ok(ran) => ran,
                // Past the value bands (the last one alpha): done.
                Err(e) if b > 1 && e.contains("bant yok") => break,
                Err(e) => return Err(e),
            };
            feedback.progress(f64::from(b) / f64::from(bands.max(1)), label);
            if let Some((_, _, _, grid)) = &ran.raster {
                size = (grid.width, grid.height);
            }
            if r.flag("add") {
                add.push(raster_entity(
                    &ran,
                    &path,
                    first.srid,
                    &r.layer("layer").id,
                    BTreeMap::new(),
                )?);
            }
            rows.push(vec![b.to_string(), path.clone()]);
            count = b;
        }
        let list = match rows.as_slice() {
            [] => String::new(),
            [one] => format!("“{}”", one[1]),
            [a, .., z] => format!("“{}” … “{}”", a[1], z[1]),
        };
        let summary = format!(
            "{} bant ayrı rasterlere yazıldı: {list} ({} × {} hücre).",
            count_words(u64::from(count)),
            count_words(u64::from(size.0)),
            count_words(u64::from(size.1))
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
        result.outputs.insert(
            "table".to_owned(),
            json!({ "columns": ["Bant", "Dosya"], "rows": rows }),
        );
        Ok(result)
    };
    go(feedback).unwrap_or_else(broke)
}

/// Doğruluk analizi (§8): the matrix's table, the overall accuracy and kappa; nothing written.
pub fn run_accuracy(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = |feedback: &mut dyn Feedback| -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, true)?;
        let names = names_of(&rasters, ctx);
        let (shapes, texts) = objects_with(r, "reference", r.text("referenceField"));
        let tool = json!({
            "kind": "accuracy",
            "band": r.number("band").unwrap_or(1.0) as u32,
            "reference": texts,
        });
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let label = "Doğruluk hesaplanıyor";
        let ran = drive(&*files, &rasters, &spec, shapes, None, feedback, label)?;
        feedback.progress(1.0, label);
        let n = &ran.notes.remote;
        for w in &n.warnings {
            feedback.warn(w.clone());
        }
        let mut result = RunResult {
            summary: Some(n.tail.clone()),
            ..RunResult::default()
        };
        if let Some(t) = &n.table {
            result.outputs.insert("table".to_owned(), table_json(t));
        }
        result
            .outputs
            .insert("overall".to_owned(), json!(n.overall));
        result.outputs.insert("kappa".to_owned(), json!(n.kappa));
        if let Some(leaked) = leaked(r, ctx, rasters[0]) {
            if leaked > 0 {
                feedback.warn(format!(
                    "{} doğrulama nesnesi rasterin eğitim alanlarıyla örtüşüyor: doğruluk iyimser çıkar; eğitimde kullanılmayan nesnelerle doğrulayın.",
                    count_words(leaked as u64)
                ));
            }
            result.outputs.insert("leaked".to_owned(), json!(leaked));
        }
        Ok(result)
    };
    go(feedback).unwrap_or_else(broke)
}

/// The model's record a classification's raster carries (docs/adr/0644 §4).
pub mod model {
    pub const METHOD: &str = "model.method";
    pub const CLASSES: &str = "model.classes";
    pub const TRAINING: &str = "model.training";
    pub const TRAINING_LAYER: &str = "model.training_layer";
    pub const SAMPLES: &str = "model.samples";
    pub const ITERATIONS: &str = "model.iterations";
    pub const START: &str = "model.start";
    pub const INPUT: &str = "model.input";
    pub const SOURCES: &str = "model.sources";
}

/// What the input raster came from: its file (or address, or library item) and its STAC item and time, when it has
/// them (the external source's revision, docs/adr/0644 §4).
pub fn input_record(raster: &RasterEntity) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let f = &raster.raster;
    if let Some(from) = f.file.as_ref().or(f.url.as_ref()).or(f.asset.as_ref()) {
        out.insert(model::INPUT.to_owned(), from.clone());
    }
    let a = &raster.base.attrs;
    if let Some(item) = a.get(kentos_commands::stac_attrs::ITEM) {
        let at = a
            .get(kentos_commands::stac_attrs::DATETIME)
            .map(|d| format!("@{d}"))
            .unwrap_or_default();
        out.insert(model::SOURCES.to_owned(), format!("{item}{at}"));
    }
    out
}

/// The classes as the run's table names them: `1=Orman; 2=Su`.
pub fn classes_of(n: &RemoteNotes) -> String {
    n.table
        .as_ref()
        .map(|t| {
            t.rows
                .iter()
                .filter_map(|row| Some(format!("{}={}", row.get(1)?, row.first()?)))
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

/// How many reference objects meet the training areas of the classified raster's model (the same samples counted
/// twice); none when the raster names no training layer the drawing has.
fn leaked(r: &Resolved<'_>, ctx: &RunContext<'_>, raster: &RasterEntity) -> Option<usize> {
    use kentos_geometry_core::ops::spatial_query::{geometry_of, intersects};
    let layers = raster.base.attrs.get(model::TRAINING_LAYER)?;
    let training: Vec<_> = layers
        .split(',')
        .map(str::trim)
        .filter(|l| ctx.doc.layers().get(l).is_some())
        .flat_map(|l| ctx.doc.by_layer(l))
        .filter_map(|e| geometry_of(&shape(e)))
        .collect();
    if training.is_empty() && !layers.split(',').any(|l| ctx.doc.layers().get(l.trim()).is_some()) {
        return None;
    }
    Some(
        r.features("reference")
            .entities
            .iter()
            .filter_map(|e| geometry_of(&shape(e)))
            .filter(|g| training.iter().any(|t| intersects(g, t)))
            .count(),
    )
}
