//! The catalog's words (docs/adr/8059 §10): which catalog, which release, who calls and how, the moves a request asks
//! for, and why a call did not go through. The service ([`super::Catalog`]) and the routes speak them.

use std::fmt;

use kentos_extensions::release::{
    CatalogScope, Channel, GateKind, Refusal, Release, ReleaseState, RequestedChange, ScanOutcome,
};
use serde::Deserialize;

use super::store::StoreError;

/// A catalog: the public one, an organisation's, an account's own.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CatalogId {
    Public,
    /// An organisation's, by its short name.
    Tenant(String),
    /// An account's own, by the account's id.
    Personal(String),
}

impl CatalogId {
    /// `public`, `tenant:{slug}` or `personal:{account}`; none for anything else.
    pub fn parse(text: &str) -> Option<CatalogId> {
        if text == "public" {
            return Some(CatalogId::Public);
        }
        if let Some(slug) = text.strip_prefix("tenant:") {
            return is_slug(slug).then(|| CatalogId::Tenant(slug.to_owned()));
        }
        let account = text.strip_prefix("personal:")?;
        is_account(account).then(|| CatalogId::Personal(account.to_owned()))
    }

    /// The flow's word for it: who sees a release there, and who may approve it.
    pub fn scope(&self) -> CatalogScope {
        match self {
            CatalogId::Public => CatalogScope::Public,
            CatalogId::Tenant(_) => CatalogScope::Tenant,
            CatalogId::Personal(_) => CatalogScope::Private,
        }
    }
}

impl fmt::Display for CatalogId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogId::Public => f.write_str("public"),
            CatalogId::Tenant(slug) => write!(f, "tenant:{slug}"),
            CatalogId::Personal(account) => write!(f, "personal:{account}"),
        }
    }
}

/// An organisation's short name as the server's tenants have it: `[a-z0-9][a-z0-9-]{1,62}`.
fn is_slug(text: &str) -> bool {
    let b = text.as_bytes();
    (2..=63).contains(&b.len())
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// An account's id: a UUID on a running server.
fn is_account(text: &str) -> bool {
    (1..=64).contains(&text.len()) && text.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

/// A release's place: its catalog, its extension and its version as uploaded.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReleaseKey {
    pub catalog: CatalogId,
    pub extension: String,
    pub version: String,
}

/// Who calls, and how the call came (PY-CLOUD-18).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    /// The account's id: the server's user id.
    pub id: String,
    pub via: Channel,
}

/// How a catalog route learns who calls it.
pub enum Accounts {
    /// The server's sessions and access tokens, as on every protected route.
    Sessions,
    /// The tests': the account the `x-kentos-test-account` header names, so the flow runs without a database. It does
    /// not exist in a server's build.
    #[cfg(test)]
    Named,
}

/// A move a request asks for: the flow's own moves with what each takes. Uploading and new content come as a package's
/// bytes, not as a move.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum Move {
    Submit {
        #[serde(default)]
        note: Option<String>,
    },
    /// A scanner's report on one gate, for the content it scanned.
    Scan {
        gate: GateKind,
        content: String,
        outcome: ScanOutcome,
    },
    /// A reviewer takes the release; its focus is computed here from the packages, never sent.
    StartReview,
    RequestChanges {
        changes: Vec<RequestedChange>,
    },
    Waive {
        gate: GateKind,
        reason: String,
    },
    Approve {
        #[serde(default)]
        note: Option<String>,
    },
    Publish,
    Suspend {
        reason: String,
    },
    Reinstate {
        reason: String,
    },
    Withdraw {
        reason: String,
    },
    Archive {
        #[serde(default)]
        reason: Option<String>,
    },
}

/// Why a catalog call did not go through.
#[derive(Debug)]
pub enum CatalogError {
    /// The flow refused the move, or the catalog's own rule did: a stable code, and the reason and remedy in Turkish.
    Refused(Refusal),
    /// No such catalog or release, or the caller may not know of it: the same answer (PY-CLOUD-17).
    NotFound,
    /// The store failed or holds what it should not; the detail is for the log.
    Store(String),
}

impl From<StoreError> for CatalogError {
    fn from(e: StoreError) -> CatalogError {
        match e {
            StoreError::Changed => CatalogError::Refused(changed()),
            StoreError::Exists => CatalogError::Refused(taken()),
            StoreError::Failed(detail) => CatalogError::Store(detail),
        }
    }
}

pub(super) fn changed() -> Refusal {
    Refusal::new(
        "release_changed",
        "Sürüm siz işlem yaparken başka bir işlemle değişti; sürümü yeniden okuyup isteği yeniden gönderin.",
    )
}

pub(super) fn taken() -> Refusal {
    Refusal::new(
        "version_taken",
        "Bu sürüm numarası bu eklentide kullanılmış; bir sürüm numarası tek bir içerik taşır. Sürüm numarasını \
         artırıp yeniden yükleyin.",
    )
}

/// What a release list asks for. The caller's view comes first: the search and the count are of what it may see.
#[derive(Clone, Debug, Default)]
pub struct Filter {
    /// Part of the extension's id or of its publisher, in any case.
    pub text: Option<String>,
    pub state: Option<ReleaseState>,
    pub extension: Option<String>,
    pub offset: usize,
    pub limit: usize,
}

/// A page of the releases a caller may see, and how many there are in all.
pub struct Listing {
    pub total: usize,
    pub releases: Vec<Release>,
}
