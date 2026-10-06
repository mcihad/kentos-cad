//! Pictures in the drawing (docs/adr/0192): a PNG or JPEG placed by its
//! lower left corner, its width and height, turned about the corner and
//! mirrored as a block's insert is. Its bytes are a project library asset
//! (embedded) or a file on disk (linked); it may be clipped to a boundary in
//! its own fractions and drawn see-through.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{EntityBase, Vec2};

/// The least and most opacity a picture is drawn with.
pub const MIN_IMAGE_OPACITY: f64 = 0.1;
pub const MAX_IMAGE_OPACITY: f64 = 1.0;
/// The most corners a clip boundary may have.
pub const MAX_CLIP_CORNERS: usize = 10_000;
/// The longest a picture may be on a side, metres.
pub const MAX_IMAGE_SIZE: f64 = 1.0e7;
/// The longest a linked file's path may be, in letters.
pub const MAX_IMAGE_PATH: usize = 4096;

/// A picture (docs/adr/0192 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImageEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub image: ImageFields,
}

/// What places and shows a picture: the image object's own fields, and its
/// geometry in the commands (`EntityGeometry::Image`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImageFields {
    /// Its lower left corner.
    pub p: Vec2,
    /// Along its bottom edge, metres, over 0.
    pub width: f64,
    /// Along its left edge, metres, over 0.
    pub height: f64,
    /// Radians, counter-clockwise from east, about `p`.
    pub rotation: f64,
    /// The picture mirrored in its own x axis (Aynala), before the turn.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mirror: bool,
    /// The project library's PNG or JPEG asset its bytes are (embedded);
    /// exactly one of `asset` and `file` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// The file it shows (linked): absolute, or relative to the drawing's folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// The part shown, in the picture's own fractions (0,0 its lower left,
    /// 1,1 its upper right): at least three corners inside the unit square;
    /// absent: the whole picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub clip: Option<Vec<Vec2>>,
    /// 0.1 to 1; absent: opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opacity: Option<f64>,
}

impl ImageFields {
    /// Why the fields do not make a picture, in the commands' words; none
    /// when they do. The asset's being in the project's library is the
    /// command's to check.
    pub fn problem(&self) -> Option<String> {
        let finite = |v: f64| v.is_finite();
        if !(finite(self.p.x) && finite(self.p.y) && finite(self.rotation)) {
            return Some("Resmin köşesi ve dönüşü sonlu sayılar olmalı.".into());
        }
        if !(self.width > 0.0
            && self.width <= MAX_IMAGE_SIZE
            && self.height > 0.0
            && self.height <= MAX_IMAGE_SIZE)
        {
            return Some(format!(
                "Resmin genişliği ve yüksekliği sıfırdan büyük ve en çok {MAX_IMAGE_SIZE} m olmalı."
            ));
        }
        let asset = self
            .asset
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let file = self
            .file
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        match (asset, file, &self.asset, &self.file) {
            (Some(_), None, _, None) | (None, Some(_), None, _) => {}
            (Some(_), Some(_), ..) => {
                return Some("Resmin kaynağı ya gömülü varlık (asset) ya bağlı dosya (file) olmalı, ikisi birden değil.".into());
            }
            _ => {
                return Some("Resmin kaynağı yok: gömülü varlığın kimliğini (asset) ya da bağlı dosyanın yolunu (file) verin.".into());
            }
        }
        if file
            .is_some_and(|f| f.chars().count() > MAX_IMAGE_PATH || f.chars().any(char::is_control))
        {
            return Some(format!(
                "Bağlı dosyanın yolu en çok {MAX_IMAGE_PATH} harf olmalı ve denetim karakteri içermemeli."
            ));
        }
        if let Some(clip) = &self.clip {
            let inside = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
            if clip.len() < 3
                || clip.len() > MAX_CLIP_CORNERS
                || !clip.iter().all(|q| inside(q.x) && inside(q.y))
            {
                return Some(format!(
                    "Resmin kırpma sınırı en az 3, en çok {MAX_CLIP_CORNERS} köşe olmalı, köşeleri resmin kesirleriyle 0 ile 1 arasında."
                ));
            }
        }
        if let Some(o) = self.opacity
            && !(o.is_finite() && (MIN_IMAGE_OPACITY..=MAX_IMAGE_OPACITY).contains(&o))
        {
            return Some(format!(
                "Resmin donukluğu {MIN_IMAGE_OPACITY} ile {MAX_IMAGE_OPACITY} arasında olmalı; {o} verildi."
            ));
        }
        None
    }
}
