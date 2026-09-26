//! The lifecycle commands against a real PostgreSQL/PostGIS (docs/adr/0028,
//! TODOS.md CLOUD-05, CLOUD-25): archiving makes a project read-only until
//! it is unarchived; the trash keeps it restorable until its retention ends;
//! removing it for good needs the trash and its name, and leaves only the
//! audit. Who may do each (docs/adr/0015), and the negative cases.

mod common;

use common::{a_point, changed, envelope, member, new_project, open, run, share, try_open};
use kentos_application::commands::{CatalogPolicy, CommandOutcome};
use kentos_application::listing::{self, CatalogQuery};
use kentos_application::{AppError, admin, changes, events, lifecycle, projects};
use kentos_contracts::{
    CatalogView, GrantRole, PROJECT_ACCESS_REVOKE, PROJECT_ARCHIVE, PROJECT_ARCHIVED,
    PROJECT_DELETED, PROJECT_DUPLICATE, PROJECT_METADATA_UPDATE, PROJECT_PURGE, PROJECT_RENAME,
    PROJECT_RESTORE, PROJECT_RESTORED, PROJECT_SHARE, PROJECT_TRASH, PROJECT_UNARCHIVE,
    PROJECT_UNARCHIVED, ProjectState, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

fn view(view: CatalogView) -> CatalogQuery {
    CatalogQuery {
        view,
        tenant: None,
        search: String::new(),
        project_type: None,
        sort: None,
        limit: None,
        after: None,
    }
}

async fn ids(
    db: &TestDb,
    who: &kentos_application::identity::Actor,
    v: CatalogView,
) -> Vec<String> {
    listing::page(
        &db.app,
        who,
        &view(v),
        CatalogPolicy::default().trash_retention,
    )
    .await
    .unwrap()
    .projects
    .into_iter()
    .map(|p| p.id)
    .collect()
}

async fn audited(
    db: &TestDb,
    project: Uuid,
    action: &str,
) -> Vec<(Option<Uuid>, serde_json::Value)> {
    sqlx::query_as(
        "select actor, detail from kentos.audit_event where project_id = $1 and action = $2 order by id",
    )
    .bind(project)
    .bind(action)
    .fetch_all(&db.owner)
    .await
    .unwrap()
}

fn forbidden(r: Result<CommandOutcome, AppError>, permission: &str) {
    match r {
        Err(AppError::Forbidden(m)) => assert!(m.contains(permission), "{m}"),
        other => panic!("expected 403 ({permission}), got {other:?}"),
    }
}

#[tokio::test]
async fn archiving_makes_a_project_read_only_until_it_is_unarchived() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 8)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    let partner = member(&db, "buro", "ortak", TenantRole::Editor).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    share(&db, &owner, &viewer.actor, GrantRole::Viewer).await;
    share(&db, &owner, &partner.actor, GrantRole::Manager).await;
    let (edits, views, manages) = (
        open(&db, &editor, project).await,
        open(&db, &viewer, project).await,
        open(&db, &partner, project).await,
    );
    changes::commit(
        &db.app,
        &edits,
        envelope(&editor, project, a_point(1.0), &[]),
    )
    .await
    .unwrap();

    // Viewers and editors may not archive (project.edit); a manager may.
    forbidden(
        run(&db, &views, PROJECT_ARCHIVE, json!({})).await,
        "project.edit",
    );
    forbidden(
        run(&db, &edits, PROJECT_ARCHIVE, json!({})).await,
        "project.edit",
    );
    let done = changed(
        run(&db, &manages, PROJECT_ARCHIVE, json!({}))
            .await
            .unwrap(),
    );
    assert!(done.changed && done.event_seq.is_some());
    assert_eq!(done.project.state, ProjectState::Archived);
    assert!(done.project.archived_at.is_some());
    // Archiving again changes nothing.
    let again = changed(
        run(&db, &manages, PROJECT_ARCHIVE, json!({}))
            .await
            .unwrap(),
    );
    assert!(!again.changed && again.event_seq.is_none());

    // Read-only: content, name and catalog metadata refuse (409); reading goes on.
    let edits = open(&db, &editor, project).await;
    assert!(edits.archived);
    let refused = changes::commit(
        &db.app,
        &edits,
        envelope(&editor, project, a_point(2.0), &[]),
    )
    .await;
    assert!(
        matches!(&refused, Err(e) if e.code() == "project_archived"),
        "{refused:?}"
    );
    for (name, input) in [
        (PROJECT_RENAME, json!({ "name": "Başka" })),
        (PROJECT_METADATA_UPDATE, json!({ "tags": ["arşiv"] })),
    ] {
        let manages = open(&db, &partner, project).await;
        let r = run(&db, &manages, name, input).await;
        assert!(
            matches!(&r, Err(AppError::Archived(m)) if m.contains("arşivlenmiş")),
            "{name}: {r:?}"
        );
    }
    let info = projects::info(&db.app, &edits).await.unwrap();
    assert_eq!(info.state, ProjectState::Archived);
    assert_eq!(
        projects::features(&db.app, &edits, None, 10)
            .await
            .unwrap()
            .features
            .len(),
        1
    );
    let log = events::after(&db.app, &edits, 0, 50).await.unwrap();
    assert_eq!(log.events.last().unwrap().kind, PROJECT_ARCHIVED);
    // Its sharing may still change.
    let manages = open(&db, &partner, project).await;
    share(&db, &manages, &viewer.actor, GrantRole::Commenter).await;

    // It leaves the active lists and is listed apart, for everyone with a role (a viewer too).
    assert!(
        listing::list(&db.app, &pm)
            .await
            .unwrap()
            .projects
            .is_empty()
    );
    assert!(ids(&db, &pm.actor, CatalogView::Mine).await.is_empty());
    assert!(
        ids(&db, &viewer.actor, CatalogView::Shared)
            .await
            .is_empty()
    );
    for who in [&pm.actor, &viewer.actor] {
        assert_eq!(
            ids(&db, who, CatalogView::Archived).await,
            [project.to_string()]
        );
    }

    // Unarchived by the manager: editable again, back in the lists; audited both ways, with who.
    forbidden(
        run(&db, &views, PROJECT_UNARCHIVE, json!({})).await,
        "project.edit",
    );
    let back = changed(
        run(&db, &manages, PROJECT_UNARCHIVE, json!({}))
            .await
            .unwrap(),
    );
    assert_eq!(
        (back.changed, back.project.state),
        (true, ProjectState::Active)
    );
    let edits = open(&db, &editor, project).await;
    changes::commit(
        &db.app,
        &edits,
        envelope(&editor, project, a_point(3.0), &[]),
    )
    .await
    .unwrap();
    let log = events::after(&db.app, &edits, 0, 50).await.unwrap();
    assert!(log.events.iter().any(|e| e.kind == PROJECT_UNARCHIVED));
    assert_eq!(
        ids(&db, &pm.actor, CatalogView::Mine).await,
        [project.to_string()]
    );
    for action in [PROJECT_ARCHIVE, PROJECT_UNARCHIVE] {
        let rows = audited(&db, project, action).await;
        assert_eq!(rows.len(), 1, "{action}");
        assert_eq!(rows[0].0, Some(partner.actor.user_id));
    }
    db.close().await;
}

