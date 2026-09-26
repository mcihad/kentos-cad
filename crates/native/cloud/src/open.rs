//! Opening a cloud project into a drawing (docs/adr/0040), as the web opens
//! one (apps/web/src/app/cloud/session.ts `open`): its metadata first, then
//!
//! - **a database project:** its objects page by page (the web's 2000), each
//!   under the server's id as its persistent id (docs/adr/0026), with the
//!   server's version of each kept as the base of the changes sent later
//!   (sync.rs). Pages are separate reads; like the web, the events after the
//!   cursor read with the metadata bring the rest in.
//! - **a file project:** its newest revision, checked against the SHA-256 the
//!   list gives and decoded with the shared codec (docs/adr/0031); one with
//!   nothing saved yet opens empty, with its metadata.
//!
//! The drawing is built and checked before it is handed over, off the async
//! threads: a project the desktop cannot hold never replaces the open
//! drawing (CLAUDE.md §4.8, §21.2).

use std::future::Future;

use kentos_contracts::{
    DOCUMENT_FORMAT, DOCUMENT_VERSION_2, DocumentSnapshotV2, EntityId, FeatureRecord, ProjectId,
    ProjectInfo, ProjectPermission, ProjectState, ProjectStorage,
};
use kentos_domain::Document;
use uuid::Uuid;

use crate::api::{Cloud, Progress};
use crate::failure::ApiFailure;
use crate::runtime::run;

/// Objects asked for at once (the web's page).
pub const PAGE: u32 = 2000;

/// A committed revision of a file project.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Revision {
    pub number: u64,
    pub sha256: String,
}

/// Where an opened drawing's content lives on the server.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// Object by object: the server's version of each when it was read, the
    /// base of the changes sent from here (sync.rs).
    Database { versions: Vec<(Uuid, String)> },
    /// A file project at this revision; `None` before its first save. A save
    /// is based on it (`expectedVersions["@file"]`).
    File { revision: Option<Revision> },
}

/// A cloud project read into a drawing.
#[derive(Debug)]
pub struct Opened {
    pub tenant: Uuid,
    pub project: Uuid,
    pub info: ProjectInfo,
    /// Clean and without history, every object under its persistent id.
    pub document: Document,
    pub source: Source,
}

impl Opened {
    /// Archived: it opens read-only until it is unarchived (docs/adr/0028).
    pub fn archived(&self) -> bool {
        self.info.state == ProjectState::Archived
    }

    fn may(&self, p: ProjectPermission) -> bool {
        !self.archived() && self.info.access.permissions.contains(&p)
    }

    /// Whether this account may change its objects (a file project: save revisions).
    pub fn can_write(&self) -> bool {
        self.may(ProjectPermission::FeatureWrite)
    }

    /// Whether it may change the project's name, settings, layer tree and styles.
    pub fn can_edit_meta(&self) -> bool {
        self.may(ProjectPermission::Edit)
    }
}

/// Objects under their persistent ids, numbered 1, 2 … in the order they came.
type Objects = Vec<(EntityId, kentos_contracts::Entity)>;
/// Each object's version on the server.
type Versions = Vec<(Uuid, String)>;

/// A drawing of the project's metadata and these objects.
fn snapshot(info: &ProjectInfo, project: Uuid, objects: Objects) -> DocumentSnapshotV2 {
    let (uids, entities) = objects.into_iter().unzip();
    DocumentSnapshotV2 {
        format: DOCUMENT_FORMAT.to_owned(),
        version: DOCUMENT_VERSION_2,
        name: info.name.clone(),
        settings: info.settings.clone(),
        origin: info.origin,
        home_view: info.home_view,
        layers: info.layers.clone(),
        active_layer: info.active_layer.clone(),
        entities,
        uids,
        styles: info.styles.clone(),
        project_id: Some(ProjectId(project.into_bytes())),
        migrated_from: None,
    }
}

