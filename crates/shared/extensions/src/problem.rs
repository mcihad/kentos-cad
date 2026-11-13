//! What is wrong with a package, said so that the person or the agent that made it can mend it in one step: a stable
//! code (for a test and for an agent's repair loop), the file and the place in it, and the fix in Turkish.

use std::fmt;

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    /// Stable: `path_unsafe`, `manifest_unknown_field`, `integrity_mismatch` …
    pub code: &'static str,
    /// The file in the package it is about, when one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Where in the file: a JSON pointer (`/entrypoints/processing/0/handler`) or a line (`satır 12`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// What is wrong and how to fix it.
    pub message: String,
}

impl Problem {
    pub fn new(code: &'static str, message: impl Into<String>) -> Problem {
        Problem {
            code,
            file: None,
            at: None,
            message: message.into(),
        }
    }

    pub fn file(mut self, file: impl Into<String>) -> Problem {
        self.file = Some(file.into());
        self
    }

    pub fn at(mut self, at: impl Into<String>) -> Problem {
        self.at = Some(at.into());
        self
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            f.write_str(file)?;
            if let Some(at) = &self.at {
                write!(f, " {at}")?;
            }
            f.write_str(": ")?;
        }
        write!(f, "{} ({})", self.message, self.code)
    }
}
