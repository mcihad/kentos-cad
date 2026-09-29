//! A pafta's true frame. A map sheet record (NCZ type 11) keeps two points
//! and the sheet's name, and the two points are the bounding box of the
//! sheet's geographic cell (1/1000: 22.5″ × 22.5″) projected into the file's
//! TM zone. There the cell is a quadrilateral turned by the meridian
//! convergence (0.64° at 38° E in TM39): drawn as its box, as the NCZ Reader
//! plugin's `_parse_map_sheet` draws it, each sheet overlaps its neighbours
//! (5.5 m north and 7 m east on Suşehri's PINDEX_1000).
//!
//! The cell is found back from its box: the latitudes and longitudes whose
//! projected corners span that box, by Newton's method. When they lie on a
//! sheet grid, the cell's four corners are the frame. The zone and the
//! ellipsoid come from the file's MPROJ block, never from the drawing's
//! system. A file that names no transverse Mercator zone, and a box that is no
//! grid cell's (a local sheet), keep the box; the report says how many and why.
//!
//! The transverse Mercator is PROJ's `tmerc` (Poder and Engsager's
//! sixth-order Krüger series, `src/projections/tmerc.cpp`) in libm, so both
//! targets reach the same bits (CLAUDE.md §23.4); the tests hold PROJ's
//! numbers.

use libm::{asinh, atan2, cos, exp, hypot, sin, sinh};

use crate::format::Header;

/// An ellipsoid: semi-major axis (metres) and inverse flattening.
#[derive(Clone, Copy, Debug)]
struct Ellipsoid {
    a: f64,
    rf: f64,
}

const WGS84: Ellipsoid = Ellipsoid { a: 6_378_137.0, rf: 298.257_223_563 };
const GRS80: Ellipsoid = Ellipsoid { a: 6_378_137.0, rf: 298.257_222_101 };
/// International 1924 (Hayford), ED50's.
const INTERNATIONAL: Ellipsoid = Ellipsoid { a: 6_378_388.0, rf: 297.0 };

/// Both kinds of zone: 500 000 m false easting, no false northing.
const FALSE_EASTING: f64 = 500_000.0;

/// A transverse Mercator whose origin is on the equator at its central meridian.
#[derive(Clone, Debug)]
pub(crate) struct Tm {
    /// The central meridian, degrees.
    lon0: f64,
    a: f64,
    /// The meridian quadrant normalised by `a`, the scale on the central meridian in it.
    qn: f64,
    /// Geodetic to Gaussian latitude, and back.
    cbg: [f64; 6],
    cgb: [f64; 6],
    /// Gaussian to ellipsoidal northing and easting, and back.
    gtu: [f64; 6],
    utg: [f64; 6],
}

