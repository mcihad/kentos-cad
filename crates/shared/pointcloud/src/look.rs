//! Points' colours for a look (docs/adr/0207 §5): RGBA bytes, alpha 255, or 0
//! for a hidden class (the GPU drops it). The same bytes on both platforms;
//! the ramps are the rasters' (ADR 0204 §4).

use kentos_contracts::{CloudRender, PointCloudStyle};
use kentos_formats::raster::style::{ramp_at, ramp_stops};

use crate::classes;
use crate::nodes::NodePoints;

/// A ramp's place for `v` over `min` to `max`, turned round when `invert`.
#[inline]
fn place(v: f64, min: f64, max: f64, invert: bool) -> f64 {
    let t = if max > min {
        (v - min) / (max - min)
    } else {
        0.0
    };
    let t = t.clamp(0.0, 1.0);
    if invert { 1.0 - t } else { t }
}

/// A 16-bit colour channel as 8 bits: the high byte, or the value itself when the file writes 0–255.
#[inline]
fn eight(v: u16, rgb8: bool) -> u8 {
    if rgb8 {
        v.min(255) as u8
    } else {
        (v >> 8) as u8
    }
}

/// The colours of `points` (four bytes each) for `style`; `single` is the object's colour.
pub fn colours(style: &PointCloudStyle, points: &NodePoints, single: [u8; 3], out: &mut Vec<u8>) {
    let n = points.len();
    out.clear();
    out.reserve(4 * n);
    let mut hidden = [false; 256];
    for &c in &style.hidden {
        hidden[usize::from(c)] = true;
    }
    let (min, max) = (style.min.unwrap_or(0.0), style.max.unwrap_or(1.0));
    let elevation = ramp_stops(style.ramp_name());
    let grey = ramp_stops("Gri");
    for i in 0..n {
        let class = points.class(i);
        let c: [u8; 3] = match style.render {
            CloudRender::Rgb => {
                if points.rgb.len() >= 3 * (i + 1) {
                    [
                        eight(points.rgb[3 * i], style.rgb8),
                        eight(points.rgb[3 * i + 1], style.rgb8),
                        eight(points.rgb[3 * i + 2], style.rgb8),
                    ]
                } else {
                    classes::colour(1)
                }
            }
            CloudRender::Classification => classes::colour(class),
            CloudRender::Elevation => {
                ramp_at(elevation, place(points.z(i), min, max, style.invert))
            }
            CloudRender::Intensity => ramp_at(
                grey,
                place(f64::from(points.intensity(i)), min, max, style.invert),
            ),
            CloudRender::Returns => {
                let r = points.returns[i];
                classes::RETURNS[classes::return_kind(r & 0x0F, r >> 4)]
            }
            CloudRender::Single => single,
        };
        out.extend_from_slice(&[
            c[0],
            c[1],
            c[2],
            if hidden[usize::from(class)] { 0 } else { 255 },
        ]);
    }
}

/// What a new cloud's look is chosen from (docs/adr/0207 §5): sample points' heights and
/// intensities, the largest colour value, whether the format has colours.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sample {
    pub z: Vec<f64>,
    pub intensity: Vec<f64>,
    pub rgb_max: u16,
    pub rgb: bool,
}

impl Sample {
    /// Takes records of `layout` (every `step`-th of them).
    pub fn take(
        &mut self,
        layout: &crate::record::Layout,
        records: &[u8],
        scale: [f64; 3],
        offset: [f64; 3],
        step: usize,
    ) {
        self.rgb |= layout.rgb.is_some();
        for r in records.chunks_exact(layout.len).step_by(step.max(1)) {
            self.z.push(f64::from(layout.z(r)) * scale[2] + offset[2]);
            self.intensity.push(f64::from(layout.intensity(r)));
            if let Some(c) = layout.rgb(r) {
                self.rgb_max = self.rgb_max.max(c[0]).max(c[1]).max(c[2]);
            }
        }
    }

    /// A new cloud's look: its colours when the format has them (8 significant
    /// bits when no value passes 255), else its heights through Arazi over
    /// their 2nd to 98th percentile.
    pub fn default_style(&self) -> PointCloudStyle {
        let mut z = self.z.clone();
        let (min, max) = range(&mut z).unwrap_or((0.0, 1.0));
        let colours = self.rgb && self.rgb_max > 0;
        PointCloudStyle {
            render: if colours {
                CloudRender::Rgb
            } else {
                CloudRender::Elevation
            },
            ramp: None,
            invert: false,
            min: Some(min),
            max: Some(max),
            hidden: Vec::new(),
            rgb8: colours && self.rgb_max <= 255,
            size: kentos_contracts::DEFAULT_POINT_SIZE,
            size_unit: kentos_contracts::PointSizeUnit::Px,
            shape: kentos_contracts::PointShape::Round,
        }
    }

