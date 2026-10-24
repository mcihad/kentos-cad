//! GeoPackage tile pyramids as rasters (docs/adr/0556, 0627; plan/gis.md `CBS-03`'s raster profile).
//!
//! [`read`] is `cad.rasters.gpkgread`: a tile table's window at one zoom level read as one raster. The window's
//! rounding is GDAL's GeoPackage driver's (first pixel `floor((min − origin) / pixel + ½)`, size `floor((max − min) /
//! pixel + ½)`), cut to the level's grid. The tiles are read through the formats core's reader (`raster::gpkg`):
//! pictures (8-bit PNG, JPEG, WebP; any of them in one table) as four bands of bytes, a gridded coverage's values
//! (OGC 17-066r1: 16-bit PNG integers, 32-bit float TIFFs) as one band. The raster is written strip by strip as a
//! north-up GeoTIFF beside its name (`.kentos-yaziliyor`, read back, then renamed) and, with `layer`, added to the
//! drawing linked to it as one undo step. With `link` nothing is written: the raster added shows the table itself
//! (`RasterFields::tiles`), its window's first pixel moved up and left to the coarsest zoom it draws from.
//!
//! [`write`] is `cad.rasters.gpkgwrite`: a raster of the drawing written as a tile table, built strip by strip. The
//! pyramid's finest zoom is the raster's own pixel with its grid's origin at the raster's top-left corner; each coarser
//! zoom is a 2 × 2 mean of the finer one's padded canvas. Pictures: `(a + b + c + d + 2) / 4` per sample, samples past
//! the canvas 0 (a tile that covers more than the raster is transparent there), PNG or lossless WebP. A coverage: the
//! mean of the children inside the finer level's image and not null (an integer one rounded half up), null (0 without
//! one) where none is; 16-bit PNG integers for a raster of 16-bit integers (a signed one offset by −32768, without
//! nodata), 32-bit float TIFFs for the rest.
//!
//! [`TileSource`] reads a table's tiles for the formats core's reader: the desktop's view, the headless server's
//! analyses and these commands read a linked table the same way.
//!
//! The shared cases are `fixtures/commands/v1/cad.rasters.gpkgread.json` and `cad.rasters.gpkgwrite.json`
//! (`scripts/fixtures/gpkg_tiles_cases.py`, from GDAL's own GeoPackage driver).

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, GPKG_STRIP_MOST_BYTES, GPKG_TILE_SIDE,
    GPKG_TILES_MOST_PIXELS, GpkgCoverage, GpkgTileFormat, GpkgTileSet, GpkgTileZoom, GpkgTilesMode,
    RasterEntity, RasterFields, RasterSample, RasterTiles, RastersGpkgRead, RastersGpkgReadDone,
    RastersGpkgWrite, RastersGpkgWritten, codes,
};
use kentos_domain::{Document, Uuid};
use kentos_formats::raster::gpkg::{
    self as tile_core, Content, Coverage, TileLevel, TileScales, Zoom,
};
use kentos_formats::raster::source::{BlockNeed, Put, Reader};
use kentos_formats::raster::write::{Geo, Image, Writer};
use kentos_formats::raster::{Samples, TILE, png, webp};
use kentos_geopackage::tiles::{NewCoverage, NewTile, NewTileSet, TileMatrix, TileSet, TileStats};
use kentos_geopackage::{GeoPackage, Srs};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::links::find;
use crate::raster_files::{self, Bytes, READER_BUDGET, jpeg_pixels, jpeg_size};

/// Deflate's level for the GeoTIFF's tiles and the PNG tiles (1 fast … 9 small).
const LEVEL: u8 = 6;

/// The tiles of a window looked at before it is read: their formats and sizes.
const SAMPLE_TILES: u32 = 64;

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
#[cfg(test)]
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
        coverage: set.coverage.as_ref().map(|c| GpkgCoverage {
            datatype: if c.integer() { "integer" } else { "float" }.to_owned(),
            scale: c.scale,
            offset: c.offset,
            data_null: c.data_null,
            uom: c.uom.clone(),
        }),
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

/// What a table holds, as the reader reads it: pictures, or a gridded coverage as GDAL reads it (its band's type from
/// its row and its tiles' scales and offsets, docs/adr/0627 §2).
pub fn content_of(gp: &GeoPackage, set: &TileSet) -> Result<Content, String> {
    match &set.coverage {
        None => Ok(Content::Image),
        Some(c) => {
            let (plain, plain_or_one) = gp.coverage_tile_scales(&set.name).map_err(|e| e.0)?;
            Ok(Content::Coverage(Coverage::new(
                c.integer(),
                c.scale,
                c.offset,
                c.data_null,
                c.precision,
                TileScales {
                    plain,
                    plain_or_one,
                },
            )))
        }
    }
}

/// A table's zooms that hold tiles, as a linked raster's levels are planned by them: a zoom the matrix lists but no
/// tile fills (GDAL's file written without overviews) is no level, a level of its size is worked out instead (GDAL
/// shows no overview for it either; docs/adr/0627 §3).
pub fn held_zooms(gp: &GeoPackage, set: &TileSet) -> Result<Vec<Zoom>, String> {
    let held = gp.zooms_with_tiles(&set.name).map_err(|e| e.0)?;
    Ok(zooms_of(set)
        .into_iter()
        .filter(|z| held.contains(&z.zoom))
        .collect())
}

/// A table's zooms as the reader plans by them.
pub fn zooms_of(set: &TileSet) -> Vec<Zoom> {
    set.matrices
        .iter()
        .map(|m| Zoom {
            zoom: m.zoom,
            matrix_width: m.matrix_width,
            matrix_height: m.matrix_height,
            tile_width: u32::try_from(m.tile_width).unwrap_or(0),
            tile_height: u32::try_from(m.tile_height).unwrap_or(0),
            pixel: [m.pixel_x_size, m.pixel_y_size],
        })
        .collect()
}

// ── A table's tiles for the reader ─────────────────────────────────────────────────────────────────────────────

/// A GeoPackage tile table read for the formats core's reader (docs/adr/0627 §3): its tile blocks' bytes framed with
/// their scale and offset (`raster::gpkg::frame`), a tile the table does not hold as an empty frame.
pub struct TileSource {
    gp: GeoPackage,
    table: String,
    coverage: bool,
}

impl std::fmt::Debug for TileSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileSource")
            .field("table", &self.table)
            .finish()
    }
}

impl TileSource {
    /// The block's framed bytes: the tile at the block's zoom, column and row.
    pub fn framed(&self, reader: &Reader, need: &BlockNeed) -> Result<Vec<u8>, String> {
        let (zoom, column, row, _, _) = reader
            .tile_at(need)
            .ok_or("Rasterin bu karosu okunamadı.")?;
        self.framed_at(zoom, column, row)
    }

