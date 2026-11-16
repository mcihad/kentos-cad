//! The extension catalog's server side (plan/eklenti.md ADIM 22A; docs/adr/8059 §10): a package uploaded, its release
//! moved through the flow, read back and installed, with [`kentos_extensions::release`] alone deciding. This module
//! finds the release, makes the caller's [`Actor`] from the catalog's grants, reads the packages a move needs, keeps
//! what the move made and passes a refusal on in the flow's own words.
//!
//! - **Catalogs** ([`CatalogId`]): the public one, an organisation's and an account's own. An account outside a catalog,
//!   a release it may not see and one that does not exist get the same answer (PY-CLOUD-17).
//! - **Grants** ([`Catalog::actor`]): nobody holds a catalog role the catalog did not grant, and an organisation's own
//!   roles grant nothing here. In one's own catalog the owner is its publisher, reviewer and administrator. How a call
//!   came ([`Account::via`]) is the HTTP layer's word.
//! - **Store** ([`store::ReleaseStore`]): a small trait. A move is written only over the record it was made on, and made
//!   again on the newer record when another came first, so two scanners' reports or two reviewers never overwrite each
//!   other. The server's is persistent ([`postgres::PgStore`], §13), the tests' also in memory.
//! - **Decisions** need a fresh sign-in ([`Account::fresh_sign_in`]): the server's sessions carry none yet, so on a
//!   running server approving, sending back, lifting a block and reinstating are refused (`fresh_sign_in_required`)
//!   after the flow would have allowed them; everything else is open where the server opens the catalog (§13).
//! - **Publishing** ([`OnPublish`]): a release that becomes published is handed to the server, which prepares its
//!   audit's Python environment (docs/adr/8057 §66) without holding the request.
//!
//! The submodules are named by path: the job tests build this module into their own program (docs/adr/8059 §13).

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use kentos_extensions::Limits;
use kentos_extensions::release::{Actor, Refusal, Release, ReleaseFacts, Role, Timestamp};
use kentos_extensions::version::Version;

#[path = "extension_catalog/acting.rs"]
mod acting;
#[path = "extension_catalog/audits.rs"]
pub mod audits;
#[path = "extension_catalog/grants.rs"]
pub mod grants;
#[cfg(test)]
#[path = "extension_catalog/memory.rs"]
pub mod memory;
#[path = "extension_catalog/model.rs"]
mod model;
#[path = "extension_catalog/postgres.rs"]
pub mod postgres;
#[path = "extension_catalog/revocations.rs"]
mod revocations;
#[path = "extension_catalog/store.rs"]
pub mod store;

use model::taken;
pub use model::{Account, Accounts, CatalogError, CatalogId, Filter, Listing, Move, ReleaseKey};
use store::ReleaseStore;

/// The inspector's problems a refused upload lists; the rest are counted.
const SHOWN_PROBLEMS: usize = 10;

/// What the server does with a release that became published (docs/adr/8057 §66): it is given the release and its
/// approved package's bytes once the move is kept, and must not hold the request (a Python environment takes seconds
/// to prepare, minutes with an index).
pub trait OnPublish: Send + Sync {
    fn published(&self, catalog: &CatalogId, release: &Release, bytes: Bytes);
}

/// The extension catalog: its store, how it learns who calls, the limits it inspects packages with, what is done when
/// a release is published.
pub struct Catalog {
    store: Arc<dyn ReleaseStore>,
    accounts: Accounts,
    limits: Limits,
    on_publish: Option<Arc<dyn OnPublish>>,
}

impl Catalog {
    pub fn new(store: Arc<dyn ReleaseStore>, accounts: Accounts) -> Catalog {
        Catalog {
            store,
            accounts,
            limits: Limits::default(),
            on_publish: None,
        }
    }

    /// This catalog, handing every release that becomes published to `hook`.
    pub fn on_publish(mut self, hook: Arc<dyn OnPublish>) -> Catalog {
        self.on_publish = Some(hook);
        self
    }

    pub fn accounts(&self) -> &Accounts {
        &self.accounts
    }

    /// The flow's actor for a call into a catalog: the roles the catalog granted the account, the publishers it is a
    /// member of, how the call came. An account outside the catalog is told it does not exist. In one's own catalog the
    /// owner holds the publisher's, the reviewer's and the administrator's roles; nobody else is in it but the
    /// catalog's own services.
    pub async fn actor(
        &self,
        catalog: &CatalogId,
        account: &Account,
    ) -> Result<Actor, CatalogError> {
        let mut access = self.store.access(catalog, &account.id).await?;
        if matches!(catalog, CatalogId::Personal(owner) if *owner == account.id) {
            access.member = true;
            access
                .roles
                .extend([Role::Publisher, Role::Reviewer, Role::CatalogAdmin]);
        }
        if !access.member {
            return Err(CatalogError::NotFound);
        }
        let roles: Vec<Role> = access.roles.into_iter().collect();
        let mut actor = Actor::new(&account.id, &roles, account.via);
        actor.publishers = access.publishers;
        Ok(actor)
    }

