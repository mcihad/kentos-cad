//! Why the host could not do what it was asked: not a command's refusal
//! (that is its `CommandResult`), but an unknown command, input that is not
//! the command's type, a file that cannot be read or written.

use std::fmt;

/// A failure of the host itself, with a stable code and a Turkish message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadlessError {
    /// Stable, lowercase with underscores. The host's own: `unknown_command`,
    /// `server_command`, `unknown_version`, `unknown_method`, `invalid_input`,
    /// `unknown_object`, `unknown_system`, `unknown_job`, `already_committed`,
    /// `file_unreadable`, `file_unwritable`, `legacy_file`, `no_path`, `busy`,
    /// `unwritable_result`. A file command's refusal (`Session::export_dxf`,
    /// `export_pdf`) keeps the command's code (`file_exists`,
    /// `invalid_file_path`, `nothing_to_export`, `no_sheets`, `unknown_sheet`,
    /// `pdf_unwritable` …); every code's meaning is in `kentos_contracts::codes`.
    pub code: &'static str,
    /// For people: the cause and how to fix it (CLAUDE.md §8).
    pub message: String,
}

impl HeadlessError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for HeadlessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for HeadlessError {}
