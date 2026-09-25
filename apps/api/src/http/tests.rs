//! The router end to end (middleware included), with a throwaway database.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use kentos_application::admin;
use kentos_contracts::{ApiError, GrantRole, Health, Me, ProjectRole, TenantKind, TenantRole};
use kentos_postgres::testing::TestDb;
use tower::ServiceExt;

use super::*;
use crate::config::Config;

fn config() -> Config {
    Config {
        env_file: ".env.test".into(),
        vars: Default::default(),
        database_url: None,
        owner_url: None,
        bind: "127.0.0.1".into(),
        port: 0,
        public_url: "http://app.test".into(),
        cookie_secure: false,
        local_login: true,
        oidc: None,
        event_retention: std::time::Duration::from_secs(7 * 24 * 3600),
    }
}

pub(super) fn app(database: Option<Db>) -> Router {
    router(AppState {
        config: Arc::new(config()),
        database,
        oidc: None,
        hub: crate::hub::Hub::default(),
        logins: Default::default(),
    })
}

pub(super) async fn send(
    app: &Router,
    req: Request<Body>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = app.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    (
        parts.status,
        parts.headers,
        axum::body::to_bytes(body, usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
}

fn login_request(login: &str, password: &str, client_header: bool) -> Request<Body> {
    let mut b = Request::post("/v1/auth/login").header(header::CONTENT_TYPE, "application/json");
    if client_header {
        b = b.header("x-kentos-client", "web");
    }
    b.body(Body::from(
        serde_json::json!({ "login": login, "password": password }).to_string(),
    ))
    .unwrap()
}

#[tokio::test]
async fn health_answers_without_a_database_and_carries_a_request_id() {
    let app = app(None);
    let (status, headers, body) = send(
        &app,
        Request::get("/v1/health").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let h: Health = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        (h.status.as_str(), h.contracts),
        ("ok", kentos_contracts::CONTRACTS_VERSION)
    );
    assert!(headers.contains_key("x-request-id"));
    // Everything else says the database is missing, as JSON.
    let (status, _, body) = send(&app, Request::get("/v1/me").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        serde_json::from_slice::<ApiError>(&body).unwrap().error,
        "unavailable"
    );
    let (status, _, _) = send(&app, Request::get("/v1/yok").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn local_sign_in_with_a_session_cookie() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    admin::create_local_user(&db.owner, "ayse", "Ayşe Yılmaz", None, "dogru-parola-1")
        .await
        .unwrap();
    admin::set_membership(&db.owner, "buro", "ayse", TenantRole::Editor, true)
        .await
        .unwrap();
    let app = app(Some(db.app.clone()));

    // Without the app's header the request is refused (cross-site request forgery).
    let (status, _, _) = send(&app, login_request("ayse", "dogru-parola-1", false)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, body) = send(&app, login_request("ayse", "yanlis-parola", true)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let err: ApiError = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        (err.error.as_str(), err.message.as_str()),
        ("unauthenticated", "Giriş adı ya da parola yanlış.")
    );
    assert!(err.request_id.is_some());
    let (status, _, body) = send(
        &app,
        Request::post("/v1/auth/login")
            .header("x-kentos-client", "web")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{bozuk"))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        serde_json::from_slice::<ApiError>(&body).unwrap().error,
        "invalid"
    );

    let (status, headers, body) = send(&app, login_request("ayse", "dogru-parola-1", true)).await;
    assert_eq!(status, StatusCode::OK);
    let set = headers
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        set.starts_with("kentos_session=")
            && set.contains("HttpOnly")
            && set.contains("SameSite=Strict")
    );
    let me: Me = serde_json::from_slice(&body).unwrap();
    assert_eq!(me.user.display_name, "Ayşe Yılmaz");
    assert_eq!(me.memberships[0].tenant_slug, "buro");
    // The first sign-in opened her personal space; it comes after the organisations.
    assert_eq!(me.memberships.len(), 2);
    assert_eq!(
        (me.memberships[1].tenant_kind, me.memberships[1].role),
        (TenantKind::Personal, TenantRole::Owner)
    );
    let cookie = set.split(';').next().unwrap().to_string();

    let (status, _, body) = send(
        &app,
        Request::get("/v1/me")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<Me>(&body).unwrap().user.id,
        me.user.id
    );

    // Signing out also needs the header; afterwards the cookie is worthless.
    let (status, _, _) = send(
        &app,
        Request::post("/v1/auth/logout")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, headers, _) = send(
        &app,
        Request::post("/v1/auth/logout")
            .header(header::COOKIE, &cookie)
            .header("x-kentos-client", "web")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(
        headers
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    let (status, _, _) = send(
        &app,
        Request::get("/v1/me")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    db.close().await;
}

pub(super) async fn signed_in(app: &Router, login: &str) -> String {
    let (status, headers, _) = send(app, login_request(login, "dogru-parola-1", true)).await;
    assert_eq!(status, StatusCode::OK, "{login} giriş yapamadı");
    headers
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

pub(super) fn json_req(
    method: &str,
    uri: &str,
    cookie: &str,
    body: serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .header("x-kentos-client", "web")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub(super) fn get(uri: &str, cookie: &str) -> Request<Body> {
    Request::get(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

/// A `project.share` (with a role) or `project.access.revoke` (without) command for `user`.
pub(super) fn access_command(
    tenant: &str,
    project: &str,
    cookie: &str,
    user: uuid::Uuid,
    role: Option<GrantRole>,
) -> Request<Body> {
    let (name, input) = match role {
        Some(role) => (
            "project.share",
            serde_json::json!({ "userId": user.to_string(), "role": role }),
        ),
        None => (
            "project.access.revoke",
            serde_json::json!({ "userId": user.to_string() }),
        ),
    };
    json_req(
        "POST",
        &format!("/v1/tenants/{tenant}/projects/{project}/commands"),
        cookie,
        serde_json::json!({
            "commandName": name, "version": 1, "tenantId": tenant, "projectId": project,
            "requestId": "paylasim", "idempotencyKey": uuid::Uuid::new_v4().to_string(), "expectedVersions": {},
            "input": input }),
    )
}

/// The `error` and `message` of a 404: what a caller learns about a project they may not see.
pub(super) async fn not_found_body(app: &Router, req: Request<Body>) -> (String, String) {
    let (status, _, body) = send(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let e: ApiError = serde_json::from_slice(&body).unwrap();
    (e.error, e.message)
}

#[tokio::test]
async fn projects_commands_and_events_over_http() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let other = admin::create_tenant(&db.owner, "diger", "Diğer", 3)
        .await
        .unwrap();
    for (login, t, role) in [
        ("ayse", "buro", TenantRole::ProjectManager),
        ("bora", "buro", TenantRole::Editor),
        ("dilek", "buro", TenantRole::ProjectManager),
        ("can", "diger", TenantRole::Owner),
    ] {
        admin::create_local_user(&db.owner, login, login, None, "dogru-parola-1")
            .await
            .unwrap();
        admin::set_membership(&db.owner, t, login, role, true)
            .await
            .unwrap();
    }
    let app = app(Some(db.app.clone()));
    let (ayse, bora, dilek, can) = (
        signed_in(&app, "ayse").await,
        signed_in(&app, "bora").await,
        signed_in(&app, "dilek").await,
        signed_in(&app, "can").await,
    );
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": "Ada 101", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] } });
    let base = format!("/v1/tenants/{tenant}/projects");
    let mut req = json_req("POST", &base, &ayse, create.clone());
    req.headers_mut()
        .insert("idempotency-key", "proje-olustur-1".parse().unwrap());
    let (status, _, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    let info: kentos_contracts::ProjectInfo = serde_json::from_slice(&body).unwrap();
    // A retried create with the same key gives the same project, not a second one.
    let mut again = json_req("POST", &base, &ayse, create.clone());
    again
        .headers_mut()
        .insert("idempotency-key", "proje-olustur-1".parse().unwrap());
    let (_, _, body) = send(&app, again).await;
    assert_eq!(
        serde_json::from_slice::<kentos_contracts::ProjectInfo>(&body)
            .unwrap()
            .id,
        info.id
    );
    // An editor cannot create projects.
    let (status, _, _) = send(&app, json_req("POST", &base, &bora, create)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let project = &info.id;
    let fid = uuid::Uuid::new_v4().to_string();
    let command = |cookie: &str, x: f64, op: &str, expected: serde_json::Value| {
        let entity = serde_json::json!({ "kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": { "x": x, "y": 4420210.0 } });
        let change = if op == "create" || op == "update" {
            serde_json::json!({ "op": op, "id": fid, "entity": entity })
        } else {
            serde_json::json!({ "op": op, "id": fid })
        };
        json_req(
            "POST",
            &format!("{base}/{project}/commands"),
            cookie,
            serde_json::json!({
            "commandName": "project.changes", "version": 1, "tenantId": tenant.to_string(), "projectId": project,
            "requestId": "istek-1", "idempotencyKey": uuid::Uuid::new_v4().to_string(), "expectedVersions": expected,
            "input": { "features": [change] } }),
        )
    };
    let (status, _, body) = send(&app, command(&ayse, 1.0, "create", serde_json::json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    // The project is ayse's: bora (a member of the same tenant) cannot write to it, or even find it, until she shares it.
    let (status, _, _) = send(
        &app,
        command(
            &bora,
            2.0,
            "update",
            serde_json::json!({ fid.clone(): "1" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let bora_id = admin::user_id(&db.owner, "bora").await.unwrap();
    let (status, _, body) = send(
        &app,
        access_command(
            &tenant.to_string(),
            project,
            &ayse,
            bora_id,
            Some(GrantRole::Editor),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let shared: kentos_contracts::ProjectAccessChange = serde_json::from_slice(&body).unwrap();
    assert!(shared.changed && shared.grant.unwrap().role == GrantRole::Editor);
    let (status, _, _) = send(
        &app,
        command(
            &bora,
            2.0,
            "update",
            serde_json::json!({ fid.clone(): "1" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, body) = send(
        &app,
        command(
            &ayse,
            3.0,
            "update",
            serde_json::json!({ fid.clone(): "1" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let err: ApiError = serde_json::from_slice(&body).unwrap();
    assert_eq!(err.error, "conflict");
    assert_eq!(err.conflicts.unwrap()[0].actual.as_deref(), Some("2"));

    let (status, _, body) = send(
        &app,
        Request::get(format!("{base}/{project}/features"))
            .header(header::COOKIE, &ayse)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let page: kentos_contracts::FeaturePage = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        (page.features.len(), page.features[0].version.as_str()),
        (1, "2")
    );
    let (_, _, body) = send(
        &app,
        Request::get(format!("{base}/{project}/events?after=0"))
            .header(header::COOKIE, &bora)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let log: kentos_contracts::EventPage = serde_json::from_slice(&body).unwrap();
    let kinds: Vec<&str> = log.events.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(
        kinds,
        ["project.changes", "project.access", "project.changes"]
    );
    // What each may do comes with the project.
    let (_, _, body) = send(&app, get(&format!("{base}/{project}"), &bora)).await;
    let seen: kentos_contracts::ProjectInfo = serde_json::from_slice(&body).unwrap();
    assert_eq!(seen.access.role, ProjectRole::Editor);
    assert!(
        !seen
            .access
            .permissions
            .contains(&kentos_contracts::ProjectPermission::Edit)
    );

    // Another tenant's member: every route is 404, even with the right ids; a wrong tenant in the body is refused.
    for uri in [
        base.clone(),
        format!("{base}/{project}"),
        format!("{base}/{project}/features"),
        format!("{base}/{project}/events"),
        format!("{base}/{project}/access"),
    ] {
        let (status, _, _) = send(&app, get(&uri, &can)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
    // A member it was not shared with sees none of it: the list leaves it out, every route is 404.
    let (_, _, body) = send(&app, get(&base, &dilek)).await;
    assert!(
        serde_json::from_slice::<kentos_contracts::ProjectList>(&body)
            .unwrap()
            .projects
            .is_empty()
    );
    // Every 404 of a project reads the same: not shared, another tenant's, guessed or malformed.
    let reference = not_found_body(&app, get(&format!("{base}/{project}"), &dilek)).await;
    for (uri, cookie) in [
        (format!("{base}/{project}/features"), &dilek),
        (format!("{base}/{project}/events?after=0"), &dilek),
        (format!("{base}/{project}"), &can),
        (format!("/v1/tenants/{other}/projects/{project}"), &can),
        (format!("{base}/{}", uuid::Uuid::now_v7()), &ayse),
        (format!("{base}/bozuk"), &ayse),
        (format!("/v1/tenants/bozuk/projects/{project}"), &ayse),
    ] {
        assert_eq!(
            not_found_body(&app, get(&uri, cookie)).await,
            reference,
            "{uri}"
        );
    }
    assert_eq!(
        not_found_body(
            &app,
            command(
                &dilek,
                9.0,
                "update",
                serde_json::json!({ fid.clone(): "2" })
            )
        )
        .await,
        reference
    );
    let (status, _, _) = send(
        &app,
        Request::get(format!("/v1/tenants/{other}/projects/{project}"))
            .header(header::COOKIE, &can)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send(
        &app,
        Request::get("/v1/tenants/bozuk/projects")
            .header(header::COOKIE, &ayse)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Without the app header a command is refused even with a valid session.
    let mut no_header = command(
        &ayse,
        4.0,
        "update",
        serde_json::json!({ fid.clone(): "2" }),
    );
    no_header.headers_mut().remove("x-kentos-client");
    let (status, _, _) = send(&app, no_header).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    db.close().await;
}

fn delete_req(uri: &str, cookie: &str, client_header: bool) -> Request<Body> {
    let mut b = Request::delete(uri).header(header::COOKIE, cookie);
    if client_header {
        b = b.header("x-kentos-client", "web");
    }
    b.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn deleting_a_project_over_http() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let other = admin::create_tenant(&db.owner, "diger", "Diğer", 3)
        .await
        .unwrap();
    for (login, t, role) in [
        ("ayse", "buro", TenantRole::ProjectManager),
        ("zeynep", "buro", TenantRole::Admin),
        ("izzet", "buro", TenantRole::Viewer),
        ("can", "diger", TenantRole::Owner),
    ] {
        admin::create_local_user(&db.owner, login, login, None, "dogru-parola-1")
            .await
            .unwrap();
        admin::set_membership(&db.owner, t, login, role, true)
            .await
            .unwrap();
    }
    let app = app(Some(db.app.clone()));
    let (ayse, zeynep, izzet, can) = (
        signed_in(&app, "ayse").await,
        signed_in(&app, "zeynep").await,
        signed_in(&app, "izzet").await,
        signed_in(&app, "can").await,
    );
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": "Ada 101", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] } });
    let base = format!("/v1/tenants/{tenant}/projects");
    let (status, _, body) = send(&app, json_req("POST", &base, &ayse, create)).await;
    assert_eq!(status, StatusCode::CREATED);
    let project = serde_json::from_slice::<kentos_contracts::ProjectInfo>(&body)
        .unwrap()
        .id;
    let uri = format!("{base}/{project}");
    let command = serde_json::json!({
        "commandName": "project.changes", "version": 1, "tenantId": tenant.to_string(), "projectId": project,
        "requestId": "istek-1", "idempotencyKey": uuid::Uuid::new_v4().to_string(), "expectedVersions": {},
        "input": { "features": [{ "op": "create", "id": uuid::Uuid::new_v4().to_string(),
            "entity": { "kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": { "x": 1.0, "y": 2.0 } } }] } });

    // A member it was not shared with cannot find it; shared as a viewer, may not delete it; another
    // tenant's owner cannot find it; the app header is required.
    let (status, _, _) = send(&app, delete_req(&uri, &izzet, true)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let izzet_id = admin::user_id(&db.owner, "izzet").await.unwrap();
    let (status, _, _) = send(
        &app,
        access_command(
            &tenant.to_string(),
            &project,
            &ayse,
            izzet_id,
            Some(GrantRole::Viewer),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, body) = send(&app, delete_req(&uri, &izzet, true)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(
        serde_json::from_slice::<ApiError>(&body)
            .unwrap()
            .message
            .contains("project.delete")
    );
    for path in [
        uri.clone(),
        format!("/v1/tenants/{other}/projects/{project}"),
    ] {
        let (status, _, _) = send(&app, delete_req(&path, &can, true)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
    let (status, _, _) = send(&app, delete_req(&uri, &zeynep, false)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // The organisation's admin deletes it (the policy; its owner could too); asking again is the same answer.
    for _ in 0..2 {
        let (status, _, _) = send(&app, delete_req(&uri, &zeynep, true)).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    // Opening, reading and writing answer 410; the list leaves it out; the event log tells why.
    for req in [
        Request::get(&uri)
            .header(header::COOKIE, &ayse)
            .body(Body::empty())
            .unwrap(),
        Request::get(format!("{uri}/features"))
            .header(header::COOKIE, &ayse)
            .body(Body::empty())
            .unwrap(),
        json_req("POST", &format!("{uri}/commands"), &ayse, command),
    ] {
        let (status, _, body) = send(&app, req).await;
        assert_eq!(status, StatusCode::GONE);
        assert_eq!(
            serde_json::from_slice::<ApiError>(&body).unwrap().error,
            "project_deleted"
        );
    }
    let (_, _, body) = send(
        &app,
        Request::get(&base)
            .header(header::COOKIE, &izzet)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(
        serde_json::from_slice::<kentos_contracts::ProjectList>(&body)
            .unwrap()
            .projects
            .is_empty()
    );
    let (status, _, body) = send(
        &app,
        Request::get(format!("{uri}/events?after=0"))
            .header(header::COOKIE, &izzet)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let log: kentos_contracts::EventPage = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        log.events.last().unwrap().kind,
        kentos_contracts::PROJECT_DELETED
    );
    // To anyone who never had access it is simply not there: 404, not 410.
    let (status, _, _) = send(&app, get(&uri, &can)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    db.close().await;
}

#[tokio::test]
async fn the_personal_space_sharing_and_my_projects_over_http() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    for login in ["ayse", "bora"] {
        admin::create_local_user(&db.owner, login, login, None, "dogru-parola-1")
            .await
            .unwrap();
    }
    let app = app(Some(db.app.clone()));
    let (ayse, bora) = (signed_in(&app, "ayse").await, signed_in(&app, "bora").await);
    // No organisation: the sign-in opened a personal space, where she may create projects.
    let (_, _, body) = send(&app, get("/v1/me", &ayse)).await;
    let me: Me = serde_json::from_slice(&body).unwrap();
    assert_eq!(me.memberships.len(), 1);
    let space = &me.memberships[0];
    assert_eq!(
        (space.tenant_kind, space.capabilities.clone()),
        (TenantKind::Personal, vec!["project.create".to_string()])
    );
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": "Bahçe", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] } });
    let base = format!("/v1/tenants/{}/projects", space.tenant_id);
    let (status, _, body) = send(&app, json_req("POST", &base, &ayse, create)).await;
    assert_eq!(status, StatusCode::CREATED);
    let info: kentos_contracts::ProjectInfo = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        (info.tenant_kind, info.access.role, info.access.via),
        (
            TenantKind::Personal,
            ProjectRole::Owner,
            kentos_contracts::AccessSource::Owner
        )
    );
    let uri = format!("{base}/{}", info.id);
    // Bora is not a member of her space: he cannot list it, nor find the project.
    let (status, _, _) = send(&app, get(&base, &bora)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let missing = not_found_body(&app, get(&uri, &bora)).await;
    let (_, _, body) = send(&app, get("/v1/me/projects", &bora)).await;
    assert!(
        serde_json::from_slice::<kentos_contracts::ProjectList>(&body)
            .unwrap()
            .projects
            .is_empty()
    );
    // Shared with him: in his “Projelerim”, and open to him at his role.
    let bora_id = admin::user_id(&db.owner, "bora").await.unwrap();
    let (status, _, _) = send(
        &app,
        access_command(
            &space.tenant_id,
            &info.id,
            &ayse,
            bora_id,
            Some(GrantRole::Editor),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, _, body) = send(&app, get("/v1/me/projects", &bora)).await;
    let mine: kentos_contracts::ProjectList = serde_json::from_slice(&body).unwrap();
    assert_eq!(mine.projects.len(), 1);
    assert_eq!(
        (
            mine.projects[0].id.as_str(),
            mine.projects[0].tenant_id.as_str(),
            mine.projects[0].access.role
        ),
        (
            info.id.as_str(),
            space.tenant_id.as_str(),
            ProjectRole::Editor
        )
    );
    let (status, _, _) = send(&app, get(&uri, &bora)).await;
    assert_eq!(status, StatusCode::OK);
    // Who has access is shown to whoever may share (she may, he may not).
    let (status, _, body) = send(&app, get(&format!("{uri}/access"), &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    let list: kentos_contracts::ProjectAccessList = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        (
            list.owner_name.as_str(),
            list.people.len(),
            list.people[1].display_name.as_str(),
            list.people[1].grant
        ),
        ("ayse", 2, "bora", Some(GrantRole::Editor))
    );
    let (status, _, _) = send(&app, get(&format!("{uri}/access"), &bora)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Taken away: the same 404 as before it was shared.
    let (status, _, _) = send(
        &app,
        access_command(&space.tenant_id, &info.id, &ayse, bora_id, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(not_found_body(&app, get(&uri, &bora)).await, missing);
    db.close().await;
}

#[tokio::test]
async fn repeated_wrong_passwords_lock_the_login_for_a_while() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_local_user(&db.owner, "ayse", "Ayşe", None, "dogru-parola-1")
        .await
        .unwrap();
    admin::create_local_user(&db.owner, "bora", "Bora", None, "dogru-parola-1")
        .await
        .unwrap();
    let app = app(Some(db.app.clone()));
    for _ in 0..super::limit::MAX_FAILURES {
        let (status, _, _) = send(&app, login_request("ayse", "yanlis-parola", true)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    // Now even the right password waits, and the answer says for how long.
    let (status, headers, body) = send(&app, login_request("AYSE", "dogru-parola-1", true)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(headers.get(header::RETRY_AFTER).is_some());
    assert_eq!(
        serde_json::from_slice::<ApiError>(&body).unwrap().error,
        "rate_limited"
    );
    // Another login is not affected.
    let (status, _, _) = send(&app, login_request("bora", "dogru-parola-1", true)).await;
    assert_eq!(status, StatusCode::OK);
    db.close().await;
}
