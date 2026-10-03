//! The sheet template library over HTTP (docs/sheet/design.md §13): the
//! caller's list, one template with its newest content (`ETag` its
//! revision), its sharing and the people it may be shared with (the owner
//! only), and the caller's template events after a cursor, waiting for the
//! next one when asked (a long poll, as a project's events, docs/adr/0044;
//! a template command of this process wakes it, one of another process is
//! seen at the next look).
//! A template the caller has no role in, and a malformed id, answer 404 word
//! for word as one that does not exist. The commands that change templates
//! go to the command route of the caller's personal space
//! (`POST /v1/tenants/{tenant}/commands`, `kentos_application::sheet_templates`).

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kentos_application::commands::CommandOutcome;
use kentos_application::{AppError, sheet_templates};
use kentos_sheet::cloud::{
    SheetTemplateAccess, SheetTemplateCandidates, SheetTemplateEventPage, SheetTemplateList,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use super::auth::Caller;
use super::error::Failure;
use crate::hub::Hub;

fn id_of(text: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(text).map_err(|_| sheet_templates::not_found())
}

/// `GET /v1/me/sheet-templates`: the caller's own templates and the ones shared with them.
pub async fn mine(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<SheetTemplateList>, Failure> {
    let run = async { sheet_templates::mine(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/sheet-templates/{id}`: the metadata and the newest content. The
/// `ETag` is the revision; asked again with it in `If-None-Match`, an
/// unchanged template answers 304 without its content.
pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<Response, Failure> {
    let run = async { sheet_templates::detail(state.db()?, &caller.0, id_of(&id)?).await };
    let d = run.await.map_err(|e| Failure::with(e, &headers))?;
    let tag = format!("\"{}\"", d.summary.revision);
    let etag = HeaderValue::from_str(&tag).unwrap_or(HeaderValue::from_static("\"0\""));
    let unchanged = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == tag));
    if unchanged {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }
    Ok(([(header::ETAG, etag)], Json(d)).into_response())
}

/// `GET /v1/sheet-templates/{id}/access`: whom the owner has shared it with (the owner only).
pub async fn access(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<Json<SheetTemplateAccess>, Failure> {
    let run = async { sheet_templates::access_list(state.db()?, &caller.0, id_of(&id)?).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Deserialize)]
pub struct CandidateQuery {
    q: Option<String>,
}

/// `GET /v1/sheet-templates/{id}/access/candidates?q=`: people of the owner's organisations to share it with (the owner only).
pub async fn candidates(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Query(q): Query<CandidateQuery>,
) -> Result<Json<SheetTemplateCandidates>, Failure> {
    let run = async {
        sheet_templates::candidates(
            state.db()?,
            &caller.0,
            id_of(&id)?,
            q.q.as_deref().unwrap_or(""),
        )
        .await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Deserialize)]
pub struct EventQuery {
    after: Option<String>,
    limit: Option<i64>,
    /// Seconds to wait for an event when none is new (a long poll).
    wait: Option<u64>,
}

/// How often a waiting request looks again without a signal: a template
/// command of another process (another server) does not signal this one.
const LOOK_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// The hub's signal for "a sheet template changed": no project is the nil one,
/// so a project's listeners pass it by; the template listeners read their own events.
const SIGNAL: (Uuid, Uuid) = (Uuid::nil(), Uuid::nil());

/// After a command of the personal space: a template that changed wakes the
/// waiting template event requests of this process (each reads only its own).
pub(super) fn announce(hub: &Hub, outcome: &CommandOutcome) {
    if let CommandOutcome::SheetTemplate(c) = outcome
        && c.changed
        && !c.replayed
    {
        hub.notify(SIGNAL.0, SIGNAL.1);
    }
}

/// `GET /v1/me/sheet-templates/events?after=&limit=&wait=`: the caller's
/// template events after a cursor; with `wait` (at most
/// [`super::projects::EVENTS_WAIT_MAX`] seconds) a request that finds none
/// waits for the next one.
pub async fn events(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Query(q): Query<EventQuery>,
) -> Result<Json<SheetTemplateEventPage>, Failure> {
    let run = async {
        let after = q
            .after
            .as_deref()
            .unwrap_or("0")
            .parse::<i64>()
            .map_err(|_| AppError::invalid_at("after", "after bir olay imleci olmalı."))?;
        let limit = q.limit.unwrap_or(sheet_templates::EVENTS_PAGE_MAX);
        let db = state.db()?;
        let Some(wait) = q.wait.filter(|w| *w > 0) else {
            return sheet_templates::events_after(db, &caller.0, after, limit).await;
        };
        // Listening before the first read: a change between the read and the wait is not missed.
        let mut changes = state.hub.subscribe();
        let deadline = tokio::time::Instant::now()
            + std::time::Duration::from_secs(wait.min(super::projects::EVENTS_WAIT_MAX));
        loop {
            let page = sheet_templates::events_after(db, &caller.0, after, limit).await?;
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if !page.events.is_empty() || left.is_zero() {
                return Ok(page);
            }
            let signal = async {
                loop {
                    match changes.recv().await {
                        Ok(s) if s == SIGNAL => return,
                        Ok(_) => {}
                        // Missed signals: read again, one of them may be a template's.
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => return,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            std::future::pending::<()>().await
                        }
                    }
                }
            };
            let _ = tokio::time::timeout(left.min(LOOK_EVERY), signal).await;
        }
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}
