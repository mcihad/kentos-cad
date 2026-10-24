//! Geoid models and velocity grids (docs/adr/0631 §1, §2): a grid of latitude and longitude whose nodes hold the
//! geoid's height above the ellipsoid N (metres), or a point's velocity east, north and up (millimetres a year),
//! read bilinearly between the nodes as PROJ's `vgridshift` and `deformation` read theirs. An orthometric height is
//! the ellipsoidal one less N; a point at another epoch is moved by its velocity times the years between, in
//! Earth-centred coordinates as PROJ's deformation moves it. GTX files are read here; a GeoTIFF's pixels are given
//! by the caller (the formats core reads TIFF) through [`NodeGrid::new`]. The grids are kept by their files'
//! SHA-256 for the transforms, as the NTv2 grids are. The reference is PROJ itself
//! (`scripts/fixtures/geoid_cases.py`).

use std::sync::{Arc, Mutex};

use super::System;
use super::Unreached;
use super::datum::{from_geocentric, to_geocentric};
use crate::jsmath::{PI, js_max, js_min};
use crate::vec2::Vec2;

/// The largest grid file read (1 GiB: EGM2008 at 1′ is 933 MB as GTX).
pub const MAX_BYTES: usize = 1 << 30;

/// Why a file is not read as a grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeoidError {
    TooLarge,
    /// Not a GTX file (its header's numbers say no grid).
    NotGtx,
    /// The origin, spacing or size is not a grid of latitude and longitude.
    Extent,
    /// The file ends before its nodes do.
    Truncated,
    /// A band count the kind does not take.
    Bands,
}

impl GeoidError {
    pub fn as_str(self) -> &'static str {
        match self {
            GeoidError::TooLarge => "tooLarge",
            GeoidError::NotGtx => "notGtx",
            GeoidError::Extent => "extent",
            GeoidError::Truncated => "truncated",
            GeoidError::Bands => "bands",
        }
    }
}

/// A grid of nodes in latitude and longitude: the south-west node, the spacing, the size; each node's values
/// (`bands` of them, row by row from the south, west to east). A value the grid does not have is NaN.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeGrid {
    pub west: f64,
    pub south: f64,
    pub dlon: f64,
    pub dlat: f64,
    pub cols: usize,
    pub rows: usize,
    pub bands: usize,
    /// The extension PROJ reads the file by: `gtx`, else `tif`.
    pub ext: &'static str,
    values: Vec<f32>,
}

