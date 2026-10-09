//! Raster to vector (docs/adr/0234): a raster's regions as areas, its thin
//! lines as polylines, its cells as points, and the semi-automatic capture of
//! a scanned sheet's lines and areas. The operation job (`crate::ops`) reads
//! the raster strip by strip and hands the rows over; the work here runs on
//! cell space (u along a row, v down the rows) and the results come back in
//! the world through the raster's affine.
//!
//! - [`label`]: regions of equal value, labelled row after row (§4).
//! - [`rings`]: a region's rings along its cells' sides (§4).
//! - [`thin`]: Zhang–Suen thinning and the skeleton's paths (§5).
//! - [`simplify`]: Douglas–Peucker in cell space (§5).
//! - [`capture`]: the seed, the colour and the window of a capture (§7, §8).
//! - [`work`]: each tool's part in the operation job's passes.

pub mod capture;
pub mod label;
pub mod rings;
pub mod simplify;
pub mod thin;
pub mod work;

use kentos_contracts::RasterSample;

use crate::inputs::point_of;

/// Cells a vectorizing run that reads the whole raster takes (8192 × 8192).
pub const MOST_CELLS: u64 = 1 << 26;
/// Areas or polylines a run writes.
pub const MOST_FEATURES: usize = 1_000_000;
/// Points a run writes.
pub const MOST_POINTS: usize = 2_000_000;

/// What a vectorizing run gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureKind {
    Polygons,
    Lines,
    Points,
}

/// A run's features, flat for the hosts: each feature's value (NaN: none)
/// and its text, its tag (a point: 0, 1 a peak, 2 a pit; Alan kapat's area:
/// 1 when its simplified rings crossed and it is written as it is), its
/// rings (an area: the first its outline), each ring's or line's vertices
/// and the vertices' world coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Features {
    pub kind: FeatureKind,
    pub values: Vec<f64>,
    pub texts: Vec<String>,
    pub tags: Vec<u8>,
    /// An area's ring count; empty for lines and points.
    pub rings: Vec<u32>,
    /// Each ring's or line's vertex count (a point: 1).
    pub sizes: Vec<u32>,
    /// x, y of every vertex in order.
    pub xy: Vec<f64>,
}

impl Features {
    pub fn new(kind: FeatureKind) -> Features {
        Features {
            kind,
            values: Vec::new(),
            texts: Vec::new(),
            tags: Vec::new(),
            rings: Vec::new(),
            sizes: Vec::new(),
            xy: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// A feature's value and its text.
    fn value(&mut self, value: f64, text: String, tag: u8) {
        self.values.push(value);
        self.texts.push(text);
        self.tags.push(tag);
    }

    /// An area: its rings (cell-space corners, each without its first
    /// repeated), taken to the world through `affine`; its value and text, its tag.
    pub fn push_area(
        &mut self,
        rings: &[Vec<[f64; 2]>],
        affine: &[f64; 6],
        (value, text): (f64, String),
        tag: u8,
    ) {
        self.value(value, text, tag);
        self.rings.push(rings.len() as u32);
        for r in rings {
            self.sizes.push(r.len() as u32);
            for &[u, v] in r {
                let (x, y) = point_of(affine, u, v);
                self.xy.extend([x, y]);
            }
        }
    }

    /// A line through cell-space points, taken to the world.
    pub fn push_line(&mut self, pts: &[[f64; 2]], affine: &[f64; 6], value: f64, text: String) {
        self.value(value, text, 0);
        self.sizes.push(pts.len() as u32);
        for &[u, v] in pts {
            let (x, y) = point_of(affine, u, v);
            self.xy.extend([x, y]);
        }
    }

    /// A point at a cell-space place, taken to the world.
    pub fn push_point(
        &mut self,
        at: [f64; 2],
        affine: &[f64; 6],
        value: f64,
        text: String,
        tag: u8,
    ) {
        self.value(value, text, tag);
        self.sizes.push(1);
        let (x, y) = point_of(affine, at[0], at[1]);
        self.xy.extend([x, y]);
    }
}

/// A value's text (§2): the sample type's shortest decimal that reads back
/// the same number, without an exponent; a whole number in an integer type.
pub fn value_text(v: f64, sample: RasterSample) -> String {
    if v.is_nan() {
        return String::new();
    }
    match sample {
        RasterSample::F32 => format!("{}", v as f32),
        RasterSample::F64 => format!("{v}"),
        // Integer samples are whole and within ±2⁶³.
        _ => format!("{}", v as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_texts_are_the_shortest_decimals() {
        assert_eq!(value_text(3.0, RasterSample::F32), "3");
        assert_eq!(value_text(f64::from(0.1f32), RasterSample::F32), "0.1");
        assert_eq!(value_text(-2.5, RasterSample::F64), "-2.5");
        assert_eq!(value_text(1e-7, RasterSample::F64), "0.0000001");
        assert_eq!(value_text(255.0, RasterSample::U8), "255");
        assert_eq!(value_text(-7.0, RasterSample::I16), "-7");
        assert_eq!(value_text(f64::NAN, RasterSample::F32), "");
    }
}
