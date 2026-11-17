//! A dossier's routes (docs/adr/9000 §5–7): a professional's list, figures and new application; one dossier's page,
//! edits, removal, steps and messages; its files coming in (streamed into the store, hashed as they arrive), going out
//! (checked against their hash first), taken from a cloud project and opened as one.

use std::pin::Pin;

use axum::Json;
use axum::body::{Body as Bytes, HttpBody};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kentos_application::AppError;
use kentos_application::blobs::WriteError;
use kentos_application::portal::{detail, dossiers, files, steps};
use kentos_contracts::{
    ApplicationCreate, DossierActionRequest, DossierDashboard, DossierDetail, DossierEdit,
    DossierFileImport, DossierFileImported, DossierFileView, DossierList, DossierMessageView,
    FromProject, MessagePost,
};
use serde::{Deserialize, Serialize};

use super::{ListQuery, path_id};
use crate::http::AppState;
use crate::http::auth::Caller;
use crate::http::error::{Body, Failure};
use crate::http::files::{FileBody, attachment};

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
        let media = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream");
        let recorded = files::record_file(
            db,
            &caller.0,
            &prepared,
            &q.name,
            media,
            (size, &sha256),
            q.doc_type.as_deref(),
            None,
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
        let mut response = Bytes::new(FileBody::new(opened)).into_response();
        let h = response.headers_mut();
        h.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&view.media_type)
                .unwrap_or(HeaderValue::from_static("application/octet-stream")),
        );
        h.insert(header::CONTENT_LENGTH, HeaderValue::from(view.size));
        if let Ok(tag) = HeaderValue::from_str(&format!("\"{}\"", view.sha256)) {
            h.insert(header::ETAG, tag);
        }
        if let Some(v) = attachment(&view.name) {
            h.insert(header::CONTENT_DISPOSITION, v);
        }
        h.insert(
            "x-content-type-options",
            HeaderValue::from_static("nosniff"),
        );
        Ok(response)
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