impl NodeGrid {
    /// A grid from its south-west node, spacing and size and its values (`bands` a node, rows from the south);
    /// `nodata` values become NaN.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        west: f64,
        south: f64,
        dlon: f64,
        dlat: f64,
        cols: usize,
        rows: usize,
        bands: usize,
        mut values: Vec<f32>,
        nodata: Option<f32>,
    ) -> Result<NodeGrid, GeoidError> {
        let fine = |v: f64| v.is_finite();
        if !(fine(west) && fine(south) && fine(dlon) && fine(dlat))
            || dlon <= 0.0
            || dlat <= 0.0
            || cols < 2
            || rows < 2
            || !(1..=3).contains(&bands)
            || south < -90.0 - 1e-9
            || south + dlat * (rows - 1) as f64 > 90.0 + 1e-9
            || dlon * (cols - 1) as f64 > 360.0 + 1e-9
        {
            return Err(GeoidError::Extent);
        }
        if values.len() != cols * rows * bands {
            return Err(GeoidError::Truncated);
        }
        for v in &mut values {
            if !v.is_finite() || nodata.is_some_and(|n| *v == n) {
                *v = f32::NAN;
            }
        }
        Ok(NodeGrid {
            west,
            south,
            dlon,
            dlat,
            cols,
            rows,
            bands,
            ext: "tif",
            values,
        })
    }

    /// Whether the grid goes round the Earth: its east edge one spacing from its west, wrapping.
    pub fn global(&self) -> bool {
        (self.dlon * self.cols as f64 - 360.0).abs() < self.dlon * 1e-6
    }

    /// West, south, east, north of its nodes (degrees).
    pub fn extent(&self) -> [f64; 4] {
        [
            self.west,
            self.south,
            self.west + self.dlon * (self.cols - 1) as f64,
            self.south + self.dlat * (self.rows - 1) as f64,
        ]
    }

    fn value(&self, col: usize, row: usize, band: usize) -> f64 {
        f64::from(self.values[(row * self.cols + col) * self.bands + band])
    }

    /// The bands' values at a longitude and latitude (degrees), bilinear between the four nodes round it; none
    /// outside the grid or by a node without a value.
    pub fn at(&self, lon: f64, lat: f64) -> Option<[f64; 3]> {
        if !(lon.is_finite() && lat.is_finite()) {
            return None;
        }
        let fy = (lat - self.south) / self.dlat;
        let last_row = (self.rows - 1) as f64;
        if !(-1e-9..=last_row + 1e-9).contains(&fy) {
            return None;
        }
        let fy = fy.clamp(0.0, last_row);
        let global = self.global();
        // The longitude brought to the grid's turn of the Earth.
        let mut x = lon - self.west;
        x -= 360.0 * libm::floor(x / 360.0);
        let mut fx = x / self.dlon;
        let last_col = (self.cols - 1) as f64;
        if !global {
            if fx > last_col + 1e-9 {
                // West of the grid, a turn round.
                let back = fx - 360.0 / self.dlon;
                if back < -1e-9 {
                    return None;
                }
                fx = js_max(back, 0.0);
            }
            fx = js_min(fx, last_col);
        }
        let (i0, j0) = (libm::floor(fx) as usize, libm::floor(fy) as usize);
        let (i1, j1) = if global {
            ((i0 + 1) % self.cols, (j0 + 1).min(self.rows - 1))
        } else {
            ((i0 + 1).min(self.cols - 1), (j0 + 1).min(self.rows - 1))
        };
        let (u, v) = (fx - i0 as f64, fy - j0 as f64);
        let mut out = [0.0; 3];
        for (b, o) in out.iter_mut().enumerate().take(self.bands) {
            let (a, c) = (self.value(i0, j0, b), self.value(i1, j0, b));
            let (d, e) = (self.value(i0, j1, b), self.value(i1, j1, b));
            let low = a + (c - a) * u;
            let high = d + (e - d) * u;
            *o = low + (high - low) * v;
            if !o.is_finite() {
                return None;
            }
        }
        Some(out)
    }
}

/// A GTX file (NOAA's vertical grid, PROJ's): a 40-byte big-endian header (the south-west node's latitude and
/// longitude, the spacing in latitude and longitude, all f64; rows and columns, i32), then f32 values row by row
/// from the south; −88.8888 is no value.
pub fn read_gtx(bytes: &[u8]) -> Result<NodeGrid, GeoidError> {
    if bytes.len() > MAX_BYTES {
        return Err(GeoidError::TooLarge);
    }
    if bytes.len() < 40 {
        return Err(GeoidError::NotGtx);
    }
    let f = |at: usize| f64::from_be_bytes(bytes[at..at + 8].try_into().unwrap_or([0; 8]));
    let i = |at: usize| i32::from_be_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4]));
    let (south, mut west, dlat, dlon) = (f(0), f(8), f(16), f(24));
    let (rows, cols) = (i(32), i(36));
    if rows < 2 || cols < 2 || !(dlat > 0.0 && dlon > 0.0) {
        return Err(GeoidError::NotGtx);
    }
    let (rows, cols) = (rows as usize, cols as usize);
    let count = rows.checked_mul(cols).ok_or(GeoidError::Extent)?;
    if bytes.len() < 40 + count * 4 {
        return Err(GeoidError::Truncated);
    }
    // GTX writes eastern longitudes past 180 (0–360); the grid keeps them from −180.
    if west >= 180.0 {
        west -= 360.0;
    }
    let values = bytes[40..40 + count * 4]
        .chunks_exact(4)
        .map(|b| f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let mut grid = NodeGrid::new(
        west,
        south,
        dlon,
        dlat,
        cols,
        rows,
        1,
        values,
        Some(-88.8888),
    )?;
    grid.ext = "gtx";
    Ok(grid)
}

