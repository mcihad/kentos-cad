//! The routes of files staged for server jobs (docs/adr/0561 §1): opening one, sending its bytes and asking how it
//! stands. The job that reads it is asked for on the jobs' route (`project.import`, its `upload` the id given here).
//! The bytes stream to the object store frame by frame under the upload router's own size limit and timeout.

use std::pin::Pin;

use axum::Json;
use axum::body::{Body, HttpBody};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::blobs::WriteError;
use kentos_application::{AppError, job_inputs};
use kentos_contracts::{JobInput, JobInputBegin};
use uuid::Uuid;

use super::AppState;
use super::auth::Caller;
use super::error::{Body as JsonBody, Failure};
use super::projects::project_access;

fn input_id(text: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(text)
        .map_err(|_| AppError::not_found("Hazırlanan dosya bulunamadı: adresteki kimlik geçersiz."))
}

/// `POST …/projects/{project}/job-inputs`: opens a staged file for a file of the declared size and SHA-256 (`jobs.run`
/// and `feature.write`).
pub async fn begin(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    JsonBody(input): JsonBody<JobInputBegin>,
) -> Result<(StatusCode, Json<JobInput>), Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        job_inputs::begin(state.db()?, &a, input, state.config.job_limits.input_max).await
    };
    run.await
        .map(|u| (StatusCode::CREATED, Json(u)))
        .map_err(|e| Failure::with(e, &headers))
}

/// `PUT …/projects/{project}/job-inputs/{input}`: the file's bytes (`application/octet-stream`), kept only when their
/// size and SHA-256 are the declared ones.
pub async fn receive(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, input)): Path<(String, String, String)>,
    body: Body,
) -> Result<Json<JobInput>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let id = input_id(&input)?;
        let db = state.db()?;
        let mut receiving = job_inputs::start_receive(db, &state.blobs, &a, id).await?;
        let mut body = body;
        loop {
            let frame = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await;
            match frame {
                None => break,
                Some(Err(e)) => {
                    let why = WriteError::Interrupted(e.to_string());
                    return Err(job_inputs::failed_write(&state.blobs, receiving, why).await);
                }
                Some(Ok(frame)) => {
                    let Ok(data) = frame.into_data() else {
                        continue;
                    };
                    if let Err(e) = receiving.writer.write(&data).await {
                        return Err(job_inputs::failed_write(&state.blobs, receiving, e).await);
                    }
                }
            }
        }
        job_inputs::finish_receive(db, &state.blobs, &a, receiving).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/job-inputs/{input}`: the caller's own staged file as it stands: whether its bytes arrived.
pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, input)): Path<(String, String, String)>,
) -> Result<Json<JobInput>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        job_inputs::get(state.db()?, &a, input_id(&input)?).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}
