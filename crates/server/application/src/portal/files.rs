//! A dossier's files (docs/adr/9000 §7). A file goes into the package being prepared (by its side, while it prepares
//! it) or is attached to a message; until it is sent it may be removed, and only its own side sees it. Its bytes are
//! kept in the object store under the dossier, never changed, and checked against their SHA-256 each time they are
//! read.
//!
//! A cloud project goes in as its present state: a file project's newest revision (the same object, shared), a
//! database project's KCAD file written now. The other way round, a KCAD file of a dossier becomes a new file project
//! of the caller (in their personal space, or the organisation they work in): that is how a transferred project reaches
//! a professional's workspace and a delivered one the municipality's.

use kentos_contracts::{
    DossierFileImport, DossierFileImported, DossierFileView, DossierSide, FileSource, FromProject,
    ProjectPermission, ProjectStorage,
};
use kentos_postgres::{Db, Scope};
use sqlx::types::Json;
use uuid::Uuid;

use super::accounts::user_scope;
use super::dossiers::{hold, record};
use super::flow;
use super::rows::{FILE_SELECT, FileRow, is_project_file, side_code};
use super::{catalog, id_at};
use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::snapshot::DrawingMeta;
use crate::tenancy::{self, Capability};

/// The largest file a dossier takes.
pub const MAX_FILE_BYTES: u64 = 200 * 1024 * 1024;

/// The media type of a KentOS project file.
pub const KCAD_MEDIA_TYPE: &str = "application/vnd.kentos.kcad";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// The package being prepared.
    Package,
    /// A message's attachment, sent with it.
    Attachment,
}

impl Purpose {
    pub fn of(text: &str) -> AppResult<Self> {
        match text {
            "package" | "" => Ok(Purpose::Package),
            "attachment" => Ok(Purpose::Attachment),
            other => Err(AppError::invalid_at(
                "purpose",
                format!("“{other}”: package ya da attachment olmalı."),
            )),
        }
    }

    fn code(self) -> &'static str {
        match self {
            Purpose::Package => "package",
            Purpose::Attachment => "attachment",
        }
    }
}

/// Where a file's bytes go before its row is written, and for whom.
#[derive(Clone, Debug)]
pub struct Prepared {
    pub dossier: Uuid,
    pub file: Uuid,
    pub key: String,
    purpose: Purpose,
}

/// What the person may not add now, said.
fn not_now(purpose: Purpose) -> AppError {
    AppError::forbidden(match purpose {
        Purpose::Package => {
            "Pakete şimdi dosya eklenmez: paketi gönderecek taraf, gönderene kadar ekler."
        }
        Purpose::Attachment => "Bu dosyaya şimdi ileti eki eklenmez.",
    })
}

async fn allowed(
    tx: &mut sqlx::Transaction<'static, sqlx::Postgres>,
    actor: &Actor,
    id: Uuid,
    purpose: Purpose,
    lock: bool,
) -> AppResult<(DossierSide, u32)> {
    let held = hold(tx, actor, id, lock).await?;
    let (direction, status) = (held.direction(), held.status());
    let ok = held.may_act()
        && match purpose {
            Purpose::Package => flow::can_add_files(direction, status, held.party),
            Purpose::Attachment => flow::can_message(status, held.party),
        };
    if !ok {
        return Err(not_now(purpose));
    }
    Ok((held.party.side(), held.row.round() + 1))
}

/// Whether `actor` may add a file to the dossier now; where its bytes go.
pub async fn prepare(db: &Db, actor: &Actor, id: Uuid, purpose: Purpose) -> AppResult<Prepared> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    allowed(&mut tx, actor, id, purpose, false).await?;
    tx.commit().await?;
    let file = Uuid::now_v7();
    Ok(Prepared {
        dossier: id,
        file,
        key: Blobs::dossier_key(id, file),
        purpose,
    })
}

/// A file's name as the person gave it, without a folder; refused when nothing is left of it.
pub fn file_name(name: &str) -> AppResult<String> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default().trim();
    let clean: String = base.chars().filter(|c| !c.is_control()).collect();
    let n = clean.chars().count();
    if n == 0 || clean == "." || clean == ".." {
        return Err(AppError::invalid_at("name", "Dosyanın adı yok."));
    }
    if n > 200 {
        return Err(AppError::invalid_at(
            "name",
            "Dosya adı en çok 200 karakter olabilir.",
        ));
    }
    Ok(clean)
}

