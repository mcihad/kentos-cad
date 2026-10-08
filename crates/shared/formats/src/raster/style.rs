//! A tile's colours (docs/adr/0204 §4), worked out here so that both
//! platforms draw the same bytes: the bands through the style (RGB, grey,
//! palette, ramp, shaded relief, ramp × shade), stretched by the band's
//! statistics or the style's bounds, nodata and NaN clear, a fourth RGB band
//! as alpha; premultiplied, 258 × 258 (the tile and a pixel of its
//! neighbours round it).
//!
//! The shaded relief is gdaldem hillshade's default (Horn's gradient, the
//! light's azimuth from north clockwise and its altitude, the z factor):
//! 1 + 254 · cos of the angle between the light and the surface's normal,
//! 1 when the surface turns away; a neighbour that is nodata takes the
//! centre's value.

use kentos_contracts::{RasterRender, RasterSample, RasterStretch, RasterStyle};

use super::source::Region;
use super::stats::Stats;
use super::{Samples, TILE_APRON};
use crate::math;

/// The ramps' stops (sRGB), evenly spaced, by name (docs/adr/0204 §4).
pub fn ramp_stops(name: &str) -> &'static [[u8; 3]] {
    match name {
        "Arazi" => &[
            [0x2E, 0x7D, 0x32],
            [0x9C, 0xCC, 0x65],
            [0xFF, 0xF5, 0x9D],
            [0xA1, 0x88, 0x7F],
            [0xFA, 0xFA, 0xFA],
        ],
        "Spektral" => &[
            [0x2B, 0x83, 0xBA],
            [0xAB, 0xDD, 0xA4],
            [0xFF, 0xFF, 0xBF],
            [0xFD, 0xAE, 0x61],
            [0xD7, 0x19, 0x1C],
        ],
        "Viridis" => &[
            [0x44, 0x01, 0x54],
            [0x3B, 0x52, 0x8B],
            [0x21, 0x91, 0x8C],
            [0x5E, 0xC9, 0x62],
            [0xFD, 0xE7, 0x25],
        ],
        "Mavi-kırmızı" => &[
            [0x21, 0x66, 0xAC],
            [0x92, 0xC5, 0xDE],
            [0xF7, 0xF7, 0xF7],
            [0xF4, 0xA5, 0x82],
            [0xB2, 0x18, 0x2B],
        ],
        "Sıcaklık" => &[
            [0xFF, 0xFF, 0xB2],
            [0xFE, 0xCC, 0x5C],
            [0xFD, 0x8D, 0x3C],
            [0xF0, 0x3B, 0x20],
            [0xBD, 0x00, 0x26],
        ],
        _ => &[[0, 0, 0], [255, 255, 255]],
    }
}

/// The ramp's colour at `t` (0–1): between its two stops, linearly, each
/// channel rounded half up.
pub fn ramp_at(stops: &[[u8; 3]], t: f64) -> [u8; 3] {
    let n = stops.len();
    if n == 0 {
        return [0, 0, 0];
    }
    if n == 1 || !(t > 0.0) {
        return stops[0];
    }
    if t >= 1.0 {
        return stops[n - 1];
    }
    let p = t * (n - 1) as f64;
    let k = (p.floor() as usize).min(n - 2);
    let f = p - k as f64;
    let (a, b) = (stops[k], stops[k + 1]);
    let mix = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * f + 0.5).floor() as u8;
    [mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2])]
}

/// What a tile's colours are made with.
#[derive(Clone, Debug)]
pub struct Look<'a> {
    pub style: &'a RasterStyle,
    pub stats: Option<&'a Stats>,
    pub palette: Option<&'a [[u16; 3]]>,
    /// The file's nodata; the style's takes its place.
    pub file_nodata: Option<f64>,
    pub sample: RasterSample,
    /// A pixel's size at the tile's level along its columns and its rows
    /// (the rows' signed: negative when they go south), metres.
    pub ewres: f64,
    pub nsres: f64,
}

impl Look<'_> {
    fn nodata(&self) -> Option<f64> {
        self.style.nodata.or(self.file_nodata)
    }

    /// The bounds band `b` (0-based in the style's list) is stretched
    /// between; none: as it is (8 bit).
    fn bounds(&self, band: u32) -> Option<(f64, f64)> {
        let i = band.saturating_sub(1) as usize;
        let stat = |f: fn(&Stats, usize) -> Option<(f64, f64)>| self.stats.and_then(|s| f(s, i));
        let fallback = match self.sample {
            RasterSample::U8 => (0.0, 255.0),
            RasterSample::U16 => (0.0, 65535.0),
            RasterSample::I8 => (-128.0, 127.0),
            RasterSample::I16 => (-32768.0, 32767.0),
            _ => (0.0, 1.0),
        };
        match self.style.stretch {
            RasterStretch::None => {
                (self.sample != RasterSample::U8).then_some(stat(Stats::minmax).unwrap_or(fallback))
            }
            RasterStretch::MinMax => Some(stat(Stats::minmax).unwrap_or(fallback)),
            RasterStretch::Percent => Some(stat(Stats::percent).unwrap_or(fallback)),
            RasterStretch::Manual => Some((
                self.style.min.unwrap_or(fallback.0),
                self.style.max.unwrap_or(fallback.1),
            )),
        }
    }
}

