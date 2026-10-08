//! A raster's band statistics for stretching (docs/adr/0204 §4): each band's
//! least and most value and its 2nd and 98th percentile, nodata and NaN left
//! out, from a level no wider than 1024 pixels (approximate, as GDAL's
//! quick statistics are). The p-th percentile of n sorted values is the one
//! at ⌊p · (n − 1) + ½⌋. 8 and 16-bit bands are counted, not sorted.

use kentos_contracts::{PERCENT_HIGH, PERCENT_LOW};
use serde::Serialize;

use super::Samples;
use super::source::Region;

/// The side of the level statistics are taken from, at most.
pub const STATS_SIDE: u32 = 1024;

/// One band's statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BandStats {
    /// Values counted.
    pub count: u64,
    pub min: f64,
    pub max: f64,
    /// The 2nd and 98th percentile.
    pub low: f64,
    pub high: f64,
}

/// Every band's statistics.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub bands: Vec<BandStats>,
}

impl Stats {
    /// Band `i`'s least and most value (0-based), none when it has no values.
    pub fn minmax(&self, i: usize) -> Option<(f64, f64)> {
        self.bands
            .get(i)
            .filter(|b| b.count > 0)
            .map(|b| (b.min, b.max))
    }

    /// Band `i`'s 2nd and 98th percentile.
    pub fn percent(&self, i: usize) -> Option<(f64, f64)> {
        self.bands
            .get(i)
            .filter(|b| b.count > 0)
            .map(|b| (b.low, b.high))
    }
}

/// The index of the p-th percentile among `n` sorted values.
pub fn rank(p: f64, n: u64) -> u64 {
    if n == 0 {
        0
    } else {
        (p * (n - 1) as f64 + 0.5).floor() as u64
    }
}

/// The statistics of a whole level's region.
pub fn of_region(region: &Region, nodata: Option<f64>) -> Stats {
    let bands = region.bands as usize;
    let n = region.width as usize * region.height as usize;
    let skip = |v: f64| v.is_nan() || nodata.is_some_and(|d| v == d);
    let mut out = Vec::with_capacity(bands);
    for b in 0..bands {
        let stats = match &region.samples {
            Samples::U8(v) => counted(
                256,
                (0..n).map(|i| v[i * bands + b] as usize),
                |k| k as f64,
                &skip,
            ),
            Samples::U16(v) => counted(
                65536,
                (0..n).map(|i| v[i * bands + b] as usize),
                |k| k as f64,
                &skip,
            ),
            s => {
                let mut vals: Vec<f64> = (0..n)
                    .map(|i| s.get(i * bands + b))
                    .filter(|&v| !skip(v))
                    .collect();
                vals.sort_unstable_by(f64::total_cmp);
                let c = vals.len() as u64;
                if c == 0 {
                    BandStats::default()
                } else {
                    BandStats {
                        count: c,
                        min: vals[0],
                        max: vals[vals.len() - 1],
                        low: vals[rank(PERCENT_LOW, c) as usize],
                        high: vals[rank(PERCENT_HIGH, c) as usize],
                    }
                }
            }
        };
        out.push(stats);
    }
    Stats { bands: out }
}

/// Statistics by counting values of `size` kinds.
fn counted(
    size: usize,
    values: impl Iterator<Item = usize>,
    value: impl Fn(usize) -> f64,
    skip: &impl Fn(f64) -> bool,
) -> BandStats {
    let mut hist = vec![0u64; size];
    for v in values {
        if v < size && !skip(value(v)) {
            hist[v] += 1;
        }
    }
    let count: u64 = hist.iter().sum();
    if count == 0 {
        return BandStats::default();
    }
    let at = |r: u64| {
        let mut acc = 0u64;
        for (k, &c) in hist.iter().enumerate() {
            acc += c;
            if acc > r {
                return value(k);
            }
        }
        value(size - 1)
    };
    BandStats {
        count,
        min: at(0),
        max: at(count - 1),
        low: at(rank(PERCENT_LOW, count)),
        high: at(rank(PERCENT_HIGH, count)),
    }
}
