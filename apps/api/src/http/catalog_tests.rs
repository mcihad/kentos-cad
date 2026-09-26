//! The catalog's routes end to end, with a throwaway database
//! (docs/adr/0028, TODOS.md CLOUD-04, CLOUD-05, CLOUD-12): every lifecycle
//! command and the details answer 404 word for word as a project that does
//! not exist to anyone who may not see the project; a viewer archives and
//! deletes nothing; `project.create` goes to the workspace's route and a
//! retry answers the same project; the catalog's views are the caller's own;
//! an archived project refuses writing with 409.

use axum::http::StatusCode;
use kentos_application::admin;
use kentos_contracts::{
    ApiError, GrantRole, ProjectCatalogChange, ProjectDetails, ProjectInfo, ProjectPage,
    ProjectState, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;

use super::tests::{access_command, app, get, json_req, not_found_body, send, signed_in};

fn envelope(
    tenant: &str,
    project: &str,
    name: &str,
    input: serde_json::Value,
) -> serde_json::Value {
    json!({
        "commandName": name, "version": 1, "tenantId": tenant, "projectId": project,
        "requestId": format!("istek-{name}"), "idempotencyKey": uuid::Uuid::new_v4().to_string(),
        "expectedVersions": {}, "input": input
    })
}

#[tokio::test]
async fn catalog_routes_answer_404_alike_and_check_every_permission() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 6)
        .await
        .unwrap();
    let other = admin::create_tenant(&db.owner, "diger", "Diğer", 3)
        .await
        .unwrap();
    for (login, t, role) in [
        ("ayse", "buro", TenantRole::ProjectManager),
        ("dilek", "buro", TenantRole::Viewer),
        ("komsu", "buro", TenantRole::Editor),
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
    let (ayse, dilek, komsu, can) = (
        signed_in(&router, "ayse").await,
        signed_in(&router, "dilek").await,
        signed_in(&router, "komsu").await,
        signed_in(&router, "can").await,
    );
    let t = tenant.to_string();

    // project.create goes to the workspace's route (projectId empty): 201 with the project; a retry
    // with the same key answers the same one; a project's id there, or a project's command, is refused.
    let layer = json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = json!({ "name": "Ada 101", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] },
        "projectType": "zoningPlan", "tags": ["Kadıköy"] });
    let commands = format!("/v1/tenants/{t}/commands");
    let mut env = envelope(&t, "", "project.create", create.clone());
    let (status, _, body) = send(&router, json_req("POST", &commands, &ayse, env.clone())).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let made: ProjectInfo = serde_json::from_slice(&body).unwrap();
    let (_, _, body) = send(&router, json_req("POST", &commands, &ayse, env.clone())).await;
    assert_eq!(
        serde_json::from_slice::<ProjectInfo>(&body).unwrap().id,
        made.id
    );
    env["projectId"] = json!(made.id);
    env["idempotencyKey"] = json!(uuid::Uuid::new_v4().to_string());
    let (status, _, _) = send(&router, json_req("POST", &commands, &ayse, env)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _, _) = send(
        &router,
        json_req(
            "POST",
            &commands,
            &ayse,
            envelope(&t, "", "project.archive", json!({})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    // A viewer may not open projects in the organisation (project.create).
    let (status, _, _) = send(
        &router,
        json_req(
            "POST",
            &commands,
            &dilek,
            envelope(&t, "", "project.create", create),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let project = made.id.clone();
    let uri = format!("/v1/tenants/{t}/projects/{project}");
    let dilek_id = admin::user_id(&db.owner, "dilek").await.unwrap();
    let (status, _, _) = send(
        &router,
        access_command(&t, &project, &ayse, dilek_id, Some(GrantRole::Viewer)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // To anyone who may not see it, every catalog route answers as for a project that does not exist.
    let ghost = format!("/v1/tenants/{t}/projects/{}", uuid::Uuid::now_v7());
    let reference = not_found_body(&router, get(&format!("{ghost}/details"), &komsu)).await;
    for (who, path) in [
        (&komsu, &uri),
        (&can, &uri),
        (&can, &format!("/v1/tenants/{other}/projects/{project}")),
    ] {
        assert_eq!(
            not_found_body(&router, get(&format!("{path}/details"), who)).await,
            reference
        );
        for (name, input) in [
            ("project.rename", json!({ "name": "Başka" })),
            ("project.metadata.update", json!({ "tags": ["x"] })),
            ("project.duplicate", json!({})),
            ("project.archive", json!({})),
            ("project.unarchive", json!({})),
            ("project.trash", json!({})),
            ("project.restore", json!({})),
            ("project.purge", json!({ "confirmName": "Ada 101" })),
            ("project.favorite", json!({ "favorite": true })),
        ] {
            let req = json_req(
                "POST",
                &format!("{path}/commands"),
                who,
                envelope(&t, &project, name, input),
            );
            assert_eq!(
                not_found_body(&router, req).await,
                reference,
                "{name} {path}"
            );
        }
    }

    // The viewer sees its details but archives, moves to the trash and removes nothing (403 naming the right).
    let (status, _, body) = send(&router, get(&format!("{uri}/details"), &dilek)).await;
    assert_eq!(status, StatusCode::OK);
    let d: ProjectDetails = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        (d.project.tags, d.feature_count.as_str()),
        (vec!["Kadıköy".to_string()], "0")
    );
    for (name, input, right) in [
        ("project.archive", json!({}), "project.edit"),
        ("project.trash", json!({}), "project.delete"),
        (
            "project.purge",
            json!({ "confirmName": "Ada 101" }),
            "project.delete",
        ),
        ("project.rename", json!({ "name": "Başka" }), "project.edit"),
    ] {
        let (status, _, body) = send(
            &router,
            json_req(
                "POST",
                &format!("{uri}/commands"),
                &dilek,
                envelope(&t, &project, name, input),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{name}");
        assert!(
            serde_json::from_slice::<ApiError>(&body)
                .unwrap()
                .message
                .contains(right),
            "{name}"
        );
    }

    // Archived by the owner: writing answers 409 project_archived; the catalog lists it apart.
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("{uri}/commands"),
            &ayse,
            envelope(&t, &project, "project.archive", json!({})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let archived: ProjectCatalogChange = serde_json::from_slice(&body).unwrap();
    assert_eq!(archived.project.state, ProjectState::Archived);
    let point = json!({ "features": [{ "op": "create", "id": uuid::Uuid::new_v4().to_string(),
        "entity": { "kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": { "x": 1.0, "y": 2.0 } } }] });
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("{uri}/commands"),
            &ayse,
            envelope(&t, &project, "project.changes", point),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        serde_json::from_slice::<ApiError>(&body).unwrap().error,
        "project_archived"
    );
    let (status, _, body) = send(&router, get("/v1/me/catalog?view=archived", &dilek)).await;
    assert_eq!(status, StatusCode::OK);
    let listed: ProjectPage = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        (
            listed.total,
            listed.projects[0].id.as_str(),
            listed.trash_retention_days
        ),
        (1, project.as_str(), 30)
    );

    // The DELETE route moves it to the trash, restorable for the retention; the trash lists it for the owner only.
    let del = axum::http::Request::delete(&uri)
        .header(axum::http::header::COOKIE, &ayse)
        .header("x-kentos-client", "web")
        .body(axum::body::Body::empty())
        .unwrap();
    assert_eq!(send(&router, del).await.0, StatusCode::NO_CONTENT);
    let (_, _, body) = send(&router, get("/v1/me/catalog?view=trash", &ayse)).await;
    let trash: ProjectPage = serde_json::from_slice(&body).unwrap();
    assert!(trash.projects[0].purge_after.is_some());
    let (_, _, body) = send(&router, get("/v1/me/catalog?view=trash", &dilek)).await;
    assert_eq!(
        serde_json::from_slice::<ProjectPage>(&body).unwrap().total,
        0
    );

    // The catalog's parameters are checked; a workspace one is not in is 404 there too.
    for (path, status) in [
        ("/v1/me/catalog", StatusCode::UNPROCESSABLE_ENTITY),
        (
            "/v1/me/catalog?view=hepsi",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "/v1/me/catalog?view=mine&type=imar",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "/v1/me/catalog?view=mine&sort=trashed",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "/v1/me/catalog?view=organization",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "/v1/me/catalog?view=mine&q=Ada&type=zoningPlan&sort=name&limit=5",
            StatusCode::OK,
        ),
    ] {
        assert_eq!(send(&router, get(path, &ayse)).await.0, status, "{path}");
    }
    let (status, _, _) = send(
        &router,
        get(
            &format!("/v1/me/catalog?view=organization&tenant={other}"),
            &ayse,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    db.close().await;
}
