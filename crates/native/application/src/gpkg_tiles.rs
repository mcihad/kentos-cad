//! GeoPackage tile pyramids as rasters (docs/adr/0556; plan/gis.md `CBS-03`'s raster profile).
//!
//! [`read`] is `cad.rasters.gpkgread`: a tile table's window at one zoom level read as one raster. The window's
//! rounding is GDAL's GeoPackage driver's (first pixel `floor((min − origin) / pixel + ½)`, size `floor((max − min) /
//! pixel + ½)`), cut to the level's grid; the tiles are PNGs of the matrix's tile size and become four bands of bytes
//! (a grey tile its grey in all three colours, alpha 255), a place without a tile alpha 0. The raster is written as a
//! north-up GeoTIFF beside its name (`.kentos-yaziliyor`, read back, then renamed) and, with `layer`, added to the
//! drawing linked to it as one undo step.
//!
//! [`write`] is `cad.rasters.gpkgwrite`: a raster of the drawing written as a tile table. The pyramid's finest zoom is
//! the raster's own pixel with its grid's origin at the raster's top-left corner; each coarser zoom is a 2 × 2 mean of
//! the finer one's padded canvas (`(a + b + c + d + 2) / 4` per sample, samples past the canvas 0), so a tile that
//! covers more than the raster is transparent there. Tiles are 256 × 256 PNGs written by the formats core.
//!
//! The shared cases are `fixtures/commands/v1/cad.rasters.gpkgread.json` and `cad.rasters.gpkgwrite.json`
//! (`scripts/fixtures/gpkg_tiles_cases.py`, from GDAL's own GeoPackage driver).

use std::path::{Path, PathBuf};

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, GPKG_TILE_SIDE, GPKG_TILES_MOST_PIXELS,
    GpkgTileSet, GpkgTileZoom, GpkgTilesMode, RasterEntity, RasterFields, RasterSample,
    RastersGpkgRead, RastersGpkgReadDone, RastersGpkgWrite, RastersGpkgWritten, codes,
};
use kentos_domain::{Document, Uuid};
use kentos_formats::raster::png;
use kentos_formats::raster::write::{Geo, Image, Writer};
use kentos_formats::raster::{Samples, TILE};
use kentos_geopackage::tiles::{NewTile, NewTileSet, TileMatrix, TileSet, tile_format};
use kentos_geopackage::{GeoPackage, Srs};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::links::find;
use crate::raster_files::{self, Bytes};

/// Deflate's level for the GeoTIFF's tiles and the PNG tiles (1 fast … 9 small).
const LEVEL: u8 = 6;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// A path the input gives (absolute, or relative to the drawing's folder): where it is.
fn given_path(text: &str, drawing: Option<&str>, at: &str) -> Result<PathBuf, Stop> {
    let t = text.trim();
    if t.is_empty() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            "Dosya yolu boş; bir dosyanın yolunu verin.".into(),
            at,
        ));
    }
    let p = Path::new(t);
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    match drawing.and_then(|d| Path::new(d).parent()) {
        Some(folder) => Ok(crate::source_places::resolve(folder, &t.replace('\\', "/"))),
        None => Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{t}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin."
            ),
            at,
        )),
    }
}

fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(ext))
}

/// A file's neighbour the work is written to before it replaces the file.
fn beside(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".kentos-yaziliyor");
    PathBuf::from(tmp)
}

/// Whether an EPSG code names a geographic system (the GeoTIFF's model type).
fn is_geographic(srid: u32) -> bool {
    kentos_project::crs::system(srid).map_or(matches!(srid, 4326 | 4258 | 4269 | 4617), |s| {
        s.kind == "geographic"
    })
}

// ── The pyramid's arithmetic ───────────────────────────────────────────────────────────────────────────────────

/// The window of a level read: the first pixel, the size and where it lies. The rounding is GDAL's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Window {
    pub x: i64,
    pub y: i64,
    pub width: u32,
    pub height: u32,
    /// The window's top-left corner.
    pub origin: [f64; 2],
}

/// `rect` (`[min x, min y, max x, max y]`) as whole pixels of a matrix, cut to its grid; none when nothing of it lies
/// on the grid. `set` is the tile matrix set's box (its top-left corner is the grid's origin).
pub(crate) fn window(m: &TileMatrix, set: [f64; 4], rect: [f64; 4]) -> Option<Window> {
    let (x0, y1) = (set[0], set[3]);
    let (px, py) = (m.pixel_x_size, m.pixel_y_size);
    let first_x = ((rect[0] - x0) / px + 0.5).floor();
    let first_y = ((y1 - rect[3]) / py + 0.5).floor();
    let wide = ((rect[2] - rect[0]) / px + 0.5).floor();
    let high = ((rect[3] - rect[1]) / py + 0.5).floor();
    let grid_w = (m.matrix_width * m.tile_width) as f64;
    let grid_h = (m.matrix_height * m.tile_height) as f64;
    let (xa, xb) = (first_x.max(0.0), (first_x + wide).min(grid_w));
    let (ya, yb) = (first_y.max(0.0), (first_y + high).min(grid_h));
    if !(xb > xa && yb > ya) {
        return None;
    }
    Some(Window {
        x: xa as i64,
        y: ya as i64,
        width: (xb - xa) as u32,
        height: (yb - ya) as u32,
        origin: [x0 + xa * px, y1 - ya * py],
    })
}

/// The levels of a pyramid over a `width` × `height` raster, finest first. Each has its own canvas: a grid of the
/// tiles that hold some of its image; the matrix a level is declared with is wider: a level's matrix is the whole
/// extent of the set (what GDAL's validator holds a pyramid to), `2^(coarsest − level)` tiles a side, and the tiles
/// that hold none of the image are not written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Level {
    /// Pixels of the level's image.
    pub width: u32,
    pub height: u32,
    /// Tiles across and down that hold some of it.
    pub tiles_x: u32,
    pub tiles_y: u32,
    /// Tiles a side of the level's matrix.
    pub matrix: u32,
}

pub(crate) fn levels(width: u32, height: u32) -> Vec<Level> {
    let side = GPKG_TILE_SIDE;
    let (mut w, mut h) = (width.max(1), height.max(1));
    let mut out = Vec::new();
    loop {
        out.push(Level {
            width: w,
            height: h,
            tiles_x: w.div_ceil(side),
            tiles_y: h.div_ceil(side),
            matrix: 0,
        });
        if w <= side && h <= side {
            break;
        }
        w = w.div_ceil(2);
        h = h.div_ceil(2);
    }
    let top = out.len() as u32 - 1;
    for (j, l) in out.iter_mut().enumerate() {
        l.matrix = 1 << (top - j as u32);
    }
    out
}

