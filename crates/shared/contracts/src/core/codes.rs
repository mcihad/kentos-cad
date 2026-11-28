//! The stable codes of the product commands' answers (`CommandError.code`,
//! `CommandWarning.code`; docs/adr/0022 … 0253) and what each means, in one
//! place (docs/adr/0253 §7). Programs branch on these, never on the Turkish
//! message; the catalog lists the codes each command answers with
//! (`CommandCapabilities`) and [`CODES`] says what each means, whether it is
//! a refusal, a conflict or a warning, and whether calling again may help.
//!
//! The desktop's handlers (`kentos_native_application::codes`) use these
//! constants; the sheet core's own refusals (`kentos.sheet/1`) are listed
//! too, as `cad.sheets.edit` answers with them.

use serde::Serialize;

/// How a code comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CodeKind {
    /// A refusal: `failed`, nothing written.
    Failed,
    /// The state changed: `conflict`, nothing written.
    Conflict,
    /// Something only the user can give: `needs_input`.
    NeedsInput,
    /// A warning of a completed command.
    Warning,
    /// A warning when some of the work could not be done, a refusal when none could.
    FailedOrWarning,
    /// The host's own failure, not a command's answer: a Python library's,
    /// a console's or a tool server's (`HostError`, MCP's error).
    Host,
}

/// What a code means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeInfo {
    pub code: &'static str,
    pub kind: CodeKind,
    /// Whether calling again may succeed (prepared again from the state now,
    /// or sent again): a conflict's; never an input wrong in itself.
    pub retryable: bool,
    /// Turkish, for people and for an agent choosing what to do.
    pub meaning: &'static str,
}

/// What `code` means; none for a code this table does not know.
pub fn info(code: &str) -> Option<&'static CodeInfo> {
    CODES.iter().find(|c| c.code == code)
}

/// Whether calling again with `code` may succeed ([`CodeInfo::retryable`]).
pub fn retryable(code: &str) -> bool {
    info(code).is_some_and(|c| c.retryable)
}

macro_rules! codes {
    ($part:ident; $( $(#[$doc:meta])* $name:ident = $code:literal, $kind:ident, $retry:literal, $meaning:literal; )*) => {
        $( $(#[$doc])* pub const $name: &str = $code; )*
        /// This part's codes with what each means, in the file's order.
        pub(super) const $part: &[$crate::codes::CodeInfo] = &[
            $( $crate::codes::CodeInfo {
                code: $code,
                kind: $crate::codes::CodeKind::$kind,
                retryable: $retry,
                meaning: $meaning,
            }, )*
        ];
    };
}

// The table in parts, by what the codes are about (docs/adr/9401 §8). As one file it was past the size limit and could
// take no new code: a command borrowed another's.
#[path = "codes/architecture.rs"]
mod architecture;
#[path = "codes/drawing.rs"]
mod drawing;
#[path = "codes/editing.rs"]
mod editing;
#[path = "codes/rasters_host.rs"]
mod rasters_host;
#[path = "codes/services.rs"]
mod services;
#[path = "codes/sheets_files.rs"]
mod sheets_files;

pub use self::{
    architecture::*, drawing::*, editing::*, rasters_host::*, services::*, sheets_files::*,
};

/// The parts in the table's order: the catalog lists the codes so.
const PARTS: [&[CodeInfo]; 6] = [
    DRAWING_CODES,
    SERVICES_CODES,
    EDITING_CODES,
    SHEETS_FILES_CODES,
    ARCHITECTURE_CODES,
    RASTERS_HOST_CODES,
];

const COUNT: usize = {
    let (mut n, mut p) = (0, 0);
    while p < PARTS.len() {
        n += PARTS[p].len();
        p += 1;
    }
    n
};

// A static: the table is large, and a constant array is copied where it is named.
static ALL: [CodeInfo; COUNT] = {
    let mut all = [CodeInfo {
        code: "",
        kind: CodeKind::Failed,
        retryable: false,
        meaning: "",
    }; COUNT];
    let (mut at, mut p) = (0, 0);
    while p < PARTS.len() {
        let mut i = 0;
        while i < PARTS[p].len() {
            all[at] = PARTS[p][i];
            at += 1;
            i += 1;
        }
        p += 1;
    }
    all
};

/// Every code with what it means, in the parts' order.
pub const CODES: &[CodeInfo] = &ALL;

#[cfg(test)]
#[path = "codes/tests.rs"]
mod tests;
