//! Values at points of a raster read by its blocks (docs/adr/0243 §8, §9):
//! a point's cell's value bands (nodata, NaN and a zero alpha none; the
//! analysis input's rule), or on a mesh's raster the mesh's interpolation at
//! the point, not a pixel's. The points are read in batches whose blocks the
//! reader holds at once.

use kentos_formats::raster::source::{BlockNeed, Put};

use super::{Finished, profile, series};
use crate::inputs::{Input, floor_i};

/// The most blocks a batch reads (the reader keeps them all while it is read).
const BATCH_BLOCKS: usize = 16;
/// The most points a batch reads.
const BATCH_POINTS: usize = 4096;

/// The values of a raster's value bands at points.
pub struct PointValues {
    input: Input,
    points: Vec<[f64; 2]>,
    /// Each point's cell, none outside the grid (a mesh's: unused).
    cells: Vec<Option<(i64, i64)>>,
    bands: Vec<usize>,
    mesh: bool,
    /// The first point not read, and the end of the batch the needs listed.
    next: usize,
    end: usize,
    pending: Vec<BlockNeed>,
    /// Each point's bands' values in turn.
    values: Vec<f64>,
}

impl PointValues {
    /// `bands` (from 0) of `input` at `points` (drawing coordinates); a
    /// mesh's raster gives its one value whatever the bands.
    pub fn new(input: Input, points: Vec<[f64; 2]>, bands: Vec<usize>) -> PointValues {
        let mesh = input.reader.mesh_levels().is_some();
        let (w, h) = (i64::from(input.width), i64::from(input.height));
        let cells = points
            .iter()
            .map(|&[x, y]| {
                let (u, v) = input.place(x, y);
                if !(u.is_finite() && v.is_finite() && u.abs() < 1e15 && v.abs() < 1e15) {
                    return None;
                }
                let (i, j) = (floor_i(u), floor_i(v));
                (i >= 0 && j >= 0 && i < w && j < h).then_some((i, j))
            })
            .collect();
        let bands = if mesh { vec![0] } else { bands };
        PointValues {
            values: Vec::with_capacity(points.len() * bands.len()),
            input,
            points,
            cells,
            bands,
            mesh,
            next: 0,
            end: 0,
            pending: Vec::new(),
        }
    }

    /// The bands each point gives.
    pub fn band_count(&self) -> usize {
        self.bands.len()
    }

    /// The blocks the next batch needs that the reader does not hold.
    pub fn needs(&mut self) -> Vec<BlockNeed> {
        self.pending.clear();
        if self.mesh {
            self.end = self.points.len();
            self.pending = self.input.reader.needs(0, 0, 0, 1, 1);
            return self.pending.clone();
        }
        let mut k = self.next;
        while k < self.points.len() && k - self.next < BATCH_POINTS {
            if let Some((i, j)) = self.cells[k] {
                let fresh: Vec<BlockNeed> = self
                    .input
                    .needs((i, j, 1, 1))
                    .into_iter()
                    .filter(|b| !self.pending.contains(b))
                    .collect();
                if !fresh.is_empty()
                    && k > self.next
                    && self.pending.len() + fresh.len() > BATCH_BLOCKS
                {
                    break;
                }
                self.pending.extend(fresh);
            }
            k += 1;
        }
        self.end = k;
        self.pending.clone()
    }

    /// Need `k`'s bytes; a JPEG block's stream back.
    pub fn put(&mut self, k: usize, bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let need = self
            .pending
            .get(k)
            .cloned()
            .ok_or("Rasterin böyle bir bloğu istenmedi.")?;
        match self.input.reader.put_block(&need, bytes).map_err(|e| e.0)? {
            Put::Done => Ok(None),
            Put::Jpeg(s) => Ok(Some(s)),
        }
    }

    /// A JPEG block's pixels.
    pub fn put_pixels(&mut self, k: usize, pixels: Vec<u8>, components: u32) -> Result<(), String> {
        let need = self
            .pending
            .get(k)
            .cloned()
            .ok_or("Rasterin böyle bir bloğu istenmedi.")?;
        self.input
            .reader
            .put_pixels(&need, pixels, components)
            .map_err(|e| e.0)
    }

    /// Reads the batch's values (its blocks put).
    pub fn step(&mut self) -> Result<(), String> {
        let nb = self.bands.len();
        if self.mesh {
            for &[x, y] in &self.points[self.next..self.end] {
                let v = self
                    .input
                    .reader
                    .mesh_value(x, y)
                    .ok_or("Mesh'in değerleri eksik okundu.")?;
                self.values.push(v);
            }
        } else {
            let mut out = vec![0.0; nb];
            for k in self.next..self.end {
                match self.cells[k] {
                    Some((i, j)) => {
                        let raw = self.input.raw((i, j, 1, 1))?;
                        raw.cells(j, i, &self.bands, &mut out);
                        self.values.extend_from_slice(&out);
                    }
                    None => self.values.extend(std::iter::repeat_n(f64::NAN, nb)),
                }
            }
        }
        self.next = self.end;
        Ok(())
    }

    pub fn done(&self) -> bool {
        self.next >= self.points.len()
    }

    pub fn share(&self) -> f64 {
        self.next as f64 / self.points.len().max(1) as f64
    }

    /// Each point's bands' values in turn (NaN where none).
    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

/// What a points job's values make.
pub enum Make {
    Profile(profile::Made),
    Bands(series::BandsMade),
}

/// A job of values at points: Kesit, or Zaman serisi over bands.
pub struct PointsJob {
    pub values: PointValues,
    pub make: Make,
}

impl PointsJob {
    pub fn needs(&mut self) -> Vec<(u64, u64)> {
        self.values
            .needs()
            .iter()
            .map(|n| (n.offset, n.len))
            .collect()
    }

    pub fn put(&mut self, k: usize, bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
        self.values.put(k, bytes)
    }

    pub fn put_pixels(&mut self, k: usize, pixels: Vec<u8>, components: u32) -> Result<(), String> {
        self.values.put_pixels(k, pixels, components)
    }

    pub fn step(&mut self) -> Result<(), String> {
        self.values.step()
    }

    pub fn done(&self) -> bool {
        self.values.done()
    }

    pub fn share(&self) -> f64 {
        self.values.share()
    }

    pub fn finish(self) -> Finished {
        let PointsJob { values, make } = self;
        match make {
            Make::Profile(m) => m.finish(values.values()),
            Make::Bands(m) => m.finish(values.values(), values.band_count()),
        }
    }
}
