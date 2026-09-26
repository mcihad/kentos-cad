//! `project.duplicate` v1 (docs/adr/0028, TODOS.md CLOUD-05): a copy of a
//! project as a new project with its own id and its own access.
//!
//! - **Copied:** the settings, layer tree (locks included), styles, origin,
//!   home view, description, type and tags, and every object with its
//!   persistent id (docs/adr/0014, 0026: the same ids in two projects are two
//!   independent objects), each at version 1: the copy's first commit is its
//!   data revision 1.
//! - **Not copied:** history (command log, events, audit records), grants,
//!   favourites and recents, the archived state. The copy is the caller's:
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
    CommandEnvelope, PROJECT_DUPLICATE, PROJECT_DUPLICATE_VERSION, ProjectDuplicate,
    ProjectDuplicated, ProjectPermission,
};
use kentos_postgres::{Scope, rescope};
use serde_json::json;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::changes::{check_target, lock};
use crate::commands::input;
use crate::error::{AppError, AppResult};
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
    let request = Some(envelope.request_id.as_str());
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
    let name = name
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| copy_name(&now.name));
    let id = Uuid::now_v7();
    let objects: i64 = sqlx::query_scalar("select kentos.duplicate_project($1, $2, $3, $4, $5)")
        .bind(now.tenant)
        .bind(now.project)
        .bind(target)
        .bind(id)
        .bind(&name)
        .fetch_one(&mut *tx)
        .await?;
    let revision = journal::revision(&mut tx, &now).await?;
    journal::audit(
        &mut tx,
        &now,
        PROJECT_DUPLICATE,
        request,
        revision,
        json!({ "copy": id, "tenant": target, "name": name, "objects": objects }),
    )
    .await?;
    // The copy's own rows, in its scope: its creation and its owner's recent use.
    let copy = journal::Subject {
        tenant: target,
        project: id,
        actor: now.actor.user_id,
    };
    rescope(
        &mut tx,
        Scope {
            tenant: Some(target),
            user: Some(now.actor.user_id),
            project: Some(id),
        },
    )
    .await?;
    journal::audit(
        &mut tx,
        copy,
        "project.create",
        request,
        i64::from(objects > 0),
        json!({ "name": name, "copyOf": { "tenant": now.tenant, "project": now.project }, "objects": objects }),
    )
    .await?;
    opened(&mut tx, target, id, now.actor.user_id).await?;
    let project = listing::entry(&mut tx, target, id).await?;
    // Back in the source's scope, where its command log is.
    rescope(&mut tx, now.scope()).await?;
    let result = ProjectDuplicated {
        project,
        source_id: now.project.to_string(),
        objects: objects.to_string(),
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &result).await?;
    tx.commit().await?;
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
