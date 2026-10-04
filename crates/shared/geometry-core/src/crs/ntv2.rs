//! NTv2 horizontal shift grids (docs/adr/0168 §4): a datum shift by a grid
//! of latitude and longitude shifts, the way PROJ applies one (its
//! `hgridshift`, `src/grids.cpp`): the densest grid holding the point (a
//! file's subgrids nest by their parents' names), bilinear interpolation of
//! the shifts PROJ keeps as single-precision radians, the forward step adds
//! them, the reverse one iterates to the point they carry onto the given
//! one. The reference is `scripts/fixtures/ntv2_cases.py` (PROJ on grids
//! written without KentOS code).
//!
//! A file is read whole and checked: the overview and every subgrid's
//! header, extents PROJ would take, a record count its extent allows, the
//! bytes there, finite shifts; anything else says what is wrong. Grids are
//! kept by their caller's name for them (the SHA-256 of the file) for the
//! transforms to find (`register`, `get`).

use std::sync::{Arc, Mutex};

use crate::jsmath::PI;

/// A file larger than this is not read.
pub const MAX_BYTES: usize = 256 * 1024 * 1024;
/// More subgrids than this is not a grid file.
const MAX_SUBGRIDS: u32 = 4096;
const RECORD: usize = 16;
const HEADER: usize = 11 * RECORD;
/// PROJ's relative tolerance at a grid's edges.
const TOLERANCE: f64 = 1e-5;
const DEG_TO_RAD: f64 = PI / 180.0;

/// What is wrong with a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridError {
    /// Larger than [`MAX_BYTES`].
    TooLarge,
    /// Not an NTv2 file (no overview header).
    NotNtv2,
    /// Its shifts are not in seconds.
    NotSeconds,
    /// A header is not where or what it should be.
    Header,
    /// A subgrid's extent or spacing PROJ would refuse.
    Extent,
    /// A subgrid's record count does not fit its extent.
    Count,
    /// The file ends before its records do.
    Truncated,
    /// A shift is not a finite number.
    Values,
}

impl GridError {
    /// As the reference writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            GridError::TooLarge => "tooLarge",
            GridError::NotNtv2 => "notNtv2",
            GridError::NotSeconds => "notSeconds",
            GridError::Header => "header",
            GridError::Extent => "extent",
            GridError::Count => "count",
            GridError::Truncated => "truncated",
            GridError::Values => "values",
        }
    }
}

/// A subgrid's extent and spacing in radians, longitudes east-positive.
#[derive(Clone, Copy, Debug)]
struct Extent {
    west: f64,
    east: f64,
    south: f64,
    north: f64,
    res_x: f64,
    res_y: f64,
}

impl Extent {
    fn full_world(&self) -> bool {
        self.east - self.west + self.res_x >= 2.0 * PI - 1e-10
    }

    fn epsilon(&self) -> f64 {
        (self.res_x + self.res_y) * TOLERANCE
    }

    /// PROJ's `isPointInExtent` for a geographic grid.
    fn holds(&self, x: f64, y: f64, eps: f64) -> bool {
        if !(y + eps >= self.south && y - eps <= self.north) {
            return false;
        }
        if self.full_world() {
            return true;
        }
        let x = if x + eps < self.west {
            x + 2.0 * PI
        } else if x - eps > self.east {
            x - 2.0 * PI
        } else {
            x
        };
        x + eps >= self.west && x - eps <= self.east
    }
}

#[derive(Debug)]
struct Sub {
    name: [u8; 8],
    extent: Extent,
    width: usize,
    height: usize,
    /// Per node, west to east and south to north: the latitude and the
    /// longitude shift in radians as PROJ keeps them (single precision; the
    /// longitude's sign turned east-positive).
    shifts: Vec<(f32, f32)>,
    children: Vec<usize>,
}

impl Sub {
    /// PROJ's `valueAt`: the longitude and the latitude shift at node (x, y).
    fn value(&self, x: i64, y: i64) -> (f32, f32) {
        let (lat, lon) = self.shifts[y as usize * self.width + x as usize];
        (lon, lat)
    }
}

/// A grid file: the datums it shifts from and to, as its header names them
/// with their ellipsoids' axes, and its subgrids.
#[derive(Debug)]
pub struct Grid {
    pub from: String,
    pub to: String,
    /// Semi-major and semi-minor axes (m) of the source and target ellipsoids.
    pub from_axes: (f64, f64),
    pub to_axes: (f64, f64),
    subs: Vec<Sub>,
    top: Vec<usize>,
}

