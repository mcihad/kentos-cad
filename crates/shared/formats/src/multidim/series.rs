//! A dataset's values at points over its time steps (Zaman serisi,
//! docs/adr/0243 §9): the values the points need and no others are read,
//! the runs merged where they lie close. Each value is made as the raster
//! shows it: a grid's cell (the drawing's place of the raster, its rows as
//! the reader turns them), a mesh's interpolation with the tiles' own
//! arithmetic (`Mesh::value_by`); the other slice dimensions stay at the
//! raster's values.

use std::sync::Arc;

use kentos_contracts::RasterSample;

use super::cf::{self, Unpack};
use super::cube::{Cube, Part};
use super::float_sample;
use super::mesh::Mesh;
use super::netcdf::{self, NcType, Var};
use super::ugrid::Location;
use crate::raster::{Need, RasterError};

/// Values closer than this many bytes are read in one run.
const GAP: u64 = 4096;
/// The longest run read at once.
const RUN_MOST: u64 = 1 << 20;
/// The most values a series reads (steps × points × the values a point reads).
pub const VALUES_MOST: u64 = 20_000_000;

/// How a point's value is made.
#[derive(Clone, Copy, Debug)]
enum Pick {
    /// Outside the grid or the mesh: no value at any step.
    Outside,
    /// A grid's cell.
    Cell,
    /// A point of a mesh's triangle.
    Tri { t: u32, x: f64, y: f64 },
}

/// A variable read value by value: its type and how its values open.
#[derive(Clone, Debug)]
struct Kind {
    nc: NcType,
    unpack: Unpack,
}

impl Kind {
    fn of(v: &Var, unpack: Unpack) -> Kind {
        Kind { nc: v.kind, unpack }
    }

    /// A stored value as the raster's reader keeps it (a 32-bit float sample rounded to 32 bits).
    fn value(&self, raw: f64) -> f64 {
        self.unpack.stored(raw)
    }
}

/// The values at points over the time steps of a NetCDF raster's dataset.
#[derive(Debug)]
pub struct Series {
    /// The runs of the file still to read.
    pub runs: Vec<Need>,
    /// The time dimension's name and its values (ms since 1970).
    pub time_name: String,
    pub times: Vec<f64>,
    picks: Vec<Pick>,
    /// Each step's each point's values' offsets: a grid's one; a mesh's
    /// places' (x, then y for a vector), then its face's mask.
    offsets: Vec<u64>,
    /// Where (step, point)'s offsets start; one past the last at the end.
    starts: Vec<usize>,
    mesh: Option<(Arc<Mesh>, Location)>,
    x: Kind,
    y: Option<Kind>,
    mask: Option<Kind>,
    /// A grid's nodata (the look's, else the file's).
    nodata: Option<f64>,
    /// The bytes read, by where they start (sorted).
    bytes: Vec<(u64, Vec<u8>)>,
}

/// Why a raster without a time dimension has no series.
pub const NO_TIME: &str = "Rasterin veri setinin zaman boyutu yok: Zaman serisi zaman boyutlu veri seti ya da çok bantlı raster ister.";

