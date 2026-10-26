//! `cad.stac.prepare` v1 (docs/adr/0528 §4–§6; plan/gis.md `CBS-31`): the assets of a STAC item the caller chose,
//! added as rasters linked by address, in one new group with a layer per asset, as one undo step “STAC verisini
//! hazırla”. What is planned (the grid, data type, nodata, scale and offset, the quality band and its mask, the
//! record) is the services core's (`kentos_services::stac::prepare`); this writes it. No file is opened and no
//! network is used: the catalog's own words place the rasters. The record travels in the command's answer and in
//! each raster's attributes (`stac.*`).
//! The shared cases are `fixtures/commands/v1/cad.stac.prepare.json` (`scripts/fixtures/stac_cases.py`).

use std::path::{Path, PathBuf};

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, RasterEntity, StacPrepare, StacPrepared,
    StacReprojected,
};
use kentos_domain::{Document, NewLayer};
use kentos_services::stac::prepare::{self, Prepared, RasterPlan};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::rasters_reproject::{self, Warp};

/// A preparation with its moves into the project's system (docs/adr/0644 §3): each raster's move, in the order of
/// `bands` then `qa`.
struct Checked {
    planned: Prepared,
    moves: Vec<Option<(Warp, PathBuf)>>,
    warnings: Vec<CommandWarning>,
}

fn refuse(code: &str, message: String, path: String) -> Stop {
    Stop::failed(checks::error(code, message, Some(path)))
}

/// A local copy's file: absolute, or from the drawing's folder.
fn local_path(text: &str, drawing: Option<&str>, at: &str) -> Result<PathBuf, Stop> {
    let p = Path::new(text);
    let path = if p.is_absolute() {
        p.to_path_buf()
    } else if let Some(folder) = drawing.and_then(|d| Path::new(d).parent()) {
        folder.join(p)
    } else {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{text}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da drawing'i verin."
            ),
            at.into(),
        ));
    };
    if !path.is_file() {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("Yerel kopya {} bulunamadı.", path.display()),
            at.into(),
        ));
    }
    Ok(path)
}

fn check(doc: &Document, input: &StacPrepare) -> Result<Checked, Stop> {
    let mut planned = prepare::plan(input, doc.settings().srid)
        .map_err(|p| Stop::failed(checks::error(p.code, p.message, p.path)))?;
    let mut warnings = warnings_of(&planned);
    let drawing = input.drawing.as_deref();
    let settings = doc.settings();
    let mut moves = Vec::new();
    let mut records: Vec<Option<StacReprojected>> = Vec::new();
    for plan in planned.bands.iter_mut().chain(planned.qa.iter_mut()) {
        let key = plan.layer.clone();
        let at = format!("local.{key}");
        let Some(text) = plan.fields.file.clone() else {
            moves.push(None);
            records.push(None);
            continue;
        };
        let path = local_path(&text, drawing, &at)?;
        let Some(from) = plan.reproject_from else {
            moves.push(None);
            records.push(None);
            continue;
        };
        // A class or quality band keeps its codes: nearest, as every band does here.
        let w = rasters_reproject::plan_file(
            settings,
            &path,
            &text,
            (None, Some(from)),
            None,
            true,
            &at,
            &mut warnings,
        )?;
        let stem = path
            .file_stem()
            .map_or_else(|| key.clone(), |s| s.to_string_lossy().into_owned());
        let out = path.with_file_name(format!("{stem}-epsg{}.tif", settings.srid));
        if out == path {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                "Çevrilen raster yerel kopyanın üzerine yazılmaz; kopyayı başka bir adla verin."
                    .into(),
                at,
            ));
        }
        let shown = out.display().to_string();
        plan.fields.affine = w.grid.affine;
        plan.fields.width = w.grid.width;
        plan.fields.height = w.grid.height;
        plan.fields.srid = settings.srid;
        plan.fields.file = Some(shown.clone());
        plan.fields.url = None;
        let reprojected = StacReprojected {
            from: w.from,
            to: settings.srid,
            way: w.way.clone(),
            file: shown,
            width: w.grid.width,
            height: w.grid.height,
            affine: w.grid.affine,
        };
        records.push(Some(reprojected));
        if w.unofficial {
            warnings.push(CommandWarning {
                code: codes::CRS_UNOFFICIAL.into(),
                message: format!("“{key}” resmî olmayan bir dönüşümle çevrildi ({}).", w.way),
                path: Some(at),
            });
        }
        moves.push(Some((w, out)));
    }
    let count = planned.preparation.bands.len();
    for (k, r) in records.into_iter().enumerate() {
        if k < count {
            planned.preparation.bands[k].reprojected = r;
        } else if let Some(q) = planned.preparation.qa.as_mut() {
            q.reprojected = r;
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked {
        planned,
        moves,
        warnings,
    })
}

