//! `/v1/ws`: the live channel of an open project (CLAUDE.md §21.1). The
//! client subscribes with the cursor it has applied; missed events are
//! replayed first, then new ones follow as they are committed. A cursor the
//! log cannot continue from (older than the events still kept, or beyond
//! the newest) gets `resyncRequired`: the client opens the project again.
//! The client sends a heartbeat (`ping`) at least every 60 s or is
//! disconnected. A job or a commit never depends on this socket: closing it
//! loses nothing on the server.
//!
//! Access (docs/adr/0015, TODOS.md CLOUD-13): the caller's access to the
//! project is asked again before every delivery: when a commit, a deletion
//! or a change of grants in the project is signalled, and at every 5 s poll
//! (which also catches changes made by another process, such as a
//! membership turned off from the command line). A subscription whose
//! access is gone gets the same `not_found` error as a project that never
//! existed and is dropped; nothing more of the project is sent. The session
//! itself is checked every 30 s.

use std::time::{Duration, Instant};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use kentos_application::access::{self, ProjectAccess};
use kentos_application::identity::{self, Actor};
use kentos_application::{AppError, events};
use kentos_contracts::{ClientMessage, ServerMessage};
use uuid::Uuid;

use super::AppState;
use super::auth::{Caller, SESSION_COOKIE, cookie};
use super::error::Failure;

const IDLE_LIMIT: Duration = Duration::from_secs(60);
const POLL_EVERY: Duration = Duration::from_secs(5);
const RECHECK_EVERY: Duration = Duration::from_secs(30);

pub async fn upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    ws: WebSocketUpgrade,
) -> Result<Response, Failure> {
    // Another site's page cannot open this socket with the user's cookie (cross-site WebSocket hijacking).
    let origin = headers.get("origin").and_then(|v| v.to_str().ok());
    if origin.is_some_and(|o| o != state.config.public_url) {
        return Err(Failure::with(
            AppError::forbidden("Bağlantı uygulamanın kendi adresinden gelmiyor."),
            &headers,
        ));
    }
    let session = cookie(&headers, SESSION_COOKIE);
    Ok(ws
        .on_upgrade(move |socket| run(socket, state, caller.0, session))
        .into_response())
}

struct Subscription {
    access: ProjectAccess,
    cursor: i64,
}

impl Subscription {
    fn project(&self) -> Uuid {
        self.access.project
    }
}

async fn send(socket: &mut WebSocket, msg: &ServerMessage) -> bool {
    let text = serde_json::to_string(msg).expect("server messages serialize");
    socket.send(Message::Text(text.into())).await.is_ok()
}

fn error(e: &AppError) -> ServerMessage {
    ServerMessage::Error {
        error: e.code().into(),
        message: e.to_string(),
    }
}

/// What a failed replay tells the client: reopen, or why it stopped.
fn answer(e: &AppError, project: Uuid) -> ServerMessage {
    match e {
        AppError::ResyncRequired(_) => ServerMessage::ResyncRequired {
            project_id: project.to_string(),
        },
        _ => error(e),
    }
}

/// Asks the subscriber's access again, then sends every event after the
/// subscription's cursor; false when the socket is gone. An error (access
/// gone, a cursor the log cannot continue from) ends the subscription.
async fn flush(
    socket: &mut WebSocket,
    state: &AppState,
    sub: &mut Subscription,
) -> Result<bool, AppError> {
    let db = state.db()?;
    sub.access = access::project(db, &sub.access.actor, sub.access.tenant, sub.project()).await?;
    loop {
        let page = events::after(db, &sub.access, sub.cursor, events::PAGE_MAX).await?;
        if page.events.is_empty() {
            return Ok(true);
        }
        sub.cursor = page.next.parse().unwrap_or(sub.cursor);
        let msg = ServerMessage::Events {
            project_id: sub.project().to_string(),
            events: page.events,
        };
        if !send(socket, &msg).await {
            return Ok(false);
        }
    }
}