    /// The framed bytes of the tile at a zoom, column and row.
    pub fn framed_at(&self, zoom: i64, column: i64, row: i64) -> Result<Vec<u8>, String> {
        Ok(
            match self
                .gp
                .tile(&self.table, zoom, column, row, self.coverage)
                .map_err(|e| format!("{zoom}/{column}/{row} karosu okunamadı: {}", e.0))?
            {
                Some(b) => tile_core::frame(b.scale, b.offset, &b.data),
                None => tile_core::frame(1.0, 0.0, &[]),
            },
        )
    }

    /// Every tile a region of `level` wants, read, decoded and kept; why not (the tile's place in the words).
    pub fn fill(
        &self,
        reader: &mut Reader,
        level: usize,
        x: i64,
        y: i64,
        w: u32,
        h: u32,
    ) -> Result<(), TileFault> {
        let content = reader.tile_content().ok_or_else(|| TileFault {
            unsupported: false,
            why: "Raster bir karo tablosu değil.".into(),
        })?;
        for need in reader.needs(level, x, y, w, h) {
            let at = reader.tile_at(&need);
            let bytes = self.framed(reader, &need).map_err(|why| TileFault {
                unsupported: false,
                why,
            })?;
            let place = at.map_or_else(String::new, |(z, c, r, _, _)| {
                format!("{z}/{c}/{r} karosu ")
            });
            // A tile of a format its table does not hold is said as such, before it is decoded.
            if let Ok((_, _, data)) = tile_core::unframe(&bytes)
                && !data.is_empty()
                && let Err(p) = tile_core::header(&content, data)
            {
                return Err(TileFault {
                    unsupported: p.unsupported,
                    why: format!("{place}okunmaz: {}", p.why),
                });
            }
            let fault = |why: String| TileFault {
                unsupported: false,
                why,
            };
            if let Put::Jpeg(stream) = reader.put_block(&need, &bytes).map_err(|e| fault(e.0))? {
                let (pixels, comps) =
                    jpeg_pixels(&stream).map_err(|e| fault(format!("{place}okunamadı: {e}")))?;
                reader
                    .put_pixels(&need, pixels, comps)
                    .map_err(|e| fault(format!("{place}okunamadı: {}", e.0)))?;
            }
        }
        Ok(())
    }
}

/// Why a tile was not read: its words (its place in them), and whether its format is one its table does not hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileFault {
    pub unsupported: bool,
    pub why: String,
}

/// What a raster linked to a tile table is opened by (docs/adr/0627 §3; the scene's key holds it, the desktop's
/// tiles open it): the table, its zoom and window's first pixel, and the raster's size, bands and sample.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct TableOpen {
    pub table: String,
    pub zoom: u32,
    pub x: i64,
    pub y: i64,
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub sample: RasterSample,
}

impl TableOpen {
    /// A raster's own; none when it shows no tile table.
    pub fn of(f: &RasterFields) -> Option<TableOpen> {
        let t = f.tiles.as_ref()?;
        Some(TableOpen {
            table: t.table.clone(),
            zoom: t.zoom,
            x: t.x,
            y: t.y,
            width: f.width,
            height: f.height,
            bands: f.bands,
            sample: f.sample,
        })
    }
}

/// A raster linked to a GeoPackage tile table (`RasterFields::tiles`, docs/adr/0627 §3) opened from its file at
/// `path`: its reader (the raster's own size and place) and its tiles' source.
pub fn open_linked(
    path: &Path,
    f: &RasterFields,
    budget: usize,
) -> Result<(Reader, TileSource), String> {
    let open = TableOpen::of(f).ok_or("Raster bir karo tablosunu göstermiyor.")?;
    open_table(path, &open, Some(f.affine), f.srid, budget)
}

/// A tile table opened as `open` says: its reader (placed at `affine`, else at the window's own place on the table's
/// grid; `srid` the system when the table's is none) and its tiles' source; why not (the table gone, its content
/// changed, its zoom without tiles).
pub fn open_table(
    path: &Path,
    open: &TableOpen,
    affine: Option<[f64; 6]>,
    srid: u32,
    budget: usize,
) -> Result<(Reader, TileSource), String> {
    let gp = GeoPackage::open(path, false)
        .map_err(|e| format!("{} GeoPackage olarak açılamadı: {}", path.display(), e.0))?;
    let set = gp
        .tile_sets()
        .map_err(|e| e.0)?
        .into_iter()
        .find(|s| s.name == open.table)
        .ok_or_else(|| {
            format!(
                "“{}” karo tablosu {} dosyasında yok.",
                open.table,
                path.display()
            )
        })?;
    let content = content_of(&gp, &set)?;
    if content.bands() != open.bands || content.sample() != open.sample {
        return Err(format!(
            "“{}” karo tablosunun içeriği değişmiş ({} bant, {}); rasteri yeniden bağlayın.",
            open.table,
            content.bands(),
            content.sample().name()
        ));
    }
    let zooms = held_zooms(&gp, &set)?;
    let zoom = i64::from(open.zoom);
    let plan = tile_core::levels(&zooms, zoom, open.x, open.y, open.width, open.height);
    let (Some(Some(_)), Some(m)) = (plan.first(), set.matrix(zoom)) else {
        return Err(format!(
            "“{}” karo tablosunun {}. zoom düzeyi yok ya da karosu kalmamış.",
            open.table, open.zoom
        ));
    };
    let affine = affine.unwrap_or([
        set.bounds[0] + open.x as f64 * m.pixel_x_size,
        m.pixel_x_size,
        0.0,
        set.bounds[3] - open.y as f64 * m.pixel_y_size,
        0.0,
        -m.pixel_y_size,
    ]);
    let epsg = srid_of(&gp, set.srs_id)?.or((srid != 0).then_some(srid));
    let reader = Reader::tiles(
        content,
        open.width,
        open.height,
        &plan,
        affine,
        epsg,
        epsg.is_some_and(is_geographic),
        budget,
    )
    .map_err(|e| e.0)?;
    Ok((
        reader,
        TileSource {
            gp,
            table: set.name.clone(),
            coverage: set.coverage.is_some(),
        },
    ))
}

/// A tile's problem before its pixels are read: its format against the table's kind, its size against the matrix's.
fn tile_problem(content: &Content, data: &[u8], m: &TileMatrix) -> Option<(bool, String)> {
    let head = match tile_core::header(content, data) {
        Ok(h) => h,
        Err(p) => return Some((p.unsupported, p.why)),
    };
    let size = match head.size {
        Some(s) => s,
        None => match jpeg_size(data) {
            Ok(s) => s,
            Err(_) => return Some((false, "JPEG'in başlığı okunmuyor".into())),
        },
    };
    let want = (
        u32::try_from(m.tile_width).unwrap_or(0),
        u32::try_from(m.tile_height).unwrap_or(0),
    );
    (size != want).then(|| {
        (
            false,
            format!(
                "{} × {} piksel, matrisin karo boyu {} × {}",
                size.0, size.1, want.0, want.1
            ),
        )
    })
}