/// The bytes the store holds for `prepared`, recorded as a pending file, checking the dossier again under its lock.
#[allow(clippy::too_many_arguments)]
pub async fn record_file(
    db: &Db,
    actor: &Actor,
    prepared: &Prepared,
    name: &str,
    media_type: &str,
    (size, sha256): (u64, &str),
    doc_type: Option<&str>,
    source: Option<FileSource>,
    scanned: Option<&crate::scan::Scanned>,
) -> AppResult<DossierFileView> {
    let name = file_name(name)?;
    let doc_type = match doc_type.map(str::trim).filter(|d| !d.is_empty()) {
        Some(d) if catalog::is_doc_type(d) => d.to_owned(),
        Some(d) => {
            return Err(AppError::invalid_at(
                "docType",
                format!("“{d}” bir belge türü değil."),
            ));
        }
        None if is_project_file(&name, "") => "kentos_project".to_owned(),
        None => "other".to_owned(),
    };
    let media_type = if is_project_file(&name, &doc_type) {
        KCAD_MEDIA_TYPE.to_owned()
    } else if let Some(s) = scanned {
        s.media_type.clone()
    } else {
        let m = media_type.trim();
        if (3..=120).contains(&m.len()) && m.contains('/') && m.is_ascii() {
            m.to_owned()
        } else {
            "application/octet-stream".to_owned()
        }
    };
    let mut tx = db.scoped(user_scope(actor)).await?;
    let (side, pending_round) =
        allowed(&mut tx, actor, prepared.dossier, prepared.purpose, true).await?;
    let round = (prepared.purpose == Purpose::Package)
        .then(|| i32::try_from(pending_round).unwrap_or(i32::MAX));
    let row: FileRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with added as (
           insert into kentos.dossier_file (id, dossier_id, side, purpose, round, state, doc_type, name, media_type,
                                            size, sha256, blob_key, source, added_by, added_by_name, scan, scan_note)
           values ($1, $2, $3, $4, $5, 'pending', $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
           returning *)
         {}",
        FILE_SELECT.replace("from kentos.dossier_file f", "from added f")
    )))
    .bind(prepared.file)
    .bind(prepared.dossier)
    .bind(side_code(side))
    .bind(prepared.purpose.code())
    .bind(round)
    .bind(&doc_type)
    .bind(&name)
    .bind(&media_type)
    .bind(i64::try_from(size).unwrap_or(i64::MAX))
    .bind(sha256)
    .bind(&prepared.key)
    .bind(source.map(Json))
    .bind(actor.user_id)
    .bind(&actor.display_name)
    .bind(scanned.map_or("unscanned", |s| s.state.code()))
    .bind(scanned.and_then(|s| s.note.as_deref()))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row.view())
}

/// `POST /v1/dossiers/{id}/files/from-upload`: the caller's received upload (uploads.rs) as a pending file. The upload
/// is given up only when the file is recorded; a refusal leaves it to try again.
pub async fn from_upload(
    db: &Db,
    blobs: &Blobs,
    actor: &Actor,
    id: Uuid,
    input: &kentos_contracts::DossierFileFromUpload,
) -> AppResult<DossierFileView> {
    let purpose = Purpose::of(&input.purpose)?;
    let prepared = prepare(db, actor, id, purpose).await?;
    let mut own = db.scoped(user_scope(actor)).await?;
    let taken = crate::uploads::take(&mut own, actor, &input.upload_id).await?;
    if taken.size > MAX_FILE_BYTES {
        return Err(AppError::invalid_at(
            "uploadId",
            format!("Dosyaya eklenen bir belge en çok {} MB olabilir.", MAX_FILE_BYTES / (1024 * 1024)),
        ));
    }
    blobs.promote(&taken.key, &prepared.key).await?;
    let recorded = record_file(
        db,
        actor,
        &prepared,
        &taken.name,
        &taken.scanned.media_type,
        (taken.size, &taken.sha256),
        input.doc_type.as_deref(),
        None,
        Some(&taken.scanned),
    )
    .await;
    match recorded {
        Ok(view) => {
            if let Err(e) = own.commit().await {
                // The file stands; the upload's row goes with the day's cleanup.
                eprintln!("⚠ dosyaya eklenen yüklemenin kaydı kapatılamadı: {e}");
            }
            Ok(view)
        }
        Err(e) => {
            let _ = blobs.promote(&prepared.key, &taken.key).await;
            Err(e)
        }
    }
}

