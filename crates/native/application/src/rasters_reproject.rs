//! `cad.rasters.reproject` v1 (docs/adr/0644 §3; plan/gis.md `CBS-31`): a raster file in another coordinate system
//! resampled onto a north-up grid of the project's system, written as a GeoTIFF and added as a raster linked to it, as
//! one undo step “Rasteri yeniden izdüşür”. The grid covers the source's border (20 steps a side) carried into the
//! project's system, its corner on a multiple of the pixel (given, or the source's middle pixel's footprint to six
//! significant figures); each output pixel's centre goes back through the project's datum choices to the source's
//! pixels (`kentos_formats::raster::warp`, its values kept). The STAC preparation reprojects its local copies with the
//! same (`warp_file`). Shared cases: `fixtures/commands/v1/cad.rasters.reproject.json`
//! (`scripts/fixtures/raster_reproject_cases.py`, GDAL and PROJ).

use std::path::{Path, PathBuf};

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, REPROJECT_MOST_CELLS, RasterEntity,
    RasterFields, RasterSample, RastersReproject, RastersReprojected, ReprojectSample,
};
use kentos_domain::{Document, NewLayer};
use kentos_formats::raster::warp::{self, Grid, Job};
use kentos_geometry_core::crs::{self as core, Choice, System};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::links::find;
use crate::raster_files::{self, Bytes, Out};

/// The undo step's name.
pub const STEP: &str = "Rasteri yeniden izdüşür";

/// Points along each side of the source whose images bound the grid.
const PER_SIDE: u32 = 20;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// A value to six significant figures (the default pixel: the same wherever the transform's last bits differ).
fn six_figures(v: f64) -> f64 {
    format!("{v:.5e}").parse().unwrap_or(v)
}

/// A raster's way into the project's system: the systems, the grid and how its pixels are taken.
pub(crate) struct Warp {
    pub source: PathBuf,
    pub from: u32,
    pub from_system: System,
    pub project: System,
    pub choices: Vec<Choice>,
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub sample: RasterSample,
    pub nodata: Option<f64>,
    pub grid: Grid,
    pub nearest: bool,
    /// The transformation's path and accuracy (“EPSG:5261, ±1 m”).
    pub way: String,
    pub unofficial: bool,
    pub epsg: Option<u32>,
    pub geographic: bool,
}

/// The transforms' words for a move (`layers_crs`'s).
fn way_of(t: &core::Transformed) -> String {
    let mut words = if t.via.is_empty() {
        "aynı datum".to_owned()
    } else {
        t.via.clone()
    };
    // Only the projection changed: no accuracy to say.
    if let Some(a) = t.accuracy.filter(|a| *a > 0.0) {
        words.push_str(&format!(", ±{a} m"));
    }
    if t.unofficial {
        words.push_str(", resmî dönüşüm değil");
    }
    words
}

/// Where the source pixel corner (u, v) lies in the project's system.
fn forward(w: &Warp, u: f64, v: f64) -> Option<Vec2> {
    let [x0, a, b, y0, c, d] = w.affine;
    let p = Vec2::new(x0 + a * u + b * v, y0 + c * u + d * v);
    core::transform_in(&w.from_system, &w.project, p, &w.choices)
        .ok()
        .map(|t| t.point)
}