fn label(b: &[u8]) -> String {
    String::from_utf8_lossy(b)
        .trim_end()
        .trim_end_matches('\0')
        .trim()
        .to_owned()
}

/// Reads a whole NTv2 file.
pub fn read(bytes: &[u8]) -> Result<Grid, GridError> {
    if bytes.len() > MAX_BYTES {
        return Err(GridError::TooLarge);
    }
    if bytes.len() < HEADER || &bytes[..8] != b"NUM_OREC" {
        return Err(GridError::NotNtv2);
    }
    // The overview's record count is 11: its first byte says which end comes first.
    let little = match (bytes[8], bytes[11]) {
        (11, _) => true,
        (_, 11) => false,
        _ => return Err(GridError::NotNtv2),
    };
    let u32_at = |b: &[u8], at: usize| {
        let mut v = [0u8; 4];
        v.copy_from_slice(&b[at..at + 4]);
        if little {
            u32::from_le_bytes(v)
        } else {
            u32::from_be_bytes(v)
        }
    };
    let f64_at = |b: &[u8], at: usize| {
        let mut v = [0u8; 8];
        v.copy_from_slice(&b[at..at + 8]);
        if little {
            f64::from_le_bytes(v)
        } else {
            f64::from_be_bytes(v)
        }
    };
    let f32_at = |b: &[u8], at: usize| {
        let mut v = [0u8; 4];
        v.copy_from_slice(&b[at..at + 4]);
        if little {
            f32::from_le_bytes(v)
        } else {
            f32::from_be_bytes(v)
        }
    };
    if &bytes[56..63] != b"SECONDS" {
        return Err(GridError::NotSeconds);
    }
    let count = u32_at(bytes, 40);
    if count == 0 || count > MAX_SUBGRIDS {
        return Err(GridError::Header);
    }
    let mut grid = Grid {
        from: label(&bytes[88..96]),
        to: label(&bytes[104..112]),
        from_axes: (f64_at(bytes, 120), f64_at(bytes, 136)),
        to_axes: (f64_at(bytes, 152), f64_at(bytes, 168)),
        subs: Vec::new(),
        top: Vec::new(),
    };
    let mut at = HEADER;
    for _ in 0..count {
        let h = bytes.get(at..at + HEADER).ok_or(GridError::Truncated)?;
        if &h[..8] != b"SUB_NAME" {
            return Err(GridError::Header);
        }
        let mut name = [0u8; 8];
        name.copy_from_slice(&h[8..16]);
        let mut parent = [0u8; 8];
        parent.copy_from_slice(&h[24..32]);
        let seconds = |at: usize| f64_at(h, at) * DEG_TO_RAD / 3600.0;
        let extent = Extent {
            south: seconds(72),
            north: seconds(88),
            east: -f64_at(h, 104) * DEG_TO_RAD / 3600.0,
            west: -f64_at(h, 120) * DEG_TO_RAD / 3600.0,
            res_y: seconds(136),
            res_x: seconds(152),
        };
        let ok = extent.west.abs() <= 4.0 * PI
            && extent.east.abs() <= 4.0 * PI
            && extent.north.abs() <= PI + 1e-5
            && extent.south.abs() <= PI + 1e-5
            && extent.west < extent.east
            && extent.south < extent.north
            && extent.res_x > 1e-10
            && extent.res_y > 1e-10;
        if !ok {
            return Err(GridError::Extent);
        }
        let (inv_x, inv_y) = (1.0 / extent.res_x, 1.0 / extent.res_y);
        let width = (((extent.east - extent.west) * inv_x + 0.5).abs() + 1.0) as usize;
        let height = (((extent.north - extent.south) * inv_y + 0.5).abs() + 1.0) as usize;
        let records = u32_at(h, 168) as usize;
        if width == 0 || records / width != height {
            return Err(GridError::Count);
        }
        let data = at + HEADER;
        let end = data
            .checked_add(records.checked_mul(RECORD).ok_or(GridError::Count)?)
            .ok_or(GridError::Count)?;
        if end > bytes.len() {
            return Err(GridError::Truncated);
        }
        let to_radians = (PI / 180.0) / 3600.0;
        let mut shifts = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                // Each row runs from east to west in the file.
                let r = data + (y * width + (width - 1 - x)) * RECORD;
                let (lat, lon) = (f32_at(bytes, r), f32_at(bytes, r + 4));
                if !lat.is_finite() || !lon.is_finite() {
                    return Err(GridError::Values);
                }
                shifts.push((
                    (f64::from(lat) * to_radians) as f32,
                    -((f64::from(lon) * to_radians) as f32),
                ));
            }
        }
        let index = grid.subs.len();
        // The latest subgrid of that name, as PROJ's map of names keeps it.
        match grid.subs.iter().rposition(|s| s.name == parent) {
            Some(p) => grid.subs[p].children.push(index),
            None => grid.top.push(index),
        }
        grid.subs.push(Sub {
            name,
            extent,
            width,
            height,
            shifts,
            children: Vec::new(),
        });
        at = end;
    }
    Ok(grid)
}

