//! Invitations over HTTP (docs/adr/0035): made through the project's command
//! route, accepted through `POST /v1/invitations/accept`, listed for those
//! who share; a guest opens the project, a stranger finds the list missing.

use axum::http::StatusCode;
use kentos_application::admin;
use kentos_contracts::{
    CommandEnvelope, InvitationAccepted, InvitationChange, InvitationState, ProjectInfo,
    ProjectInvitations, TenantRole,
};
use kentos_postgres::testing::TestDb;

use super::tests::{app, get, json_req, not_found_body, send, signed_in};

#[tokio::test]
async fn an_invitation_is_made_accepted_and_listed_over_http() {
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
    for (login, email, t, role) in [
        ("ayse", "ayse@buro.org", "buro", TenantRole::ProjectManager),
        ("can", "can@diger.org", "diger", TenantRole::Owner),
    ] {
        admin::create_local_user(&db.owner, login, login, Some(email), "dogru-parola-1")
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
    let create = serde_json::json!({ "name": "Ada 9", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
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

    // Before any invitation, the outsider finds nothing.
    let missing = not_found_body(
        &router,
        get(
            &format!("/v1/tenants/{tenant}/projects/{}", uuid::Uuid::now_v7()),
            &can,
        ),
    )
    .await;
    assert_eq!(not_found_body(&router, get(&base, &can)).await, missing);
    assert_eq!(
        not_found_body(&router, get(&format!("{base}/invitations"), &can)).await,
        missing
    );

    let envelope = CommandEnvelope {
        command_name: "project.invite".into(),
        version: 1,
        tenant_id: tenant.clone(),
        project_id: project.clone(),
        request_id: "istek-davet-1".into(),
        idempotency_key: uuid::Uuid::now_v7().to_string(),
        expected_versions: Default::default(),
        input: serde_json::json!({ "email": "can@diger.org", "role": "viewer" }),
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
    let token = serde_json::from_slice::<InvitationChange>(&body)
        .unwrap()
        .token
        .unwrap();

    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            "/v1/invitations/accept",
            &can,
            serde_json::json!({ "token": token }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let accepted: InvitationAccepted = serde_json::from_slice(&body).unwrap();
    assert!(accepted.guest && accepted.project_id == project);
    // The guest opens it now; the link does not work twice.
    let (status, _, _) = send(&router, get(&base, &can)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(
        &router,
        json_req(
            "POST",
            "/v1/invitations/accept",
            &can,
            serde_json::json!({ "token": token }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, _, body) = send(&router, get(&format!("{base}/invitations"), &ayse)).await;
    let listed: ProjectInvitations = serde_json::from_slice(&body).unwrap();
    assert_eq!(listed.invitations[0].state, InvitationState::Accepted);
    db.close().await;
}
