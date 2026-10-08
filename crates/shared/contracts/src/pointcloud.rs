//! Point clouds (docs/adr/0207 §3, §5): one or more files (a virtual cloud
//! when more), each linked (`file`), read from an address (`url`) or embedded
//! (`asset`), shown by their style. The points are never in the drawing; only
//! where they are, what they hold and how they are shown.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::EntityBase;

/// The most members a virtual cloud may have.
pub const MAX_CLOUD_SOURCES: usize = 4096;
/// The longest a path or address may be, in letters.
pub const MAX_CLOUD_PATH: usize = 4096;
/// A point's least and most size (pixels or metres).
pub const MIN_POINT_SIZE: f64 = 0.5;
pub const MAX_POINT_SIZE: f64 = 32.0;
/// The least and most opacity a cloud is drawn with.
pub const MIN_CLOUD_OPACITY: f64 = 0.1;
pub const MAX_CLOUD_OPACITY: f64 = 1.0;
/// The size a new cloud's points are drawn with, pixels.
pub const DEFAULT_POINT_SIZE: f64 = 2.0;

/// What a member's file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CloudFormat {
    Las,
    Laz,
    Copc,
    Xyz,
}

/// How points are coloured (docs/adr/0207 §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CloudRender {
    /// The file's colours.
    Rgb,
    /// ASPRS's classes, each its colour.
    Classification,
    /// Heights through a ramp.
    Elevation,
    /// Intensity in grey.
    Intensity,
    /// Single, first, intermediate and last returns in four colours.
    Returns,
    /// The object's colour.
    Single,
}

/// The unit of a point's size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PointSizeUnit {
    /// Pixels on the screen (independent of the device's pixels).
    #[default]
    Px,
    /// Metres on the ground (at least a pixel).
    M,
}

/// A point's shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PointShape {
    #[default]
    Round,
    Square,
}

/// Names as the files and the wire hold them, both ways.
macro_rules! named {
    ($t:ty { $($v:ident => $n:literal),+ $(,)? }) => {
        impl $t {
            /// Every value, in the contract's order.
            pub const ALL: &'static [$t] = &[$(<$t>::$v),+];

            /// The name files and the wire hold.
            pub fn name(self) -> &'static str {
                match self {
                    $(<$t>::$v => $n),+
                }
            }

            pub fn from_name(name: &str) -> Option<$t> {
                match name {
                    $($n => Some(<$t>::$v)),+,
                    _ => None,
                }
            }
        }
    };
}

named!(CloudFormat { Las => "las", Laz => "laz", Copc => "copc", Xyz => "xyz" });
named!(CloudRender { Rgb => "rgb", Classification => "classification", Elevation => "elevation", Intensity => "intensity", Returns => "returns", Single => "single" });
named!(PointSizeUnit { Px => "px", M => "m" });
named!(PointShape { Round => "round", Square => "square" });

impl CloudRender {
    /// Whether it colours by a ramp over `min` to `max`.
    pub fn ramped(self) -> bool {
        matches!(self, CloudRender::Elevation | CloudRender::Intensity)
    }
}

/// A cloud's look (docs/adr/0207 §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudStyle {
    pub render: CloudRender,
    /// One of the rasters' ramps (`RASTER_RAMPS`); absent: Arazi.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ramp: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub invert: bool,
    /// The ramp's least and most value (heights in metres, or intensities).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<f64>,
    /// Classes not drawn (0–255), in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u8>>", optional))]
    pub hidden: Vec<u8>,
    /// Colours with 8 significant bits (a file that writes 0–255 in its 16-bit fields).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub rgb8: bool,
    pub size: f64,
    #[serde(default, skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<PointSizeUnit>", optional))]
    pub size_unit: PointSizeUnit,
    #[serde(default, skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<PointShape>", optional))]
    pub shape: PointShape,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

impl PointCloudStyle {
    /// The look as the contract's JSON text (the geometry core carries it so).
    pub fn to_json_text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// A look from the contract's JSON text; none when it is not one.
    pub fn from_json_text(text: &str) -> Option<PointCloudStyle> {
        serde_json::from_str(text).ok()
    }

    /// The ramp's name: the style's, else Arazi.
    pub fn ramp_name(&self) -> &str {
        self.ramp.as_deref().unwrap_or("Arazi")
    }

