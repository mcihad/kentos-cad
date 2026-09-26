//! The routes of file projects (docs/adr/0031): opening an upload, sending
//! its bytes, listing the revisions and downloading one. The commit is the
//! product command `project.file.commit` on the project's command route.
//! A database project comes as one `.kcad` file too: its snapshot of one
//! moment (docs/adr/0033).
//!
//! Bodies stream: an upload goes to the object store frame by frame (the
//! upload router's own size limit and timeout, `router`), and a download is
//! read from it in parts, so neither holds a whole file in memory here.

use std::pin::Pin;
use std::task::{Context, Poll};

use axum::Json;
use axum::body::{Body, Bytes, HttpBody};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use http_body::Frame;
use kentos_application::blobs::WriteError;
use kentos_application::files;
use kentos_contracts::{FileRevisions, FileUpload, FileUploadBegin};
use tokio::io::{AsyncRead, ReadBuf};
use uuid::Uuid;

use super::AppState;
use super::auth::Caller;
use super::error::{Body as JsonBody, Failure};
use super::projects::project_access;
use kentos_application::AppError;

/// `POST …/projects/{project}/uploads`: opens an upload for a file of the
/// declared size and SHA-256 (`feature.write`).
pub async fn begin(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    JsonBody(input): JsonBody<FileUploadBegin>,
) -> Result<(StatusCode, Json<FileUpload>), Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        files::begin(state.db()?, &a, input).await
    };
    run.await
        .map(|u| (StatusCode::CREATED, Json(u)))
        .map_err(|e| Failure::with(e, &headers))
}

#[derive(serde::Deserialize)]
pub struct PartQuery {
    offset: Option<String>,
}

