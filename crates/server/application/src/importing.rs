//! A `.kcad` file into a database project (docs/adr/0034, 0036): restoring a
//! checkpoint as a new project, and `project.import` of an upload.
//!
//! - [`read`] reads a stored file whole, checks its SHA-256 when it is known
//!   and decodes it with the shared codec off the async threads, as many at
//!   once as uploads are verified.
//! - [`objects`] turns its objects into feature rows (`cad.rs`), each under
//!   its persistent id; the first one the server does not keep is refused
//!   by its place (`entities[i]`).
//! - [`insert_objects`] writes up to [`BATCH`] of them in one statement, each
//!   at version 1.

use kentos_contracts::{
    CommandEnvelope, DocumentSnapshotV2, PROJECT_IMPORT, PROJECT_IMPORT_VERSION, PROJECT_IMPORTED,
    ProjectDuplicated, ProjectImport, ProjectImported, ProjectPermission, ProjectStorage,
};
use kentos_postgres::{Scope, rescope};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::Blobs;
use crate::cad::{PROJECTION_VERSION, Stored, to_stored};
use crate::changes::{check_target, lock};
use crate::commands::input;
use crate::duplicate::Origin;
use crate::error::{AppError, AppResult};
use crate::files::{own_upload, upload_gone, verifying};
use crate::projects::{check_srid, check_tree, opened, storage_name, storage_of};
use crate::snapshot::DrawingMeta;
use crate::{idempotency, journal, listing};

/// Objects inserted by one statement.
pub(crate) const BATCH: usize = 1000;

/// A stored `.kcad` file (`what` names it in errors), checked against its
/// hash when it is given and decoded off the async threads.
pub(crate) async fn read(
    blobs: &Blobs,
    key: &str,
    sha256: Option<&str>,
    what: &str,
) -> AppResult<DocumentSnapshotV2> {
    let _turn = verifying()
        .acquire()
        .await
        .map_err(|e| AppError::Storage(std::io::Error::other(e)))?;
    let bytes = blobs.read(key).await?;
    if let Some(sha256) = sha256 {
        let got: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if got != sha256 {
            return Err(AppError::Storage(std::io::Error::other(format!(
                "{what} bozulmuş ({key}: SHA-256 {got}, beklenen {sha256})"
            ))));
        }
    }
    let what = what.to_string();
    tokio::task::spawn_blocking(move || kentos_kcad::decode(&bytes))
        .await
        .map_err(|e| AppError::Storage(std::io::Error::other(e)))?
        .map_err(|e| {
            AppError::Storage(std::io::Error::other(format!(
                "{what} okunamadı: {}",
                e.message
            )))
        })
}

/// The file's objects as feature rows, each under its persistent id.
pub(crate) fn objects(doc: &DocumentSnapshotV2) -> AppResult<Vec<(Uuid, Stored)>> {
    doc.uids
        .iter()
        .zip(&doc.entities)
        .enumerate()
        .map(|(i, (uid, e))| {
            to_stored(e, doc.settings.srid)
                .map(|s| (Uuid::from_bytes(uid.0), s))
                .map_err(|why| {
                    AppError::invalid_at(
                        format!("entities[{i}]"),
                        format!("Dosyanın {}. nesnesi içe aktarılamadı: {why}", i + 1),
                    )
                })
        })
        .collect()
}

