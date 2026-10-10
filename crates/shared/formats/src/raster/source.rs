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

use kentos_contracts::RasterSample;
use serde::Serialize;

use kentos_contracts::RasterStyle;

pub use super::layout::{Layout, Put2, decode_block};
use super::layout::{compression_name, layout};
use super::stats::{self, STATS_SIDE, Stats};
use super::style::{self, Look};
use super::tiff::{Ifd, Tiff};
use super::{RasterError, Samples, TILE, geotiff};
use crate::math;

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
    pub file: u8,
    pub ifd: u16,
    pub index: u32,
    pub offset: u64,
    pub len: u64,
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

/// What a level's samples come from.
#[derive(Clone, Debug, PartialEq)]
pub enum LevelFrom {
    /// A directory's blocks.
    File(Layout),
    /// The whole decoded image (level 0 of a PNG or JPEG).
    Image,
    /// Worked out from the level below, in 256 × 256 tiles.
    Computed,
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
        })
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
        })
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
        out.sort_by_key(|n| (n.file, n.ifd, n.index));
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

    /// Takes a block's bytes: decoded and kept, or its JPEG stream for the host.
    pub fn put_block(&mut self, need: &BlockNeed, bytes: &[u8]) -> Result<Put, RasterError> {
        let Some(l) = self.layout_of(need.file, need.ifd) else {
            return Err(RasterError::new("Bu raster'in böyle bir bloğu yok."));
        };
        let key = BlockKey {
            file: need.file,
            ifd: need.ifd,
            index: need.index,
        };
        match decode_block(&l, need.index, bytes)? {
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
        self.layout_of(need.file, need.ifd)
    }

    /// Keeps a block decoded off the reader: its samples, or its JPEG stream
    /// waiting for the host's pixels ([`Reader::put_pixels`]).
    pub fn keep(&mut self, need: &BlockNeed, decoded: Put2) -> Put {
        let key = BlockKey {
            file: need.file,
            ifd: need.ifd,
            index: need.index,
        };
        match decoded {
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
        let key = BlockKey {
            file: need.file,
            ifd: need.ifd,
            index: need.index,
        };
        self.jpeg_waiting.retain(|k| *k != key);
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

    fn layout_of(&self, file: u8, ifd: u16) -> Option<Layout> {
        self.levels.iter().find_map(|lv| match &lv.from {
            LevelFrom::File(l) if l.file == file && l.ifd == ifd => Some(l.clone()),
            _ => None,
        })
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
                LevelFrom::Computed => {
                    for ty in y0 / TILE..=(y1 - 1) / TILE {
                        for tx in x0 / TILE..=(x1 - 1) / TILE {
                            let key = computed_key(level, tx, ty);
                            if !self.blocks.contains(&key) {
                                let tile = self.compute_tile(level, tx, ty)?;
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
        let (x, y, w, h) = Reader::tile_region(tx, ty);
        let region = self.region(level, x, y, w, h)?;
        let f = f64::from(1u32 << level.min(31));
        let [_, a, b, _, c, d] = *affine;
        let ewres = math::hypot(a, c) * f;
        let ns = math::hypot(b, d) * f;
        Some(TileParts {
            region,
            palette: self.palette.clone(),
            file_nodata: self.nodata,
            sample: self.info.sample,
            ewres,
            nsres: if d < 0.0 { -ns } else { ns },
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
        match self.tile_parts(level, tx, ty, affine) {
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