impl Cube {
    /// The series of `part`'s dataset at `points` (drawing coordinates) over
    /// its time dimension, a grid placed in the drawing by `affine`; after
    /// [`super::cube::Want::Part`]'s needs. Its runs are then read and put.
    pub fn series(
        &mut self,
        part: &Part,
        affine: [f64; 6],
        nodata: Option<f64>,
        points: &[[f64; 2]],
    ) -> Result<Series, RasterError> {
        let h = self.header.clone();
        let var = |name: &str| {
            h.var(name)
                .ok_or_else(|| RasterError::new(format!("NetCDF'te “{name}” değişkeni yok.")))
        };
        let v = var(&part.variable)?;
        // The variable's slice dimensions, the places of its values, and how a point picks them.
        let (dims, mesh, x, y, mask) = match &part.mesh {
            None => {
                let g = self.grid_shape(&part.variable)?;
                (
                    g.slice.clone(),
                    None,
                    Kind::of(v, g.unpack.clone()),
                    None,
                    None,
                )
            }
            Some(m) => {
                let d = self.dataset(m, &part.variable).cloned().ok_or_else(|| {
                    RasterError::new(format!("“{m}” ağında “{}” veri seti yok.", part.variable))
                })?;
                let mesh = self.mesh(m)?;
                let float = |v: &Var| {
                    let mut u = cf::unpack_of(v);
                    u.sample = float_sample(u.sample);
                    Kind::of(v, u)
                };
                let y = match &d.vector {
                    Some(n) => Some(float(var(n)?)),
                    None => None,
                };
                let mask = match &d.mask {
                    Some(n) => {
                        let mv = var(n)?;
                        let mut u = cf::unpack_of(mv);
                        u.sample = RasterSample::F32;
                        u.packed = false;
                        Some(Kind::of(mv, u))
                    }
                    None => None,
                };
                (
                    d.slice_dims.clone(),
                    Some((mesh, d.location)),
                    float(v),
                    y,
                    mask,
                )
            }
        };
        let slice = self.slice_of(part, &dims)?;
        let slice_dims = self.slice_dims_of(&dims)?;
        let Some(tk) = slice_dims.iter().position(|d| d.time) else {
            return Err(RasterError::new(NO_TIME));
        };
        let times = slice_dims[tk].values.clone();
        let steps = times.len();
        // Where each point's value comes from.
        let grid = match &part.mesh {
            None => Some(self.grid_shape(&part.variable)?),
            Some(_) => None,
        };
        let [x0, a, b, y0, c, d] = affine;
        let det = a * d - b * c;
        let picks: Vec<Pick> = points
            .iter()
            .map(|&[px, py]| match (&grid, &mesh) {
                (Some(g), _) => {
                    let (dx, dy) = (px - x0, py - y0);
                    let (u, w) = ((d * dx - b * dy) / det, (a * dy - c * dx) / det);
                    let (i, j) = (u.floor(), w.floor());
                    if det != 0.0
                        && i >= 0.0
                        && j >= 0.0
                        && i < f64::from(g.width)
                        && j < f64::from(g.height)
                    {
                        Pick::Cell
                    } else {
                        Pick::Outside
                    }
                }
                (None, Some((m, _))) => match m.locate(px, py) {
                    Some(t) => Pick::Tri { t, x: px, y: py },
                    None => Pick::Outside,
                },
                _ => Pick::Outside,
            })
            .collect();
        let per = |p: &Pick| -> u64 {
            match (p, &mesh) {
                (Pick::Outside, _) => 0,
                (Pick::Cell, _) => 1,
                (Pick::Tri { .. }, Some((_, loc))) => {
                    let k = if *loc == Location::Node { 3 } else { 1 };
                    k * (1 + u64::from(y.is_some())) + u64::from(mask.is_some())
                }
                _ => 0,
            }
        };
        let total: u64 = picks
            .iter()
            .map(per)
            .sum::<u64>()
            .saturating_mul(steps as u64);
        if total > VALUES_MOST {
            return Err(RasterError::new(format!(
                "Zaman serisi {total} değer okur, en çok {VALUES_MOST}; daha az nokta seçin."
            )));
        }
        let mut offsets: Vec<u64> = Vec::with_capacity(total as usize);
        // Each offset's value's size, for the runs.
        let mut sizes: Vec<u64> = Vec::with_capacity(total as usize);
        let mut starts: Vec<usize> = Vec::with_capacity(steps * points.len() + 1);
        let at = |name: &str, idx: &[u64]| -> Result<Need, RasterError> {
            let v = var(name)?;
            h.run(v, idx, 1).ok_or_else(|| {
                RasterError::new(format!("“{name}” değişkeninin değerleri dosyanın dışında."))
            })
        };
        let mask_name = part
            .mesh
            .as_deref()
            .and_then(|m| self.dataset(m, &part.variable))
            .and_then(|d| d.mask.clone());
        for s in 0..steps {
            let mut idx = slice.clone();
            idx[tk] = s as u64;
            for (k, p) in picks.iter().enumerate() {
                starts.push(offsets.len());
                match (p, &grid, &mesh) {
                    (Pick::Cell, Some(g), _) => {
                        let [px, py] = points[k];
                        let (dx, dy) = (px - x0, py - y0);
                        let (u, w) = ((d * dx - b * dy) / det, (a * dy - c * dx) / det);
                        let (i, j) = (u.floor() as u64, w.floor() as u64);
                        let row = if g.flip {
                            u64::from(g.height) - 1 - j
                        } else {
                            j
                        };
                        let mut full = idx.clone();
                        full.extend([row, i]);
                        let n = at(&part.variable, &full)?;
                        offsets.push(n.offset);
                        sizes.push(n.len);
                    }
                    (Pick::Tri { t, .. }, _, Some((m, loc))) => {
                        let (places, face) = m.places_of(*t, *loc);
                        let mut place_at = |name: &str, p: u32| -> Result<(), RasterError> {
                            let mut full = idx.clone();
                            full.push(u64::from(p));
                            let n = at(name, &full)?;
                            offsets.push(n.offset);
                            sizes.push(n.len);
                            Ok(())
                        };
                        for &p in &places {
                            place_at(&part.variable, p)?;
                        }
                        if let Some(yn) = &part.vector {
                            for &p in &places {
                                place_at(yn, p)?;
                            }
                        }
                        if let Some(mn) = &mask_name {
                            place_at(mn, face)?;
                        }
                    }
                    _ => {}
                }
            }
        }
        starts.push(offsets.len());
        // The runs: every value once, close ones together.
        let mut sorted: Vec<(u64, u64)> =
            offsets.iter().copied().zip(sizes.iter().copied()).collect();
        sorted.sort_unstable();
        let mut runs: Vec<Need> = Vec::new();
        for (off, len) in sorted {
            match runs.last_mut() {
                Some(r) if off <= r.offset + r.len + GAP && off + len - r.offset <= RUN_MOST => {
                    r.len = r.len.max(off + len - r.offset);
                }
                _ => runs.push(Need { offset: off, len }),
            }
        }
        Ok(Series {
            runs,
            time_name: slice_dims[tk].name.clone(),
            times,
            picks,
            offsets,
            starts,
            mesh,
            x,
            y,
            mask,
            nodata: if part.mesh.is_none() { nodata } else { None },
            bytes: Vec::new(),
        })
    }
}