/// Up to [`BATCH`] objects in one statement, each at version 1.
pub(crate) async fn insert_objects(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
    project: Uuid,
    srid: u32,
    actor: Uuid,
    rows: &[(Uuid, Stored)],
) -> AppResult<()> {
    let ids: Vec<Uuid> = rows.iter().map(|(id, _)| *id).collect();
    let layers: Vec<&str> = rows.iter().map(|(_, s)| s.layer_id.as_str()).collect();
    let kinds: Vec<&str> = rows.iter().map(|(_, s)| s.kind.as_str()).collect();
    let sources: Vec<&str> = rows.iter().map(|(_, s)| s.source_kind).collect();
    let geoms: Vec<Option<&[u8]>> = rows.iter().map(|(_, s)| s.geom.as_deref()).collect();
    let definitions: Vec<Option<&serde_json::Value>> = rows
        .iter()
        .map(|(_, s)| s.cad_definition.as_ref())
        .collect();
    let properties: Vec<&serde_json::Value> = rows.iter().map(|(_, s)| &s.properties).collect();
    let labels: Vec<Option<&str>> = rows.iter().map(|(_, s)| s.label.as_deref()).collect();
    let colors: Vec<Option<&str>> = rows.iter().map(|(_, s)| s.color.as_deref()).collect();
    let symbols: Vec<Option<&str>> = rows.iter().map(|(_, s)| s.symbol.as_deref()).collect();
    sqlx::query(
        "insert into kentos.feature (tenant_id, project_id, id, layer_id, kind, source_kind, srid, geom, cad_definition, properties,
                                     label, color, symbol, projection_version, created_by, updated_by, version)
         select $1, $2, f.id, f.layer_id, f.kind, f.source_kind, $3, public.st_geomfromewkb(f.geom), f.cad_definition, f.properties,
                f.label, f.color, f.symbol, $4, $5, $5, 1
           from unnest($6::uuid[], $7::text[], $8::text[], $9::text[], $10::bytea[], $11::jsonb[], $12::jsonb[], $13::text[], $14::text[], $15::text[])
             as f(id, layer_id, kind, source_kind, geom, cad_definition, properties, label, color, symbol)",
    )
    .bind(tenant)
    .bind(project)
    .bind(srid as i32)
    .bind(PROJECTION_VERSION)
    .bind(actor)
    .bind(&ids)
    .bind(&layers)
    .bind(&kinds)
    .bind(&sources)
    .bind(&geoms)
    .bind(&definitions)
    .bind(&properties)
    .bind(&labels)
    .bind(&colors)
    .bind(&symbols)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// `project.import` v1 (docs/adr/0036): the caller's own verified upload
