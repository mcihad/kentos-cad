//! Uzaktan algılama (docs/adr/0242): Bant birleştir, Bantlara ayır, Spektral
//! indis, Denetimli ve Denetimsiz sınıflandırma, Doğruluk analizi, Değişim
//! tespiti and Görüntü birleştirme as runs of the operation job (`ops`).
//! Each tool's settings are checked here, its cells worked out here and its
//! texts (the table, the summary's tail, the warnings) written here, so that
//! both platforms say the same.
//!
//! - [`spectral`]: the indices' formulas (§5).
//! - [`classify`]: the training cells' moments and the two classifiers (§6).
//! - [`cluster`]: k-means over a sample of the cells (§7).
//! - [`accuracy`]: the confusion matrix and kappa (§8).
//! - [`work`]: the runs: their grids, passes, blocks and notes.

pub mod accuracy;
pub mod classify;
pub mod cluster;
pub mod spectral;
pub mod work;

/// A run's table as written: its columns and rows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(columns: &[&str]) -> Table {
        Table {
            columns: columns.iter().map(|c| (*c).to_owned()).collect(),
            rows: Vec::new(),
        }
    }
}

/// What a remote sensing run says (§2).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RemoteNotes {
    pub table: Option<Table>,
    /// The summary's tail after the raster's size and file; Doğruluk
    /// analizi's whole summary.
    pub tail: String,
    pub warnings: Vec<String>,
    /// Doğruluk analizi: the overall accuracy (per cent) and kappa.
    pub overall: Option<f64>,
    pub kappa: Option<f64>,
}

/// `n` with a dot between its thousands (the summaries' counts).
pub fn count(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    out
}

/// `v` with `d` decimals by the display rule (ADR 0149).
pub fn fixed(v: f64, d: usize) -> String {
    kentos_geometry_core::display::fixed(v, d)
}

/// A cell's area in square metres (the drawing's units) of `affine`.
pub fn cell_area(affine: &[f64; 6]) -> f64 {
    let [_, a, b, _, c, d] = *affine;
    (a * d - b * c).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_have_dots() {
        assert_eq!(count(0), "0");
        assert_eq!(count(999), "999");
        assert_eq!(count(1000), "1.000");
        assert_eq!(count(16_777_216), "16.777.216");
    }
}
