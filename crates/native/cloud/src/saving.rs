//! Saving a drawing to the cloud as a `.kcad` file (docs/adr/0031, 0036, 0040):
//!
//! - **a file project's new revision:** the file's bytes go to an upload
//!   (its size and SHA-256 declared first, checked by the server), then
//!   `project.file.commit` makes it the next revision, based on the one the
//!   drawing came from (`expectedVersions["@file"]`). Someone else's save in
//!   between is a conflict: nothing is written and both revisions stay.
//! - **a new cloud project from a drawing:** the project is created with the
//!   drawing's metadata, and the file becomes its first revision (a file
//!   project) or its objects (a database project, `project.import`). When the
//!   server refuses the drawing after the project was created, the empty
//!   project goes to the trash, so a failed upload leaves nothing behind.
//!
//! A step whose answer does not come is sent again: the same upload, the
//! same idempotency key, so the server does it once. An upload whose bytes
//! may have arrived is asked about before they are sent again, since the
//! server refuses them a second time.

use std::collections::BTreeMap;
use std::future::Future;

use bytes::Bytes;
use kentos_contracts::{
    CommandEnvelope, EmptyInput, FILE_UPLOAD_MAX, FileCommitted, FileUpload, FileUploadBegin,
    PROJECT_FILE_COMMIT, PROJECT_FILE_COMMIT_VERSION, PROJECT_IMPORT, PROJECT_IMPORT_VERSION,
    PROJECT_TRASH, PROJECT_TRASH_VERSION, ProjectCatalogChange, ProjectCreate, ProjectImported,
    ProjectInfo, ProjectStorage,
};
use kentos_domain::Document;
use serde_json::json;
use uuid::Uuid;

use crate::api::{Cloud, Progress, hex_sha256};
use crate::failure::ApiFailure;
use crate::runtime::run;

/// Tries of one step whose answer does not come, before the failure is given back.
pub const TRIES: u32 = 5;

/// A file larger than this goes in parts of this size (docs/adr/0045): a
/// connection cut short costs only the part on its way.
pub const PART: usize = 8 * 1024 * 1024;

/// The expected-version key of a file project's revision (the server's `PROJECT_FILE_KEY`).
pub const FILE_KEY: &str = "@file";

/// Sends `call` again while it fails for a passing reason, waiting a little longer each time.
pub(crate) async fn retrying<T, F, Fut>(mut call: F) -> Result<T, ApiFailure>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ApiFailure>>,
{
    let mut tries = 1;
    loop {
        match call().await {
            Err(f) if f.transient() && tries < TRIES => {
                tokio::time::sleep(f.backoff(tries)).await;
                tries += 1;
            }
            done => return done,
        }
    }
}

/// A command envelope of the desktop: a new request id; `key` is the
/// idempotency key every try of it shares.
pub fn envelope(
    tenant: Uuid,
    project: Uuid,
    command: &str,
    version: u32,
    key: Uuid,
    expected: BTreeMap<String, String>,
    input: serde_json::Value,
) -> CommandEnvelope {
    CommandEnvelope {
        command_name: command.into(),
        version,
        tenant_id: tenant.to_string(),
        project_id: project.to_string(),
        request_id: format!("desktop-{}", Uuid::new_v4()),
        idempotency_key: key.to_string(),
        expected_versions: expected,
        input,
    }
}

/// A file's size as an upload declares it, when the server takes files that large.
fn checked_size(len: usize) -> Result<u32, ApiFailure> {
    u32::try_from(len)
        .ok()
        .filter(|&n| n > 0 && n <= FILE_UPLOAD_MAX)
        .ok_or_else(|| {
            ApiFailure::local(format!(
                "Dosya {len} bayt; buluta 1 bayt ile {} MiB arası dosyalar kaydedilir. Daha büyük projeler için parçalı yükleme henüz yok.",
                FILE_UPLOAD_MAX / (1024 * 1024)
            ))
        })
}

/// How many bytes of an upload arrived, as the server says.
fn arrived(u: &FileUpload) -> u64 {
    u.received_bytes
        .as_deref()
        .and_then(|b| b.parse().ok())
        .unwrap_or(0)
}

