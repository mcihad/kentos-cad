//! Why a driver could not do what it was asked (docs/adr/9100 §6), each said in plain Turkish with the way out, so a
//! window shows it as it is and a script reads its kind.

use std::fmt;

/// Why a source could not be reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// The server's name does not resolve, nothing listens, the network does not reach it; a file is not there.
    Offline,
    /// No answer in time.
    Timeout,
    /// The login is refused, or the account may not read.
    Denied,
    /// Anything else (TLS, a certificate, the server at its connection limit, a locked file).
    Failed,
}

/// The kind of a constraint that refused a write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintKind {
    Check,
    Unique,
    Foreign,
    NotNull,
}

impl ConstraintKind {
    fn words(self) -> &'static str {
        match self {
            ConstraintKind::Check => "denetim kuralı",
            ConstraintKind::Unique => "teklik kuralı",
            ConstraintKind::Foreign => "başvuru kuralı",
            ConstraintKind::NotNull => "boş geçilemez kuralı",
        }
    }
}

/// A driver's failure.
#[derive(Clone, Debug, PartialEq)]
pub enum DataError {
    /// The source cannot be reached or refuses the login.
    Unreachable { reach: Reach, message: String },
    /// No such table.
    NoTable(String),
    /// No project of this name kept in the source (docs/adr/9107).
    NoProject(String),
    /// No such field in the table.
    NoField { table: String, field: String },
    /// The account may not do this (no privilege, a row security policy, a read-only file).
    Denied(String),
    /// The source or the driver cannot do this (a view written, a type it has no word for).
    Unsupported(String),
    /// The request is not well formed (an empty name, a value of the wrong kind).
    Invalid(String),
    /// One of the table's constraints refused the write; nothing was written.
    Constraint {
        kind: ConstraintKind,
        name: Option<String>,
        column: Option<String>,
        detail: String,
    },
    /// Rows changed by someone else since they were read; nothing was written.
    Conflict(String),
    /// Stopped on request.
    Cancelled,
    /// The source refused for another reason; its words.
    Failed(String),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::Unreachable { message, .. } => f.write_str(message),
            DataError::NoTable(t) => write!(
                f,
                "“{t}” tablosu kaynakta yok; adını ya da şemasını denetleyin."
            ),
            DataError::NoProject(p) => write!(
                f,
                "“{p}” projesi bu kaynakta yok; adını denetleyin ya da listeden seçin."
            ),
            DataError::NoField { table, field } => {
                write!(f, "“{table}” tablosunda “{field}” alanı yok.")
            }
            DataError::Denied(m) => write!(f, "İzin yok: {m}"),
            DataError::Unsupported(m) => write!(f, "Bu kaynakta yapılamaz: {m}"),
            DataError::Invalid(m) => f.write_str(m),
            DataError::Constraint {
                kind,
                name,
                column,
                detail,
            } => {
                write!(f, "Tablonun {}", kind.words())?;
                if let Some(n) = name {
                    write!(f, " (“{n}”)")?;
                }
                if let Some(c) = column {
                    write!(f, " “{c}” alanında")?;
                }
                write!(f, " yazmayı reddetti; hiçbir şey yazılmadı. {detail}")
            }
            DataError::Conflict(m) => write!(
                f,
                "Kayıtlar siz okuduktan sonra başkasınca değiştirildi; hiçbir şey yazılmadı. {m}"
            ),
            DataError::Cancelled => f.write_str("Durduruldu."),
            DataError::Failed(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for DataError {}

impl DataError {
    /// The error's code for scripts: `unreachable`, `no_table` …
    pub fn code(&self) -> &'static str {
        match self {
            DataError::Unreachable { .. } => "unreachable",
            DataError::NoTable(_) => "no_table",
            DataError::NoProject(_) => "no_project",
            DataError::NoField { .. } => "no_field",
            DataError::Denied(_) => "denied",
            DataError::Unsupported(_) => "unsupported",
            DataError::Invalid(_) => "invalid",
            DataError::Constraint { .. } => "constraint",
            DataError::Conflict(_) => "conflict",
            DataError::Cancelled => "cancelled",
            DataError::Failed(_) => "failed",
        }
    }
}
