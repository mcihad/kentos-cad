//! What the engine is (`engineInfo` at the WASM boundary): its version, the
//! schemas it reads and writes, the typefaces it measures, the preflight's
//! placeholder codes and the constants of a drag, so a platform names none
//! of them itself.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::model::{ASSET_MAX_BYTES, BOOK_SCHEMA, TEMPLATE_ASSETS_MAX_BYTES};
use crate::preflight::PLACEHOLDERS;
use crate::snap::{NUDGE, NUDGE_ALT, NUDGE_SHIFT, TOLERANCE_PX};
use crate::template::TEMPLATE_SCHEMA;
use crate::units::Um;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct EngineInfo {
    /// The core's version (`kentos-sheet`).
    pub version: String,
    /// The book a project keeps (`kentos.sheet/1`).
    pub book_schema: String,
    /// The template file (`kentos.sheet.template/1`).
    pub template_schema: String,
    /// The typefaces whose metrics the core has (`DRAWING_FONTS` ids), in its order.
    pub fonts: Vec<String>,
    /// The preflight codes that are values only the user gives.
    pub placeholders: Vec<String>,
    /// An arrow key's nudge: plain, with Shift, with Alt (micrometres).
    pub nudge: [Um; 3],
    /// The snapping tolerance in screen pixels (the caller turns it into micrometres at its zoom).
    pub tolerance_px: u32,
    /// The largest picture a book keeps, in bytes.
    pub asset_max_bytes: u32,
    /// The largest total of a template's pictures, in bytes.
    pub template_assets_max_bytes: u32,
}

pub fn engine_info() -> EngineInfo {
    EngineInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        book_schema: BOOK_SCHEMA.to_owned(),
        template_schema: TEMPLATE_SCHEMA.to_owned(),
        fonts: crate::text::fonts()
            .into_iter()
            .map(str::to_owned)
            .collect(),
        placeholders: PLACEHOLDERS.iter().map(|p| (*p).to_owned()).collect(),
        nudge: [NUDGE, NUDGE_SHIFT, NUDGE_ALT],
        tolerance_px: TOLERANCE_PX,
        asset_max_bytes: ASSET_MAX_BYTES,
        template_assets_max_bytes: u32::try_from(TEMPLATE_ASSETS_MAX_BYTES).unwrap_or(u32::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_says_what_it_is() {
        let i = engine_info();
        assert_eq!(i.book_schema, "kentos.sheet/1");
        assert_eq!(i.template_schema, "kentos.sheet.template/1");
        assert!(i.fonts.iter().any(|f| f == "barlow"));
        assert_eq!(i.nudge, [1_000, 10_000, 100]);
        assert_eq!(i.template_assets_max_bytes, 8 * 1024 * 1024);
    }
}