impl Tm {
    fn new(lon0: f64, k0: f64, ellipsoid: Ellipsoid) -> Self {
        let f = 1.0 / ellipsoid.rf;
        // The third flattening; the series are PROJ's, term for term.
        let n = f / (2.0 - f);
        let n2 = n * n;
        let n3 = n2 * n;
        let n4 = n3 * n;
        let n5 = n4 * n;
        let n6 = n5 * n;
        Self {
            lon0,
            a: ellipsoid.a,
            qn: k0 / (1.0 + n) * (1.0 + n2 * (1.0 / 4.0 + n2 * (1.0 / 64.0 + n2 / 256.0))),
            cgb: [
                n * (2.0 + n * (-2.0 / 3.0 + n * (-2.0 + n * (116.0 / 45.0 + n * (26.0 / 45.0 + n * (-2854.0 / 675.0)))))),
                n2 * (7.0 / 3.0 + n * (-8.0 / 5.0 + n * (-227.0 / 45.0 + n * (2704.0 / 315.0 + n * (2323.0 / 945.0))))),
                n3 * (56.0 / 15.0 + n * (-136.0 / 35.0 + n * (-1262.0 / 105.0 + n * (73814.0 / 2835.0)))),
                n4 * (4279.0 / 630.0 + n * (-332.0 / 35.0 + n * (-399572.0 / 14175.0))),
                n5 * (4174.0 / 315.0 + n * (-144838.0 / 6237.0)),
                n6 * (601676.0 / 22275.0),
            ],
            cbg: [
                n * (-2.0 + n * (2.0 / 3.0 + n * (4.0 / 3.0 + n * (-82.0 / 45.0 + n * (32.0 / 45.0 + n * (4642.0 / 4725.0)))))),
                n2 * (5.0 / 3.0 + n * (-16.0 / 15.0 + n * (-13.0 / 9.0 + n * (904.0 / 315.0 + n * (-1522.0 / 945.0))))),
                n3 * (-26.0 / 15.0 + n * (34.0 / 21.0 + n * (8.0 / 5.0 + n * (-12686.0 / 2835.0)))),
                n4 * (1237.0 / 630.0 + n * (-12.0 / 5.0 + n * (-24832.0 / 14175.0))),
                n5 * (-734.0 / 315.0 + n * (109598.0 / 31185.0)),
                n6 * (444337.0 / 155925.0),
            ],
            utg: [
                n * (-0.5 + n * (2.0 / 3.0 + n * (-37.0 / 96.0 + n * (1.0 / 360.0 + n * (81.0 / 512.0 + n * (-96199.0 / 604800.0)))))),
                n2 * (-1.0 / 48.0 + n * (-1.0 / 15.0 + n * (437.0 / 1440.0 + n * (-46.0 / 105.0 + n * (1118711.0 / 3870720.0))))),
                n3 * (-17.0 / 480.0 + n * (37.0 / 840.0 + n * (209.0 / 4480.0 + n * (-5569.0 / 90720.0)))),
                n4 * (-4397.0 / 161280.0 + n * (11.0 / 504.0 + n * (830251.0 / 7257600.0))),
                n5 * (-4583.0 / 161280.0 + n * (108847.0 / 3991680.0)),
                n6 * (-20648693.0 / 638668800.0),
            ],
            gtu: [
                n * (0.5 + n * (-2.0 / 3.0 + n * (5.0 / 16.0 + n * (41.0 / 180.0 + n * (-127.0 / 288.0 + n * (7891.0 / 37800.0)))))),
                n2 * (13.0 / 48.0 + n * (-3.0 / 5.0 + n * (557.0 / 1440.0 + n * (281.0 / 630.0 + n * (-1983433.0 / 1935360.0))))),
                n3 * (61.0 / 240.0 + n * (-103.0 / 140.0 + n * (15061.0 / 26880.0 + n * (167603.0 / 181440.0)))),
                n4 * (49561.0 / 161280.0 + n * (-179.0 / 168.0 + n * (6601661.0 / 7257600.0))),
                n5 * (34729.0 / 80640.0 + n * (-3418889.0 / 1995840.0)),
                n6 * (212378941.0 / 319334400.0),
            ],
        }
    }

