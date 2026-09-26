//! Checkpoints over HTTP (docs/adr/0034): made through the project's command
//! route, listed and downloaded through their own; a stranger finds both
//! routes missing as for no project.

use axum::http::{StatusCode, header};
use kentos_application::admin;
use kentos_contracts::{
    CheckpointChange, CommandEnvelope, ProjectCheckpoints, ProjectInfo, TenantRole,
};
use kentos_postgres::testing::TestDb;
use sha2::{Digest, Sha256};

use super::tests::{app, get, json_req, not_found_body, send, signed_in};

fn envelope(
    tenant: &str,
    project: &str,
    name: &str,
    input: serde_json::Value,
) -> serde_json::Value {
    serde_json::to_value(CommandEnvelope {
        command_name: name.into(),
        version: 1,
        tenant_id: tenant.into(),
        project_id: project.into(),
        request_id: format!("istek-{}", uuid::Uuid::now_v7()),
        idempotency_key: uuid::Uuid::now_v7().to_string(),
        expected_versions: Default::default(),
        input,
    })
    .unwrap()
}

#[tokio::test]
async fn a_checkpoint_is_made_listed_and_downloaded_over_http() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap()
        .to_string();
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
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": "Ada 3", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] } });
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("/v1/tenants/{tenant}/projects"),
            &ayse,
            create,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let project = serde_json::from_slice::<ProjectInfo>(&body).unwrap().id;
    let base = format!("/v1/tenants/{tenant}/projects/{project}");

    let make = envelope(
        &tenant,
        &project,
        "project.checkpoint.create",
        serde_json::json!({ "name": "Teslim" }),
    );
    let (status, _, body) = send(
        &router,
        json_req("POST", &format!("{base}/commands"), &ayse, make),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let made: CheckpointChange = serde_json::from_slice(&body).unwrap();

    let (_, _, body) = send(&router, get(&format!("{base}/checkpoints"), &ayse)).await;
    let listed: ProjectCheckpoints = serde_json::from_slice(&body).unwrap();
    assert_eq!(listed.checkpoints, std::slice::from_ref(&made.checkpoint));
    let one = format!("{base}/checkpoints/{}", made.checkpoint.id);
    let (status, headers, body) = send(&router, get(&one, &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    let digest: String = Sha256::digest(&body)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(digest, made.checkpoint.sha256);
    assert_eq!(kentos_kcad::decode(&body).unwrap().name, "Ada 3");
    assert_eq!(
        headers["x-kentos-revision"].to_str().unwrap(),
        made.checkpoint.revision
    );
    assert!(
        headers[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .contains("filename*=UTF-8''Ada%203%20-%20Teslim.kcad")
    );

    // Someone of another organisation finds nothing, as for no project.
    let missing = not_found_body(
        &router,
        get(
            &format!("/v1/tenants/{tenant}/projects/{}", uuid::Uuid::now_v7()),
            &can,
        ),
    )
    .await;
    for uri in [format!("{base}/checkpoints"), one] {
        assert_eq!(not_found_body(&router, get(&uri, &can)).await, missing);
    }
    db.close().await;
}