// ── The raster read from tiles ─────────────────────────────────────────────────────────────────────────────────

pub mod read {
    use super::*;

    /// What a run found and made: the answer, the raster entity to add, and what a GeoTIFF is read from.
    struct Planned {
        out: RastersGpkgReadDone,
        raster: Option<Entity>,
        writing: Option<Writing>,
        warnings: Vec<CommandWarning>,
    }

    /// A GeoTIFF to read and write: the GeoPackage and its table, the zoom, the window, the content, where it goes.
    struct Writing {
        source: PathBuf,
        table: String,
        coverage: bool,
        zoom: Zoom,
        window: Window,
        content: Content,
        affine: [f64; 6],
        srid: u32,
        target: PathBuf,
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
        let link = input.link == Some(true);
        if link && (target.is_some() || input.layer.is_none()) {
            return Err(refuse(
                codes::LINK_UNUSABLE,
                if target.is_some() {
                    "Bağlı okuma (link) GeoTIFF yazmaz: karolar dosyadan okunur; output'u vermeyin."
                        .into()
                } else {
                    "Bağlı okuma (link) rasteri bir katmana ekler; layer'ı verin.".into()
                },
                "link",
            ));
        }
        if input.layer.is_some() && target.is_none() && !link {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                "layer verildi ama output yok; rasteri eklemek için yazılacak GeoTIFF'in yolunu (output) verin ya da tabloya bağlı ekleyin (link).".into(),
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
        let unreadable = |why: String| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("{} okunamadı: {why}", file_name(&input.path)),
                "path",
            )
        };
        let sets = gp.tile_sets().map_err(|e| unreadable(e.0))?;
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
        let empty_at = if input.area.is_some() {
            "area"
        } else {
            "table"
        };
        let Some(mut w) = window(m, set.bounds, rect) else {
            return Err(refuse(
                codes::TILE_TABLE_EMPTY,
                format!(
                    "Okunacak alan “{}” tablosunun {zoom}. düzeyinin ızgarasının dışında; alanı ızgaraya çekin.",
                    set.name
                ),
                empty_at,
            ));
        };
        let content = content_of(&gp, set).map_err(unreadable)?;
        let zooms = zooms_of(set);
        if link {
            // The window moved up and left so that each coarser zoom that holds tiles has whole pixels of a level.
            let held = held_zooms(&gp, set).map_err(unreadable)?;
            let (x, y, width, height) =
                tile_core::linked_window(&held, zoom, w.x, w.y, w.width, w.height);
            w = Window {
                x,
                y,
                width,
                height,
                origin: [
                    set.bounds[0] + x as f64 * m.pixel_x_size,
                    set.bounds[3] - y as f64 * m.pixel_y_size,
                ],
            };
        }
        let pixels = u64::from(w.width) * u64::from(w.height);
        let too_large = |why: String| {
            refuse(
                codes::GPKG_TILES_TOO_LARGE,
                why,
                if input.zoom.is_some() || input.area.is_some() {
                    "area"
                } else {
                    "zoom"
                },
            )
        };
        let (tw, th) = (m.tile_width, m.tile_height);
        let (c0, c1) = (w.x / tw, (w.x + i64::from(w.width) - 1) / tw);
        let (r0, r1) = (w.y / th, (w.y + i64::from(w.height) - 1) / th);
        let expected = ((c1 - c0 + 1) as u64).saturating_mul((r1 - r0 + 1) as u64);
        if target.is_some() && pixels > GPKG_TILES_MOST_PIXELS {
            return Err(too_large(format!(
                "Raster {} × {} piksel olurdu; GeoTIFF olarak en çok 4 milyar piksel okunur. Daha kaba bir zoom düzeyi ya da dar bir alan verin, ya da tabloya bağlı ekleyin (link).",
                w.width, w.height
            )));
        }
        if link && expected > u64::from(u32::MAX) {
            return Err(too_large(format!(
                "Pencerede {expected} karo var; bir düzeyde en çok 4 milyar karo okunur. Dar bir alan verin."
            )));
        }
        let read = gp
            .tile_count_in(&set.name, zoom, (c0, c1), (r0, r1))
            .map_err(|e| unreadable(e.0))?;
        if read == 0 {
            return Err(refuse(
                codes::TILE_TABLE_EMPTY,
                format!(
                    "“{}” tablosunun {zoom}. düzeyinde okunan pencerede hiç karo yok.",
                    set.name
                ),
                empty_at,
            ));
        }
        // The tiles that will be read: their formats and sizes, looked at in up to 64 of them.
        if target.is_some() || link {
            let sample = gp
                .tile_sample(&set.name, zoom, (c0, c1), (r0, r1), SAMPLE_TILES)
                .map_err(|e| unreadable(e.0))?;
            for (c, r, data) in &sample {
                if let Some((unsupported, why)) = tile_problem(&content, data, m) {
                    return Err(refuse(
                        if unsupported {
                            codes::TILE_FORMAT_UNSUPPORTED
                        } else {
                            codes::TILE_UNREADABLE
                        },
                        format!("{zoom}/{c}/{r} karosu okunmaz: {why}."),
                        "table",
                    ));
                }
            }
        }
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
                    "Okunan pencerenin {} karosu tabloda yok; o yerler {}.",
                    expected - read,
                    if content == Content::Image {
                        "saydam (alfa 0) kaldı"
                    } else {
                        "boş (nodata) kaldı"
                    }
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
        let (bands, sample) = (content.bands(), content.sample());
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
                        bands,
                        sample,
                        asset: None,
                        file: if link {
                            Some(input.path.clone())
                        } else {
                            input.output.clone()
                        },
                        url: None,
                        srid,
                        style: kentos_formats::raster::style::default_style(bands, sample, false),
                        opacity: None,
                        dataset: None,
                        georef: None,
                        tiles: link.then(|| RasterTiles {
                            table: set.name.clone(),
                            zoom: u32::try_from(zoom).unwrap_or(0),
                            x: w.x,
                            y: w.y,
                        }),
                    },
                }))
            }
        };
        checks::revision(doc, input.expected_revision.as_deref())?;
        let writing = target.as_ref().map(|t| Writing {
            source: path.clone(),
            table: set.name.clone(),
            coverage: set.coverage.is_some(),
            zoom: zooms
                .iter()
                .find(|z| z.zoom == zoom)
                .copied()
                .unwrap_or(zooms[0]),
            window: w,
            content,
            affine,
            srid,
            target: t.clone(),
        });
        Ok(Planned {
            out: RastersGpkgReadDone {
                tables,
                table: set.name.clone(),
                zoom: u32::try_from(zoom).unwrap_or(0),
                area,
                width: w.width,
                height: w.height,
                bands,
                sample,
                nodata: content.nodata(),
                linked: link,
                affine,
                srid,
                tiles_read: read,
                tiles_missing: expected - read,
                path: target.as_ref().map(|t| t.display().to_string()),
                bytes: None,
                uid: None,
                revision: doc.revision().to_string(),
            },
            raster,
            writing,
            warnings,
        })
    }

    /// The window's tiles read through the reader and written as a tiled, Deflate-coded GeoTIFF, strip by strip,
    /// beside its place, read back, then moved there; its size. A tile that does not read: its words (`unsupported`
    /// when its format is one the table does not hold).
    fn write_file(w: &Writing) -> Result<u64, (bool, String)> {
        let plain = |e: String| (false, e);
        let gp = GeoPackage::open(&w.source, false).map_err(|e| plain(e.0))?;
        let source = TileSource {
            gp,
            table: w.table.clone(),
            coverage: w.coverage,
        };
        let (width, height) = (w.window.width, w.window.height);
        let mut reader = Reader::tiles(
            w.content,
            width,
            height,
            &[Some(TileLevel {
                zoom: w.zoom,
                x: w.window.x,
                y: w.window.y,
            })],
            w.affine,
            (w.srid != 0).then_some(w.srid),
            w.srid != 0 && is_geographic(w.srid),
            READER_BUDGET,
        )
        .map_err(|e| plain(e.0))?;
        let bands = w.content.bands();
        let image = Image {
            width,
            height,
            bands,
            sample: w.content.sample(),
            alpha: w.content == Content::Image,
            geo: Some(Geo {
                affine: w.affine,
                epsg: (w.srid != 0).then_some(w.srid),
                geographic: w.srid != 0 && is_geographic(w.srid),
            }),
            nodata: w.content.nodata(),
        };
        let (mut writer, head) = Writer::new(vec![image], LEVEL).map_err(|e| plain(e.0))?;
        let tmp = beside(&w.target);
        let done = (|| -> Result<u64, (bool, String)> {
            let mut f = File::create(&tmp)
                .map_err(|e| plain(format!("{} yazılamadı: {e}", tmp.display())))?;
            let io = |e: std::io::Error| plain(format!("{} yazılamadı: {e}", tmp.display()));
            f.write_all(&head).map_err(io)?;
            let side = TILE as usize;
            let b = bands as usize;
            for ty in 0..height.div_ceil(TILE) {
                for tx in 0..width.div_ceil(TILE) {
                    let (x, y) = (i64::from(tx * TILE), i64::from(ty * TILE));
                    let cw = TILE.min(width - tx * TILE);
                    let ch = TILE.min(height - ty * TILE);
                    source
                        .fill(&mut reader, 0, x, y, cw, ch)
                        .map_err(|f| (f.unsupported, f.why))?;
                    let region = reader
                        .region(0, x, y, cw, ch)
                        .ok_or_else(|| plain("karolar okunamadı".into()))?;
                    let mut samples = Samples::filled(w.content.sample(), side * side * b, 0.0);
                    for v in 0..ch as usize {
                        samples.copy_run(
                            v * side * b,
                            &region.samples,
                            v * cw as usize * b,
                            cw as usize * b,
                        );
                    }
                    let bytes = writer.tile(0, tx, ty, &samples).map_err(|e| plain(e.0))?;
                    f.write_all(&bytes).map_err(io)?;
                }
            }
            let (dirs, head) = writer.finish().map_err(|e| plain(e.0))?;
            f.write_all(&dirs).map_err(io)?;
            f.seek(SeekFrom::Start(0)).map_err(io)?;
            f.write_all(&head).map_err(io)?;
            f.sync_all().map_err(io)?;
            drop(f);
            let opened = raster_files::open(Bytes::file(&tmp).map_err(plain)?).map_err(plain)?;
            let info = &opened.reader.info;
            if info.width != width || info.height != height || info.bands != bands {
                return Err(plain("yazılan GeoTIFF geri okununca aynı çıkmadı".into()));
            }
            let size = std::fs::metadata(&tmp)
                .map_err(|e| plain(e.to_string()))?
                .len();
            std::fs::rename(&tmp, &w.target)
                .map_err(|e| plain(format!("{} yerine konamadı: {e}", w.target.display())))?;
            Ok(size)
        })();
        if done.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        done
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
    /// adds the raster (linked to the GeoTIFF, or with `link` to the table) as one undo step “GeoPackage karoları”.
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

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: RastersGpkgRead,
    ) -> CommandResult<RastersGpkgReadDone> {
        let Planned {
            mut out,
            raster,
            writing,
            warnings,
        } = match check(cx.doc, &input) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        if let Some(w) = &writing {
            match write_file(w) {
                Ok(bytes) => out.bytes = Some(bytes),
                Err((unsupported, why)) if why.contains("karosu") || unsupported => {
                    return refuse(
                        if unsupported {
                            codes::TILE_FORMAT_UNSUPPORTED
                        } else {
                            codes::TILE_UNREADABLE
                        },
                        format!("{why}. Hiçbir şey yazılmadı."),
                        "table",
                    )
                    .into();
                }
                Err((_, why)) => {
                    return refuse(
                        codes::FILE_UNWRITABLE,
                        format!("GeoTIFF yazılamadı: {why}. Hiçbir şey değişmedi."),
                        "output",
                    )
                    .into();
                }
            }
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

    /// What the tiles hold (docs/adr/0627 §2).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(super) enum Kind {
        /// Pictures of `bands` bytes a pixel (1 grey, 2 grey and alpha, 4 RGBA), PNG or lossless WebP.
        Picture { bands: usize, webp: bool },
        /// A coverage of 16-bit integers: a stored value is the value less `offset` (0, or −32768 for a signed
        /// raster); `null` its stored null.
        Integer { offset: i32, null: Option<u16> },
        /// A coverage of 32-bit floats; `null` its null value.
        Float { null: Option<f32> },
    }

    impl Kind {
        fn bands(self) -> usize {
            match self {
                Kind::Picture { bands, .. } => bands,
                _ => 1,
            }
        }

        /// The tiles' format in the answer.
        fn format(self) -> &'static str {
            match self {
                Kind::Picture { webp: true, .. } => "webp",
                Kind::Float { .. } => "tiff",
                _ => "png",
            }
        }

        fn coverage(self) -> Option<GpkgCoverage> {
            match self {
                Kind::Picture { .. } => None,
                Kind::Integer { offset, null } => Some(GpkgCoverage {
                    datatype: "integer".into(),
                    scale: 1.0,
                    offset: f64::from(offset),
                    data_null: null.map(f64::from),
                    uom: None,
                }),
                Kind::Float { null } => Some(GpkgCoverage {
                    datatype: "float".into(),
                    scale: 1.0,
                    offset: 0.0,
                    data_null: null.map(f64::from),
                    uom: None,
                }),
            }
        }
    }

    struct Prepared {
        path: PathBuf,
        table: String,
        mode: GpkgTilesMode,
        identifier: Option<String>,
        raster: RasterFields,
        srid: u32,
        kind: Kind,
        levels: Vec<Level>,
        set: NewTileSet,
        source: raster_files::Opened,
        /// The source's bands (a picture's; 1 for a coverage) and its nodata.
        source_bands: usize,
        nodata: Option<f64>,
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

    /// The raster's source opened for its pixels: its file, its library asset, or the tile table it shows.
    fn open_source(
        doc: &Document,
        f: &RasterFields,
        drawing: Option<&str>,
    ) -> Result<raster_files::Opened, Stop> {
        if let Some(id) = &f.asset {
            return match crate::rasters_georef::asset_bytes(doc, id) {
                Some(b) => raster_files::open(Bytes::Memory(b)).map_err(|e| {
                    refuse(
                        codes::IMPORT_UNREADABLE,
                        format!("Rasterin kaynağı okunamadı: {e}"),
                        "uid",
                    )
                }),
                None => Err(refuse(
                    codes::UNKNOWN_ASSET,
                    format!(
                        "Rasterin kaynağı “{id}” projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir."
                    ),
                    "uid",
                )),
            };
        }
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
        let opened = if f.tiles.is_some() {
            raster_files::open_tiles(&at, f)
        } else {
            Bytes::file(&at).and_then(raster_files::open)
        };
        opened.map_err(|e| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("Rasterin kaynağı okunamadı: {e}"),
                "uid",
            )
        })
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
        let coverage = f.sample != RasterSample::U8;
        if coverage && f.bands != 1 {
            return Err(unsupported(&format!(
                "{} bantlı ve 8 bitten fazla (yükseklik karosu tek bantlıdır)",
                f.bands
            )));
        }
        if f.sample == RasterSample::I8 {
            return Err(unsupported("8 bitlik işaretli"));
        }
        if !(1..=4).contains(&f.bands) {
            return Err(unsupported(&format!("{} bantlı", f.bands)));
        }
        let webp = input.format == Some(GpkgTileFormat::Webp);
        if coverage && webp {
            return Err(refuse(
                codes::TILE_FORMAT_UNSUPPORTED,
                "Yükseklik (kapsam) karoları WebP yazılmaz: 16 bitlik PNG ya da 32 bitlik TIFF olur; format'ı vermeyin."
                    .into(),
                "format",
            ));
        }
        let [x0, a, b, y0, c, d] = f.affine;
        if !(b == 0.0 && c == 0.0 && a > 0.0 && d < 0.0) {
            return Err(refuse(
                codes::RASTER_NOT_NORTH_UP,
                "Raster dönük ya da kuzeyi yukarıda değil; karo ızgarası yatay ve düşey olduğu için önce Raster oturt ile yeniden örnekleyin.".into(),
                "uid",
            ));
        }
        // The finest strip: 256 rows of the tile grid, every band.
        let lv = levels(f.width, f.height);
        let bytes_per = if coverage {
            if f.sample == RasterSample::U16 || f.sample == RasterSample::I16 {
                2
            } else {
                4
            }
        } else {
            4
        };
        let strip = u64::from(lv[0].tiles_x)
            * u64::from(GPKG_TILE_SIDE)
            * u64::from(GPKG_TILE_SIDE)
            * bytes_per;
        if strip > GPKG_STRIP_MOST_BYTES {
            return Err(refuse(
                codes::GPKG_TILES_TOO_LARGE,
                format!(
                    "Raster {} piksel genişliğinde; karolar 256 satırlık şeritlerle yazılır ve bir şerit en çok 256 MB olur. Rasteri bölün ya da daha kaba yeniden örnekleyin.",
                    f.width
                ),
                "uid",
            ));
        }
        let source = open_source(doc, f, drawing)?;
        let info = &source.reader.info;
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
        if source.reader.palette.is_some() {
            return Err(unsupported("paletli (önce RGB'ye çevirin)"));
        }
        if info.sample != f.sample || info.bands != f.bands {
            return Err(unsupported(
                "kaynağının bantları ya da veri türü kayıttakinden başka",
            ));
        }
        let nodata = info.nodata;
        let kind = if coverage {
            match (f.sample, nodata) {
                (RasterSample::U16, nd) => Kind::Integer {
                    offset: 0,
                    null: nd
                        .filter(|n| n.fract() == 0.0 && (0.0..=65535.0).contains(n))
                        .map(|n| n as u16),
                },
                (RasterSample::I16, None) => Kind::Integer {
                    offset: -32768,
                    null: None,
                },
                (_, nd) => Kind::Float {
                    null: nd.map(|n| n as f32),
                },
            }
        } else {
            let bands = match (info.bands, info.alpha) {
                (1, _) => 1,
                (2, true) => 2,
                (3, _) | (4, true) => 4,
                (2, false) => return Err(unsupported("2 bantlı ve alfasız")),
                _ => {
                    return Err(unsupported(
                        "4 bantlı ve alfasız (renkli ama alfa bandı değil)",
                    ));
                }
            };
            Kind::Picture { bands, webp }
        };
        let source_bands = info.bands as usize;
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
        if nodata.is_some() && matches!(kind, Kind::Picture { .. }) {
            warnings.push(CommandWarning {
                code: codes::NODATA_LOST.into(),
                message: "Rasterin nodata değeri karolara yazılamaz (GeoPackage karoları alfa taşır); değer olduğu gibi piksel sayıldı.".into(),
                path: Some("uid".into()),
            });
        }
        if let (Kind::Integer { null: None, .. }, Some(n)) = (kind, nodata) {
            warnings.push(CommandWarning {
                code: codes::NODATA_LOST.into(),
                message: format!(
                    "Rasterin nodata değeri ({n}) 16 bitlik karonun değeri olamaz; boş değer yazılmadı."
                ),
                path: Some("uid".into()),
            });
        }
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
            coverage: kind.coverage().map(|c| NewCoverage {
                datatype: c.datatype,
                scale: c.scale,
                offset: c.offset,
                data_null: c.data_null,
                uom: None,
            }),
            webp: matches!(kind, Kind::Picture { webp: true, .. }),
        };
        Ok(Prepared {
            path,
            table,
            mode,
            identifier: input.identifier.clone(),
            raster: f.clone(),
            srid: f.srid,
            kind,
            levels: lv,
            set,
            source,
            source_bands,
            nodata,
            warnings,
        })
    }

    fn zooms_out(p: &Prepared) -> Vec<GpkgTileZoom> {
        // Coarsest first, as the matrices are; the levels are finest first.
        p.set
            .matrices
            .iter()
            .zip(p.levels.iter().rev())
            .map(|(m, l)| {
                zoom_info(
                    m,
                    u64::from(l.tiles_x) * u64::from(l.tiles_y),
                    vec![p.kind.format()],
                )
            })
            .collect()
    }

    fn answer(p: &Prepared, revision: String, bytes: u64) -> RastersGpkgWritten {
        RastersGpkgWritten {
            path: p.path.display().to_string(),
            table: p.table.clone(),
            mode: p.mode,
            srid: p.srid,
            bands: p.kind.bands() as u32,
            coverage: p.kind.coverage(),
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

    /// A level's strip of samples: 256 rows of its tile grid, bands interleaved.
    #[derive(Clone, Debug, PartialEq)]
    pub(super) enum Buf {
        U8(Vec<u8>),
        U16(Vec<u16>),
        F32(Vec<f32>),
    }

    /// The pyramid built strip by strip (docs/adr/0627 §4): each level holds one strip of 256 rows of its tile grid;
    /// a finer strip halved fills half of the coarser one, which is cut into tiles when both halves are in (or its
    /// finer level ended). Tiles are given finest first within a strip, a coarser strip's after the strip that ends it.
    pub(super) struct Pyramid<'a> {
        levels: Vec<Level>,
        zooms: Vec<i64>,
        kind: Kind,
        strips: Vec<Buf>,
        /// Each level's strip: the tile row it holds, and its halves filled (finer levels' strips halved into it).
        rows: Vec<u32>,
        halves: Vec<u8>,
        /// The next source row (of level 0's tile grid) to read.
        next: u32,
        made: std::collections::VecDeque<NewTile>,
        read: &'a mut dyn FnMut(u32, u32) -> Result<Buf, String>,
        fill: f64,
        done: bool,
    }

    impl<'a> Pyramid<'a> {
        pub(super) fn new(
            levels: Vec<Level>,
            zooms: Vec<i64>,
            kind: Kind,
            read: &'a mut dyn FnMut(u32, u32) -> Result<Buf, String>,
        ) -> Pyramid<'a> {
            let fill = match kind {
                Kind::Picture { .. } => 0.0,
                Kind::Integer { offset, null } => null.map_or(f64::from(-offset), f64::from),
                Kind::Float { null } => null.map_or(0.0, f64::from),
            };
            let strips = levels
                .iter()
                .map(|l| blank(kind, l.tiles_x as usize * GPKG_TILE_SIDE as usize, fill))
                .collect();
            let n = levels.len();
            Pyramid {
                levels,
                zooms,
                kind,
                strips,
                rows: vec![0; n],
                halves: vec![0; n],
                next: 0,
                made: std::collections::VecDeque::new(),
                read,
                fill,
                done: false,
            }
        }

        pub(super) fn next(&mut self) -> Result<Option<NewTile>, String> {
            loop {
                if let Some(t) = self.made.pop_front() {
                    return Ok(Some(t));
                }
                if self.done {
                    return Ok(None);
                }
                if self.next < self.levels[0].tiles_y {
                    let row = self.next;
                    self.next += 1;
                    let rows = GPKG_TILE_SIDE.min(self.levels[0].height - row * GPKG_TILE_SIDE);
                    self.strips[0] = (self.read)(row, rows)?;
                    self.rows[0] = row;
                    // Halved into the coarser strip before it is cut (cutting blanks it).
                    self.push_down(0, row + 1 == self.levels[0].tiles_y)?;
                    self.cut(0)?;
                } else {
                    // Level 0 ended: every coarser strip with a half in is cut.
                    for k in 1..self.levels.len() {
                        if self.halves[k] > 0 {
                            self.push_down(k, true)?;
                            self.cut(k)?;
                        }
                    }
                    self.done = true;
                }
            }
        }

        /// Level `k`'s strip halved into half of level `k + 1`'s; that one cut when both halves are in or `last`.
        fn push_down(&mut self, k: usize, last: bool) -> Result<(), String> {
            if k + 1 >= self.levels.len() {
                return Ok(());
            }
            let half = (self.rows[k] % 2) as usize;
            let from = self.levels[k];
            let to = self.levels[k + 1];
            let fw = from.tiles_x as usize * GPKG_TILE_SIDE as usize;
            let tw = to.tiles_x as usize * GPKG_TILE_SIDE as usize;
            let base_row = self.rows[k] as usize * GPKG_TILE_SIDE as usize;
            let (lower, upper) = self.strips.split_at_mut(k + 1);
            halve_into(
                self.kind,
                &lower[k],
                fw,
                (from.width as usize, from.height as usize),
                base_row,
                &mut upper[0],
                tw,
                half * GPKG_TILE_SIDE as usize / 2,
                self.fill,
            );
            self.rows[k + 1] = self.rows[k] / 2;
            self.halves[k + 1] += 1;
            if half == 1 || last {
                let ended = last && self.rows[k + 1] + 1 == to.tiles_y;
                self.push_down(k + 1, ended)?;
                self.cut(k + 1)?;
            }
            Ok(())
        }

        /// Level `k`'s strip cut into its row of tiles; the strip blanked for the next row.
        fn cut(&mut self, k: usize) -> Result<(), String> {
            let l = self.levels[k];
            let side = GPKG_TILE_SIDE as usize;
            let w = l.tiles_x as usize * side;
            let b = self.kind.bands();
            let row = self.rows[k];
            for column in 0..l.tiles_x {
                let x0 = column as usize * side;
                let (data, stats) = match &self.strips[k] {
                    Buf::U8(v) => {
                        let mut t = Vec::with_capacity(side * side * b);
                        for y in 0..side {
                            let s = (y * w + x0) * b;
                            t.extend_from_slice(&v[s..s + side * b]);
                        }
                        let data = match self.kind {
                            Kind::Picture { webp: true, .. } => {
                                webp::encode_lossless(GPKG_TILE_SIDE, GPKG_TILE_SIDE, b as u32, &t)
                            }
                            _ => png::encode(GPKG_TILE_SIDE, GPKG_TILE_SIDE, b as u32, &t, LEVEL),
                        }
                        .map_err(|e| e.0)?;
                        (data, None)
                    }
                    Buf::U16(v) => {
                        let mut t = Vec::with_capacity(side * side);
                        for y in 0..side {
                            let s = y * w + x0;
                            t.extend_from_slice(&v[s..s + side]);
                        }
                        let Kind::Integer { offset, null } = self.kind else {
                            return Err("karonun türü uymuyor".into());
                        };
                        let stats = tile_stats(
                            t.iter()
                                .enumerate()
                                .filter_map(|(i, &s)| inside(l, row, column, i).then_some(s))
                                .filter(|&s| Some(s) != null)
                                .map(|s| f64::from(i32::from(s) + offset)),
                        );
                        (
                            png::encode16(GPKG_TILE_SIDE, GPKG_TILE_SIDE, &t, LEVEL)
                                .map_err(|e| e.0)?,
                            Some(stats),
                        )
                    }
                    Buf::F32(v) => {
                        let mut t = Vec::with_capacity(side * side);
                        for y in 0..side {
                            let s = y * w + x0;
                            t.extend_from_slice(&v[s..s + side]);
                        }
                        let Kind::Float { null } = self.kind else {
                            return Err("karonun türü uymuyor".into());
                        };
                        let stats = tile_stats(
                            t.iter()
                                .enumerate()
                                .filter_map(|(i, &s)| inside(l, row, column, i).then_some(s))
                                .filter(|&s| !s.is_nan() && Some(s) != null)
                                .map(f64::from),
                        );
                        (
                            tile_core::float_tiff(GPKG_TILE_SIDE, GPKG_TILE_SIDE, &t)
                                .map_err(|e| e.0)?,
                            Some(stats),
                        )
                    }
                };
                self.made.push_back(NewTile {
                    zoom: self.zooms[k],
                    column: i64::from(column),
                    row: i64::from(row),
                    data,
                    stats,
                });
            }
            self.strips[k] = blank(self.kind, w, self.fill);
            self.halves[k] = 0;
            Ok(())
        }
    }

    /// Whether sample `i` of tile (`column`, `row`) of level `l` lies in the level's image.
    fn inside(l: Level, row: u32, column: u32, i: usize) -> bool {
        let side = GPKG_TILE_SIDE as usize;
        let x = column as usize * side + i % side;
        let y = row as usize * side + i / side;
        x < l.width as usize && y < l.height as usize
    }

    /// A coverage tile's values' least, greatest, mean and (population) standard deviation; none without values.
    fn tile_stats(values: impl Iterator<Item = f64>) -> TileStats {
        let v: Vec<f64> = values.collect();
        if v.is_empty() {
            return TileStats::default();
        }
        let n = v.len() as f64;
        let mean = v.iter().sum::<f64>() / n;
        let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / n;
        TileStats {
            min: v.iter().copied().reduce(f64::min),
            max: v.iter().copied().reduce(f64::max),
            mean: Some(mean),
            std_dev: Some(var.sqrt()),
        }
    }

    /// A strip of `width` samples a row, 256 rows, every sample `fill`.
    fn blank(kind: Kind, width: usize, fill: f64) -> Buf {
        let n = width * GPKG_TILE_SIDE as usize * kind.bands();
        match kind {
            Kind::Picture { .. } => Buf::U8(vec![0; n]),
            Kind::Integer { .. } => Buf::U16(vec![fill as u16; n]),
            Kind::Float { .. } => Buf::F32(vec![fill as f32; n]),
        }
    }

    /// A finer strip (`fw` samples a row; level image `image`; its first row `base_row` of the level) halved into the
    /// coarser strip `to` (`tw` a row) from its row `at`, 128 rows. Pictures: `(a + b + c + d + 2) / 4`, samples past
    /// the finer canvas 0. A coverage: the mean of the children in the finer image and not null (an integer one
    /// rounded half up), `fill` (its null, or 0) where none is.
    #[allow(clippy::too_many_arguments)]
    fn halve_into(
        kind: Kind,
        from: &Buf,
        fw: usize,
        image: (usize, usize),
        base_row: usize,
        to: &mut Buf,
        tw: usize,
        at: usize,
        fill: f64,
    ) {
        let half = GPKG_TILE_SIDE as usize / 2;
        match (from, to) {
            (Buf::U8(f), Buf::U8(t)) => {
                let b = kind.bands();
                for v in 0..half {
                    for u in 0..tw {
                        for k in 0..b {
                            let get = |x: usize, y: usize| {
                                if x < fw {
                                    u32::from(f[(y * fw + x) * b + k])
                                } else {
                                    0
                                }
                            };
                            let sum = get(2 * u, 2 * v)
                                + get(2 * u + 1, 2 * v)
                                + get(2 * u, 2 * v + 1)
                                + get(2 * u + 1, 2 * v + 1);
                            t[((at + v) * tw + u) * b + k] = ((sum + 2) / 4) as u8;
                        }
                    }
                }
            }
            (Buf::U16(f), Buf::U16(t)) => {
                let Kind::Integer { null, .. } = kind else {
                    return;
                };
                for v in 0..half {
                    for u in 0..tw {
                        let (mut sum, mut n) = (0u64, 0u64);
                        for (x, y) in [
                            (2 * u, 2 * v),
                            (2 * u + 1, 2 * v),
                            (2 * u, 2 * v + 1),
                            (2 * u + 1, 2 * v + 1),
                        ] {
                            if x < image.0 && base_row + y < image.1 {
                                let s = f[y * fw + x];
                                if Some(s) != null {
                                    sum += u64::from(s);
                                    n += 1;
                                }
                            }
                        }
                        t[(at + v) * tw + u] = if n == 0 {
                            fill as u16
                        } else {
                            ((2 * sum + n) / (2 * n)) as u16
                        };
                    }
                }
            }
            (Buf::F32(f), Buf::F32(t)) => {
                let Kind::Float { null } = kind else {
                    return;
                };
                for v in 0..half {
                    for u in 0..tw {
                        let (mut sum, mut n, mut nan) = (0f64, 0u32, false);
                        for (x, y) in [
                            (2 * u, 2 * v),
                            (2 * u + 1, 2 * v),
                            (2 * u, 2 * v + 1),
                            (2 * u + 1, 2 * v + 1),
                        ] {
                            if x < image.0 && base_row + y < image.1 {
                                let s = f[y * fw + x];
                                if s.is_nan() {
                                    nan = true;
                                } else if Some(s) != null {
                                    sum += f64::from(s);
                                    n += 1;
                                }
                            }
                        }
                        t[(at + v) * tw + u] = if n > 0 {
                            (sum / f64::from(n)) as f32
                        } else if nan && null.is_none() {
                            f32::NAN
                        } else {
                            fill as f32
                        };
                    }
                }
            }
            _ => {}
        }
    }

    /// Level 0's strip of tile row `row` (`rows` of the raster's rows from `row · 256`): the raster's samples in the
    /// tile bands, the grid's padding 0 (a coverage's: its null, or 0).
    fn read_strip(
        p: &mut Prepared,
        row: u32,
        rows: u32,
        rounded: &mut bool,
    ) -> Result<Buf, String> {
        let width = p.raster.width;
        let l = p.levels[0];
        let side = GPKG_TILE_SIDE as usize;
        let cw = l.tiles_x as usize * side;
        let y0 = i64::from(row * GPKG_TILE_SIDE);
        let region = p
            .source
            .region(0, y0, width, rows)
            .map_err(|e| format!("Rasterin kaynağı okunamadı: {e}"))?
            .ok_or("Rasterin pikselleri okunamadı.")?;
        let fill = match p.kind {
            Kind::Picture { .. } => 0.0,
            Kind::Integer { offset, null } => null.map_or(f64::from(-offset), f64::from),
            Kind::Float { null } => null.map_or(0.0, f64::from),
        };
        let mut out = blank(p.kind, cw, fill);
        let sb = p.source_bands;
        let nodata = p.nodata;
        match (&mut out, p.kind) {
            (Buf::U8(o), Kind::Picture { bands: nb, .. }) => {
                let Samples::U8(src) = &region.samples else {
                    return Err("Raster karo olarak yazılamaz: 8 bitlik değil.".into());
                };
                for y in 0..rows as usize {
                    for x in 0..width as usize {
                        let s = (y * width as usize + x) * sb;
                        let d = (y * cw + x) * nb;
                        o[d..d + sb].copy_from_slice(&src[s..s + sb]);
                        if nb == sb + 1 {
                            o[d + sb] = 255;
                        }
                    }
                }
            }
            (Buf::U16(o), Kind::Integer { offset, null }) => {
                for y in 0..rows as usize {
                    for x in 0..width as usize {
                        let v = region.samples.get(y * width as usize + x);
                        o[y * cw + x] = match (nodata, null) {
                            (Some(nd), Some(n)) if v == nd => n,
                            _ => (v as i64 - i64::from(offset)).clamp(0, 65535) as u16,
                        };
                    }
                }
            }
            (Buf::F32(o), Kind::Float { null }) => {
                for y in 0..rows as usize {
                    for x in 0..width as usize {
                        let v = region.samples.get(y * width as usize + x);
                        o[y * cw + x] = match (nodata, null) {
                            (Some(nd), Some(n)) if v == nd => n,
                            _ => {
                                let s = v as f32;
                                if v.is_finite() && f64::from(s) != v {
                                    *rounded = true;
                                }
                                s
                            }
                        };
                    }
                }
            }
            _ => return Err("karonun türü uymuyor".into()),
        }
        Ok(out)
    }

    /// The file made whole beside its place, read back, then put there (mode `new`); or the table added in one
    /// transaction (mode `add`). Its size, and whether a coverage's value was rounded.
    fn write_file(p: &mut Prepared, overwrite: bool) -> Result<(u64, bool), String> {
        let (_, srs) = srs_for(p.srid);
        let levels = p.levels.clone();
        let zooms: Vec<i64> = p.set.matrices.iter().rev().map(|m| m.zoom).collect();
        let kind = p.kind;
        let expected: u64 = levels
            .iter()
            .map(|l| u64::from(l.tiles_x) * u64::from(l.tiles_y))
            .sum();
        let mut set = p.set.clone();
        set.identifier = p.identifier.clone();
        let table = p.table.clone();
        let target = p.path.clone();
        let mode = p.mode;
        let mut rounded = false;
        let mut read = |row: u32, rows: u32| read_strip(p, row, rows, &mut rounded);
        let mut pyramid = Pyramid::new(levels, zooms, kind, &mut read);
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
        let size = match mode {
            GpkgTilesMode::New => {
                let tmp = beside(&target);
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
                    if target.exists() && !overwrite {
                        return Err(format!("{} var", target.display()));
                    }
                    let size = std::fs::metadata(&tmp).map_err(|e| e.to_string())?.len();
                    std::fs::rename(&tmp, &target)
                        .map_err(|e| format!("{} yerine konamadı: {e}", target.display()))?;
                    Ok(size)
                })();
                if done.is_err() {
                    let _ = std::fs::remove_file(&tmp);
                    let _ = std::fs::remove_file(format!("{}-journal", tmp.display()));
                }
                done?
            }
            GpkgTilesMode::Add => {
                let mut gp = GeoPackage::open(&target, true).map_err(|e| e.0)?;
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
                verify(&target)?;
                std::fs::metadata(&target).map_err(|e| e.to_string())?.len()
            }
        };
        drop(pyramid);
        Ok((size, rounded))
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
            Ok((bytes, rounded)) => {
                let out = answer(&p, cx.doc.revision().to_string(), bytes);
                let mut warnings = p.warnings;
                if rounded {
                    warnings.push(CommandWarning {
                        code: codes::PRECISION_LOST.into(),
                        message: "Rasterin bazı değerleri 32 bitlik kayan noktalı karoya yuvarlanarak yazıldı (karolar 32 bitlik TIFF'tir).".into(),
                        path: Some("uid".into()),
                    });
                }
                CommandResult::Completed {
                    output: out,
                    warnings,
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
    fn the_strip_pyramid_makes_the_whole_canvas_pyramid_s_tiles() {
        // A grey picture of 700 × 600: three levels (700 × 600, 350 × 300, 175 × 150); its pixels (x · 7 + y · 3) % 256.
        let (w, h) = (700u32, 600u32);
        let lv = levels(w, h);
        let px = |x: usize, y: usize| ((x * 7 + y * 3) % 256) as u8;
        let side = GPKG_TILE_SIDE as usize;
        // The whole canvas pyramid, as ADR 0556 makes it.
        let mut canvas = vec![0u8; lv[0].tiles_x as usize * side * lv[0].tiles_y as usize * side];
        let cw0 = lv[0].tiles_x as usize * side;
        for y in 0..h as usize {
            for x in 0..w as usize {
                canvas[y * cw0 + x] = px(x, y);
            }
        }
        let mut whole = vec![canvas.clone()];
        for k in 1..lv.len() {
            let (f, t) = (lv[k - 1], lv[k]);
            let next = halve(
                &whole[k - 1],
                (f.tiles_x as usize * side, f.tiles_y as usize * side),
                (t.tiles_x as usize * side, t.tiles_y as usize * side),
                1,
            );
            whole.push(next);
        }
        let mut read = |row: u32, rows: u32| -> Result<write::Buf, String> {
            let mut s = vec![0u8; cw0 * side];
            for y in 0..rows as usize {
                for x in 0..w as usize {
                    s[y * cw0 + x] = px(x, row as usize * side + y);
                }
            }
            Ok(write::Buf::U8(s))
        };
        let kind = write::Kind::Picture {
            bands: 1,
            webp: false,
        };
        let zooms = vec![2, 1, 0];
        let mut p = write::Pyramid::new(lv.clone(), zooms, kind, &mut read);
        let mut n = 0;
        while let Some(t) = p.next().unwrap() {
            n += 1;
            let k = (2 - t.zoom) as usize;
            let cw = lv[k].tiles_x as usize * side;
            let decoded = png::decode(&t.data).unwrap();
            let Samples::U8(v) = decoded.samples else {
                panic!()
            };
            for y in 0..side {
                for x in 0..side {
                    let at = (t.row as usize * side + y) * cw + t.column as usize * side + x;
                    assert_eq!(
                        v[y * side + x],
                        whole[k][at],
                        "zoom {} tile {}/{}",
                        t.zoom,
                        t.column,
                        t.row
                    );
                }
            }
        }
        assert_eq!(n, lv.iter().map(|l| l.tiles_x * l.tiles_y).sum::<u32>());
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
