//! A raster's source and the samples of any region of any level
//! (docs/adr/0204 §1, §3). Level 0 is the file (a TIFF's main directory, or
//! a whole decoded PNG or JPEG); a coarser level is the file's reduced
//! directory of that size, the pyramid file's, or worked out from the level
//! below as the mean of 2 × 2 samples (nodata and NaN left out; a palette's
//! upper left sample). Blocks are decoded once and kept, least recently used
//! let go first, within a budget of bytes.
//!
//! The host asks [`Reader::needs`] which blocks a region wants, reads them
//! and hands them to [`Reader::put_block`]; a JPEG block comes back as a
//! stream for the host's decoder, whose pixels go to [`Reader::put_pixels`].

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use kentos_contracts::RasterSample;
use serde::Serialize;

use kentos_contracts::RasterStyle;

pub use super::layout::{Layout, Put2, decode_block};
use super::layout::{compression_name, layout};
use super::stats::{self, STATS_SIDE, Stats};
use super::style::{self, Look};
use super::tiff::{Ifd, Tiff};
use super::vrt::{self, Member, Vrt};
use super::{RasterError, Samples, TILE, TILE_APRON, geotiff};
use crate::math;
use crate::multidim::mesh::{Data, Mesh};
use crate::multidim::ugrid::Location;

/// A block of a level: which file and directory, and its index there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockKey {
    /// 0 the raster's file, 1 its pyramid file, 2 a level worked out here.
    pub file: u8,
    pub ifd: u16,
    pub index: u32,
}

/// A block the host is to read: where it is in which file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockNeed {
    /// 0 the raster's own files; k a mosaic's k-th file (docs/adr/0517 §2), whose
    /// `file` is then that file's (0 the file, 1 its pyramid).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub member: u32,
    pub file: u8,
    pub ifd: u16,
    pub index: u32,
    pub offset: u64,
    pub len: u64,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

/// What a handed-over block gave.
#[derive(Clone, Debug, PartialEq)]
pub enum Put {
    Done,
    /// A JPEG stream for the host's decoder; its pixels go to `put_pixels`.
    Jpeg(Vec<u8>),
}

/// A region's samples: `width` × `height` pixels from (`x`, `y`), bands interleaved.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub x: i64,
    pub y: i64,
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub samples: Samples,
}

impl Region {
    /// Band `b` of the pixel at (`i`, `j`) of the region.
    #[inline]
    pub fn at(&self, i: u32, j: u32, b: u32) -> f64 {
        self.samples.get(
            ((j as usize * self.width as usize) + i as usize) * self.bands as usize + b as usize,
        )
    }
}

/// A mesh slice's values read: the dataset's (a vector's x), its y and its face mask.
type MeshSlab = (Arc<Samples>, Option<Arc<Samples>>, Option<Arc<Samples>>);

/// What a level's samples come from. (A reader has a handful of levels,
/// each looked at for every block: its layout is kept in place, not boxed.)
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum LevelFrom {
    /// A directory's blocks.
    File(Layout),
    /// The whole decoded image (level 0 of a PNG or JPEG).
    Image,
    /// Worked out from the level below, in 256 × 256 tiles.
    Computed,
    /// A mesh's slice drawn at the level, in 256 × 256 tiles (docs/adr/0243 §5).
    Mesh(Arc<MeshLevels>),
    /// A virtual mosaic's files laid over each other at the level (docs/adr/0517 §2).
    Mosaic,
    /// A GeoPackage tile table's zoom (docs/adr/0627 §3): its tiles are the level's blocks.
    Tiles(super::gpkg::TileLevel),
}

/// The `file` of a GeoPackage tile's block: its `ifd` is the level, its `index` the tile among the level's
/// ([`Reader::tile_at`] names its zoom, column and row).
pub const TILES_FILE: u8 = 3;

/// A file of a virtual mosaic as the reader takes it (docs/adr/0517 §2): its
/// own reader, where it lies on the mosaic and which of its bands each band
/// of the mosaic is.
#[derive(Debug)]
pub struct MosaicPart {
    pub reader: Reader,
    /// The mosaic's level-0 pixel coordinate x lies at the file's sx·x + ox (y the same with sy, oy).
    sx: f64,
    ox: f64,
    sy: f64,
    oy: f64,
    /// The mosaic's level-0 rectangle it covers, `[x₀, y₀, x₁, y₁]`.
    dst: [f64; 4],
    /// The file's samples that let the files below show.
    nodata: Option<f64>,
    /// The file's band (from 0) of each band of the mosaic.
    bands: Vec<u32>,
}

impl MosaicPart {
    /// The file's level a mosaic level `level` reads: the coarsest whose pixel is no larger than the mosaic's.
    fn level_for(&self, level: usize) -> usize {
        let ratio = self.sx.min(self.sy) * f64::from(1u32 << level.min(31));
        let mut k = 0usize;
        while k + 1 < self.reader.levels.len() && f64::from(1u32 << (k + 1).min(31)) <= ratio * (1.0 + 1e-9) {
            k += 1;
        }
        k
    }

    /// The mosaic level pixels `[i₀, i₁) × [j₀, j₁)` of the clamped rectangle whose centres lie on the file
    /// (`f` the level's pixel in level-0 pixels).
    fn covered(&self, f: f64, x0: u32, y0: u32, x1: u32, y1: u32) -> Option<(u32, u32, u32, u32)> {
        let lo = |s: f64| (s / f - 0.5).ceil();
        let (fw, fh) = (f64::from(self.reader.info.width), f64::from(self.reader.info.height));
        // The file's own extent as mosaic level-0 pixels.
        let (sx0, sx1) = (-self.ox / self.sx, (fw - self.ox) / self.sx);
        let (sy0, sy1) = (-self.oy / self.sy, (fh - self.oy) / self.sy);
        let a = |v: f64, lo_: u32, hi: u32| v.clamp(f64::from(lo_), f64::from(hi)) as u32;
        let i0 = a(lo(self.dst[0].max(sx0)), x0, x1);
        let i1 = a(lo(self.dst[2].min(sx1)), x0, x1);
        let j0 = a(lo(self.dst[1].max(sy0)), y0, y1);
        let j1 = a(lo(self.dst[3].min(sy1)), y0, y1);
        (i0 < i1 && j0 < j1).then_some((i0, j0, i1, j1))
    }

    /// The file's level-`k` pixel under the mosaic's level-0 coordinate (x, y).
    fn pixel(&self, k: usize, x: f64, y: f64) -> (i64, i64) {
        let g = f64::from(1u32 << k.min(31));
        (((self.sx * x + self.ox) / g).floor() as i64, ((self.sy * y + self.oy) / g).floor() as i64)
    }

    /// The file's level-`k` rectangle the mosaic's level-`level` pixels `[i₀, i₁) × [j₀, j₁)` read.
    fn source_rect(&self, level: usize, k: usize, (i0, j0, i1, j1): (u32, u32, u32, u32)) -> (i64, i64, u32, u32) {
        let f = f64::from(1u32 << level.min(31));
        let (ax, ay) = self.pixel(k, (f64::from(i0) + 0.5) * f, (f64::from(j0) + 0.5) * f);
        let (bx, by) = self.pixel(k, (f64::from(i1) - 0.5) * f, (f64::from(j1) - 0.5) * f);
        (ax, ay, (bx - ax + 1).max(1) as u32, (by - ay + 1).max(1) as u32)
    }
}

/// A mesh's slice as a raster's levels (docs/adr/0243 §5): the mesh, where
/// its values lie, the blocks of its values (a vector's two components) and
/// of its face mask, and the sanal grid's affine.
#[derive(Debug)]
pub struct MeshLevels {
    pub mesh: Arc<Mesh>,
    pub location: Location,
    pub blocks: Vec<Layout>,
    pub mask: Option<Layout>,
    pub affine: [f64; 6],
}

impl PartialEq for MeshLevels {
    fn eq(&self, other: &MeshLevels) -> bool {
        Arc::ptr_eq(&self.mesh, &other.mesh)
            && self.blocks == other.blocks
            && self.mask == other.mask
    }
}

impl MeshLevels {
    /// Its value and mask blocks' layouts, the mask last.
    fn layouts(&self) -> impl Iterator<Item = &Layout> {
        self.blocks.iter().chain(self.mask.iter())
    }
}

