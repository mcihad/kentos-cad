//! The project WebSocket over a real TCP socket, with a throwaway database:
//! a client whose cursor is older than the events still kept is told to
//! reopen (`resyncRequired`), and one at the horizon gets the rest; a
//! subscriber whose grant is taken away is cut off at once (TODOS.md
//! CLOUD-13). The test speaks the few parts of RFC 6455 it needs itself (the
//! upgrade, masked text frames out, plain text frames in).

use std::net::SocketAddr;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use kentos_application::{admin, events};
use kentos_contracts::{ApiError, GrantRole, TenantRole};
use kentos_postgres::testing::TestDb;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::tests::{access_command, app, json_req, send, signed_in};

struct Socket {
    stream: TcpStream,
}

impl Socket {
    /// Opens `/v1/ws` as the app's page would: its Origin and the session cookie.
    async fn open(addr: SocketAddr, cookie: &str) -> Socket {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let upgrade = format!(
            "GET /v1/ws HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\
             Origin: http://app.test\r\nCookie: {cookie}\r\n\r\n"
        );
        stream.write_all(upgrade.as_bytes()).await.unwrap();
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).await.unwrap();
            head.push(byte[0]);
        }
        let head = String::from_utf8(head).unwrap();
        assert!(head.starts_with("HTTP/1.1 101"), "{head}");
        Socket { stream }
    }

    /// One text frame; a client's frames are masked.
    async fn send(&mut self, message: serde_json::Value) {
        let payload = message.to_string().into_bytes();
        let mask = [0x37u8, 0xfa, 0x21, 0x3d];
        let mut frame = vec![0x81];
        if payload.len() < 126 {
            frame.push(0x80 | payload.len() as u8);
        } else {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        frame.extend_from_slice(&mask);
        frame.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
        self.stream.write_all(&frame).await.unwrap();
    }

    /// The next text message from the server (whose frames are not masked).
    async fn next(&mut self) -> serde_json::Value {
        let read = async {
            loop {
                let mut h = [0u8; 2];
                self.stream.read_exact(&mut h).await.unwrap();
                let mut len = u64::from(h[1] & 0x7f);
                if len == 126 {
                    let mut b = [0u8; 2];
                    self.stream.read_exact(&mut b).await.unwrap();
                    len = u64::from(u16::from_be_bytes(b));
                } else if len == 127 {
                    let mut b = [0u8; 8];
                    self.stream.read_exact(&mut b).await.unwrap();
                    len = u64::from_be_bytes(b);
                }
                let mut payload = vec![0; len as usize];
                self.stream.read_exact(&mut payload).await.unwrap();
                if h[0] & 0x0f == 0x1 {
                    return serde_json::from_slice(&payload).unwrap();
                }
            }
        };
        tokio::time::timeout(Duration::from_secs(10), read)
            .await
            .expect("the server said nothing for 10 s")
    }
}

#[tokio::test]
async fn a_cursor_older_than_the_kept_events_must_reopen() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    admin::create_local_user(&db.owner, "ayse", "ayse", None, "dogru-parola-1")
        .await
        .unwrap();
    admin::set_membership(&db.owner, "buro", "ayse", TenantRole::ProjectManager, true)
        .await
        .unwrap();
    let router = app(Some(db.app.clone()));
    let ayse = signed_in(&router, "ayse").await;
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": "Ada 101", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] } });
    let base = format!("/v1/tenants/{tenant}/projects");
    let (_, _, body) = send(&router, json_req("POST", &base, &ayse, create)).await;
    let project = serde_json::from_slice::<kentos_contracts::ProjectInfo>(&body)
        .unwrap()
        .id;
    let mut seqs = Vec::new();
    for x in [1.0, 2.0, 3.0] {
        let command = serde_json::json!({
            "commandName": "project.changes", "version": 1, "tenantId": tenant.to_string(), "projectId": project,
            "requestId": "istek", "idempotencyKey": uuid::Uuid::new_v4().to_string(), "expectedVersions": {},
            "input": { "features": [{ "op": "create", "id": uuid::Uuid::new_v4().to_string(),
                "entity": { "kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": { "x": x, "y": 2.0 } } }] } });
        let (_, _, body) = send(
            &router,
            json_req(
                "POST",
                &format!("{base}/{project}/commands"),
                &ayse,
                command,
            ),
        )
        .await;
        let done: kentos_contracts::CommitResult = serde_json::from_slice(&body).unwrap();
        seqs.push(done.event_seq.parse::<i64>().unwrap());
    }
    // The first two events are older than the week that is kept; pruning (as `kentosd serve` does hourly) removes them.
    sqlx::query(
        "update kentos.outbox_event set created_at = now() - interval '8 days' where seq <= $1",
    )
    .bind(seqs[1])
    .execute(&db.owner)
    .await
    .unwrap();
    let week = Duration::from_secs(7 * 24 * 3600);
    assert_eq!(events::prune(&db.app, week, 100).await.unwrap(), 2);

    // Over HTTP: 410 resync_required before the horizon, the rest from it.
    let log = |after: i64| {
        Request::get(format!("{base}/{project}/events?after={after}"))
            .header(header::COOKIE, &ayse)
            .body(Body::empty())
            .unwrap()
    };
    let (status, _, body) = send(&router, log(0)).await;
    assert_eq!(status, StatusCode::GONE);
    assert_eq!(
        serde_json::from_slice::<ApiError>(&body).unwrap().error,
        "resync_required"
    );
    let (status, _, body) = send(&router, log(seqs[1])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<kentos_contracts::EventPage>(&body)
            .unwrap()
            .events
            .len(),
        1
    );

    // Over the WebSocket: the same answers, live.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let mut ws = Socket::open(addr, &ayse).await;
    let subscribe = |after: i64| serde_json::json!({ "type": "subscribe", "tenantId": tenant.to_string(), "projectId": project, "after": after.to_string() });
    ws.send(subscribe(seqs[0])).await;
    assert_eq!(
        ws.next().await,
        serde_json::json!({ "type": "resyncRequired", "projectId": project })
    );
    ws.send(subscribe(seqs[1])).await;
    let subscribed = ws.next().await;
    assert_eq!(
        (subscribed["type"].as_str(), subscribed["after"].as_str()),
        (Some("subscribed"), Some(seqs[1].to_string().as_str()))
    );
    let replay = ws.next().await;
    assert_eq!(replay["type"], "events");
    assert_eq!(replay["events"][0]["seq"], seqs[2].to_string());
    // A cursor beyond anything the server has (a restored database) is sent to reopen too.
    ws.send(subscribe(seqs[2] + 1000)).await;
    assert_eq!(ws.next().await["type"], "resyncRequired");
    server.abort();
    db.close().await;
}