/// A canvas's 2 × 2 means: the next coarser level's canvas of `to` × `to` pixels, `bands` interleaved; samples past
/// the finer canvas count 0.
fn halve(canvas: &[u8], from: (usize, usize), to: (usize, usize), bands: usize) -> Vec<u8> {
    let mut out = vec![0u8; to.0 * to.1 * bands];
    for v in 0..to.1 {
        for u in 0..to.0 {
            for b in 0..bands {
                let at = |x: usize, y: usize| {
                    if x < from.0 && y < from.1 {
                        u32::from(canvas[(y * from.0 + x) * bands + b])
                    } else {
                        0
                    }
                };
                let sum = at(2 * u, 2 * v)
                    + at(2 * u + 1, 2 * v)
                    + at(2 * u, 2 * v + 1)
                    + at(2 * u + 1, 2 * v + 1);
                out[(v * to.0 + u) * bands + b] = ((sum + 2) / 4) as u8;
            }
        }
    }
    out
}

// ── Tile sets ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The EPSG code of a table's system, when it names one.
fn srid_of(gp: &GeoPackage, srs_id: i32) -> Result<Option<u32>, String> {
    if srs_id <= 0 {
        return Ok(None);
    }
    let s = gp.srs(srs_id).map_err(|e| e.0)?;
    Ok(s.filter(|s| s.organization.eq_ignore_ascii_case("EPSG"))
        .and_then(|s| u32::try_from(s.organization_id).ok()))
}

fn zoom_info(m: &TileMatrix, tiles: u64, formats: Vec<&'static str>) -> GpkgTileZoom {
    GpkgTileZoom {
        zoom: u32::try_from(m.zoom).unwrap_or(0),
        matrix_width: u32::try_from(m.matrix_width).unwrap_or(0),
        matrix_height: u32::try_from(m.matrix_height).unwrap_or(0),
        tile_width: u32::try_from(m.tile_width).unwrap_or(0),
        tile_height: u32::try_from(m.tile_height).unwrap_or(0),
        pixel: [m.pixel_x_size, m.pixel_y_size],
        tiles,
        formats: formats.into_iter().map(str::to_owned).collect(),
    }
}

/// What the file says of a tile table.
fn describe_set(gp: &GeoPackage, set: &TileSet) -> Result<GpkgTileSet, String> {
    let mut zooms = Vec::with_capacity(set.matrices.len());
    for m in &set.matrices {
        let (tiles, formats) = gp.tile_census(&set.name, m.zoom).map_err(|e| e.0)?;
        zooms.push(zoom_info(m, tiles, formats));
    }
    let extensions = gp
        .extensions()
        .map_err(|e| e.0)?
        .into_iter()
        .filter(|e| e.table.as_deref() == Some(set.name.as_str()))
        .map(|e| e.name)
        .collect();
    Ok(GpkgTileSet {
        table: set.name.clone(),
        identifier: set.identifier.clone(),
        srs_id: set.srs_id,
        srid: srid_of(gp, set.srs_id)?,
        bounds: set.bounds,
        data_bounds: set.data_bounds,
        zooms,
        extensions,
    })
}

/// The tile tables of a GeoPackage as `cad.rasters.gpkgread` describes them, for a window that lists them before
/// anything is read.
pub fn tile_tables(path: &Path) -> Result<Vec<GpkgTileSet>, String> {
    let gp = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let sets = gp.tile_sets().map_err(|e| e.0)?;
    sets.iter().map(|s| describe_set(&gp, s)).collect()
}

// ── The raster read from tiles ─────────────────────────────────────────────────────────────────────────────────

pub mod read {
    use super::*;

    /// What a run found and made: the answer, the raster entity to add, the GeoTIFF's bytes (none without
    /// `output`) and where it goes.
    struct Planned {
        out: RastersGpkgReadDone,
        raster: Option<Entity>,
        target: Option<PathBuf>,
        tiff: Option<Vec<u8>>,
        warnings: Vec<CommandWarning>,
    }

    /// A tile's pixels as red, green, blue and alpha bytes: a PNG of 8 bits and the matrix's size.
    fn rgba_of(data: &[u8], m: &TileMatrix, at: &str) -> Result<Vec<u8>, Stop> {
        match tile_format(data) {
            "png" => {}
            f @ ("jpeg" | "webp") => {
                return Err(refuse(
                    codes::TILE_FORMAT_UNSUPPORTED,
                    format!(
                        "{at} karosu {}; bu okuyucu yalnız 8 bitlik PNG karoları açar (JPEG ve WebP karolar okunmaz).",
                        f.to_uppercase()
                    ),
                    "table",
                ));
            }
            _ => {
                return Err(refuse(
                    codes::TILE_UNREADABLE,
                    format!("{at} karosu PNG değil; GeoPackage karoları PNG olmalı."),
                    "table",
                ));
            }
        }
        let p = png::decode(data).map_err(|e| {
            refuse(
                codes::TILE_UNREADABLE,
                format!("{at} karosu okunamadı: {e}"),
                "table",
            )
        })?;
        if i64::from(p.width) != m.tile_width || i64::from(p.height) != m.tile_height {
            return Err(refuse(
                codes::TILE_UNREADABLE,
                format!(
                    "{at} karosu {} × {} piksel, matrisin karo boyu {} × {}.",
                    p.width, p.height, m.tile_width, m.tile_height
                ),
                "table",
            ));
        }
        let Samples::U8(v) = p.samples else {
            return Err(refuse(
                codes::TILE_FORMAT_UNSUPPORTED,
                format!(
                    "{at} karosu 8 bitten fazla; bu okuyucu yalnız 8 bitlik PNG karoları açar."
                ),
                "table",
            ));
        };
        let n = (p.width * p.height) as usize;
        let mut out = Vec::with_capacity(n * 4);
        match p.bands {
            1 => v
                .iter()
                .for_each(|&g| out.extend_from_slice(&[g, g, g, 255])),
            3 => v
                .chunks_exact(3)
                .for_each(|c| out.extend_from_slice(&[c[0], c[1], c[2], 255])),
            4 => out = v,
            _ => {
                return Err(refuse(
                    codes::TILE_UNREADABLE,
                    format!("{at} karosunun {} bandı var; 1, 3 ya da 4 olmalı.", p.bands),
                    "table",
                ));
            }
        }
        Ok(out)
    }

