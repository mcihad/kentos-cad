//! A dossier's routes (docs/adr/9000 §5–7): a professional's list, figures and new application; one dossier's page,
//! edits, removal, steps and messages; its files coming in (streamed into the store, hashed as they arrive), going out
//! (checked against their hash first), taken from a cloud project and opened as one.

use std::pin::Pin;

use axum::Json;
use axum::body::{Body as Bytes, HttpBody};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use kentos_application::AppError;
use kentos_application::blobs::WriteError;
use kentos_application::portal::{detail, dossiers, files, steps};
use kentos_contracts::{
    ApplicationCreate, DossierActionRequest, DossierDashboard, DossierDetail, DossierEdit,
    DossierFileFromUpload, DossierFileImport, DossierOfficeSet, DossierFileImported, DossierFileView, DossierList, DossierMessageView,
    FromProject, MessagePost,
};
use serde::{Deserialize, Serialize};

use super::{ListQuery, path_id};
use crate::http::AppState;
use crate::http::auth::Caller;
use crate::http::error::{Body, Failure};
use crate::http::download::{Stored, picture, respond};

/// `GET /v1/me/dossiers`.
pub async fn mine(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Query(q): Query<ListQuery>,
) -> Result<Json<DossierList>, Failure> {
    let run = async { dossiers::list_for_professional(state.db()?, &caller.0, &q.filter()?).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/me/dossiers/summary`.
pub async fn my_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<DossierDashboard>, Failure> {
    let run = async { dossiers::dashboard_for_professional(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    id: String,
}

/// `POST /v1/me/dossiers`: an application, as a draft.
pub async fn apply(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<ApplicationCreate>,
) -> Result<(StatusCode, Json<Opened>), Failure> {
    let run = async { dossiers::create_application(state.db()?, &caller.0, &input).await };
    run.await
        .map(|id| (StatusCode::CREATED, Json(Opened { id: id.to_string() })))
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<Json<DossierDetail>, Failure> {
    let run = async { detail::detail(state.db()?, &caller.0, path_id(&id, "Dosya")?).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<DossierEdit>,
) -> Result<Json<DossierDetail>, Failure> {
    let run = async {
        let (db, id) = (state.db()?, path_id(&id, "Dosya")?);
        dossiers::edit(db, &caller.0, id, &input).await?;
        detail::detail(db, &caller.0, id).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    let run = async {
        dossiers::delete(state.db()?, &state.blobs, &caller.0, path_id(&id, "Dosya")?).await
    };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn act(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<DossierActionRequest>,
) -> Result<Json<DossierDetail>, Failure> {
    let run = async { steps::act(state.db()?, &caller.0, path_id(&id, "Dosya")?, &input).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn message(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<MessagePost>,
) -> Result<(StatusCode, Json<DossierMessageView>), Failure> {
    let run =
        async { steps::message(state.db()?, &caller.0, path_id(&id, "Dosya")?, &input).await };
    run.await
        .map(|m| (StatusCode::CREATED, Json(m)))
        .map_err(|e| Failure::with(e, &headers))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadQuery {
    name: String,
    doc_type: Option<String>,
    purpose: Option<String>,
}

/// `POST /v1/dossiers/{id}/files?name=&docType=&purpose=`: the file's bytes are the request's body.
pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Query(q): Query<UploadQuery>,
    body: Bytes,
) -> Result<(StatusCode, Json<DossierFileView>), Failure> {
    let run = async {
        let db = state.db()?;
        let id = path_id(&id, "Dosya")?;
        let purpose = files::Purpose::of(q.purpose.as_deref().unwrap_or_default())?;
        files::file_name(&q.name)?;
        let prepared = files::prepare(db, &caller.0, id, purpose).await?;
        let mut writer = state
            .blobs
            .create(&prepared.key, files::MAX_FILE_BYTES)
            .await?;
        let mut body = body;
        let failed = loop {
            match std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await {
                None => break None,
                Some(Err(e)) => break Some(WriteError::Interrupted(e.to_string())),
                Some(Ok(frame)) => {
                    if let Ok(data) = frame.into_data()
                        && let Err(e) = writer.write(&data).await
                    {
                        break Some(e);
                    }
                }
            }
        };
        if let Some(e) = failed {
            drop(writer);
            let _ = state.blobs.remove(&prepared.key).await;
            return Err(match e {
                WriteError::TooLarge { .. } => AppError::invalid(format!(
                    "Dosya en çok {} MB olabilir.",
                    files::MAX_FILE_BYTES / (1024 * 1024)
                )),
                WriteError::Interrupted(why) => AppError::invalid(format!(
                    "Dosyanın baytları yarıda kesildi ({why}); yeniden yükleyin."
                )),
                WriteError::Io(e) => AppError::Storage(e),
            });
        }
        let (size, sha256) = writer.finish().await?;
        if size == 0 {
            let _ = state.blobs.remove(&prepared.key).await;
            return Err(AppError::invalid(
                "Dosya boş; içinde bir şey olan bir dosya seçin.",
            ));
        }
        // Scanned before the other side may see it (docs/adr/9000 §14); a refused file is not kept.
        let scanned = match state.config.scanner.check(&state.blobs, &prepared.key, &q.name).await {
            Ok(Ok(s)) => s,
            Ok(Err(refusal)) => {
                let _ = state.blobs.remove(&prepared.key).await;
                return Err(
                    kentos_application::uploads::refused(db, &caller.0, &q.name, &refusal, "dossier").await,
                );
            }
            Err(e) => {
                let _ = state.blobs.remove(&prepared.key).await;
                return Err(e);
            }
        };
        let recorded = files::record_file(
            db,
            &caller.0,
            &prepared,
            &q.name,
            &scanned.media_type,
            (size, &sha256),
            q.doc_type.as_deref(),
            None,
            Some(&scanned),
        )
        .await;
        if recorded.is_err() {
            let _ = state.blobs.remove(&prepared.key).await;
        }
        recorded
    };
    run.await
        .map(|f| (StatusCode::CREATED, Json(f)))
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn from_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<FromProject>,
) -> Result<(StatusCode, Json<DossierFileView>), Failure> {
    let run = async {
        files::from_project(
            state.db()?,
            &state.blobs,
            &caller.0,
            path_id(&id, "Dosya")?,
            &input,
        )
        .await
    };
    run.await
        .map(|f| (StatusCode::CREATED, Json(f)))
        .map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/dossiers/{id}/files/{file}`: its bytes, once they match their hash.
pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((id, file)): Path<(String, String)>,
    Query(q): Query<InlineQuery>,
) -> Result<Response, Failure> {
    let run = async {
        let (view, opened) = files::download(
            state.db()?,
            &state.blobs,
            &caller.0,
            path_id(&id, "Dosya")?,
            path_id(&file, "Dosya")?,
        )
        .await?;
        respond(
            &headers,
            Stored {
                file: opened,
                size: view.size,
                sha256: &view.sha256,
                name: &view.name,
                media_type: &view.media_type,
                inline: matches!(q.inline.as_deref(), Some("1" | "true" | "yes")),
            },
        )
        .await
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}

#[derive(Debug, Default, Deserialize)]
pub struct InlineQuery {
    inline: Option<String>,
}

/// `PUT /v1/dossiers/{id}/office`: the responsible professional shares the dossier with an office, or takes it back.
pub async fn set_office(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<DossierOfficeSet>,
) -> Result<Json<DossierDetail>, Failure> {
    let run = async {
        let db = state.db()?;
        let id = path_id(&id, "Dosya")?;
        dossiers::set_office(db, &caller.0, id, &input).await?;
        detail::detail(db, &caller.0, id).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `PUT /v1/dossiers/{id}/live-project`: the municipality's live project the transfer is worked on in (docs/adr/9000
/// §24), or none.
pub async fn set_live_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<kentos_contracts::DossierLiveProjectSet>,
) -> Result<Json<DossierDetail>, Failure> {
    let run = async {
        let db = state.db()?;
        let id = path_id(&id, "Dosya")?;
        dossiers::set_live_project(db, &caller.0, id, &input).await?;
        detail::detail(db, &caller.0, id).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/dossiers/{id}/files/from-upload`: a received upload (`/v1/uploads`) as a pending file (201).
pub async fn from_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<DossierFileFromUpload>,
) -> Result<(StatusCode, Json<DossierFileView>), Failure> {
    let run = async {
        files::from_upload(state.db()?, &state.blobs, &caller.0, path_id(&id, "Dosya")?, &input).await
    };
    run.await
        .map(|f| (StatusCode::CREATED, Json(f)))
        .map_err(|e| Failure::with(e, &headers))
}

#[derive(Debug, Default, Deserialize)]
pub struct PreviewQuery {
    width: Option<u32>,
}

/// `GET /v1/dossiers/{id}/files/{file}/preview.svg`: a KentOS project file of the dossier as one SVG.
pub async fn preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((id, file)): Path<(String, String)>,
    Query(q): Query<PreviewQuery>,
) -> Result<Response, Failure> {
    let run = async {
        let p = kentos_application::preview::dossier_file(
            state.db()?,
            &state.blobs,
            &caller.0,
            (path_id(&id, "Dosya")?, path_id(&file, "Dosya")?),
            q.width.unwrap_or(kentos_application::preview::DEFAULT_WIDTH),
            headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()),
        )
        .await?;
        Ok(match p.picture {
            Some(pic) => picture(&p.tag, Some(pic.svg), pic.drawn, pic.skipped),
            None => picture(&p.tag, None, 0, 0),
        })
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}

pub async fn remove_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((id, file)): Path<(String, String)>,
) -> Result<StatusCode, Failure> {
    let run = async {
        files::remove(
            state.db()?,
            &state.blobs,
            &caller.0,
            path_id(&id, "Dosya")?,
            path_id(&file, "Dosya")?,
        )
        .await
    };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn import(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((id, file)): Path<(String, String)>,
    Body(input): Body<DossierFileImport>,
) -> Result<(StatusCode, Json<DossierFileImported>), Failure> {
    let run = async {
        files::import(
            state.db()?,
            &state.blobs,
            &caller.0,
            path_id(&id, "Dosya")?,
            path_id(&file, "Dosya")?,
            &input,
        )
        .await
    };
    run.await
        .map(|f| (StatusCode::CREATED, Json(f)))
        .map_err(|e| Failure::with(e, &headers))
}