/// PROJ's `pj_hgrid_interpolate`: the shift at `t` (radians from the
/// subgrid's south-west corner); none outside it.
fn interpolate(t: (f64, f64), g: &Sub) -> Option<(f64, f64)> {
    let e = &g.extent;
    let lam = t.0 / e.res_x;
    let phi = t.1 / e.res_y;
    let mut ix = if lam.is_nan() { 0 } else { lam.floor() as i64 };
    let mut iy = if phi.is_nan() { 0 } else { phi.floor() as i64 };
    let mut fx = lam - ix as f64;
    let mut fy = phi - iy as f64;
    let (w, h) = (g.width as i64, g.height as i64);
    if ix < 0 {
        if ix == -1 && fx > 1.0 - 10.0 * TOLERANCE {
            ix += 1;
            fx = 0.0;
        } else {
            return None;
        }
    } else if ix + 1 >= w {
        if ix + 1 == w && fx < 10.0 * TOLERANCE {
            ix -= 1;
            fx = 1.0;
        } else {
            return None;
        }
    }
    if iy < 0 {
        if iy == -1 && fy > 1.0 - 10.0 * TOLERANCE {
            iy += 1;
            fy = 0.0;
        } else {
            return None;
        }
    } else if iy + 1 >= h {
        if iy + 1 == h && fy < 10.0 * TOLERANCE {
            iy -= 1;
            fy = 1.0;
        } else {
            return None;
        }
    }
    if ix < 0 || iy < 0 || ix + 1 >= w || iy + 1 >= h {
        return None;
    }
    let f00 = g.value(ix, iy);
    let f10 = g.value(ix + 1, iy);
    let f01 = g.value(ix, iy + 1);
    let f11 = g.value(ix + 1, iy + 1);
    let mut m10 = fx;
    let mut m11 = m10;
    let mut m01 = 1.0 - fx;
    let mut m00 = m01;
    m11 *= fy;
    m01 *= fy;
    let gy = 1.0 - fy;
    m00 *= gy;
    m10 *= gy;
    let lon = m00 * f64::from(f00.0)
        + m10 * f64::from(f10.0)
        + m01 * f64::from(f01.0)
        + m11 * f64::from(f11.0);
    let lat = m00 * f64::from(f00.1)
        + m10 * f64::from(f10.1)
        + m01 * f64::from(f01.1)
        + m11 * f64::from(f11.1);
    Some((lon, lat))
}

/// PROJ's `adjlon`.
fn adjlon(lon: f64) -> f64 {
    if lon.abs() < PI + 1e-12 {
        return lon;
    }
    let two_pi = 2.0 * PI;
    let l = lon + PI;
    let l = l - two_pi * (l / two_pi).floor();
    l - PI
}

impl Grid {
    /// The densest subgrid holding the point (radians), as PROJ finds it.
    fn find(&self, lam: f64, phi: f64) -> Option<usize> {
        fn down(g: &Grid, i: usize, lam: f64, phi: f64) -> usize {
            for &c in &g.subs[i].children {
                let e = &g.subs[c].extent;
                if e.holds(lam, phi, e.epsilon()) {
                    return down(g, c, lam, phi);
                }
            }
            i
        }
        self.top.iter().find_map(|&i| {
            let e = &self.subs[i].extent;
            e.holds(lam, phi, e.epsilon())
                .then(|| down(self, i, lam, phi))
        })
    }