    fn check(doc: &Document, input: &RastersGpkgRead) -> Result<Planned, Stop> {
        let drawing = input.drawing.as_deref();
        let path = given_path(&input.path, drawing, "path")?;
        let target = match &input.output {
            Some(text) => {
                let t = given_path(text, drawing, "output")?;
                if !has_extension(&t, "tif") && !has_extension(&t, "tiff") {
                    return Err(refuse(
                        codes::INVALID_FILE_PATH,
                        format!(
                            "“{}” bir .tif dosyası değil; raster GeoTIFF olarak yazılır.",
                            file_name(text)
                        ),
                        "output",
                    ));
                }
                if !t.parent().is_some_and(Path::is_dir) {
                    return Err(refuse(
                        codes::INVALID_FILE_PATH,
                        format!(
                            "“{}” klasörü yok; raster var olan bir klasöre yazılır.",
                            t.parent().unwrap_or(Path::new("")).display()
                        ),
                        "output",
                    ));
                }
                Some(t)
            }
            None => None,
        };
        if input.layer.is_some() && target.is_none() {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                "layer verildi ama output yok; rasteri eklemek için yazılacak GeoTIFF'in yolunu (output) da verin.".into(),
                "output",
            ));
        }
        if !path.is_file() {
            return Err(refuse(
                codes::FILE_NOT_FOUND,
                format!(
                    "{} bulunamadı; GeoPackage'ın yolunu denetleyin.",
                    path.display()
                ),
                "path",
            ));
        }
        let gp = GeoPackage::open(&path, false).map_err(|e| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!(
                    "{} GeoPackage olarak okunamadı: {e}",
                    file_name(&input.path)
                ),
                "path",
            )
        })?;
        let sets = gp.tile_sets().map_err(|e| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("{} okunamadı: {e}", file_name(&input.path)),
                "path",
            )
        })?;
        if sets.is_empty() {
            return Err(refuse(
                codes::NO_TILE_TABLE,
                format!(
                    "{} dosyasında karo (raster) tablosu yok; GeoPackage'ın yalnız vektör ve öznitelik tabloları var.",
                    file_name(&input.path)
                ),
                "path",
            ));
        }
        let unreadable = |why: String| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("{} okunamadı: {why}", file_name(&input.path)),
                "path",
            )
        };
        let tables = sets
            .iter()
            .map(|s| describe_set(&gp, s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(unreadable)?;
        let set = match &input.table {
            Some(name) => sets.iter().find(|s| s.name == *name).ok_or_else(|| {
                refuse(
                    codes::TABLE_NOT_FOUND,
                    format!(
                        "“{name}” dosyada bir karo tablosu değil; karo tabloları: {}.",
                        sets.iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    "table",
                )
            })?,
            None if sets.len() == 1 => &sets[0],
            None => {
                return Err(refuse(
                    codes::TABLE_NOT_FOUND,
                    format!(
                        "Dosyada {} karo tablosu var; table ile birini seçin: {}.",
                        sets.len(),
                        sets.iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    "table",
                ));
            }
        };
        let table_srid = tables
            .iter()
            .find(|t| t.table == set.name)
            .and_then(|t| t.srid);
        let zoom = match input.zoom {
            Some(z) => i64::from(z),
            None => set.matrices.last().map_or(0, |m| m.zoom),
        };
        let Some(m) = set.matrix(zoom) else {
            return Err(refuse(
                codes::ZOOM_NOT_FOUND,
                format!(
                    "“{}” tablosunun {zoom}. zoom düzeyi yok; düzeyler: {}.",
                    set.name,
                    set.matrices
                        .iter()
                        .map(|m| m.zoom.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                "zoom",
            ));
        };
        if let Some(a) = input.area
            && !(a.iter().all(|v| v.is_finite()) && a[0] < a[2] && a[1] < a[3])
        {
            return Err(refuse(
                codes::INVALID_AREA,
                "Alan [en küçük x, en küçük y, en büyük x, en büyük y] olmalı: sonlu sayılar, en küçükler en büyüklerden küçük.".into(),
                "area",
            ));
        }
        let rect = input.area.or(set.data_bounds).unwrap_or(set.bounds);
        let Some(w) = window(m, set.bounds, rect) else {
            return Err(refuse(
                codes::TILE_TABLE_EMPTY,
                format!(
                    "Okunacak alan “{}” tablosunun {zoom}. düzeyinin ızgarasının dışında; alanı ızgaraya çekin.",
                    set.name
                ),
                if input.area.is_some() {
                    "area"
                } else {
                    "table"
                },
            ));
        };
        let pixels = u64::from(w.width) * u64::from(w.height);
        if pixels > GPKG_TILES_MOST_PIXELS {
            return Err(refuse(
                codes::GPKG_TILES_TOO_LARGE,
                format!(
                    "Raster {} × {} piksel olurdu; en çok 25 milyon piksel okunur. Daha kaba bir zoom düzeyi ya da dar bir alan verin.",
                    w.width, w.height
                ),
                if input.zoom.is_some() || input.area.is_some() {
                    "area"
                } else {
                    "zoom"
                },
            ));
        }
        let (tw, th) = (m.tile_width, m.tile_height);
        let (c0, c1) = (w.x / tw, (w.x + i64::from(w.width) - 1) / tw);
        let (r0, r1) = (w.y / th, (w.y + i64::from(w.height) - 1) / th);
        let tiles = gp
            .tiles_in(&set.name, zoom, (c0, c1), (r0, r1))
            .map_err(|e| unreadable(e.0))?;
        if tiles.is_empty() {
            return Err(refuse(
                codes::TILE_TABLE_EMPTY,
                format!(
                    "“{}” tablosunun {zoom}. düzeyinde okunan pencerede hiç karo yok.",
                    set.name
                ),
                if input.area.is_some() {
                    "area"
                } else {
                    "table"
                },
            ));
        }
        let expected = ((c1 - c0 + 1) * (r1 - r0 + 1)) as u64;
        let read = tiles.len() as u64;
        let srid = table_srid.unwrap_or(0);
        let mut warnings = Vec::new();
        if table_srid.is_none() {
            warnings.push(CommandWarning {
                code: codes::CRS_UNDECLARED.into(),
                message:
                    "Karo tablosu EPSG koordinat sistemi bildirmiyor; raster sistemsiz alındı."
                        .into(),
                path: Some("table".into()),
            });
        }
        if read < expected {
            warnings.push(CommandWarning {
                code: codes::TILES_MISSING.into(),
                message: format!(
                    "Okunan pencerenin {} karosu tabloda yok; o yerler saydam (alfa 0) kaldı.",
                    expected - read
                ),
                path: Some("table".into()),
            });
        }
        let affine = [
            w.origin[0],
            m.pixel_x_size,
            0.0,
            w.origin[1],
            0.0,
            -m.pixel_y_size,
        ];
        let area = [
            w.origin[0],
            w.origin[1] - f64::from(w.height) * m.pixel_y_size,
            w.origin[0] + f64::from(w.width) * m.pixel_x_size,
            w.origin[1],
        ];
        // The raster's pixels, the tiles laid side by side (only with an output: a description reads none).
        let mut tiff = None;
        if target.is_some() {
            let mut canvas = vec![0u8; pixels as usize * 4];
            for (c, r, data) in &tiles {
                let at = format!("{zoom}/{c}/{r}");
                let rgba = rgba_of(data, m, &at)?;
                let (gx, gy) = (c * tw, r * th);
                for v in 0..th {
                    let y = gy + v - w.y;
                    if y < 0 || y >= i64::from(w.height) {
                        continue;
                    }
                    for u in 0..tw {
                        let x = gx + u - w.x;
                        if x < 0 || x >= i64::from(w.width) {
                            continue;
                        }
                        let (s, d) = (
                            ((v * tw + u) * 4) as usize,
                            ((y * i64::from(w.width) + x) * 4) as usize,
                        );
                        canvas[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
                    }
                }
            }
            tiff = Some(
                geotiff(&canvas, w.width, w.height, affine, srid).map_err(|why| {
                    refuse(
                        codes::FILE_UNWRITABLE,
                        format!("GeoTIFF kurulamadı: {why}"),
                        "output",
                    )
                })?,
            );
        }
        if let Some(t) = &target
            && t.exists()
            && input.overwrite != Some(true)
        {
            return Err(refuse(
                codes::FILE_EXISTS,
                format!("{} var; üzerine yazmak için overwrite verin.", t.display()),
                "output",
            ));
        }
        let raster = match &input.layer {
            None => None,
            Some(layer) => {
                warnings.extend(checks::layer_at(doc, layer, "layer")?);
                let project = doc.settings().srid;
                if srid != 0 && srid != project {
                    return Err(refuse(
                        codes::CRS_MISMATCH,
                        format!(
                            "Karo tablosunun sistemi EPSG:{srid}, projeninki {}; raster çevrilmeden eklenmez. Dosyayı yazmak için layer'ı vermeyin.",
                            if project == 0 {
                                "yerel".to_owned()
                            } else {
                                format!("EPSG:{project}")
                            }
                        ),
                        "layer",
                    ));
                }
                Some(Entity::Raster(RasterEntity {
                    base: EntityBase {
                        look: Default::default(),
                        id: 0,
                        layer_id: layer.clone(),
                        color: None,
                        attrs: Default::default(),
                        label: None,
                        symbol: None,
                        line_weight: None,
                        label_pins: Vec::new(),
                        attachments: Vec::new(),
                    },
                    raster: RasterFields {
                        affine,
                        width: w.width,
                        height: w.height,
                        bands: 4,
                        sample: RasterSample::U8,
                        asset: None,
                        file: input.output.clone(),
                        url: None,
                        srid,
                        style: kentos_formats::raster::style::default_style(
                            4,
                            RasterSample::U8,
                            false,
                        ),
                        opacity: None,
                        dataset: None,
                        georef: None,
                    },
                }))
            }
        };
        checks::revision(doc, input.expected_revision.as_deref())?;
        Ok(Planned {
            out: RastersGpkgReadDone {
                tables,
                table: set.name.clone(),
                zoom: u32::try_from(zoom).unwrap_or(0),
                area,
                width: w.width,
                height: w.height,
                bands: 4,
                sample: RasterSample::U8,
                affine,
                srid,
                tiles_read: read,
                tiles_missing: expected - read,
                path: target.as_ref().map(|t| t.display().to_string()),
                bytes: tiff.as_ref().map(|t| t.len() as u64),
                uid: None,
                revision: doc.revision().to_string(),
            },
            raster,
            target,
            tiff,
            warnings,
        })
    }

    /// The canvas (`width` × `height` pixels of red, green, blue and alpha) as a tiled, Deflate-coded GeoTIFF.
    fn geotiff(
        canvas: &[u8],
        width: u32,
        height: u32,
        affine: [f64; 6],
        srid: u32,
    ) -> Result<Vec<u8>, String> {
        let image = Image {
            width,
            height,
            bands: 4,
            sample: RasterSample::U8,
            alpha: true,
            geo: Some(Geo {
                affine,
                epsg: (srid != 0).then_some(srid),
                geographic: srid != 0 && is_geographic(srid),
            }),
            nodata: None,
        };
        let (mut writer, mut file) = Writer::new(vec![image], LEVEL).map_err(|e| e.0)?;
        let side = TILE as usize;
        let mut samples = vec![0u8; side * side * 4];
        for ty in 0..height.div_ceil(TILE) {
            for tx in 0..width.div_ceil(TILE) {
                samples.fill(0);
                let (x0, y0) = (tx as usize * side, ty as usize * side);
                let across = side.min(width as usize - x0);
                for v in 0..side.min(height as usize - y0) {
                    let s = ((y0 + v) * width as usize + x0) * 4;
                    samples[v * side * 4..v * side * 4 + across * 4]
                        .copy_from_slice(&canvas[s..s + across * 4]);
                }
                let bytes = writer
                    .tile(0, tx, ty, &Samples::U8(samples.clone()))
                    .map_err(|e| e.0)?;
                file.extend_from_slice(&bytes);
            }
        }
        let (dirs, head) = writer.finish().map_err(|e| e.0)?;
        file.extend_from_slice(&dirs);
        file[..head.len()].copy_from_slice(&head);
        Ok(file)
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &RastersGpkgRead) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(p) => CommandResult::Completed {
                output: (),
                warnings: p.warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    /// What execute would read and write; writes nothing.
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &RastersGpkgRead,
    ) -> CommandResult<RastersGpkgReadDone> {
        match check(cx.doc, input) {
            Ok(p) => CommandResult::Completed {
                output: p.out,
                warnings: p.warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    /// Checks `input` again, writes the GeoTIFF (beside its place, read back, then moved there) and, with `layer`,
    /// adds the raster as one undo step “GeoPackage karoları”.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: RastersGpkgRead,
    ) -> CommandResult<RastersGpkgReadDone> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_RASTERS_GPKGREAD,
            kentos_commands::CAD_RASTERS_GPKGREAD_VERSION,
            input,
            write_execute,
        )
    }

    /// The GeoTIFF's bytes written at `path` whole or not at all: beside it, read back as the reader will, then
    /// moved over it.
    fn write_file(path: &Path, bytes: &[u8], width: u32, height: u32) -> Result<(), String> {
        let tmp = beside(path);
        let done = (|| -> Result<(), String> {
            std::fs::write(&tmp, bytes)
                .map_err(|e| format!("{} yazılamadı: {e}", tmp.display()))?;
            let opened = raster_files::open(Bytes::file(&tmp)?)?;
            let info = &opened.reader.info;
            if info.width != width || info.height != height || info.bands != 4 {
                return Err("yazılan GeoTIFF geri okununca aynı çıkmadı".into());
            }
            std::fs::rename(&tmp, path)
                .map_err(|e| format!("{} yerine konamadı: {e}", path.display()))
        })();
        if done.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        done
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: RastersGpkgRead,
    ) -> CommandResult<RastersGpkgReadDone> {
        let Planned {
            mut out,
            raster,
            target,
            tiff,
            warnings,
        } = match check(cx.doc, &input) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        if let (Some(path), Some(bytes)) = (&target, &tiff)
            && let Err(why) = write_file(path, bytes, out.width, out.height)
        {
            return refuse(
                codes::FILE_UNWRITABLE,
                format!("GeoTIFF yazılamadı: {why}. Hiçbir şey değişmedi."),
                "output",
            )
            .into();
        }
        if let Some(entity) = raster {
            let added = cx
                .doc
                .transact("GeoPackage karoları", |doc| checks::add(doc, entity));
            match added {
                Ok((_, uid, revision)) => {
                    out.uid = Some(uid);
                    out.revision = revision;
                }
                Err(stop) => return stop.into(),
            }
        }
        CommandResult::Completed {
            output: out,
            warnings,
        }
    }
}

// ── A raster written as tiles ──────────────────────────────────────────────────────────────────────────────────

pub mod write {
    use super::*;

    /// A table name SQLite takes plainly and GeoPackage does not reserve.
    pub(super) fn table_name_ok(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        let mut chars = name.chars();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && name.len() <= 64
            && !lower.starts_with("gpkg_")
            && !lower.starts_with("gpkgext_")
            && !lower.starts_with("sqlite_")
            && !lower.starts_with("rtree_")
    }

    /// The tile table's WKT system, by its EPSG code; the file's undefined cartesian one (id −1) for 0.
    fn srs_for(srid: u32) -> (i32, Option<Srs>) {
        if srid == 0 {
            return (-1, None);
        }
        let found = kentos_project::crs::system(srid).and_then(|s| {
            let core = s.transform_system()?;
            let wkt = kentos_geometry_core::crs::text::write_wkt(&s.name, &core, None)?;
            Some(Srs {
                name: s.name.clone(),
                id: srid as i32,
                organization: "EPSG".into(),
                organization_id: srid as i32,
                definition: wkt,
                description: None,
            })
        });
        let srs = found.unwrap_or(Srs {
            name: format!("EPSG:{srid}"),
            id: srid as i32,
            organization: "EPSG".into(),
            organization_id: srid as i32,
            definition: "undefined".into(),
            description: None,
        });
        (srid as i32, Some(srs))
    }

    /// What a raster is read as: its tile bands, and what it needs besides.
    struct Source {
        opened: raster_files::Opened,
        /// The source's bands: 1 grey, 2 grey and alpha, 3 colour, 4 colour and alpha.
        bands: u32,
    }

    struct Prepared {
        path: PathBuf,
        table: String,
        mode: GpkgTilesMode,
        identifier: Option<String>,
        raster: RasterFields,
        srid: u32,
        /// Tile bands: 1, 2 or 4.
        tile_bands: u32,
        levels: Vec<Level>,
        set: NewTileSet,
        source: Source,
        warnings: Vec<CommandWarning>,
    }

    fn zooms_of(levels: &[Level], a: f64, b: f64) -> Vec<TileMatrix> {
        let top = levels.len() as i64 - 1;
        let side = i64::from(GPKG_TILE_SIDE);
        let mut out: Vec<TileMatrix> = levels
            .iter()
            .enumerate()
            .map(|(j, l)| TileMatrix {
                zoom: top - j as i64,
                matrix_width: i64::from(l.matrix),
                matrix_height: i64::from(l.matrix),
                tile_width: side,
                tile_height: side,
                pixel_x_size: a * 2f64.powi(j as i32),
                pixel_y_size: b * 2f64.powi(j as i32),
            })
            .collect();
        out.reverse();
        out
    }

    fn check(doc: &Document, input: &RastersGpkgWrite) -> Result<Prepared, Stop> {
        if !checks::is_uid_text(&input.uid) {
            return Err(refuse(
                codes::INVALID_UID,
                format!(
                    "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.",
                    input.uid
                ),
                "uid",
            ));
        }
        let found = Uuid::parse_str(&input.uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|slot| doc.get(slot));
        let Some(entity) = found else {
            return Err(refuse(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir.",
                    input.uid
                ),
                "uid",
            ));
        };
        let Entity::Raster(r) = entity else {
            return Err(refuse(
                codes::NOT_A_RASTER,
                "Verilen nesne bir raster değil; yalnız raster karo olarak yazılır.".into(),
                "uid",
            ));
        };
        let drawing = input.drawing.as_deref();
        let path = given_path(&input.path, drawing, "path")?;
        if !has_extension(&path, "gpkg") {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                format!(
                    "“{}” bir .gpkg dosyası değil; karolar GeoPackage olarak yazılır.",
                    file_name(&input.path)
                ),
                "path",
            ));
        }
        if !path.parent().is_some_and(Path::is_dir) {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                format!(
                    "“{}” klasörü yok; GeoPackage var olan bir klasöre yazılır.",
                    path.parent().unwrap_or(Path::new("")).display()
                ),
                "path",
            ));
        }
        let table = input.table.clone().unwrap_or_else(|| "tiles".into());
        if !table_name_ok(&table) {
            return Err(refuse(
                codes::INVALID_TABLE_NAME,
                format!(
                    "“{table}” tablo adı olmaz: harf ya da alt çizgiyle başlamalı, yalnız harf, rakam ve alt çizgi içermeli, 64 karakteri geçmemeli ve gpkg_, sqlite_ ya da rtree_ ile başlamamalı."
                ),
                "table",
            ));
        }
        let f = &r.raster;
        let unsupported = |why: &str| {
            refuse(
                codes::RASTER_UNSUPPORTED,
                format!("Raster karo olarak yazılamaz: {why}."),
                "uid",
            )
        };
        if f.dataset.is_some() {
            return Err(unsupported("veri seti (NetCDF, mesh) gösteriyor"));
        }
        if f.asset.is_none() && f.file.is_none() {
            return Err(unsupported(
                "kaynağı bir adres; dosyasını indirip bağlayın ya da gömün",
            ));
        }
        if f.sample != RasterSample::U8 {
            return Err(unsupported("8 bitlik değil (karolar 8 bitlik PNG'dir)"));
        }
        if !(1..=4).contains(&f.bands) {
            return Err(unsupported(&format!("{} bantlı", f.bands)));
        }
        let [x0, a, b, y0, c, d] = f.affine;
        if !(b == 0.0 && c == 0.0 && a > 0.0 && d < 0.0) {
            return Err(refuse(
                codes::RASTER_NOT_NORTH_UP,
                "Raster dönük ya da kuzeyi yukarıda değil; karo ızgarası yatay ve düşey olduğu için önce Raster oturt ile yeniden örnekleyin.".into(),
                "uid",
            ));
        }
        let pixels = u64::from(f.width) * u64::from(f.height);
        if pixels > GPKG_TILES_MOST_PIXELS {
            return Err(refuse(
                codes::GPKG_TILES_TOO_LARGE,
                format!(
                    "Raster {} × {} piksel; karo olarak en çok 25 milyon piksel yazılır.",
                    f.width, f.height
                ),
                "uid",
            ));
        }
        // The source's bytes and header.
        let bytes = if let Some(id) = &f.asset {
            match crate::rasters_georef::asset_bytes(doc, id) {
                Some(b) => Bytes::Memory(b),
                None => {
                    return Err(refuse(
                        codes::UNKNOWN_ASSET,
                        format!(
                            "Rasterin kaynağı “{id}” projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir."
                        ),
                        "uid",
                    ));
                }
            }
        } else {
            let file = f.file.as_deref().unwrap_or_default();
            let found = find(file, None, drawing, &doc.settings().xref_paths);
            let Some(at) = found.at else {
                return Err(refuse(
                    codes::FILE_NOT_FOUND,
                    format!(
                        "Rasterin kaynağı {} bulunamadı. Bakılan yerler: {}.",
                        file_name(file),
                        found.tried.join(", ")
                    ),
                    "uid",
                ));
            };
            Bytes::file(&at).map_err(|e| refuse(codes::FILE_NOT_FOUND, e, "uid"))?
        };
        let opened = raster_files::open(bytes).map_err(|e| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("Rasterin kaynağı okunamadı: {e}"),
                "uid",
            )
        })?;
        let info = &opened.reader.info;
        if info.width != f.width || info.height != f.height {
            return Err(refuse(
                codes::IMPORT_UNREADABLE,
                format!(
                    "Rasterin kaynağı {} × {} piksel, kayıttaki {} × {}; kaynak değişmiş. Rasteri yeniden ekleyin.",
                    info.width, info.height, f.width, f.height
                ),
                "uid",
            ));
        }
        if opened.reader.palette.is_some() {
            return Err(unsupported("paletli (önce RGB'ye çevirin)"));
        }
        if info.sample != RasterSample::U8 || info.bands != f.bands {
            return Err(unsupported(
                "kaynağının bantları ya da veri türü kayıttakinden başka",
            ));
        }
        let tile_bands = match (info.bands, info.alpha) {
            (1, _) => 1,
            (2, true) => 2,
            (3, _) | (4, true) => info.bands + u32::from(info.bands == 3),
            (2, false) => return Err(unsupported("2 bantlı ve alfasız")),
            _ => {
                return Err(unsupported(
                    "4 bantlı ve alfasız (renkli ama alfa bandı değil)",
                ));
            }
        };
        let nodata = info.nodata.is_some();
        let source_bands = info.bands;
        // The file.
        let mode = input.mode;
        match mode {
            GpkgTilesMode::New => {
                if path.exists() && input.overwrite != Some(true) {
                    return Err(refuse(
                        codes::FILE_EXISTS,
                        format!(
                            "{} var; üzerine yazmak için overwrite verin ya da mode add ile tabloyu içine ekleyin.",
                            path.display()
                        ),
                        "path",
                    ));
                }
            }
            GpkgTilesMode::Add => {
                if !path.is_file() {
                    return Err(refuse(
                        codes::FILE_NOT_FOUND,
                        format!(
                            "{} bulunamadı; mode add var olan bir GeoPackage'a ekler.",
                            path.display()
                        ),
                        "path",
                    ));
                }
                let gp = GeoPackage::open(&path, false).map_err(|e| {
                    refuse(
                        codes::IMPORT_UNREADABLE,
                        format!(
                            "{} GeoPackage olarak okunamadı: {e}",
                            file_name(&input.path)
                        ),
                        "path",
                    )
                })?;
                let there = gp.table_exists(&table).map_err(|e| {
                    refuse(
                        codes::IMPORT_UNREADABLE,
                        format!("{} okunamadı: {e}", file_name(&input.path)),
                        "path",
                    )
                })?;
                if there {
                    return Err(refuse(
                        codes::TABLE_EXISTS,
                        format!(
                            "Dosyada “{table}” adlı tablo var; başka bir ad verin. Dosyaya dokunulmadı."
                        ),
                        "table",
                    ));
                }
            }
        }
        let mut warnings = Vec::new();
        if nodata {
            warnings.push(CommandWarning {
                code: codes::NODATA_LOST.into(),
                message: "Rasterin nodata değeri karolara yazılamaz (GeoPackage karoları alfa taşır); değer olduğu gibi piksel sayıldı.".into(),
                path: Some("uid".into()),
            });
        }
        let lv = levels(f.width, f.height);
        let matrices = zooms_of(&lv, a, -d);
        let top = lv.len() as i64 - 1;
        let span = f64::from(GPKG_TILE_SIDE) * 2f64.powi(top as i32);
        let (srs_id, _) = srs_for(f.srid);
        let set = NewTileSet {
            name: table.clone(),
            identifier: input.identifier.clone(),
            description: None,
            srs_id,
            bounds: [x0, y0 - span * -d, x0 + span * a, y0],
            data_bounds: [
                x0,
                y0 + f64::from(f.height) * d,
                x0 + f64::from(f.width) * a,
                y0,
            ],
            matrices,
        };
        Ok(Prepared {
            path,
            table,
            mode,
            identifier: input.identifier.clone(),
            raster: f.clone(),
            srid: f.srid,
            tile_bands,
            levels: lv,
            set,
            source: Source {
                opened,
                bands: source_bands,
            },
            warnings,
        })
    }

    fn zooms_out(p: &Prepared) -> Vec<GpkgTileZoom> {
        // Coarsest first, as the matrices are; the levels are finest first.
        p.set
            .matrices
            .iter()
            .zip(p.levels.iter().rev())
            .map(|(m, l)| zoom_info(m, u64::from(l.tiles_x) * u64::from(l.tiles_y), vec!["png"]))
            .collect()
    }

    fn answer(p: &Prepared, revision: String, bytes: u64) -> RastersGpkgWritten {
        RastersGpkgWritten {
            path: p.path.display().to_string(),
            table: p.table.clone(),
            mode: p.mode,
            srid: p.srid,
            bands: p.tile_bands,
            bounds: p.set.bounds,
            data_bounds: p.set.data_bounds,
            zooms: zooms_out(p),
            tiles: p
                .levels
                .iter()
                .map(|l| u64::from(l.tiles_x) * u64::from(l.tiles_y))
                .sum(),
            bytes,
            revision,
        }
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &RastersGpkgWrite) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(p) => CommandResult::Completed {
                output: (),
                warnings: p.warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    /// What execute would write; writes nothing (the file's size is not known before: 0).
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &RastersGpkgWrite,
    ) -> CommandResult<RastersGpkgWritten> {
        match check(cx.doc, input) {
            Ok(p) => {
                let out = answer(&p, cx.doc.revision().to_string(), 0);
                CommandResult::Completed {
                    output: out,
                    warnings: p.warnings,
                }
            }
            Err(stop) => stop.into(),
        }
    }

    /// The pyramid's tiles, finest level first: each level's canvas cut into PNGs, the next canvas the mean of this.
    struct Pyramid {
        levels: Vec<Level>,
        zooms: Vec<i64>,
        bands: usize,
        level: usize,
        canvas: Vec<u8>,
        column: u32,
        row: u32,
    }

    impl Pyramid {
        fn next(&mut self) -> Result<Option<NewTile>, String> {
            let side = GPKG_TILE_SIDE as usize;
            let l = self.levels[self.level];
            if self.column == l.tiles_x {
                self.column = 0;
                self.row += 1;
            }
            if self.row == l.tiles_y {
                if self.level + 1 == self.levels.len() {
                    return Ok(None);
                }
                let n = self.levels[self.level + 1];
                self.canvas = halve(
                    &self.canvas,
                    (l.tiles_x as usize * side, l.tiles_y as usize * side),
                    (n.tiles_x as usize * side, n.tiles_y as usize * side),
                    self.bands,
                );
                self.level += 1;
                self.column = 0;
                self.row = 0;
                return self.next();
            }
            let cw = l.tiles_x as usize * side;
            let mut buf = Vec::with_capacity(side * side * self.bands);
            for v in 0..side {
                let s = ((self.row as usize * side + v) * cw + self.column as usize * side)
                    * self.bands;
                buf.extend_from_slice(&self.canvas[s..s + side * self.bands]);
            }
            let data = png::encode(
                GPKG_TILE_SIDE,
                GPKG_TILE_SIDE,
                self.bands as u32,
                &buf,
                LEVEL,
            )
            .map_err(|e| e.0)?;
            let tile = NewTile {
                zoom: self.zooms[self.level],
                column: i64::from(self.column),
                row: i64::from(self.row),
                data,
            };
            self.column += 1;
            Ok(Some(tile))
        }
    }

    /// The finest canvas: the raster's pixels in the tile bands (a colour raster without alpha gets 255 inside it),
    /// the grid's padding 0.
    fn first_canvas(p: &mut Prepared) -> Result<Vec<u8>, Stop> {
        let f = &p.raster;
        let l = p.levels[0];
        let side = GPKG_TILE_SIDE as usize;
        let cw = l.tiles_x as usize * side;
        let ch = l.tiles_y as usize * side;
        let nb = p.tile_bands as usize;
        let region = p
            .source
            .opened
            .region(0, 0, f.width, f.height)
            .map_err(|e| {
                refuse(
                    codes::IMPORT_UNREADABLE,
                    format!("Rasterin kaynağı okunamadı: {e}"),
                    "uid",
                )
            })?
            .ok_or_else(|| {
                refuse(
                    codes::IMPORT_UNREADABLE,
                    "Rasterin pikselleri okunamadı.".into(),
                    "uid",
                )
            })?;
        let Samples::U8(src) = &region.samples else {
            return Err(refuse(
                codes::RASTER_UNSUPPORTED,
                "Raster karo olarak yazılamaz: 8 bitlik değil.".into(),
                "uid",
            ));
        };
        let sb = p.source.bands as usize;
        let mut canvas = vec![0u8; cw * ch * nb];
        for y in 0..f.height as usize {
            for x in 0..f.width as usize {
                let s = (y * f.width as usize + x) * sb;
                let d = (y * cw + x) * nb;
                canvas[d..d + sb].copy_from_slice(&src[s..s + sb]);
                if nb == sb + 1 {
                    canvas[d + sb] = 255;
                }
            }
        }
        Ok(canvas)
    }

    /// The file made whole beside its place, read back, then put there (mode `new`); or the table added in one
    /// transaction (mode `add`).
    fn write_file(p: &mut Prepared, overwrite: bool) -> Result<u64, String> {
        let canvas = first_canvas(p).map_err(|s| s.into_error().message)?;
        let (_, srs) = srs_for(p.srid);
        let mut pyramid = Pyramid {
            levels: p.levels.clone(),
            zooms: p.set.matrices.iter().rev().map(|m| m.zoom).collect(),
            bands: p.tile_bands as usize,
            level: 0,
            canvas,
            column: 0,
            row: 0,
        };
        let expected: u64 = p
            .levels
            .iter()
            .map(|l| u64::from(l.tiles_x) * u64::from(l.tiles_y))
            .sum();
        let mut set = p.set.clone();
        set.identifier = p.identifier.clone();
        let table = p.table.clone();
        let verify = |path: &Path| -> Result<(), String> {
            let gp = GeoPackage::open(path, false).map_err(|e| e.0)?;
            let sets = gp.tile_sets().map_err(|e| e.0)?;
            let back = sets
                .iter()
                .find(|s| s.name == table)
                .ok_or("yazılan tablo geri okunamadı")?;
            if back.matrices != set.matrices || back.bounds != set.bounds {
                return Err("yazılan karo matrisleri geri okununca aynı çıkmadı".into());
            }
            let mut n = 0u64;
            for m in &back.matrices {
                n += gp.tile_census(&table, m.zoom).map_err(|e| e.0)?.0;
            }
            if n != expected {
                return Err(format!("{expected} karo yazıldı, {n} karo okundu"));
            }
            Ok(())
        };
        match p.mode {
            GpkgTilesMode::New => {
                let tmp = beside(&p.path);
                let _ = std::fs::remove_file(&tmp);
                let done = (|| -> Result<u64, String> {
                    let mut gp = GeoPackage::create(&tmp).map_err(|e| e.0)?;
                    if let Some(s) = &srs {
                        gp.add_srs(s).map_err(|e| e.0)?;
                    }
                    let n = gp
                        .write_tile_set(&set, &mut || {
                            pyramid.next().map_err(kentos_geopackage::Error)
                        })
                        .map_err(|e| e.0)?;
                    gp.close().map_err(|e| e.0)?;
                    if n != expected {
                        return Err(format!("{expected} karo beklenirken {n} karo yazıldı"));
                    }
                    verify(&tmp)?;
                    if p.path.exists() && !overwrite {
                        return Err(format!("{} var", p.path.display()));
                    }
                    let size = std::fs::metadata(&tmp).map_err(|e| e.to_string())?.len();
                    std::fs::rename(&tmp, &p.path)
                        .map_err(|e| format!("{} yerine konamadı: {e}", p.path.display()))?;
                    Ok(size)
                })();
                if done.is_err() {
                    let _ = std::fs::remove_file(&tmp);
                    let _ = std::fs::remove_file(format!("{}-journal", tmp.display()));
                }
                done
            }
            GpkgTilesMode::Add => {
                let mut gp = GeoPackage::open(&p.path, true).map_err(|e| e.0)?;
                if let Some(s) = &srs {
                    gp.add_srs(s).map_err(|e| e.0)?;
                }
                let n = gp
                    .write_tile_set(&set, &mut || {
                        pyramid.next().map_err(kentos_geopackage::Error)
                    })
                    .map_err(|e| e.0)?;
                gp.close().map_err(|e| e.0)?;
                if n != expected {
                    return Err(format!("{expected} karo beklenirken {n} karo yazıldı"));
                }
                verify(&p.path)?;
                Ok(std::fs::metadata(&p.path).map_err(|e| e.to_string())?.len())
            }
        }
    }

    /// Writes the tile table; the drawing is not changed.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: RastersGpkgWrite,
    ) -> CommandResult<RastersGpkgWritten> {
        let mut p = match check(cx.doc, &input) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        match write_file(&mut p, input.overwrite == Some(true)) {
            Ok(bytes) => {
                let out = answer(&p, cx.doc.revision().to_string(), bytes);
                CommandResult::Completed {
                    output: out,
                    warnings: p.warnings,
                }
            }
            Err(why) => refuse(
                codes::FILE_UNWRITABLE,
                format!("GeoPackage yazılamadı: {why}. Dosya olduğu gibi kaldı."),
                "path",
            )
            .into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(w: i64, h: i64, px: f64) -> TileMatrix {
        TileMatrix {
            zoom: 3,
            matrix_width: w,
            matrix_height: h,
            tile_width: 256,
            tile_height: 256,
            pixel_x_size: px,
            pixel_y_size: px,
        }
    }

    #[test]
    fn a_window_is_whole_pixels_cut_to_the_grid() {
        let m = matrix(4, 3, 2.0);
        let set = [1000.0, 2000.0, 3048.0, 3536.0];
        // The set's box: 4 × 256 × 2 = 2048 wide, 3 × 256 × 2 = 1536 high from (1000, 3536).
        let w = window(&m, set, [1100.0, 2200.0, 1500.0, 3000.0]).unwrap();
        assert_eq!((w.x, w.y, w.width, w.height), (50, 268, 200, 400));
        assert_eq!(w.origin, [1100.0, 3000.0]);
        // Past the grid on every side: cut.
        let w = window(&m, set, [900.0, 1900.0, 4000.0, 3700.0]).unwrap();
        assert_eq!((w.x, w.y, w.width, w.height), (0, 0, 1024, 768));
        assert_eq!(w.origin, [1000.0, 3536.0]);
        // A half pixel rounds up: 1000.9 is 0.45 pixel in, and 1001 is half a pixel in.
        let w = window(&m, set, [1001.0, 3000.0, 1003.0, 3002.0]).unwrap();
        assert_eq!((w.x, w.width), (1, 1));
        // Nothing on the grid.
        assert!(window(&m, set, [5000.0, 2000.0, 6000.0, 3000.0]).is_none());
    }

    #[test]
    fn a_pyramid_halves_until_one_tile() {
        let l = levels(1000, 300);
        let dims: Vec<_> = l
            .iter()
            .map(|l| (l.width, l.height, l.tiles_x, l.tiles_y))
            .collect();
        assert_eq!(
            dims,
            vec![(1000, 300, 4, 2), (500, 150, 2, 1), (250, 75, 1, 1)]
        );
        // The matrices span the set's whole extent: 4 × 4 tiles at the finest zoom, 2 × 2, then one.
        assert_eq!(
            l.iter().map(|l| l.matrix).collect::<Vec<_>>(),
            vec![4, 2, 1]
        );
        assert_eq!(levels(256, 256).len(), 1);
        assert_eq!(levels(257, 1).len(), 2);
        assert_eq!(levels(1, 1).len(), 1);
    }

    #[test]
    fn a_canvas_halves_by_2_by_2_means_with_zero_past_its_edge() {
        // 3 × 1 pixel canvas, one band: (10, 20, 30); the coarser canvas is 2 × 1.
        let out = halve(&[10, 20, 30], (3, 1), (2, 1), 1);
        // (10 + 20 + 0 + 0 + 2) / 4 = 8; (30 + 0 + 0 + 0 + 2) / 4 = 8.
        assert_eq!(out, vec![8, 8]);
        let out = halve(&[255, 255, 255, 255], (2, 2), (1, 1), 1);
        assert_eq!(out, vec![255]);
        let out = halve(&[1, 1, 1, 0], (2, 2), (1, 1), 1);
        assert_eq!(out, vec![1]);
    }

    #[test]
    fn table_names_are_plain_identifiers() {
        assert!(write::table_name_ok("orto"));
        assert!(write::table_name_ok("_Orto_2"));
        assert!(!write::table_name_ok("2orto"));
        assert!(!write::table_name_ok("orto kaplama"));
        assert!(!write::table_name_ok("gpkg_contents"));
        assert!(!write::table_name_ok("SQLITE_x"));
        assert!(!write::table_name_ok(""));
    }
}
