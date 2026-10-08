//! A GeoTIFF's place and system (docs/adr/0204 §1): the affine transform from
//! pixel corners to the model space (ModelTransformation, else
//! ModelPixelScale with one ModelTiepoint; PixelIsPoint moves it half a
//! pixel, as GDAL does), the EPSG code of its projected or geographic
//! system (none when the file names none or a user-defined one), and the
//! GDAL_NODATA value.

use super::tiff::Ifd;

/// GeoKey ids read.
const GT_MODEL_TYPE: u64 = 1024;
const GT_RASTER_TYPE: u64 = 1025;
const GEOGRAPHIC_TYPE: u64 = 2048;
const PROJECTED_CS_TYPE: u64 = 3072;
/// A user-defined system: KentOS does not read its parameters.
const USER_DEFINED: u64 = 32767;

/// What a directory says of its place.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoInfo {
    /// `[x₀, a, b, y₀, c, d]` (pixel corners), when the file gives one.
    pub affine: Option<[f64; 6]>,
    /// The EPSG code of its system, when it names one KentOS can know.
    pub epsg: Option<u32>,
    /// Whether the system is geographic (degrees).
    pub geographic: bool,
    /// The file's nodata value.
    pub nodata: Option<f64>,
}

/// A GeoKey's short value, when the directory holds it inline.
fn key(ifd: &Ifd, id: u64) -> Option<u64> {
    let k = &ifd.geo_keys;
    if k.len() < 4 {
        return None;
    }
    let n = usize::try_from(k[3]).ok()?;
    (0..n).find_map(|i| {
        let e = k.get(4 + 4 * i..8 + 4 * i)?;
        (e[0] == id && e[1] == 0 && e[2] == 1).then_some(e[3])
    })
}

/// The nodata value GDAL writes as text: a number, `nan`, `inf` or `-inf`.
pub fn read_nodata(text: &str) -> Option<f64> {
    let t = text.trim();
    match t.to_ascii_lowercase().as_str() {
        "nan" | "-nan" => Some(f64::NAN),
        "inf" | "+inf" => Some(f64::INFINITY),
        "-inf" => Some(f64::NEG_INFINITY),
        _ => t.parse::<f64>().ok(),
    }
}

/// The directory's place, system and nodata.
pub fn read(ifd: &Ifd) -> GeoInfo {
    let point = key(ifd, GT_RASTER_TYPE) == Some(2);
    let affine = if ifd.transformation.len() >= 16 {
        let m = &ifd.transformation;
        Some([m[3], m[0], m[1], m[7], m[4], m[5]])
    } else if ifd.pixel_scale.len() >= 2 && ifd.tiepoints.len() >= 6 {
        let (sx, sy) = (ifd.pixel_scale[0], ifd.pixel_scale[1]);
        let t = &ifd.tiepoints;
        let (i, j, x, y) = (t[0], t[1], t[3], t[4]);
        Some([x - i * sx, sx, 0.0, y + j * sy, 0.0, -sy])
    } else {
        None
    }
    .filter(|a| a.iter().all(|v| v.is_finite()))
    .map(|[x0, a, b, y0, c, d]| {
        if point {
            [x0 - 0.5 * (a + b), a, b, y0 - 0.5 * (c + d), c, d]
        } else {
            [x0, a, b, y0, c, d]
        }
    });
    let model = key(ifd, GT_MODEL_TYPE);
    let code = |id| key(ifd, id).filter(|&c| c > 0 && c < USER_DEFINED);
    let (epsg, geographic) = match model {
        Some(2) => (code(GEOGRAPHIC_TYPE), true),
        Some(1) => (code(PROJECTED_CS_TYPE), false),
        _ => (
            code(PROJECTED_CS_TYPE).or_else(|| code(GEOGRAPHIC_TYPE)),
            code(PROJECTED_CS_TYPE).is_none() && code(GEOGRAPHIC_TYPE).is_some(),
        ),
    };
    GeoInfo {
        affine,
        epsg: epsg.and_then(|c| u32::try_from(c).ok()),
        geographic,
        nodata: ifd.gdal_nodata.as_deref().and_then(read_nodata),
    }
}