static GRIDS: Mutex<Vec<(String, Arc<NodeGrid>)>> = Mutex::new(Vec::new());

/// Keeps `grid` under `id` (its file's SHA-256) for the transforms, the same id replaced.
pub fn register(id: &str, grid: NodeGrid) {
    if let Ok(mut all) = GRIDS.lock() {
        all.retain(|(k, _)| k != id);
        all.push((id.to_owned(), Arc::new(grid)));
    }
}

/// The grid kept under `id`.
pub fn get(id: &str) -> Option<Arc<NodeGrid>> {
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

/// What a side's heights are to the transform: heights above the system's ellipsoid, or orthometric heights above
/// a geoid model's surface (N read at the point's latitude and longitude on the side's own datum).
#[derive(Clone, Copy, Debug)]
pub enum Vertical<'a> {
    Ellipsoidal,
    Geoid(&'a NodeGrid),
}

impl Vertical<'_> {
    /// N where the point is (0 for an ellipsoidal height); none outside the geoid's grid.
    fn n(&self, lat: f64, lon: f64) -> Option<f64> {
        match self {
            Vertical::Ellipsoidal => Some(0.0),
            Vertical::Geoid(g) => g.at(lon, lat).map(|v| v[0]),
        }
    }
}

/// How a point's third coordinate rides a transformation: aside, the datum shifted at height 0 (heights that are
/// neither ellipsoidal nor tied to a geoid, and points without one), or through the shift as an ellipsoidal height,
/// each side's orthometric height made ellipsoidal by its geoid (h = H + N) and back (H = h − N).
#[derive(Clone, Copy, Debug)]
pub enum Ride<'a> {
    Aside,
    Through {
        from: Vertical<'a>,
        to: Vertical<'a>,
    },
}

/// A point of `from` (its height `z`, when it has one) in `to`, in PROJ's pipeline's order (docs/adr/0631): the
/// source's geoid, the velocity grid's `years` (`velocity`), the datum shift, the target's geoid. A height riding
/// aside stays as it is. Outside a geoid or the velocity grid: `OutsideGrid`.
#[allow(clippy::too_many_arguments)]
pub fn transform_point(
    from: &System,
    to: &System,
    p: Vec2,
    z: Option<f64>,
    ride: Ride<'_>,
    velocity: Option<(&NodeGrid, f64)>,
    choices: &[super::Choice],
) -> Result<(Vec2, Option<f64>), Unreached> {
    let path = super::operation::way(from, to, choices)?;
    let ellipsoid = from.datum().ellipsoid();
    let (lat, lon) = from.unproject(p).ok_or(Unreached::Outside)?;
    match (ride, z) {
        (Ride::Through { from: fv, to: tv }, Some(z)) => {
            let h = z + fv.n(lat, lon).ok_or(Unreached::OutsideGrid)?;
            let (lat, lon, h) = match velocity {
                Some((g, years)) => move_geographic(ellipsoid, g, lat, lon, h, years)?,
                None => (lat, lon, h),
            };
            let (lat, lon, h) = path
                .run_height(ellipsoid, lat, lon, h)
                .ok_or(Unreached::OutsideGrid)?;
            let h = h - tv.n(lat, lon).ok_or(Unreached::OutsideGrid)?;
            Ok((to.project(lat, lon).ok_or(Unreached::Outside)?, Some(h)))
        }
        _ => {
            // A height aside stays: the velocity moves the point at height 0 (PROJ's pipeline keeps it with
            // `push`/`pop` round the velocity and the shift).
            let (lat, lon) = match velocity {
                Some((g, years)) => {
                    let (la, lo, _) = move_geographic(ellipsoid, g, lat, lon, 0.0, years)?;
                    (la, lo)
                }
                None => (lat, lon),
            };
            let (lat, lon) = path
                .run(ellipsoid, lat, lon)
                .ok_or(Unreached::OutsideGrid)?;
            Ok((to.project(lat, lon).ok_or(Unreached::Outside)?, z))
        }
    }
}

