//! A project's life beyond its content, against a real PostgreSQL/PostGIS:
//! renaming (a metadata change of `project.changes`), deleting (soft, only
//! for admins, told to open editors) and restoring by the operator.

mod common;

use common::{a_point, envelope, member, new_project};
use kentos_application::{AppError, admin, changes, events, lifecycle, projects};
use kentos_contracts::{ConflictReason, PROJECT_DELETED, ProjectChanges, ProjectPatch, TenantRole};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

fn renamed(name: &str) -> ProjectChanges {
    ProjectChanges {
        features: vec![],
        project: Some(ProjectPatch {
            name: Some(name.into()),
            ..Default::default()
        }),
    }
}

#[tokio::test]
async fn renaming_is_a_metadata_change_with_its_version() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let project = new_project(&db, &pm, "Ada 101").await;

    let done = changes::commit(
        &db.app,
        &pm,
        envelope(
            &pm,
            project,
            renamed("  Ada 101 (revize)  "),
            &[("@project", "1")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(done.meta_version, "2");
    let list = projects::list(&db.app, &pm).await.unwrap();
    assert_eq!(list.projects[0].name, "Ada 101 (revize)");
    // Open editors learn it from the event (metadata changed).
    let log = events::after(&db.app, &editor, project, 0, 10)
        .await
        .unwrap();
    assert!(log.events.last().unwrap().meta);
    // An editor may not rename; an empty name and a stale version are refused.
    assert!(matches!(
        changes::commit(
            &db.app,
            &editor,
            envelope(&editor, project, renamed("Başka"), &[("@project", "2")])
        )
        .await,
        Err(AppError::Forbidden(_))
    ));
    assert!(matches!(
        changes::commit(
            &db.app,
            &pm,
            envelope(&pm, project, renamed("  "), &[("@project", "2")])
        )
        .await,
        Err(AppError::Invalid(_))
    ));
    assert!(matches!(
        changes::commit(&db.app, &pm, envelope(&pm, project, renamed("Eski sürümden"), &[("@project", "1")])).await,
        Err(AppError::Conflict { conflicts, .. }) if conflicts[0].reason == ConflictReason::Project
    ));
    db.close().await;
}

#[tokio::test]
async fn admins_delete_projects_softly_and_editors_are_told() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 2)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    let boss = member(&db, "buro", "mudur", TenantRole::Admin).await;
    let stranger = member(&db, "diger", "yabanci", TenantRole::Owner).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let first = envelope(&editor, project, a_point(486512.0), &[]);
    changes::commit(&db.app, &editor, first.clone())
        .await
        .unwrap();

    // Only an admin (or the owner) deletes; another tenant cannot even see the project.
    for who in [&viewer, &editor, &pm] {
        assert!(
            matches!(lifecycle::delete(&db.app, who, project, None).await, Err(AppError::Forbidden(m)) if m.contains("project.delete"))
        );
    }
    assert!(matches!(
        lifecycle::delete(&db.app, &stranger, project, None).await,
        Err(AppError::NotFound(_))
    ));
    let seq = lifecycle::delete(&db.app, &boss, project, Some("istek-sil"))
        .await
        .unwrap()
        .expect("deleted now");

    // Gone from the list; opening, reading and writing answer 410.
    assert!(
        projects::list(&db.app, &pm)
            .await
            .unwrap()
            .projects
            .is_empty()
    );
    assert!(
        matches!(projects::info(&db.app, &pm, project).await, Err(AppError::Deleted(m)) if m.contains("“Ada 101” projesi silindi"))
    );
    assert!(matches!(
        projects::features(&db.app, &editor, project, None, 10).await,
        Err(AppError::Deleted(_))
    ));
    assert!(matches!(
        projects::features_by_id(&db.app, &editor, project, &[Uuid::new_v4()]).await,
        Err(AppError::Deleted(_))
    ));
    let late = changes::commit(
        &db.app,
        &editor,
        envelope(&editor, project, a_point(1.0), &[]),
    )
    .await;
    assert!(
        matches!(&late, Err(e) if e.code() == "project_deleted"),
        "{late:?}"
    );
    // A command committed before the deletion is still answered from the log (a retry after a lost answer).
    assert!(
        changes::commit(&db.app, &editor, first)
            .await
            .unwrap()
            .replayed
    );

    // Open editors read the deletion from the event log, with who and which request.
    let log = events::after(&db.app, &viewer, project, 0, 10)
        .await
        .unwrap();
    let last = log.events.last().unwrap();
    assert_eq!(
        (
            last.kind.as_str(),
            last.seq.clone(),
            last.actor.clone(),
            last.request_id.as_deref(),
            last.features.len()
        ),
        (
            PROJECT_DELETED,
            seq.to_string(),
            Some(boss.actor.user_id.to_string()),
            Some("istek-sil"),
            0
        )
    );
    assert_eq!(
        events::latest(&db.app, &viewer, project).await.unwrap(),
        seq
    );
    assert!(matches!(
        events::after(&db.app, &stranger, project, 0, 10).await,
        Err(AppError::NotFound(_))
    ));
    // Deleting again changes nothing.
    assert_eq!(
        lifecycle::delete(&db.app, &boss, project, None)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        events::after(&db.app, &viewer, project, 0, 10)
            .await
            .unwrap()
            .events
            .len(),
        log.events.len()
    );

    // Nothing was removed; the deletion is audited; the operator lists and restores it.
    let (objects, audited): (i64, i64) = sqlx::query_as(
        "select (select count(*) from kentos.feature where project_id = $1),
                (select count(*) from kentos.audit_event where project_id = $1 and action = 'project.delete' and actor = $2)",
    )
    .bind(project)
    .bind(boss.actor.user_id)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert_eq!((objects, audited), (1, 1));
    let deleted = admin::deleted_projects(&db.owner, "buro").await.unwrap();
    assert_eq!(
        (deleted.len(), deleted[0].id, deleted[0].deleted_by.as_str()),
        (1, project, "mudur")
    );
    assert_eq!(
        admin::restore_project(&db.owner, "buro", project)
            .await
            .unwrap(),
        "Ada 101"
    );
    assert_eq!(
        projects::list(&db.app, &pm).await.unwrap().projects.len(),
        1
    );
    let info = projects::info(&db.app, &editor, project).await.unwrap();
    assert_eq!(
        (info.feature_count.as_str(), info.event_cursor.clone()),
        ("1", seq.to_string())
    );
    changes::commit(
        &db.app,
        &editor,
        envelope(&editor, project, a_point(2.0), &[]),
    )
    .await
    .unwrap();
    assert!(matches!(
        admin::restore_project(&db.owner, "buro", project).await,
        Err(AppError::NotFound(_))
    ));
    assert!(
        admin::deleted_projects(&db.owner, "buro")
            .await
            .unwrap()
            .is_empty()
    );
    db.close().await;
}

#[tokio::test]
async fn a_commit_waiting_for_the_project_sees_the_deletion() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let boss = member(&db, "buro", "mudur", TenantRole::Admin).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    // Deleting and writing at once: whichever takes the project's lock second sees the first.
    let (deleted, written) = tokio::join!(
        lifecycle::delete(&db.app, &boss, project, None),
        changes::commit(&db.app, &pm, envelope(&pm, project, a_point(3.0), &[]))
    );
    assert!(deleted.unwrap().is_some());
    match written {
        Ok(_) => {
            // The write went first: the deletion event comes after it.
            let log = events::after(&db.app, &pm, project, 0, 10).await.unwrap();
            assert_eq!(log.events.last().unwrap().kind, PROJECT_DELETED);
        }
        Err(e) => assert_eq!(e.code(), "project_deleted"),
    }
    db.close().await;
}
