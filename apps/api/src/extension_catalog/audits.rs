//! The catalog's side of `project.audit` jobs (docs/adr/8057 §42; docs/adr/8059 §10): the release a cloud job runs and
//! its approved package's bytes ([`Catalog::runnable`]), and the worker's audit source over it ([`CatalogAudits`]): the
//! organisation's catalog first, then the public one, and the server's prepared environments.
//!
//! A catalog knows an extension when anyone may see one of its releases there (published, suspended, withdrawn, or
//! retired after it was published). The organisation's catalog that knows it decides alone: the same id in the public
//! catalog may be another publisher's, and a release the organisation suspended must not be stood in for by it. A
//! draft there claims nothing, and its being there is not said.

use std::sync::Arc;

use axum::body::Bytes;
use kentos_application::audit_catalog::{AuditFuture, AuditPackage, AuditPythons, AuditSource};
use kentos_extensions::release::{Actor, Channel, Refusal, Release, ReleaseState};
use kentos_extensions::version::Version;

use super::{Catalog, CatalogError, CatalogId};

impl Catalog {
    /// The release of `extension` a cloud job runs from `catalog`: `version` when given and it may run (published, or
    /// retired after it was), else the newest published version; with its package's bytes, which must be the approved
    /// content, as an install's are. `NotFound` when the catalog does not know the extension (no release of it anyone
    /// may see); a refusal when it does but not that version, or none that may run.
    pub async fn runnable(
        &self,
        catalog: &CatalogId,
        extension: &str,
        version: Option<&str>,
    ) -> Result<(Release, Bytes), CatalogError> {
        // Nobody: no role, no publisher; what this account sees, everyone does.
        let nobody = Actor::new("", &[], Channel::Program);
        let releases: Vec<Release> = self
            .store
            .releases(catalog)
            .await?
            .into_iter()
            .filter(|r| r.extension_id() == extension && r.visible_to(&nobody))
            .collect();
        if releases.is_empty() {
            return Err(CatalogError::NotFound);
        }
        let chosen = match version {
            Some(v) => releases.into_iter().find(|r| r.version() == v),
            None => releases
                .into_iter()
                .filter(|r| r.state() == ReleaseState::Published)
                .max_by(|a, b| Version::parse(a.version()).cmp(&Version::parse(b.version()))),
        }
        .ok_or_else(|| {
            CatalogError::Refused(Refusal::new(
                "not_runnable",
                match version {
                    Some(v) => format!("“{extension}” {v} bu katalogda yayımlanmış bir sürüm değil."),
                    None => format!(
                        "“{extension}” eklentisinin bu katalogda çalıştırılabilir (yayımlanmış) bir sürümü yok."
                    ),
                },
            ))
        })?;
        chosen.allows_run().map_err(CatalogError::Refused)?;
        let digest = chosen.content().package.clone();
        let bytes = self.stored(&digest).await?;
        let approved = chosen.approval().map(|a| &a.content);
        match self.facts(&bytes) {
            Ok(facts)
                if Some(&facts.content()) == approved && facts.content() == *chosen.content() =>
            {
                Ok((chosen, bytes))
            }
            _ => Err(CatalogError::Store(format!(
                "{digest} paketinin depodaki baytları onaylı içerik değil"
            ))),
        }
    }
}

/// The worker's audit source over the catalog and the server's prepared environments.
pub struct CatalogAudits {
    catalog: Arc<Catalog>,
    pythons: AuditPythons,
}

impl CatalogAudits {
    pub fn new(catalog: Arc<Catalog>, pythons: AuditPythons) -> CatalogAudits {
        CatalogAudits { catalog, pythons }
    }
}

impl AuditSource for CatalogAudits {
    fn release<'a>(
        &'a self,
        tenant: &'a str,
        extension: &'a str,
        version: Option<&'a str>,
    ) -> AuditFuture<'a, Result<AuditPackage, String>> {
        Box::pin(async move {
            // The organisation's own catalog first: an id it knows is its, whatever the public one holds.
            for catalog in [CatalogId::Tenant(tenant.to_owned()), CatalogId::Public] {
                match self.catalog.runnable(&catalog, extension, version).await {
                    Ok((release, bytes)) => {
                        return Ok(AuditPackage {
                            catalog: catalog.to_string(),
                            version: release.version().to_owned(),
                            digest: release.content().package.clone(),
                            bytes: bytes.to_vec(),
                        });
                    }
                    Err(CatalogError::NotFound) => continue,
                    Err(CatalogError::Refused(refusal)) if catalog != CatalogId::Public => {
                        return Err(format!(
                            "{} Eklenti kurumun kataloğunda olduğu için genel katalogdaki aynı kimliğe bakılmadı.",
                            refusal.message
                        ));
                    }
                    Err(CatalogError::Refused(refusal)) => return Err(refusal.message),
                    Err(CatalogError::Store(detail)) => {
                        return Err(format!("Eklenti kataloğu okunamadı: {detail}"));
                    }
                }
            }
            Err(match version {
                Some(v) => format!(
                    "“{extension}” {v} kurumun kataloğunda da genel katalogda da yok; eklentinin kimliğini ve sürümünü denetleyin."
                ),
                None => format!(
                    "“{extension}” eklentisinin yayımlanmış bir sürümü kurumun kataloğunda da genel katalogda da yok."
                ),
            })
        })
    }

    fn python(&self, extension: &str, version: &str) -> Option<String> {
        self.pythons.get(extension, version)
    }
}