    /// Latitude and longitude, degrees, to easting and northing, metres.
    pub(crate) fn forward(&self, lat: f64, lon: f64) -> [f64; 2] {
        let phi = lat.to_radians();
        let lam = lon.to_radians() - self.lon0.to_radians();
        // Geodetic latitude to Gaussian, then to the complementary spherical one.
        let cn = gatg(&self.cbg, phi, cos(2.0 * phi), sin(2.0 * phi));
        let (sin_cn, cos_cn) = (sin(cn), cos(cn));
        let (sin_ce, cos_ce) = (sin(lam), cos(lam));
        let cos_cn_cos_ce = cos_cn * cos_ce;
        let cn = atan2(sin_cn, cos_cn_cos_ce);
        let inv_denom_tan_ce = 1.0 / hypot(sin_cn, cos_cn_cos_ce);
        let tan_ce = sin_ce * cos_cn * inv_denom_tan_ce;
        let ce = asinh(tan_ce);
        // The sines and cosines of 2Cn and 2Ce from the values at hand, as PROJ has them.
        let two_inv_denom_tan_ce = 2.0 * inv_denom_tan_ce;
        let two_inv_denom_tan_ce_square = two_inv_denom_tan_ce * inv_denom_tan_ce;
        let tmp_r = cos_cn_cos_ce * two_inv_denom_tan_ce_square;
        let sin_arg_r = sin_cn * tmp_r;
        let cos_arg_r = cos_cn_cos_ce * tmp_r - 1.0;
        let sinh_arg_i = tan_ce * two_inv_denom_tan_ce;
        let cosh_arg_i = two_inv_denom_tan_ce_square - 1.0;
        let (dcn, dce) = clens(&self.gtu, sin_arg_r, cos_arg_r, sinh_arg_i, cosh_arg_i);
        let (cn, ce) = (cn + dcn, ce + dce);
        [self.a * (self.qn * ce) + FALSE_EASTING, self.a * (self.qn * cn)]
    }

    /// Easting and northing, metres, to latitude and longitude, degrees.
    pub(crate) fn inverse(&self, x: f64, y: f64) -> [f64; 2] {
        let cn = y / self.a / self.qn;
        let ce = (x - FALSE_EASTING) / self.a / self.qn;
        let (sin_arg_r, cos_arg_r) = (sin(2.0 * cn), cos(2.0 * cn));
        let exp_2_ce = exp(2.0 * ce);
        let half_inv_exp_2_ce = 0.5 / exp_2_ce;
        let sinh_arg_i = 0.5 * exp_2_ce - half_inv_exp_2_ce;
        let cosh_arg_i = 0.5 * exp_2_ce + half_inv_exp_2_ce;
        let (dcn, dce) = clens(&self.utg, sin_arg_r, cos_arg_r, sinh_arg_i, cosh_arg_i);
        let (cn, ce) = (cn + dcn, ce + dce);
        // The complementary spherical latitude to Gaussian, then to geodetic.
        let (sin_cn, cos_cn) = (sin(cn), cos(cn));
        let sinh_ce = sinh(ce);
        let lam = atan2(sinh_ce, cos_cn);
        let modulus_ce = hypot(sinh_ce, cos_cn);
        let cn = atan2(sin_cn, modulus_ce);
        let tmp = 2.0 * modulus_ce / (sinh_ce * sinh_ce + 1.0);
        let sin_2_cn = sin_cn * tmp;
        let cos_2_cn = tmp * modulus_ce - 1.0;
        let phi = gatg(&self.cgb, cn, cos_2_cn, sin_2_cn);
        [phi.to_degrees(), (lam + self.lon0.to_radians()).to_degrees()]
    }

    /// The box of a cell's projection, [least easting, least northing, greatest easting,
    /// greatest northing]: its corners' and, when it spans the central meridian, its
    /// parallels' points on it (a parallel's northing is smallest there).
    fn extent(&self, [south, north, west, east]: Cell) -> [f64; 4] {
        let points = [(south, west), (south, east), (north, east), (north, west), (south, self.lon0), (north, self.lon0)];
        let count = if west < self.lon0 && self.lon0 < east { 6 } else { 4 };
        let mut out = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for &(lat, lon) in &points[..count] {
            let [x, y] = self.forward(lat, lon);
            out = [out[0].min(x), out[1].min(y), out[2].max(x), out[3].max(y)];
        }
        out
    }
}

/// B plus Σ c[k]·sin(2(k+1)B), by Clenshaw's summation (PROJ's `gatg`).
fn gatg(c: &[f64; 6], b: f64, cos_2b: f64, sin_2b: f64) -> f64 {
    let two_cos_2b = 2.0 * cos_2b;
    let (mut h, mut h1, mut h2) = (0.0, c[5], 0.0);
    for &ck in c[..5].iter().rev() {
        h = -h2 + two_cos_2b * h1 + ck;
        h2 = h1;
        h1 = h;
    }
    b + h * sin_2b
}