/// A mesh slice's values once read: one block a component, the mask.
#[derive(Debug, Default)]
struct MeshValues {
    x: Option<Arc<Samples>>,
    y: Option<Arc<Samples>>,
    mask: Option<Arc<Samples>>,
}

/// A level of the pyramid.
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    pub width: u32,
    pub height: u32,
    pub from: LevelFrom,
}

/// The sizes of a raster's levels (the geometry core's, which the raster passes use too).
pub use kentos_geometry_core::geom::raster::level_sizes;

/// What the window shows of a raster before it is added (docs/adr/0204 §8).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RasterInfo {
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub sample: RasterSample,
    /// `gray`, `rgb`, `palette` or `ycbcr`.
    pub color: String,
    /// The 4th band (RGB) or an extra sample is alpha.
    pub alpha: bool,
    pub compression: String,
    pub tiled: bool,
    pub big: bool,
    /// Directories of reduced size the file holds (used as levels).
    pub overviews: u32,
    /// Levels from the file itself (0) to one tile.
    pub levels: u32,
    /// `[x₀, a, b, y₀, c, d]` when the file (or its world file) places it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affine: Option<[f64; 6]>,
    /// `geotiff`, `world` or `none`.
    pub placed_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsg: Option<u32>,
    pub geographic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodata: Option<f64>,
    /// Whether a pyramid file is to be made (large, without overviews).
    pub needs_pyramid: bool,
}

/// The side past which a raster without overviews gets a pyramid file (docs/adr/0204 §3).
pub const PYRAMID_FROM: u32 = 4096;
/// The first level a pyramid file holds; finer ones are worked out from level 0.
pub const PYRAMID_FIRST: usize = 3;

/// A least-recently-used store of blocks within a budget of bytes.
#[derive(Debug, Default)]
struct Lru {
    map: HashMap<BlockKey, (Samples, u64)>,
    order: BTreeMap<u64, BlockKey>,
    tick: u64,
    bytes: usize,
    budget: usize,
}

impl Lru {
    fn get(&mut self, key: &BlockKey) -> Option<&Samples> {
        self.tick += 1;
        let tick = self.tick;
        let entry = self.map.get_mut(key)?;
        self.order.remove(&entry.1);
        entry.1 = tick;
        self.order.insert(tick, *key);
        Some(&entry.0)
    }

    fn contains(&self, key: &BlockKey) -> bool {
        self.map.contains_key(key)
    }

    fn put(&mut self, key: BlockKey, samples: Samples) {
        self.tick += 1;
        let size = samples.bytes();
        if let Some((old, t)) = self.map.remove(&key) {
            self.order.remove(&t);
            self.bytes -= old.bytes();
        }
        while self.bytes + size > self.budget {
            let Some((&t, &k)) = self.order.iter().next() else {
                break;
            };
            self.order.remove(&t);
            if let Some((s, _)) = self.map.remove(&k) {
                self.bytes -= s.bytes();
            }
        }
        self.bytes += size;
        self.order.insert(self.tick, key);
        self.map.insert(key, (samples, self.tick));
    }
}

/// A raster being read.
#[derive(Debug)]
pub struct Reader {
    pub info: RasterInfo,
    pub levels: Vec<Level>,
    /// A palette's colours (16 bits a channel), when the raster has one.
    pub palette: Option<Vec<[u16; 3]>>,
    /// The file's nodata value.
    pub nodata: Option<f64>,
    image: Option<Samples>,
    blocks: Lru,
    jpeg_waiting: Vec<BlockKey>,
    /// A mesh slice's values, kept outside the blocks' budget (every tile needs them).
    mesh_values: MeshValues,
    /// A virtual mosaic's files, bottom first (docs/adr/0517 §2).
    mosaic: Vec<MosaicPart>,
    /// A GeoPackage tile table's content (docs/adr/0627 §3), when the raster is one.
    tiles: Option<super::gpkg::Content>,
}

/// The level-0 block of a TIFF directory's palette, by its colour map (r…, g…, b…).
fn palette_of(ifd: &Ifd) -> Option<Vec<[u16; 3]>> {
    let n = ifd.color_map.len() / 3;
    (ifd.photometric == Some(3) && n > 0).then(|| {
        (0..n)
            .map(|i| {
                let c = |k: usize| u16::try_from(ifd.color_map[k * n + i]).unwrap_or(u16::MAX);
                [c(0), c(1), c(2)]
            })
            .collect()
    })
}

impl Reader {
    /// A TIFF's reader: its main directory, its overviews as levels; `world`
    /// places it when the file does not. `budget` bounds the kept blocks.
    pub fn tiff(
        tiff: &Tiff,
        world: Option<[f64; 6]>,
        budget: usize,
    ) -> Result<Reader, RasterError> {
        // The main image: the first directory that is neither reduced (bit 0) nor a mask (bit 2).
        let main = tiff
            .ifds
            .iter()
            .position(|d| d.subfile & 0b101 == 0)
            .ok_or_else(|| {
                RasterError::new("TIFF'te ana görüntü yok (yalnız önizleme ya da maske).")
            })?;
        let ifd = &tiff.ifds[main];
        let base = layout(0, main, ifd, tiff.little)?;
        let geo = geotiff::read(ifd);
        let sizes = level_sizes(base.width, base.height);
        let mut levels = vec![Level {
            width: base.width,
            height: base.height,
            from: LevelFrom::File(base.clone()),
        }];
        let mut overviews = 0;
        for &(w, h) in sizes.iter().skip(1) {
            let found = tiff.ifds.iter().enumerate().find(|(_, d)| {
                d.subfile & 0b101 == 0b001
                    && d.width == u64::from(w)
                    && d.height == u64::from(h)
                    && d.samples == ifd.samples
            });
            let from = match found.map(|(k, d)| layout(0, k, d, tiff.little)) {
                Some(Ok(l)) if l.sample == base.sample => {
                    overviews += 1;
                    LevelFrom::File(l)
                }
                _ => LevelFrom::Computed,
            };
            levels.push(Level {
                width: w,
                height: h,
                from,
            });
        }
        let photometric = ifd.photometric.unwrap_or(1);
        let color = match photometric {
            2 => "rgb",
            3 => "palette",
            6 => "ycbcr",
            _ => "gray",
        };
        // ExtraSamples: 1 associated and 2 unassociated alpha; 0 is data
        // (GDAL's extra bands of a multi-band raster: a satellite image's
        // near infrared is no mask). Without the tag an RGB raster's fourth
        // sample is alpha, as writers that leave it out mean.
        let alpha = match ifd.extra.last() {
            Some(&e) => e == 1 || e == 2,
            None => photometric == 2 && base.bands >= 4,
        };
        let (affine, placed_by) = match (geo.affine, world) {
            (Some(a), _) => (Some(a), "geotiff"),
            (None, Some(w)) => (Some(w), "world"),
            _ => (None, "none"),
        };
        let needs_pyramid = (base.width > PYRAMID_FROM || base.height > PYRAMID_FROM)
            && levels
                .iter()
                .skip(PYRAMID_FIRST)
                .any(|l| l.from == LevelFrom::Computed);
        let info = RasterInfo {
            width: base.width,
            height: base.height,
            bands: base.bands,
            sample: base.sample,
            color: color.to_owned(),
            alpha,
            compression: compression_name(base.compression).unwrap_or("").to_owned(),
            tiled: ifd.tile_width.is_some(),
            big: tiff.big,
            overviews,
            levels: levels.len() as u32,
            affine,
            placed_by: placed_by.to_owned(),
            epsg: geo.epsg,
            geographic: geo.geographic,
            nodata: geo.nodata,
            needs_pyramid,
        };
        Ok(Reader {
            info,
            levels,
            palette: palette_of(ifd),
            nodata: geo.nodata,
            image: None,
            blocks: Lru {
                budget,
                ..Lru::default()
            },
            jpeg_waiting: Vec::new(),
            mesh_values: MeshValues::default(),
            mosaic: Vec::new(),
            tiles: None,
        })
    }

