//! Why a cloud request failed, as the web's `ApiFailure` tells it
//! (apps/web/src/app/cloud/api.ts): the server's stable code and Turkish
//! message (`ApiError`, TODOS.md ARCH-07), or `network`/`timeout` when no
//! answer came, and whether the same request may go again unchanged.

use std::fmt;
use std::ops::Deref;
use std::time::Duration;

use kentos_contracts::{ApiError, FeatureConflict};

/// A failed request (its [`Failure`] behind a pointer: a `Result` stays small).
#[derive(Clone, Debug, PartialEq)]
pub struct ApiFailure(Box<Failure>);

impl Deref for ApiFailure {
    type Target = Failure;

    fn deref(&self) -> &Failure {
        &self.0
    }
}

/// What went wrong. The message is for people and says the cause and the fix.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    /// HTTP status; 0 when no answer came from the server.
    pub status: u16,
    /// The server's stable code (`conflict`, `forbidden` …); `network` or
    /// `timeout` when no answer came; `answer` when the answer could not be
    /// read; `corrupt` when a file arrived other than the server said;
    /// `drawing` when what came cannot become a drawing here; `local` when
    /// the request never left this program.
    pub code: String,
    pub message: String,
    /// The objects in conflict (`conflict` of `project.changes`).
    pub conflicts: Vec<FeatureConflict>,
    pub request_id: Option<String>,
    /// The field the error is about, when the server knows it (`name`, `entities[13]`).
    pub path: Option<String>,
    /// The project's revision now, when the error depends on it (a conflict).
    pub revision: Option<String>,
    /// The server says the same request may go again unchanged.
    pub retryable: bool,
    /// Seconds the server asks to wait before that (a rate limit).
    pub retry_after: Option<u32>,
}

impl ApiFailure {
    /// A failure with this status (0: no answer), code and message, and nothing more.
    pub fn new(status: u16, code: &str, message: impl Into<String>) -> Self {
        Self(Box::new(Failure {
            status,
            code: code.into(),
            message: message.into(),
            conflicts: Vec::new(),
            request_id: None,
            path: None,
            revision: None,
            retryable: false,
            retry_after: None,
        }))
    }

    fn plain(status: u16, code: &str, message: String) -> Self {
        Self::new(status, code, message)
    }

    /// The same failure with these objects in conflict.
    pub fn with_conflicts(mut self, conflicts: Vec<FeatureConflict>) -> Self {
        self.0.conflicts = conflicts;
        self
    }

    /// The same failure under another code.
    pub(crate) fn with_code(mut self, code: &str) -> Self {
        self.0.code = code.into();
        self
    }

    /// The server's error answer.
    pub(crate) fn answered(status: u16, body: ApiError, retry_header: Option<u32>) -> Self {
        Self(Box::new(Failure {
            status,
            code: body.error,
            message: body.message,
            conflicts: body.conflicts.unwrap_or_default(),
            request_id: body.request_id,
            path: body.path,
            revision: body.revision,
            retryable: body.retryable,
            retry_after: body.retry_after.or(retry_header),
        }))
    }

    /// An error status whose body is not the server's error (a proxy's page, say).
    pub(crate) fn status_only(status: u16) -> Self {
        Self::plain(status, "http", format!("Sunucu {status} yanıtı verdi."))
    }

    /// No answer came: no connection, a timeout, a connection cut short.
    pub(crate) fn unreachable(e: &reqwest::Error, waited: Duration) -> Self {
        if e.is_timeout() {
            Self::plain(
                0,
                "timeout",
                format!(
                    "Sunucu {} saniyede yanıt vermedi; bağlantınızı denetleyip yeniden deneyin.",
                    waited.as_secs()
                ),
            )
        } else {
            Self::plain(
                0,
                "network",
                "Sunucuya ulaşılamadı; bağlantınızı ve sunucu adresini denetleyin.".into(),
            )
        }
    }

    /// An answer this program cannot read (`what` names it).
    pub(crate) fn unreadable(status: u16, what: &str, why: impl fmt::Display) -> Self {
        Self::plain(
            status,
            "answer",
            format!(
                "Sunucunun yanıtı okunamadı ({what}: {why}). Sunucu bu sürümle uyumlu olmayabilir."
            ),
        )
    }

