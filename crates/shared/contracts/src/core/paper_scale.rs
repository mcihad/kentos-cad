//! Shared physical paper/model ratio; available without the sheet schema feature.
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperScaleRatio"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaperScaleRatio {
    /// Paper and model lengths in the same physical unit: 2:1 enlarges twice.
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
    pub paper: f64,
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
    pub model: f64,
}
impl PaperScaleRatio {
    pub const MIN_DENOMINATOR: f64 = 1e-6;
    pub const MAX_DENOMINATOR: f64 = 1e8;
    pub fn valid_denominator(n: f64) -> bool {
        n.is_finite() && (Self::MIN_DENOMINATOR..=Self::MAX_DENOMINATOR).contains(&n)
    }
    pub fn denominator(self) -> f64 {
        self.model / self.paper
    }
    pub fn valid(self) -> bool {
        self.paper.is_finite()
            && self.model.is_finite()
            && self.paper > 0.
            && self.model > 0.
            && Self::valid_denominator(self.denominator())
    }
}