    /// The releases a caller may see, newest version first under each extension; filtered and counted after that.
    pub async fn list(
        &self,
        catalog: &CatalogId,
        account: &Account,
        filter: &Filter,
    ) -> Result<Listing, CatalogError> {
        let actor = self.actor(catalog, account).await?;
        let text = filter
            .text
            .as_deref()
            .map(str::to_lowercase)
            .filter(|t| !t.is_empty());
        let mut seen: Vec<Release> = self
            .store
            .releases(catalog)
            .await?
            .into_iter()
            // What the caller may see first: the search and the count never reach the rest (PY-CLOUD-17).
            .filter(|r| r.visible_to(&actor))
            .filter(|r| filter.state.is_none_or(|s| r.state() == s))
            .filter(|r| {
                filter
                    .extension
                    .as_deref()
                    .is_none_or(|e| r.extension_id() == e)
            })
            .filter(|r| {
                text.as_deref().is_none_or(|t| {
                    r.extension_id().to_lowercase().contains(t)
                        || r.publisher().to_lowercase().contains(t)
                })
            })
            .collect();
        seen.sort_by(|a, b| {
            a.extension_id()
                .cmp(b.extension_id())
                .then_with(|| Version::parse(b.version()).cmp(&Version::parse(a.version())))
        });
        let total = seen.len();
        let releases = seen
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit)
            .collect();
        Ok(Listing { total, releases })
    }

    /// One release, when the caller may see it.
    pub async fn release(
        &self,
        key: &ReleaseKey,
        account: &Account,
    ) -> Result<(Release, Actor), CatalogError> {
        let actor = self.actor(&key.catalog, account).await?;
        let release = self.visible(key, &actor).await?;
        Ok((release, actor))
    }

    /// Uploads a package as a draft. In the public and an organisation's catalog it goes under the publisher its
    /// manifest names, which the uploader must be a member of; in one's own catalog under that or the owner. An
    /// extension's releases in a catalog are one publisher's: another publisher's id is refused before its version
    /// numbers could be taken.
    pub async fn upload(
        &self,
        catalog: &CatalogId,
        account: &Account,
        bytes: Bytes,
    ) -> Result<(Release, Actor), CatalogError> {
        let mut actor = self.actor(catalog, account).await?;
        let facts = self.facts(&bytes)?;
        let publisher = match (catalog, &facts.manifest.publisher_id) {
            (_, Some(claimed)) => claimed.clone(),
            (CatalogId::Personal(owner), None) => format!("@{owner}"),
            (_, None) => {
                return Err(CatalogError::Refused(Refusal::new(
                    "publisher_missing",
                    "Manifestte publisherId yok: bu katalogda bir sürüm bir yayımcının adına yüklenir. Manifeste \
                     yayımcınızın kimliğini yazıp yeniden paketleyin.",
                )));
            }
        };
        if let CatalogId::Personal(_) = catalog {
            // One's own catalog: nobody else sees it, so its owner publishes there whatever the package claims.
            actor.publishers.insert(publisher.clone());
        }
        let id = facts.manifest.id.clone();
        let existing: Vec<Release> = self
            .store
            .releases(catalog)
            .await?
            .into_iter()
            .filter(|r| r.extension_id() == id)
            .collect();
        if existing.iter().any(|r| r.publisher() != publisher) {
            return Err(CatalogError::Refused(Refusal::new(
                "extension_owned",
                format!(
                    "“{id}” bu katalogda başka bir yayımcının eklentisi; kendi yayımcınızın kimlik önekiyle yükleyin."
                ),
            )));
        }
        let release = Release::upload(
            &actor,
            catalog.scope(),
            &publisher,
            &facts,
            &existing,
            now(),
        )
        .map_err(|refusal| unseen_version(refusal, &existing, &actor, &facts.manifest.version))?;
        self.store
            .put_package(&release.content().package, bytes)
            .await?;
        let key = ReleaseKey {
            catalog: catalog.clone(),
            extension: id,
            version: release.version().to_owned(),
        };
        self.store.write(&key, None, &release).await?;
        Ok((release, actor))
    }

    /// New content for a draft or a release sent back. The move is tried before the bytes are kept, so a refused one
    /// leaves nothing behind.
    pub async fn replace(
        &self,
        key: &ReleaseKey,
        account: &Account,
        bytes: Bytes,
    ) -> Result<(Release, Actor), CatalogError> {
        let actor = self.actor(&key.catalog, account).await?;
        let facts = self.facts(&bytes)?;
        let now = now();
        let mut trial = self.visible(key, &actor).await?;
        trial
            .replace(&actor, &facts, now)
            .map_err(CatalogError::Refused)?;
        self.store
            .put_package(&facts.content().package, bytes)
            .await?;
        let release = self
            .moved(key, &actor, |r| r.replace(&actor, &facts, now))
            .await?;
        Ok((release, actor))
    }

    /// The release a caller may install: published, its approval of its content (PY-CLOUD-10, 15).
    pub async fn installable(
        &self,
        key: &ReleaseKey,
        account: &Account,
    ) -> Result<Release, CatalogError> {
        let actor = self.actor(&key.catalog, account).await?;
        let release = self.visible(key, &actor).await?;
        release
            .allows_install(release.content())
            .map_err(CatalogError::Refused)?;
        Ok(release)
    }

    /// The approved package's bytes, checked before they leave: what the store holds must be the content a reviewer
    /// approved (PY-CLOUD-10).
    pub async fn package(
        &self,
        key: &ReleaseKey,
        account: &Account,
    ) -> Result<(Release, Bytes), CatalogError> {
        let release = self.installable(key, account).await?;
        let digest = &release.content().package;
        let bytes = self.stored(digest).await?;
        let facts = self
            .facts(&bytes)
            .map_err(|_| CatalogError::Store(format!("{digest} paketi depoda bozuk")))?;
        release.allows_install(&facts.content()).map_err(|r| {
            CatalogError::Store(format!(
                "{digest} paketinin depodaki baytları onaylı içerik değil: {r}"
            ))
        })?;
        Ok((release, bytes))
    }

    /// A release the actor may see; any other answers as one that does not exist.
    async fn visible(&self, key: &ReleaseKey, actor: &Actor) -> Result<Release, CatalogError> {
        match self.store.release(key).await? {
            Some(release) if release.visible_to(actor) => Ok(release),
            _ => Err(CatalogError::NotFound),
        }
    }

    async fn stored(&self, digest: &str) -> Result<Bytes, CatalogError> {
        self.store
            .package(digest)
            .await?
            .ok_or_else(|| CatalogError::Store(format!("{digest} paketi depoda yok")))
    }

    async fn stored_facts(&self, digest: &str) -> Result<ReleaseFacts, CatalogError> {
        let bytes = self.stored(digest).await?;
        match self.facts(&bytes) {
            Ok(facts) if facts.content().package == digest => Ok(facts),
            _ => Err(CatalogError::Store(format!("{digest} paketi depoda bozuk"))),
        }
    }

    /// A package's facts, or why it is not one: every problem the inspector found, with its file and place, so its
    /// author or agent mends them in one go.
    fn facts(&self, bytes: &[u8]) -> Result<ReleaseFacts, CatalogError> {
        let opened = kentos_extensions::open(bytes, &self.limits);
        let problems = &opened.inspection.problems;
        if !problems.is_empty() {
            let shown: Vec<String> = problems
                .iter()
                .take(SHOWN_PROBLEMS)
                .map(ToString::to_string)
                .collect();
            let more = match problems.len().saturating_sub(SHOWN_PROBLEMS) {
                0 => String::new(),
                n => format!(" (ve {n} sorun daha)"),
            };
            return Err(CatalogError::Refused(Refusal::new(
                "package_invalid",
                format!("Paket denetimden geçmedi: {}{more}", shown.join("; ")),
            )));
        }
        ReleaseFacts::of(&opened).map_err(CatalogError::Refused)
    }
}

/// A taken version number, said without the state of a release the caller may not see (PY-CLOUD-17).
fn unseen_version(
    refusal: Refusal,
    existing: &[Release],
    actor: &Actor,
    version: &str,
) -> CatalogError {
    if refusal.code == "version_taken" {
        let wanted = Version::parse(version);
        let seen = existing
            .iter()
            .any(|r| Version::parse(r.version()) == wanted && r.visible_to(actor));
        if !seen {
            return CatalogError::Refused(taken());
        }
    }
    CatalogError::Refused(refusal)
}

/// Now, as the flow takes it: milliseconds since 1970-01-01 UTC.
fn now() -> Timestamp {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    Timestamp(u64::try_from(ms).unwrap_or(u64::MAX))
}
