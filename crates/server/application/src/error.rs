//! Errors a use case can end in. Each one maps to one HTTP status and a
//! Turkish message that says what happened and what to do (CLAUDE.md §8).
//! Database errors keep their detail for the log, never for the client.

use std::fmt;

use kentos_contracts::FeatureConflict;

#[derive(Debug)]
pub enum AppError {
    /// No valid session or token (401).
    Unauthenticated(String),
    /// Signed in, but not allowed (403). Also used when a membership or seat is missing.
    Forbidden(String),
    /// The thing does not exist, or this actor may not know it exists (404).
    NotFound(String),
    /// The request is malformed or breaks a rule (400/422).
    Invalid(String),
    /// The project was deleted (moved to the trash): kept for recovery, but it can no longer be opened or changed (410).
    Deleted(String),
    /// The project is archived: it opens, but nothing in it changes until it is unarchived (409).
    Archived(String),
    /// The events after this cursor are no longer kept (or never existed): reopen the project (410).
    ResyncRequired(String),
    /// Someone changed what this edit was based on (409); nothing was written.
    Conflict {
        message: String,
        conflicts: Vec<FeatureConflict>,
    },
    /// Too many attempts; try again after this many seconds (429).
    Limited { message: String, retry_after: u64 },
    /// The database failed; the detail is logged, the client sees a generic message (500/503).
    Database(sqlx::Error),
}

impl AppError {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }
    pub fn deleted(message: impl Into<String>) -> Self {
        Self::Deleted(message.into())
    }
    pub fn archived(message: impl Into<String>) -> Self {
        Self::Archived(message.into())
    }

    /// A stable machine-readable code for the client (`error` in the response).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unauthenticated(_) => "unauthenticated",
            Self::Forbidden(_) => "forbidden",
            Self::NotFound(_) => "not_found",
            Self::Invalid(_) => "invalid",
            Self::Deleted(_) => "project_deleted",
            Self::Archived(_) => "project_archived",
            Self::ResyncRequired(_) => "resync_required",
            Self::Conflict { .. } => "conflict",
            Self::Limited { .. } => "rate_limited",
            Self::Database(e) if is_unavailable(e) => "unavailable",
            Self::Database(_) => "internal",
        }
    }
}

/// Pool exhausted or connection lost: worth retrying, unlike a failed statement.
fn is_unavailable(e: &sqlx::Error) -> bool {
    matches!(
        e,
        sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_)
    )
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthenticated(m)
            | Self::Forbidden(m)
            | Self::NotFound(m)
            | Self::Invalid(m)
            | Self::Deleted(m)
            | Self::Archived(m)
            | Self::ResyncRequired(m) => f.write_str(m),
            Self::Conflict { message, .. } | Self::Limited { message, .. } => f.write_str(message),
            Self::Database(e) if is_unavailable(e) => {
                f.write_str("Veritabanına şu an ulaşılamıyor. Birazdan yeniden deneyin.")
            }
            Self::Database(_) => {
                f.write_str("Sunucuda beklenmeyen bir hata oldu; ayrıntı sunucu günlüğünde.")
            }
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

pub type AppResult<T> = Result<T, AppError>;
