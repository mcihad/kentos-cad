//! Mahal renk planı (docs/adr/8176): the plan's rooms filled by what a scheme reads of each, each value's colour the
//! same in every storey and in the legend.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// What a colour plan colours the rooms by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ColorScheme {
    /// Kullanım: the room's use as written, else the group its name reads as (oturma, yatma, ıslak hacim …).
    #[default]
    Usage,
    /// Bağımsız bölüm: the program's unit the room belongs to (a flat, a common area, an outdoor area).
    Unit,
}

impl ColorScheme {
    pub const ALL: [ColorScheme; 2] = [ColorScheme::Usage, ColorScheme::Unit];

    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            ColorScheme::Usage => "Kullanım",
            ColorScheme::Unit => "Bağımsız bölüm",
        }
    }
}

/// A colour plan: its scheme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ColorPlan {
    pub scheme: ColorScheme,
}