#[tokio::test]
async fn the_trash_keeps_a_project_restorable_and_only_its_name_removes_it_for_good() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 8)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 2)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    let partner = member(&db, "buro", "ortak", TenantRole::Editor).await;
    let boss = member(&db, "buro", "mudur", TenantRole::Admin).await;
    let stranger = member(&db, "diger", "yabanci", TenantRole::Owner).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    for (who, role) in [
        (&editor, GrantRole::Editor),
        (&viewer, GrantRole::Viewer),
        (&partner, GrantRole::Manager),
    ] {
        share(&db, &owner, &who.actor, role).await;
    }
    let edits = open(&db, &editor, project).await;
    changes::commit(
        &db.app,
        &edits,
        envelope(&editor, project, a_point(1.0), &[]),
    )
    .await
    .unwrap();

    // Moving to the trash is project.delete: the owner, and the organisation's admins under its
    // policy; not a viewer, an editor or a manager given by sharing (the owner's decision, ADR 0015).
    for who in [&viewer, &editor, &partner] {
        let a = open(&db, who, project).await;
        forbidden(
            run(&db, &a, PROJECT_TRASH, json!({})).await,
            "project.delete",
        );
    }
    let before = time::OffsetDateTime::now_utc();
    let trashed = changed(run(&db, &owner, PROJECT_TRASH, json!({})).await.unwrap());
    assert_eq!(trashed.project.state, ProjectState::Trashed);
    // Kept for the retention (30 days by default), fixed now; who moved it is shown.
    let purge = time::OffsetDateTime::parse(
        trashed.project.purge_after.as_deref().unwrap(),
        &time::format_description::well_known::Rfc3339,
    )
    .unwrap();
    let days = (purge - before).whole_hours();
    assert!((30 * 24 - 1..=30 * 24).contains(&days), "{days}");
    assert_eq!(trashed.project.trashed_by_name.as_deref(), Some("yonetici"));
    // Open editors hear it; opening and writing answer 410; it leaves every list but the trash.
    let edits = open(&db, &editor, project).await;
    let log = events::after(&db.app, &edits, 0, 50).await.unwrap();
    assert_eq!(log.events.last().unwrap().kind, PROJECT_DELETED);
    assert!(
        matches!(projects::info(&db.app, &edits).await, Err(AppError::Deleted(m)) if m.contains("çöp kutusunda"))
    );
    for v in [
        CatalogView::Mine,
        CatalogView::Recent,
        CatalogView::Archived,
    ] {
        assert!(ids(&db, &pm.actor, v).await.is_empty(), "{v:?}");
    }
    // The trash lists it for those who may restore it: its owner and the admin, not the others.
    for who in [&pm.actor, &boss.actor] {
        assert_eq!(
            ids(&db, who, CatalogView::Trash).await,
            [project.to_string()]
        );
    }
    for who in [
        &editor.actor,
        &viewer.actor,
        &partner.actor,
        &stranger.actor,
    ] {
        assert!(ids(&db, who, CatalogView::Trash).await.is_empty());
    }
    // Content and catalog commands refuse a project in the trash (410); another tenant gets 404.
    let manages = open(&db, &partner, project).await;
    for (name, input) in [
        (PROJECT_ARCHIVE, json!({})),
        (PROJECT_RENAME, json!({ "name": "Başka" })),
    ] {
        assert!(
            matches!(
                run(&db, &manages, name, input).await,
                Err(AppError::Deleted(_))
            ),
            "{name}"
        );
    }
    assert!(matches!(
        try_open(&db, &stranger.actor, pm.tenant, project).await,
        Err(AppError::NotFound(_))
    ));

    // Restoring is project.delete too; the admin restores it with everything.
    forbidden(
        run(&db, &manages, PROJECT_RESTORE, json!({})).await,
        "project.delete",
    );
    let by_admin = open(&db, &boss, project).await;
    let restored = changed(
        run(&db, &by_admin, PROJECT_RESTORE, json!({}))
            .await
            .unwrap(),
    );
    assert_eq!(restored.project.state, ProjectState::Active);
    assert!(restored.project.purge_after.is_none() && restored.project.trashed_at.is_none());
    let edits = open(&db, &editor, project).await;
    assert_eq!(
        projects::info(&db.app, &edits).await.unwrap().feature_count,
        "1"
    );
    let log = events::after(&db.app, &edits, 0, 50).await.unwrap();
    assert_eq!(log.events.last().unwrap().kind, PROJECT_RESTORED);
    // Restoring what is not in the trash changes nothing.
    assert!(
        !changed(
            run(&db, &by_admin, PROJECT_RESTORE, json!({}))
                .await
                .unwrap()
        )
        .changed
    );

    // Removing for good: only from the trash, only with its exact name, only by who may delete.
    let owner = open(&db, &pm, project).await;
    let not_yet = run(
        &db,
        &owner,
        PROJECT_PURGE,
        json!({ "confirmName": "Ada 101" }),
    )
    .await;
    assert!(
        matches!(&not_yet, Err(AppError::Invalid { message: m, .. }) if m.contains("çöp kutusunda değil")),
        "{not_yet:?}"
    );
    changed(run(&db, &owner, PROJECT_TRASH, json!({})).await.unwrap());
    let owner = open(&db, &pm, project).await;
    let wrong = run(
        &db,
        &owner,
        PROJECT_PURGE,
        json!({ "confirmName": "Ada 10" }),
    )
    .await;
    assert!(
        matches!(&wrong, Err(AppError::Invalid { message: m, .. }) if m.contains("Hiçbir şey silinmedi")),
        "{wrong:?}"
    );
    let views = open(&db, &viewer, project).await;
    forbidden(
        run(
            &db,
            &views,
            PROJECT_PURGE,
            json!({ "confirmName": "Ada 101" }),
        )
        .await,
        "project.delete",
    );
    // A favourite and a recent use go with it.
    sqlx::query(
        "insert into kentos.project_favorite (tenant_id, project_id, user_id) values ($1, $2, $3)",
    )
    .bind(pm.tenant)
    .bind(project)
    .bind(editor.actor.user_id)
    .execute(&db.owner)
    .await
    .unwrap();
    let purged = match run(
        &db,
        &owner,
        PROJECT_PURGE,
        json!({ "confirmName": " Ada 101 " }),
    )
    .await
    .unwrap()
    {
        CommandOutcome::Purged(p) => p,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        (purged.name.as_str(), purged.objects.as_str()),
        ("Ada 101", "1")
    );

    // Nothing of it stays but the audit, which records who removed it.
    let left: (i64, i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "select (select count(*) from kentos.project where id = $1),
                (select count(*) from kentos.feature where project_id = $1),
                (select count(*) from kentos.project_grant where project_id = $1),
                (select count(*) from kentos.project_recent where project_id = $1),
                (select count(*) from kentos.project_favorite where project_id = $1),
                (select count(*) from kentos.outbox_event where project_id = $1),
                (select count(*) from kentos.command_log where project_id = $1),
                (select count(*) from kentos.audit_event where project_id = $1)",
    )
    .bind(project)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert_eq!(
        (left.0, left.1, left.2, left.3, left.4, left.5, left.6),
        (0, 0, 0, 0, 0, 0, 0)
    );
    assert!(left.7 > 0);
    let purge_audit = audited(&db, project, PROJECT_PURGE).await;
    assert_eq!(purge_audit.len(), 1);
    assert_eq!(purge_audit[0].0, Some(pm.actor.user_id));
    assert_eq!(purge_audit[0].1["objects"], 1);
    // Gone: for everyone who had it, the same 404 as a project that never existed.
    let reference = match try_open(&db, &pm.actor, pm.tenant, Uuid::now_v7()).await {
        Err(AppError::NotFound(m)) => m,
        other => panic!("{other:?}"),
    };
    for who in [&pm, &editor, &boss] {
        match try_open(&db, &who.actor, pm.tenant, project).await {
            Err(AppError::NotFound(m)) => assert_eq!(m, reference),
            other => panic!("{other:?}"),
        }
    }
    db.close().await;
}