/// Plans the move of the raster file at `source` (its system the file's, else `srid`; `srid` given and other than
/// the file's warns) onto a grid of the project's system with pixel `cell` (none: the default). `at` names the input
/// field a refusal points at.
#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_file(
    settings: &kentos_commands::ProjectSettings,
    source: &Path,
    shown: &str,
    (srid, fallback): (Option<u32>, Option<u32>),
    cell: Option<f64>,
    nearest: bool,
    at: &str,
    warnings: &mut Vec<CommandWarning>,
) -> Result<Warp, Stop> {
    let name = file_name(shown);
    let bytes = Bytes::file(source).map_err(|e| refuse(codes::IMPORT_UNREADABLE, e, at))?;
    let opened = raster_files::open(bytes).map_err(|e| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!("{name} raster olarak okunamadı: {e}"),
            at,
        )
    })?;
    let info = &opened.reader.info;
    let Some(affine) = info.affine else {
        return Err(refuse(
            codes::IMPORT_UNREADABLE,
            format!(
                "{name} dosyasının yeri bilinmiyor (GeoTIFF etiketi ya da dünya dosyası yok); önce Raster oturt ile yerine koyun."
            ),
            at,
        ));
    };
    let from = match (srid, info.epsg) {
        (Some(s), Some(e)) if s != e => {
            warnings.push(CommandWarning {
                code: codes::CRS_OVERRIDDEN.into(),
                message: format!(
                    "{name} EPSG:{e} bildiriyor; verilen EPSG:{s} sayıldı (koordinatlar dönüştürülmeden o sistemde)."
                ),
                path: Some("srid".into()),
            });
            s
        }
        (Some(s), _) => s,
        (None, Some(e)) => e,
        (None, None) if fallback.is_some() => fallback.unwrap_or_default(),
        (None, None) => {
            return Err(refuse(
                codes::CRS_UNKNOWN,
                format!("{name} koordinat sistemi bildirmiyor; dosyanın sistemini srid ile verin."),
                "srid",
            ));
        }
    };
    let srid_at = if srid.is_some() { "srid" } else { at };
    let Some(from_system) = kentos_project::crs::system(from).and_then(|s| s.transform_system())
    else {
        return Err(refuse(
            codes::CRS_UNKNOWN,
            format!(
                "EPSG:{from} KentOS'un kayıtlı koordinat sistemlerinde yok; dosyanın sistemini kayıtlı bir srid ile verin."
            ),
            srid_at,
        ));
    };
    if settings.srid == 0 {
        return Err(refuse(
            codes::CRS_MISMATCH,
            "Projenin koordinat sistemi yok (yerel proje); raster çevrilecek bir sisteme gidemez. Projeye sistem verin ya da rasteri Raster ekle ile olduğu gibi ekleyin.".into(),
            at,
        ));
    }
    if from == settings.srid {
        return Err(refuse(
            codes::CRS_MISMATCH,
            format!(
                "{name} zaten projenin sisteminde (EPSG:{from}); çevrilecek bir şey yok, Raster ekle ile ekleyin."
            ),
            at,
        ));
    }
    let Some(project) = kentos_project::systems::own(settings).and_then(|n| n.system) else {
        return Err(refuse(
            codes::CRS_MISMATCH,
            "Projenin koordinat sistemi dönüştürülebilen bir sistem değil.".into(),
            at,
        ));
    };
    let choices = kentos_project::systems::choices(settings);
    let geographic = matches!(project, System::Geographic { .. });
    let mut w = Warp {
        source: source.to_path_buf(),
        from,
        from_system,
        project,
        choices,
        affine,
        width: info.width,
        height: info.height,
        bands: info.bands,
        sample: info.sample,
        nodata: opened.reader.nodata,
        grid: Grid {
            affine: [0.0; 6],
            width: 0,
            height: 0,
        },
        nearest,
        way: String::new(),
        unofficial: false,
        epsg: (settings.srid > 0).then_some(settings.srid),
        geographic,
    };
    // The way at the source's middle.
    let (cu, cv) = (f64::from(w.width / 2), f64::from(w.height / 2));
    let [x0, a, b, y0, c, d] = affine;
    let middle = Vec2::new(x0 + a * cu + b * cv, y0 + c * cu + d * cv);
    let moved = core::transform_in(&w.from_system, &w.project, middle, &w.choices);
    let border: Vec<Vec2> = warp::border(w.width, w.height, PER_SIDE)
        .into_iter()
        .filter_map(|p| forward(&w, p.x, p.y))
        .collect();
    let Ok(t) = moved else {
        return Err(refuse(
            codes::CRS_UNREACHED,
            format!(
                "{name} EPSG:{from}'den projenin sistemine çevrilemiyor (sistemin dışında); doğru sistemi srid ile verin."
            ),
            srid_at,
        ));
    };
    if border.len() < 4 {
        return Err(refuse(
            codes::CRS_UNREACHED,
            format!(
                "{name} EPSG:{from}'den projenin sistemine çevrilemiyor (sistemin dışında); doğru sistemi srid ile verin."
            ),
            srid_at,
        ));
    }
    w.way = way_of(&t);
    w.unofficial = t.unofficial;
    let size = match cell {
        Some(s) => s,
        None => {
            let f = |p: Vec2| forward(&w, p.x, p.y);
            match warp::default_pixel(&f, &[Vec2::new(cu, cv)]) {
                Some(p) => six_figures(p),
                None => {
                    return Err(refuse(
                        codes::CRS_UNREACHED,
                        format!(
                            "{name} dosyasının orta pikseli projenin sistemine çevrilemiyor; piksel boyunu cell ile verin."
                        ),
                        "cell",
                    ));
                }
            }
        }
    };
    let grid = Grid::covering(&border, size)
        .filter(|g| u64::from(g.width) * u64::from(g.height) <= REPROJECT_MOST_CELLS);
    let Some(grid) = grid else {
        return Err(refuse(
            codes::INVALID_INPUT,
            format!(
                "{size} birimlik pikselle sonuç çok büyük olurdu (en çok {REPROJECT_MOST_CELLS} piksel); daha büyük bir piksel boyu verin."
            ),
            "cell",
        ));
    };
    w.grid = grid;
    Ok(w)
}

