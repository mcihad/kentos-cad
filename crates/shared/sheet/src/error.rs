//! The sheet core's one error: a stable code a program decides by, a Turkish
//! message that says what is wrong and how to put it right (CLAUDE.md §8),
//! and where in the input it is, when that is known.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetError {
    /// Stable, `snake_case` (`duplicate_id`, `unknown_item`, `bad_json` …).
    pub code: String,
    /// Turkish: the cause and the fix.
    pub message: String,
    /// Where: `sheets[0].items[2].frame.width`, an item's id, a JSON path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
}

impl SheetError {
    pub fn new(code: &str, message: impl Into<String>) -> SheetError {
        SheetError {
            code: code.to_owned(),
            message: message.into(),
            path: None,
        }
    }

    pub fn at(code: &str, path: impl Into<String>, message: impl Into<String>) -> SheetError {
        SheetError {
            code: code.to_owned(),
            message: message.into(),
            path: Some(path.into()),
        }
    }

    /// A JSON text that does not read as the type it should be.
    pub fn json(what: &str, e: &serde_json::Error) -> SheetError {
        SheetError::new(
            "bad_json",
            format!(
                "{what} okunamadı: {e}. Uygulama ile pafta çekirdeği uyuşmuyor olabilir; sayfayı yenileyin ya da dosyayı denetleyin."
            ),
        )
    }
}

impl std::fmt::Display for SheetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(p) => write!(f, "{} ({}): {}", self.code, p, self.message),
            None => write!(f, "{}: {}", self.code, self.message),
        }
    }
}

impl std::error::Error for SheetError {}

pub type Result<T> = std::result::Result<T, SheetError>;