#[tokio::test]
async fn the_database_removes_nothing_for_someone_who_may_not_delete() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let partner = member(&db, "buro", "ortak", TenantRole::Editor).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &partner.actor, GrantRole::Manager).await;
    lifecycle::delete(&db.app, &owner, None).await.unwrap();
    // A mistake in the server that called the purge for a manager given by sharing: the function refuses.
    let mut tx = db
        .app
        .scoped(kentos_postgres::Scope {
            tenant: Some(pm.tenant),
            user: Some(partner.actor.user_id),
            project: Some(project),
        })
        .await
        .unwrap();
    let r = sqlx::query("select * from kentos.purge_project($1, $2, 'yanlis')")
        .bind(pm.tenant)
        .bind(project)
        .execute(&mut *tx)
        .await;
    assert!(r.is_err());
    drop(tx);
    let still: i64 = sqlx::query_scalar("select count(*) from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    assert_eq!(still, 1);
    db.close().await;
}

#[tokio::test]
async fn the_retention_removes_what_stayed_too_long_and_nothing_else() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let old = new_project(&db, &pm, "Eski").await;
    let fresh = new_project(&db, &pm, "Yeni silinen").await;
    let before = new_project(&db, &pm, "Kural öncesi").await;
    let kept = new_project(&db, &pm, "Duran").await;
    for p in [old, fresh, before] {
        lifecycle::delete(&db.app, &open(&db, &pm, p).await, None)
            .await
            .unwrap();
    }
    // Its time is over; another's purge_after has passed but it went to the trash an hour ago;
    // one was deleted before the retention existed (no purge_after).
    sqlx::query("update kentos.project set deleted_at = now() - interval '40 days', purge_after = now() - interval '10 days' where id = $1")
        .bind(old)
        .execute(&db.owner)
        .await
        .unwrap();
    sqlx::query("update kentos.project set deleted_at = now() - interval '1 hour', purge_after = now() - interval '1 minute' where id = $1")
        .bind(fresh)
        .execute(&db.owner)
        .await
        .unwrap();
    sqlx::query("update kentos.project set deleted_at = now() - interval '400 days', purge_after = null where id = $1")
        .bind(before)
        .execute(&db.owner)
        .await
        .unwrap();
    let mut removed = 0;
    loop {
        let n = lifecycle::purge_expired(&db.app, 1).await.unwrap();
        removed += n;
        if n == 0 {
            break;
        }
    }
    assert_eq!(removed, 1);
    let left: Vec<Uuid> = sqlx::query_scalar("select id from kentos.project order by id")
        .fetch_all(&db.owner)
        .await
        .unwrap();
    let mut want = vec![fresh, before, kept];
    want.sort();
    assert_eq!(left, want);
    let audit = audited(&db, old, PROJECT_PURGE).await;
    assert_eq!(audit.len(), 1);
    assert_eq!((audit[0].0, &audit[0].1["by"]), (None, &json!("retention")));
    db.close().await;
}

