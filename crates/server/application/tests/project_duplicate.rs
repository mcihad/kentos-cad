//! `project.duplicate` against a real PostgreSQL/PostGIS (docs/adr/0028,
//! TODOS.md CLOUD-05): what a copy takes (the settings, layer tree, catalog
//! metadata and every object under its persistent id, bit for bit) and what
//! it does not (history, sharing, favourites, the archived state); who may
//! copy where; a retry copies once.

mod common;

use common::{
    account, catalog_envelope, envelope, member, new_project, open, personal, run, share, try_open,
};
use kentos_application::commands::{self, CatalogPolicy, CommandOutcome};
use kentos_application::{AppError, admin, changes, events, projects};
use kentos_contracts::{
    Entity, FeatureChange, GrantRole, PROJECT_ARCHIVE, PROJECT_DUPLICATE, PROJECT_FAVORITE,
    PROJECT_METADATA_UPDATE, ProjectChanges, ProjectDuplicated, ProjectState, ProjectType,
    TenantKind, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

fn copied(r: Result<CommandOutcome, AppError>) -> ProjectDuplicated {
    match r {
        Ok(CommandOutcome::Duplicated(d)) => d,
        other => panic!("expected a copy, got {other:?}"),
    }
}

/// A point, a line and a circle (a CAD source: its definition and its projection).
fn content() -> ProjectChanges {
    let entities: [serde_json::Value; 3] = [
        json!({ "kind": "point", "id": 1, "layerId": "cizim", "attrs": { "ad": "P1" }, "p": { "x": 486512.3456789, "y": 4420210.9876543 } }),
        json!({ "kind": "line", "id": 2, "layerId": "cizim", "attrs": {}, "a": { "x": 486500.1, "y": 4420200.2 }, "b": { "x": 486520.3, "y": 4420215.4 } }),
        json!({ "kind": "circle", "id": 3, "layerId": "cizim", "attrs": {}, "c": { "x": 486510.0, "y": 4420205.0 }, "r": 3.25 }),
    ];
    ProjectChanges {
        features: entities
            .into_iter()
            .map(|e| FeatureChange::Create {
                id: Uuid::now_v7().to_string(),
                entity: serde_json::from_value::<Entity>(e).unwrap(),
            })
            .collect(),
        project: None,
    }
}

/// Every object of a project as stored: id, version, kind, the geometry's bytes and the CAD definition.
async fn stored(
    db: &TestDb,
    project: Uuid,
) -> Vec<(
    Uuid,
    i64,
    String,
    Option<Vec<u8>>,
    Option<serde_json::Value>,
)> {
    sqlx::query_as(
        "select id, version, kind, public.st_asewkb(geom), cad_definition from kentos.feature where project_id = $1 order by id",
    )
    .bind(project)
    .fetch_all(&db.owner)
    .await
    .unwrap()
}

#[tokio::test]
async fn a_copy_takes_the_content_and_ids_but_not_history_or_sharing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 8)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    let edits = open(&db, &editor, project).await;
    changes::commit(&db.app, &edits, envelope(&editor, project, content(), &[]))
        .await
        .unwrap();
    run(
        &db,
        &owner,
        PROJECT_METADATA_UPDATE,
        json!({ "projectType": "subdivision", "description": "İfraz", "tags": ["Kadıköy"] }),
    )
    .await
    .unwrap();
    run(&db, &edits, PROJECT_FAVORITE, json!({ "favorite": true }))
        .await
        .unwrap();
    // Archived: the copy starts active anyway.
    run(&db, &owner, PROJECT_ARCHIVE, json!({})).await.unwrap();

    // An editor may download: they copy it into their personal space (not the organisation,
    // where an editor opens no projects).
    let edits = open(&db, &editor, project).await;
    let refused = run(&db, &edits, PROJECT_DUPLICATE, json!({})).await;
    assert!(
        matches!(&refused, Err(AppError::Forbidden(m)) if m.contains("project.create")),
        "{refused:?}"
    );
    let space = personal(&db, &editor.actor).await;
    let copy = copied(
        run(
            &db,
            &edits,
            PROJECT_DUPLICATE,
            json!({ "tenantId": space.tenant.to_string() }),
        )
        .await,
    );
    let id = Uuid::parse_str(&copy.project.id).unwrap();
    assert_eq!(copy.project.name, "Ada 101 (kopya)");
    assert_eq!(copy.project.tenant_kind, TenantKind::Personal);
    assert_eq!(copy.project.state, ProjectState::Active);
    assert_eq!(
        (
            copy.project.project_type,
            copy.project.description.as_str(),
            copy.project.tags.clone()
        ),
        (
            ProjectType::Subdivision,
            "İfraz",
            vec!["Kadıköy".to_string()]
        )
    );
    assert_eq!(copy.objects, "3");
    // The copy is the editor's own, and its first commit: data revision 1.
    let mine = open(&db, &space, id).await;
    assert_eq!(mine.role, kentos_contracts::ProjectRole::Owner);
    let info = projects::info(&db.app, &mine).await.unwrap();
    assert_eq!(
        (
            info.data_revision.as_str(),
            info.feature_count.as_str(),
            info.event_cursor.as_str()
        ),
        ("1", "3", "0")
    );
    assert_eq!(
        info.layers,
        projects::info(&db.app, &open(&db, &pm, project).await)
            .await
            .unwrap()
            .layers
    );

    // Every object under its persistent id, at version 1, geometry and CAD definition bit for bit.
    let (a, b) = (stored(&db, project).await, stored(&db, id).await);
    assert_eq!(a.len(), 3);
    for (x, y) in a.iter().zip(&b) {
        assert_eq!((x.0, &x.2, &x.3, &x.4), (y.0, &y.2, &y.3, &y.4));
        assert_eq!(y.1, 1);
    }
    // Not copied: history (events, command log, audit other than its creation), sharing, favourites.
    let (log, grants, favorites, audit): (i64, i64, i64, Vec<String>) = sqlx::query_as(
        "select (select count(*) from kentos.command_log where project_id = $1),
                (select count(*) from kentos.project_grant where project_id = $1),
                (select count(*) from kentos.project_favorite where project_id = $1),
                (select array_agg(action) from kentos.audit_event where project_id = $1)",
    )
    .bind(id)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert_eq!((log, grants, favorites), (0, 0, 0));
    assert_eq!(audit, ["project.create"]);
    assert!(
        events::after(&db.app, &mine, 0, 10)
            .await
            .unwrap()
            .events
            .is_empty()
    );
    // The source records the copy; its owner cannot see the editor's copy.
    let source_audit: serde_json::Value = sqlx::query_scalar(
        "select detail from kentos.audit_event where project_id = $1 and action = 'project.duplicate'",
    )
    .bind(project)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert_eq!(source_audit["copy"], json!(id));
    assert!(matches!(
        try_open(&db, &pm.actor, space.tenant, id).await,
        Err(AppError::NotFound(_))
    ));
    // The copy changes on its own: the same id in two projects is two objects.
    let first = b[0].0.to_string();
    changes::commit(
        &db.app,
        &mine,
        kentos_contracts::CommandEnvelope {
            expected_versions: [(first.clone(), "1".to_string())].into(),
            ..common::envelope_in(
                space.tenant,
                id,
                ProjectChanges {
                    features: vec![FeatureChange::Delete { id: first.clone() }],
                    project: None,
                },
                &[],
            )
        },
    )
    .await
    .unwrap();
    assert_eq!(stored(&db, project).await.len(), 3);

    // A retry with the same key answers the first copy; nothing is copied twice.
    let owner = open(&db, &pm, project).await;
    let env = catalog_envelope(
        &owner,
        PROJECT_DUPLICATE,
        json!({ "name": "Ada 101 yedek" }),
        &[],
    );
    let policy = CatalogPolicy::default();
    let one = copied(commands::run(&db.app, &common::blobs(), &policy, &owner, env.clone()).await);
    let two = copied(commands::run(&db.app, &common::blobs(), &policy, &owner, env).await);
    assert!(
        two.replayed && two.project.id == one.project.id && one.project.name == "Ada 101 yedek"
    );
    let copies: i64 =
        sqlx::query_scalar("select count(*) from kentos.project where name = 'Ada 101 yedek'")
            .fetch_one(&db.owner)
            .await
            .unwrap();
    assert_eq!(copies, 1);
    db.close().await;
}