/// The file's bytes in parts of `part`, each going on from what the server
/// has: after a failure the upload is asked how far it came, so a part whose
/// answer was lost is not sent twice, and the rest goes on from there.
/// `progress`: the bytes the server has, of all of them.
async fn send_in_parts(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    id: Uuid,
    bytes: &Bytes,
    part: usize,
    progress: Option<&Progress>,
) -> Result<FileUpload, ApiFailure> {
    let total = bytes.len() as u64;
    let mut offset = 0u64;
    let mut tries = 1;
    loop {
        let start = usize::try_from(offset)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        let end = start.saturating_add(part).min(bytes.len());
        match cloud
            .send_part(tenant, project, id, offset, bytes.slice(start..end))
            .await
        {
            Ok(u) if u.received => return Ok(u),
            Ok(u) => {
                offset = arrived(&u);
                tries = 1;
            }
            Err(f) if f.path.as_deref() == Some("offset") || (f.transient() && tries < TRIES) => {
                if f.transient() {
                    tokio::time::sleep(f.backoff(tries)).await;
                    tries += 1;
                }
                // Where it stands now: a part may have arrived with its answer lost.
                match cloud.upload_status(tenant, project, id).await {
                    Ok(u) if u.received => return Ok(u),
                    Ok(u) => offset = arrived(&u),
                    Err(e) if e.transient() => {}
                    Err(e) => return Err(e),
                }
            }
            Err(f) => return Err(f),
        }
        if let Some(p) = progress {
            p(offset, total);
        }
    }
}

/// The file's bytes in an upload of the project, received and verified by
/// the server. Only its own caller can commit or import it.
async fn upload(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    bytes: Bytes,
) -> Result<FileUpload, ApiFailure> {
    upload_parted(cloud, tenant, project, bytes, PART, None).await
}

/// `upload`, a file larger than `part` in parts of that size.
async fn upload_parted(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    bytes: Bytes,
    part: usize,
    progress: Option<&Progress>,
) -> Result<FileUpload, ApiFailure> {
    let size = checked_size(bytes.len())?;
    let sha256 = hex_sha256(&bytes);
    let begun = retrying(|| {
        cloud.begin_upload(
            tenant,
            project,
            FileUploadBegin {
                size,
                sha256: sha256.clone(),
            },
        )
    })
    .await?;
    let id = Uuid::parse_str(&begun.id).map_err(|_| {
        ApiFailure::unreadable(
            201,
            "yükleme",
            format!("“{}” bir yükleme kimliği değil", begun.id),
        )
    })?;
    if bytes.len() > part {
        return send_in_parts(cloud, tenant, project, id, &bytes, part, progress).await;
    }
    let mut tries = 1;
    loop {
        match cloud.send_upload(tenant, project, id, bytes.clone()).await {
            Ok(done) => return Ok(done),
            Err(f) if f.transient() && tries < TRIES => {
                tokio::time::sleep(f.backoff(tries)).await;
                tries += 1;
                // The bytes may have arrived and only the answer been lost: they are not sent twice.
                match cloud.upload_status(tenant, project, id).await {
                    Ok(status) if status.received => return Ok(status),
                    Ok(_) => {}
                    Err(e) if e.transient() => {}
                    Err(e) => return Err(e),
                }
            }
            Err(f) => return Err(f),
        }
    }
}

/// Commits a received upload as the file project's next revision, based on `based_on`.
async fn commit(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    upload: &FileUpload,
    based_on: u64,
) -> Result<FileCommitted, ApiFailure> {
    let command = envelope(
        tenant,
        project,
        PROJECT_FILE_COMMIT,
        PROJECT_FILE_COMMIT_VERSION,
        Uuid::new_v4(),
        [(FILE_KEY.to_string(), based_on.to_string())].into(),
        json!({ "uploadId": upload.id }),
    );
    retrying(|| cloud.command::<FileCommitted>(command.clone())).await
}

/// The file revision `based_on` was replaced meanwhile: the revision the server has now.
pub fn conflicting_revision(failure: &ApiFailure) -> Option<u64> {
    if !failure.conflict() {
        return None;
    }
    failure
        .conflicts
        .iter()
        .find(|c| c.id == FILE_KEY)
        .and_then(|c| c.actual.as_deref())
        .and_then(|a| a.parse().ok())
}

/// Saves `bytes` (a `.kcad` v2 file) as the next revision of a file project,
/// based on revision `based_on` (0 before the first). A conflict
/// ([`conflicting_revision`]) writes nothing.
pub fn save_revision(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    bytes: Vec<u8>,
    based_on: u64,
) -> impl Future<Output = Result<FileCommitted, ApiFailure>> + Send + 'static {
    save_revision_watched(cloud, tenant, project, bytes, based_on, PART, None)
}

/// `save_revision`, reporting the bytes the server has, of all of them, to
/// `progress`, a file larger than `part` sent in parts of that size.
pub fn save_revision_watched(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    bytes: Vec<u8>,
    based_on: u64,
    part: usize,
    progress: Option<Progress>,
) -> impl Future<Output = Result<FileCommitted, ApiFailure>> + Send + 'static {
    let cloud = cloud.clone();
    run(async move {
        let sent = upload_parted(
            &cloud,
            tenant,
            project,
            Bytes::from(bytes),
            part.max(1),
            progress.as_ref(),
        )
        .await?;
        commit(&cloud, tenant, project, &sent, based_on).await
    })
}