/// Σ c[k]·sin(2(k+1)(Cn + i·Ce)), real and imaginary parts, by complex Clenshaw summation
/// (PROJ's `clenS`), given the sine and cosine of 2Cn and the hyperbolic ones of 2Ce.
fn clens(c: &[f64; 6], sin_r: f64, cos_r: f64, sinh_i: f64, cosh_i: f64) -> (f64, f64) {
    let r = 2.0 * cos_r * cosh_i;
    let i = -2.0 * sin_r * sinh_i;
    let (mut hr, mut hi, mut hr1, mut hi1) = (c[5], 0.0, 0.0, 0.0);
    for &ck in c[..5].iter().rev() {
        let (hr2, hi2) = (hr1, hi1);
        hr1 = hr;
        hi1 = hi;
        hr = -hr2 + r * hr1 - i * hi1 + ck;
        hi = -hi2 + i * hr1 + r * hi1;
    }
    let r = sin_r * cosh_i;
    let i = cos_r * sinh_i;
    (r * hr - i * hi, r * hi + i * hr)
}

/// A cell's parallels and meridians, degrees: south, north, west, east.
type Cell = [f64; 4];

/// The zone a file's MPROJ block names, and what the report calls it.
#[derive(Clone, Debug)]
pub(crate) struct Zone {
    pub tm: Tm,
    /// “ITRF TM39”, “ED50 UTM 37”.
    pub name: String,
}

/// Why a sheet keeps the box the file stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kept {
    /// The file has no MPROJ block.
    Unsaid,
    /// Its MPROJ names no transverse Mercator zone.
    NotTm,
    /// Its datum's ellipsoid is not known.
    Datum,
    /// No sheet grid cell projects to the box: a local sheet.
    NoCell,
}

impl Kept {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Kept::Unsaid => "dosya koordinat sistemini (MPROJ) bildirmiyor",
            Kept::NotTm => "dosyanın bildirdiği sistem bir TM ya da UTM dilimi değil",
            Kept::Datum => "dosyanın bildirdiği datumun elipsoidi bilinmiyor",
            Kept::NoCell => "kutusu bir enlem-boylam paftasının izdüşümüne uymuyor (yerel pafta)",
        }
    }
}

/// The zone of the file's MPROJ block: a 3° zone's byte is its central meridian (scale 1),
/// a 6° zone's is its UTM number (central meridian 6·zone − 183°, scale 0.9996).
pub(crate) fn zone(h: &Header) -> Result<Zone, Kept> {
    if !h.mproj {
        return Err(Kept::Unsaid);
    }
    let zone = f64::from(h.zone);
    let (lon0, k0, kind) = match h.projection {
        3 if h.zone <= 180 => (zone, 1.0, format!("TM{}", h.zone)),
        2 if (1..=60).contains(&h.zone) => (6.0 * zone - 183.0, 0.9996, format!("UTM {}", h.zone)),
        _ => return Err(Kept::NotTm),
    };
    let (ellipsoid, datum) = match h.datum {
        0 => (WGS84, "WGS-84"),
        1 => (GRS80, "ITRF"),
        4 => (INTERNATIONAL, "ED50"),
        254 => (INTERNATIONAL, "ED50 (HGK)"),
        _ => return Err(Kept::Datum),
    };
    Ok(Zone {
        tm: Tm::new(lon0, k0, ellipsoid),
        name: format!("{datum} {kind}"),
    })
}

/// Newton's steps at most.
const STEPS: usize = 8;
/// Degrees: the step of the Jacobian's differences, about a centimetre.
const STEP: f64 = 1e-7;
/// Metres: a cell this close to the box needs no further step.
const CLOSE: f64 = 1e-7;
/// Metres: a cell further than this from the box is not its cell.
const RESIDUAL: f64 = 0.01;
/// A cell lies on a grid when its south-west corner is a whole number of cells from the
/// equator and the prime meridian, within this.
const WHOLE: f64 = 1e-3;

