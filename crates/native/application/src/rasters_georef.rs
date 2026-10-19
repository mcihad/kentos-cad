//! `cad.rasters.georef` v1 (docs/adr/0518; plan/gis.md `CBS-29`): Raster oturt as a command that keeps its record. The
//! points' targets are carried to the project's system when given in another (`srid`, the project's datum choices);
//! the control points are fitted by the geometry core's transforms (`ops::georef`, docs/adr/0204 §6), the check points
//! are not and their errors are kept; Helmert and the affine set the raster's affine, the others resample the source
//! onto the north-up grid of `kentos_formats::raster::warp` (the host's own resampling taken by `made` when it is the
//! plan's grid). The raster keeps the record: method, points, source and its SHA-256, m0, check errors, output grid.
//! A resampled raster's points are its record's source's pixels: it is georeferenced again from that source, never
//! from its own output. The shared cases are `fixtures/commands/v1/cad.rasters.georef.json`.

use std::path::{Path, PathBuf};

use kentos_contracts::encoding::{base64_decode, base64_encode, sha256_hex, sha256_reader_hex};
use kentos_contracts::{
    CommandResult, CommandWarning, Entity, GeorefMethod, GeorefOutput, GeorefResidual, GeorefSource,
    MAX_GEOREF_POINTS, RasterEntity, RasterFields, RasterGeoref, RasterRender, RasterResampling, RasterSample,
    RasterStretch, RasterStyle, RastersGeoref, RastersGeoreffed, codes,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_formats::raster::warp::{self, Grid};
use kentos_geometry_core::ops::georef::{Gcp, Georef, GeorefError, Method, solve};
use kentos_geometry_core::vec2::Vec2;
use serde_json::json;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::links::find;
use crate::raster_files::{self, Bytes, Out};

/// Points along each side of the source whose forward bounds the output grid (docs/adr/0204 §6).
pub const BORDER_STEPS: u32 = 64;

/// The largest resampled raster embedded in the project's library (docs/adr/0204 §6).
pub const MOST_EMBEDDED: usize = 32 * 1024 * 1024;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

fn file_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// The source's header: what a resampling or a raster put back on its source reads.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceHeader {
    pub bands: u32,
    pub sample: RasterSample,
    pub palette: bool,
}

/// A command checked: the raster, its source, the transform, every point's residual, the output grid and the record.
#[derive(Debug)]
pub struct Prepared {
    pub slot: Slot,
    pub uid: String,
    pub before: RasterEntity,
    /// The raster the pixels are of (with its SHA-256 when read).
    pub source: GeorefSource,
    /// The source's file where it was found (a linked source).
    pub source_at: Option<PathBuf>,
    pub header: Option<SourceHeader>,
    pub georef: Georef,
    pub residuals: Vec<GeorefResidual>,
    pub control: u32,
    pub checks: u32,
    /// A resampling's grid.
    pub grid: Option<Grid>,
    pub nearest: bool,
    /// Where a resampling of a linked source is written: as kept, and the path.
    pub output: Option<(String, PathBuf)>,
    /// The host's own output, read: kept as (file or asset), its header, its SHA-256.
    pub made: Option<(MadeAt, SourceHeader, bool, String)>,
    pub record: RasterGeoref,
    /// The project's coordinate system (EPSG; 0 local) and whether it is geographic.
    pub srid: u32,
    pub geographic: bool,
    pub warnings: Vec<CommandWarning>,
}

/// Where the host's resampled raster is.
#[derive(Clone, Debug, PartialEq)]
pub enum MadeAt {
    File(String),
    Asset(String),
}

