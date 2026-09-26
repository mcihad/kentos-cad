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
    /// The request is malformed or breaks a rule (400/422); `path` names the
    /// field when it is known (relative to a command's input, as in
    /// `CommandError`, or a query parameter's name).
    Invalid {
        message: String,
        path: Option<String>,
    },
    /// The project was deleted (moved to the trash): kept for recovery, but it can no longer be opened or changed (410).
    Deleted(String),
    /// The project is archived: it opens, but nothing in it changes until it is unarchived (409).
    Archived(String),
    /// The events after this cursor are no longer kept (or never existed): reopen the project (410).
    ResyncRequired(String),
    /// Someone changed what this edit was based on (409); nothing was written.
    /// `revision` is the project's data revision now, when the conflict is about the data.
    Conflict {
        message: String,
        conflicts: Vec<FeatureConflict>,
        revision: Option<i64>,
    },
    /// Too many attempts; try again after this many seconds (429).
    Limited { message: String, retry_after: u64 },
    /// The database failed; the detail is logged, the client sees a generic message (500/503).
    Database(sqlx::Error),
}

impl AppError {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid {
            message: message.into(),
            path: None,
        }
    }
    /// An invalid request whose field is known (`name`, `tags[2]`, `q`).
    pub fn invalid_at(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid {
            message: message.into(),
            path: Some(path.into()),
        }
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
            Self::Invalid { .. } => "invalid",
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

impl AppError {
    /// Whether the same request may be sent again unchanged, and the seconds
    /// to wait when the server knows them: a rate limit says how long, a
    /// database briefly away does not. Everything else needs a change first.
    pub fn retry(&self) -> (bool, Option<u32>) {
        match self {
            Self::Limited { retry_after, .. } => {
                (true, Some(u32::try_from(*retry_after).unwrap_or(u32::MAX)))
            }
            Self::Database(e) if is_unavailable(e) => (true, None),
            _ => (false, None),
        }
    }

    /// The same error about this field: an invalid request gets the path, any other error stays as it is.
    pub fn at(self, field: impl Into<String>) -> Self {
        match self {
            Self::Invalid { message, .. } => Self::Invalid {
                message,
                path: Some(field.into()),
            },
            other => other,
        }
    }

    /// The field an invalid request is about, when it is known.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Invalid { path, .. } => path.as_deref(),
            _ => None,
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
            | Self::Deleted(m)
            | Self::Archived(m)
            | Self::ResyncRequired(m) => f.write_str(m),
            Self::Invalid { message, .. }
            | Self::Conflict { message, .. }
            | Self::Limited { message, .. } => f.write_str(message),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// What a caller needs besides the message (TODOS.md ARCH-07).
    #[test]
    fn an_error_says_its_field_and_whether_to_send_again() {
        let plain = AppError::invalid("Proje adı boş olamaz.");
        assert_eq!((plain.code(), plain.path()), ("invalid", None));
        let named = AppError::invalid_at("name", "Proje adı boş olamaz.");
        assert_eq!(
            (named.path(), named.to_string().as_str()),
            (Some("name"), "Proje adı boş olamaz.")
        );
        // A field given later replaces none but an invalid request's.
        assert_eq!(plain.at("tags[2]").path(), Some("tags[2]"));
        assert_eq!(AppError::forbidden("Yetki yok.").at("name").path(), None);
        let limited = AppError::Limited {
            message: "Çok fazla deneme.".into(),
            retry_after: 90,
        };
        assert_eq!(limited.retry(), (true, Some(90)));
        assert_eq!(AppError::invalid("x").retry(), (false, None));
        assert_eq!(
            AppError::Database(sqlx::Error::PoolTimedOut).retry(),
            (true, None)
        );
        assert_eq!(
            AppError::Database(sqlx::Error::RowNotFound).retry(),
            (false, None)
        );
    }
}
