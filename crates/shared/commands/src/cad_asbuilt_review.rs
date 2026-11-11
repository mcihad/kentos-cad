//! `cad.asbuilt.review` v1 (plan/map-cad.md MAP-19, docs/adr/5220): İzleme incelemesi. A monitoring job's check
//! (`cad.asbuilt.check` with `kind: monitoring`) kept in the project under a name with the reviewer's statement: the
//! job as asked, its thresholds with their source, each mark's last movement, speed and state, and the digest of the
//! points the job read. The same name is replaced; `remove` removes it. Every kept review is listed with whether the
//! points read are still the same (`current`): changed, the review belongs to the earlier result. KentOS writes what
//! the reviewer states; it neither checks their authority nor signs. One undo step. The shared cases are
//! `fixtures/commands/v1/cad.asbuilt.review.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{AsbuiltCheck, MonitoringReview, MonitoringReviewer, ReviewPhrase};

pub const CAD_ASBUILT_REVIEW: &str = "cad.asbuilt.review";
pub const CAD_ASBUILT_REVIEW_VERSION: u32 = 1;

/// Input of `cad.asbuilt.review` v1. To keep: `check` (a monitoring job), `reviewer` and, with the check's thresholds,
/// `thresholdsSource` (who set them, by which document, when; the program gives none). To remove: `remove` alone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltReview {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub check: Option<AsbuiltCheck>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub thresholds_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reviewer: Option<MonitoringReviewer>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub remove: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// A kept review in the list: its name, the reviewer's name, day and phrase, its marks in alarm and warning, and
/// whether the points its job reads are the ones reviewed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltReviewRow {
    pub name: String,
    pub reviewer: String,
    pub date: String,
    pub phrase: ReviewPhrase,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub alarm: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub warning: u32,
    pub current: bool,
}

/// Output of `cad.asbuilt.review` v1: the review kept (none when removed), whether one of its name was replaced or
/// removed, every kept review in the project's order and the revision after the write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltReviewed {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kept: Option<MonitoringReview>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub replaced: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub removed: bool,
    pub reviews: Vec<AsbuiltReviewRow>,
    pub revision: String,
}