    /// A NetCDF grid's reader (docs/adr/0243 §5): level 0 its rows of a
    /// slice, the coarser worked out (or a pyramid file's past 4096).
    pub fn grid(
        mut info: RasterInfo,
        layout: Layout,
        budget: usize,
    ) -> Result<Reader, RasterError> {
        let (w, h) = (layout.width, layout.height);
        let mut levels = vec![Level {
            width: w,
            height: h,
            from: LevelFrom::File(layout),
        }];
        for &(lw, lh) in level_sizes(w, h).iter().skip(1) {
            levels.push(Level {
                width: lw,
                height: lh,
                from: LevelFrom::Computed,
            });
        }
        info.levels = levels.len() as u32;
        info.needs_pyramid = (w > PYRAMID_FROM || h > PYRAMID_FROM) && levels.len() > PYRAMID_FIRST;
        let nodata = info.nodata;
        Ok(Reader {
            info,
            levels,
            palette: None,
            nodata,
            image: None,
            blocks: Lru {
                budget,
                ..Lru::default()
            },
            jpeg_waiting: Vec::new(),
            mesh_values: MeshValues::default(),
            mosaic: Vec::new(),
            tiles: None,
        })
    }

    /// A mesh slice's reader (docs/adr/0243 §5): every level drawn from the mesh.
    pub fn mesh(
        mut info: RasterInfo,
        mesh: MeshLevels,
        budget: usize,
    ) -> Result<Reader, RasterError> {
        let (w, h) = (info.width, info.height);
        if w == 0 || h == 0 {
            return Err(RasterError::new("Mesh'in ızgarası boş."));
        }
        let m = Arc::new(mesh);
        let levels: Vec<Level> = level_sizes(w, h)
            .into_iter()
            .map(|(lw, lh)| Level {
                width: lw,
                height: lh,
                from: LevelFrom::Mesh(m.clone()),
            })
            .collect();
        info.levels = levels.len() as u32;
        info.needs_pyramid = false;
        Ok(Reader {
            info,
            levels,
            palette: None,
            nodata: None,
            image: None,
            blocks: Lru {
                budget,
                ..Lru::default()
            },
            jpeg_waiting: Vec::new(),
            mesh_values: MeshValues::default(),
            mosaic: Vec::new(),
            tiles: None,
        })
    }

    /// The mesh a mesh slice's reader draws.
    pub fn mesh_levels(&self) -> Option<&Arc<MeshLevels>> {
        self.levels.first().and_then(|l| match &l.from {
            LevelFrom::Mesh(m) => Some(m),
            _ => None,
        })
    }

    /// A mesh slice's values, once read.
    fn mesh_data(&self, m: &MeshLevels) -> Option<MeshSlab> {
        let x = self.mesh_values.x.clone()?;
        let y = match m.blocks.len() > 1 {
            true => Some(self.mesh_values.y.clone()?),
            false => None,
        };
        let mask = match m.mask.is_some() {
            true => Some(self.mesh_values.mask.clone()?),
            false => None,
        };
        Some((x, y, mask))
    }

    /// The value of a mesh slice at a point of the drawing (interpolated on
    /// the mesh, not a pixel's; docs/adr/0243 §8); none while its values are not read.
    pub fn mesh_value(&self, x: f64, y: f64) -> Option<f64> {
        let m = self.mesh_levels()?;
        let (vx, vy, mask) = self.mesh_data(m)?;
        let data = Data {
            location: m.location,
            x: &vx,
            y: vy.as_deref(),
            mask: mask.as_deref(),
        };
        Some(m.mesh.value_at(&data, x, y))
    }

    /// A whole decoded image's reader (a PNG or JPEG), placed by `world`.
    pub fn image(
        width: u32,
        height: u32,
        bands: u32,
        samples: Samples,
        nodata: Option<f64>,
        world: Option<[f64; 6]>,
        budget: usize,
    ) -> Result<Reader, RasterError> {
        if samples.len() != width as usize * height as usize * bands as usize {
            return Err(RasterError::new("Resmin pikselleri boyuna uymuyor."));
        }
        let sample = samples.kind();
        let mut levels = vec![Level {
            width,
            height,
            from: LevelFrom::Image,
        }];
        for &(w, h) in level_sizes(width, height).iter().skip(1) {
            levels.push(Level {
                width: w,
                height: h,
                from: LevelFrom::Computed,
            });
        }
        let info = RasterInfo {
            width,
            height,
            bands,
            sample,
            color: if bands >= 3 { "rgb" } else { "gray" }.to_owned(),
            alpha: bands == 4 || bands == 2,
            compression: String::new(),
            tiled: false,
            big: false,
            overviews: 0,
            levels: levels.len() as u32,
            affine: world,
            placed_by: if world.is_some() { "world" } else { "none" }.to_owned(),
            epsg: None,
            geographic: false,
            nodata,
            needs_pyramid: false,
        };
        Ok(Reader {
            info,
            levels,
            palette: None,
            nodata,
            image: Some(samples),
            blocks: Lru {
                budget,
                ..Lru::default()
            },
            jpeg_waiting: Vec::new(),
            mesh_values: MeshValues::default(),
            mosaic: Vec::new(),
            tiles: None,
        })
    }

    /// A virtual mosaic's reader (docs/adr/0517 §2): `vrt`'s files, each with
    /// its own reader the host opened, bottom first as [`vrt::members`] gives
    /// them; every level is the files laid over each other at that level, so
    /// a coarse view reads each file's own overviews. Why not when a file
    /// does not fit the VRT (another data type, a band it has not, a mosaic).
    pub fn mosaic(vrt: &Vrt, files: Vec<(Member, Reader)>, budget: usize) -> Result<Reader, RasterError> {
        let bands = u32::try_from(vrt.bands.len()).unwrap_or(u32::MAX);
        let sample = vrt.bands[0].sample;
        let mut parts = Vec::with_capacity(files.len());
        for (m, r) in files {
            let name = &m.file;
            if !r.mosaic.is_empty() {
                return Err(RasterError::new(format!("“{name}” kendisi bir mozaik; mozaiğin içinde mozaik açılmaz.")));
            }
            if r.mesh_levels().is_some() {
                return Err(RasterError::new(format!("“{name}” bir mesh; mozaiğe konmaz.")));
            }
            if r.info.sample != sample {
                return Err(RasterError::new(format!(
                    "“{name}” {} örnekli; mozaik {} (VRT'nin veri türü dosyanınkine uymuyor).",
                    vrt::type_name(r.info.sample),
                    vrt::type_name(sample)
                )));
            }
            if let Some(&b) = m.bands.iter().find(|&&b| b == 0 || b > r.info.bands) {
                return Err(RasterError::new(format!("“{name}” dosyasında {b}. band yok.")));
            }
            let sx = m.src[2] / m.dst[2];
            let sy = m.src[3] / m.dst[3];
            parts.push(MosaicPart {
                sx,
                ox: m.src[0] - m.dst[0] * sx,
                sy,
                oy: m.src[1] - m.dst[1] * sy,
                dst: [m.dst[0], m.dst[1], m.dst[0] + m.dst[2], m.dst[1] + m.dst[3]],
                nodata: m.nodata,
                bands: m.bands.iter().map(|b| b - 1).collect(),
                reader: r,
            });
        }
        let first = parts
            .first()
            .ok_or_else(|| RasterError::new("Mozaikte dosya yok."))?;
        let levels: Vec<Level> = level_sizes(vrt.width, vrt.height)
            .into_iter()
            .map(|(width, height)| Level { width, height, from: LevelFrom::Mosaic })
            .collect();
        let palette = (bands == 1).then(|| first.reader.palette.clone()).flatten();
        let color = match (bands, &palette) {
            (1, Some(_)) => "palette",
            (b, _) if b >= 3 => "rgb",
            _ => "gray",
        };
        let alpha = vrt.bands.len() > 1
            && vrt.bands.last().and_then(|b| b.color.as_deref()).is_some_and(|c| c.eq_ignore_ascii_case("alpha"));
        let geographic = first.reader.info.geographic && first.reader.info.epsg == vrt.epsg;
        let nodata = vrt.bands[0].nodata;
        let info = RasterInfo {
            width: vrt.width,
            height: vrt.height,
            bands,
            sample,
            color: color.to_owned(),
            alpha,
            compression: "VRT".to_owned(),
            tiled: true,
            big: false,
            overviews: 0,
            levels: levels.len() as u32,
            affine: vrt.affine,
            placed_by: if vrt.affine.is_some() { "vrt" } else { "none" }.to_owned(),
            epsg: vrt.epsg,
            geographic,
            nodata,
            needs_pyramid: false,
        };
        Ok(Reader {
            info,
            levels,
            palette,
            nodata,
            image: None,
            blocks: Lru {
                budget,
                ..Lru::default()
            },
            jpeg_waiting: Vec::new(),
            mesh_values: MeshValues::default(),
            mosaic: parts,
            tiles: None,
        })
    }