/// The server's objects as a drawing's: numbered in the order they came,
/// each under the server's id; the version of each for the tracker.
fn objects_of(
    info: &ProjectInfo,
    records: Vec<FeatureRecord>,
) -> Result<(Objects, Versions), ApiFailure> {
    let mut objects = Vec::with_capacity(records.len());
    let mut versions = Vec::with_capacity(records.len());
    for (i, record) in records.into_iter().enumerate() {
        let id = EntityId::parse(&record.id).ok_or_else(|| {
            ApiFailure::drawing(
                &info.name,
                format!("nesne kimliği “{}” bir UUID değil", record.id),
            )
        })?;
        let slot = u32::try_from(i + 1)
            .map_err(|_| ApiFailure::drawing(&info.name, "nesne sayısı sınırı aşıyor"))?;
        let mut entity = record.entity;
        entity.base_mut().id = slot;
        versions.push((Uuid::from_bytes(id.0), record.version));
        objects.push((id, entity));
    }
    Ok((objects, versions))
}

/// Builds the document off the async threads (a large drawing takes a while).
async fn build(name: String, doc: DocumentSnapshotV2) -> Result<Document, ApiFailure> {
    tokio::task::spawn_blocking(move || {
        Document::from_snapshot_v2(doc).map_err(|e| ApiFailure::drawing(&name, e))
    })
    .await
    .map_err(|e| ApiFailure::local(format!("Çizim kurulamadı: {e}")))?
}

async fn database(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    info: ProjectInfo,
    progress: Option<Progress>,
) -> Result<Opened, ApiFailure> {
    let total: u64 = info.feature_count.parse().unwrap_or(0);
    let mut records = Vec::with_capacity(usize::try_from(total).unwrap_or(0).min(1 << 20));
    let mut after: Option<String> = None;
    if let Some(p) = &progress {
        p(0, total);
    }
    loop {
        let page = cloud
            .features(tenant, project, after.as_deref(), PAGE)
            .await?;
        let more = page.next.is_some();
        if more && page.features.is_empty() {
            // A server that sends empty pages with a next one would never end.
            return Err(ApiFailure::unreadable(
                200,
                "nesne sayfası",
                "boş sayfanın ardından yeni sayfa bildirildi",
            ));
        }
        records.extend(page.features);
        if let Some(p) = &progress {
            let done = records.len() as u64;
            p(done, total.max(done));
        }
        if !more {
            break;
        }
        after = page.next;
    }
    let (objects, versions) = objects_of(&info, records)?;
    let document = build(info.name.clone(), snapshot(&info, project, objects)).await?;
    Ok(Opened {
        tenant,
        project,
        info,
        document,
        source: Source::Database { versions },
    })
}

async fn file(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    info: ProjectInfo,
    progress: Option<Progress>,
) -> Result<Opened, ApiFailure> {
    let list = cloud.file_revisions(tenant, project).await?;
    let Some(current) = list.current else {
        // Nothing saved yet: the project's metadata, no objects.
        let document = build(info.name.clone(), snapshot(&info, project, Vec::new())).await?;
        return Ok(Opened {
            tenant,
            project,
            info,
            document,
            source: Source::File { revision: None },
        });
    };
    let number: u64 = current.parse().map_err(|_| {
        ApiFailure::unreadable(
            200,
            "revizyon listesi",
            format!("“{current}” bir revizyon numarası değil"),
        )
    })?;
    let listed = list
        .revisions
        .iter()
        .find(|r| r.revision == current)
        .map(|r| r.sha256.clone())
        .ok_or_else(|| {
            ApiFailure::unreadable(
                200,
                "revizyon listesi",
                format!("en yeni revizyon ({current}) listede yok"),
            )
        })?;
    let got = cloud
        .download_revision(tenant, project, number, progress)
        .await?;
    if got.sha256 != listed {
        return Err(ApiFailure::local(format!(
            "“{}” projesinin {number}. revizyonu listedekinden başka geldi (SHA-256 {}, listede {listed}); yeniden açın.",
            info.name, got.sha256
        ))
        .with_code("corrupt"));
    }
    let name = info.name.clone();
    let bytes = got.bytes;
    let document = tokio::task::spawn_blocking(move || {
        let doc = kentos_kcad::decode(&bytes).map_err(|e| ApiFailure::drawing(&name, e))?;
        Document::from_snapshot_v2(doc).map_err(|e| ApiFailure::drawing(&name, e))
    })
    .await
    .map_err(|e| ApiFailure::local(format!("Çizim kurulamadı: {e}")))??;
    Ok(Opened {
        tenant,
        project,
        info,
        document,
        source: Source::File {
            revision: Some(Revision {
                number,
                sha256: listed,
            }),
        },
    })
}

