//! An insert's clipping (docs/adr/0351 §1; plan/cad.md `CAD-REF-12`, AutoCAD's XCLIP): a block's, an external
//! reference's or a PDF underlay's insert draws only the part of its content inside a boundary, or outside it; its
//! frame is hidden, shown on the screen only, or drawn and plotted. The boundary is kept in the definition's
//! coordinates, so moving, turning or scaling the insert takes it along. `cad.inserts.clip` sets, changes and removes
//! it, each one undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::Vec2;

pub const CAD_INSERTS_CLIP: &str = "cad.inserts.clip";
pub const CAD_INSERTS_CLIP_VERSION: u32 = 1;

/// The most corners a clipping boundary has.
pub const MAX_CLIPPING_CORNERS: usize = 10_000;

/// How a clipping boundary's frame shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ClipFrame {
    /// Not drawn.
    #[default]
    Hide,
    /// Drawn on the screen over the drawing, never plotted.
    Show,
    /// Drawn with the content, in the insert's look, and plotted.
    Plot,
}

fn is_hide(f: &ClipFrame) -> bool {
    *f == ClipFrame::Hide
}

/// An insert's clipping boundary (docs/adr/0351 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InsertClip {
    /// Its corners in the definition's coordinates (relative to nothing: the definition's own), at least three, the
    /// ring closing by itself; no edge crosses another.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub boundary: Vec<Vec2>,
    /// The content outside the boundary is drawn, the inside left out.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub inverted: bool,
    #[serde(default, skip_serializing_if = "is_hide")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<ClipFrame>"))]
    pub frame: ClipFrame,
}

impl InsertClip {
    /// What is wrong with it, in the commands' words: too few or too many corners, a corner not finite, no area.
    /// Crossing edges are the command's to find (the core's).
    pub fn problem(&self) -> Option<String> {
        let n = self.boundary.len();
        if n < 3 {
            return Some("Kırpma sınırının en az üç köşesi olmalı.".into());
        }
        if n > MAX_CLIPPING_CORNERS {
            return Some(format!("Kırpma sınırı en çok {MAX_CLIPPING_CORNERS} köşeli olabilir."));
        }
        if self.boundary.iter().any(|p| !(p.x.is_finite() && p.y.is_finite())) {
            return Some("Kırpma sınırının köşeleri sonlu sayılar olmalı.".into());
        }
        let mut twice = 0.0;
        for i in 0..n {
            let (a, b) = (self.boundary[i], self.boundary[(i + 1) % n]);
            twice += a.x * b.y - b.x * a.y;
        }
        if twice.abs().partial_cmp(&1e-12) != Some(std::cmp::Ordering::Greater) {
            return Some("Kırpma sınırının alanı yok: köşeleri bir doğru üstünde ya da üst üste.".into());
        }
        None
    }
}

/// A boundary as given to `cad.inserts.clip`, in the drawing's coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ClipBoundary {
    /// A rectangle along the drawing's axes, by two opposite corners.
    Rectangle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
    },
    /// A polygon by its corners, the ring closing by itself.
    Polygon {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        points: Vec<Vec2>,
    },
}

/// What `cad.inserts.clip` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum InsertsClipOperation {
    /// The inserts clipped by `boundary` (an earlier boundary replaced); `inverted` keeps the outside.
    Set {
        boundary: ClipBoundary,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        inverted: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<ClipFrame>"))]
        frame: ClipFrame,
    },
    /// Their boundaries removed: the whole content drawn again.
    Remove,
    /// Their frames shown as `frame`.
    Frame { frame: ClipFrame },
    /// Inside and outside swapped.
    Invert,
}

/// Input of `cad.inserts.clip` v1 (Kırp: blok ve dış başvuru), one undo step “Kırp”. Inserts on a locked layer are
/// left out and counted (docs/adr/0278).
///
/// Refusals, in this order: `no_entities`, `invalid_uid`, `entity_not_found`, `not_an_insert`, `layer_locked`,
/// `invalid_clipping` (the boundary has too few corners, no area, a corner not finite, crossing edges), `not_clipped`
/// (Remove, Frame or Invert on inserts none of which is clipped), `invalid_revision`, `revision_conflict`. Warnings:
/// `layer_locked`, `clip_empty` (the boundary leaves an insert nothing to draw).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InsertsClip {
    /// The inserts' persistent ids.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub operation: InsertsClipOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// An insert's clipping after the command.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InsertClipped {
    pub uid: String,
    /// Its clipping now, absent once removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub clipping: Option<InsertClip>,
}

/// Output of `cad.inserts.clip` v1 (its plan's too).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InsertsClipped {
    /// The inserts changed, in the drawing's order.
    pub inserts: Vec<InsertClipped>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
