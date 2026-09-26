//! File projects over HTTP (docs/adr/0031): an upload's bytes stream in and
//! a revision's stream out through the real router (its own upload limits);
//! a database project downloads as one `.kcad` of one moment (docs/adr/0033);
//! a stranger finds every route of the project missing alike.

use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::{Body, Bytes, HttpBody};
use axum::http::{Request, StatusCode, header};
use http_body::Frame;
use kentos_application::admin;
use kentos_contracts::{
    ApiError, CommandEnvelope, FileCommitted, FileRevisions, FileUpload, ProjectInfo, TenantRole,
};
use kentos_postgres::testing::TestDb;
use sha2::{Digest, Sha256};

use super::tests::{app, get, json_req, not_found_body, send, signed_in};

const MINIMAL: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/minimal.kcad");

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn put_bytes(uri: &str, cookie: &str, bytes: &[u8]) -> Request<Body> {
    put_body(uri, cookie, Body::from(bytes.to_vec()))
}

fn put_body(uri: &str, cookie: &str, body: Body) -> Request<Body> {
    Request::put(uri)
        .header(header::COOKIE, cookie)
        .header("x-kentos-client", "web")
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(body)
        .unwrap()
}

/// A body that sends its bytes and then breaks off, as a client that goes away.
struct BrokenBody(Option<Bytes>);

impl HttpBody for BrokenBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, std::io::Error>>> {
        Poll::Ready(Some(match self.get_mut().0.take() {
            Some(bytes) => Ok(Frame::data(bytes)),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "bağlantı koptu",
            )),
        }))
    }
}

/// Two organisations: `ayse` manages projects in the first, `can` owns the second; both signed in.
async fn two_offices(db: &TestDb) -> (axum::Router, uuid::Uuid, String, String) {
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 2)
        .await
        .unwrap();
    for (login, t, role) in [
        ("ayse", "buro", TenantRole::ProjectManager),
        ("can", "diger", TenantRole::Owner),
    ] {
        admin::create_local_user(&db.owner, login, login, None, "dogru-parola-1")
            .await
            .unwrap();
        admin::set_membership(&db.owner, t, login, role, true)
            .await
            .unwrap();
    }
    let router = app(Some(db.app.clone()));
    let (ayse, can) = (
        signed_in(&router, "ayse").await,
        signed_in(&router, "can").await,
    );
    (router, tenant, ayse, can)
}