/// `PUT …/projects/{project}/uploads/{upload}`: the upload's bytes
/// (`application/octet-stream`). They are kept only when the size and the
/// SHA-256 are the declared ones and they read as a KCAD v2 file. With
/// `?offset=N` the body is one part of the file, going on from the bytes
/// that arrived (docs/adr/0045).
pub async fn receive(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, upload)): Path<(String, String, String)>,
    axum::extract::Query(part): axum::extract::Query<PartQuery>,
    body: Body,
) -> Result<Json<FileUpload>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let upload = Uuid::parse_str(&upload).map_err(|_| {
            AppError::not_found("Yükleme bulunamadı: adresteki yükleme kimliği geçersiz.")
        })?;
        let db = state.db()?;
        if let Some(offset) = part.offset {
            let offset: u64 = offset.parse().map_err(|_| {
                AppError::invalid_at("offset", "offset bir bayt sırası (sayı) olmalı.")
            })?;
            // A part is taken whole before anything is written: one cut short changes nothing.
            let bytes = axum::body::to_bytes(body, kentos_application::upload_parts::PART_MAX)
                .await
                .map_err(|e| {
                    AppError::invalid(format!(
                        "Parça alınamadı ({e}); en çok {} MiB olabilir. Aynı parçayı yeniden gönderin.",
                        kentos_application::upload_parts::PART_MAX / (1024 * 1024)
                    ))
                })?;
            return kentos_application::upload_parts::receive_part(
                db,
                &state.blobs,
                &a,
                upload,
                offset,
                &bytes,
            )
            .await;
        }
        let mut receiving = files::start_receive(db, &state.blobs, &a, upload).await?;
        let mut body = body;
        loop {
            let frame = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await;
            match frame {
                None => break,
                Some(Err(e)) => {
                    let why = WriteError::Interrupted(e.to_string());
                    return Err(files::failed_write(&state.blobs, receiving, why).await);
                }
                Some(Ok(frame)) => {
                    let Ok(data) = frame.into_data() else {
                        continue;
                    };
                    if let Err(e) = receiving.writer.write(&data).await {
                        return Err(files::failed_write(&state.blobs, receiving, e).await);
                    }
                }
            }
        }
        files::finish_receive(db, &state.blobs, &a, receiving).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/uploads/{upload}`: the caller's own upload as it
/// stands (`feature.write`): whether its bytes arrived. A client whose answer
/// to `PUT` was lost asks this before sending them again (docs/adr/0040).
pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, upload)): Path<(String, String, String)>,
) -> Result<Json<FileUpload>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let upload = Uuid::parse_str(&upload).map_err(|_| {
            AppError::not_found("Yükleme bulunamadı: adresteki yükleme kimliği geçersiz.")
        })?;
        files::upload(state.db()?, &state.blobs, &a, upload).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/files`: the revisions, newest first (`project.read`).
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Json<FileRevisions>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        files::list(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// The start of an open-ended byte range (`Range: bytes=N-`): the only kind
/// a download goes on with; anything else is answered with the whole file.
fn range_start(headers: &HeaderMap) -> Option<u64> {
    let text = headers.get(header::RANGE)?.to_str().ok()?.trim();
    let (start, end) = text.strip_prefix("bytes=")?.split_once('-')?;
    if !end.trim().is_empty() {
        return None;
    }
    start.trim().parse().ok()
}

/// `GET …/projects/{project}/files/{revision}`: one revision's bytes
/// (`project.download`), with its SHA-256 as the entity tag. A revision
/// never changes, so a download cut short goes on from where it stopped:
/// `Range: bytes=N-` answers the rest (206) (docs/adr/0045).
pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, revision)): Path<(String, String, String)>,
) -> Result<Response, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let revision: i64 = revision.parse().ok().filter(|r| *r > 0).ok_or_else(|| {
            AppError::not_found(format!("“{}” projesinde böyle bir revizyon yok.", a.name))
        })?;
        let (info, mut file) = files::download(state.db()?, &state.blobs, &a, revision).await?;
        let size = u64::from(info.size);
        let start = range_start(&headers).filter(|s| *s > 0);
        if let Some(s) = start
            && s >= size
        {
            let mut response = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
            if let Ok(v) = HeaderValue::from_str(&format!("bytes */{size}")) {
                response.headers_mut().insert(header::CONTENT_RANGE, v);
            }
            return Ok(response);
        }
        if let Some(s) = start {
            use tokio::io::AsyncSeekExt as _;
            file.seek(std::io::SeekFrom::Start(s))
                .await
                .map_err(AppError::Storage)?;
        }
        let mut response = Body::new(FileBody::new(file)).into_response();
        let h = response.headers_mut();
        h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
        match start {
            Some(s) => {
                h.insert(header::CONTENT_LENGTH, HeaderValue::from(size - s));
                if let Ok(v) = HeaderValue::from_str(&format!("bytes {s}-{}/{size}", size - 1)) {
                    h.insert(header::CONTENT_RANGE, v);
                }
            }
            None => {
                h.insert(header::CONTENT_LENGTH, HeaderValue::from(size));
            }
        }
        kcad_headers(
            h,
            &info.sha256,
            &info.revision,
            &format!("{}-r{}.kcad", a.name, info.revision),
        );
        if start.is_some() {
            *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        }
        Ok(response)
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/snapshot`: a database project as one KCAD v2
/// file of one data revision (`project.download`, docs/adr/0033), with that
/// revision and the event cursor of the same moment.
pub async fn snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Response, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let s = kentos_application::snapshot::snapshot(state.db()?, &a).await?;
        let revision = s.revision.to_string();
        let name = format!("{}-r{revision}.kcad", s.name);
        let mut response = Body::from(s.bytes).into_response();
        let h = response.headers_mut();
        kcad_headers(h, &s.sha256, &revision, &name);
        h.insert("x-kentos-event-cursor", HeaderValue::from(s.event_cursor));
        Ok(response)
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}

/// The headers of a `.kcad` response: its type, its SHA-256 as the entity
/// tag, its revision and its file name (non-ASCII names in the RFC 5987 form).
pub(crate) fn kcad_headers(h: &mut HeaderMap, sha256: &str, revision: &str, name: &str) {
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    if let Ok(tag) = HeaderValue::from_str(&format!("\"{sha256}\"")) {
        h.insert(header::ETAG, tag);
    }
    if let Ok(v) = HeaderValue::from_str(revision) {
        h.insert("x-kentos-revision", v);
    }
    let ascii: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_. ".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let encoded: String = name
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    if let Ok(v) = HeaderValue::from_str(&format!(
        "attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}"
    )) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
}

/// A file read in parts as a response body.
pub(crate) struct FileBody {
    file: tokio::fs::File,
    buffer: Vec<u8>,
}

impl FileBody {
    const PART: usize = 64 * 1024;

    pub(crate) fn new(file: tokio::fs::File) -> Self {
        Self {
            file,
            buffer: vec![0; Self::PART],
        }
    }
}

impl HttpBody for FileBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let mut read = ReadBuf::new(&mut this.buffer);
        match Pin::new(&mut this.file).poll_read(cx, &mut read) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Ready(Ok(())) if read.filled().is_empty() => Poll::Ready(None),
            Poll::Ready(Ok(())) => {
                Poll::Ready(Some(Ok(Frame::data(Bytes::copy_from_slice(read.filled())))))
            }
        }
    }
}