    /// A GeoPackage tile table's reader (docs/adr/0627 §3): level 0 is `width` × `height` pixels of `levels[0]`'s
    /// zoom, each coarser level its zoom when `levels` names one, else worked out from the level below; the samples
    /// are the table's `content` (pictures as RGBA bytes, a coverage as one band). The host reads a block's tile by
    /// [`Reader::tile_at`] and hands it over decoded ([`Reader::keep_tile`]).
    #[allow(clippy::too_many_arguments)]
    pub fn tiles(
        content: super::gpkg::Content,
        width: u32,
        height: u32,
        levels: &[Option<super::gpkg::TileLevel>],
        affine: [f64; 6],
        epsg: Option<u32>,
        geographic: bool,
        budget: usize,
    ) -> Result<Reader, RasterError> {
        let sizes = level_sizes(width, height);
        if width == 0 || height == 0 || !matches!(levels.first(), Some(Some(_))) {
            return Err(RasterError::new("Karo tablosunun okunacak düzeyi yok."));
        }
        let mut out = Vec::with_capacity(sizes.len());
        let mut tiled = 0;
        for (k, &(w, h)) in sizes.iter().enumerate() {
            let from = match levels.get(k).copied().flatten() {
                Some(t) => {
                    let (_, _, across, down) = t.span(w, h);
                    if across.saturating_mul(down) > i64::from(u32::MAX) {
                        return Err(RasterError::new(
                            "Karo tablosunun penceresi çok büyük: bir düzeyinde 4 milyardan çok karo olurdu.",
                        ));
                    }
                    tiled += 1;
                    LevelFrom::Tiles(t)
                }
                None => LevelFrom::Computed,
            };
            out.push(Level {
                width: w,
                height: h,
                from,
            });
        }
        let nodata = content.nodata();
        // A large table without the coarse zooms (GDAL's file written without overviews): the host's pyramid file
        // makes them once, as a TIFF's (docs/adr/0204 §3).
        let needs_pyramid = (width > PYRAMID_FROM || height > PYRAMID_FROM)
            && out
                .iter()
                .skip(PYRAMID_FIRST)
                .any(|l| l.from == LevelFrom::Computed);
        let info = RasterInfo {
            width,
            height,
            bands: content.bands(),
            sample: content.sample(),
            color: if content.bands() == 4 { "rgb" } else { "gray" }.to_owned(),
            alpha: content.bands() == 4,
            compression: "gpkg".to_owned(),
            tiled: true,
            big: false,
            overviews: tiled - 1,
            levels: out.len() as u32,
            affine: Some(affine),
            placed_by: "gpkg".to_owned(),
            epsg,
            geographic,
            nodata,
            needs_pyramid,
        };
        Ok(Reader {
            info,
            levels: out,
            palette: None,
            nodata,
            image: None,
            blocks: Lru {
                budget,
                ..Lru::default()
            },
            jpeg_waiting: Vec::new(),
            mesh_values: MeshValues::default(),
            mosaic: Vec::new(),
            tiles: Some(content),
        })
    }

    /// A tile table's content, when the raster is one.
    pub fn tile_content(&self) -> Option<super::gpkg::Content> {
        self.tiles
    }

    /// What a host needs to read and decode the tile table's blocks off the reader; none for another raster.
    pub fn tile_blocks(&self) -> Option<super::gpkg::TileBlocks> {
        Some(super::gpkg::TileBlocks {
            content: self.tiles?,
            levels: self
                .levels
                .iter()
                .map(|lv| match &lv.from {
                    LevelFrom::Tiles(t) => Some((*t, lv.width, lv.height)),
                    _ => None,
                })
                .collect(),
        })
    }

    /// A tile block's zoom, column and row in its table, and the tile's width and height (docs/adr/0627 §3).
    pub fn tile_at(&self, need: &BlockNeed) -> Option<(i64, i64, i64, u32, u32)> {
        if need.file != TILES_FILE || need.member != 0 {
            return None;
        }
        let lv = self.levels.get(usize::from(need.ifd))?;
        let LevelFrom::Tiles(t) = &lv.from else {
            return None;
        };
        let (c0, r0, across, _) = t.span(lv.width, lv.height);
        let i = i64::from(need.index);
        Some((
            t.zoom.zoom,
            c0 + i % across,
            r0 + i / across,
            t.zoom.tile_width,
            t.zoom.tile_height,
        ))
    }

    /// Keeps a tile block decoded off the reader (`gpkg::decode`; a tile the table does not hold `gpkg::missing`):
    /// its samples, or its JPEG waiting for the host's pixels ([`Reader::put_pixels`]).
    pub fn keep_tile(&mut self, need: &BlockNeed, decoded: super::gpkg::Decoded) -> Put {
        let key = BlockKey {
            file: need.file,
            ifd: need.ifd,
            index: need.index,
        };
        match decoded {
            super::gpkg::Decoded::Samples(s) => {
                self.blocks.put(key, s);
                Put::Done
            }
            super::gpkg::Decoded::Jpeg(stream) => {
                self.jpeg_waiting.push(key);
                Put::Jpeg(stream)
            }
        }
    }

    /// A mosaic's files' count (0 for any other raster).
    pub fn mosaic_files(&self) -> usize {
        self.mosaic.len()
    }

    /// Takes a pyramid file's levels (docs/adr/0204 §3): each of its
    /// directories whose size is a level's stands for that level.
    pub fn attach_pyramid(&mut self, tiff: &Tiff) -> Result<(), RasterError> {
        for (k, d) in tiff.ifds.iter().enumerate() {
            let l = layout(1, k, d, tiff.little)?;
            if l.bands != self.info.bands || l.sample != self.info.sample {
                return Err(RasterError::new("Önizleme piramidi bu rastere ait değil."));
            }
            if let Some(level) = self.levels.iter_mut().skip(1).find(|lv| {
                lv.width == l.width && lv.height == l.height && lv.from == LevelFrom::Computed
            }) {
                level.from = LevelFrom::File(l);
            }
        }
        self.info.needs_pyramid = false;
        Ok(())
    }

    /// The blocks a region of `level` (pixels `x`…`x + w`, `y`…`y + h`,
    /// clamped to the level) wants that are not yet kept, each once.
    pub fn needs(&mut self, level: usize, x: i64, y: i64, w: u32, h: u32) -> Vec<BlockNeed> {
        let mut out = Vec::new();
        self.collect_needs(level, x, y, w, h, &mut out);
        out.sort_by_key(|n| (n.member, n.file, n.ifd, n.index));
        out.dedup();
        out
    }

