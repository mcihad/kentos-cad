//! A viewport boundary in millionths of its content frame; never a camera rescale.
use crate::{Result, SheetError, units::RectUm};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const FULL: i32 = 1_000_000;
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperViewportClip"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Clip {
    #[default]
    Frame,
    Rectangle {
        /// Left, top, right, bottom in frame millionths.
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="frame-millionths")))]
        bounds: [i32; 4],
    },
    Polygon {
        /// A simple ring, either winding; closure is implicit. Coordinates are frame millionths.
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="frame-millionths")))]
        points: Vec<[i32; 2]>,
    },
}
// Version 1 books could explicitly carry the string "frame". Keep reading that spelling.
impl<'de> Deserialize<'de> for Clip {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
        enum Wire {
            Frame,
            Rectangle { bounds: [i32; 4] },
            Polygon { points: Vec<[i32; 2]> },
        }
        let value = serde_json::Value::deserialize(d)?;
        if value.as_str() == Some("frame") {
            return Ok(Self::Frame);
        }
        Ok(
            match serde_json::from_value::<Wire>(value).map_err(serde::de::Error::custom)? {
                Wire::Frame => Self::Frame,
                Wire::Rectangle { bounds } => Self::Rectangle { bounds },
                Wire::Polygon { points } => Self::Polygon { points },
            },
        )
    }
}
impl Clip {
    pub fn is_frame(&self) -> bool {
        matches!(self, Self::Frame)
    }
    pub fn points(&self) -> Vec<[i32; 2]> {
        match self {
            Self::Frame => vec![[0, 0], [FULL, 0], [FULL, FULL], [0, FULL]],
            Self::Rectangle {
                bounds: [l, t, r, b],
            } => vec![[*l, *t], [*r, *t], [*r, *b], [*l, *b]],
            Self::Polygon { points } => points.clone(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if let Self::Rectangle {
            bounds: [l, t, r, b],
        } = self
            && (l >= r || t >= b)
        {
            return Err(SheetError::new(
                "bad_value",
                "Kırpma dikdörtgeninin genişliği ve yüksekliği pozitif olmalı.",
            ));
        }
        let pts = self.points();
        if !(3..=256).contains(&pts.len()) {
            return Err(SheetError::new(
                "bad_value",
                "Kırpma sınırında 3–256 köşe olmalı.",
            ));
        }
        if pts.iter().flatten().any(|v| !(0..=FULL).contains(v)) {
            return Err(SheetError::new(
                "out_of_range",
                "Kırpma köşeleri içerik çerçevesinin %0–100 aralığında olmalı.",
            ));
        }
        let mut area = 0i128;
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            if a == b {
                return Err(SheetError::new(
                    "bad_value",
                    "Ardışık kırpma köşeleri farklı olmalı; ilk köşeyi sonda yinelemeyin.",
                ));
            }
            area += i128::from(a[0]) * i128::from(b[1]) - i128::from(a[1]) * i128::from(b[0]);
            for j in i + 1..pts.len() {
                if j == i + 1 || (i == 0 && j + 1 == pts.len()) {
                    continue;
                }
                if intersects(a, b, pts[j], pts[(j + 1) % pts.len()]) {
                    return Err(SheetError::new(
                        "bad_value",
                        "Kırpma sınırı kendiyle kesişemez veya kendine değemez.",
                    ));
                }
            }
        }
        if area == 0 {
            return Err(SheetError::new(
                "bad_value",
                "Kırpma sınırının alanı sıfır olamaz.",
            ));
        }
        Ok(())
    }
    pub fn paper_points(&self, frame: &RectUm) -> Vec<[f64; 2]> {
        self.points()
            .iter()
            .map(|p| {
                [
                    f64::from(frame.left)
                        + f64::from(p[0]) * f64::from(frame.width) / f64::from(FULL),
                    f64::from(frame.top)
                        + f64::from(p[1]) * f64::from(frame.height) / f64::from(FULL),
                ]
            })
            .collect()
    }
    /// Fractional frame coordinates; boundary is inside.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let pts = self.points();
        let p = [x * f64::from(FULL), y * f64::from(FULL)];
        let mut inside = false;
        for i in 0..pts.len() {
            let a = pts[i].map(f64::from);
            let b = pts[(i + 1) % pts.len()].map(f64::from);
            let cross = (p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0]);
            if cross.abs() <= 1e-6
                && p[0] >= a[0].min(b[0])
                && p[0] <= a[0].max(b[0])
                && p[1] >= a[1].min(b[1])
                && p[1] <= a[1].max(b[1])
            {
                return true;
            }
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
            {
                inside = !inside;
            }
        }
        inside
    }
}
fn orient(a: [i32; 2], b: [i32; 2], c: [i32; 2]) -> i128 {
    (i128::from(b[0]) - i128::from(a[0])) * (i128::from(c[1]) - i128::from(a[1]))
        - (i128::from(b[1]) - i128::from(a[1])) * (i128::from(c[0]) - i128::from(a[0]))
}
fn on(a: [i32; 2], b: [i32; 2], c: [i32; 2]) -> bool {
    orient(a, b, c) == 0 && (0..2).all(|j| c[j] >= a[j].min(b[j]) && c[j] <= a[j].max(b[j]))
}
fn intersects(a: [i32; 2], b: [i32; 2], c: [i32; 2], d: [i32; 2]) -> bool {
    let (p, q, r, s) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    (p.signum() != q.signum() && r.signum() != s.signum() && p != 0 && q != 0 && r != 0 && s != 0)
        || on(a, b, c)
        || on(a, b, d)
        || on(c, d, a)
        || on(c, d, b)
}