impl Prepared {
    /// The source pixel a point of the drawing comes from (the transform's inverse), for the host's resampling.
    pub fn inverse(&self) -> impl Fn(Vec2) -> Option<Vec2> + Send + 'static {
        let g = self.georef.clone();
        move |p| g.inverse(p)
    }

    /// The raster as the command writes it (a resampling the command makes itself: `written`).
    fn raster(&self, written: Option<&Written>) -> RasterFields {
        let mut f = self.before.raster.clone();
        f.srid = self.srid;
        let mut record = self.record.clone();
        match &self.grid {
            None => {
                f.affine = self.georef.affine.unwrap_or(f.affine);
                // A resampled raster put back on its source by a placing transform.
                if self.before.raster.georef.as_ref().is_some_and(|g| g.output.is_some()) {
                    f.width = self.source.width;
                    f.height = self.source.height;
                    f.asset = self.source.asset.clone();
                    f.file = self.source.file.clone();
                    f.url = self.source.url.clone();
                    if let Some(h) = &self.header {
                        f.bands = h.bands;
                        f.sample = h.sample;
                        f.style = fit_look(&f.style, h);
                    }
                }
            }
            Some(grid) => {
                f.affine = grid.affine;
                f.width = grid.width;
                f.height = grid.height;
                let (header, alpha, digest) = match (&self.made, written) {
                    (Some((_, h, alpha, digest)), _) => (h.clone(), *alpha, Some(digest.clone())),
                    (None, Some(w)) => (
                        SourceHeader { bands: w.bands, sample: w.sample, palette: false },
                        w.alpha,
                        Some(w.digest.clone()),
                    ),
                    (None, None) => {
                        // Planned: the output's bands as the resampling will make them.
                        let h = self.header.clone().unwrap_or(SourceHeader {
                            bands: f.bands,
                            sample: f.sample,
                            palette: false,
                        });
                        if h.sample == RasterSample::U8 || h.palette {
                            (SourceHeader { bands: 4, sample: RasterSample::U8, palette: false }, true, None)
                        } else {
                            (h, false, None)
                        }
                    }
                };
                f.bands = header.bands;
                f.sample = header.sample;
                f.style = if alpha {
                    RasterStyle {
                        render: RasterRender::Rgb,
                        bands: vec![1, 2, 3, 4],
                        stretch: RasterStretch::None,
                        ramp: None,
                        ..f.style.clone()
                    }
                } else {
                    fit_look(&f.style, &header)
                };
                let (asset, file) = match (&self.made, written, &self.output) {
                    (Some((MadeAt::File(k), ..)), _, _) => (None, Some(k.clone())),
                    (Some((MadeAt::Asset(a), ..)), _, _) => (Some(a.clone()), None),
                    (None, Some(w), _) if w.asset.is_some() => (w.asset.clone(), None),
                    (None, _, Some((kept, _))) => (None, Some(kept.clone())),
                    // Planned into the library: its id is its bytes' and comes when written.
                    (None, _, None) => (None, None),
                };
                f.asset = asset;
                f.file = file;
                f.url = None;
                if let Some(o) = record.output.as_mut() {
                    o.digest = digest;
                }
            }
        }
        f.georef = Some(Box::new(record));
        f
    }
}

/// The style kept when it shows the raster's bands, else the default look of its bands.
fn fit_look(style: &RasterStyle, h: &SourceHeader) -> RasterStyle {
    if style.problem(h.bands).is_none() {
        style.clone()
    } else {
        kentos_formats::raster::style::default_style(h.bands, h.sample, h.palette)
    }
}

/// A resampling the command made itself.
#[derive(Debug)]
struct Written {
    bands: u32,
    sample: RasterSample,
    alpha: bool,
    digest: String,
    asset: Option<String>,
}

/// The raster the points' pixels are of: a resampled raster's record's source, else the raster itself.
fn source_of(r: &RasterFields) -> GeorefSource {
    match &r.georef {
        Some(g) if g.output.is_some() => GeorefSource { digest: None, ..g.source.clone() },
        _ => GeorefSource {
            width: r.width,
            height: r.height,
            asset: r.asset.clone(),
            file: r.file.clone(),
            url: r.url.clone(),
            digest: None,
        },
    }
}

/// An embedded raster's bytes from the project's library.
pub fn asset_bytes(doc: &Document, id: &str) -> Option<Vec<u8>> {
    let item = doc.styles().items.iter().find(|it| {
        it.get("kind").and_then(|k| k.as_str()) == Some("asset") && it.get("id").and_then(|k| k.as_str()) == Some(id)
    })?;
    let url = item.get("data").and_then(|d| d.as_str())?;
    let (_, data) = url.split_once(";base64,")?;
    base64_decode(data)
}

/// A path the input gives (absolute, or relative to the drawing's folder): where it is.
fn given_path(text: &str, drawing: Option<&str>, at: &str) -> Result<PathBuf, Stop> {
    let t = text.trim();
    if t.is_empty() {
        return Err(refuse(codes::INVALID_FILE_PATH, "Dosya yolu boş; bir dosyanın yolunu verin.".into(), at));
    }
    let p = Path::new(t);
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    match drawing.and_then(|d| Path::new(d).parent()) {
        Some(folder) => Ok(crate::source_places::resolve(folder, &t.replace('\\', "/"))),
        None => Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("“{t}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin."),
            at,
        )),
    }
}