#[tokio::test]
async fn who_may_copy_and_where() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 8)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 3)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    let stranger = member(&db, "diger", "yabanci", TenantRole::Owner).await;
    let outsider = account(&db, "disaridan").await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &viewer.actor, GrantRole::Viewer).await;
    let viewer_space = personal(&db, &viewer.actor).await;
    let into_own = json!({ "tenantId": viewer_space.tenant.to_string() });

    // A viewer may download, so may copy into their own space …
    let views = open(&db, &viewer, project).await;
    copied(run(&db, &views, PROJECT_DUPLICATE, into_own.clone()).await);
    // … until the organisation keeps downloads from viewers: then neither.
    admin::set_tenant_policy(&db.owner, "buro", None, Some(false))
        .await
        .unwrap();
    let views = open(&db, &viewer, project).await;
    let refused = run(&db, &views, PROJECT_DUPLICATE, into_own.clone()).await;
    assert!(
        matches!(&refused, Err(AppError::Forbidden(m)) if m.contains("project.download")),
        "{refused:?}"
    );
    // Nor can the database be asked directly for them (the function checks the same rule).
    let mut tx = db.app.scoped(views.scope()).await.unwrap();
    let direct = sqlx::query("select kentos.duplicate_project($1, $2, $3, $4, 'kaçak')")
        .bind(pm.tenant)
        .bind(project)
        .bind(viewer_space.tenant)
        .bind(Uuid::now_v7())
        .execute(&mut *tx)
        .await;
    assert!(direct.is_err());
    drop(tx);

    // Into a workspace one is not in: 404 (the same whether it exists); the source of another tenant: 404.
    let other_space = personal(&db, &outsider).await;
    let into_theirs = run(
        &db,
        &owner,
        PROJECT_DUPLICATE,
        json!({ "tenantId": other_space.tenant.to_string() }),
    )
    .await;
    assert!(
        matches!(&into_theirs, Err(AppError::NotFound(_))),
        "{into_theirs:?}"
    );
    let nowhere = run(
        &db,
        &owner,
        PROJECT_DUPLICATE,
        json!({ "tenantId": Uuid::now_v7().to_string() }),
    )
    .await;
    assert!(
        matches!(&nowhere, Err(AppError::NotFound(_))),
        "{nowhere:?}"
    );
    assert!(matches!(
        try_open(&db, &stranger.actor, pm.tenant, project).await,
        Err(AppError::NotFound(_))
    ));

    // A project in the trash is not copied (410).
    kentos_application::lifecycle::delete(&db.app, &owner, None)
        .await
        .unwrap();
    let owner = open(&db, &pm, project).await;
    assert!(matches!(
        run(&db, &owner, PROJECT_DUPLICATE, json!({})).await,
        Err(AppError::Deleted(_))
    ));
    db.close().await;
}