/// Writes the move planned in `w` as a GeoTIFF at `out` (beside it as `.part`, renamed when whole).
pub(crate) fn warp_file(
    w: &Warp,
    out: &Path,
    stop: &dyn Fn() -> bool,
    share: &dyn Fn(u64, u64),
) -> Result<raster_files::Written, String> {
    let mut opened = raster_files::open(Bytes::file(&w.source)?)?;
    let [x0, a, b, y0, c, d] = w.affine;
    let det = a * d - b * c;
    let (project, from, choices) = (w.project.clone(), w.from_system.clone(), w.choices.clone());
    let inverse = Box::new(move |p: Vec2| -> Option<Vec2> {
        let q = core::transform_in(&project, &from, p, &choices).ok()?.point;
        let (dx, dy) = (q.x - x0, q.y - y0);
        Some(Vec2::new((d * dx - b * dy) / det, (a * dy - c * dx) / det))
    });
    let (job, header) = Job::keeping(
        w.grid,
        inverse,
        w.width,
        w.height,
        w.bands,
        w.sample,
        w.nodata,
        opened.reader.palette.clone(),
        w.nearest,
        w.epsg,
        w.geographic,
    )
    .map_err(|e| e.0)?;
    raster_files::resample(
        &mut opened,
        job,
        header,
        Out::File(out.to_path_buf()),
        stop,
        share,
    )
}

/// What the command writes.
struct Planned {
    warp: Warp,
    out: RastersReprojected,
    output: PathBuf,
    layer: Layer,
    entity: Entity,
    warnings: Vec<CommandWarning>,
}

enum Layer {
    On(String),
    New(String),
}

fn path_in(text: &str, drawing: Option<&str>, at: &str) -> Result<PathBuf, Stop> {
    if Path::new(text).is_absolute() {
        return Ok(PathBuf::from(text));
    }
    match drawing.and_then(|d| Path::new(d).parent()) {
        Some(folder) => Ok(folder.join(text)),
        None => Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{text}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin."
            ),
            at,
        )),
    }
}

fn check(doc: &Document, input: &RastersReproject) -> Result<Planned, Stop> {
    let drawing = input.drawing.as_deref();
    let text = input.file.trim();
    if text.is_empty() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            "Çevrilecek raster dosyasının yolu boş.".into(),
            "file",
        ));
    }
    if Path::new(text).is_relative() && drawing.is_none() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{text}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin."
            ),
            "file",
        ));
    }
    let given_out = input
        .output
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());
    let named_out = match given_out {
        Some(t) => Some(path_in(t, drawing, "output")?),
        None => None,
    };
    if let Some(cell) = input.cell
        && !(cell.is_finite() && cell > 0.0)
    {
        return Err(refuse(
            codes::INVALID_INPUT,
            "Piksel boyu sıfırdan büyük, sonlu bir sayı olmalı.".into(),
            "cell",
        ));
    }
    let found = find(text, None, drawing, &doc.settings().xref_paths);
    let Some(source) = found.at else {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!(
                "{} bulunamadı. Bakılan yerler: {}.",
                file_name(text),
                found.tried.join(", ")
            ),
            "file",
        ));
    };
    let mut warnings = Vec::new();
    let nearest = input.sample == ReprojectSample::Nearest;
    let w = plan_file(
        doc.settings(),
        &source,
        text,
        (input.srid, None),
        input.cell,
        nearest,
        "file",
        &mut warnings,
    )?;
    if w.unofficial {
        warnings.push(CommandWarning {
            code: codes::CRS_UNOFFICIAL.into(),
            message: format!("Raster resmî olmayan bir dönüşümle çevrildi ({}).", w.way),
            path: Some("file".into()),
        });
    }
    // The output: given, or beside the source.
    let (output, shown_out) = match named_out {
        Some(p) => (p, given_out.unwrap_or_default().to_owned()),
        None => {
            let stem = source
                .file_stem()
                .map_or_else(|| "raster".to_owned(), |s| s.to_string_lossy().into_owned());
            let p = source.with_file_name(format!("{stem}-epsg{}.tif", doc.settings().srid));
            let shown = p.display().to_string();
            (p, shown)
        }
    };
    if output == source
        || std::fs::canonicalize(&output)
            .ok()
            .zip(std::fs::canonicalize(&source).ok())
            .is_some_and(|(a, b)| a == b)
    {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            "Çıktı kaynağın kendisi; çevrilen raster kaynağın üzerine yazılmaz, başka bir dosya adı verin.".into(),
            "output",
        ));
    }
    let folder = output.parent().map(Path::to_path_buf).unwrap_or_default();
    if !folder.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” klasörü yok; raster var olan bir klasöre yazılır.",
                folder.display()
            ),
            "output",
        ));
    }
    if output.exists() && input.overwrite != Some(true) {
        return Err(refuse(
            codes::FILE_EXISTS,
            format!(
                "{} var; üzerine yazmak için overwrite verin.",
                file_name(&shown_out)
            ),
            "output",
        ));
    }
    let layer = match &input.layer {
        Some(id) => {
            warnings.extend(checks::layer_at(doc, id, "layer")?);
            Layer::On(id.clone())
        }
        None => Layer::New(
            source
                .file_stem()
                .map_or_else(|| "Raster".to_owned(), |s| s.to_string_lossy().into_owned()),
        ),
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    let style = kentos_formats::raster::style::default_style(w.bands, w.sample, false);
    let entity = Entity::Raster(RasterEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: String::new(),
            color: None,
            attrs: input.attributes.clone(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        raster: RasterFields {
            affine: w.grid.affine,
            width: w.grid.width,
            height: w.grid.height,
            bands: w.bands,
            sample: w.sample,
            asset: None,
            file: Some(shown_out.clone()),
            url: None,
            srid: doc.settings().srid,
            style,
            opacity: None,
            dataset: None,
            georef: None,
            tiles: None,
        },
    });
    let out = RastersReprojected {
        uid: None,
        layer: match &layer {
            Layer::On(id) => id.clone(),
            Layer::New(name) => doc.layers().preview(NewLayer::layer(name.clone())).id,
        },
        file: shown_out,
        from: w.from,
        to: doc.settings().srid,
        affine: w.grid.affine,
        width: w.grid.width,
        height: w.grid.height,
        bands: w.bands,
        sample: w.sample,
        way: w.way.clone(),
        revision: doc.revision().to_string(),
    };
    Ok(Planned {
        warp: w,
        out,
        output,
        layer,
        entity,
        warnings,
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &RastersReproject) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &RastersReproject,
) -> CommandResult<RastersReprojected> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: p.out,
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again, writes the GeoTIFF and adds the raster as one undo step “Rasteri yeniden izdüşür”.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: RastersReproject,
) -> CommandResult<RastersReprojected> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_RASTERS_REPROJECT,
        kentos_commands::CAD_RASTERS_REPROJECT_VERSION,
        input,
        write,
    )
}