    /// The request never left this program.
    pub(crate) fn local(message: impl Into<String>) -> Self {
        Self::plain(0, "local", message.into())
    }

    /// What the server sent of project `name` cannot become a drawing here.
    pub(crate) fn drawing(name: &str, why: impl fmt::Display) -> Self {
        Self::plain(0, "drawing", format!("“{name}” açılamadı: {why}"))
    }
}

impl Failure {
    /// Worth sending again unchanged: the server says so (a rate limit, its
    /// database briefly away), or no answer came from it at all — no
    /// connection, a timeout, a gateway in front of it — or a file came
    /// damaged on the way.
    pub fn transient(&self) -> bool {
        self.retryable
            || matches!(self.code.as_str(), "network" | "timeout" | "corrupt")
            || matches!(self.status, 408 | 502 | 503 | 504)
    }

    /// The project is in the trash (410): nothing more can be read from it or saved to it.
    pub fn deleted(&self) -> bool {
        self.code == "project_deleted"
    }

    /// The project is archived (409): it can be read, not changed (docs/adr/0028).
    pub fn archived(&self) -> bool {
        self.code == "project_archived"
    }

    /// Not there for this account (404): it never existed, or its access was
    /// taken away; the server answers both alike (docs/adr/0015).
    pub fn not_found(&self) -> bool {
        self.code == "not_found"
    }

    /// Someone saved first: the request was based on an older version.
    pub fn conflict(&self) -> bool {
        self.code == "conflict"
    }

    /// The session is gone (401): sign in again.
    pub fn signed_out(&self) -> bool {
        self.status == 401 || self.code == "unauthenticated"
    }

    /// How long to wait before try `tries` (1, 2 …) of a transient failure:
    /// 1 s doubling up to 30 s, never sooner than the server asked.
    pub fn backoff(&self, tries: u32) -> Duration {
        let doubling = 1000u64 << tries.saturating_sub(1).min(5);
        let asked = u64::from(self.retry_after.unwrap_or(0)) * 1000;
        Duration::from_millis(doubling.min(30_000).max(asked))
    }
}

impl fmt::Display for ApiFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApiFailure {}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(error: &str, retryable: bool, retry_after: Option<u32>) -> ApiError {
        ApiError {
            error: error.into(),
            message: "İleti".into(),
            request_id: Some("istek-1".into()),
            conflicts: None,
            path: Some("name".into()),
            revision: None,
            retryable,
            retry_after,
        }
    }

    #[test]
    fn the_server_says_what_may_go_again() {
        let limited = ApiFailure::answered(429, body("rate_limited", true, Some(90)), None);
        assert!(limited.transient());
        assert_eq!(limited.backoff(1), Duration::from_secs(90));
        let invalid = ApiFailure::answered(422, body("invalid", false, None), None);
        assert!(!invalid.transient());
        assert_eq!(invalid.path.as_deref(), Some("name"));
        // A gateway in front of the server, or no answer at all: again, later.
        assert!(ApiFailure::status_only(502).transient());
        assert!(ApiFailure::plain(0, "network", String::new()).transient());
        // Something this program refused before sending never goes again by itself.
        assert!(!ApiFailure::local("yerel").transient());
        // The header's wait counts when the body has none.
        let header = ApiFailure::answered(503, body("unavailable", true, None), Some(5));
        assert_eq!(header.retry_after, Some(5));
    }

    #[test]
    fn the_wait_doubles_up_to_half_a_minute() {
        let f = ApiFailure::status_only(503);
        let waits: Vec<u64> = (1..=8).map(|t| f.backoff(t).as_secs()).collect();
        assert_eq!(waits, [1, 2, 4, 8, 16, 30, 30, 30]);
    }

    #[test]
    fn the_codes_name_what_happened() {
        let f = |code: &str, status| ApiFailure::answered(status, body(code, false, None), None);
        assert!(f("project_deleted", 410).deleted());
        assert!(f("project_archived", 409).archived());
        assert!(f("not_found", 404).not_found());
        assert!(f("conflict", 409).conflict());
        assert!(f("unauthenticated", 401).signed_out());
    }
}
