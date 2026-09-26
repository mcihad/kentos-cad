//! `project.duplicate` v1 (docs/adr/0028, TODOS.md CLOUD-05): a copy of a
//! project as a new project with its own id and its own access.
//!
//! - **Copied:** the settings, layer tree (locks included), styles, origin,
//!   home view, description, type and tags, and every object with its
//!   persistent id (docs/adr/0014, 0026: the same ids in two projects are two
//!   independent objects), each at version 1: the copy's first commit is its
//!   data revision 1.
//! - **A file project** (docs/adr/0031) is copied as a file project whose
//!   revision 1 is the source's newest revision: the object is shared in the
//!   store under the copy's key before the rows are written; a copy that is
//!   not committed leaves an object of no project, which the store's
//!   cleanup removes.
//! - **Not copied:** history (command log, events, audit records, older file
//!   revisions), grants, favourites and recents, the archived state. The copy is the caller's:
//!   they own it, and only they (and, in an organisation, its owners and
//!   admins under its policy) see it until they share it.
//! - **Who:** the source needs `project.download` (a copy is a download into
//!   the cloud: the organisation's `viewer_download` keeps viewers from
//!   copying when it keeps them from downloading); the workspace the copy
//!   goes to needs `project.create`. A project in the trash is not copied.
//! - **How:** `kentos.duplicate_project` copies the rows in one statement
//!   (the server's role reads one project at a time), checking the same
//!   rights itself. The source's audit records the copy and keeps the
//!   answer for a retry; the copy's audit records its creation from the
//!   source. One transaction, under the source's lock: a retry waits and is
//!   answered from the log, never copied twice. External data sources, when
//!   they come, are a choice of this command (docs/adr/0028).

use kentos_contracts::{
    CommandEnvelope, PROJECT_CHECKPOINT_RESTORE, PROJECT_DUPLICATE, PROJECT_DUPLICATE_VERSION,
    ProjectDuplicate, ProjectDuplicated, ProjectPermission,
};
use kentos_postgres::{Scope, rescope};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::Blobs;
use crate::changes::{check_target, lock};
use crate::commands::input;
use crate::error::{AppError, AppResult};
use crate::files::{self, RevisionRow};
use crate::projects::{check_name, gone, opened};
use crate::tenancy::{self, Capability};
use crate::{idempotency, journal, listing};

/// What a copy is called unless it is given a name: the source's, with this after it.
pub const COPY_SUFFIX: &str = " (kopya)";

/// The default name of a copy of `name`, kept within the 200 bytes of a name.
pub fn copy_name(name: &str) -> String {
    let room = 200 - COPY_SUFFIX.len();
    let mut end = name.len().min(room);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{COPY_SUFFIX}", name[..end].trim_end())
}

pub async fn duplicate(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectDuplicated> {
    let ProjectDuplicate { name, tenant_id } =
        input(&envelope, PROJECT_DUPLICATE, PROJECT_DUPLICATE_VERSION)?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    if let Some(n) = &name {
        check_name(n)?;
    }
    // Where the copy goes: a workspace the caller may open projects in (404 for one they are not in).
    let target = match tenant_id.as_deref() {
        None => access.tenant,
        Some(t) => Uuid::parse_str(t)
            .map_err(|_| AppError::not_found("Kurum bulunamadı ya da üyesi değilsiniz."))?,
    };
    let into = tenancy::access(db, &access.actor, target).await?;
    into.require(Capability::ProjectCreate)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let now = lock(&mut tx, access).await?;
    if let Some(mut earlier) =
        idempotency::earlier::<ProjectDuplicated>(&mut tx, &now, &envelope, &text).await?
    {
        tx.commit().await?;
        earlier.replayed = true;
        return Ok(earlier);
    }
    if now.deleted {
        return Err(gone(&now.name));
    }
    now.require(ProjectPermission::Download)?;
    let storage: String =
        sqlx::query_scalar("select storage from kentos.project where tenant_id = $1 and id = $2")
            .bind(now.tenant)
            .bind(now.project)
            .fetch_one(&mut *tx)
            .await?;
    let name = name
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| copy_name(&now.name));
    let id = Uuid::now_v7();
    // A file project's content is its newest revision (docs/adr/0031): its object goes first.
    let newest = match storage.as_str() {
        "file" => files::newest(&mut tx, now.tenant, now.project).await?,
        _ => None,
    };
    let shared = match &newest {
        Some(row) => {
            let key = Blobs::revision_key(target, id, 1, &row.2);
            blobs.share(&row.7, &key).await?;
            Some(key)
        }
        None => None,
    };
    let copy = Copy {
        target,
        id,
        name: &name,
        file: newest.as_ref().zip(shared.as_deref()),
        origin: Origin::Duplicate,
    };
    let result = match write(&mut tx, &now, &envelope, &text, copy).await {
        Ok(result) => result,
        Err(e) => {
            // Nothing was written: the shared object goes too.
            if let Some(key) = &shared {
                let _ = blobs.remove(key).await;
            }
            return Err(e);
        }
    };
    // A commit that fails may still have been written (its answer lost), so the object
    // stays; if nothing was written, the store's cleanup removes it with its project.
    tx.commit().await?;
    Ok(result)
}

