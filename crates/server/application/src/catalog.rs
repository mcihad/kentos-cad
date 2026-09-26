//! What describes a project in the catalog (docs/adr/0028, TODOS.md
//! CLOUD-02, CLOUD-03): its name, description, type and tags, and the
//! commands that change them (`project.rename`, `project.metadata.update`),
//! and a person's favourites (`project.favorite`).
//!
//! - Changing the metadata needs `project.edit`; an archived project (409)
//!   or one in the trash (410) keeps its own.
//! - Every change raises the project's catalog version. A command that gives
//!   the version it was prepared from (`expectedVersions["@catalog"]`) is
//!   refused when the project moved on (409): nobody's edit is overwritten
//!   unseen. A new name is also the drawing's (its meta version rises, open
//!   editors take it from the `meta` event).
//! - Each change is audited with what changed (not the description's text)
//!   and heard as a `project.metadata` event.
//! - A favourite is the caller's own: no audit, no event, nothing another
//!   person sees.

use kentos_contracts::{
    CommandEnvelope, ConflictReason, FeatureConflict, PROJECT_CATALOG_KEY, PROJECT_DESCRIPTION_MAX,
    PROJECT_FAVORITE, PROJECT_FAVORITE_VERSION, PROJECT_METADATA_CHANGED, PROJECT_METADATA_UPDATE,
    PROJECT_METADATA_UPDATE_VERSION, PROJECT_RENAME, PROJECT_RENAME_VERSION, PROJECT_TAG_MAX,
    PROJECT_TAGS_MAX, ProjectCatalogChange, ProjectFavorite, ProjectMetadataUpdate,
    ProjectPermission, ProjectRename, ProjectType,
};
use serde_json::{Value, json};

use crate::access::{ProjectAccess, archived};
use crate::changes::{check_target, lock};
use crate::commands::input;
use crate::error::{AppError, AppResult};
use crate::people::fold;
use crate::projects::{check_name, gone};
use crate::{idempotency, journal, listing};

/// A description as stored: surrounding spaces dropped, at most 2 000 characters.
pub fn check_description(text: &str) -> AppResult<String> {
    let text = text.trim();
    if text.chars().count() > PROJECT_DESCRIPTION_MAX {
        return Err(AppError::invalid_at(
            "description",
            format!(
                "Açıklama en çok {PROJECT_DESCRIPTION_MAX} karakter olabilir ({} yazıldı).",
                text.chars().count()
            ),
        ));
    }
    Ok(text.to_string())
}

/// Tags as stored: each trimmed with inner spaces collapsed, empty ones
/// dropped, a tag equal to an earlier one but for case or Turkish letters
/// dropped; at most 12, each at most 32 characters.
pub fn normalize_tags(tags: &[String]) -> AppResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (i, tag) in tags.iter().enumerate() {
        let tag = tag.split_whitespace().collect::<Vec<_>>().join(" ");
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > PROJECT_TAG_MAX {
            return Err(AppError::invalid_at(
                format!("tags[{i}]"),
                format!(
                    "“{tag}” etiketi çok uzun; bir etiket en çok {PROJECT_TAG_MAX} karakter olabilir."
                ),
            ));
        }
        let key = fold(&tag);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push(tag);
    }
    if out.len() > PROJECT_TAGS_MAX {
        return Err(AppError::invalid_at(
            "tags",
            format!(
                "Bir projenin en çok {PROJECT_TAGS_MAX} etiketi olabilir ({} verildi).",
                out.len()
            ),
        ));
    }
    Ok(out)
}

/// A metadata change, checked and normalized before the database is asked.
struct Patch {
    name: Option<String>,
    description: Option<String>,
    project_type: Option<ProjectType>,
    tags: Option<Vec<String>>,
}