/// Removes the objects of `keys`, as far as the store lets it; a leftover object of no row is harmless.
pub(crate) async fn remove_all(blobs: &Blobs, keys: &[String]) {
    for key in keys {
        if let Err(e) = blobs.remove(key).await {
            eprintln!("⚠ dosyanın nesnesi silinemedi ({key}): {e}");
        }
    }
}

/// `POST /v1/dossiers/{id}/files/from-project`: a cloud project's present state as a file of the package.
pub async fn from_project(
    db: &Db,
    blobs: &Blobs,
    actor: &Actor,
    id: Uuid,
    input: &FromProject,
) -> AppResult<DossierFileView> {
    let tenant = id_at("tenantId", &input.tenant_id)?;
    let project = id_at("projectId", &input.project_id)?;
    let prepared = prepare(db, actor, id, Purpose::Package).await?;
    let access = crate::access::project(db, actor, tenant, project).await?;
    access.live()?;
    access.require(ProjectPermission::Download)?;
    let mut tx = db.scoped(access.scope()).await?;
    let storage: String =
        sqlx::query_scalar("select storage from kentos.project where tenant_id = $1 and id = $2")
            .bind(tenant)
            .bind(project)
            .fetch_one(&mut *tx)
            .await?;
    let (size, sha256, revision) = if crate::projects::storage_name(ProjectStorage::File) == storage
    {
        crate::projects::require_whole_copy(&mut tx, &access).await?;
        let newest = crate::files::newest(&mut tx, tenant, project).await?;
        tx.commit().await?;
        let row = newest.ok_or_else(|| {
            AppError::invalid_at(
                "projectId",
                format!(
                    "“{}” projesinin henüz kaydedilmiş bir dosyası yok.",
                    access.name
                ),
            )
        })?;
        blobs.share(&row.7, &prepared.key).await?;
        (u64::try_from(row.1).unwrap_or(0), row.2, row.0)
    } else {
        tx.commit().await?;
        let snap = crate::snapshot::snapshot(db, &access).await?;
        let mut writer = blobs.create(&prepared.key, snap.bytes.len() as u64).await?;
        writer
            .write(&snap.bytes)
            .await
            .map_err(|e| AppError::Storage(std::io::Error::other(format!("{e:?}"))))?;
        let (size, sha) = writer.finish().await?;
        (size, sha, snap.revision)
    };
    let source = FileSource {
        tenant_id: tenant.to_string(),
        project_id: project.to_string(),
        project_name: access.name.clone(),
        revision: revision.to_string(),
    };
    let name: String = format!("{}.kcad", access.name.replace(['/', '\\'], "-"));
    let recorded = record_file(
        db,
        actor,
        &prepared,
        &name,
        KCAD_MEDIA_TYPE,
        (size, &sha256),
        Some(input.doc_type.as_deref().unwrap_or("kentos_project")),
        Some(source),
        None,
    )
    .await;
    if recorded.is_err() {
        remove_all(blobs, std::slice::from_ref(&prepared.key)).await;
    }
    recorded
}

/// `DELETE /v1/dossiers/{id}/files/{file}`: a pending file, by its side.
pub async fn remove(db: &Db, blobs: &Blobs, actor: &Actor, id: Uuid, file: Uuid) -> AppResult<()> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, id, true).await?;
    let found: Option<(String, String, String)> = sqlx::query_as(
        "select blob_key, purpose, side from kentos.dossier_file where id = $1 and dossier_id = $2 and state = 'pending'",
    )
    .bind(file)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((key, purpose, side)) = found else {
        return Err(AppError::not_found(
            "Dosya bulunamadı ya da gönderilmiş; gönderilen dosya kaldırılmaz.",
        ));
    };
    let mine = side == side_code(held.party.side());
    let may = match purpose.as_str() {
        "package" => flow::can_add_files(held.direction(), held.status(), held.party),
        _ => true,
    };
    if !mine || !may {
        return Err(AppError::forbidden("Bu dosyayı kaldıramazsınız."));
    }
    sqlx::query("delete from kentos.dossier_file where id = $1")
        .bind(file)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    remove_all(blobs, &[key]).await;
    Ok(())
}