impl Series {
    /// Takes a run's bytes (at `offset`).
    pub fn put(&mut self, offset: u64, bytes: Vec<u8>) {
        let at = self.bytes.partition_point(|(s, _)| *s < offset);
        self.bytes.insert(at, (offset, bytes));
    }

    /// The raw value of type `nc` at `offset`, when its bytes were put.
    fn raw(&self, nc: NcType, offset: u64) -> Option<f64> {
        let k = self
            .bytes
            .partition_point(|(s, _)| *s <= offset)
            .checked_sub(1)?;
        let (start, b) = &self.bytes[k];
        let a = usize::try_from(offset - start).ok()?;
        let e = a.checked_add(nc.size() as usize)?;
        let mut out = Vec::with_capacity(1);
        netcdf::decode(nc, b.get(a..e)?, &mut out);
        out.first().copied()
    }

    /// Each step's value at each point (`[step][point]`; NaN where none),
    /// after every run was put.
    pub fn values(&self) -> Result<Vec<Vec<f64>>, RasterError> {
        let n = self.picks.len();
        let missing = || RasterError::new("Zaman serisinin değerleri eksik okundu.");
        let mut out = Vec::with_capacity(self.times.len());
        for s in 0..self.times.len() {
            let mut row = Vec::with_capacity(n);
            for (k, p) in self.picks.iter().enumerate() {
                let from = self.starts[s * n + k];
                let offs = &self.offsets[from..self.starts[s * n + k + 1]];
                let v = match (p, &self.mesh) {
                    (Pick::Outside, _) => f64::NAN,
                    (Pick::Cell, _) => {
                        let v = self
                            .x
                            .value(self.raw(self.x.nc, offs[0]).ok_or_else(missing)?);
                        if v.is_nan() || self.nodata.is_some_and(|d| v == d) {
                            f64::NAN
                        } else {
                            v
                        }
                    }
                    (Pick::Tri { t, x, y }, Some((m, loc))) => {
                        let (places, face) = m.places_of(*t, *loc);
                        let k = places.len();
                        let mut xs = Vec::with_capacity(k);
                        for &o in &offs[..k] {
                            xs.push(self.x.value(self.raw(self.x.nc, o).ok_or_else(missing)?));
                        }
                        let mut ys = Vec::new();
                        if let Some(yk) = &self.y {
                            for &o in &offs[k..2 * k] {
                                ys.push(yk.value(self.raw(yk.nc, o).ok_or_else(missing)?));
                            }
                        }
                        let mv = match &self.mask {
                            Some(mk) => {
                                let o = *offs.last().ok_or_else(missing)?;
                                Some(mk.value(self.raw(mk.nc, o).ok_or_else(missing)?))
                            }
                            None => None,
                        };
                        let find = |vals: &[f64], p: usize| {
                            places
                                .iter()
                                .position(|&q| q as usize == p)
                                .and_then(|i| vals.get(i).copied())
                                .unwrap_or(f64::NAN)
                        };
                        let ys = (!ys.is_empty()).then_some(ys);
                        m.value_by(
                            *t,
                            *x,
                            *y,
                            *loc,
                            |p| find(&xs, p),
                            ys.as_ref().map(|ys| move |p: usize| find(ys, p)),
                            mv.map(|v| {
                                move |f: usize| if f == face as usize { v } else { f64::NAN }
                            }),
                        )
                    }
                    _ => f64::NAN,
                };
                row.push(v);
            }
            out.push(row);
        }
        Ok(out)
    }
}
