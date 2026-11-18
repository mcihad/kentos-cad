//! An upload of the account's own (docs/adr/9000 §13): one way to send a file of any size from the desktop, a phone
//! or an agent, before a dossier or a project takes it.
//!
//! - `POST /v1/uploads` declares the file's name, size and SHA-256; nothing is written yet.
//! - `PUT /v1/uploads/{id}?offset=N` carries one part of at most [`PART_MAX`] bytes. Under the upload's row lock its
//!   offset must be the bytes that arrived so far, so two parts never interleave and a part cut short never reaches
//!   the object; the next request goes on from `receivedBytes`.
//! - The last part checks the size and the hash from the disk, then the scan (scan.rs). A refused file is removed with
//!   its upload, and the platform's event log says who sent what; a passed one is `received`.
//! - A destination takes a received upload in its own transaction ([`take`]): the row goes, the object moves to the
//!   destination's key. Nobody else sees an upload; one nobody takes goes after [`LIFETIME_HOURS`].
//! - An account keeps at most [`OPEN_MAX`] uploads at once, [`UPLOAD_MAX`] bytes each.

use kentos_contracts::{ScanState, UploadBegin, UploadView};
use kentos_postgres::Db;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::portal::accounts::user_scope;
use crate::projects::rfc3339;
use crate::scan::{Refusal, Scanned, Scanner};

/// The largest file one upload carries.
pub const UPLOAD_MAX: u64 = 512 * 1024 * 1024;
/// The largest part one request carries.
pub const PART_MAX: u64 = 32 * 1024 * 1024;
/// The uploads an account keeps open at once.
pub const OPEN_MAX: i64 = 20;
/// How long an upload waits for a destination.
pub const LIFETIME_HOURS: i64 = 24;

/// Scans one file at a time per two: an archive is read whole and a daemon scan takes a while.
fn scanning() -> &'static Semaphore {
    static TURNS: std::sync::OnceLock<Semaphore> = std::sync::OnceLock::new();
    TURNS.get_or_init(|| Semaphore::new(2))
}

type Row = (
    Uuid,
    String,
    String,
    i64,
    String,
    String,
    OffsetDateTime,
    Option<OffsetDateTime>,
    Option<String>,
    Option<String>,
);

const SELECT: &str =
    "select id, name, media_type, size, sha256, blob_key, created_at, received_at, scan, scan_note from kentos.upload";

fn view_of(row: &Row, arrived: u64) -> UploadView {
    let (id, name, media_type, size, sha256, _, created, received, scan, scan_note) = row;
    UploadView {
        id: id.to_string(),
        name: name.clone(),
        media_type: media_type.clone(),
        size: u64::try_from(*size).unwrap_or(0),
        sha256: sha256.clone(),
        received_bytes: arrived,
        received: received.is_some(),
        scan: scan.as_deref().map(ScanState::of),
        scan_note: scan_note.clone(),
        part_max: PART_MAX,
        created_at: rfc3339(*created),
        expires_at: rfc3339(*created + time::Duration::hours(LIFETIME_HOURS)),
    }
}

pub fn key_of(actor: &Actor, id: Uuid) -> String {
    format!("uploads/accounts/{}/{id}", actor.user_id)
}

fn gone() -> AppError {
    AppError::not_found(
        "Böyle bir yükleme yok: süresi dolmuş, başka bir yere eklenmiş ya da iptal edilmiş olabilir. Yeni bir yükleme başlatın.",
    )
}

fn sha256_of(text: &str) -> AppResult<String> {
    let t = text.trim().to_lowercase();
    if t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(t)
    } else {
        Err(AppError::invalid_at("sha256", "SHA-256 64 onaltılık rakam olmalı."))
    }
}