/// A file of the dossier as the caller may see it, its bytes checked against its record.
async fn checked(
    db: &Db,
    blobs: &Blobs,
    actor: &Actor,
    id: Uuid,
    file: Uuid,
) -> AppResult<FileRow> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    hold(&mut tx, actor, id, false).await?;
    let row: Option<FileRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{FILE_SELECT} where f.id = $1 and f.dossier_id = $2"
    )))
    .bind(file)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let row = row.ok_or_else(|| AppError::not_found("Dosya bulunamadı."))?;
    let sha = blobs.sha256(&row.blob_key).await?;
    if sha != row.sha256 {
        return Err(AppError::Storage(std::io::Error::other(format!(
            "“{}” dosyasının baytları kaydındaki özetle tutmuyor; dosya bozulmuş. Sistem yöneticisine bildirin.",
            row.name
        ))));
    }
    Ok(row)
}

/// `GET /v1/dossiers/{id}/files/{file}`: its record and its bytes.
pub async fn download(
    db: &Db,
    blobs: &Blobs,
    actor: &Actor,
    id: Uuid,
    file: Uuid,
) -> AppResult<(DossierFileView, tokio::fs::File)> {
    let row = checked(db, blobs, actor, id, file).await?;
    let opened = blobs.open(&row.blob_key).await?;
    Ok((row.view(), opened))
}

/// The project's name: the file's without `.kcad`, with the dossier's number, within 200 characters.
fn project_name(file: &str, number: Option<&str>) -> String {
    let stem = file
        .strip_suffix(".kcad")
        .or_else(|| file.strip_suffix(".KCAD"))
        .unwrap_or(file)
        .trim();
    // A file already named after another dossier ("Konut (2026-0001)", as a project taken out of one is) keeps one
    // number: this dossier's.
    let stem = match stem.rsplit_once(" (") {
        Some((head, tail)) if tail.ends_with(')') && is_number(&tail[..tail.len() - 1]) => {
            head.trim_end()
        }
        _ => stem,
    };
    let named = match number {
        Some(n) => format!("{stem} ({n})"),
        None => stem.to_owned(),
    };
    named.chars().take(200).collect()
}

/// `POST /v1/dossiers/{id}/files/{file}/import`: a KCAD file of the dossier as a new file project of the caller.
pub async fn import(
    db: &Db,
    blobs: &Blobs,
    actor: &Actor,
    id: Uuid,
    file: Uuid,
    input: &DossierFileImport,
) -> AppResult<DossierFileImported> {
    let row = checked(db, blobs, actor, id, file).await?;
    if !is_project_file(&row.name, &row.doc_type) {
        return Err(AppError::invalid(
            "Yalnız KentOS proje dosyası (KCAD) proje olarak açılır.",
        ));
    }
    let number = {
        let mut tx = db.scoped(user_scope(actor)).await?;
        let held = hold(&mut tx, actor, id, false).await?;
        tx.commit().await?;
        held.row.number()
    };
    let tenant = match input.tenant_id.as_deref() {
        Some(t) => id_at("tenantId", t)?,
        None => tenancy::ensure_personal(db, actor).await?,
    };
    let access = tenancy::access(db, actor, tenant).await?;
    access.require(Capability::ProjectCreate)?;
    let bytes = blobs.read(&row.blob_key).await?;
    let decoded = tokio::task::spawn_blocking(move || {
        kentos_kcad::read(&bytes).map(|(_, doc)| {
            (
                DrawingMeta::of(&doc),
                i64::try_from(doc.entities.len()).unwrap_or(i64::MAX),
            )
        })
    })
    .await
    .map_err(|e| AppError::Storage(std::io::Error::other(e)))?;
    let (meta, objects) = decoded.map_err(|e| {
        AppError::invalid(format!(
            "“{}” geçerli bir KCAD dosyası değil: {e}. Gönderen taraftan dosyayı KentOS ile yeniden kaydetmesini isteyin.",
            row.name
        ))
    })?;
    let project = Uuid::now_v7();
    let name = project_name(&row.name, number.as_deref());
    let key = Blobs::revision_key(tenant, project, 1, &row.sha256);
    blobs.share(&row.blob_key, &key).await?;
    let created = create_file_project(
        db,
        &access,
        project,
        &name,
        &meta,
        (&key, row.size, &row.sha256, objects),
        id,
    )
    .await;
    if let Err(e) = created {
        remove_all(blobs, &[key]).await;
        return Err(e);
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, id, false).await?;
    record(
        &mut tx,
        id,
        actor,
        held.party.side(),
        "imported",
        held.row.round(),
        serde_json::json!({ "file": row.name, "project": project }),
    )
    .await?;
    tx.commit().await?;
    Ok(DossierFileImported {
        tenant_id: tenant.to_string(),
        project_id: project.to_string(),
        name,
    })
}