/// Opens a cloud project (`progress`: objects or bytes so far, and all of
/// them). Dropping the future stops it; nothing is changed by it.
pub fn open(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    progress: Option<Progress>,
) -> impl Future<Output = Result<Opened, ApiFailure>> + Send + 'static {
    let cloud = cloud.clone();
    run(async move {
        let info = cloud.project(tenant, project).await?;
        match info.storage {
            ProjectStorage::Database => database(&cloud, tenant, project, info, progress).await,
            ProjectStorage::File => file(&cloud, tenant, project, info, progress).await,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::{
        AccessSource, DocumentSnapshotV1, ProjectAccessView, ProjectRole, TenantKind,
    };

    const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

    fn info() -> ProjectInfo {
        let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
        ProjectInfo {
            id: "0199aaaa-0000-7000-8000-000000000001".into(),
            tenant_id: "0199aaaa-0000-7000-8000-000000000002".into(),
            tenant_name: "Harita Bürosu".into(),
            tenant_kind: TenantKind::Organization,
            access: ProjectAccessView {
                role: ProjectRole::Editor,
                via: AccessSource::Grant,
                permissions: vec![ProjectPermission::Read, ProjectPermission::FeatureWrite],
            },
            state: ProjectState::Active,
            name: "Ada 101".into(),
            settings: s.settings,
            origin: s.origin,
            home_view: s.home_view,
            layers: s.layers,
            active_layer: s.active_layer,
            styles: s.styles,
            meta_version: "3".into(),
            data_revision: "7".into(),
            feature_count: "2".into(),
            event_cursor: "11".into(),
            storage: ProjectStorage::Database,
        }
    }

    #[test]
    fn server_objects_become_numbered_objects_under_their_ids() {
        let info = info();
        let sample = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
        let records: Vec<FeatureRecord> = sample.entities[..2]
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let mut e = e.clone();
                // On the wire the slot means nothing.
                e.base_mut().id = 900 + i as u32;
                FeatureRecord {
                    id: format!("0199aaaa-0000-7000-8000-00000000010{i}"),
                    version: format!("{}", 5 + i),
                    entity: e,
                }
            })
            .collect();
        let project = Uuid::parse_str(&info.id).unwrap();
        let (objects, versions) = objects_of(&info, records).unwrap();
        let doc = Document::from_snapshot_v2(snapshot(&info, project, objects)).unwrap();
        assert_eq!(doc.len(), 2);
        let uid = Uuid::parse_str("0199aaaa-0000-7000-8000-000000000101").unwrap();
        let slot = doc.slot_of(uid).unwrap();
        assert_eq!(slot.0, 2);
        assert_eq!(doc.get(slot).unwrap().base().id, 2);
        assert_eq!(versions[1], (uid, "6".to_string()));
        assert_eq!(doc.project_id(), Some(project));
        assert!(!doc.is_dirty());
    }

    #[test]
    fn a_bad_id_from_the_server_opens_nothing() {
        let info = info();
        let sample = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
        let records = vec![FeatureRecord {
            id: "BÜYÜK".into(),
            version: "1".into(),
            entity: sample.entities[0].clone(),
        }];
        let e = objects_of(&info, records).unwrap_err();
        assert_eq!(e.code, "drawing");
        assert!(e.message.contains("Ada 101"), "{}", e.message);
    }

    #[test]
    fn what_an_account_may_do_follows_its_permissions_and_the_archive() {
        let info = info();
        let project = Uuid::parse_str(&info.id).unwrap();
        let opened = |info: ProjectInfo| Opened {
            tenant: Uuid::nil(),
            project,
            document: Document::from_snapshot_v2(snapshot(&info, project, Vec::new())).unwrap(),
            info,
            source: Source::File { revision: None },
        };
        let editor = opened(info.clone());
        assert!(editor.can_write() && !editor.can_edit_meta());
        let mut archived = info;
        archived.state = ProjectState::Archived;
        assert!(!opened(archived).can_write());
    }
}