/// `POST /v1/uploads`.
pub async fn begin(db: &Db, actor: &Actor, input: UploadBegin) -> AppResult<UploadView> {
    let name = crate::portal::files::file_name(&input.name)?;
    if input.size == 0 || input.size > UPLOAD_MAX {
        return Err(AppError::invalid_at(
            "size",
            format!("Dosya 1 bayt ile {} MiB arasında olmalı.", UPLOAD_MAX / (1024 * 1024)),
        ));
    }
    let sha256 = sha256_of(&input.sha256)?;
    let media_type = crate::scan::media_type_of(&name);
    let id = Uuid::now_v7();
    let mut tx = db.scoped(user_scope(actor)).await?;
    let open: i64 = sqlx::query_scalar("select count(*) from kentos.upload where user_id = $1")
        .bind(actor.user_id)
        .fetch_one(&mut *tx)
        .await?;
    if open >= OPEN_MAX {
        return Err(AppError::Quota(format!(
            "Aynı anda en çok {OPEN_MAX} yükleme açık olabilir; bitmemiş olanları tamamlayın ya da iptal edin."
        )));
    }
    let row: Row = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with made as (
           insert into kentos.upload (id, user_id, name, media_type, size, sha256, blob_key)
           values ($1, $2, $3, $4, $5, $6, $7) returning *)
         {}",
        SELECT.replace("from kentos.upload", "from made")
    )))
    .bind(id)
    .bind(actor.user_id)
    .bind(&name)
    .bind(media_type)
    .bind(i64::try_from(input.size).unwrap_or(i64::MAX))
    .bind(&sha256)
    .bind(key_of(actor, id))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(view_of(&row, 0))
}

async fn own(tx: &mut Transaction<'static, Postgres>, actor: &Actor, id: Uuid, lock: bool) -> AppResult<Row> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} where id = $1 and user_id = $2{}",
        if lock { " for update" } else { "" }
    )))
    .bind(id)
    .bind(actor.user_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(gone)
}

/// `GET /v1/uploads/{id}`: how far it came.
pub async fn view(db: &Db, blobs: &Blobs, actor: &Actor, id: Uuid) -> AppResult<UploadView> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let row = own(&mut tx, actor, id, false).await?;
    tx.commit().await?;
    let arrived = blobs.len(&row.5).await?;
    Ok(view_of(&row, arrived))
}

/// `DELETE /v1/uploads/{id}`.
pub async fn cancel(db: &Db, blobs: &Blobs, actor: &Actor, id: Uuid) -> AppResult<()> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let key: Option<String> =
        sqlx::query_scalar("delete from kentos.upload where id = $1 and user_id = $2 returning blob_key")
            .bind(id)
            .bind(actor.user_id)
            .fetch_optional(&mut *tx)
            .await?;
    tx.commit().await?;
    let key = key.ok_or_else(gone)?;
    let _ = blobs.remove(&key).await;
    Ok(())
}

/// `PUT /v1/uploads/{id}?offset=N`: one part (see the module comment).
pub async fn receive(
    db: &Db,
    blobs: &Blobs,
    scanner: &Scanner,
    actor: &Actor,
    id: Uuid,
    offset: u64,
    bytes: &[u8],
) -> AppResult<UploadView> {
    if bytes.is_empty() || bytes.len() as u64 > PART_MAX {
        return Err(AppError::invalid_at(
            "size",
            format!(
                "Bir parça 1 bayt ile {} MiB arasında olmalı ({} bayt geldi).",
                PART_MAX / (1024 * 1024),
                bytes.len()
            ),
        ));
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    let row = own(&mut tx, actor, id, true).await?;
    if row.7.is_some() {
        return Err(AppError::invalid(
            "Bu yüklemenin bütün baytları zaten geldi; bir dosyaya ya da projeye ekleyin.",
        ));
    }
    let size = u64::try_from(row.3).unwrap_or(0);
    let arrived = blobs.len(&row.5).await?;
    if offset != arrived {
        return Err(AppError::invalid_at(
            "offset",
            format!("Yüklemenin {arrived} baytı geldi; sonraki parça {arrived}. bayttan başlamalı ({offset} değil)."),
        ));
    }
    if arrived + bytes.len() as u64 > size {
        return Err(AppError::invalid_at(
            "size",
            format!(
                "Bu parçayla bildirilenden ({size} bayt) fazla bayt gelirdi; parça alınmadı. Doğru boyutla yeni bir yükleme başlatın."
            ),
        ));
    }
    let now = blobs.append(&row.5, bytes).await?;
    tx.commit().await?;
    if now < size {
        return Ok(view_of(&row, now));
    }
    // The last part: the whole file is checked and scanned, outside the row's lock.
    let got = blobs.sha256(&row.5).await?;
    if got != row.4 {
        discard(db, blobs, actor, id, &row.5).await;
        return Err(AppError::invalid_at(
            "sha256",
            format!(
                "Gelen dosya bildirilenle aynı değil (SHA-256 {got}, beklenen {}). Hiçbir şey saklanmadı; özeti yeniden hesaplayıp yeni bir yükleme başlatın.",
                row.4
            ),
        ));
    }
    let scanned = {
        let _turn = scanning()
            .acquire()
            .await
            .map_err(|e| AppError::Storage(std::io::Error::other(e)))?;
        scanner.check(blobs, &row.5, &row.1).await
    };
    let scanned = match scanned {
        Ok(Ok(s)) => s,
        Ok(Err(refusal)) => {
            discard(db, blobs, actor, id, &row.5).await;
            return Err(refused(db, actor, &row.1, &refusal, "upload").await);
        }
        Err(e) => {
            discard(db, blobs, actor, id, &row.5).await;
            return Err(e);
        }
    };
    let mut tx = db.scoped(user_scope(actor)).await?;
    let marked: Option<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with marked as (
           update kentos.upload set received_at = now(), scan = $3, scan_note = $4, media_type = $5
            where id = $1 and user_id = $2 and received_at is null returning *)
         {}",
        SELECT.replace("from kentos.upload", "from marked")
    )))
    .bind(id)
    .bind(actor.user_id)
    .bind(scanned.state.code())
    .bind(scanned.note.as_deref())
    .bind(&scanned.media_type)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    match marked {
        Some(row) => Ok(view_of(&row, size)),
        None => {
            // Cancelled or expired meanwhile: the bytes belong to nothing.
            let _ = blobs.remove(&row.5).await;
            Err(gone())
        }
    }
}