    /// Why the style does not make a look, in the commands' words; none when it does.
    pub fn problem(&self) -> Option<String> {
        if let Some(r) = &self.ramp
            && !crate::RASTER_RAMPS.contains(&r.as_str())
        {
            return Some(format!(
                "“{r}” diye bir renk rampası yok; {} rampalarından biri seçilmeli.",
                crate::RASTER_RAMPS.join(", ")
            ));
        }
        match (self.min, self.max) {
            (None, None) => {}
            (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() && lo < hi => {}
            _ => {
                return Some(
                    "Görünüşün aralığı iki sonlu sayı olmalı, en küçük en büyükten küçük.".into(),
                );
            }
        }
        if self.render.ramped() && (self.min.is_none() || self.max.is_none()) {
            return Some("Rampalı görünüşün aralığı (en küçük ve en büyük) verilmeli.".into());
        }
        if !(self.size.is_finite() && (MIN_POINT_SIZE..=MAX_POINT_SIZE).contains(&self.size)) {
            return Some(format!(
                "Noktanın boyu {MIN_POINT_SIZE} ile {MAX_POINT_SIZE} arasında olmalı."
            ));
        }
        if self.hidden.windows(2).any(|w| w[0] >= w[1]) {
            return Some("Gizlenen sınıflar küçükten büyüğe ve birer kez yazılmalı.".into());
        }
        None
    }
}

/// One file of a cloud.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudSource {
    /// The project library's asset its bytes are (embedded); exactly one of
    /// `asset`, `file` and `url` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// The file (linked): absolute, or relative to the drawing's folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// An HTTP or HTTPS address read by ranges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub url: Option<String>,
    pub format: CloudFormat,
    /// Its points, as its header says.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub count: u64,
    /// `[x₁, y₁, z₁, x₂, y₂, z₂]`, as its header says.
    pub bounds: [f64; 6],
}

/// A point cloud (docs/adr/0207 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub cloud: PointCloudFields,
}

/// What places and shows a cloud: the object's own fields, and its geometry
/// in the commands (`EntityGeometry::PointCloud`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudFields {
    pub sources: Vec<CloudSource>,
    /// The members' bounds together.
    pub bounds: [f64; 6],
    /// The members' points together.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub count: u64,
    /// The files' system; 0: they named none and the user took the project's.
    pub srid: u32,
    pub style: PointCloudStyle,
    /// 0.1 to 1; absent: opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opacity: Option<f64>,
}

/// Why a path or address is not one, in the commands' words.
fn path_problem(what: &str, p: &str) -> Option<String> {
    (p.chars().count() > MAX_CLOUD_PATH || p.chars().any(char::is_control)).then(|| {
        format!("{what} en çok {MAX_CLOUD_PATH} harf olmalı ve denetim karakteri içermemeli.")
    })
}

/// Why an address is not an HTTP or HTTPS one; none when it is.
pub fn url_problem(u: &str) -> Option<String> {
    let lower = u.trim().to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) || u.trim().len() < 9 {
        return Some(format!(
            "“{u}” bir HTTP ya da HTTPS adresi değil (http:// ya da https:// ile başlamalı)."
        ));
    }
    path_problem("Adres", u)
}

fn bounds_problem(b: &[f64; 6]) -> bool {
    !(b.iter().all(|v| v.is_finite()) && b[0] <= b[3] && b[1] <= b[4] && b[2] <= b[5])
}

/// A cloud's files as the contract's JSON text (the geometry core carries them so).
pub fn sources_json_text(sources: &[CloudSource]) -> String {
    serde_json::to_string(sources).unwrap_or_default()
}

/// A cloud's files from the contract's JSON text; none when it is not one.
pub fn sources_from_json_text(text: &str) -> Option<Vec<CloudSource>> {
    serde_json::from_str(text).ok()
}

impl PointCloudFields {
    /// Its plan: `[x₁, y₁, x₂, y₂]`.
    pub fn rect(&self) -> [f64; 4] {
        [
            self.bounds[0],
            self.bounds[1],
            self.bounds[3],
            self.bounds[4],
        ]
    }