fn checked(update: ProjectMetadataUpdate) -> AppResult<Patch> {
    let name = update
        .name
        .map(|n| {
            check_name(&n)?;
            Ok::<_, AppError>(n.trim().to_string())
        })
        .transpose()?;
    Ok(Patch {
        name,
        description: update
            .description
            .as_deref()
            .map(check_description)
            .transpose()?,
        project_type: update.project_type,
        tags: update.tags.as_deref().map(normalize_tags).transpose()?,
    })
}

/// `project.rename` v1.
pub async fn rename(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectCatalogChange> {
    let ProjectRename { name } = input(&envelope, PROJECT_RENAME, PROJECT_RENAME_VERSION)?;
    let update = ProjectMetadataUpdate {
        name: Some(name),
        ..Default::default()
    };
    apply(db, access, envelope, checked(update)?).await
}

/// `project.metadata.update` v1.
pub async fn update(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectCatalogChange> {
    let update: ProjectMetadataUpdate = input(
        &envelope,
        PROJECT_METADATA_UPDATE,
        PROJECT_METADATA_UPDATE_VERSION,
    )?;
    apply(db, access, envelope, checked(update)?).await
}

type Current = (String, String, String, Vec<String>, i64);

async fn apply(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
    patch: Patch,
) -> AppResult<ProjectCatalogChange> {
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let expected = envelope
        .expected_versions
        .get(PROJECT_CATALOG_KEY)
        .map(|v| {
            v.parse::<i64>().map_err(|_| {
                AppError::invalid(format!(
                    "expectedVersions[\"{PROJECT_CATALOG_KEY}\"] bir tamsayı değil: {v}"
                ))
            })
        })
        .transpose()?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let now = lock(&mut tx, access).await?;
    if let Some(mut earlier) =
        idempotency::earlier::<ProjectCatalogChange>(&mut tx, &now, &envelope, &text).await?
    {
        tx.commit().await?;
        earlier.replayed = true;
        return Ok(earlier);
    }
    if now.deleted {
        return Err(gone(&now.name));
    }
    now.require(ProjectPermission::Edit)?;
    if now.archived {
        return Err(archived(&now.name));
    }
    let (name, description, project_type, tags, version): Current = sqlx::query_as(
        "select name, description, project_type, tags, catalog_version from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(now.tenant)
    .bind(now.project)
    .fetch_one(&mut *tx)
    .await?;
    if let Some(want) = expected
        && want != version
    {
        return Err(AppError::Conflict {
            message: "Proje bilgileri siz düzenlerken başka biri tarafından değiştirildi; hiçbir şey kaydedilmedi. Güncel bilgilere bakıp yeniden deneyin.".into(),
            conflicts: vec![FeatureConflict {
                id: PROJECT_CATALOG_KEY.into(),
                reason: ConflictReason::Project,
                expected: Some(want.to_string()),
                actual: Some(version.to_string()),
                current: None,
            }],
            // The catalog's own version is in the conflict; the data revision is not what it is about.
            revision: None,
        });
    }
    // What differs from the project now; the rest is left as it is.
    let new_name = patch.name.filter(|n| *n != name);
    let new_description = patch.description.filter(|d| *d != description);
    let new_type = patch
        .project_type
        .filter(|t| t.name() != project_type)
        .map(|t| t.name());
    let new_tags = patch.tags.filter(|t| *t != tags);
    let mut detail = serde_json::Map::new();
    if let Some(n) = &new_name {
        detail.insert("name".into(), json!({ "from": name, "to": n }));
    }
    if new_description.is_some() {
        detail.insert("description".into(), Value::Bool(true));
    }
    if let Some(t) = new_type {
        detail.insert(
            "projectType".into(),
            json!({ "from": project_type, "to": t }),
        );
    }
    if let Some(t) = &new_tags {
        detail.insert("tags".into(), json!({ "from": tags, "to": t }));
    }
    let changed = !detail.is_empty();
    let mut event_seq = None;
    if changed {
        let renamed = new_name.is_some();
        // A new name is the drawing's too: its meta version and data revision rise as with project.changes.
        let revision: i64 = sqlx::query_scalar(
            "update kentos.project set name = coalesce($3, name), description = coalesce($4, description),
                    project_type = coalesce($5, project_type), tags = coalesce($6, tags),
                    catalog_version = catalog_version + 1, meta_version = meta_version + $7::int,
                    data_revision = data_revision + $7::int, updated_at = now()
              where tenant_id = $1 and id = $2 returning data_revision",
        )
        .bind(now.tenant)
        .bind(now.project)
        .bind(&new_name)
        .bind(&new_description)
        .bind(new_type)
        .bind(&new_tags)
        .bind(i32::from(renamed))
        .fetch_one(&mut *tx)
        .await?;
        let request = Some(envelope.request_id.as_str());
        journal::audit(
            &mut tx,
            &now,
            &envelope.command_name,
            request,
            revision,
            Value::Object(detail),
        )
        .await?;
        let seq = journal::event(
            &mut tx,
            &now,
            PROJECT_METADATA_CHANGED,
            request,
            revision,
            renamed,
        )
        .await?;
        event_seq = Some(seq.to_string());
    }
    let result = ProjectCatalogChange {
        project: listing::entry(&mut tx, now.tenant, now.project).await?,
        changed,
        event_seq,
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &result).await?;
    tx.commit().await?;
    Ok(result)
}

/// `project.favorite` v1: the caller's own favourites.
pub async fn favorite(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectCatalogChange> {
    let ProjectFavorite { favorite } =
        input(&envelope, PROJECT_FAVORITE, PROJECT_FAVORITE_VERSION)?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let now = lock(&mut tx, access).await?;
    if let Some(mut earlier) =
        idempotency::earlier::<ProjectCatalogChange>(&mut tx, &now, &envelope, &text).await?
    {
        tx.commit().await?;
        earlier.replayed = true;
        return Ok(earlier);
    }
    if now.deleted {
        return Err(gone(&now.name));
    }
    now.require(ProjectPermission::Read)?;
    let sql = if favorite {
        "insert into kentos.project_favorite (tenant_id, project_id, user_id) values ($1, $2, $3) on conflict do nothing"
    } else {
        "delete from kentos.project_favorite where tenant_id = $1 and project_id = $2 and user_id = $3"
    };
    let done = sqlx::query(sql)
        .bind(now.tenant)
        .bind(now.project)
        .bind(now.actor.user_id)
        .execute(&mut *tx)
        .await?;
    let result = ProjectCatalogChange {
        project: listing::entry(&mut tx, now.tenant, now.project).await?,
        changed: done.rows_affected() > 0,
        event_seq: None,
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &result).await?;
    tx.commit().await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn tags_are_trimmed_folded_for_duplicates_and_limited() {
        assert_eq!(
            normalize_tags(&tags(&["  Kadıköy ", "", "KADIKÖY", "kadikoy", "DOP  %40"])).unwrap(),
            ["Kadıköy", "DOP %40"]
        );
        assert!(normalize_tags(&tags(&[&"ş".repeat(PROJECT_TAG_MAX + 1)])).is_err());
        assert_eq!(
            normalize_tags(&tags(&[&"ş".repeat(PROJECT_TAG_MAX)]))
                .unwrap()
                .len(),
            1
        );
        let many: Vec<String> = (0..=PROJECT_TAGS_MAX).map(|i| format!("e{i}")).collect();
        assert!(normalize_tags(&many).is_err());
        assert_eq!(
            normalize_tags(&many[..PROJECT_TAGS_MAX]).unwrap().len(),
            PROJECT_TAGS_MAX
        );
    }

    #[test]
    fn descriptions_are_trimmed_and_limited_in_characters() {
        assert_eq!(check_description("  Ada 101  ").unwrap(), "Ada 101");
        // Characters, not bytes: 2 000 Turkish letters are 4 000 bytes and still fit.
        assert!(check_description(&"ğ".repeat(PROJECT_DESCRIPTION_MAX)).is_ok());
        assert!(check_description(&"a".repeat(PROJECT_DESCRIPTION_MAX + 1)).is_err());
    }
}
