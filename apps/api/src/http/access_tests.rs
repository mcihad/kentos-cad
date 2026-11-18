//! Programs signing in with bearer tokens (docs/adr/9000 §12): a program's session needs no `x-kentos-client` header
//! (that header guards cookies), lists itself among the account's sessions and signs out; a personal access token
//! reads only when made so, never makes another token, and stops at once when revoked.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use kentos_application::admin;
use kentos_contracts::{ApiTokenCreated, Me, SessionView, TokenIssued};
use kentos_postgres::testing::TestDb;

use super::tests::{app, send};

fn json(method: &str, uri: &str, bearer: Option<&str>, body: serde_json::Value) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).unwrap()
}

fn get(uri: &str, bearer: &str) -> Request<Body> {
    Request::get(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .body(Body::empty())
        .unwrap()
}

async fn signed_in(app: &axum::Router) -> String {
    let (status, _, body) = send(
        app,
        json(
            "POST",
            "/v1/auth/token",
            None,
            serde_json::json!({ "login": "ayse", "password": "dogru-parola-1", "client": "mobile", "label": "Telefon" }),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let issued: TokenIssued = serde_json::from_slice(&body).unwrap();
    assert!(issued.token.starts_with("kentos_s_") && issued.token.len() == 9 + 64);
    assert_eq!(issued.me.user.display_name, "Ayşe Yılmaz");
    issued.token
}

#[tokio::test]
async fn a_program_signs_in_with_a_bearer_token_and_out_again() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_local_user(&db.owner, "ayse", "Ayşe Yılmaz", None, "dogru-parola-1")
        .await
        .unwrap();
    let app = app(Some(db.app.clone()));
    // A wrong password is the same refusal as for the browser.
    let (status, _, _) = send(
        &app,
        json(
            "POST",
            "/v1/auth/token",
            None,
            serde_json::json!({ "login": "ayse", "password": "yanlis-parola" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let token = signed_in(&app).await;
    let (status, _, body) = send(&app, get("/v1/me", &token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<Me>(&body)
            .unwrap()
            .user
            .display_name,
        "Ayşe Yılmaz"
    );
    // A change without the x-kentos-client header: a bearer request is no cross-site cookie.
    let (status, _, _) = send(
        &app,
        json(
            "POST",
            "/v1/me/notifications/read",
            Some(&token),
            serde_json::json!({ "all": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, _, body) = send(&app, get("/v1/me/sessions", &token)).await;
    let sessions: Vec<SessionView> = serde_json::from_slice(&body).unwrap();
    let mine = sessions
        .iter()
        .find(|s| s.current)
        .expect("its own session");
    assert_eq!(
        (
            mine.kind.as_str(),
            mine.client.as_deref(),
            mine.label.as_deref()
        ),
        ("bearer", Some("mobile"), Some("Telefon"))
    );
    // Signing out ends it.
    let (status, _, _) = send(
        &app,
        json(
            "POST",
            "/v1/auth/logout",
            Some(&token),
            serde_json::json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = send(&app, get("/v1/me", &token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_personal_token_reads_or_writes_as_made_and_makes_no_other() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_local_user(&db.owner, "ayse", "Ayşe Yılmaz", None, "dogru-parola-1")
        .await
        .unwrap();
    let app = app(Some(db.app.clone()));
    let session = signed_in(&app).await;
    let make = |scope: &str| {
        json(
            "POST",
            "/v1/me/tokens",
            Some(&session),
            serde_json::json!({ "name": format!("ajan {scope}"), "scope": scope, "expiresInDays": 30 }),
        )
    };
    let (status, _, body) = send(&app, make("read")).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let read: ApiTokenCreated = serde_json::from_slice(&body).unwrap();
    assert!(read.token.starts_with("kentos_pat_") && read.token.starts_with(&read.view.prefix));
    let (_, _, body) = send(&app, make("write")).await;
    let write: ApiTokenCreated = serde_json::from_slice(&body).unwrap();
    // A read token reads, and writes nothing.
    assert_eq!(
        send(&app, get("/v1/me", &read.token)).await.0,
        StatusCode::OK
    );
    let (status, _, body) = send(
        &app,
        json(
            "POST",
            "/v1/me/notifications/read",
            Some(&read.token),
            serde_json::json!({ "all": true }),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "{}",
        String::from_utf8_lossy(&body)
    );
    // A write token writes, but makes no token and changes no password.
    let (status, _, _) = send(
        &app,
        json(
            "POST",
            "/v1/me/notifications/read",
            Some(&write.token),
            serde_json::json!({ "all": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(
        &app,
        json(
            "POST",
            "/v1/me/tokens",
            Some(&write.token),
            serde_json::json!({ "name": "bir daha", "scope": "write", "expiresInDays": 1 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = send(
        &app,
        json(
            "POST",
            "/v1/portal/me/password",
            Some(&write.token),
            serde_json::json!({ "current": "dogru-parola-1", "new": "baska-parola-123" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Revoked: refused at once; a token of another shape is no one's.
    let (status, _, _) = send(
        &app,
        Request::delete(format!("/v1/me/tokens/{}", write.view.id))
            .header(header::AUTHORIZATION, format!("Bearer {session}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        send(&app, get("/v1/me", &write.token)).await.0,
        StatusCode::UNAUTHORIZED
    );
    let forged = format!("kentos_pat_{}", "0".repeat(64));
    assert_eq!(
        send(&app, get("/v1/me", &forged)).await.0,
        StatusCode::UNAUTHORIZED
    );
}
