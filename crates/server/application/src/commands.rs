//! The product commands the server runs (docs/adr/0013): one handler per
//! catalog command marked `server` ([`crate::SERVER_COMMANDS`],
//! `tests/catalog.rs` keeps the two equal). Each checks its own version,
//! input and permissions.
//!
//! - A project's commands ([`run`]) go to the project's route and start from
//!   the caller's [`ProjectAccess`]: its content (`project.changes`), its
//!   sharing, its catalog metadata and its lifecycle (docs/adr/0028).
//! - Opening a new project (`project.create`, [`run_in_tenant`]) has no
//!   project yet: it goes to the workspace's route and starts from the
//!   caller's membership of it.

use std::time::Duration;

use kentos_contracts::{
    CheckpointChange, CommandEnvelope, CommitResult, FileCommitted, PROJECT_ACCESS_REVOKE,
    PROJECT_ARCHIVE, PROJECT_CHANGES, PROJECT_CHECKPOINT_CREATE, PROJECT_CHECKPOINT_DELETE,
    PROJECT_CREATE, PROJECT_CREATE_VERSION, PROJECT_DUPLICATE, PROJECT_FAVORITE,
    PROJECT_FILE_COMMIT, PROJECT_METADATA_UPDATE, PROJECT_PURGE, PROJECT_RENAME, PROJECT_RESTORE,
    PROJECT_SHARE, PROJECT_TRASH, PROJECT_UNARCHIVE, ProjectAccessChange, ProjectCatalogChange,
    ProjectCreate, ProjectDuplicated, ProjectInfo, ProjectPurged,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::access::ProjectAccess;
use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};
use crate::lifecycle::StateChange;
use crate::tenancy::Access;
use crate::{
    catalog, changes, checkpoints, duplicate, files, idempotency, lifecycle, projects, sharing,
};

/// How the server keeps its catalog (docs/adr/0028): how long a project
/// moved to the trash stays there before it is removed for good.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogPolicy {
    pub trash_retention: Duration,
}

/// The retention unless the server is told otherwise (`KENTOS_TRASH_RETENTION_DAYS`).
pub const DEFAULT_TRASH_RETENTION: Duration = Duration::from_secs(30 * 24 * 3600);

impl Default for CatalogPolicy {
    fn default() -> Self {
        Self {
            trash_retention: DEFAULT_TRASH_RETENTION,
        }
    }
}

/// What a command answered.
#[derive(Clone, Debug, PartialEq)]
pub enum CommandOutcome {
    Changes(CommitResult),
    Access(ProjectAccessChange),
    Catalog(ProjectCatalogChange),
    Duplicated(ProjectDuplicated),
    Purged(ProjectPurged),
    Created(ProjectInfo),
    /// A file project's new revision (docs/adr/0031).
    FileCommitted(FileCommitted),
    /// A checkpoint made or removed (docs/adr/0034).
    Checkpoint(CheckpointChange),
}

impl CommandOutcome {
    /// Something was committed now that the project's open connections hear
    /// of (not a retry's stored answer, not a no-op): they are told.
    pub fn committed(&self) -> bool {
        match self {
            Self::Changes(r) => !r.replayed,
            Self::Access(r) => !r.replayed && r.changed,
            Self::Catalog(r) => !r.replayed && r.event_seq.is_some(),
            // The source of a copy does not change; a new project has no connections yet.
            Self::Duplicated(_) | Self::Created(_) => false,
            // Its connections ask again and learn it is gone.
            Self::Purged(_) => true,
            Self::FileCommitted(r) => !r.replayed,
            Self::Checkpoint(r) => !r.replayed,
        }
    }

    /// The answer as the API sends it (the command's own output type).
    pub fn to_json(&self) -> Value {
        match self {
            Self::Changes(r) => serde_json::to_value(r),
            Self::Access(r) => serde_json::to_value(r),
            Self::Catalog(r) => serde_json::to_value(r),
            Self::Duplicated(r) => serde_json::to_value(r),
            Self::Purged(r) => serde_json::to_value(r),
            Self::Created(r) => serde_json::to_value(r),
            Self::FileCommitted(r) => serde_json::to_value(r),
            Self::Checkpoint(r) => serde_json::to_value(r),
        }
        .expect("command results serialize")
    }
}