    /// The range a ramped look takes (Otomatik): heights for `elevation`, intensities for `intensity`.
    pub fn range_for(&self, render: CloudRender) -> Option<(f64, f64)> {
        let mut v = if render == CloudRender::Intensity {
            self.intensity.clone()
        } else {
            self.z.clone()
        };
        range(&mut v)
    }
}

/// A look's range from sample values: their 2nd and 98th percentiles (docs/adr/0207 §5);
/// the whole range when the sample is small; none without values.
pub fn range(values: &mut Vec<f64>) -> Option<(f64, f64)> {
    values.retain(|v| v.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    // The value at ⌊q·(n − 1) + ½⌋ of the sorted ones.
    let at = |q: f64| values[((q * (n - 1) as f64 + 0.5).floor() as usize).min(n - 1)];
    let (lo, hi) = if n >= 50 {
        (at(0.02), at(0.98))
    } else {
        (values[0], values[n - 1])
    };
    if hi > lo {
        Some((lo, hi))
    } else {
        Some((lo - 0.5, lo + 0.5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::{PointShape, PointSizeUnit};

    fn style(render: CloudRender) -> PointCloudStyle {
        PointCloudStyle {
            render,
            ramp: None,
            invert: false,
            min: Some(0.0),
            max: Some(10.0),
            hidden: vec![7],
            rgb8: false,
            size: 2.0,
            size_unit: PointSizeUnit::Px,
            shape: PointShape::Round,
        }
    }

    fn points() -> NodePoints {
        NodePoints {
            center: [0.0, 0.0, 5.0],
            xyz: vec![0.0, 0.0, -5.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0],
            class: vec![2, 7, 6],
            intensity: vec![0, 5, 10],
            returns: vec![1 | (1 << 4), 1 | (3 << 4), 3 | (3 << 4)],
            rgb: vec![0xFF00, 0x8000, 0x0100, 0, 0, 0, 65535, 65535, 65535],
        }
    }

    #[test]
    fn looks_colour_by_their_kind() {
        let p = points();
        let mut out = Vec::new();
        colours(&style(CloudRender::Classification), &p, [0, 0, 0], &mut out);
        assert_eq!(&out[0..4], &[0xA9, 0x7C, 0x50, 255]);
        assert_eq!(out[7], 0, "class 7 hidden");
        colours(&style(CloudRender::Rgb), &p, [0, 0, 0], &mut out);
        assert_eq!(&out[0..3], &[0xFF, 0x80, 0x01]);
        colours(&style(CloudRender::Elevation), &p, [0, 0, 0], &mut out);
        assert_eq!(
            &out[0..3],
            &[0x2E, 0x7D, 0x32],
            "lowest: the ramp's first stop"
        );
        assert_eq!(&out[4..7], &[0xFA, 0xFA, 0xFA], "highest: its last");
        assert_eq!(&out[8..11], &[0xFF, 0xF5, 0x9D], "middle: its middle stop");
        colours(&style(CloudRender::Intensity), &p, [0, 0, 0], &mut out);
        assert_eq!(&out[8..11], &[255, 255, 255]);
        colours(&style(CloudRender::Returns), &p, [0, 0, 0], &mut out);
        assert_eq!(&out[0..3], &classes::RETURNS[0]);
        assert_eq!(&out[4..7], &classes::RETURNS[1]);
        assert_eq!(&out[8..11], &classes::RETURNS[3]);
        colours(&style(CloudRender::Single), &p, [1, 2, 3], &mut out);
        assert_eq!(&out[0..4], &[1, 2, 3, 255]);
    }

    #[test]
    fn ranges_are_percentiles() {
        let mut v: Vec<f64> = (0..=100).map(f64::from).collect();
        assert_eq!(range(&mut v), Some((2.0, 98.0)));
        assert_eq!(range(&mut vec![5.0, 5.0]), Some((4.5, 5.5)));
        assert_eq!(range(&mut Vec::new()), None);
    }
}
