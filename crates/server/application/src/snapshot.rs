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
    BlockDefinition, Bounds, DOCUMENT_FORMAT, DOCUMENT_VERSION_2, DocumentSnapshotV2, EntityId,
    LayerNode, ProjectId, ProjectPermission, ProjectSettings, ProjectStorage, ProjectStyles, Vec2,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

use crate::access::{ProjectAccess, not_found};
use crate::error::{AppError, AppResult};
use crate::files::verifying;
use crate::projects::{FEATURE_COLUMNS, FeatureRow, gone, record, storage_of};

/// What a new project takes from a drawing besides its objects: its
/// settings, origin, view, layers, active layer, styles, block definitions
/// (docs/adr/0144 §5) and sheet book (docs/adr/0251).
#[derive(Clone, Debug)]
pub struct DrawingMeta {
    pub settings: ProjectSettings,
    pub origin: Vec2,
    pub home_view: Option<Bounds>,
    pub layers: Vec<LayerNode>,
    pub active_layer: String,
    pub styles: ProjectStyles,
    pub blocks: Vec<BlockDefinition>,
    pub sheets: Option<kentos_contracts::ProjectSheets>,
    /// The associative arrays (docs/adr/0300).
    pub arrays: Vec<kentos_contracts::AssocArray>,
    /// The constraints and parameters (docs/adr/0336 §3).
    pub constraints: kentos_contracts::ConstraintSet,
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
            blocks: doc.blocks.clone(),
            sheets: doc.sheets.clone(),
            arrays: doc.arrays.clone(),
            constraints: doc.constraints.clone(),
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
    /// What the file was cut by (docs/adr/0550 §3): the reach's stamp ([`kentos_forms::reach::Reach::stamp`])
    /// of the caller it was taken for; none for a whole copy.
    pub reach: Option<String>,
    /// Whether the objects were cut by it: the copy is not the project (docs/adr/0559: a result computed on it is not
    /// written back).
    pub cut: bool,
}

/// How a copy of the whole project is taken (docs/adr/0550 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Take {
    /// As the records are: a backup (a checkpoint) or a copy for someone who may have all of it.
    Whole,
    /// As the caller may take it out: the attributes forms mark `redact` cut, unless the caller designs forms.
    Cut,
}

/// The project in scope as one KCAD v2 file (`project.download`).
pub async fn snapshot(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
) -> AppResult<ProjectSnapshot> {
    access.live()?;
    access.require(ProjectPermission::Download)?;
    take(db, access, Take::Cut).await
}

/// The file itself, for a caller that checked what it needs (a checkpoint
/// is kept by `feature.write`, docs/adr/0034); row security still applies.
pub(crate) async fn take(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    how: Take,
) -> AppResult<ProjectSnapshot> {
    let mut tx = db.snapshot(access.scope()).await?;
    let (mut doc, revision, event_cursor) = read(&mut tx, access).await?;
    tx.commit().await?;
    // Cut by the same moment's forms: the settings the file carries are the ones it was judged by.
    let mut cut = false;
    let reach = match how {
        Take::Whole => None,
        Take::Cut => {
            // A Veri girişi reads records through forms; it has no right to a copy (no `project.download`).
            if access.through_forms() {
                return Err(AppError::forbidden(
                    "Veri girişi projenin kopyasını almaz; kayıtlarını formla okur.",
                ));
            }
            let variables =
                crate::form_guard::variables_for(&doc.name, &doc.settings, &access.actor);
            let reach = crate::form_guard::reach_with(
                &doc.settings,
                &doc.layers,
                &variables,
                crate::projects::caller_of(access),
            );
            if !reach.whole_copy_ok() {
                cut = true;
                doc.entities = std::mem::take(&mut doc.entities)
                    .into_iter()
                    .map(|e| crate::form_guard::cut(e, &reach))
                    .collect();
            }
            Some(reach.stamp())
        }
    };
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
        reach,
        cut,
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
    Option<Value>,
    Option<Value>,
    Option<Value>,
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
                p.storage, p.deleted_at is not null, p.sheets, p.arrays, p.constraints
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
        sheets,
        arrays,
        constraints,
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
    let blocks: Vec<BlockDefinition> =
        crate::blocks::read(tx, access.tenant, access.project, false)
            .await?
            .into_iter()
            .map(|s| s.block)
            .collect();
    // The arrays as the blocks and objects of this moment hold them (docs/adr/0300 §5): another editor may have
    // removed an item or a block since they were written.
    let arrays: Vec<kentos_contracts::AssocArray> = arrays
        .map(serde_json::from_value)
        .transpose()
        .map_err(bad)?
        .unwrap_or_default();
    let block_ids: Vec<_> = blocks.iter().map(|b| b.id).collect();
    let arrays = kentos_contracts::arrays_kept(&arrays, &block_ids, &uids);
    // The constraints as the objects of this moment hold them (docs/adr/0336 §3).
    let constraints: kentos_contracts::ConstraintSet = constraints
        .map(serde_json::from_value)
        .transpose()
        .map_err(bad)?
        .unwrap_or_default();
    let present: std::collections::HashSet<EntityId> = uids.iter().copied().collect();
    let constraints = kentos_contracts::constraints_kept(&constraints, &present);
    let doc = DocumentSnapshotV2 {
        constraints,
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
        blocks,
        project_id: Some(ProjectId(*access.project.as_bytes())),
        migrated_from: None,
        sheets: sheets
            .map(serde_json::from_value)
            .transpose()
            .map_err(bad)?,
        // The server keeps its own idempotency (docs/adr/0026); a drawing's ledger is the file's.
        requests: Vec::new(),
        arrays,
    };
    Ok((doc, revision, cursor))
}