pub fn yes() -> bool {
    true
}
pub fn is_yes(v: &bool) -> bool {
    *v
}
pub fn yes_on_null<'de, D: serde::Deserializer<'de>>(d: D) -> core::result::Result<bool, D::Error> {
    Ok(Option::<bool>::deserialize(d)?.unwrap_or(true))
}

impl Clip {
    /// Apply the boundary to straight RGBA. Four vertical samples and exact horizontal
    /// pixel overlap retain edge coverage without rendering a second scene.
    pub fn mask_rgba(&self, rgba: &mut [u8], width: u32, height: u32) -> bool {
        let (w, h) = (width as usize, height as usize);
        if w == 0 || h == 0 || w.checked_mul(h).and_then(|v| v.checked_mul(4)) != Some(rgba.len()) {
            return false;
        }
        if self.is_frame() {
            return true;
        }
        let points = self
            .points()
            .iter()
            .map(|p| {
                [
                    f64::from(p[0]) * w as f64 / f64::from(FULL),
                    f64::from(p[1]) * h as f64 / f64::from(FULL),
                ]
            })
            .collect::<Vec<_>>();
        let mut cuts = Vec::with_capacity(points.len());
        let mut coverage = vec![0f64; w];
        for y in 0..h {
            coverage.fill(0.);
            for sy in [0.125, 0.375, 0.625, 0.875] {
                cuts.clear();
                let yy = y as f64 + sy;
                for i in 0..points.len() {
                    let (a, b) = (points[i], points[(i + 1) % points.len()]);
                    if (a[1] > yy) != (b[1] > yy) {
                        cuts.push(a[0] + (yy - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
                    }
                }
                cuts.sort_by(f64::total_cmp);
                for pair in cuts.chunks_exact(2) {
                    let (lo, hi) = (pair[0].max(0.), pair[1].min(w as f64));
                    for (x, v) in coverage
                        .iter_mut()
                        .enumerate()
                        .take(hi.ceil().max(0.) as usize)
                        .skip(lo.floor().max(0.) as usize)
                    {
                        *v += (hi.min(x as f64 + 1.) - lo.max(x as f64)).clamp(0., 1.) / 4.;
                    }
                }
            }
            for (x, v) in coverage.iter().enumerate() {
                let a = (y * w + x) * 4 + 3;
                rgba[a] = (f64::from(rgba[a]) * v.clamp(0., 1.)).round() as u8;
            }
        }
        true
    }
}