/// A latitude, longitude and ellipsoidal height moved `years` along a velocity grid (bands east, north, up in
/// millimetres a year): the velocity read there, turned from east, north, up into Earth-centred
/// coordinates and added, as PROJ's `deformation` adds it.
fn move_geographic(
    ellipsoid: (f64, f64),
    grid: &NodeGrid,
    lat: f64,
    lon: f64,
    height: f64,
    years: f64,
) -> Result<(f64, f64, f64), Unreached> {
    let v = grid.at(lon, lat).ok_or(Unreached::OutsideGrid)?;
    let (e, n, u) = (
        v[0] / 1000.0 * years,
        v[1] / 1000.0 * years,
        if grid.bands >= 3 {
            v[2] / 1000.0 * years
        } else {
            0.0
        },
    );
    let x = to_geocentric(ellipsoid, lat, lon, height);
    let rad = PI / 180.0;
    let (sp, cp) = (libm::sin(lat * rad), libm::cos(lat * rad));
    let (sl, cl) = (libm::sin(lon * rad), libm::cos(lon * rad));
    let moved = [
        x[0] - sl * e - sp * cl * n + cp * cl * u,
        x[1] + cl * e - sp * sl * n + cp * sl * u,
        x[2] + cp * n + sp * u,
    ];
    Ok(from_geocentric(ellipsoid, moved))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gtx(south: f64, west: f64, d: f64, rows: i32, cols: i32, values: &[f32]) -> Vec<u8> {
        let mut out = Vec::new();
        for v in [south, west, d, d] {
            out.extend_from_slice(&v.to_be_bytes());
        }
        out.extend_from_slice(&rows.to_be_bytes());
        out.extend_from_slice(&cols.to_be_bytes());
        for v in values {
            out.extend_from_slice(&v.to_be_bytes());
        }
        out
    }

    #[test]
    fn a_gtx_is_read_bilinearly_and_hostile_files_say_why() {
        let g = read_gtx(&gtx(39.0, 35.0, 1.0, 2, 2, &[10.0, 12.0, 14.0, 20.0])).expect("a grid");
        assert_eq!(g.extent(), [35.0, 39.0, 36.0, 40.0]);
        assert_eq!(g.at(35.0, 39.0), Some([10.0, 0.0, 0.0]));
        assert_eq!(g.at(35.5, 39.5), Some([14.0, 0.0, 0.0]));
        assert_eq!(g.at(36.0, 40.0), Some([20.0, 0.0, 0.0]));
        assert_eq!(g.at(36.1, 39.5), None);
        // Eastern longitudes past 180, as GTX writes them.
        let east = read_gtx(&gtx(39.0, 395.0, 1.0, 2, 2, &[1.0; 4])).expect("a grid");
        assert_eq!(east.west, 35.0);
        // No value: −88.8888.
        let hole =
            read_gtx(&gtx(39.0, 35.0, 1.0, 2, 2, &[1.0, -88.8888, 1.0, 1.0])).expect("a grid");
        assert_eq!(hole.at(35.5, 39.5), None);
        assert_eq!(read_gtx(&[0; 10]).err(), Some(GeoidError::NotGtx));
        assert_eq!(
            read_gtx(&gtx(39.0, 35.0, 1.0, 2, 2, &[1.0; 3])).err(),
            Some(GeoidError::Truncated)
        );
        assert_eq!(
            read_gtx(&gtx(39.0, 35.0, 0.0, 2, 2, &[1.0; 4])).err(),
            Some(GeoidError::NotGtx)
        );
    }

    #[test]
    fn a_global_grid_wraps_round() {
        // Four columns of 90°: 0, 90, 180, 270 (−90); the last cell runs back to the first.
        let g = NodeGrid::new(
            0.0,
            -10.0,
            90.0,
            20.0,
            4,
            2,
            1,
            vec![0.0, 1.0, 2.0, 3.0, 0.0, 1.0, 2.0, 3.0],
            None,
        )
        .expect("a grid");
        assert!(g.global());
        assert_eq!(g.at(315.0, 0.0), Some([1.5, 0.0, 0.0]));
        assert_eq!(g.at(-45.0, 0.0), Some([1.5, 0.0, 0.0]));
        assert_eq!(g.at(45.0, 0.0), Some([0.5, 0.0, 0.0]));
    }

    /// The engine's times (docs/adr/0631 Doğrulama): a global geoid at 2.5′ (EGM2008's spacing, 37 million nodes) read
    /// from GTX; 100 000 points of TUREF / TM36 into its orthometric heights, without and with the geoid, and carried
    /// along a velocity grid.
    ///
    /// `cargo test --release -p kentos-geometry-core --lib crs::geoid::tests::timing -- --ignored --nocapture`
    #[test]
    #[ignore = "a timing, run on purpose"]
    fn timing() {
        use crate::crs::{Datum, System};
        use std::time::Instant;
        let ms = |t: Instant| t.elapsed().as_secs_f64() * 1e3;
        let (cols, rows) = (8640usize, 4321usize);
        let d = 2.5 / 60.0;
        let mut bytes = Vec::with_capacity(40 + cols * rows * 4);
        for v in [-90.0f64, 0.0, d, d] {
            bytes.extend_from_slice(&v.to_be_bytes());
        }
        bytes.extend_from_slice(&(rows as i32).to_be_bytes());
        bytes.extend_from_slice(&(cols as i32).to_be_bytes());
        for j in 0..rows {
            for i in 0..cols {
                let v = 20.0 * libm::sin(i as f64 * 0.001) + 10.0 * libm::cos(j as f64 * 0.002);
                bytes.extend_from_slice(&(v as f32).to_be_bytes());
            }
        }
        let t = Instant::now();
        let g = read_gtx(&bytes).expect("a grid");
        println!(
            "küresel 2,5′ geoit GTX okuma: {:.0} ms ({} MB)",
            ms(t),
            bytes.len() >> 20
        );
        let tm = System::Tm {
            datum: Datum::Turef,
            central_meridian: 36.0,
            scale_factor: 1.0,
            false_easting: 500_000.0,
            false_northing: 0.0,
            latitude_of_origin: None,
        };
        let pts: Vec<Vec2> = (0..100_000)
            .map(|k| {
                Vec2::new(
                    450_000.0 + f64::from(k % 400) * 200.0,
                    4_350_000.0 + f64::from(k / 400) * 600.0,
                )
            })
            .collect();
        fn through(to: Vertical<'_>) -> Ride<'_> {
            Ride::Through {
                from: Vertical::Ellipsoidal,
                to,
            }
        }
        for (name, ride) in [
            ("elipsoit → elipsoit", through(Vertical::Ellipsoidal)),
            (
                "elipsoit → ortometrik (geoit)",
                through(Vertical::Geoid(&g)),
            ),
        ] {
            let t = Instant::now();
            let n = pts
                .iter()
                .filter(|p| transform_point(&tm, &tm, **p, Some(1000.0), ride, None, &[]).is_ok())
                .count();
            println!("100 000 nokta, {name}: {:.1} ms ({n})", ms(t));
        }
        let vel = NodeGrid::new(
            25.0,
            35.0,
            0.5,
            0.5,
            43,
            17,
            3,
            vec![-20.0; 43 * 17 * 3],
            None,
        )
        .expect("a velocity grid");
        let t = Instant::now();
        let n = pts
            .iter()
            .filter(|p| {
                transform_point(
                    &tm,
                    &tm,
                    **p,
                    Some(1000.0),
                    through(Vertical::Ellipsoidal),
                    Some((&vel, 21.5)),
                    &[],
                )
                .is_ok()
            })
            .count();
        println!(
            "100 000 nokta, hız ızgarasıyla 21,5 yıl: {:.1} ms ({n})",
            ms(t)
        );
    }
}