/// `v` brought to 0–1 between `lo` and `hi`.
#[inline]
fn unit(v: f64, (lo, hi): (f64, f64)) -> f64 {
    if hi > lo {
        ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[inline]
fn byte(t: f64) -> u8 {
    (t * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8
}

/// A value as a channel: stretched when the look says so, else held within 0–255.
#[inline]
fn channel(v: f64, bounds: Option<(f64, f64)>) -> u8 {
    match bounds {
        Some(b) => byte(unit(v, b)),
        None => (v + 0.5).floor().clamp(0.0, 255.0) as u8,
    }
}

/// gdaldem hillshade's light, worked out once.
struct Light {
    sin_alt_254: f64,
    cos_az_254: f64,
    sin_az_254: f64,
    square_z: f64,
    inv_ew: f64,
    inv_ns: f64,
}

impl Light {
    fn new(style: &RasterStyle, ewres: f64, nsres: f64) -> Light {
        let (az, alt, z) = style.light();
        let rad = std::f64::consts::PI / 180.0;
        let z_scaled = z / 8.0;
        let cos_alt_z = math::cos(alt * rad) * z_scaled;
        Light {
            sin_alt_254: 254.0 * math::sin(alt * rad),
            cos_az_254: 254.0 * math::cos(az * rad) * cos_alt_z,
            sin_az_254: 254.0 * math::sin(az * rad) * cos_alt_z,
            square_z: z_scaled * z_scaled,
            inv_ew: 1.0 / ewres,
            inv_ns: 1.0 / nsres,
        }
    }

    /// The shade (1–255) of a 3 × 3 window, top row first.
    #[inline]
    fn shade(&self, w: &[f64; 9]) -> f64 {
        let x = ((w[0] + w[3] + w[3] + w[6]) - (w[2] + w[5] + w[5] + w[8])) * self.inv_ew;
        let y = ((w[6] + w[7] + w[7] + w[8]) - (w[0] + w[1] + w[1] + w[2])) * self.inv_ns;
        let num = self.sin_alt_254 - (y * self.cos_az_254 - x * self.sin_az_254);
        let c = num / (1.0 + self.square_z * (x * x + y * y)).sqrt();
        if c <= 0.0 { 1.0 } else { 1.0 + c }
    }
}

/// The tile's colours from `region` (the tile and 2 pixels round it),
/// premultiplied RGBA, `TILE_APRON` × `TILE_APRON`, into `out`.
pub fn render(region: &Region, look: &Look<'_>, out: &mut Vec<u8>) {
    let side = TILE_APRON as usize;
    out.clear();
    out.resize(side * side * 4, 0);
    let style = look.style;
    let nodata = look.nodata();
    let empty = |v: f64| v.is_nan() || nodata.is_some_and(|d| v == d);
    // Region pixel of output (u, v): one in from its corner.
    let at = |u: usize, v: usize, b: u32| region.at(u as u32 + 1, v as u32 + 1, b);
    match style.render {
        RasterRender::Rgb => {
            let bands = &style.bands;
            let (r, g, b) = (
                bands[0] - 1,
                bands.get(1).map_or(0, |x| x - 1),
                bands.get(2).map_or(0, |x| x - 1),
            );
            let alpha = bands.get(3).map(|x| x - 1);
            let bounds = [
                look.bounds(bands[0]),
                look.bounds(bands.get(1).copied().unwrap_or(1)),
                look.bounds(bands.get(2).copied().unwrap_or(1)),
            ];
            // The common case quickly: 8-bit bands as they are, row by row.
            if let (Samples::U8(src), None, None, None) =
                (&region.samples, bounds[0], bounds[1], bounds[2])
            {
                let rb = region.bands as usize;
                let rw = region.width as usize;
                for v in 0..side {
                    let row = (v + 1) * rw + 1;
                    for u in 0..side {
                        let p = (row + u) * rb;
                        let (cr, cg, cb) = (
                            src[p + r as usize],
                            src[p + g as usize],
                            src[p + b as usize],
                        );
                        let a = alpha.map_or(255, |k| src[p + k as usize]);
                        let clear = nodata.is_some_and(|d| {
                            f64::from(cr) == d && f64::from(cg) == d && f64::from(cb) == d
                        });
                        let o = (v * side + u) * 4;
                        if clear || a == 0 {
                            continue;
                        }
                        out[o..o + 4].copy_from_slice(&premul(cr, cg, cb, a));
                    }
                }
                return;
            }
            for v in 0..side {
                for u in 0..side {
                    let (vr, vg, vb) = (at(u, v, r), at(u, v, g), at(u, v, b));
                    if vr.is_nan()
                        || vg.is_nan()
                        || vb.is_nan()
                        || nodata.is_some_and(|d| vr == d && vg == d && vb == d)
                    {
                        continue;
                    }
                    let a = alpha.map_or(255, |k| {
                        let x = at(u, v, k);
                        if look.sample == RasterSample::U16 {
                            byte(x / 65535.0)
                        } else {
                            channel(x, None)
                        }
                    });
                    if a == 0 {
                        continue;
                    }
                    let o = (v * side + u) * 4;
                    out[o..o + 4].copy_from_slice(&premul(
                        channel(vr, bounds[0]),
                        channel(vg, bounds[1]),
                        channel(vb, bounds[2]),
                        a,
                    ));
                }
            }
        }
        RasterRender::Gray | RasterRender::Palette | RasterRender::Ramp => {
            let band = style.bands[0];
            let k = band - 1;
            let bounds = look.bounds(band);
            let stops = ramp_stops(style.ramp_name());
            for v in 0..side {
                for u in 0..side {
                    let x = at(u, v, k);
                    if empty(x) {
                        continue;
                    }
                    let c = match style.render {
                        RasterRender::Palette => match look.palette.and_then(|p| p.get(x as usize))
                        {
                            Some(c) => [(c[0] >> 8) as u8, (c[1] >> 8) as u8, (c[2] >> 8) as u8],
                            None => [0, 0, 0],
                        },
                        RasterRender::Ramp => {
                            let mut t = unit(x, bounds.unwrap_or((0.0, 255.0)));
                            if style.invert {
                                t = 1.0 - t;
                            }
                            ramp_at(stops, t)
                        }
                        _ => {
                            let g = channel(x, bounds);
                            [g, g, g]
                        }
                    };
                    let o = (v * side + u) * 4;
                    out[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
                }
            }
        }
        RasterRender::Hillshade | RasterRender::RampShade => {
            let band = style.bands[0];
            let k = band - 1;
            let light = Light::new(style, look.ewres, look.nsres);
            let bounds = look.bounds(band);
            let stops = ramp_stops(style.ramp_name());
            let mut w = [0.0f64; 9];
            for v in 0..side {
                for u in 0..side {
                    let centre = at(u, v, k);
                    if empty(centre) {
                        continue;
                    }
                    for dj in 0..3 {
                        for di in 0..3 {
                            let x = region.at((u + di) as u32, (v + dj) as u32, k);
                            w[dj * 3 + di] = if empty(x) { centre } else { x };
                        }
                    }
                    let shade = light.shade(&w);
                    let o = (v * side + u) * 4;
                    if style.render == RasterRender::Hillshade {
                        let s = (shade + 0.5).floor().clamp(0.0, 255.0) as u8;
                        out[o..o + 4].copy_from_slice(&[s, s, s, 255]);
                    } else {
                        let mut t = unit(centre, bounds.unwrap_or((0.0, 255.0)));
                        if style.invert {
                            t = 1.0 - t;
                        }
                        let c = ramp_at(stops, t);
                        let f = 0.4 + 0.6 * shade / 255.0;
                        let m = |x: u8| (f64::from(x) * f + 0.5).floor().clamp(0.0, 255.0) as u8;
                        out[o..o + 4].copy_from_slice(&[m(c[0]), m(c[1]), m(c[2]), 255]);
                    }
                }
            }
        }
    }
}

/// A colour premultiplied by its alpha (rounded as the pictures are).
#[inline]
fn premul(r: u8, g: u8, b: u8, a: u8) -> [u8; 4] {
    if a == 255 {
        return [r, g, b, 255];
    }
    let m = |c: u8| ((u32::from(c) * u32::from(a) + 127) / 255) as u8;
    [m(r), m(g), m(b), a]
}

/// The style a raster gets when it is added (docs/adr/0204 §4).
pub fn default_style(bands: u32, sample: RasterSample, palette: bool) -> RasterStyle {
    let base = |render, bands: Vec<u32>, stretch| RasterStyle {
        render,
        bands,
        stretch,
        min: None,
        max: None,
        ramp: None,
        invert: false,
        azimuth: None,
        altitude: None,
        z_factor: None,
        nodata: None,
        resampling: kentos_contracts::RasterResampling::Bilinear,
    };
    if palette {
        return base(RasterRender::Palette, vec![1], RasterStretch::None);
    }
    match (bands, sample) {
        (b, RasterSample::U8) if b >= 4 => {
            base(RasterRender::Rgb, vec![1, 2, 3, 4], RasterStretch::None)
        }
        (3, RasterSample::U8) => base(RasterRender::Rgb, vec![1, 2, 3], RasterStretch::None),
        (b, _) if b >= 3 => base(RasterRender::Rgb, vec![1, 2, 3], RasterStretch::MinMax),
        (_, RasterSample::U8) => base(RasterRender::Gray, vec![1], RasterStretch::None),
        _ => RasterStyle {
            ramp: Some("Arazi".to_owned()),
            ..base(RasterRender::RampShade, vec![1], RasterStretch::MinMax)
        },
    }
}