/// into a database project that has nothing in it yet, in one transaction.
/// The project takes the file's settings, layers, active layer, origin,
/// view and styles; every object comes under its persistent id at version
/// 1 (data revision 1). The first object the server does not keep refuses
/// the whole file, by its place; nothing is written then. The upload goes
/// once its drawing is in.
pub async fn import(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectImported> {
    let ProjectImport { upload_id } = input(&envelope, PROJECT_IMPORT, PROJECT_IMPORT_VERSION)?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let upload = Uuid::parse_str(&upload_id).map_err(|_| {
        AppError::invalid_at(
            "uploadId",
            format!("uploadId bir yükleme kimliği (UUID) olmalı: {upload_id}"),
        )
    })?;
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    access.require(ProjectPermission::Edit)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    // The project is new and empty: holding its lock while the file is decoded keeps nobody waiting.
    let now = lock(&mut tx, access).await?;
    if let Some(mut earlier) =
        idempotency::earlier::<ProjectImported>(&mut tx, &now, &envelope, &text).await?
    {
        tx.commit().await?;
        earlier.replayed = true;
        return Ok(earlier);
    }
    now.writable()?;
    now.require(ProjectPermission::FeatureWrite)?;
    now.require(ProjectPermission::Edit)?;
    let (storage, data_revision, objects_now): (String, i64, i64) = sqlx::query_as(
        "select p.storage, p.data_revision,
                (select count(*) from kentos.feature f where f.tenant_id = p.tenant_id and f.project_id = p.id)
           from kentos.project p where p.tenant_id = $1 and p.id = $2",
    )
    .bind(now.tenant)
    .bind(now.project)
    .fetch_one(&mut *tx)
    .await?;
    if storage_of(&storage) == ProjectStorage::File {
        return Err(AppError::invalid(format!(
            "“{}” projesi dosya olarak saklanıyor; dosyası project.file.commit ile yeni revizyon olarak kaydedilir.",
            now.name
        )));
    }
    if data_revision != 0 || objects_now != 0 {
        return Err(AppError::invalid(format!(
            "“{}” projesinde kayıt var; içe aktarım yalnız yeni, boş bir projeye yapılır. Yeni bir proje açıp oraya aktarın.",
            now.name
        )));
    }
    let found = own_upload(&mut tx, &now, upload)
        .await?
        .ok_or_else(upload_gone)?;
    if !found.received {
        return Err(AppError::invalid_at(
            "uploadId",
            "Yüklemenin baytları henüz gelmedi; önce PUT …/uploads/{yükleme} ile gönderin.",
        ));
    }
    let doc = read(
        blobs,
        &found.blob_key,
        Some(&found.sha256),
        "yüklenen dosya",
    )
    .await?;
    let rows = objects(&doc)?;
    check_tree(&doc.layers, &doc.active_layer)?;
    check_srid(&mut tx, doc.settings.srid).await?;
    let value = |v: serde_json::Result<serde_json::Value>| {
        v.map_err(|e| AppError::invalid(format!("Yüklenen dosyanın kaydı yazılamadı: {e}")))
    };
    let (revision, meta): (i64, i64) = sqlx::query_as(
        "update kentos.project set settings = $3, layers = $4, active_layer = $5, origin_x = $6, origin_y = $7,
                home_view = $8, styles = $9, srid = $10, data_revision = 1, meta_version = meta_version + 1, updated_at = now()
          where tenant_id = $1 and id = $2 returning data_revision, meta_version",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(value(serde_json::to_value(&doc.settings))?)
    .bind(value(serde_json::to_value(&doc.layers))?)
    .bind(&doc.active_layer)
    .bind(doc.origin.x)
    .bind(doc.origin.y)
    .bind(doc.home_view.map(serde_json::to_value).transpose().map_err(|e| {
        AppError::invalid(format!("Yüklenen dosyanın kaydı yazılamadı: {e}"))
    })?)
    .bind(value(serde_json::to_value(&doc.styles))?)
    .bind(doc.settings.srid as i32)
    .fetch_one(&mut *tx)
    .await?;
    for batch in rows.chunks(BATCH) {
        insert_objects(
            &mut tx,
            now.tenant,
            now.project,
            doc.settings.srid,
            now.actor.user_id,
            batch,
        )
        .await?;
    }
    sqlx::query(
        "delete from kentos.project_upload where tenant_id = $1 and project_id = $2 and id = $3",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(upload)
    .execute(&mut *tx)
    .await?;
    let request = Some(envelope.request_id.as_str());
    journal::audit(
        &mut tx,
        &now,
        PROJECT_IMPORT,
        request,
        revision,
        json!({ "objects": rows.len(), "size": found.size, "sha256": found.sha256 }),
    )
    .await?;
    journal::event(&mut tx, &now, PROJECT_IMPORTED, request, revision, true).await?;
    let result = ProjectImported {
        objects: rows.len().to_string(),
        data_revision: revision.to_string(),
        meta_version: meta.to_string(),
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &result).await?;
    tx.commit().await?;
    // Its drawing is in the database now: the upload's bytes are nobody's.
    let _ = blobs.remove(&found.blob_key).await;
    Ok(result)
}

/// What a new project made from a drawing holds.
pub(crate) enum Content<'a> {
    /// A database project: these objects, each at version 1.
    Objects(&'a [(Uuid, Stored)]),
    /// A file project: its revision 1, an object already in the store.
    File {
        key: &'a str,
        size: i64,
        sha256: &'a str,
        objects: i64,
    },
}

/// A new project made from a drawing: where, its id and name, the drawing's
/// settings, layers and styles, what it holds, and where it comes from.
pub(crate) struct Drawing<'a> {
    pub target: Uuid,
    pub id: Uuid,
    pub name: &'a str,
    pub meta: &'a DrawingMeta,
    pub content: Content<'a>,
    pub origin: Origin,
}

/// The new project's rows, both projects' audit records and the stored
/// answer, in the open transaction under the source's lock: the project
/// restored from a checkpoint (docs/adr/0034) or converted into the other
/// storage mode (docs/adr/0039). It takes the source's catalog metadata
/// (description, type, tags), as a copy does; the caller owns it.
pub(crate) async fn create_from_drawing(
    tx: &mut Transaction<'static, Postgres>,
    now: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
    drawing: Drawing<'_>,
) -> AppResult<ProjectDuplicated> {
    let Drawing {
        target,
        id,
        name,
        meta,
        content,
        origin,
    } = drawing;
    let request = Some(envelope.request_id.as_str());
    let actor = now.actor.user_id;
    let objects = match &content {
        Content::Objects(rows) => i64::try_from(rows.len()).unwrap_or(i64::MAX),
        Content::File { objects, .. } => *objects,
    };
    let storage = match &content {
        Content::Objects(_) => ProjectStorage::Database,
        Content::File { .. } => ProjectStorage::File,
    };
    let (description, project_type, tags): (String, String, Vec<String>) = sqlx::query_as(
        "select description, project_type, tags from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(now.tenant)
    .bind(now.project)
    .fetch_one(&mut **tx)
    .await?;
    let source_revision = journal::revision(tx, now).await?;
    journal::audit(
        tx,
        now,
        origin.action(),
        request,
        source_revision,
        json!({ "copy": id, "tenant": target, "name": name, "objects": objects, "from": origin.detail() }),
    )
    .await?;
    // The new project's own rows, in its scope.
    rescope(
        tx,
        Scope {
            tenant: Some(target),
            user: Some(actor),
            project: Some(id),
        },
    )
    .await?;
    check_srid(tx, meta.settings.srid).await?;
    let value = |v: serde_json::Result<serde_json::Value>| {
        v.map_err(|e| AppError::invalid(format!("Çizimin kaydı yazılamadı: {e}")))
    };
    sqlx::query(
        "insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y, home_view, styles, created_by, owner_user_id,
                                     description, project_type, tags, storage)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12, $13, $14, $15, $16)",
    )
    .bind(target)
    .bind(id)
    .bind(name)
    .bind(meta.settings.srid as i32)
    .bind(value(serde_json::to_value(&meta.settings))?)
    .bind(value(serde_json::to_value(&meta.layers))?)
    .bind(&meta.active_layer)
    .bind(meta.origin.x)
    .bind(meta.origin.y)
    .bind(meta.home_view.map(serde_json::to_value).transpose().map_err(|e| {
        AppError::invalid(format!("Çizimin kaydı yazılamadı: {e}"))
    })?)
    .bind(value(serde_json::to_value(&meta.styles))?)
    .bind(actor)
    .bind(&description)
    .bind(&project_type)
    .bind(&tags)
    .bind(storage_name(storage))
    .execute(&mut **tx)
    .await?;
    let written = match content {
        Content::Objects(rows) => {
            for batch in rows.chunks(BATCH) {
                insert_objects(tx, target, id, meta.settings.srid, actor, batch).await?;
            }
            !rows.is_empty()
        }
        Content::File {
            key,
            size,
            sha256,
            objects,
        } => {
            sqlx::query(
                "insert into kentos.project_file_revision (tenant_id, project_id, revision, size, sha256, blob_key, created_by, request_id, objects)
                 values ($1, $2, 1, $3, $4, $5, $6, $7, $8)",
            )
            .bind(target)
            .bind(id)
            .bind(size)
            .bind(sha256)
            .bind(key)
            .bind(actor)
            .bind(&envelope.request_id)
            .bind(objects)
            .execute(&mut **tx)
            .await?;
            sqlx::query(
                "update kentos.project set file_revision = 1 where tenant_id = $1 and id = $2",
            )
            .bind(target)
            .bind(id)
            .execute(&mut **tx)
            .await?;
            true
        }
    };
    if written {
        // A created object's version is its commit's data revision (docs/adr/0026): this is the first.
        sqlx::query("update kentos.project set data_revision = 1 where tenant_id = $1 and id = $2")
            .bind(target)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    let subject = journal::Subject {
        tenant: target,
        project: id,
        actor,
    };
    journal::audit(
        tx,
        subject,
        "project.create",
        request,
        i64::from(written),
        json!({ "name": name, "copyOf": { "tenant": now.tenant, "project": now.project }, "from": origin.detail(),
                "storage": storage_name(storage), "objects": objects }),
    )
    .await?;
    opened(tx, target, id, actor).await?;
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