/// The subscription and where the project's log stands.
async fn subscribe(
    state: &AppState,
    actor: &Actor,
    tenant: &str,
    project: &str,
    after: &str,
) -> Result<(Subscription, events::Bounds), AppError> {
    let cursor = after
        .parse::<i64>()
        .map_err(|_| AppError::invalid("after bir olay imleci olmalı."))?;
    let (Ok(tenant), Ok(project)) = (Uuid::parse_str(tenant), Uuid::parse_str(project)) else {
        return Err(access::not_found());
    };
    let access = access::project(state.db()?, actor, tenant, project).await?;
    let bounds = events::bounds(state.db()?, &access).await?;
    Ok((Subscription { access, cursor }, bounds))
}

async fn run(mut socket: WebSocket, state: AppState, actor: Actor, session: Option<String>) {
    let mut changes = state.hub.subscribe();
    let mut sub: Option<Subscription> = None;
    let mut last_heard = Instant::now();
    let mut last_check = Instant::now();
    let mut poll = tokio::time::interval(POLL_EVERY);
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(message)) = incoming else { break };
                last_heard = Instant::now();
                let text = match message {
                    Message::Text(t) => t,
                    Message::Close(_) => break,
                    _ => continue,
                };
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Ping { t }) => {
                        if !send(&mut socket, &ServerMessage::Pong { t }).await { break }
                    }
                    Ok(ClientMessage::Unsubscribe) => sub = None,
                    Ok(ClientMessage::Subscribe { tenant_id, project_id, after }) => {
                        // A new subscription replaces the old one, which ends here whatever happens next.
                        sub = None;
                        match subscribe(&state, &actor, &tenant_id, &project_id, &after).await {
                            // Older than the events still kept, or beyond the newest (a restored database): reopen.
                            Ok((s, bounds)) if !bounds.can_continue(s.cursor) => {
                                if !send(&mut socket, &ServerMessage::ResyncRequired { project_id: s.project().to_string() }).await { break }
                            }
                            Ok((mut s, _)) => {
                                let ok = send(&mut socket, &ServerMessage::Subscribed { project_id: s.project().to_string(), after: s.cursor.to_string() }).await;
                                if !ok { break }
                                match flush(&mut socket, &state, &mut s).await {
                                    Ok(true) => sub = Some(s),
                                    Ok(false) => break,
                                    Err(e) => { if !send(&mut socket, &answer(&e, s.project())).await { break } }
                                }
                            }
                            Err(e) => { if !send(&mut socket, &error(&e)).await { break } }
                        }
                    }
                    Err(e) => {
                        let msg = error(&AppError::invalid(format!("İleti okunamadı: {e}")));
                        if !send(&mut socket, &msg).await { break }
                    }
                }
            }
            changed = changes.recv() => {
                let relevant = match (&changed, &sub) {
                    (Ok((t, p)), Some(s)) => *t == s.access.tenant && *p == s.project(),
                    // Missed signals (a slow socket): check anyway.
                    (Err(tokio::sync::broadcast::error::RecvError::Lagged(_)), Some(_)) => true,
                    (Err(tokio::sync::broadcast::error::RecvError::Closed), _) => break,
                    _ => false,
                };
                if relevant && let Some(s) = sub.as_mut() {
                    match flush(&mut socket, &state, s).await {
                        Ok(true) => {}
                        Ok(false) => break,
                        Err(e) => { let msg = answer(&e, s.project()); sub = None; if !send(&mut socket, &msg).await { break } }
                    }
                }
            }
            _ = poll.tick() => {
                if last_heard.elapsed() > IDLE_LIMIT {
                    let _ = socket.send(Message::Close(None)).await;
                    break;
                }
                if last_check.elapsed() > RECHECK_EVERY {
                    last_check = Instant::now();
                    let Ok(db) = state.db() else { break };
                    // A cookie session must still be open (the project access is asked at every flush below).
                    if let Some(token) = &session && !matches!(identity::session_actor(db, token).await, Ok(Some(_))) {
                        let _ = send(&mut socket, &error(&AppError::Unauthenticated("Oturumunuz sona erdi; yeniden giriş yapın.".into()))).await;
                        break;
                    }
                }
                if let Some(s) = sub.as_mut() {
                    match flush(&mut socket, &state, s).await {
                        Ok(true) => {}
                        Ok(false) => break,
                        Err(e) => { let msg = answer(&e, s.project()); sub = None; if !send(&mut socket, &msg).await { break } }
                    }
                }
            }
        }
    }
}