    fn collect_needs(
        &mut self,
        level: usize,
        x: i64,
        y: i64,
        w: u32,
        h: u32,
        out: &mut Vec<BlockNeed>,
    ) {
        let Some(lv) = self.levels.get(level) else {
            return;
        };
        let (x0, y0, x1, y1) = clamp_rect(lv.width, lv.height, x, y, w, h);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        match &lv.from {
            LevelFrom::Image => {}
            LevelFrom::Mosaic => {
                let f = f64::from(1u32 << level.min(31));
                for (m, part) in self.mosaic.iter_mut().enumerate() {
                    let Some(cov) = part.covered(f, x0, y0, x1, y1) else {
                        continue;
                    };
                    let k = part.level_for(level);
                    let (sx, sy, sw, sh) = part.source_rect(level, k, cov);
                    for mut n in part.reader.needs(k, sx, sy, sw, sh) {
                        n.member = m as u32 + 1;
                        out.push(n);
                    }
                }
            }
            LevelFrom::File(l) => {
                let l = l.clone();
                for by in y0 / l.block_h..=(y1 - 1) / l.block_h {
                    for bx in x0 / l.block_w..=(x1 - 1) / l.block_w {
                        for band in 0..if l.planar { l.bands } else { 1 } {
                            let index = l.index(bx, by, band);
                            let key = BlockKey {
                                file: l.file,
                                ifd: l.ifd,
                                index,
                            };
                            if !self.blocks.contains(&key) && !self.jpeg_waiting.contains(&key) {
                                let i = index as usize;
                                out.push(BlockNeed {
                                    member: 0,
                                    file: l.file,
                                    ifd: l.ifd,
                                    index,
                                    offset: l.offsets.get(i).copied().unwrap_or(0),
                                    len: l.counts.get(i).copied().unwrap_or(0),
                                });
                            }
                        }
                    }
                }
            }
            LevelFrom::Tiles(t) => {
                let t = *t;
                let (c0, r0, across, _) = t.span(lv.width, lv.height);
                let (tw, th) = (i64::from(t.zoom.tile_width), i64::from(t.zoom.tile_height));
                let ca = (t.x + i64::from(x0)).div_euclid(tw);
                let cb = (t.x + i64::from(x1) - 1).div_euclid(tw);
                let ra = (t.y + i64::from(y0)).div_euclid(th);
                let rb = (t.y + i64::from(y1) - 1).div_euclid(th);
                for r in ra..=rb {
                    for c in ca..=cb {
                        let index = ((r - r0) * across + (c - c0)) as u32;
                        let key = BlockKey {
                            file: TILES_FILE,
                            ifd: u16::try_from(level).unwrap_or(u16::MAX),
                            index,
                        };
                        if !self.blocks.contains(&key) && !self.jpeg_waiting.contains(&key) {
                            out.push(BlockNeed {
                                member: 0,
                                file: TILES_FILE,
                                ifd: key.ifd,
                                index,
                                offset: 0,
                                len: 0,
                            });
                        }
                    }
                }
            }
            LevelFrom::Mesh(m) => {
                let m = m.clone();
                for l in m.layouts() {
                    let slot = match l.ifd {
                        100 => &self.mesh_values.x,
                        101 => &self.mesh_values.y,
                        _ => &self.mesh_values.mask,
                    };
                    if slot.is_none() {
                        out.push(BlockNeed {
                            member: 0,
                            file: l.file,
                            ifd: l.ifd,
                            index: 0,
                            offset: l.offsets[0],
                            len: l.counts[0],
                        });
                    }
                }
            }
            LevelFrom::Computed => {
                // Its tiles that are not kept need the level below's region.
                for ty in y0 / TILE..=(y1 - 1) / TILE {
                    for tx in x0 / TILE..=(x1 - 1) / TILE {
                        let key = computed_key(level, tx, ty);
                        if !self.blocks.contains(&key) {
                            self.collect_needs(
                                level - 1,
                                i64::from(tx * TILE * 2),
                                i64::from(ty * TILE * 2),
                                TILE * 2,
                                TILE * 2,
                                out,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Takes a block's bytes: decoded and kept, or its JPEG stream for the host. A tile table's block is its framed
    /// tile (`gpkg::frame`).
    pub fn put_block(&mut self, need: &BlockNeed, bytes: &[u8]) -> Result<Put, RasterError> {
        if let Some((part, inner)) = self.part_of(need) {
            return part.reader.put_block(&inner, bytes);
        }
        if need.file == TILES_FILE {
            let decoded = self
                .tile_blocks()
                .ok_or_else(|| RasterError::new("Bu raster bir karo tablosu değil."))?
                .decode(need, bytes)?;
            return Ok(self.keep_tile(need, decoded));
        }
        let Some(l) = self.layout_of(need.file, need.ifd) else {
            return Err(RasterError::new("Bu raster'in böyle bir bloğu yok."));
        };
        let key = BlockKey {
            file: need.file,
            ifd: need.ifd,
            index: need.index,
        };
        match decode_block(&l, need.index, bytes)? {
            Put2::Samples(s) if self.keep_mesh(need.ifd, &s) => Ok(Put::Done),
            Put2::Samples(s) => {
                self.blocks.put(key, s);
                Ok(Put::Done)
            }
            Put2::Jpeg(stream) => {
                self.jpeg_waiting.push(key);
                Ok(Put::Jpeg(stream))
            }
        }
    }

    /// The layout a block's bytes decode by, so that a host's threads decode
    /// off the reader (`layout::decode_block`) and hand the result to [`Reader::keep`].
    pub fn layout_for(&self, need: &BlockNeed) -> Option<Layout> {
        if need.member > 0 {
            let part = self.mosaic.get(need.member as usize - 1)?;
            return part.reader.layout_for(&BlockNeed { member: 0, ..*need });
        }
        self.layout_of(need.file, need.ifd)
    }

    /// Keeps a block decoded off the reader: its samples, or its JPEG stream
    /// waiting for the host's pixels ([`Reader::put_pixels`]).
    pub fn keep(&mut self, need: &BlockNeed, decoded: Put2) -> Put {
        if let Some((part, inner)) = self.part_of(need) {
            return part.reader.keep(&inner, decoded);
        }
        let key = BlockKey {
            file: need.file,
            ifd: need.ifd,
            index: need.index,
        };
        match decoded {
            Put2::Samples(s) if self.keep_mesh(need.ifd, &s) => Put::Done,
            Put2::Samples(s) => {
                self.blocks.put(key, s);
                Put::Done
            }
            Put2::Jpeg(stream) => {
                self.jpeg_waiting.push(key);
                Put::Jpeg(stream)
            }
        }
    }

    /// Takes a JPEG block's decoded pixels (`components` a pixel, 8 bits each).
    pub fn put_pixels(
        &mut self,
        need: &BlockNeed,
        pixels: Vec<u8>,
        components: u32,
    ) -> Result<(), RasterError> {
        if let Some((part, inner)) = self.part_of(need) {
            return part.reader.put_pixels(&inner, pixels, components);
        }
        let key = BlockKey {
            file: need.file,
            ifd: need.ifd,
            index: need.index,
        };
        self.jpeg_waiting.retain(|k| *k != key);
        if let Some((_, _, _, tw, th)) = self.tile_at(need) {
            let samples = super::gpkg::from_jpeg(&pixels, components, tw, th)?;
            self.blocks.put(key, samples);
            return Ok(());
        }
        let Some(l) = self.layout_of(need.file, need.ifd) else {
            return Err(RasterError::new("Bu raster'in böyle bir bloğu yok."));
        };
        let want = l.block_w as usize * l.block_h as usize;
        let samples = match (components, l.per_block) {
            (c, p) if c == p => pixels,
            (1, 3) => pixels.iter().flat_map(|&g| [g, g, g]).collect(),
            (3, 1) => pixels.chunks_exact(3).map(|c| c[0]).collect(),
            // A browser's decoder gives RGBA: its alpha (always opaque for a JPEG) goes.
            (4, 3) => pixels
                .chunks_exact(4)
                .flat_map(|c| [c[0], c[1], c[2]])
                .collect(),
            (4, 1) => pixels.chunks_exact(4).map(|c| c[0]).collect(),
            _ => {
                return Err(RasterError::new(
                    "JPEG bloğunun bileşen sayısı rasterinkine uymuyor.",
                ));
            }
        };
        let mut samples = samples;
        samples.resize(want * l.per_block as usize, 0);
        self.blocks.put(key, Samples::U8(samples));
        Ok(())
    }

    /// A mosaic's file a block is of, and the block as that file's reader names it.
    fn part_of(&mut self, need: &BlockNeed) -> Option<(&mut MosaicPart, BlockNeed)> {
        let m = need.member.checked_sub(1)?;
        let part = self.mosaic.get_mut(m as usize)?;
        Some((part, BlockNeed { member: 0, ..*need }))
    }

    fn layout_of(&self, file: u8, ifd: u16) -> Option<Layout> {
        self.levels.iter().find_map(|lv| match &lv.from {
            LevelFrom::File(l) if l.file == file && l.ifd == ifd => Some(l.clone()),
            LevelFrom::Mesh(m) => m
                .layouts()
                .find(|l| l.file == file && l.ifd == ifd)
                .cloned(),
            _ => None,
        })
    }

    /// Keeps a mesh slice's block in its slot; false for any other block.
    fn keep_mesh(&mut self, ifd: u16, s: &Samples) -> bool {
        if self.mesh_levels().is_none() {
            return false;
        }
        let slot = match ifd {
            100 => &mut self.mesh_values.x,
            101 => &mut self.mesh_values.y,
            102 => &mut self.mesh_values.mask,
            _ => return false,
        };
        *slot = Some(Arc::new(s.clone()));
        true
    }

    /// The samples of a region of `level`, the level's edge repeated past
    /// its sides; none while a block it wants is not kept.
    pub fn region(&mut self, level: usize, x: i64, y: i64, w: u32, h: u32) -> Option<Region> {
        let lv = self.levels.get(level)?.clone();
        let bands = self.info.bands;
        let kind = self.info.sample;
        let mut out = Region {
            x,
            y,
            width: w,
            height: h,
            bands,
            samples: Samples::filled(kind, w as usize * h as usize * bands as usize, 0.0),
        };
        let (x0, y0, x1, y1) = clamp_rect(lv.width, lv.height, x, y, w, h);
        if x0 < x1 && y0 < y1 {
            match &lv.from {
                LevelFrom::Mosaic => self.mosaic_region(level, x0, y0, x1, y1, &mut out)?,
                LevelFrom::Image => {
                    let img = self.image.as_ref()?;
                    copy_rows(img, 0, 0, lv.width, bands, x0, y0, x1, y1, &mut out);
                }
                LevelFrom::File(l) => {
                    for by in y0 / l.block_h..=(y1 - 1) / l.block_h {
                        for bx in x0 / l.block_w..=(x1 - 1) / l.block_w {
                            let bx0 = bx * l.block_w;
                            let by0 = by * l.block_h;
                            let (cx0, cy0) = (x0.max(bx0), y0.max(by0));
                            let (cx1, cy1) = (x1.min(bx0 + l.block_w), y1.min(by0 + l.block_h));
                            for band in 0..if l.planar { l.bands } else { 1 } {
                                let key = BlockKey {
                                    file: l.file,
                                    ifd: l.ifd,
                                    index: l.index(bx, by, band),
                                };
                                let block = self.blocks.get(&key)?;
                                copy_block(block, l, bx0, by0, band, cx0, cy0, cx1, cy1, &mut out);
                            }
                        }
                    }
                }
                LevelFrom::Tiles(t) => {
                    let (c0, r0, across, _) = t.span(lv.width, lv.height);
                    let (tw, th) = (i64::from(t.zoom.tile_width), i64::from(t.zoom.tile_height));
                    let ca = (t.x + i64::from(x0)).div_euclid(tw);
                    let cb = (t.x + i64::from(x1) - 1).div_euclid(tw);
                    let ra = (t.y + i64::from(y0)).div_euclid(th);
                    let rb = (t.y + i64::from(y1) - 1).div_euclid(th);
                    for r in ra..=rb {
                        for c in ca..=cb {
                            let key = BlockKey {
                                file: TILES_FILE,
                                ifd: u16::try_from(level).unwrap_or(u16::MAX),
                                index: ((r - r0) * across + (c - c0)) as u32,
                            };
                            let block = self.blocks.get(&key)?;
                            // The tile's place in the level's pixels.
                            let bx0 = c * tw - t.x;
                            let by0 = r * th - t.y;
                            let cx0 = i64::from(x0).max(bx0);
                            let cy0 = i64::from(y0).max(by0);
                            let cx1 = i64::from(x1).min(bx0 + tw);
                            let cy1 = i64::from(y1).min(by0 + th);
                            let b = bands as usize;
                            for yy in cy0..cy1 {
                                let from = (((yy - by0) * tw + (cx0 - bx0)) as usize) * b;
                                let (ox, oy) = (cx0 - out.x, yy - out.y);
                                if ox < 0 || oy < 0 {
                                    continue;
                                }
                                let to = (oy as usize * out.width as usize + ox as usize) * b;
                                out.samples.copy_run(to, block, from, (cx1 - cx0) as usize * b);
                            }
                        }
                    }
                }
                LevelFrom::Computed | LevelFrom::Mesh(_) => {
                    let mesh = match &lv.from {
                        LevelFrom::Mesh(m) => Some(m.clone()),
                        _ => None,
                    };
                    for ty in y0 / TILE..=(y1 - 1) / TILE {
                        for tx in x0 / TILE..=(x1 - 1) / TILE {
                            let key = computed_key(level, tx, ty);
                            if !self.blocks.contains(&key) {
                                let tile = match &mesh {
                                    Some(m) => self.mesh_tile(m, level, tx, ty)?,
                                    None => self.compute_tile(level, tx, ty)?,
                                };
                                self.blocks.put(key, tile);
                            }
                            let tw = TILE.min(lv.width - tx * TILE);
                            let th = TILE.min(lv.height - ty * TILE);
                            let (bx0, by0) = (tx * TILE, ty * TILE);
                            let (cx0, cy0) = (x0.max(bx0), y0.max(by0));
                            let (cx1, cy1) = (x1.min(bx0 + tw), y1.min(by0 + th));
                            let tile = self.blocks.get(&key)?;
                            copy_rows(tile, bx0, by0, tw, bands, cx0, cy0, cx1, cy1, &mut out);
                        }
                    }
                }
            }
            replicate_edges(&mut out, lv.width, lv.height);
        }
        Some(out)
    }

    /// A mosaic level's pixels `[x₀, x₁) × [y₀, y₁)` into `out`: the files bottom first, each pixel's centre
    /// looked up in the file's level nearest the mosaic's (nearest neighbour, as GDAL reads a VRT's sources),
    /// a file's nodata or NaN leaving what is below; what no file covers the mosaic's nodata (or 0).
    fn mosaic_region(&mut self, level: usize, x0: u32, y0: u32, x1: u32, y1: u32, out: &mut Region) -> Option<()> {
        let bands = out.bands as usize;
        let fill = self.nodata.unwrap_or(0.0);
        let index = |out: &Region, i: u32, j: u32| {
            (((i64::from(j) - out.y) * i64::from(out.width) + (i64::from(i) - out.x)) as usize) * bands
        };
        for j in y0..y1 {
            for i in x0..x1 {
                let at = index(out, i, j);
                for b in 0..bands {
                    out.samples.set(at + b, fill);
                }
            }
        }
        let f = f64::from(1u32 << level.min(31));
        for part in &mut self.mosaic {
            let Some(cov) = part.covered(f, x0, y0, x1, y1) else {
                continue;
            };
            let k = part.level_for(level);
            let (rx, ry, rw, rh) = part.source_rect(level, k, cov);
            let src = part.reader.region(k, rx, ry, rw, rh)?;
            let (lw, lh) = {
                let lv = part.reader.levels.get(k)?;
                (i64::from(lv.width), i64::from(lv.height))
            };
            let (i0, j0, i1, j1) = cov;
            for j in j0..j1 {
                let y = (f64::from(j) + 0.5) * f;
                for i in i0..i1 {
                    let x = (f64::from(i) + 0.5) * f;
                    let (px, py) = part.pixel(k, x, y);
                    let (px, py) = (px.clamp(0, lw - 1), py.clamp(0, lh - 1));
                    let si = (px - rx).clamp(0, i64::from(rw) - 1) as u32;
                    let sj = (py - ry).clamp(0, i64::from(rh) - 1) as u32;
                    let at = index(out, i, j);
                    for (b, &fb) in part.bands.iter().enumerate() {
                        let v = src.at(si, sj, fb);
                        if v.is_nan() || part.nodata.is_some_and(|d| v == d) {
                            continue;
                        }
                        out.samples.set(at + b, v);
                    }
                }
            }
        }
        Some(())
    }

    /// Level `level`'s tile (`tx`, `ty`) of a mesh slice: each pixel's centre
    /// sampled on the mesh (docs/adr/0243 §5); none while its values are not read.
    fn mesh_tile(&self, m: &MeshLevels, level: usize, tx: u32, ty: u32) -> Option<Samples> {
        let lv = self.levels.get(level)?;
        let tw = TILE.min(lv.width - tx * TILE);
        let th = TILE.min(lv.height - ty * TILE);
        let (vx, vy, mask) = self.mesh_data(m)?;
        let data = Data {
            location: m.location,
            x: &vx,
            y: vy.as_deref(),
            mask: mask.as_deref(),
        };
        let f = f64::from(1u32 << level.min(31));
        let [x0, a, b, y0, c, d] = m.affine;
        let mut out = Samples::filled(self.info.sample, tw as usize * th as usize, f64::NAN);
        for j in 0..th {
            let v = (f64::from(ty * TILE + j) + 0.5) * f;
            for i in 0..tw {
                let u = (f64::from(tx * TILE + i) + 0.5) * f;
                let px = x0 + a * u + b * v;
                let py = y0 + c * u + d * v;
                out.set((j * tw + i) as usize, m.mesh.value_at(&data, px, py));
            }
        }
        Some(out)
    }

    /// Level `level`'s tile (`tx`, `ty`) worked out from the level below.
    fn compute_tile(&mut self, level: usize, tx: u32, ty: u32) -> Option<Samples> {
        let lv = self.levels.get(level)?.clone();
        let tw = TILE.min(lv.width - tx * TILE);
        let th = TILE.min(lv.height - ty * TILE);
        let below = self.levels.get(level - 1)?.clone();
        let src = self.region(
            level - 1,
            i64::from(tx * TILE * 2),
            i64::from(ty * TILE * 2),
            tw * 2,
            th * 2,
        )?;
        let bands = self.info.bands as usize;
        let nodata = self.nodata;
        let nearest = self.palette.is_some();
        let mut out = Samples::filled(
            self.info.sample,
            tw as usize * th as usize * bands,
            nodata.unwrap_or(0.0),
        );
        for j in 0..th {
            for i in 0..tw {
                for b in 0..bands {
                    let mut sum = 0.0;
                    let mut n = 0u32;
                    for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let (si, sj) = (2 * i + di, 2 * j + dj);
                        // Only the level below's own samples, not its repeated edge.
                        let (gx, gy) = (tx * TILE * 2 + si, ty * TILE * 2 + sj);
                        if gx >= below.width || gy >= below.height {
                            continue;
                        }
                        let v = src.at(si, sj, b as u32);
                        if v.is_nan() || nodata.is_some_and(|d| v == d) {
                            continue;
                        }
                        if nearest {
                            sum = v;
                            n = 1;
                            break;
                        }
                        sum += v;
                        n += 1;
                    }
                    let k = (j as usize * tw as usize + i as usize) * bands + b;
                    if n > 0 {
                        out.set(k, sum / f64::from(n));
                    } else if let Some(d) = nodata {
                        out.set(k, d);
                    } else if self.info.sample.float() {
                        out.set(k, f64::NAN);
                    }
                }
            }
        }
        Some(out)
    }

    /// The region a tile's colours want: the tile and two pixels round it
    /// (a pixel for the drawn apron, one more for the shaded relief's window).
    pub fn tile_region(tx: u32, ty: u32) -> (i64, i64, u32, u32) {
        (
            i64::from(tx * TILE) - 2,
            i64::from(ty * TILE) - 2,
            TILE + 4,
            TILE + 4,
        )
    }

    /// What tile (`tx`, `ty`) of `level` is coloured from, for a raster placed
    /// by `affine`: its region's samples and what the reader knows of them,
    /// so that the colouring runs off the reader; none while a block it wants
    /// is not kept.
    pub fn tile_parts(
        &mut self,
        level: usize,
        tx: u32,
        ty: u32,
        affine: &[f64; 6],
    ) -> Option<TileParts> {
        self.tile_parts_with(level, tx, ty, affine, false)
    }

    /// [`Reader::tile_parts`], with a mesh's edges in the tile when `edges`
    /// (docs/adr/0243 §5: none while the mean edge is under four pixels).
    pub fn tile_parts_with(
        &mut self,
        level: usize,
        tx: u32,
        ty: u32,
        affine: &[f64; 6],
        edges: bool,
    ) -> Option<TileParts> {
        let (x, y, w, h) = Reader::tile_region(tx, ty);
        let region = self.region(level, x, y, w, h)?;
        let f = f64::from(1u32 << level.min(31));
        let [_, a, b, _, c, d] = *affine;
        let ewres = math::hypot(a, c) * f;
        let ns = math::hypot(b, d) * f;
        let lines = match (edges, self.mesh_levels()) {
            (true, Some(m)) => mesh_lines(m, level, tx, ty),
            _ => Vec::new(),
        };
        Some(TileParts {
            region,
            palette: self.palette.clone(),
            file_nodata: self.nodata,
            sample: self.info.sample,
            ewres,
            nsres: if d < 0.0 { -ns } else { ns },
            lines,
        })
    }

    /// Tile (`tx`, `ty`) of `level` coloured by `style` into `out`
    /// (`TILE_APRON`² premultiplied RGBA) for a raster placed by `affine`;
    /// false while a block it wants is not kept.
    #[allow(clippy::too_many_arguments)]
    pub fn render_tile(
        &mut self,
        level: usize,
        tx: u32,
        ty: u32,
        style: &RasterStyle,
        stats: Option<&Stats>,
        affine: &[f64; 6],
        out: &mut Vec<u8>,
    ) -> bool {
        match self.tile_parts_with(level, tx, ty, affine, style.edges.is_some()) {
            Some(parts) => {
                parts.render(style, stats, out);
                true
            }
            None => false,
        }
    }

    /// The level statistics are taken from: the finest no wider than `STATS_SIDE`.
    pub fn stats_level(&self) -> usize {
        self.levels
            .iter()
            .position(|l| l.width <= STATS_SIDE && l.height <= STATS_SIDE)
            .unwrap_or(self.levels.len().saturating_sub(1))
    }

    /// The bands' statistics; none while a block of the statistics level is not kept.
    pub fn stats(&mut self) -> Option<Stats> {
        let level = self.stats_level();
        let lv = self.levels.get(level)?;
        let (w, h) = (lv.width, lv.height);
        let region = self.region(level, 0, 0, w, h)?;
        Some(stats::of_region(&region, self.nodata))
    }

    /// Band values at pixel (`i`, `j`) of level 0; none while its block is not kept.
    pub fn sample(&mut self, i: i64, j: i64) -> Option<Vec<f64>> {
        if i < 0 || j < 0 || i >= i64::from(self.info.width) || j >= i64::from(self.info.height) {
            return None;
        }
        let r = self.region(0, i, j, 1, 1)?;
        Some((0..r.bands).map(|b| r.at(0, 0, b)).collect())
    }
}

/// What a tile is coloured from ([`Reader::tile_parts`]): its region and
/// the raster's palette, nodata, samples and pixel size at its level.
#[derive(Clone, Debug)]
pub struct TileParts {
    pub region: Region,
    pub palette: Option<Vec<[u16; 3]>>,
    pub file_nodata: Option<f64>,
    pub sample: RasterSample,
    pub ewres: f64,
    pub nsres: f64,
    /// A mesh's edges in the tile (its output's pixels: x₁, y₁, x₂, y₂).
    pub lines: Vec<[f32; 4]>,
}

impl TileParts {
    /// The tile's colours by `style` into `out` (`TILE_APRON`² premultiplied RGBA).
    pub fn render(&self, style: &RasterStyle, stats: Option<&Stats>, out: &mut Vec<u8>) {
        let look = Look {
            style,
            stats,
            palette: self.palette.as_deref(),
            file_nodata: self.file_nodata,
            sample: self.sample,
            ewres: self.ewres,
            nsres: self.nsres,
        };
        style::render(&self.region, &look, out);
        if let Some(color) = style.edges.as_deref().and_then(edge_color) {
            draw_lines(&self.lines, color, out);
        }
    }
}

/// `#RRGGBB` as its three channels.
fn edge_color(text: &str) -> Option<[u8; 3]> {
    let h = text.strip_prefix('#')?;
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let ch = |k: usize| u8::from_str_radix(&h[k..k + 2], 16).ok();
    Some([ch(0)?, ch(2)?, ch(4)?])
}

/// A mesh's edges in tile (`tx`, `ty`) of `level`, in its output's pixels;
/// none when the mean edge is under four of the level's pixels (§5).
fn mesh_lines(m: &MeshLevels, level: usize, tx: u32, ty: u32) -> Vec<[f32; 4]> {
    let f = f64::from(1u32 << level.min(31));
    let [x0, a, b, y0, c, d] = m.affine;
    let det = a * d - b * c;
    let pixel = math::hypot(a, c).max(math::hypot(b, d)) * f;
    if !(det != 0.0 && det.is_finite()) || m.mesh.mean_edge < 4.0 * pixel {
        return Vec::new();
    }
    // The output's pixel (0, 0) is the level's (tx·256 − 1, ty·256 − 1).
    let (ox, oy) = (f64::from(tx * TILE) - 1.0, f64::from(ty * TILE) - 1.0);
    let side = f64::from(TILE_APRON);
    let to_pixel = |px: f64, py: f64| {
        let (dx, dy) = (px - x0, py - y0);
        (
            (d * dx - b * dy) / det / f - ox,
            (a * dy - c * dx) / det / f - oy,
        )
    };
    let at = |u: f64, v: f64| {
        let (uu, vv) = ((u + ox) * f, (v + oy) * f);
        (x0 + a * uu + b * vv, y0 + c * uu + d * vv)
    };
    let corners = [at(0.0, 0.0), at(side, 0.0), at(0.0, side), at(side, side)];
    let bbox = [
        corners.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
        corners.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
        corners
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max),
        corners
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max),
    ];
    let mut ids = Vec::new();
    m.mesh.edges_in(bbox, &mut ids);
    ids.iter()
        .map(|&e| {
            let [p, q] = m.mesh.edges[e as usize];
            let (u1, v1) = to_pixel(m.mesh.x[p as usize], m.mesh.y[p as usize]);
            let (u2, v2) = to_pixel(m.mesh.x[q as usize], m.mesh.y[q as usize]);
            [u1 as f32, v1 as f32, u2 as f32, v2 as f32]
        })
        .collect()
}

/// Draws lines a pixel wide into a tile's output (`TILE_APRON`² RGBA): each
/// line clipped to the tile, then the pixels it passes through stepping
/// along its longer axis.
fn draw_lines(lines: &[[f32; 4]], color: [u8; 3], out: &mut [u8]) {
    let side = TILE_APRON as i64;
    let edge = side as f64;
    for l in lines {
        let (x1, y1, x2, y2) = (
            f64::from(l[0]),
            f64::from(l[1]),
            f64::from(l[2]),
            f64::from(l[3]),
        );
        let (dx, dy) = (x2 - x1, y2 - y1);
        // Liang–Barsky: the part of the line inside [0, side]².
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut inside = true;
        for (p, q) in [(-dx, x1), (dx, edge - x1), (-dy, y1), (dy, edge - y1)] {
            if p == 0.0 {
                if q < 0.0 {
                    inside = false;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    t0 = t0.max(r);
                } else {
                    t1 = t1.min(r);
                }
            }
        }
        if !inside || t0 > t1 {
            continue;
        }
        let (ax, ay) = (x1 + dx * t0, y1 + dy * t0);
        let (bx, by) = (x1 + dx * t1, y1 + dy * t1);
        let steps = (bx - ax).abs().max((by - ay).abs()).ceil() as i64;
        for k in 0..=steps {
            let t = if steps == 0 {
                0.0
            } else {
                k as f64 / steps as f64
            };
            let (x, y) = (
                (ax + (bx - ax) * t).floor() as i64,
                (ay + (by - ay) * t).floor() as i64,
            );
            if x >= 0 && y >= 0 && x < side && y < side {
                let i = ((y * side + x) * 4) as usize;
                out[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
    }
}

/// A raster from a whole file's bytes (a TIFF or a PNG; a JPEG is the
/// host's to decode), placed by a world file's text when given, with every
/// block of a TIFF taken at once (tests, small files).
pub fn open_bytes(bytes: &[u8], world: Option<&str>, budget: usize) -> Result<Reader, RasterError> {
    let world = world.map(super::world::read).transpose()?;
    if super::tiff::sniff(bytes) {
        let mut store = super::ByteStore::new();
        store.put(0, bytes.to_vec());
        let super::Step::Done(t) = super::tiff::parse(&store, bytes.len() as u64)? else {
            return Err(RasterError::new("TIFF eksik okundu."));
        };
        Reader::tiff(&t, world, budget)
    } else if super::png::sniff(bytes) {
        let p = super::png::decode(bytes)?;
        Reader::image(
            p.width, p.height, p.bands, p.samples, p.nodata, world, budget,
        )
    } else {
        Err(RasterError::new("Dosya GeoTIFF, TIFF ya da PNG değil."))
    }
}

/// The key of a worked-out level's tile.
fn computed_key(level: usize, tx: u32, ty: u32) -> BlockKey {
    BlockKey {
        file: 2,
        ifd: u16::try_from(level).unwrap_or(u16::MAX),
        index: (ty << 16) | (tx & 0xFFFF),
    }
}

/// The part of a region inside a level of `w` × `h`: x0, y0, x1, y1.
fn clamp_rect(w: u32, h: u32, x: i64, y: i64, rw: u32, rh: u32) -> (u32, u32, u32, u32) {
    let c = |v: i64, hi: u32| v.clamp(0, i64::from(hi)) as u32;
    (
        c(x, w),
        c(y, h),
        c(x + i64::from(rw), w),
        c(y + i64::from(rh), h),
    )
}

/// Copies the part `x0`…`x1`, `y0`…`y1` (level pixels) of an image whose
/// upper left is at (`sx0`, `sy0`) and which is `width` wide (bands
/// interleaved) into `out`.
#[allow(clippy::too_many_arguments)]
fn copy_rows(
    src: &Samples,
    sx0: u32,
    sy0: u32,
    width: u32,
    bands: u32,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    out: &mut Region,
) {
    let b = bands as usize;
    let n = (x1 - x0) as usize * b;
    for y in y0..y1 {
        let from = ((y - sy0) as usize * width as usize + (x0 - sx0) as usize) * b;
        let (ox, oy) = (i64::from(x0) - out.x, i64::from(y) - out.y);
        if ox < 0 || oy < 0 {
            continue;
        }
        let to = (oy as usize * out.width as usize + ox as usize) * b;
        out.samples.copy_run(to, src, from, n);
    }
}

/// Copies a block's part (`cx0`…`cx1`, `cy0`…`cy1` in level pixels) into `out`.
#[allow(clippy::too_many_arguments)]
fn copy_block(
    block: &Samples,
    l: &Layout,
    bx0: u32,
    by0: u32,
    band: u32,
    cx0: u32,
    cy0: u32,
    cx1: u32,
    cy1: u32,
    out: &mut Region,
) {
    let per = l.per_block as usize;
    let bands = out.bands as usize;
    for y in cy0..cy1 {
        let row = (y - by0) as usize;
        let oy = i64::from(y) - out.y;
        let ox = i64::from(cx0) - out.x;
        if oy < 0 || ox < 0 {
            continue;
        }
        let from = (row * l.block_w as usize + (cx0 - bx0) as usize) * per;
        let to = (oy as usize * out.width as usize + ox as usize) * bands;
        if !l.planar {
            out.samples
                .copy_run(to, block, from, (cx1 - cx0) as usize * per);
        } else {
            for k in 0..(cx1 - cx0) as usize {
                let v = block.get(from + k);
                out.samples.set(to + k * bands + band as usize, v);
            }
        }
    }
}

/// Fills the region's pixels past the level's sides with its nearest edge pixel.
fn replicate_edges(r: &mut Region, w: u32, h: u32) {
    let b = r.bands as usize;
    let (rw, rh) = (r.width as i64, r.height as i64);
    // The region's columns and rows that are inside the level.
    let ix0 = (-r.x).clamp(0, rw);
    let ix1 = (i64::from(w) - r.x).clamp(0, rw);
    let iy0 = (-r.y).clamp(0, rh);
    let iy1 = (i64::from(h) - r.y).clamp(0, rh);
    if ix0 >= ix1 || iy0 >= iy1 {
        return;
    }
    let idx = |i: i64, j: i64| ((j * rw + i) as usize) * b;
    for j in iy0..iy1 {
        for i in 0..ix0 {
            let (to, from) = (idx(i, j), idx(ix0, j));
            for k in 0..b {
                let v = r.samples.get(from + k);
                r.samples.set(to + k, v);
            }
        }
        for i in ix1..rw {
            let (to, from) = (idx(i, j), idx(ix1 - 1, j));
            for k in 0..b {
                let v = r.samples.get(from + k);
                r.samples.set(to + k, v);
            }
        }
    }
    let row = rw as usize * b;
    for j in 0..iy0 {
        r.samples
            .copy_within(iy0 as usize * row, j as usize * row, row);
    }
    for j in iy1..rh {
        r.samples
            .copy_within((iy1 - 1) as usize * row, j as usize * row, row);
    }
}
