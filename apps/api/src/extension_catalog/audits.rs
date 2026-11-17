//! The catalog's side of `project.audit` jobs (docs/adr/8057 §42; docs/adr/8059 §10): the release a cloud job runs and
//! its approved package's bytes ([`Catalog::runnable`]), and the worker's audit source over it ([`CatalogAudits`]): the
//! organisation's catalog first, then the public one, and the server's prepared environments ([`PythonSource`]: the
//! setting's map, or the map the server keeps, read again at every job: docs/adr/8057 §66).
//!
//! A catalog knows an extension when anyone may see one of its releases there (published, suspended, withdrawn, or
//! retired after it was published). The organisation's catalog that knows it decides alone: the same id in the public
//! catalog may be another publisher's, and a release the organisation suspended must not be stood in for by it. A
//! draft there claims nothing, and its being there is not said.
//!
//! The server's publishing hook ([`PrepareEnvironments`]) prepares a published release's environment, so the jobs find
//! its interpreter in the map without the administrator's command.

use std::sync::Arc;

use axum::body::Bytes;
use kentos_application::audit_catalog::{AuditFuture, AuditPackage, AuditPythons, AuditSource};
use kentos_extensions::Limits;
use kentos_extensions::release::{Actor, Channel, Refusal, Release, ReleaseState};
use kentos_extensions::version::Version;

use super::{Catalog, CatalogError, CatalogId, OnPublish};
use crate::audit_env::{EnvSettings, prepare_release};

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

/// Where the audit source finds the interpreter prepared for a version.
pub trait PythonSource: Send + Sync {
    fn python(&self, extension: &str, version: &str) -> Option<String>;
}

impl PythonSource for AuditPythons {
    fn python(&self, extension: &str, version: &str) -> Option<String> {
        self.get(extension, version)
    }
}

/// The map the server keeps over the setting's: read again at every job, so a version prepared while the server runs
/// is found.
impl PythonSource for crate::audit_env::AuditMap {
    fn python(&self, extension: &str, version: &str) -> Option<String> {
        self.get(extension, version)
    }
}

/// The worker's audit source over the catalog and the server's prepared environments.
pub struct CatalogAudits {
    catalog: Arc<Catalog>,
    pythons: Box<dyn PythonSource>,
}

impl CatalogAudits {
    pub fn new(catalog: Arc<Catalog>, pythons: impl PythonSource + 'static) -> CatalogAudits {
        CatalogAudits {
            catalog,
            pythons: Box::new(pythons),
        }
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
        self.pythons.python(extension, version)
    }
}

/// The server's publishing hook (docs/adr/8057 §66): a release published with an audit and a Python project gets its
/// environment, prepared on a blocking thread of its own (the request has its answer by then) as the administrator's
/// command prepares it; the outcome goes to the log. A release with nothing to run in the cloud has nothing to prepare;
/// one the cloud box refuses (it asks for the network) is logged with why.
pub struct PrepareEnvironments {
    settings: EnvSettings,
}

impl PrepareEnvironments {
    pub fn new(settings: EnvSettings) -> PrepareEnvironments {
        PrepareEnvironments { settings }
    }
}

/// Whether a package has an audit the cloud runs: an audit entry point and its Python project.
fn cloud_audit(bytes: &[u8]) -> bool {
    let opened = kentos_extensions::open(bytes, &Limits::default());
    opened
        .inspection
        .manifest
        .as_ref()
        .is_some_and(|m| !m.entrypoints.audits.is_empty() && m.python.is_some())
}

impl OnPublish for PrepareEnvironments {
    fn published(&self, catalog: &CatalogId, release: &Release, bytes: Bytes) {
        let settings = self.settings.clone();
        let catalog = catalog.to_string();
        let extension = release.extension_id().to_owned();
        let version = release.version().to_owned();
        let digest = release.content().package.clone();
        tokio::task::spawn_blocking(move || {
            if !cloud_audit(&bytes) {
                return;
            }
            match prepare_release(&settings, &extension, &version, Some(&digest), &bytes) {
                Ok(done) => tracing::info!(
                    katalog = %catalog,
                    eklenti = %extension,
                    surum = %version,
                    sure_sn = done.took.as_secs_f64(),
                    yeni = done.made,
                    "yayımlanan sürümün denetim ortamı hazır"
                ),
                Err(why) => tracing::warn!(
                    katalog = %catalog,
                    eklenti = %extension,
                    surum = %version,
                    neden = %why,
                    "yayımlanan sürümün denetim ortamı hazırlanamadı; bulut denetim işi bu sürümü çalıştıramaz"
                ),
            }
        });
    }
}