    /// Why the fields do not make a cloud, in the commands' words; none when
    /// they do. An asset's being in the project's library is the command's to check.
    pub fn problem(&self) -> Option<String> {
        if self.sources.is_empty() {
            return Some("Nokta bulutunun en az bir dosyası olmalı.".into());
        }
        if self.sources.len() > MAX_CLOUD_SOURCES {
            return Some(format!(
                "Sanal bulutun en çok {MAX_CLOUD_SOURCES} dosyası olabilir."
            ));
        }
        for (i, s) in self.sources.iter().enumerate() {
            let given = |v: &Option<String>| {
                v.as_deref()
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_owned)
            };
            let (a, f, u) = (given(&s.asset), given(&s.file), given(&s.url));
            let n = usize::from(a.is_some()) + usize::from(f.is_some()) + usize::from(u.is_some());
            let raw = usize::from(s.asset.is_some())
                + usize::from(s.file.is_some())
                + usize::from(s.url.is_some());
            if n != 1 || raw != 1 {
                return Some(format!(
                    "{}. dosyanın kaynağı ya gömülü varlık (asset), ya bağlı dosya (file), ya adres (url) olmalı; yalnız biri.",
                    i + 1
                ));
            }
            if let Some(p) = f
                .as_deref()
                .and_then(|p| path_problem("Bağlı dosyanın yolu", p))
            {
                return Some(p);
            }
            if let Some(p) = u.as_deref().and_then(url_problem) {
                return Some(p);
            }
            if bounds_problem(&s.bounds) {
                return Some(format!(
                    "{}. dosyanın kapsamı sonlu altı sayı olmalı, her eksende en küçük en büyükten büyük olmamalı.",
                    i + 1
                ));
            }
        }
        if bounds_problem(&self.bounds) {
            return Some(
                "Nokta bulutunun kapsamı sonlu altı sayı olmalı, her eksende en küçük en büyükten büyük olmamalı.".into(),
            );
        }
        let total: Option<u64> = self
            .sources
            .iter()
            .try_fold(0u64, |a, s| a.checked_add(s.count));
        if total != Some(self.count) {
            return Some("Nokta bulutunun nokta sayısı dosyalarınkinin toplamı olmalı.".into());
        }
        if let Some(p) = self.style.problem() {
            return Some(p);
        }
        if let Some(o) = self.opacity
            && !(o.is_finite() && (MIN_CLOUD_OPACITY..=MAX_CLOUD_OPACITY).contains(&o))
        {
            return Some(format!(
                "Nokta bulutunun donukluğu {MIN_CLOUD_OPACITY} ile {MAX_CLOUD_OPACITY} arasında olmalı; {o} verildi."
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cloud() -> PointCloudFields {
        PointCloudFields {
            sources: vec![CloudSource {
                asset: None,
                file: Some("bulut.laz".into()),
                url: None,
                format: CloudFormat::Laz,
                count: 10,
                bounds: [0.0, 0.0, 0.0, 10.0, 10.0, 5.0],
            }],
            bounds: [0.0, 0.0, 0.0, 10.0, 10.0, 5.0],
            count: 10,
            srid: 5254,
            style: PointCloudStyle {
                render: CloudRender::Elevation,
                ramp: None,
                invert: false,
                min: Some(0.0),
                max: Some(5.0),
                hidden: vec![],
                rgb8: false,
                size: DEFAULT_POINT_SIZE,
                size_unit: PointSizeUnit::Px,
                shape: PointShape::Round,
            },
            opacity: None,
        }
    }

    #[test]
    fn names_are_serdes() {
        fn check<T: Serialize + Copy>(all: &[T], name: impl Fn(T) -> &'static str) {
            for &v in all {
                assert_eq!(
                    serde_json::to_value(v).unwrap(),
                    serde_json::Value::from(name(v))
                );
            }
        }
        check(CloudFormat::ALL, CloudFormat::name);
        check(CloudRender::ALL, CloudRender::name);
        check(PointSizeUnit::ALL, PointSizeUnit::name);
        check(PointShape::ALL, PointShape::name);
    }

    #[test]
    fn problems() {
        assert_eq!(cloud().problem(), None);
        let mut c = cloud();
        c.sources[0].url = Some("https://example.org/a.copc.laz".into());
        assert!(c.problem().unwrap().contains("yalnız biri"));
        let mut c = cloud();
        c.sources[0].file = None;
        c.sources[0].url = Some("ftp://x".into());
        assert!(c.problem().unwrap().contains("HTTP"));
        let mut c = cloud();
        c.count = 11;
        assert!(c.problem().unwrap().contains("toplamı"));
        let mut c = cloud();
        c.style.min = None;
        assert!(c.problem().is_some());
        let mut c = cloud();
        c.style.hidden = vec![7, 2];
        assert!(c.problem().unwrap().contains("küçükten"));
        let mut c = cloud();
        c.opacity = Some(0.05);
        assert!(c.problem().is_some());
        let mut c = cloud();
        c.bounds[3] = -1.0;
        assert!(c.problem().is_some());
    }
}