/// What a new cloud project got from the drawing.
#[derive(Clone, Debug, PartialEq)]
pub enum Uploaded {
    /// A file project with the drawing as its revision 1.
    File(FileCommitted),
    /// A database project with the drawing's objects, each at version 1.
    Database(ProjectImported),
}

/// The project a drawing makes: its metadata (the catalog's name,
/// description, type and tags are the caller's).
pub fn project_create(doc: &Document, name: &str, storage: ProjectStorage) -> ProjectCreate {
    ProjectCreate {
        name: name.trim().to_string(),
        settings: doc.settings().clone(),
        origin: doc.origin(),
        home_view: doc.home_view(),
        layers: doc.layers().nodes().to_vec(),
        active_layer: doc.layers().active().to_string(),
        styles: doc.styles().clone(),
        description: None,
        project_type: None,
        tags: None,
        storage: Some(storage),
    }
}

/// Makes `bytes` (the drawing `create` describes, as a `.kcad` v2 file) a new
/// cloud project in `tenant`, kept as `create.storage` says, and returns the
/// project as it stands after with what it got.
///
/// `key` is the creation's idempotency key, one per upload the user asked
/// for: called again with it (after a failure), the server answers with the
/// same project, and a project that already got its content is not filled a
/// second time; its content is reported as it is (`replayed`).
pub fn upload_new(
    cloud: &Cloud,
    tenant: Uuid,
    create: ProjectCreate,
    bytes: Vec<u8>,
    key: Uuid,
) -> impl Future<Output = Result<(ProjectInfo, Uploaded), ApiFailure>> + Send + 'static {
    let cloud = cloud.clone();
    run(async move {
        // A file the server would never take creates no project.
        checked_size(bytes.len())?;
        let storage = create.storage.unwrap_or_default();
        let made = retrying(|| cloud.create_project(tenant, create.clone(), key)).await?;
        let project = Uuid::parse_str(&made.id).map_err(|_| {
            ApiFailure::unreadable(
                201,
                "proje",
                format!("“{}” bir proje kimliği değil", made.id),
            )
        })?;
        match fill(&cloud, tenant, project, storage, Bytes::from(bytes)).await {
            Ok(uploaded) => Ok((retrying(|| cloud.project(tenant, project)).await?, uploaded)),
            Err(failure) => {
                // Refused for good: the empty project is not left behind. A passing failure keeps
                // it, so the same key finds it again and the upload is tried once more.
                if !failure.transient() {
                    let trash = envelope(
                        tenant,
                        project,
                        PROJECT_TRASH,
                        PROJECT_TRASH_VERSION,
                        Uuid::new_v4(),
                        BTreeMap::new(),
                        serde_json::to_value(EmptyInput {}).unwrap_or(json!({})),
                    );
                    let _ = retrying(|| cloud.command::<ProjectCatalogChange>(trash.clone())).await;
                }
                Err(failure)
            }
        }
    })
}

/// A new project's content from the file, unless an earlier try gave it already.
async fn fill(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    storage: ProjectStorage,
    bytes: Bytes,
) -> Result<Uploaded, ApiFailure> {
    match storage {
        ProjectStorage::File => {
            let list = retrying(|| cloud.file_revisions(tenant, project)).await?;
            if let Some(first) = list.revisions.into_iter().next() {
                return Ok(Uploaded::File(FileCommitted {
                    revision: first.revision,
                    size: first.size,
                    sha256: first.sha256,
                    objects: first.objects,
                    replayed: true,
                }));
            }
            let sent = upload(cloud, tenant, project, bytes).await?;
            Ok(Uploaded::File(
                commit(cloud, tenant, project, &sent, 0).await?,
            ))
        }
        ProjectStorage::Database => {
            let info = retrying(|| cloud.project(tenant, project)).await?;
            if info.data_revision != "0" {
                return Ok(Uploaded::Database(ProjectImported {
                    objects: info.feature_count,
                    data_revision: info.data_revision,
                    meta_version: info.meta_version,
                    replayed: true,
                }));
            }
            let sent = upload(cloud, tenant, project, bytes).await?;
            let command = envelope(
                tenant,
                project,
                PROJECT_IMPORT,
                PROJECT_IMPORT_VERSION,
                Uuid::new_v4(),
                BTreeMap::new(),
                json!({ "uploadId": sent.id }),
            );
            Ok(Uploaded::Database(
                retrying(|| cloud.command::<ProjectImported>(command.clone())).await?,
            ))
        }
    }
}