    /// The point (longitude, latitude in radians) shifted from the grid's
    /// source datum to its target (`reverse` false) or back; none outside
    /// the grid, or where the reverse step does not settle.
    pub fn apply(&self, lam: f64, phi: f64, reverse: bool) -> Option<(f64, f64)> {
        let mut grid = self.find(lam, phi)?;
        // PROJ takes the first subgrid's tolerance for every normalisation.
        let eps = self.subs[grid].extent.epsilon();
        let normalise = |g: usize| {
            let e = &self.subs[g].extent;
            let mut x = lam - e.west;
            if x + eps < 0.0 {
                x += 2.0 * PI;
            } else if x - eps > e.east - e.west {
                x -= 2.0 * PI;
            }
            (x, phi - e.south)
        };
        let mut tb = normalise(grid);
        let t = interpolate(tb, &self.subs[grid])?;
        if !reverse {
            return Some((lam + t.0, phi + t.1));
        }
        let mut t = (tb.0 - t.0, tb.1 - t.1);
        let toltol = 1e-12 * 1e-12;
        let mut i = 10;
        loop {
            let dif;
            match interpolate(t, &self.subs[grid]) {
                Some(del) => {
                    dif = (t.0 + del.0 - tb.0, t.1 + del.1 - tb.1);
                    t = (t.0 - dif.0, t.1 - dif.1);
                }
                None => {
                    // The guess left this subgrid: find the one it is in.
                    let e = self.subs[grid].extent;
                    let (x, y) = (t.0 + e.west, t.1 + e.south);
                    match self.find(x, y) {
                        Some(g) if g != grid => {
                            grid = g;
                            let e = self.subs[g].extent;
                            t = (x - e.west, y - e.south);
                            tb = normalise(g);
                            dif = (f64::MAX, f64::MAX);
                        }
                        // PROJ keeps its first approximation.
                        _ => break,
                    }
                }
            }
            i -= 1;
            if !(i != 0 && dif.0 * dif.0 + dif.1 * dif.1 > toltol) {
                break;
            }
        }
        if i == 0 {
            return None;
        }
        let e = self.subs[grid].extent;
        Some((adjlon(t.0 + e.west), t.1 + e.south))
    }

    /// The subgrids' count and the whole grid's extent in degrees (west,
    /// south, east, north), for the grid's description.
    pub fn summary(&self) -> (usize, [f64; 4]) {
        let deg = 180.0 / PI;
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for &i in &self.top {
            let e = self.subs[i].extent;
            b = [
                crate::jsmath::js_min(b[0], e.west * deg),
                crate::jsmath::js_min(b[1], e.south * deg),
                crate::jsmath::js_max(b[2], e.east * deg),
                crate::jsmath::js_max(b[3], e.north * deg),
            ];
        }
        (self.subs.len(), b)
    }
}

impl Grid {
    /// What the grid says, as JSON (the web's): the datums, their
    /// ellipsoids' axes, the subgrids' count and the extent in degrees.
    pub fn info_json(&self) -> String {
        use crate::api::json::field;
        let (subgrids, extent) = self.summary();
        let mut out = String::from("{");
        let mut first = true;
        field(&mut out, &mut first, "from", &self.from);
        field(&mut out, &mut first, "to", &self.to);
        field(
            &mut out,
            &mut first,
            "fromAxes",
            &[self.from_axes.0, self.from_axes.1],
        );
        field(
            &mut out,
            &mut first,
            "toAxes",
            &[self.to_axes.0, self.to_axes.1],
        );
        field(&mut out, &mut first, "subgrids", &subgrids);
        field(&mut out, &mut first, "extent", &extent);
        out.push('}');
        out
    }
}

/// The grids the transforms may use, by their callers' names (the files'
/// SHA-256).
static GRIDS: Mutex<Vec<(String, Arc<Grid>)>> = Mutex::new(Vec::new());

/// Keeps `grid` under `id` for the transforms (the same id replaced).
pub fn register(id: &str, grid: Grid) {
    if let Ok(mut all) = GRIDS.lock() {
        all.retain(|(k, _)| k != id);
        all.push((id.to_owned(), Arc::new(grid)));
    }
}

/// The grid kept under `id`.
pub fn get(id: &str) -> Option<Arc<Grid>> {
    GRIDS
        .lock()
        .ok()?
        .iter()
        .find(|(k, _)| k == id)
        .map(|(_, g)| Arc::clone(g))
}

/// Lets the grid kept under `id` go.
pub fn forget(id: &str) {
    if let Ok(mut all) = GRIDS.lock() {
        all.retain(|(k, _)| k != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostile_files_say_what_is_wrong() {
        assert_eq!(read(b"NUM_OREC").err(), Some(GridError::NotNtv2));
        assert_eq!(read(&[0u8; 400]).err(), Some(GridError::NotNtv2));
        let mut h = vec![0u8; HEADER];
        h[..8].copy_from_slice(b"NUM_OREC");
        h[8] = 11;
        h[56..63].copy_from_slice(b"MINUTES");
        assert_eq!(read(&h).err(), Some(GridError::NotSeconds));
        h[56..63].copy_from_slice(b"SECONDS");
        assert_eq!(read(&h).err(), Some(GridError::Header));
        h[40] = 1;
        assert_eq!(read(&h).err(), Some(GridError::Truncated));
    }
}