/// A new project with one layer, `cizim`, kept as `storage` ("database" or "file"); its id.
async fn new_project(
    router: &axum::Router,
    tenant: uuid::Uuid,
    cookie: &str,
    name: &str,
    storage: &str,
) -> String {
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": name, "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] },
        "storage": storage });
    let (status, _, body) = send(
        router,
        json_req(
            "POST",
            &format!("/v1/tenants/{tenant}/projects"),
            cookie,
            create,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    serde_json::from_slice::<ProjectInfo>(&body).unwrap().id
}

#[tokio::test]
async fn a_revision_goes_up_and_comes_down_over_http() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (router, tenant, ayse, can) = two_offices(&db).await;
    let project = new_project(&router, tenant, &ayse, "Ada 7 dosyası", "file").await;
    let base = format!("/v1/tenants/{tenant}/projects/{project}");

    // Open an upload, send its bytes in one streamed body, commit it as revision 1.
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("{base}/uploads"),
            &ayse,
            serde_json::json!({ "size": MINIMAL.len(), "sha256": sha(MINIMAL) }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let upload: FileUpload = serde_json::from_slice(&body).unwrap();
    let (status, _, body) = send(
        &router,
        put_bytes(&format!("{base}/uploads/{}", upload.id), &ayse, MINIMAL),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(
        serde_json::from_slice::<FileUpload>(&body)
            .unwrap()
            .received
    );
    let envelope = CommandEnvelope {
        command_name: "project.file.commit".into(),
        version: 1,
        tenant_id: tenant.to_string(),
        project_id: project.clone(),
        request_id: "istek-dosya-1".into(),
        idempotency_key: uuid::Uuid::now_v7().to_string(),
        expected_versions: [("@file".to_string(), "0".to_string())]
            .into_iter()
            .collect(),
        input: serde_json::json!({ "uploadId": upload.id }),
    };
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("{base}/commands"),
            &ayse,
            serde_json::to_value(&envelope).unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(
        serde_json::from_slice::<FileCommitted>(&body)
            .unwrap()
            .revision,
        "1"
    );

    let (_, _, body) = send(&router, get(&format!("{base}/files"), &ayse)).await;
    let listed: FileRevisions = serde_json::from_slice(&body).unwrap();
    assert_eq!(listed.current.as_deref(), Some("1"));
    let (status, headers, body) = send(&router, get(&format!("{base}/files/1"), &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, MINIMAL);
    assert_eq!(
        headers[header::ETAG].to_str().unwrap(),
        format!("\"{}\"", sha(MINIMAL))
    );
    assert_eq!(headers["x-kentos-revision"].to_str().unwrap(), "1");
    assert!(
        headers[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .contains("filename*=UTF-8''Ada%207%20dosyas%C4%B1-r1.kcad")
    );

    // More bytes than declared: refused, and the field is named.
    let (_, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("{base}/uploads"),
            &ayse,
            serde_json::json!({ "size": 16, "sha256": sha(MINIMAL) }),
        ),
    )
    .await;
    let small: FileUpload = serde_json::from_slice(&body).unwrap();
    let (status, _, body) = send(
        &router,
        put_bytes(&format!("{base}/uploads/{}", small.id), &ayse, MINIMAL),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        serde_json::from_slice::<ApiError>(&body)
            .unwrap()
            .path
            .as_deref(),
        Some("size")
    );

    // The body breaks off: nothing is kept, and the same upload is sent again.
    let (_, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("{base}/uploads"),
            &ayse,
            serde_json::json!({ "size": MINIMAL.len(), "sha256": sha(MINIMAL) }),
        ),
    )
    .await;
    let broken: FileUpload = serde_json::from_slice(&body).unwrap();
    let uri = format!("{base}/uploads/{}", broken.id);
    let part = Body::new(BrokenBody(Some(Bytes::from_static(&MINIMAL[..10]))));
    let (status, _, body) = send(&router, put_body(&uri, &ayse, part)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let said = serde_json::from_slice::<ApiError>(&body).unwrap().message;
    assert!(said.contains("yarıda kesildi"), "{said}");
    let (status, _, _) = send(&router, put_bytes(&uri, &ayse, MINIMAL)).await;
    assert_eq!(status, StatusCode::OK);

    // A file project is its revisions already: no snapshot of it.
    let (status, _, _) = send(&router, get(&format!("{base}/snapshot"), &ayse)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Someone of another organisation finds nothing: every route answers as for no project.
    let missing = not_found_body(
        &router,
        get(
            &format!("/v1/tenants/{tenant}/projects/{}", uuid::Uuid::now_v7()),
            &can,
        ),
    )
    .await;
    for req in [
        get(&format!("{base}/files"), &can),
        get(&format!("{base}/files/1"), &can),
        get(&format!("{base}/snapshot"), &can),
        json_req(
            "POST",
            &format!("{base}/uploads"),
            &can,
            serde_json::json!({ "size": 3, "sha256": sha(b"abc") }),
        ),
        put_bytes(&format!("{base}/uploads/{}", upload.id), &can, b"abc"),
    ] {
        assert_eq!(not_found_body(&router, req).await, missing);
    }
    db.close().await;
}

#[tokio::test]
async fn a_database_project_downloads_as_one_kcad_file() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (router, tenant, ayse, can) = two_offices(&db).await;
    let project = new_project(&router, tenant, &ayse, "Ada 8", "database").await;
    let base = format!("/v1/tenants/{tenant}/projects/{project}");
    let id = uuid::Uuid::now_v7().to_string();
    let point = serde_json::json!({ "kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": { "x": 486510.0, "y": 4420210.0 } });
    let envelope = CommandEnvelope {
        command_name: "project.changes".into(),
        version: 1,
        tenant_id: tenant.to_string(),
        project_id: project.clone(),
        request_id: "istek-nesne-1".into(),
        idempotency_key: uuid::Uuid::now_v7().to_string(),
        expected_versions: Default::default(),
        input: serde_json::json!({ "features": [{ "op": "create", "id": id, "entity": point }] }),
    };
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("{base}/commands"),
            &ayse,
            serde_json::to_value(&envelope).unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    let (_, _, body) = send(&router, get(&base, &ayse)).await;
    let info: ProjectInfo = serde_json::from_slice(&body).unwrap();
    let (status, headers, body) = send(&router, get(&format!("{base}/snapshot"), &ayse)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let doc = kentos_kcad::decode(&body).unwrap();
    assert_eq!(doc.name, "Ada 8");
    assert_eq!(
        doc.uids.iter().map(|u| u.to_text()).collect::<Vec<_>>(),
        [id]
    );
    assert_eq!(
        headers[header::ETAG].to_str().unwrap(),
        format!("\"{}\"", sha(&body))
    );
    assert_eq!(
        headers["x-kentos-revision"].to_str().unwrap(),
        info.data_revision
    );
    assert_eq!(
        headers["x-kentos-event-cursor"].to_str().unwrap(),
        info.event_cursor
    );
    let disposition = headers[header::CONTENT_DISPOSITION].to_str().unwrap();
    let file = format!("filename*=UTF-8''Ada%208-r{}.kcad", info.data_revision);
    assert!(disposition.contains(&file), "{disposition}");

    // Someone of another organisation finds nothing, as for no project.
    let missing = not_found_body(
        &router,
        get(
            &format!("/v1/tenants/{tenant}/projects/{}", uuid::Uuid::now_v7()),
            &can,
        ),
    )
    .await;
    assert_eq!(
        not_found_body(&router, get(&format!("{base}/snapshot"), &can)).await,
        missing
    );
    db.close().await;
}