/// The new file project's rows in `access`'s organisation: the drawing's settings, revision 1 (the object shared under
/// its key), its creation recorded with the dossier it came from, and its owner's recent use.
async fn create_file_project(
    db: &Db,
    access: &tenancy::Access,
    project: Uuid,
    name: &str,
    meta: &DrawingMeta,
    (key, size, sha256, objects): (&str, i64, &str, i64),
    dossier: Uuid,
) -> AppResult<()> {
    let actor = access.actor.user_id;
    let mut tx = db
        .scoped(Scope {
            tenant: Some(access.tenant),
            user: Some(actor),
            project: Some(project),
        })
        .await?;
    crate::projects::check_srid(&mut tx, meta.settings.srid).await?;
    let value = |v: serde_json::Result<serde_json::Value>| {
        v.map_err(|e| AppError::invalid(format!("Çizimin kaydı yazılamadı: {e}")))
    };
    sqlx::query(
        "insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y,
                                     home_view, styles, created_by, owner_user_id, description, storage)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12, $13, 'file')",
    )
    .bind(access.tenant)
    .bind(project)
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
    .bind("Portaldaki bir dosyadan açıldı.")
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into kentos.project_file_revision (tenant_id, project_id, revision, size, sha256, blob_key, created_by,
                                                   request_id, objects)
         values ($1, $2, 1, $3, $4, $5, $6, $7, $8)",
    )
    .bind(access.tenant)
    .bind(project)
    .bind(size)
    .bind(sha256)
    .bind(key)
    .bind(actor)
    .bind(format!("dossier:{dossier}"))
    .bind(objects)
    .execute(&mut *tx)
    .await?;
    sqlx::query("update kentos.project set file_revision = 1, data_revision = 1 where tenant_id = $1 and id = $2")
        .bind(access.tenant)
        .bind(project)
        .execute(&mut *tx)
        .await?;
    crate::journal::audit(
        &mut tx,
        crate::journal::Subject {
            tenant: access.tenant,
            project,
            actor,
        },
        "project.create",
        None,
        1,
        serde_json::json!({ "name": name, "fromDossier": dossier, "storage": "file", "objects": objects }),
    )
    .await?;
    crate::projects::opened(&mut tx, access.tenant, project, actor).await?;
    tx.commit().await?;
    Ok(())
}

/// "2026/0007" or "2026-0007": a dossier's number as a name carries it.
fn is_number(text: &str) -> bool {
    let mut parts = text.split(['/', '-']);
    matches!((parts.next(), parts.next(), parts.next()), (Some(y), Some(n), None)
        if y.len() == 4 && !n.is_empty() && y.bytes().chain(n.bytes()).all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_files_name_keeps_no_folder_and_a_project_takes_the_dossiers_number() {
        assert_eq!(
            project_name("Konut-412-7 (2026-0001).kcad", Some("2026/0006")),
            "Konut-412-7 (2026/0006)"
        );
        assert_eq!(
            project_name("Konut (taslak).kcad", Some("2026/0006")),
            "Konut (taslak) (2026/0006)"
        );
        assert_eq!(
            file_name("C:\\Users\\a\\Ada 412.kcad").unwrap(),
            "Ada 412.kcad"
        );
        assert_eq!(file_name("../../etc/passwd").unwrap(), "passwd");
        assert!(file_name("dir/").is_err() && file_name("..").is_err());
        assert_eq!(
            project_name("Ada 412.kcad", Some("2026/0007")),
            "Ada 412 (2026/0007)"
        );
        assert_eq!(project_name("Ada 412.kcad", None), "Ada 412");
    }
}