/// Removes an upload whose file was refused: its object and its row.
async fn discard(db: &Db, blobs: &Blobs, actor: &Actor, id: Uuid, key: &str) {
    let _ = blobs.remove(key).await;
    if let Ok(mut tx) = db.scoped(user_scope(actor)).await {
        let _ = sqlx::query("delete from kentos.upload where id = $1 and user_id = $2")
            .bind(id)
            .bind(actor.user_id)
            .execute(&mut *tx)
            .await;
        let _ = tx.commit().await;
    }
}

/// A received upload a destination takes, in the destination's transaction (its row goes with it).
pub struct Taken {
    pub key: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub scanned: Scanned,
}

/// Takes the caller's received upload `id` in `tx`; its object is still at `key` until the destination moves it.
pub async fn take(tx: &mut Transaction<'static, Postgres>, actor: &Actor, id: &str) -> AppResult<Taken> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| gone().at("uploadId"))?;
    let row: Row = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with taken as (delete from kentos.upload where id = $1 and user_id = $2 returning *) {}",
        SELECT.replace("from kentos.upload", "from taken")
    )))
    .bind(id)
    .bind(actor.user_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| gone().at("uploadId"))?;
    let (_, name, media_type, size, sha256, key, _, received, scan, scan_note) = row;
    if received.is_none() {
        return Err(AppError::invalid_at(
            "uploadId",
            "Yüklemenin bütün baytları henüz gelmedi; önce yüklemeyi tamamlayın.",
        ));
    }
    Ok(Taken {
        key,
        name,
        size: u64::try_from(size).unwrap_or(0),
        sha256,
        scanned: Scanned {
            state: ScanState::of(scan.as_deref().unwrap_or("unscanned")),
            note: scan_note,
            media_type,
        },
    })
}

/// A refusal written to the platform's event log as the sender's; what the sender is told.
pub async fn refused(db: &Db, actor: &Actor, name: &str, refusal: &Refusal, place: &str) -> AppError {
    let reason = match refusal {
        Refusal::Blocked(why) | Refusal::Infected(why) => why.clone(),
    };
    if let Ok(mut tx) = db.scoped(user_scope(actor)).await {
        let logged = sqlx::query("select kentos.log_refused_file($1, $2)")
            .bind(name)
            .bind(serde_json::json!({ "verdict": refusal.code(), "reason": reason, "place": place }))
            .execute(&mut *tx)
            .await;
        if logged.is_ok() {
            let _ = tx.commit().await;
        }
    }
    AppError::invalid_at("file", refusal.message(name))
}

/// Removes the uploads nobody took within their lifetime, rows and objects; how many went.
pub async fn expire(db: &Db, blobs: &Blobs) -> AppResult<usize> {
    const BATCH: i32 = 200;
    let mut gone = 0;
    loop {
        let keys: Vec<String> = sqlx::query_scalar(
            "select kentos.expire_user_uploads(now() - make_interval(hours => $1), $2)",
        )
        .bind(i32::try_from(LIFETIME_HOURS).unwrap_or(24))
        .bind(BATCH)
        .fetch_all(&db.pool)
        .await?;
        for key in &keys {
            let _ = blobs.remove(key).await;
        }
        gone += keys.len();
        if keys.len() < BATCH as usize {
            return Ok(gone);
        }
    }
}
