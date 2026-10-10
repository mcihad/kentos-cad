//! Spektral indis (docs/adr/0242 §5): an index from a cell's bands. Each band
//! is first a reflectance ρ = DN × scale + offset; the formula is worked out
//! in float64 in the order written here (the reference's), the result held
//! as float32. A divisor of zero leaves the cell empty.

use serde::Deserialize;

/// The indices.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IndexKind {
    Ndvi,
    Gndvi,
    Savi,
    Evi,
    Ndwi,
    Mndwi,
    Ndbi,
    Ratio,
    Normalized,
}

/// The bands by their names, from 1 (§5: only those the index reads are used).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexBands {
    pub blue: u32,
    pub green: u32,
    pub red: u32,
    pub nir: u32,
    pub swir: u32,
    pub a: u32,
    pub b: u32,
}

/// A band's name in the index's formula.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandName {
    Blue,
    Green,
    Red,
    Nir,
    Swir,
    A,
    B,
}

impl BandName {
    /// Its number in `bands`.
    pub fn of(self, bands: &IndexBands) -> u32 {
        match self {
            BandName::Blue => bands.blue,
            BandName::Green => bands.green,
            BandName::Red => bands.red,
            BandName::Nir => bands.nir,
            BandName::Swir => bands.swir,
            BandName::A => bands.a,
            BandName::B => bands.b,
        }
    }
}

impl IndexKind {
    /// The bands the formula reads, in its order.
    pub fn uses(self) -> &'static [BandName] {
        use BandName::*;
        match self {
            IndexKind::Ndvi | IndexKind::Savi => &[Red, Nir],
            IndexKind::Gndvi | IndexKind::Ndwi => &[Green, Nir],
            IndexKind::Evi => &[Blue, Red, Nir],
            IndexKind::Mndwi => &[Green, Swir],
            IndexKind::Ndbi => &[Swir, Nir],
            IndexKind::Ratio | IndexKind::Normalized => &[A, B],
        }
    }

    /// Its name in the summary.
    pub fn name(self) -> &'static str {
        match self {
            IndexKind::Ndvi => "NDVI",
            IndexKind::Gndvi => "GNDVI",
            IndexKind::Savi => "SAVI",
            IndexKind::Evi => "EVI",
            IndexKind::Ndwi => "NDWI",
            IndexKind::Mndwi => "MNDWI",
            IndexKind::Ndbi => "NDBI",
            IndexKind::Ratio => "Oran",
            IndexKind::Normalized => "Normalize fark",
        }
    }

    /// A vegetation index (its look: the Arazi ramp turned round).
    pub fn vegetation(self) -> bool {
        matches!(
            self,
            IndexKind::Ndvi | IndexKind::Gndvi | IndexKind::Savi | IndexKind::Evi
        )
    }

    /// A water index (its look: Mavi-kırmızı turned round, water blue).
    pub fn water(self) -> bool {
        matches!(self, IndexKind::Ndwi | IndexKind::Mndwi)
    }
}

/// An index's settings: the formula and its constants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Index {
    pub kind: IndexKind,
    pub scale: f64,
    pub offset: f64,
    /// SAVI's L.
    pub savi_l: f64,
    /// EVI's G, C₁, C₂ and L.
    pub g: f64,
    pub c1: f64,
    pub c2: f64,
    pub evi_l: f64,
}

impl Index {
    /// A band's reflectance.
    #[inline]
    pub fn reflectance(&self, dn: f64) -> f64 {
        dn * self.scale + self.offset
    }

    /// The index from the reflectances `v` (in [`IndexKind::uses`]' order);
    /// none where the divisor is zero.
    #[inline]
    pub fn value(&self, v: &[f64]) -> Option<f64> {
        let (num, den) = match self.kind {
            // (p − q) / (p + q) with p, q as the formula names them.
            IndexKind::Ndvi | IndexKind::Gndvi => (v[1] - v[0], v[1] + v[0]),
            IndexKind::Ndwi | IndexKind::Mndwi | IndexKind::Ndbi | IndexKind::Normalized => {
                (v[0] - v[1], v[0] + v[1])
            }
            IndexKind::Savi => (
                (1.0 + self.savi_l) * (v[1] - v[0]),
                v[1] + v[0] + self.savi_l,
            ),
            IndexKind::Evi => (
                self.g * (v[2] - v[1]),
                v[2] + self.c1 * v[1] - self.c2 * v[0] + self.evi_l,
            ),
            IndexKind::Ratio => (v[0], v[1]),
        };
        (den != 0.0).then(|| num / den)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(kind: IndexKind) -> Index {
        Index {
            kind,
            scale: 1.0,
            offset: 0.0,
            savi_l: 0.5,
            g: 2.5,
            c1: 6.0,
            c2: 7.5,
            evi_l: 1.0,
        }
    }

    #[test]
    fn the_formulas() {
        // Red 0.1, NIR 0.5.
        assert_eq!(index(IndexKind::Ndvi).value(&[0.1, 0.5]), Some(0.4 / 0.6));
        assert_eq!(
            index(IndexKind::Savi).value(&[0.1, 0.5]),
            Some(1.5 * 0.4 / (0.5 + 0.1 + 0.5))
        );
        // NDWI's green first: (G − NIR) / (G + NIR).
        assert_eq!(index(IndexKind::Ndwi).value(&[0.2, 0.5]), Some(-0.3 / 0.7));
        assert_eq!(index(IndexKind::Ratio).value(&[3.0, 2.0]), Some(1.5));
        assert_eq!(index(IndexKind::Normalized).value(&[1.0, -1.0]), None);
        // EVI: G (NIR − R) / (NIR + C₁R − C₂B + L).
        let e = index(IndexKind::Evi).value(&[0.05, 0.1, 0.5]).unwrap();
        assert_eq!(e, 2.5 * (0.5 - 0.1) / (0.5 + 6.0 * 0.1 - 7.5 * 0.05 + 1.0));
    }
}