#[tokio::test]
async fn the_owner_is_kept_through_every_lifecycle_change() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let partner = member(&db, "buro", "ortak", TenantRole::ProjectManager).await;
    let boss = member(&db, "buro", "mudur", TenantRole::Admin).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &partner.actor, GrantRole::Manager).await;
    let owner_of = || async {
        sqlx::query_scalar::<_, Uuid>("select owner_user_id from kentos.project where id = $1")
            .bind(project)
            .fetch_one(&db.owner)
            .await
            .unwrap()
    };

    // The admin archives, unarchives, trashes and restores it: its owner stays, with every right.
    for name in [
        PROJECT_ARCHIVE,
        PROJECT_UNARCHIVE,
        PROJECT_TRASH,
        PROJECT_RESTORE,
    ] {
        let by_admin = open(&db, &boss, project).await;
        changed(run(&db, &by_admin, name, json!({})).await.unwrap());
        assert_eq!(owner_of().await, pm.actor.user_id, "{name}");
    }
    let owner = open(&db, &pm, project).await;
    assert_eq!(owner.role, kentos_contracts::ProjectRole::Owner);

    // Nobody takes the owner's access away or lowers it by sharing, the owner included.
    let manages = open(&db, &partner, project).await;
    for (who, name, input) in [
        (
            &manages,
            PROJECT_SHARE,
            json!({ "userId": pm.actor.user_id, "role": "viewer" }),
        ),
        (
            &manages,
            PROJECT_ACCESS_REVOKE,
            json!({ "userId": pm.actor.user_id }),
        ),
        (
            &owner,
            PROJECT_ACCESS_REVOKE,
            json!({ "userId": pm.actor.user_id }),
        ),
    ] {
        let r = run(&db, who, name, input).await;
        assert!(matches!(r, Err(AppError::Invalid { .. })), "{name}: {r:?}");
    }
    // A copy is its maker's: the source keeps its owner.
    let copy = match run(&db, &manages, PROJECT_DUPLICATE, json!({}))
        .await
        .unwrap()
    {
        CommandOutcome::Duplicated(d) => d,
        other => panic!("{other:?}"),
    };
    let copy_owner: Uuid =
        sqlx::query_scalar("select owner_user_id from kentos.project where id = $1")
            .bind(Uuid::parse_str(&copy.project.id).unwrap())
            .fetch_one(&db.owner)
            .await
            .unwrap();
    assert_eq!(
        (copy_owner, owner_of().await),
        (partner.actor.user_id, pm.actor.user_id)
    );
    // And the database keeps a project from losing its owner at all.
    let r = sqlx::query("update kentos.project set owner_user_id = null where id = $1")
        .bind(project)
        .execute(&db.owner)
        .await;
    assert!(r.is_err());
    db.close().await;
}