fn write(
    cx: &mut ExecutionContext<'_>,
    input: RastersReproject,
) -> CommandResult<RastersReprojected> {
    let Planned {
        warp: w,
        mut out,
        output,
        layer,
        mut entity,
        warnings,
    } = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let job = cx.job;
    let stopped = || job.is_some_and(crate::jobs::Progress::stopped);
    let share = |done: u64, total: u64| {
        if let Some(j) = job {
            j.set(done, total);
        }
    };
    // The protocol's plan runs this on a reading copy: no file then (docs/adr/0532 §4).
    let written = if cx.dry() {
        Ok(None)
    } else {
        warp_file(&w, &output, &stopped, &share).map(Some)
    };
    match written {
        Ok(None) => {}
        Err(why) if why == raster_files::STOPPED => {
            return CommandResult::Cancelled;
        }
        Ok(Some(written)) => {
            if let Entity::Raster(r) = &mut entity {
                r.raster.bands = written.bands;
                r.raster.sample = written.sample;
                out.bands = written.bands;
                out.sample = written.sample;
            }
        }
        Err(why) => {
            return Stop::failed(checks::error(
                codes::FILE_UNWRITABLE,
                format!("Çevrilen raster yazılamadı: {why}. Hiçbir şey değişmedi."),
                Some("output".into()),
            ))
            .into();
        }
    }
    let added = cx.doc.transact(STEP, |doc| {
        let layer_id = match &layer {
            Layer::On(id) => id.clone(),
            Layer::New(name) => doc
                .add_layer(NewLayer::layer(name.clone()), None, false)
                .map_err(|r| {
                    Stop::failed(checks::error(
                        codes::LAYER_REFUSED,
                        r.to_string(),
                        Some("layer".into()),
                    ))
                })?,
        };
        entity.base_mut().layer_id = layer_id.clone();
        let (_, uid, revision) = checks::add(doc, entity)?;
        Ok::<_, Stop>((layer_id, uid, revision))
    });
    match added {
        Ok((layer, uid, revision)) => {
            out.layer = layer;
            out.uid = Some(uid);
            out.revision = revision;
            CommandResult::Completed {
                output: out,
                warnings,
            }
        }
        Err(stop) => {
            let _ = std::fs::remove_file(&output);
            stop.into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_figures_rounds_as_python_does() {
        assert_eq!(six_figures(29.993_504_1), 29.9935);
        assert_eq!(six_figures(0.000_300_000_3), 0.0003);
        assert_eq!(six_figures(0.000_300_000_7), 0.000_300_001);
        assert_eq!(six_figures(25.0), 25.0);
    }
}