/// The source's bytes: its file where it is found, or the library's embedded raster.
fn source_bytes(
    doc: &Document,
    source: &GeorefSource,
    drawing: Option<&str>,
) -> Result<(Bytes, Option<PathBuf>, String), Stop> {
    if let Some(id) = &source.asset {
        let Some(b) = asset_bytes(doc, id) else {
            return Err(refuse(
                codes::UNKNOWN_ASSET,
                format!("Rasterin kaynağı “{id}” projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir."),
                "uid",
            ));
        };
        let digest = sha256_hex(&b);
        return Ok((Bytes::Memory(b), None, digest));
    }
    if let Some(file) = &source.file {
        if Path::new(file).is_relative() && drawing.is_none() {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                format!("Rasterin kaynağı “{file}” göreli bir yol, ama çizimin dosyası yok; çizimi kaydedin ya da drawing'i verin."),
                "drawing",
            ));
        }
        let found = find(file, None, drawing, &doc.settings().xref_paths);
        let Some(at) = found.at else {
            return Err(refuse(
                codes::FILE_NOT_FOUND,
                format!("Rasterin kaynağı {} bulunamadı. Bakılan yerler: {}.", file_name(file), found.tried.join(", ")),
                "uid",
            ));
        };
        let digest = std::fs::File::open(&at)
            .and_then(sha256_reader_hex)
            .map(|(_, d)| d)
            .map_err(|e| refuse(codes::FILE_NOT_FOUND, format!("{} okunamadı: {e}.", at.display()), "uid"))?;
        let bytes = Bytes::file(&at).map_err(|e| refuse(codes::FILE_NOT_FOUND, e, "uid"))?;
        return Ok((bytes, Some(at), digest));
    }
    Err(refuse(
        codes::IMPORT_UNREADABLE,
        "Adresten okunan raster bu ortamda okunmaz; dosyasını indirip bağlayın ya da gömün.".into(),
        "uid",
    ))
}

fn header_of(opened: &raster_files::Opened) -> SourceHeader {
    SourceHeader {
        bands: opened.reader.info.bands,
        sample: opened.reader.info.sample,
        palette: opened.reader.palette.is_some(),
    }
}

/// The targets in the project's system: as given, or carried from `srid` by the project's datum choices.
fn targets(doc: &Document, input: &RastersGeoref) -> Result<(Vec<Vec2>, Option<u32>), Stop> {
    let given: Vec<Vec2> = input.points.iter().map(|p| Vec2::new(p.target[0], p.target[1])).collect();
    let settings = doc.settings();
    let Some(srid) = input.srid.filter(|s| *s != settings.srid) else {
        return Ok((given, None));
    };
    let Some(from) = kentos_project::crs::system(srid) else {
        return Err(refuse(
            codes::UNKNOWN_SRID,
            format!("EPSG:{srid} KentOS'un kayıtlı koordinat sistemlerinde yok; hedefleri kayıtlı bir sistemde verin."),
            "srid",
        ));
    };
    let Some(from) = from.transform_system() else {
        return Err(refuse(
            codes::TARGET_NOT_CONVERTED,
            format!("EPSG:{srid} dönüştürülebilen bir sistem değil (yerel ya da datumu bilinmiyor)."),
            "srid",
        ));
    };
    let Some(to) = kentos_project::systems::own(settings).and_then(|n| n.system) else {
        return Err(refuse(
            codes::TARGET_NOT_CONVERTED,
            format!(
                "Hedefler EPSG:{srid} sisteminde; projenin koordinat sistemi yok (yerel proje). Hedefleri projenin koordinatlarıyla verin."
            ),
            "srid",
        ));
    };
    let choices = kentos_project::systems::choices(settings);
    let mut out = Vec::with_capacity(given.len());
    for (i, p) in given.into_iter().enumerate() {
        match kentos_geometry_core::crs::transform_in(&from, &to, p, &choices) {
            Ok(t) => out.push(t.point),
            Err(_) => {
                return Err(refuse(
                    codes::TARGET_NOT_CONVERTED,
                    format!("{}. noktanın hedefi EPSG:{srid} sisteminden projenin sistemine çevrilemedi.", i + 1),
                    &format!("points[{i}].target"),
                ));
            }
        }
    }
    Ok((out, Some(srid)))
}

fn method_of(m: GeorefMethod) -> Method {
    Method::from_name(m.name()).unwrap_or(Method::Affine)
}

