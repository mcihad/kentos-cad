//! The share dialog's routes end to end, with a throwaway database
//! (docs/adr/0015, TODOS.md CLOUD-12, CLOUD-21): the access list and the
//! search for people to share with answer only those who may share; to
//! anyone who may not see the project they answer 404 word for word as a
//! project that does not exist; a search never reaches another organisation;
//! and nobody raises their own access or takes the owner's away.

use axum::http::StatusCode;
use kentos_application::admin;
use kentos_contracts::{
    AccessSource, ApiError, GrantRole, ProjectAccessList, ProjectRole, ShareCandidates, TenantRole,
};
use kentos_postgres::testing::TestDb;

use super::tests::{access_command, app, get, json_req, not_found_body, send, signed_in};

#[tokio::test]
async fn the_share_dialog_routes_answer_only_who_may_share() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 6)
        .await
        .unwrap();
    let other = admin::create_tenant(&db.owner, "diger", "Diğer", 3)
        .await
        .unwrap();
    for (login, name, t, role) in [
        ("ayse", "Ayşe Yılmaz", "buro", TenantRole::ProjectManager),
        ("bora", "Bora Tan", "buro", TenantRole::Editor),
        ("dilek", "Dilek Er", "buro", TenantRole::Viewer),
        ("can", "Bora Can", "diger", TenantRole::Owner),
    ] {
        admin::create_local_user(&db.owner, login, name, None, "dogru-parola-1")
            .await
            .unwrap();
        admin::set_membership(&db.owner, t, login, role, true)
            .await
            .unwrap();
    }
    // Ayşe belongs to the other organisation too: still, a project of hers in Büro finds only Büro's people.
    admin::set_membership(&db.owner, "diger", "ayse", TenantRole::Editor, true)
        .await
        .unwrap();
    let router = app(Some(db.app.clone()));
    let (ayse, bora, dilek, can) = (
        signed_in(&router, "ayse").await,
        signed_in(&router, "bora").await,
        signed_in(&router, "dilek").await,
        signed_in(&router, "can").await,
    );
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": "Ada 101", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] } });
    let base = format!("/v1/tenants/{tenant}/projects");
    let (status, _, body) = send(&router, json_req("POST", &base, &ayse, create)).await;
    assert_eq!(status, StatusCode::CREATED);
    let project = serde_json::from_slice::<kentos_contracts::ProjectInfo>(&body)
        .unwrap()
        .id;
    let uri = format!("{base}/{project}");
    let search = |q: &str| format!("{uri}/access/candidates?q={q}");
    let grant = |who: uuid::Uuid, role: Option<GrantRole>, cookie: &str| {
        access_command(&tenant.to_string(), &project, cookie, who, role)
    };
    let (bora_id, dilek_id, ayse_id, can_id) = (
        admin::user_id(&db.owner, "bora").await.unwrap(),
        admin::user_id(&db.owner, "dilek").await.unwrap(),
        admin::user_id(&db.owner, "ayse").await.unwrap(),
        admin::user_id(&db.owner, "can").await.unwrap(),
    );

    // The owner finds Büro's Bora, not the other organisation's "Bora Can", and not herself.
    let (status, _, body) = send(&router, get(&search("bora"), &ayse)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let found: ShareCandidates = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        found
            .candidates
            .iter()
            .map(|c| (c.display_name.as_str(), c.user_id.clone()))
            .collect::<Vec<_>>(),
        [("Bora Tan", bora_id.to_string())]
    );
    let (_, _, body) = send(&router, get(&search("ay%C5%9Fe"), &ayse)).await;
    assert!(
        serde_json::from_slice::<ShareCandidates>(&body)
            .unwrap()
            .candidates
            .is_empty()
    );
    // A search too short to be one.
    let (status, _, body) = send(&router, get(&search("b"), &ayse)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        serde_json::from_slice::<ApiError>(&body).unwrap().error,
        "invalid"
    );
    let (status, _, _) = send(&router, get(&format!("{uri}/access/candidates"), &ayse)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Every 404 of these routes reads the same: a member it was not shared with, another
    // organisation's member (with either tenant in the path), a guessed or malformed id.
    let reference = not_found_body(&router, get(&uri, &can)).await;
    for (path, cookie) in [
        (search("bora"), &bora),
        (format!("{uri}/access"), &bora),
        (search("bora"), &can),
        (format!("{uri}/access"), &can),
        (
            format!("/v1/tenants/{other}/projects/{project}/access/candidates?q=bora"),
            &can,
        ),
        (
            format!("{base}/{}/access/candidates?q=bora", uuid::Uuid::now_v7()),
            &ayse,
        ),
        (format!("{base}/bozuk/access/candidates?q=bora"), &ayse),
        (format!("{base}/{}/access", uuid::Uuid::now_v7()), &ayse),
    ] {
        assert_eq!(
            not_found_body(&router, get(&path, cookie)).await,
            reference,
            "{path}"
        );
    }

    // Shared as a viewer: Dilek sees the project, but neither who has access nor whom to share with.
    let (status, _, _) = send(&router, grant(dilek_id, Some(GrantRole::Viewer), &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    for path in [search("bora"), format!("{uri}/access")] {
        let (status, _, body) = send(&router, get(&path, &dilek)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
        assert!(
            serde_json::from_slice::<ApiError>(&body)
                .unwrap()
                .message
                .contains("project.share")
        );
    }
    // Nor may she share, or raise herself.
    let (status, _, _) = send(&router, grant(bora_id, Some(GrantRole::Viewer), &dilek)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = send(&router, grant(dilek_id, Some(GrantRole::Manager), &dilek)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Made a manager, she may share, but never change her own access, touch the owner's,
    // give ownership, or reach outside the organisation.
    let (status, _, _) = send(&router, grant(dilek_id, Some(GrantRole::Manager), &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(&router, grant(bora_id, Some(GrantRole::Editor), &dilek)).await;
    assert_eq!(status, StatusCode::OK);
    for (who, role) in [
        (dilek_id, Some(GrantRole::Editor)),
        (dilek_id, None),
        (ayse_id, Some(GrantRole::Viewer)),
        (ayse_id, None),
        (can_id, Some(GrantRole::Viewer)),
    ] {
        let (status, _, body) = send(&router, grant(who, role, &dilek)).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{who} {role:?}: {}",
            String::from_utf8_lossy(&body)
        );
    }
    let mut as_owner = grant(bora_id, Some(GrantRole::Viewer), &dilek);
    let raw = axum::body::to_bytes(std::mem::take(as_owner.body_mut()), usize::MAX)
        .await
        .unwrap();
    let mut envelope: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    envelope["input"]["role"] = serde_json::json!("owner");
    *as_owner.body_mut() = axum::body::Body::from(envelope.to_string());
    let (status, _, _) = send(&router, as_owner).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // The list says who works with what and why; the owner first.
    let (status, _, body) = send(&router, get(&format!("{uri}/access"), &dilek)).await;
    assert_eq!(status, StatusCode::OK);
    let list: ProjectAccessList = serde_json::from_slice(&body).unwrap();
    let rows: Vec<(&str, Option<ProjectRole>, Option<AccessSource>)> = list
        .people
        .iter()
        .map(|p| (p.display_name.as_str(), p.role, p.via))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "Ayşe Yılmaz",
                Some(ProjectRole::Owner),
                Some(AccessSource::Owner)
            ),
            (
                "Bora Tan",
                Some(ProjectRole::Editor),
                Some(AccessSource::Grant)
            ),
            (
                "Dilek Er",
                Some(ProjectRole::Manager),
                Some(AccessSource::Grant)
            ),
        ]
    );
    db.close().await;
}