/// Where a copy comes from: the source as it is, or a point of its
/// history (`project.checkpoint.restore`, docs/adr/0034).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Origin {
    Duplicate,
    Restore {
        checkpoint: Option<Uuid>,
        revision: i64,
    },
}

impl Origin {
    /// The source's audit action.
    pub(crate) fn action(self) -> &'static str {
        match self {
            Self::Duplicate => PROJECT_DUPLICATE,
            Self::Restore { .. } => PROJECT_CHECKPOINT_RESTORE,
        }
    }

    /// What the audit records say of the point (nothing for a copy).
    pub(crate) fn detail(self) -> serde_json::Value {
        match self {
            Self::Duplicate => serde_json::Value::Null,
            Self::Restore {
                checkpoint,
                revision,
            } => json!({ "checkpoint": checkpoint, "revision": revision }),
        }
    }
}

/// The copy to make: where, its id and name, for a file project the
/// revision it starts from with the key its object was shared under, and
/// where it comes from.
pub(crate) struct Copy<'a> {
    pub target: Uuid,
    pub id: Uuid,
    pub name: &'a str,
    pub file: Option<(&'a RevisionRow, &'a str)>,
    pub origin: Origin,
}

/// The copy's rows, both projects' audit records and the stored answer, in the open transaction.
pub(crate) async fn write(
    tx: &mut Transaction<'static, Postgres>,
    now: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
    copy: Copy<'_>,
) -> AppResult<ProjectDuplicated> {
    let Copy {
        target,
        id,
        name,
        file,
        origin,
    } = copy;
    let request = Some(envelope.request_id.as_str());
    let features: i64 = sqlx::query_scalar("select kentos.duplicate_project($1, $2, $3, $4, $5)")
        .bind(now.tenant)
        .bind(now.project)
        .bind(target)
        .bind(id)
        .bind(name)
        .fetch_one(&mut **tx)
        .await?;
    let revision = journal::revision(tx, now).await?;
    // A file project's objects are in its file: the count its newest revision was verified with.
    let objects = match file {
        Some((row, _)) => row.6.unwrap_or(0),
        None => features,
    };
    journal::audit(
        tx,
        now,
        origin.action(),
        request,
        revision,
        json!({ "copy": id, "tenant": target, "name": name, "objects": objects, "from": origin.detail() }),
    )
    .await?;
    // The copy's own rows, in its scope: its file revision, its creation and its owner's recent use.
    let subject = journal::Subject {
        tenant: target,
        project: id,
        actor: now.actor.user_id,
    };
    rescope(
        tx,
        Scope {
            tenant: Some(target),
            user: Some(now.actor.user_id),
            project: Some(id),
        },
    )
    .await?;
    if let Some((row, key)) = file {
        sqlx::query(
            "insert into kentos.project_file_revision (tenant_id, project_id, revision, size, sha256, blob_key, created_by, request_id, objects)
             values ($1, $2, 1, $3, $4, $5, $6, $7, $8)",
        )
        .bind(target)
        .bind(id)
        .bind(row.1)
        .bind(&row.2)
        .bind(key)
        .bind(now.actor.user_id)
        .bind(&envelope.request_id)
        .bind(row.6)
        .execute(&mut **tx)
        .await?;
        sqlx::query(
            "update kentos.project set file_revision = 1, data_revision = 1 where tenant_id = $1 and id = $2",
        )
        .bind(target)
        .bind(id)
        .execute(&mut **tx)
        .await?;
    }
    journal::audit(
        tx,
        subject,
        "project.create",
        request,
        i64::from(objects > 0 || file.is_some()),
        json!({ "name": name, "copyOf": { "tenant": now.tenant, "project": now.project }, "from": origin.detail(), "objects": objects }),
    )
    .await?;
    opened(tx, target, id, now.actor.user_id).await?;
    let project = listing::entry(tx, target, id).await?;
    // Back in the source's scope, where its command log is.
    rescope(tx, now.scope()).await?;
    let result = ProjectDuplicated {
        project,
        source_id: now.project.to_string(),
        objects: objects.to_string(),
        replayed: false,
    };
    idempotency::record(tx, now, envelope, text, &result).await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_is_named_after_its_source_within_the_limit() {
        assert_eq!(copy_name("Ada 101"), "Ada 101 (kopya)");
        let long = "ş".repeat(100); // 200 bytes
        let named = copy_name(&long);
        assert!(named.len() <= 200 && named.ends_with(COPY_SUFFIX));
        assert!(named.starts_with('ş'));
    }
}
