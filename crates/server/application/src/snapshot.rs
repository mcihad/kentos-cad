//! A database project as one KCAD v2 file (docs/adr/0033; TODOS.md SYNC-05,
//! PG-15): what a client that opens the project would get, as one file.
//!
//! - **One moment:** the settings, layers, styles and objects are read in
//!   one repeatable-read, read-only transaction (`Db::snapshot`), so they
//!   are all of one data revision however many commits run meanwhile. No
//!   lock is taken: writers never wait for a snapshot.
//! - **The file:** the shared codec writes it and reads it back before it is
//!   handed out (`kentos_kcad::encode_verified`). Objects keep their
//!   persistent ids and come in id order, as a client opening the project
//!   gets them; the file carries the project's id.
//! - **What does not survive:** negative zero, which PostgreSQL's numbers do
//!   not have (`cad.rs`); every other value comes back bit for bit.
//! - **Who:** `project.download` (the organisation's `viewer_download`
//!   applies). A file project is its `.kcad` revisions already (docs/adr/0031).
//! - The event cursor of the same moment goes with the file: a client that
//!   opens the file follows the project's events after it.

use kentos_contracts::{
    Bounds, DOCUMENT_FORMAT, DOCUMENT_VERSION_2, DocumentSnapshotV2, EntityId, LayerNode,
    ProjectId, ProjectPermission, ProjectSettings, ProjectStorage, ProjectStyles, Vec2,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

use crate::access::{ProjectAccess, not_found};
use crate::error::{AppError, AppResult};
use crate::files::verifying;
use crate::projects::{FEATURE_COLUMNS, FeatureRow, gone, record, storage_of};

/// What a new project takes from a drawing besides its objects: its
/// settings, origin, view, layers, active layer and styles.
#[derive(Clone, Debug)]
pub struct DrawingMeta {
    pub settings: ProjectSettings,
    pub origin: Vec2,
    pub home_view: Option<Bounds>,
    pub layers: Vec<LayerNode>,
    pub active_layer: String,
    pub styles: ProjectStyles,
}

impl DrawingMeta {
    pub fn of(doc: &DocumentSnapshotV2) -> Self {
        Self {
            settings: doc.settings.clone(),
            origin: doc.origin,
            home_view: doc.home_view,
            layers: doc.layers.clone(),
            active_layer: doc.active_layer.clone(),
            styles: doc.styles.clone(),
        }
    }
}

/// A database project written as one KCAD v2 file.
#[derive(Debug)]
pub struct ProjectSnapshot {
    /// The project's name (the file is named after it).
    pub name: String,
    /// The data revision the file shows: the project as that commit left it.
    pub revision: i64,
    /// The project's event cursor at the same moment.
    pub event_cursor: i64,
    pub bytes: Vec<u8>,
    /// SHA-256 of `bytes`, 64 lowercase hexadecimal digits.
    pub sha256: String,
    /// How many objects the file holds.
    pub objects: usize,
    /// The drawing's settings, layers and styles, as the file has them.
    pub meta: DrawingMeta,
}

/// The project in scope as one KCAD v2 file (`project.download`).
pub async fn snapshot(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
) -> AppResult<ProjectSnapshot> {
    access.live()?;
    access.require(ProjectPermission::Download)?;
    take(db, access).await
}

/// The file itself, for a caller that checked what it needs (a checkpoint
/// is kept by `feature.write`, docs/adr/0034); row security still applies.
pub(crate) async fn take(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
) -> AppResult<ProjectSnapshot> {
    let mut tx = db.snapshot(access.scope()).await?;
    let (doc, revision, event_cursor) = read(&mut tx, access).await?;
    tx.commit().await?;
    let name = doc.name.clone();
    let objects = doc.entities.len();
    let meta = DrawingMeta::of(&doc);
    // The drawing and its bytes are in memory together: as many at once as uploads are verified.
    let _turn = verifying()
        .acquire()
        .await
        .map_err(|e| AppError::Storage(std::io::Error::other(e)))?;
    let bytes = tokio::task::spawn_blocking(move || kentos_kcad::encode_verified(&doc))
        .await
        .map_err(|e| AppError::Storage(std::io::Error::other(e)))?
        .map_err(|e| {
            AppError::invalid(format!(
                "“{name}” projesi .kcad olarak yazılamadı: {}",
                e.message
            ))
        })?;
    let sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(ProjectSnapshot {
        name,
        revision,
        event_cursor,
        bytes,
        sha256,
        objects,
        meta,
    })
}

type SnapshotRow = (
    String,
    Value,
    Value,
    String,
    f64,
    f64,
    Option<Value>,
    Value,
    i64,
    i64,
    String,
    bool,
);

/// The drawing of the project in scope, its data revision and its event
/// cursor, as `tx` sees them. Opened with `Db::snapshot`, `tx` sees one
/// moment for all of it.
pub async fn read(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
) -> AppResult<(DocumentSnapshotV2, i64, i64)> {
    let row: Option<SnapshotRow> = sqlx::query_as(
        "select p.name, p.settings, p.layers, p.active_layer, p.origin_x, p.origin_y, p.home_view, p.styles, p.data_revision,
                greatest(coalesce((select max(seq) from kentos.outbox_event o where o.tenant_id = p.tenant_id and o.project_id = p.id), 0),
                         coalesce((select pruned_through from kentos.outbox_horizon h where h.tenant_id = p.tenant_id and h.project_id = p.id), 0)),
                p.storage, p.deleted_at is not null
           from kentos.project p where p.tenant_id = $1 and p.id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_optional(&mut **tx)
    .await?;
    let (
        name,
        settings,
        layers,
        active_layer,
        ox,
        oy,
        home,
        styles,
        revision,
        cursor,
        storage,
        deleted,
    ) = row.ok_or_else(not_found)?;
    if deleted {
        return Err(gone(&name));
    }
    if storage_of(&storage) == ProjectStorage::File {
        return Err(AppError::invalid(format!(
            "“{name}” projesi dosya olarak saklanıyor; .kcad dosyası zaten onun revizyonlarıdır. Son revizyonu indirin."
        )));
    }
    let rows: Vec<FeatureRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {FEATURE_COLUMNS} from kentos.feature where tenant_id = $1 and project_id = $2 order by id"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .fetch_all(&mut **tx)
    .await?;
    let mut entities = Vec::with_capacity(rows.len());
    let mut uids = Vec::with_capacity(rows.len());
    for row in rows {
        uids.push(EntityId(*row.0.as_bytes()));
        entities.push(record(row)?.entity);
    }
    let bad = |e: serde_json::Error| {
        AppError::invalid(format!("“{name}” projesinin kaydı okunamadı: {e}"))
    };
    let doc = DocumentSnapshotV2 {
        format: DOCUMENT_FORMAT.to_owned(),
        version: DOCUMENT_VERSION_2,
        name: name.clone(),
        settings: serde_json::from_value(settings).map_err(bad)?,
        origin: Vec2 { x: ox, y: oy },
        home_view: home
            .map(serde_json::from_value::<Bounds>)
            .transpose()
            .map_err(bad)?,
        layers: serde_json::from_value(layers).map_err(bad)?,
        active_layer,
        entities,
        uids,
        styles: serde_json::from_value(styles).map_err(bad)?,
        project_id: Some(ProjectId(*access.project.as_bytes())),
        migrated_from: None,
    };
    Ok((doc, revision, cursor))
}