/// What the points give before any file is read: the transform, every point's residual, the check points' errors and a
/// resampling's grid (Raster oturt's window solves again at every change with it).
#[derive(Clone, Debug)]
pub struct Fitted {
    pub slot: Slot,
    pub before: RasterEntity,
    pub georef: Georef,
    pub residuals: Vec<GeorefResidual>,
    pub control: u32,
    pub checks: u32,
    pub m0: Option<f64>,
    pub check_rmse: Option<f64>,
    pub check_max: Option<f64>,
    pub grid: Option<Grid>,
    /// The targets' system when it was not the project's.
    pub srid: Option<u32>,
    /// The raster the pixels are of (its digest not read yet).
    pub source: GeorefSource,
    pub warnings: Vec<CommandWarning>,
}

/// [`Fitted`] for the input, nothing read; the refusal as the command answers it.
pub fn fit(doc: &Document, input: &RastersGeoref) -> Result<Fitted, Box<kentos_contracts::CommandError>> {
    fitted(doc, input).map_err(|s| Box::new(s.into_error()))
}

/// Checks the input and works out everything the command writes, reading the source where it must (the desktop's
/// resampling takes the grid and the inverse from it); the refusal as the command answers it.
pub fn prepare(doc: &Document, input: &RastersGeoref) -> Result<Prepared, Box<kentos_contracts::CommandError>> {
    check(doc, input).map_err(|s| Box::new(s.into_error()))
}