/// The frame of the sheet whose box the file stores, [least easting, least northing,
/// greatest easting, greatest northing]: the corners of its cell, south-west, south-east,
/// north-east, north-west (counter-clockwise), each to the millimetre; none when no grid
/// cell's projection is that box.
pub(crate) fn frame(tm: &Tm, target: [f64; 4]) -> Option<[[f64; 2]; 4]> {
    let [south, north, west, east] = cell(tm, target)?;
    // A local sheet's box has a cell too; only a grid's cell is a pafta's.
    let whole = |origin: f64, size: f64| {
        let cells = origin / size;
        (cells - cells.round()).abs() <= WHOLE
    };
    if !(north > south && east > west && whole(south, north - south) && whole(west, east - west)) {
        return None;
    }
    // To the millimetre once, so a corner that neighbouring sheets share is one point.
    let corner = |lat: f64, lon: f64| tm.forward(lat, lon).map(|v| (v * 1000.0).round() / 1000.0);
    Some([corner(south, west), corner(south, east), corner(north, east), corner(north, west)])
}

/// The cell whose projection spans `target`, by Newton's method from the box's corners taken
/// back; none when no cell comes within a centimetre of it.
fn cell(tm: &Tm, target: [f64; 4]) -> Option<Cell> {
    let [least_x, least_y, most_x, most_y] = target;
    if !(target.iter().all(|v| v.is_finite()) && most_x > least_x && most_y > least_y) {
        return None;
    }
    let [south, west] = tm.inverse(least_x, least_y);
    let [north, east] = tm.inverse(most_x, most_y);
    let mut cell: Cell = [south, north, west, east];
    for _ in 0..STEPS {
        let at = tm.extent(cell);
        let miss: [f64; 4] = std::array::from_fn(|i| at[i] - target[i]);
        if miss.iter().all(|d| d.abs() <= CLOSE) {
            break;
        }
        let mut jacobian = [[0.0; 4]; 4];
        for j in 0..4 {
            let mut moved = cell;
            moved[j] += STEP;
            let h = moved[j] - cell[j];
            let there = tm.extent(moved);
            for (row, (t, a)) in jacobian.iter_mut().zip(there.iter().zip(at)) {
                row[j] = (t - a) / h;
            }
        }
        let step = solve(jacobian, miss)?;
        for (c, d) in cell.iter_mut().zip(step) {
            *c -= d;
        }
    }
    let at = tm.extent(cell);
    (0..4).all(|i| (at[i] - target[i]).abs() <= RESIDUAL).then_some(cell)
}

