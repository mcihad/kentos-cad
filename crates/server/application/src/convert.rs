//! `project.convert` v1 (docs/adr/0039; TODOS.md §10.1): a new project in
//! the other storage mode from this one's present state. Moving between the
//! modes is never silent ("PostGIS'e aktar" is an explicit command), and a
//! drawing never has two writable authorities: the source stays as it is,
//! and the new project is another one, made as a copy is (docs/adr/0028).
//!
//! - **File → database ("PostGIS'e aktar"):** the newest revision is read,
//!   checked against its hash and imported: every object under its
//!   persistent id at version 1. The first object the server does not keep
//!   refuses it by its place (`entities[i]`).
//! - **Database → file:** the snapshot of one moment (docs/adr/0033) becomes
//!   revision 1 of a new file project. Its object is written to the store
//!   first and removed if the rows are not written.
//! - **Who:** the source's `project.download` (a conversion is a download
//!   into the cloud, as a copy is) and project creation in the target
//!   workspace. The source's lock is held only while the rows are written.

use kentos_contracts::{
    CommandEnvelope, PROJECT_CONVERT, PROJECT_CONVERT_VERSION, ProjectConvert, ProjectDuplicated,
    ProjectPermission, ProjectStorage,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::{Blobs, WriteError};
use crate::changes::{check_target, lock};
use crate::commands::input;
use crate::duplicate::Origin;
use crate::error::{AppError, AppResult};
use crate::importing::{self, Content, Drawing};
use crate::projects::{check_name, check_tree, gone, storage_of};
use crate::restore::restore_name;
use crate::snapshot::{self, DrawingMeta};
use crate::tenancy::{self, Capability};
use crate::{files, idempotency};

/// The name of a converted project unless it is given one: the source's with its new mode after it.
fn converted_name(name: &str, to: ProjectStorage) -> String {
    restore_name(
        name,
        match to {
            ProjectStorage::Database => "PostGIS",
            ProjectStorage::File => "dosya",
        },
    )
}

/// What writing the rows ended in, the commit still to come.
enum Done {
    Earlier(ProjectDuplicated),
    New(ProjectDuplicated),
}

/// Under the source's lock: a retry's stored answer, or the new project's rows.
async fn write(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
    drawing: Drawing<'_>,
) -> AppResult<Done> {
    let now = lock(tx, access).await?;
    if let Some(earlier) =
        idempotency::earlier::<ProjectDuplicated>(tx, &now, envelope, text).await?
    {
        return Ok(Done::Earlier(earlier));
    }
    if now.deleted {
        return Err(gone(&now.name));
    }
    now.require(ProjectPermission::Download)?;
    importing::create_from_drawing(tx, &now, envelope, text, drawing)
        .await
        .map(Done::New)
}

/// `project.convert` v1.
pub async fn convert(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectDuplicated> {
    let ProjectConvert {
        to,
        name,
        tenant_id,
    } = input(&envelope, PROJECT_CONVERT, PROJECT_CONVERT_VERSION)?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    if let Some(n) = &name {
        check_name(n)?;
    }
    // Where the new project goes: a workspace the caller may open projects in (404 for one they are not in).
    let target = match tenant_id.as_deref() {
        None => access.tenant,
        Some(t) => Uuid::parse_str(t)
            .map_err(|_| AppError::not_found("Kurum bulunamadı ya da üyesi değilsiniz."))?,
    };
    tenancy::access(db, &access.actor, target)
        .await?
        .require(Capability::ProjectCreate)?;
    access.live()?;
    access.require(ProjectPermission::Download)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let storage: String =
        sqlx::query_scalar("select storage from kentos.project where tenant_id = $1 and id = $2")
            .bind(access.tenant)
            .bind(access.project)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(crate::access::not_found)?;
    let newest = files::newest(&mut tx, access.tenant, access.project).await?;
    tx.commit().await?;
    if storage_of(&storage) == to {
        return Err(AppError::invalid_at(
            "to",
            format!(
                "“{}” zaten {} olarak saklanıyor; dönüştürme öbür biçime yeni bir proje açar.",
                access.name,
                match to {
                    ProjectStorage::Database => "veritabanında",
                    ProjectStorage::File => "dosya",
                }
            ),
        ));
    }
    let name = name
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| converted_name(&access.name, to));
    let id = Uuid::now_v7();
    match to {
        // A file project's newest revision into a database project ("PostGIS'e aktar").
        ProjectStorage::Database => {
            let row = newest.ok_or_else(|| {
                AppError::invalid(format!(
                    "“{}” projesinin henüz kaydedilmiş bir revizyonu yok; önce projeyi kaydedin.",
                    access.name
                ))
            })?;
            let doc = importing::read(blobs, &row.7, Some(&row.2), "dosya revizyonu").await?;
            let rows = importing::objects(&doc)?;
            check_tree(&doc.layers, &doc.active_layer)?;
            let meta = DrawingMeta::of(&doc);
            let drawing = Drawing {
                target,
                id,
                name: &name,
                meta: &meta,
                content: Content::Objects(&rows),
                origin: Origin::Convert {
                    to,
                    revision: row.0,
                },
            };
            let mut tx = db.scoped(access.scope()).await?;
            match write(&mut tx, access, &envelope, &text, drawing).await? {
                Done::New(result) => {
                    tx.commit().await?;
                    Ok(result)
                }
                Done::Earlier(mut earlier) => {
                    tx.commit().await?;
                    earlier.replayed = true;
                    Ok(earlier)
                }
            }
        }
        // A database project's snapshot of one moment as a file project's revision 1.
        ProjectStorage::File => {
            let snap = snapshot::take(db, access).await?;
            let key = Blobs::revision_key(target, id, 1, &snap.sha256);
            let size = i64::try_from(snap.bytes.len()).unwrap_or(i64::MAX);
            let mut writer = blobs.create(&key, snap.bytes.len() as u64).await?;
            if let Err(e) = writer.write(&snap.bytes).await {
                let _ = blobs.remove(&key).await;
                return Err(match e {
                    WriteError::Io(e) => AppError::Storage(e),
                    other => AppError::Storage(std::io::Error::other(format!("{other:?}"))),
                });
            }
            writer.finish().await?;
            let drawing = Drawing {
                target,
                id,
                name: &name,
                meta: &snap.meta,
                content: Content::File {
                    key: &key,
                    size,
                    sha256: &snap.sha256,
                    objects: i64::try_from(snap.objects).unwrap_or(i64::MAX),
                },
                origin: Origin::Convert {
                    to,
                    revision: snap.revision,
                },
            };
            let mut tx = db.scoped(access.scope()).await?;
            match write(&mut tx, access, &envelope, &text, drawing).await {
                Ok(Done::New(result)) => {
                    // A commit that fails may still have been written: the object stays; if
                    // nothing was, the store's cleanup removes it with the project that does not exist.
                    tx.commit().await?;
                    Ok(result)
                }
                Ok(Done::Earlier(mut earlier)) => {
                    tx.commit().await?;
                    // The earlier command wrote its own object; this one's is nobody's.
                    let _ = blobs.remove(&key).await;
                    earlier.replayed = true;
                    Ok(earlier)
                }
                Err(e) => {
                    let _ = blobs.remove(&key).await;
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_converted_project_is_named_after_its_source_and_mode() {
        assert_eq!(
            converted_name("Ada 101", ProjectStorage::Database),
            "Ada 101 (PostGIS)"
        );
        assert_eq!(
            converted_name("Ada 101", ProjectStorage::File),
            "Ada 101 (dosya)"
        );
    }
}