fn fitted(doc: &Document, input: &RastersGeoref) -> Result<Fitted, Stop> {
    if !checks::is_uid_text(&input.uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!("“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", input.uid),
            "uid",
        ));
    }
    let found = Uuid::parse_str(&input.uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|slot| Some((slot, doc.get(slot)?)));
    let Some((slot, entity)) = found else {
        return Err(refuse(
            codes::ENTITY_NOT_FOUND,
            format!("“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir.", input.uid),
            "uid",
        ));
    };
    let Entity::Raster(raster) = entity else {
        return Err(refuse(
            codes::NOT_A_RASTER,
            "Verilen nesne bir raster değil; yalnız raster oturtulur.".into(),
            "uid",
        ));
    };
    if raster.raster.dataset.is_some() {
        return Err(refuse(
            codes::NOT_A_RASTER,
            "Veri setini (NetCDF, mesh) gösteren raster oturtulmaz; yeri dosyasının ızgarasındandır.".into(),
            "uid",
        ));
    }
    let layers = doc.layers();
    if layers.is_locked(&raster.base.layer_id) {
        let name = layers.get(&raster.base.layer_id).map_or(raster.base.layer_id.as_str(), |n| n.name.as_str());
        return Err(refuse(
            codes::LAYER_LOCKED,
            format!("“{name}” katmanı kilitli; üzerindeki raster oturtulamaz. Kilidi Katmanlar panelinden açın."),
            "uid",
        ));
    }
    if input.points.len() > MAX_GEOREF_POINTS {
        return Err(refuse(
            codes::INVALID_CONTROL_POINT,
            format!("En çok {MAX_GEOREF_POINTS} nokta verilir; {} verildi.", input.points.len()),
            "points",
        ));
    }
    for (i, p) in input.points.iter().enumerate() {
        if let Some(why) = p.problem() {
            return Err(refuse(codes::INVALID_CONTROL_POINT, why, &format!("points[{i}]")));
        }
    }
    let (targets, srid_given) = targets(doc, input)?;
    let method = method_of(input.method);
    let gcps: Vec<Gcp> = input
        .points
        .iter()
        .zip(&targets)
        .map(|(p, t)| Gcp { pixel: Vec2::new(p.pixel[0], p.pixel[1]), target: *t, used: p.control() })
        .collect();
    let g = solve(&gcps, method).map_err(|e| match e {
        GeorefError::TooFew(n) => refuse(
            codes::TOO_FEW_CONTROL_POINTS,
            format!(
                "{} en az {n} kontrol noktası ister; {} verildi (denetim ve kullanılmayan noktalar dönüşüme girmez).",
                input.method.title(),
                gcps.iter().filter(|p| p.used).count()
            ),
            "points",
        ),
        GeorefError::Duplicate => refuse(
            codes::CONTROL_POINTS_DUPLICATE,
            "İki kontrol noktası aynı pikselde; birini düzeltin ya da kullanmayın.".into(),
            "points",
        ),
        GeorefError::Collinear => refuse(
            codes::CONTROL_POINTS_COLLINEAR,
            "Kontrol noktalarının pikselleri bir doğru üzerinde; rasterin iki yönüne yayılmış noktalar verin.".into(),
            "points",
        ),
        GeorefError::Singular => refuse(
            codes::GEOREF_SINGULAR,
            "Bu kontrol noktalarıyla dönüşüm çözülemiyor; noktaları rasterin geneline yayın.".into(),
            "points",
        ),
    })?;
    // A control point beyond a projective's horizon makes no m0.
    if gcps.iter().zip(&g.residuals).any(|(p, r)| p.used && !r[2].is_finite()) {
        return Err(refuse(
            codes::GEOREF_SINGULAR,
            "Bir kontrol noktası dönüşümün ufkunun ötesine düşüyor; noktaları rasterin geneline yayın.".into(),
            "points",
        ));
    }
    let mut residuals = Vec::with_capacity(gcps.len());
    let (mut sum, mut most, mut n) = (0.0, 0.0f64, 0usize);
    for (i, ((p, r), t)) in input.points.iter().zip(&g.residuals).zip(&targets).enumerate() {
        let fine = r[2].is_finite();
        if p.check {
            if !fine {
                return Err(refuse(
                    codes::INVALID_CONTROL_POINT,
                    "Denetim noktası dönüşümün ufkunun ötesine düşüyor; hatası ölçülemez.".into(),
                    &format!("points[{i}]"),
                ));
            }
            sum += r[2] * r[2];
            most = most.max(r[2]);
            n += 1;
        }
        residuals.push(GeorefResidual {
            dx: fine.then_some(r[0]),
            dy: fine.then_some(r[1]),
            d: fine.then_some(r[2]),
            target: [t.x, t.y],
        });
    }
    let (check_rmse, check_max) = if n > 0 { (Some((sum / n as f64).sqrt()), Some(most)) } else { (None, None) };
    let control = gcps.iter().filter(|p| p.used).count() as u32;
    let source = source_of(&raster.raster);
    let mut warnings = Vec::new();
    if n == 0 {
        warnings.push(CommandWarning {
            code: codes::NO_CHECK_POINTS.into(),
            message: "Denetim noktası yok: m0 yalnız kontrol noktalarının uyumunu söyler; dönüşüme girmeyen, rastere yayılmış denetim noktaları ekleyin.".into(),
            path: Some("points".into()),
        });
    }
    // The output grid (docs/adr/0204 §6): the forward of the source's border, on multiples of the pixel.
    let grid = if input.method.places() {
        None
    } else {
        let size = match input.pixel {
            Some(s) if !(s.is_finite() && s > 0.0) => {
                return Err(refuse(
                    codes::INVALID_PIXEL_SIZE,
                    format!("Çıktının piksel boyu sıfırdan büyük, sonlu bir uzunluk olmalı; {s} verildi."),
                    "pixel",
                ));
            }
            Some(s) => Some(s),
            None => {
                let used: Vec<Vec2> = gcps.iter().filter(|p| p.used).map(|p| p.pixel).collect();
                warp::default_pixel(&|p| g.forward(p), &used)
            }
        };
        let edge: Vec<Vec2> = warp::border(source.width, source.height, BORDER_STEPS)
            .into_iter()
            .filter_map(|p| g.forward(p))
            .collect();
        let Some(grid) = size.and_then(|s| Grid::covering(&edge, s)) else {
            return Err(refuse(
                codes::GEOREF_GRID,
                "Çıktının ızgarası kurulamadı: dönüşüm rasteri çok büyütüyor ya da bozuyor; noktaları ya da piksel boyunu denetleyin.".into(),
                if input.pixel.is_some() { "pixel" } else { "points" },
            ));
        };
        Some(grid)
    };
    Ok(Fitted {
        slot,
        before: raster.clone(),
        m0: g.m0,
        georef: g,
        residuals,
        control,
        checks: n as u32,
        check_rmse,
        check_max,
        grid,
        srid: srid_given,
        source,
        warnings,
    })
}