#[tokio::test]
async fn taking_a_grant_away_ends_the_live_subscription() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    for (login, role) in [
        ("ayse", TenantRole::ProjectManager),
        ("bora", TenantRole::Editor),
    ] {
        admin::create_local_user(&db.owner, login, login, None, "dogru-parola-1")
            .await
            .unwrap();
        admin::set_membership(&db.owner, "buro", login, role, true)
            .await
            .unwrap();
    }
    let router = app(Some(db.app.clone()));
    let (ayse, bora) = (
        signed_in(&router, "ayse").await,
        signed_in(&router, "bora").await,
    );
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": "Ada 101", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] } });
    let base = format!("/v1/tenants/{tenant}/projects");
    let (_, _, body) = send(&router, json_req("POST", &base, &ayse, create)).await;
    let project = serde_json::from_slice::<kentos_contracts::ProjectInfo>(&body)
        .unwrap()
        .id;
    let point = |x: f64| {
        json_req(
            "POST",
            &format!("{base}/{project}/commands"),
            &ayse,
            serde_json::json!({
                "commandName": "project.changes", "version": 1, "tenantId": tenant.to_string(), "projectId": project,
                "requestId": "istek", "idempotencyKey": uuid::Uuid::new_v4().to_string(), "expectedVersions": {},
                "input": { "features": [{ "op": "create", "id": uuid::Uuid::new_v4().to_string(),
                    "entity": { "kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": { "x": x, "y": 2.0 } } }] } }),
        )
    };
    let bora_id = admin::user_id(&db.owner, "bora").await.unwrap();
    let grant = |role: Option<GrantRole>| {
        access_command(&tenant.to_string(), &project, &ayse, bora_id, role)
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn({
        let router = router.clone();
        async move { axum::serve(listener, router).await }
    });
    let mut ws = Socket::open(addr, &bora).await;
    let subscribe = serde_json::json!({ "type": "subscribe", "tenantId": tenant.to_string(), "projectId": project, "after": "0" });
    // Not shared yet: the same answer as a project that does not exist.
    ws.send(subscribe.clone()).await;
    let refused = ws.next().await;
    assert_eq!(
        (refused["type"].as_str(), refused["error"].as_str()),
        (Some("error"), Some("not_found"))
    );

    // Shared: he follows it live.
    let (status, _, _) = send(&router, grant(Some(GrantRole::Editor))).await;
    assert_eq!(status, StatusCode::OK);
    ws.send(subscribe).await;
    assert_eq!(ws.next().await["type"], "subscribed");
    let replay = ws.next().await;
    assert_eq!(replay["events"][0]["kind"], "project.access");
    let (status, _, _) = send(&router, point(1.0)).await;
    assert_eq!(status, StatusCode::OK);
    let live = ws.next().await;
    assert_eq!(
        (live["type"].as_str(), live["events"][0]["kind"].as_str()),
        (Some("events"), Some("project.changes"))
    );

    // Taken away while the socket is open: he is told at once, and nothing of the project follows.
    let (status, _, _) = send(&router, grant(None)).await;
    assert_eq!(status, StatusCode::OK);
    let cut = ws.next().await;
    assert_eq!(
        (
            cut["type"].as_str(),
            cut["error"].as_str(),
            cut["message"].as_str()
        ),
        (Some("error"), Some("not_found"), Some("Proje bulunamadı."))
    );
    let (status, _, _) = send(&router, point(2.0)).await;
    assert_eq!(status, StatusCode::OK);
    // The next thing he hears is the answer to his heartbeat: no event slipped in between.
    ws.send(serde_json::json!({ "type": "ping", "t": 7.0 }))
        .await;
    assert_eq!(
        ws.next().await,
        serde_json::json!({ "type": "pong", "t": 7.0 })
    );
    // Over HTTP the same.
    let (status, _, _) = send(
        &router,
        Request::get(format!("{base}/{project}/events?after=0"))
            .header(header::COOKIE, &bora)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    server.abort();
    db.close().await;
}