/// `x` with `m·x = b`, by Gaussian elimination with partial pivoting; none when `m` is singular.
fn solve(mut m: [[f64; 4]; 4], mut b: [f64; 4]) -> Option<[f64; 4]> {
    for col in 0..4 {
        let pivot = (col..4).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
        if !(m[pivot][col].abs() > 0.0) {
            return None;
        }
        m.swap(col, pivot);
        b.swap(col, pivot);
        let top = m[col];
        for row in col + 1..4 {
            let f = m[row][col] / top[col];
            for (v, t) in m[row].iter_mut().zip(top).skip(col) {
                *v -= f * t;
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = [0.0; 4];
    for row in (0..4).rev() {
        let rest: f64 = (row + 1..4).map(|k| m[row][k] * x[k]).sum();
        x[row] = (b[row] - rest) / m[row][row];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tm39() -> Tm {
        Tm::new(39.0, 1.0, GRS80)
    }

    #[test]
    fn the_projection_gives_projs_numbers() {
        // PROJ 9.7: +proj=tmerc +lon_0=39 +k=1 +x_0=500000 +ellps=GRS80, and the others as named.
        let cases: [(Tm, f64, f64, f64, f64); 9] = [
            (tm39(), 40.1875, 38.08125, 421_758.833_548_900_55, 4_450_753.176_219_376),
            (tm39(), 40.1875, 38.0875, 422_291.094_032_371_3, 4_450_747.686_980_942),
            (tm39(), 40.19375, 38.0875, 422_298.227_374_088_9, 4_451_441.691_247_378_5),
            (tm39(), 40.19375, 38.08125, 421_766.015_759_647_8, 4_451_447.180_690_602),
            // Three degrees and more from the central meridian.
            (tm39(), 40.5, 36.0, 245_678.311_379_306_37, 4_489_375.005_375_772),
            (tm39(), 37.25, 42.0, 766_191.009_870_417_4, 4_128_476.387_678_815_5),
            // +proj=utm +zone=37 +ellps=intl; +proj=utm +zone=36 +ellps=WGS84; TM30 on intl.
            (Tm::new(39.0, 0.9996, INTERNATIONAL), 39.7, 38.2, 431_410.829_302_363_8, 4_394_841.851_857_114),
            (Tm::new(33.0, 0.9996, WGS84), 41.1, 30.9, 323_648.549_192_303_9, 4_551_983.130_188_179),
            (Tm::new(30.0, 1.0, INTERNATIONAL), 36.7, 28.4, 357_012.053_552_146, 4_064_477.800_635_579_5),
        ];
        for (tm, lat, lon, x, y) in cases {
            let [ex, ny] = tm.forward(lat, lon);
            assert!((ex - x).abs() < 1e-6 && (ny - y).abs() < 1e-6, "({lat}, {lon}): {ex} {ny}, PROJ {x} {y}");
            let [la, lo] = tm.inverse(x, y);
            assert!((la - lat).abs() < 1e-11 && (lo - lon).abs() < 1e-11, "({x}, {y}): {la} {lo}");
        }
    }

    /// The four 1/1000 sheets of a 2×2 block in TM39 starting at 40°11′15″ N 38°04′52.5″ E:
    /// the box each stores, K (northing) and D (easting) as the file keeps them, least to
    /// greatest, and the frame expected, (D, K) in millimetres, south-west first.
    type Sheet = (&'static str, [f64; 4], [(i64, i64); 4]);
    const BLOCK: [Sheet; 4] = [
        (
            "GB",
            [4_450_747.686_980_942, 421_758.833_548_900_6, 4_451_447.180_690_602, 422_298.227_374_088_9],
            [(421_758_834, 4_450_753_176), (422_291_094, 4_450_747_687), (422_298_227, 4_451_441_691), (421_766_016, 4_451_447_181)],
        ),
        (
            "GD",
            [4_450_742.235_219_674, 422_291.094_032_371_3, 4_451_441.691_247_378_5, 422_830.438_832_118_9],
            [(422_291_094, 4_450_747_687), (422_823_354, 4_450_742_235), (422_830_439, 4_451_436_239), (422_298_227, 4_451_441_691)],
        ),
        (
            "KB",
            [4_451_441.691_247_378_5, 421_766.015_759_647_8, 4_452_141.185_894_171, 422_305.361_644_388_1],
            [(421_766_016, 4_451_447_181), (422_298_227, 4_451_441_691), (422_305_362, 4_452_135_696), (421_773_199, 4_452_141_186)],
        ),
        (
            "KD",
            [4_451_436.239_282_718, 422_298.227_374_088_9, 4_452_135.696_246_419, 422_837.524_227_243_6],
            [(422_298_227, 4_451_441_691), (422_830_439, 4_451_436_239), (422_837_524, 4_452_130_244), (422_305_362, 4_452_135_696)],
        ),
    ];

    fn millimetres(frame: [[f64; 2]; 4]) -> Vec<(i64, i64)> {
        frame.iter().map(|&[x, y]| ((x * 1000.0).round() as i64, (y * 1000.0).round() as i64)).collect()
    }

    #[test]
    fn a_sheets_box_gives_back_its_cells_turned_frame() {
        let tm = tm39();
        let mut frames = Vec::new();
        for (name, [k1, d1, k2, d2], expected) in BLOCK {
            let frame = frame(&tm, [d1, k1, d2, k2]).unwrap_or_else(|| panic!("{name}: no frame"));
            assert_eq!(millimetres(frame), expected, "{name}");
            // Each coordinate is the millimetre itself, not a value near it.
            for v in frame.iter().flatten() {
                assert_eq!(*v, (v * 1000.0).round() / 1000.0, "{name}");
            }
            frames.push(frame);
        }
        // The block's middle corner is one point of all four sheets.
        let middle = [422_298.227, 4_451_441.691];
        assert_eq!([frames[0][2], frames[1][3], frames[2][1], frames[3][0]], [middle; 4]);
    }

    #[test]
    fn a_cell_across_the_central_meridian_is_found_by_its_parallels_lowest_point() {
        // 22.5″ cells across 39° E, ending on it and starting on it; in the south of a 6° zone,
        // across it and three degrees off it.
        let tm = tm39();
        let utm = Tm::new(39.0, 0.9996, INTERNATIONAL);
        for (tm, south, west) in [
            (&tm, 40.1875, 38.996_875),
            (&tm, 40.1875, 38.993_75),
            (&tm, 40.1875, 39.0),
            (&utm, 36.0, 38.998),
            (&utm, 36.0, 36.0),
        ] {
            let want = [south, south + 0.006_25, west, west + 0.006_25];
            let got = cell(tm, tm.extent(want)).unwrap_or_else(|| panic!("({south}, {west}): no cell"));
            for (g, w) in got.iter().zip(want) {
                assert!((g - w).abs() < 1e-10, "({south}, {west}): {got:?}");
            }
        }
    }

    #[test]
    fn a_local_sheet_keeps_its_box() {
        // A box of round local coordinates is some cell's, but not a grid's.
        let tm = tm39();
        assert_eq!(frame(&tm, [421_400.0, 4_448_400.0, 421_940.0, 4_449_100.0]), None);
        // No size, and no number.
        assert_eq!(frame(&tm, [421_400.0, 4_448_400.0, 421_400.0, 4_449_100.0]), None);
        assert_eq!(frame(&tm, [f64::NAN, 4_448_400.0, 421_940.0, 4_449_100.0]), None);
    }

    #[test]
    fn the_zone_is_the_files_mproj() {
        let h = |mproj: bool, projection: u8, datum: u8, zone: u8| Header { mproj, projection, datum, zone, ..Header::default() };
        let name = |z: Result<Zone, Kept>| z.map(|z| (z.name, z.tm.lon0));
        assert_eq!(name(super::zone(&h(true, 3, 1, 39))), Ok(("ITRF TM39".to_owned(), 39.0)));
        assert_eq!(name(super::zone(&h(true, 2, 4, 37))), Ok(("ED50 UTM 37".to_owned(), 39.0)));
        assert_eq!(name(super::zone(&h(true, 3, 254, 36))), Ok(("ED50 (HGK) TM36".to_owned(), 36.0)));
        assert_eq!(name(super::zone(&h(true, 2, 0, 36))), Ok(("WGS-84 UTM 36".to_owned(), 33.0)));
        assert_eq!(name(super::zone(&h(false, 3, 1, 39))), Err(Kept::Unsaid));
        assert_eq!(name(super::zone(&h(true, 1, 1, 39))), Err(Kept::NotTm));
        assert_eq!(name(super::zone(&h(true, 2, 1, 0))), Err(Kept::NotTm));
        assert_eq!(name(super::zone(&h(true, 3, 7, 39))), Err(Kept::Datum));
    }
}