fn check(doc: &Document, input: &RastersGeoref) -> Result<Prepared, Stop> {
    let Fitted {
        slot,
        before,
        georef: g,
        residuals,
        control,
        checks: n,
        m0,
        check_rmse,
        check_max,
        grid,
        srid: srid_given,
        mut source,
        mut warnings,
    } = fitted(doc, input)?;
    let raster = &before;
    let drawing = input.drawing.as_deref();
    // What is read: a resampling's source and output, a resampled raster's source when it is put back on it.
    let reverting = grid.is_none() && raster.raster.georef.as_ref().is_some_and(|g| g.output.is_some());
    let mut header = None;
    let mut source_at = None;
    let mut output = None;
    if grid.is_some() || reverting {
        let (bytes, at, digest) = source_bytes(doc, &source, drawing)?;
        let opened = raster_files::open(bytes).map_err(|e| {
            refuse(codes::IMPORT_UNREADABLE, format!("Rasterin kaynağı okunamadı: {e}"), "uid")
        })?;
        if opened.reader.info.width != source.width || opened.reader.info.height != source.height {
            return Err(refuse(
                codes::IMPORT_UNREADABLE,
                format!(
                    "Rasterin kaynağı {} × {} piksel, kayıttaki {} × {}; kaynak değişmiş. Rasteri yeniden ekleyin.",
                    opened.reader.info.width, opened.reader.info.height, source.width, source.height
                ),
                "uid",
            ));
        }
        header = Some(header_of(&opened));
        source.digest = Some(digest);
        source_at = at;
    } else {
        // Placed: its digest when its bytes are at hand; a missing linked file is said, not refused.
        match source_bytes(doc, &source, drawing) {
            Ok((_, at, digest)) => {
                source.digest = Some(digest);
                source_at = at;
            }
            Err(_) => warnings.push(CommandWarning {
                code: codes::SOURCE_NOT_READ.into(),
                message: "Rasterin kaynağı okunamadı; kayıt kaynağın özetini taşımıyor.".into(),
                path: Some("uid".into()),
            }),
        }
    }
    if let Some(grid) = &grid {
        let _ = grid;
        if input.made.is_none() && source.file.is_some() {
            let kept = match &input.output {
                Some(o) => o.trim().to_owned(),
                None => {
                    let file = source.file.clone().unwrap_or_default();
                    let stem = Path::new(&file)
                        .file_stem()
                        .map_or_else(|| "raster".to_owned(), |s| s.to_string_lossy().into_owned());
                    Path::new(&file)
                        .with_file_name(format!("{stem}-oturtulmus.tif"))
                        .to_string_lossy()
                        .into_owned()
                }
            };
            let at = match (&input.output, &source_at) {
                (Some(o), _) => given_path(o, drawing, "output")?,
                (None, Some(src)) => src.with_file_name(file_name(&kept)),
                (None, None) => given_path(&kept, drawing, "output")?,
            };
            if at.parent().is_some_and(|p| !p.as_os_str().is_empty() && !p.is_dir()) || at.is_dir() {
                return Err(refuse(
                    codes::INVALID_FILE_PATH,
                    format!("“{}” yazılamaz: klasörü yok ya da bir klasör.", at.display()),
                    "output",
                ));
            }
            if source_at.as_ref().is_some_and(|s| same_file(s, &at)) {
                return Err(refuse(
                    codes::INVALID_FILE_PATH,
                    "Çıktı kaynağın üstüne yazılmaz; kaynak dosya değişmeden kalır. Başka bir yol verin.".into(),
                    "output",
                ));
            }
            output = Some((kept, at));
        }
    }
    let made = match &input.made {
        None => None,
        Some(_) if grid.is_none() => {
            return Err(refuse(
                codes::GEOREF_OUTPUT_MISMATCH,
                format!("{} rasteri yeniden örneklemez; made verilmez.", input.method.title()),
                "made",
            ));
        }
        Some(m) => Some(made_of(doc, m, drawing, grid.as_ref())?),
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    let settings = doc.settings();
    let geographic = kentos_project::systems::own(settings).is_some_and(|n| n.geographic());
    let record = RasterGeoref {
        method: input.method,
        points: input.points.clone(),
        srid: srid_given,
        source: source.clone(),
        m0,
        check_rmse,
        check_max,
        output: grid.map(|gr| GeorefOutput {
            affine: gr.affine,
            width: gr.width,
            height: gr.height,
            pixel: gr.affine[1],
            resampling: input.resampling,
            digest: None,
        }),
    };
    Ok(Prepared {
        slot,
        uid: input.uid.clone(),
        before: before.clone(),
        source,
        source_at,
        header,
        georef: g,
        residuals,
        control,
        checks: n,
        grid,
        nearest: input.resampling == RasterResampling::Nearest,
        output,
        made,
        record,
        srid: settings.srid,
        geographic,
        warnings,
    })
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// The host's resampled raster read and held to the plan's grid.
fn made_of(
    doc: &Document,
    m: &kentos_contracts::GeorefMade,
    drawing: Option<&str>,
    grid: Option<&Grid>,
) -> Result<(MadeAt, SourceHeader, bool, String), Stop> {
    let (at, bytes, digest) = match (&m.file, &m.asset) {
        (Some(f), None) => {
            let p = given_path(f, drawing, "made.file")?;
            if !p.is_file() {
                return Err(refuse(codes::FILE_NOT_FOUND, format!("{} bulunamadı.", p.display()), "made.file"));
            }
            let digest = std::fs::File::open(&p)
                .and_then(sha256_reader_hex)
                .map(|(_, d)| d)
                .map_err(|e| refuse(codes::FILE_NOT_FOUND, format!("{} okunamadı: {e}.", p.display()), "made.file"))?;
            let bytes = Bytes::file(&p).map_err(|e| refuse(codes::FILE_NOT_FOUND, e, "made.file"))?;
            (MadeAt::File(f.trim().to_owned()), bytes, digest)
        }
        (None, Some(id)) => {
            if !crate::edit::has_raster(doc, id) {
                return Err(refuse(
                    codes::UNKNOWN_ASSET,
                    format!("“{id}” kimlikli raster projenin kitaplığında yok."),
                    "made.asset",
                ));
            }
            let b = asset_bytes(doc, id).unwrap_or_default();
            let digest = sha256_hex(&b);
            (MadeAt::Asset(id.clone()), Bytes::Memory(b), digest)
        }
        _ => {
            return Err(refuse(
                codes::GEOREF_OUTPUT_MISMATCH,
                "made ya bir dosya (file) ya da kitaplıktaki raster (asset) olmalı; yalnız biri.".into(),
                "made",
            ));
        }
    };
    let opened = raster_files::open(bytes)
        .map_err(|e| refuse(codes::IMPORT_UNREADABLE, format!("Verilen çıktı okunamadı: {e}"), "made"))?;
    let i = &opened.reader.info;
    let fits = grid.is_some_and(|g| i.width == g.width && i.height == g.height && i.affine == Some(g.affine));
    if !fits {
        return Err(refuse(
            codes::GEOREF_OUTPUT_MISMATCH,
            "Verilen çıktının ızgarası (konumu, piksel boyu ya da boyu) planınki değil; hiçbir şey yazılmadı.".into(),
            "made",
        ));
    }
    let header = SourceHeader { bands: i.bands, sample: i.sample, palette: false };
    Ok((at, header, i.alpha, digest))
}

/// Why `Stop` came from a prepare: as the command answers it.
fn answer(p: &Prepared, raster: RasterFields, written: Option<String>, revision: String) -> RastersGeoreffed {
    RastersGeoreffed {
        uid: p.uid.clone(),
        residuals: p.residuals.clone(),
        control: p.control,
        checks: p.checks,
        raster,
        written,
        revision,
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &RastersGeoref) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: (), warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &RastersGeoref) -> CommandResult<RastersGeoreffed> {
    match check(cx.doc, input) {
        Ok(p) => {
            let raster = p.raster(None);
            let written = p.output.as_ref().map(|(_, at)| at.display().to_string());
            CommandResult::Completed {
                output: answer(&p, raster, written, cx.doc.revision().to_string()),
                warnings: p.warnings.clone(),
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again, resamples when it must, and writes the raster with its record: one undo step “Raster oturt”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RastersGeoref) -> CommandResult<RastersGeoreffed> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_RASTERS_GEOREF,
        kentos_contracts::CAD_RASTERS_GEOREF_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: RastersGeoref) -> CommandResult<RastersGeoreffed> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let mut written = None;
    let mut written_at = None;
    let mut library = None;
    if let (Some(grid), None) = (p.grid, &p.made) {
        let stop = || cx.stopped();
        let share = |done: u64, total: u64| cx.progress(done, total);
        match resample(cx.doc, &p, grid, input.drawing.as_deref(), &stop, &share) {
            Ok((w, at, item)) => {
                written = Some(w);
                written_at = at;
                library = item;
            }
            Err(Resampled::Stopped) => return CommandResult::Cancelled,
            Err(Resampled::Refused(stop)) => return stop.into(),
        }
    }
    let raster = p.raster(written.as_ref());
    if let Some(why) = raster.problem() {
        return Stop::failed(checks::error(codes::INVALID_RASTER, why, Some("uid".into()))).into();
    }
    if let Some((id, item)) = library
        && !crate::edit::has_raster(cx.doc, &id)
    {
        let mut styles = cx.doc.styles().clone();
        styles.items.push(item);
        cx.doc.set_styles(styles);
    }
    let mut next = p.before.clone();
    next.raster = raster.clone();
    let slot = p.slot;
    let _ = cx.doc.transact("Raster oturt", |doc| -> Result<(), ()> {
        doc.update(slot, Entity::Raster(next));
        Ok(())
    });
    let shown = written_at.map(|a: PathBuf| a.display().to_string());
    CommandResult::Completed {
        output: answer(&p, raster, shown, cx.doc.revision().to_string()),
        warnings: p.warnings.clone(),
    }
}

enum Resampled {
    Stopped,
    Refused(Stop),
}

type LibraryItem = (String, serde_json::Value);

/// The source resampled onto `grid`: a linked source's output written beside it (or at `output`), an embedded one's
/// into the project's library (its item returned to add).
fn resample(
    doc: &Document,
    p: &Prepared,
    grid: Grid,
    drawing: Option<&str>,
    stop: &dyn Fn() -> bool,
    share: &dyn Fn(u64, u64),
) -> Result<(Written, Option<PathBuf>, Option<LibraryItem>), Resampled> {
    let bad = |e: String| Resampled::Refused(refuse(codes::IMPORT_UNREADABLE, format!("Rasterin kaynağı okunamadı: {e}"), "uid"));
    let (bytes, _, _) = source_bytes(doc, &p.source, drawing).map_err(Resampled::Refused)?;
    let mut opened = raster_files::open(bytes).map_err(bad)?;
    let i = opened.reader.info.clone();
    let inverse = Box::new(p.inverse());
    let (job, header) = kentos_formats::raster::warp::Job::new(
        grid,
        inverse,
        i.width,
        i.height,
        i.bands,
        i.sample,
        opened.reader.nodata,
        opened.reader.palette.clone(),
        p.nearest,
        (p.srid > 0).then_some(p.srid),
        p.geographic,
    )
    .map_err(|e| bad(e.0))?;
    let out = match &p.output {
        Some((_, at)) => Out::File(at.clone()),
        None => Out::Memory,
    };
    let w = raster_files::resample(&mut opened, job, header, out, stop, share).map_err(|e| {
        if e == raster_files::STOPPED {
            Resampled::Stopped
        } else {
            Resampled::Refused(refuse(codes::INVALID_FILE_PATH, format!("Yeniden örneklenen raster yazılamadı: {e}"), "output"))
        }
    })?;
    match (&w.path, &w.bytes) {
        (Some(path), _) => {
            let digest = std::fs::File::open(path)
                .and_then(sha256_reader_hex)
                .map(|(_, d)| d)
                .map_err(|e| Resampled::Refused(refuse(codes::INVALID_FILE_PATH, format!("{e}"), "output")))?;
            Ok((
                Written { bands: w.bands, sample: w.sample, alpha: w.alpha, digest, asset: None },
                Some(path.clone()),
                None,
            ))
        }
        (None, Some(bytes)) => {
            if bytes.len() > MOST_EMBEDDED {
                return Err(Resampled::Refused(refuse(
                    codes::INVALID_FILE_PATH,
                    format!(
                        "Oturtulan raster {} MB; gömülü raster en çok {} MB olabilir. output ile yazılacak bir dosya yolu verin.",
                        bytes.len() >> 20,
                        MOST_EMBEDDED >> 20
                    ),
                    "output",
                )));
            }
            let digest = sha256_hex(bytes);
            let id = format!("raster-{}", &digest[..16]);
            let layer = doc
                .layers()
                .get(&p.before.base.layer_id)
                .map_or_else(|| "raster".to_owned(), |l| l.name.clone());
            let item = json!({
                "kind": "asset",
                "id": id,
                "name": format!("{layer}-oturtulmus"),
                "path": ["Rasterler"],
                "format": "tiff",
                "data": format!("data:image/tiff;base64,{}", base64_encode(bytes)),
                "width": grid.width,
                "height": grid.height,
            });
            Ok((
                Written { bands: w.bands, sample: w.sample, alpha: w.alpha, digest, asset: Some(id.clone()) },
                None,
                Some((id, item)),
            ))
        }
        (None, None) => Err(Resampled::Refused(refuse(codes::INVALID_FILE_PATH, "Çıktı yazılmadı.".into(), "output"))),
    }
}