/// The command's input, after checking its name and version (an input that
/// does not fit the command's type is refused, whatever else it holds).
pub(crate) fn input<T: DeserializeOwned>(
    envelope: &CommandEnvelope,
    name: &str,
    version: u32,
) -> AppResult<T> {
    if envelope.command_name != name {
        return Err(AppError::invalid(format!(
            "Bilinmeyen komut: {}",
            envelope.command_name
        )));
    }
    if envelope.version != version {
        return Err(AppError::invalid(format!(
            "{name} komutunun {} sürümü desteklenmiyor (desteklenen: {version}).",
            envelope.version
        )));
    }
    serde_json::from_value(envelope.input.clone())
        .map_err(|e| AppError::invalid(format!("Komut girdisi okunamadı: {e}")))
}

/// Runs the envelope's command on the project `access` names; `blobs` is the
/// object store of file projects (docs/adr/0031).
pub async fn run(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    policy: &CatalogPolicy,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<CommandOutcome> {
    use CommandOutcome as O;
    let state = |change| lifecycle::change_state(db, policy, access, envelope.clone(), change);
    match envelope.command_name.as_str() {
        PROJECT_CHANGES => changes::commit(db, access, envelope).await.map(O::Changes),
        PROJECT_SHARE => sharing::share(db, access, envelope).await.map(O::Access),
        PROJECT_ACCESS_REVOKE => sharing::revoke(db, access, envelope).await.map(O::Access),
        PROJECT_RENAME => catalog::rename(db, access, envelope).await.map(O::Catalog),
        PROJECT_METADATA_UPDATE => catalog::update(db, access, envelope).await.map(O::Catalog),
        PROJECT_FAVORITE => catalog::favorite(db, access, envelope)
            .await
            .map(O::Catalog),
        PROJECT_ARCHIVE => state(StateChange::Archive).await.map(O::Catalog),
        PROJECT_UNARCHIVE => state(StateChange::Unarchive).await.map(O::Catalog),
        PROJECT_TRASH => state(StateChange::Trash).await.map(O::Catalog),
        PROJECT_RESTORE => state(StateChange::Restore).await.map(O::Catalog),
        PROJECT_DUPLICATE => duplicate::duplicate(db, blobs, access, envelope)
            .await
            .map(O::Duplicated),
        PROJECT_PURGE => lifecycle::purge(db, access, envelope).await.map(O::Purged),
        PROJECT_FILE_COMMIT => files::commit(db, blobs, access, envelope)
            .await
            .map(O::FileCommitted),
        PROJECT_CHECKPOINT_CREATE => checkpoints::create(db, blobs, access, envelope)
            .await
            .map(O::Checkpoint),
        PROJECT_CHECKPOINT_DELETE => checkpoints::delete(db, blobs, access, envelope)
            .await
            .map(O::Checkpoint),
        PROJECT_CREATE => Err(AppError::invalid(
            "project.create bir çalışma alanına gönderilir: POST /v1/tenants/{çalışma alanı}/commands (projectId boş).",
        )),
        other => Err(AppError::invalid(format!("Bilinmeyen komut: {other}"))),
    }
}

/// Runs a command that has no project yet (`project.create`) in the
/// workspace `access` names. The envelope names that workspace and no project.
pub async fn run_in_tenant(
    db: &kentos_postgres::Db,
    access: &Access,
    envelope: CommandEnvelope,
) -> AppResult<CommandOutcome> {
    if envelope.command_name != PROJECT_CREATE {
        return Err(
            if crate::SERVER_COMMANDS
                .iter()
                .any(|(n, _)| *n == envelope.command_name)
            {
                AppError::invalid(format!(
                    "{} bir projeye gönderilir: POST /v1/tenants/{{çalışma alanı}}/projects/{{proje}}/commands.",
                    envelope.command_name
                ))
            } else {
                AppError::invalid(format!("Bilinmeyen komut: {}", envelope.command_name))
            },
        );
    }
    let create: ProjectCreate = input(&envelope, PROJECT_CREATE, PROJECT_CREATE_VERSION)?;
    if uuid::Uuid::parse_str(&envelope.tenant_id).ok() != Some(access.tenant) {
        return Err(AppError::invalid(
            "Komutun çalışma alanı adresteki çalışma alanıyla aynı değil.",
        ));
    }
    if !envelope.project_id.is_empty() {
        return Err(AppError::invalid(
            "project.create yeni bir proje açar: projectId boş olmalı; projenin kimliğini sunucu verir.",
        ));
    }
    idempotency::check_key(&envelope)?;
    projects::create(db, access, create, Some(&envelope.idempotency_key))
        .await
        .map(CommandOutcome::Created)
}