fn warnings_of(p: &Prepared) -> Vec<CommandWarning> {
    p.warnings
        .iter()
        .map(|(code, message, path)| CommandWarning {
            code: (*code).into(),
            message: message.clone(),
            path: Some(path.clone()),
        })
        .collect()
}

fn answer(doc: &Document, p: &Prepared) -> StacPrepared {
    StacPrepared {
        preparation: p.preparation.clone(),
        group: None,
        revision: doc.revision().to_string(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &StacPrepare) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &StacPrepare) -> CommandResult<StacPrepared> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: answer(cx.doc, &c.planned),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the group, the layers and the rasters as one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: StacPrepare) -> CommandResult<StacPrepared> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_STAC_PREPARE,
        kentos_commands::CAD_STAC_PREPARE_VERSION,
        input,
        write,
    )
}

fn raster(layer_id: &str, plan: &RasterPlan) -> Entity {
    Entity::Raster(RasterEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer_id.to_owned(),
            color: None,
            attrs: plan.attrs.clone(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        raster: plan.fields.clone(),
    })
}

/// A raster's layer, then the raster on it: the layer's id and the raster's persistent id.
fn put(doc: &mut Document, group: &str, plan: &RasterPlan) -> Result<(String, String), Stop> {
    let layer = doc
        .add_layer(NewLayer::layer(plan.layer.clone()), Some(group), false)
        .map_err(|r| {
            Stop::failed(checks::error(
                kentos_commands::codes::INVALID_GROUP_NAME,
                r.to_string(),
                None,
            ))
        })?;
    let (_, uid, _) = checks::add(doc, raster(&layer, plan))?;
    Ok((layer, uid))
}

fn write(cx: &mut ExecutionContext<'_>, input: StacPrepare) -> CommandResult<StacPrepared> {
    let Checked {
        planned,
        moves,
        warnings,
    } = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    // The local copies carried into the project's system, before anything is written to the drawing.
    if !cx.dry() {
        let job = cx.job;
        let stopped = || job.is_some_and(crate::jobs::Progress::stopped);
        let mut made: Vec<PathBuf> = Vec::new();
        for (w, out) in moves.iter().flatten() {
            match rasters_reproject::warp_file(w, out, &stopped, &|_, _| {}) {
                Ok(_) => made.push(out.clone()),
                Err(why) => {
                    for m in &made {
                        let _ = std::fs::remove_file(m);
                    }
                    if why == crate::raster_files::STOPPED {
                        return CommandResult::Cancelled;
                    }
                    return Stop::failed(checks::error(
                        codes::FILE_UNWRITABLE,
                        format!("Çevrilen raster yazılamadı: {why}. Hiçbir şey değişmedi."),
                        Some("local".into()),
                    ))
                    .into();
                }
            }
        }
    }
    let written = cx.doc.transact("STAC verisini hazırla", |doc| {
        let group = doc
            .add_layer(NewLayer::group(planned.group.clone()), None, false)
            .map_err(|r| {
                Stop::failed(checks::error(
                    kentos_commands::codes::INVALID_GROUP_NAME,
                    r.to_string(),
                    Some("group".into()),
                ))
            })?;
        let mut done = Vec::new();
        for band in &planned.bands {
            done.push(put(doc, &group, band)?);
        }
        let qa = match &planned.qa {
            Some(q) => Some(put(doc, &group, q)?),
            None => None,
        };
        Ok::<_, Stop>((group, done, qa))
    });
    let (group, bands, qa) = match written {
        Ok(w) => w,
        Err(stop) => return stop.into(),
    };
    let mut out = answer(cx.doc, &planned);
    out.group = Some(group);
    for (b, (layer, uid)) in out.preparation.bands.iter_mut().zip(bands) {
        b.layer = Some(layer);
        b.uid = Some(uid);
    }
    if let (Some(q), Some((layer, uid))) = (out.preparation.qa.as_mut(), qa) {
        q.layer = Some(layer);
        q.uid = Some(uid);
    }
    CommandResult::Completed {
        output: out,
        warnings,
    }
}
