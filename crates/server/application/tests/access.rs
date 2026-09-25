//! Project ownership and access against a real PostgreSQL/PostGIS
//! (docs/adr/0015, TODOS.md CLOUD-09..13): where roles come from, the
//! personal space, sharing, the negative isolation cases of the ADR, row-level
//! security as the second layer, and what migration 0004 made of the
//! projects that existed before it.

mod common;

use common::{
    a_point, account, envelope, envelope_in, member, new_project, open, personal, revoke,
    revoke_envelope, share, share_envelope, try_open,
};
use kentos_application::access::ProjectAccess;
use kentos_application::identity::Actor;
use kentos_application::{
    AppError, admin, changes, events, lifecycle, listing, projects, sharing, tenancy,
};
use kentos_contracts::{
    AccessSource, GrantRole, PROJECT_ACCESS_CHANGED, ProjectPermission, ProjectRole, TenantKind,
    TenantRole,
};
use kentos_postgres::Scope;
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

use ProjectPermission::*;

/// Rows a query counts in a scope, as the server's role sees them.
async fn count_in(db: &TestDb, scope: Scope, sql: &'static str) -> i64 {
    let mut tx = db.app.scoped(scope).await.unwrap();
    let n: i64 = sqlx::query_scalar(sql).fetch_one(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    n
}

fn scope(tenant: Uuid, user: &Actor, project: Option<Uuid>) -> Scope {
    Scope {
        tenant: Some(tenant),
        user: Some(user.user_id),
        project,
    }
}

/// The refusal of a project someone may not see: the same text for every such project.
fn not_found_text(r: Result<ProjectAccess, AppError>) -> String {
    match r {
        Err(AppError::NotFound(m)) => m,
        other => panic!("expected 404, got {other:?}"),
    }
}

fn is_denied(r: Result<sqlx::postgres::PgQueryResult, sqlx::Error>) -> bool {
    matches!(r, Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42501"))
}

#[tokio::test]
async fn roles_come_from_ownership_grants_and_the_policy() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 8)
        .await
        .unwrap();
    let boss = member(&db, "buro", "patron", TenantRole::Owner).await;
    let admin_ = member(&db, "buro", "mudur", TenantRole::Admin).await;
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    let commenter = member(&db, "buro", "yorumcu", TenantRole::Viewer).await;
    let manager = member(&db, "buro", "ortak", TenantRole::Editor).await;
    let project = new_project(&db, &pm, "Ada 101").await;

    // The creator owns it, with every permission.
    let owner = open(&db, &pm, project).await;
    assert_eq!(
        (owner.role, owner.via),
        (ProjectRole::Owner, AccessSource::Owner)
    );
    assert_eq!(owner.permissions(), ProjectPermission::ALL.as_slice());
    // Nobody else in the organisation sees it until it is shared, whatever their tenant role …
    for who in [&editor, &viewer, &commenter, &manager] {
        assert!(matches!(
            try_open(&db, &who.actor, pm.tenant, project).await,
            Err(AppError::NotFound(_))
        ));
        assert!(
            listing::list(&db.app, who)
                .await
                .unwrap()
                .projects
                .is_empty()
        );
    }
    // … except the owners and admins, through the policy: a manager who may also delete.
    for who in [&boss, &admin_] {
        let a = open(&db, who, project).await;
        assert_eq!(
            (a.role, a.via),
            (ProjectRole::Manager, AccessSource::Policy)
        );
        assert!(a.allows(Delete) && a.allows(Share) && a.allows(Edit) && !a.allows(Transfer));
        assert_eq!(listing::list(&db.app, who).await.unwrap().projects.len(), 1);
    }

    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    share(&db, &owner, &viewer.actor, GrantRole::Viewer).await;
    share(&db, &owner, &commenter.actor, GrantRole::Commenter).await;
    share(&db, &owner, &manager.actor, GrantRole::Manager).await;
    let e = open(&db, &editor, project).await;
    assert_eq!((e.role, e.via), (ProjectRole::Editor, AccessSource::Grant));
    assert_eq!(
        e.permissions(),
        &[Read, FeatureWrite, Comment, Download, History, JobsRun]
    );
    let v = open(&db, &viewer, project).await;
    assert_eq!(v.permissions(), &[Read, Download, History]);
    let c = open(&db, &commenter, project).await;
    assert_eq!(c.permissions(), &[Read, Comment, Download, History]);
    let m = open(&db, &manager, project).await;
    assert!(m.allows(Share) && m.allows(Edit) && !m.allows(Delete));
    // The list shows each person their own access to each project.
    let listed = listing::list(&db.app, &editor).await.unwrap();
    assert_eq!(listed.projects[0].access, e.view());
    assert_eq!(listed.projects[0].tenant_kind, TenantKind::Organization);

    // The organisation may keep downloads from viewers and commenters.
    admin::set_tenant_policy(&db.owner, "buro", None, Some(false))
        .await
        .unwrap();
    assert!(!open(&db, &viewer, project).await.allows(Download));
    assert!(!open(&db, &commenter, project).await.allows(Download));
    assert!(open(&db, &editor, project).await.allows(Download));

    // A grant with an end stops by itself.
    let with_end = share_envelope(
        &owner,
        viewer.actor.user_id,
        GrantRole::Viewer,
        Some("2999-01-01T00:00:00Z".into()),
    );
    let changed = sharing::share(&db.app, &owner, with_end).await.unwrap();
    assert_eq!(
        changed.grant.unwrap().expires_at.as_deref(),
        Some("2999-01-01T00:00:00Z")
    );
    sqlx::query("update kentos.project_grant set expires_at = now() - interval '1 second' where user_id = $1")
        .bind(viewer.actor.user_id)
        .execute(&db.owner)
        .await
        .unwrap();
    assert!(matches!(
        try_open(&db, &viewer.actor, pm.tenant, project).await,
        Err(AppError::NotFound(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn the_negative_isolation_cases_of_the_adr() {
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
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let unshared = member(&db, "buro", "komsu", TenantRole::ProjectManager).await;
    let admin_ = member(&db, "buro", "mudur", TenantRole::Admin).await;
    let stranger = member(&db, "diger", "yabanci", TenantRole::Owner).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    let edits = open(&db, &editor, project).await;
    changes::commit(
        &db.app,
        &edits,
        envelope(&editor, project, a_point(1.0), &[]),
    )
    .await
    .unwrap();
    let tenant = pm.tenant;

    // The answer for a project one may not see, and for one that does not exist: word for word the same.
    let reference = not_found_text(try_open(&db, &stranger.actor, tenant, Uuid::now_v7()).await);

    // 1. An unshared member of the organisation (a project manager: tenant rights are not project rights).
    assert_eq!(
        not_found_text(try_open(&db, &unshared.actor, tenant, project).await),
        reference
    );
    assert!(
        listing::list(&db.app, &unshared)
            .await
            .unwrap()
            .projects
            .is_empty()
    );
    assert!(
        !listing::mine(&db.app, &unshared.actor)
            .await
            .unwrap()
            .projects
            .iter()
            .any(|p| p.id == project.to_string())
    );

    // 2. A member of another organisation, with this tenant or their own in the path.
    for t in [tenant, stranger.tenant] {
        assert_eq!(
            not_found_text(try_open(&db, &stranger.actor, t, project).await),
            reference
        );
    }

    // 3. A person whose grant was taken away: refused at once, even with the access they held.
    revoke(&db, &owner, &editor.actor).await;
    assert_eq!(
        not_found_text(try_open(&db, &editor.actor, tenant, project).await),
        reference
    );
    assert!(matches!(
        projects::features(&db.app, &edits, None, 10).await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        projects::info(&db.app, &edits).await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        events::after(&db.app, &edits, 0, 10).await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        changes::commit(
            &db.app,
            &edits,
            envelope(&editor, project, a_point(2.0), &[])
        )
        .await,
        Err(AppError::NotFound(_))
    ));
    assert!(
        listing::list(&db.app, &editor)
            .await
            .unwrap()
            .projects
            .is_empty()
    );

    // 4. An admin while the organisation's policy is off: only what is shared with them, at its role.
    admin::set_tenant_policy(&db.owner, "buro", Some(false), None)
        .await
        .unwrap();
    assert_eq!(
        not_found_text(try_open(&db, &admin_.actor, tenant, project).await),
        reference
    );
    assert!(
        listing::list(&db.app, &admin_)
            .await
            .unwrap()
            .projects
            .is_empty()
    );
    share(&db, &owner, &admin_.actor, GrantRole::Viewer).await;
    let a = open(&db, &admin_, project).await;
    assert_eq!((a.role, a.via), (ProjectRole::Viewer, AccessSource::Grant));
    assert!(matches!(
        lifecycle::delete(&db.app, &a, None).await,
        Err(AppError::Forbidden(m)) if m.contains("project.delete")
    ));
    admin::set_tenant_policy(&db.owner, "buro", Some(true), None)
        .await
        .unwrap();
    let a = open(&db, &admin_, project).await;
    assert_eq!(
        (a.role, a.via),
        (ProjectRole::Manager, AccessSource::Policy)
    );

    // 5. A deleted project: 410 to those who had access, 404 to everyone else; out of every list.
    let doomed = new_project(&db, &pm, "Ada 7").await;
    let doomed_owner = open(&db, &pm, doomed).await;
    share(&db, &doomed_owner, &editor.actor, GrantRole::Viewer).await;
    lifecycle::delete(&db.app, &doomed_owner, None)
        .await
        .unwrap();
    let had = open(&db, &editor, doomed).await;
    assert!(had.deleted);
    assert!(matches!(
        projects::info(&db.app, &had).await,
        Err(AppError::Deleted(_))
    ));
    assert!(
        !events::after(&db.app, &had, 0, 10)
            .await
            .unwrap()
            .events
            .is_empty()
    );
    for who in [&unshared.actor, &stranger.actor] {
        assert_eq!(
            not_found_text(try_open(&db, who, tenant, doomed).await),
            reference
        );
    }
    for who in [&pm, &editor, &admin_] {
        assert!(
            !listing::list(&db.app, who)
                .await
                .unwrap()
                .projects
                .iter()
                .any(|p| p.id == doomed.to_string())
        );
    }

    // 6. A guessed project id, and a guessed tenant id.
    for (t, p) in [
        (tenant, Uuid::new_v4()),
        (tenant, Uuid::now_v7()),
        (Uuid::now_v7(), project),
    ] {
        assert_eq!(
            not_found_text(try_open(&db, &unshared.actor, t, p).await),
            reference
        );
    }

    // 7. Lists, counts, events and errors say nothing of what one may not see. Rows exist …
    let (objects, log): (i64, i64) = sqlx::query_as(
        "select (select count(*) from kentos.feature where project_id = $1), (select count(*) from kentos.outbox_event where project_id = $1)",
    )
    .bind(project)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert!(objects == 1 && log >= 2);
    // … and even with the project's id in scope an outsider's queries find none of them.
    // (One's own grants stay visible anywhere: the editor still holds one in the deleted project.)
    for who in [&unshared.actor, &editor.actor, &stranger.actor] {
        let s = scope(tenant, who, Some(project));
        for sql in [
            "select count(*) from kentos.project",
            "select count(*) from kentos.feature",
            "select count(*) from kentos.outbox_event",
            "select count(*) from kentos.command_log",
            "select count(*) from kentos.audit_event",
            "select count(*) from kentos.project_grant where project_id = kentos.current_project()",
        ] {
            assert_eq!(count_in(&db, s, sql).await, 0, "{sql}");
        }
    }
    // A member who may not use the organisation now is told so, the same for a real and a guessed id.
    sqlx::query("delete from kentos.seat_allocation where user_id = $1")
        .bind(unshared.actor.user_id)
        .execute(&db.owner)
        .await
        .unwrap();
    let seatless = |p: Uuid| {
        let db = &db;
        let actor = unshared.actor.clone();
        async move {
            match try_open(db, &actor, tenant, p).await {
                Err(AppError::Forbidden(m)) => m,
                other => panic!("expected 403, got {other:?}"),
            }
        }
    };
    assert_eq!(seatless(project).await, seatless(Uuid::now_v7()).await);
    db.close().await;
}

#[tokio::test]
async fn row_level_security_is_the_second_layer() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let shared = new_project(&db, &pm, "Paylaşılan").await;
    let private = new_project(&db, &pm, "Özel").await;
    let owner = open(&db, &pm, shared).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    let edits = open(&db, &editor, shared).await;
    changes::commit(
        &db.app,
        &edits,
        envelope(&editor, shared, a_point(1.0), &[]),
    )
    .await
    .unwrap();
    let mine = open(&db, &pm, private).await;
    changes::commit(&db.app, &mine, envelope(&pm, private, a_point(2.0), &[]))
        .await
        .unwrap();
    let t = pm.tenant;

    // In the shared project's scope the editor sees its rows and nothing of the other project.
    let s = scope(t, &editor.actor, Some(shared));
    assert_eq!(
        count_in(&db, s, "select count(*) from kentos.feature").await,
        1
    );
    assert_eq!(
        count_in(&db, s, "select count(*) from kentos.project").await,
        1
    );
    assert_eq!(
        count_in(
            &db,
            s,
            "select count(*) from kentos.feature where project_id <> kentos.current_project()"
        )
        .await,
        0
    );
    // Even the owner sees one project's rows only inside that project's scope.
    let p = scope(t, &pm.actor, Some(shared));
    assert_eq!(
        count_in(&db, p, "select count(*) from kentos.feature").await,
        1
    );
    assert_eq!(
        count_in(&db, p, "select count(*) from kentos.project").await,
        1
    );
    assert_eq!(
        count_in(
            &db,
            scope(t, &pm.actor, None),
            "select count(*) from kentos.feature"
        )
        .await,
        0
    );
    assert_eq!(
        count_in(
            &db,
            scope(t, &pm.actor, None),
            "select count(*) from kentos.project"
        )
        .await,
        2
    );

    // With the private project in scope the editor can neither read nor write any of it.
    let wrong = scope(t, &editor.actor, Some(private));
    for sql in [
        "select count(*) from kentos.project",
        "select count(*) from kentos.feature",
        "select count(*) from kentos.outbox_event",
        "select count(*) from kentos.command_log",
        "select count(*) from kentos.audit_event",
    ] {
        assert_eq!(count_in(&db, wrong, sql).await, 0, "{sql}");
    }
    let attempt = |sql: &'static str| {
        let app = db.app.clone();
        let editor = editor.actor.user_id;
        async move {
            let mut tx = app.scoped(wrong).await.unwrap();
            let r = sqlx::query(sql)
                .bind(t)
                .bind(private)
                .bind(editor)
                .execute(&mut *tx)
                .await;
            drop(tx);
            r
        }
    };
    assert!(is_denied(
        attempt("insert into kentos.project_grant (tenant_id, project_id, user_id, role) values ($1, $2, $3, 'manager')").await
    ));
    assert!(is_denied(
        attempt("insert into kentos.outbox_event (tenant_id, project_id, data_revision, kind, payload) values ($1, $2, 0, 'x', '{}'::jsonb) returning $3").await
    ));
    // Changing the invisible project affects nothing.
    let touched = attempt("update kentos.project set name = 'ele geçirildi' where tenant_id = $1 and id = $2 and $3::uuid is not null")
        .await
        .unwrap();
    assert_eq!(touched.rows_affected(), 0);
    // A project row cannot be made for someone else, nor inside a personal space by a guest.
    let mut tx = db.app.scoped(scope(t, &editor.actor, None)).await.unwrap();
    assert!(is_denied(
        sqlx::query(
            "insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y, styles, created_by, owner_user_id)
             values ($1, $2, 'başkasının', 5256, '{}', '[]', 'x', 0, 0, '{}', $3, $4)",
        )
        .bind(t)
        .bind(Uuid::now_v7())
        .bind(editor.actor.user_id)
        .bind(pm.actor.user_id)
        .execute(&mut *tx)
        .await
    ));
    drop(tx);
    // Without a user nothing of a project is visible, even with its id in scope.
    let nobody = Scope {
        tenant: Some(t),
        user: None,
        project: Some(shared),
    };
    assert_eq!(
        count_in(&db, nobody, "select count(*) from kentos.project").await,
        0
    );
    assert_eq!(
        count_in(&db, nobody, "select count(*) from kentos.feature").await,
        0
    );

    // The server's role cannot open tenants, add members or change policies by itself.
    let pool = &db.app.pool;
    assert!(is_denied(
        sqlx::query(
            "insert into kentos.membership (tenant_id, user_id, role) values ($1, $2, 'owner')"
        )
        .bind(t)
        .bind(editor.actor.user_id)
        .execute(pool)
        .await
    ));
    assert!(is_denied(
        sqlx::query("update kentos.tenant set admins_access_all_projects = false")
            .execute(pool)
            .await
    ));
    db.close().await;
}

#[tokio::test]
async fn the_personal_space_opens_once_and_is_shared_by_project() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let ayse = account(&db, "ayse").await;
    let bora = account(&db, "bora").await;

    // Opened on first need, once, also when asked from many requests at once.
    let ids = futures_all(&db, &ayse).await;
    assert!(ids.windows(2).all(|w| w[0] == w[1]));
    let again = tenancy::ensure_personal(&db.app, &ayse).await.unwrap();
    assert_eq!(again, ids[0]);
    let (spaces, members, seats): (i64, i64, i64) = sqlx::query_as(
        "select (select count(*) from kentos.tenant where owner_user_id = $1),
                (select count(*) from kentos.membership where user_id = $1),
                (select count(*) from kentos.seat_allocation where user_id = $1)",
    )
    .bind(ayse.user_id)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert_eq!((spaces, members, seats), (1, 1, 1));
    let me = tenancy::memberships(&db.app, &ayse).await.unwrap();
    assert_eq!(me.len(), 1);
    assert_eq!(
        (me[0].tenant_kind, me[0].role, me[0].capabilities.clone()),
        (
            TenantKind::Personal,
            TenantRole::Owner,
            vec!["project.create".to_string()]
        )
    );

    // Its owner creates projects there; nobody else is a member, and none can be added.
    let home = personal(&db, &ayse).await;
    let project = new_project(&db, &home, "Bahçe").await;
    let owner = open(&db, &home, project).await;
    assert_eq!(
        (owner.role, owner.tenant_kind),
        (ProjectRole::Owner, TenantKind::Personal)
    );
    let slug: String = sqlx::query_scalar("select slug from kentos.tenant where id = $1")
        .bind(home.tenant)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    assert!(matches!(
        admin::set_membership(&db.owner, &slug, "bora", TenantRole::Editor, true).await,
        Err(AppError::Invalid(m)) if m.contains("kişisel alan")
    ));
    assert!(matches!(
        tenancy::access(&db.app, &bora, home.tenant).await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        try_open(&db, &bora, home.tenant, project).await,
        Err(AppError::NotFound(_))
    ));

    // Shared with any account: no membership needed (but an account it must be). It shows in both people's “Projelerim”.
    assert!(matches!(
        sharing::share(&db.app, &owner, share_envelope(&owner, Uuid::now_v7(), GrantRole::Viewer, None)).await,
        Err(AppError::NotFound(m)) if m.contains("hesap")
    ));
    share(&db, &owner, &bora, GrantRole::Editor).await;
    let bora_p = try_open(&db, &bora, home.tenant, project).await.unwrap();
    assert_eq!(
        (bora_p.role, bora_p.via),
        (ProjectRole::Editor, AccessSource::Grant)
    );
    changes::commit(
        &db.app,
        &bora_p,
        envelope_in(home.tenant, project, a_point(3.0), &[]),
    )
    .await
    .unwrap();
    let theirs = listing::mine(&db.app, &bora).await.unwrap();
    let listed = theirs
        .projects
        .iter()
        .find(|p| p.id == project.to_string())
        .expect("shared with bora");
    assert_eq!(
        (
            listed.tenant_kind,
            listed.access.via,
            listed.tenant_name.as_str()
        ),
        (TenantKind::Personal, AccessSource::Grant, "ayse")
    );
    let own = listing::mine(&db.app, &ayse).await.unwrap();
    assert_eq!(own.projects.len(), 1);
    assert_eq!(own.projects[0].access.via, AccessSource::Owner);
    // Bora got his own personal space by asking; nothing of ayse's leaks into it.
    let bora_home = personal(&db, &bora).await;
    assert_ne!(bora_home.tenant, home.tenant);
    assert!(
        listing::list(&db.app, &bora_home)
            .await
            .unwrap()
            .projects
            .is_empty()
    );
    // A guest cannot create projects in someone's personal space, not even with a scope made by hand.
    let mut tx = db
        .app
        .scoped(scope(home.tenant, &bora, None))
        .await
        .unwrap();
    assert!(is_denied(
        sqlx::query(
            "insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y, styles, created_by, owner_user_id)
             values ($1, $2, 'misafir', 5256, '{}', '[]', 'x', 0, 0, '{}', $3, $3)",
        )
        .bind(home.tenant)
        .bind(Uuid::now_v7())
        .bind(bora.user_id)
        .execute(&mut *tx)
        .await
    ));
    drop(tx);
    // Taken away: gone from his list and refused.
    revoke(&db, &owner, &bora).await;
    assert!(
        !listing::mine(&db.app, &bora)
            .await
            .unwrap()
            .projects
            .iter()
            .any(|p| p.id == project.to_string())
    );
    assert!(matches!(
        try_open(&db, &bora, home.tenant, project).await,
        Err(AppError::NotFound(_))
    ));
    db.close().await;
}

/// Eight requests opening the same personal space at once.
async fn futures_all(db: &TestDb, who: &Actor) -> Vec<Uuid> {
    let call = || tenancy::ensure_personal(&db.app, who);
    let (a, b, c, d, e, f, g, h) = tokio::join!(
        call(),
        call(),
        call(),
        call(),
        call(),
        call(),
        call(),
        call()
    );
    [a, b, c, d, e, f, g, h]
        .into_iter()
        .map(|r| r.unwrap())
        .collect()
}

#[tokio::test]
async fn my_projects_are_the_owned_and_the_shared_ones() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let boss = member(&db, "buro", "mudur", TenantRole::Admin).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let outsider = account(&db, "misafir").await;
    let org_project = new_project(&db, &pm, "Kurum projesi").await;
    let owner = open(&db, &pm, org_project).await;
    share(&db, &owner, &editor.actor, GrantRole::Viewer).await;
    let home = personal(&db, &pm.actor).await;
    let own_project = new_project(&db, &home, "Kendi işim").await;

    let names = |list: kentos_contracts::ProjectList| {
        let mut n: Vec<String> = list.projects.into_iter().map(|p| p.name).collect();
        n.sort();
        n
    };
    assert_eq!(
        names(listing::mine(&db.app, &pm.actor).await.unwrap()),
        ["Kendi işim", "Kurum projesi"]
    );
    assert_eq!(
        names(listing::mine(&db.app, &editor.actor).await.unwrap()),
        ["Kurum projesi"]
    );
    // What an admin reaches only through the policy is the organisation's list, not theirs.
    assert!(
        listing::mine(&db.app, &boss.actor)
            .await
            .unwrap()
            .projects
            .is_empty()
    );
    assert_eq!(
        listing::list(&db.app, &boss).await.unwrap().projects.len(),
        1
    );
    // An organisation's project is shared only with its members (guests come later).
    assert!(matches!(
        sharing::share(&db.app, &owner, share_envelope(&owner, outsider.user_id, GrantRole::Viewer, None)).await,
        Err(AppError::Invalid(m)) if m.contains("kurumunun üyesi değil")
    ));
    // A personal project is shared with anyone.
    let own = open(&db, &home, own_project).await;
    share(&db, &own, &outsider, GrantRole::Viewer).await;
    assert_eq!(
        names(listing::mine(&db.app, &outsider).await.unwrap()),
        ["Kendi işim"]
    );
    db.close().await;
}

#[tokio::test]
async fn sharing_rules_events_and_retries() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let partner = member(&db, "buro", "ortak", TenantRole::Editor).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &partner.actor, GrantRole::Manager).await;
    let manages = open(&db, &partner, project).await;

    // A manager shares; editors and viewers may not.
    share(&db, &manages, &editor.actor, GrantRole::Editor).await;
    let edits = open(&db, &editor, project).await;
    assert!(matches!(
        sharing::share(&db.app, &edits, share_envelope(&edits, viewer.actor.user_id, GrantRole::Viewer, None)).await,
        Err(AppError::Forbidden(m)) if m.contains("project.share")
    ));
    assert!(matches!(
        sharing::list(&db.app, &edits).await,
        Err(AppError::Forbidden(_))
    ));
    // Not one's own access, not the owner's, never in the past.
    for (user, role, until) in [
        (partner.actor.user_id, GrantRole::Editor, None),
        (pm.actor.user_id, GrantRole::Viewer, None),
        (
            viewer.actor.user_id,
            GrantRole::Viewer,
            Some("2020-01-01T00:00:00Z".to_string()),
        ),
        (
            viewer.actor.user_id,
            GrantRole::Viewer,
            Some("yarın".to_string()),
        ),
    ] {
        assert!(matches!(
            sharing::share(
                &db.app,
                &manages,
                share_envelope(&manages, user, role, until)
            )
            .await,
            Err(AppError::Invalid(_))
        ));
    }
    // Never an owner by sharing: the input does not even parse.
    let mut as_owner = share_envelope(&manages, viewer.actor.user_id, GrantRole::Viewer, None);
    as_owner.input["role"] = serde_json::json!("owner");
    assert!(matches!(
        sharing::share(&db.app, &manages, as_owner).await,
        Err(AppError::Invalid(m)) if m.contains("okunamadı")
    ));
    // An organisation's project: someone who is not a member (or no account at all) cannot be given a role.
    assert!(matches!(
        sharing::share(&db.app, &manages, share_envelope(&manages, Uuid::now_v7(), GrantRole::Viewer, None)).await,
        Err(AppError::Invalid(m)) if m.contains("üyesi değil")
    ));

    // The same role again changes nothing; a new one does, with an event and an audit record.
    let before = events::latest(&db.app, &owner).await.unwrap();
    let same = share(&db, &manages, &editor.actor, GrantRole::Editor).await;
    assert!(!same.changed && same.event_seq.is_none());
    assert_eq!(events::latest(&db.app, &owner).await.unwrap(), before);
    let lowered = share(&db, &manages, &editor.actor, GrantRole::Viewer).await;
    assert!(lowered.changed);
    let log = events::after(&db.app, &owner, before, 10).await.unwrap();
    assert_eq!(log.events.len(), 1);
    let event = &log.events[0];
    assert_eq!(
        (
            event.kind.as_str(),
            event.features.len(),
            event.meta,
            event.actor.clone()
        ),
        (
            PROJECT_ACCESS_CHANGED,
            0,
            false,
            Some(partner.actor.user_id.to_string())
        )
    );
    assert_eq!(Some(event.seq.clone()), lowered.event_seq);
    // A commit prepared while the grant was higher is checked again under the project's lock.
    assert!(matches!(
        changes::commit(&db.app, &edits, envelope(&editor, project, a_point(5.0), &[])).await,
        Err(AppError::Forbidden(m)) if m.contains("feature.write")
    ));

    // A retry with the same key gets the same answer and changes nothing more.
    let raise = share_envelope(&manages, viewer.actor.user_id, GrantRole::Commenter, None);
    let first = sharing::share(&db.app, &manages, raise.clone())
        .await
        .unwrap();
    let retried = sharing::share(&db.app, &manages, raise).await.unwrap();
    assert!(first.changed && !first.replayed && retried.replayed);
    assert_eq!(retried.event_seq, first.event_seq);
    // Revoking what is not there changes nothing; the owner sees the list of who has access.
    let gone = revoke(&db, &manages, &viewer.actor).await;
    assert!(gone.changed && gone.grant.is_none());
    let again = sharing::revoke(
        &db.app,
        &manages,
        revoke_envelope(&manages, viewer.actor.user_id),
    )
    .await
    .unwrap();
    assert!(!again.changed && again.event_seq.is_none());
    let list = sharing::list(&db.app, &owner).await.unwrap();
    assert_eq!(list.owner_id, pm.actor.user_id.to_string());
    let mut who: Vec<(String, GrantRole)> = list
        .grants
        .iter()
        .map(|g| (g.display_name.clone(), g.role))
        .collect();
    who.sort();
    assert_eq!(
        who,
        [
            ("editor".to_string(), GrantRole::Viewer),
            ("ortak".to_string(), GrantRole::Manager)
        ]
    );
    let audited: i64 = sqlx::query_scalar(
        "select count(*) from kentos.audit_event where project_id = $1 and action in ('project.share', 'project.access.revoke')",
    )
    .bind(project)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    // ortak, editor, editor lowered, viewer commenter, viewer revoked.
    assert_eq!(audited, 5);
    // A deleted project is not shared any more.
    lifecycle::delete(&db.app, &owner, None).await.unwrap();
    assert!(matches!(
        sharing::share(
            &db.app,
            &manages,
            share_envelope(&manages, viewer.actor.user_id, GrantRole::Viewer, None)
        )
        .await,
        Err(AppError::Deleted(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn existing_projects_keep_their_people_after_the_migration() {
    let Some(db) = TestDb::create_before(4).await else {
        return;
    };
    // The schema before project access: tenant roles alone decided.
    let tenant = admin::create_tenant(&db.owner, "eski", "Eski Büro", 10)
        .await
        .unwrap();
    let mut people = Vec::new();
    for (login, role, status, seat) in [
        ("patron", "owner", "active", true),
        ("mudur", "admin", "active", true),
        ("yonetici", "project_manager", "active", true),
        ("editor", "editor", "active", true),
        ("izleyici", "viewer", "active", true),
        ("ayrilan", "editor", "disabled", true),
        ("koltuksuz", "editor", "active", false),
    ] {
        let actor = account(&db, login).await;
        sqlx::query("insert into kentos.membership (tenant_id, user_id, role, status) values ($1, $2, $3, $4)")
            .bind(tenant)
            .bind(actor.user_id)
            .bind(role)
            .bind(status)
            .execute(&db.owner)
            .await
            .unwrap();
        if seat {
            sqlx::query("insert into kentos.seat_allocation (tenant_id, user_id) values ($1, $2)")
                .bind(tenant)
                .bind(actor.user_id)
                .execute(&db.owner)
                .await
                .unwrap();
        }
        people.push(actor);
    }
    let [boss, admin_, pm, editor, viewer, left, seatless] =
        <[Actor; 7]>::try_from(people).unwrap();
    let old_project = |name: &'static str, by: Uuid, deleted: bool| {
        let owner = db.owner.clone();
        async move {
            let id = Uuid::now_v7();
            sqlx::query(
                "insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y, styles, created_by, deleted_at, deleted_by)
                 values ($1, $2, $3, 5256, '{\"srid\":5256}', '[]', 'cizim', 0, 0, '{\"items\":[],\"categories\":[]}', $4,
                         case when $5 then now() end, case when $5 then $4 end)",
            )
            .bind(tenant)
            .bind(id)
            .bind(name)
            .bind(by)
            .bind(deleted)
            .execute(&owner)
            .await
            .unwrap();
            id
        }
    };
    let by_pm = old_project("Ada 101", pm.user_id, false).await;
    let by_admin = old_project("Ada 102", admin_.user_id, false).await;
    let deleted = old_project("Ada 103", pm.user_id, true).await;

    db.migrate().await;

    // The creator owns; members keep what their tenant role gave, now as grants; owners and admins through the policy.
    let role_of = |who: &Actor, project: Uuid| {
        let db = &db;
        let who = who.clone();
        async move {
            try_open(db, &who, tenant, project)
                .await
                .map(|a| (a.role, a.via))
        }
    };
    use AccessSource::{Grant, Owner, Policy};
    assert_eq!(
        role_of(&pm, by_pm).await.unwrap(),
        (ProjectRole::Owner, Owner)
    );
    assert_eq!(
        role_of(&pm, by_admin).await.unwrap(),
        (ProjectRole::Manager, Grant)
    );
    assert_eq!(
        role_of(&admin_, by_admin).await.unwrap(),
        (ProjectRole::Owner, Owner)
    );
    for who in [&boss, &admin_] {
        assert_eq!(
            role_of(who, by_pm).await.unwrap(),
            (ProjectRole::Manager, Policy)
        );
    }
    for project in [by_pm, by_admin] {
        assert_eq!(
            role_of(&editor, project).await.unwrap(),
            (ProjectRole::Editor, Grant)
        );
        assert_eq!(
            role_of(&viewer, project).await.unwrap(),
            (ProjectRole::Viewer, Grant)
        );
        // Status and seat still count on every use, as before the migration.
        assert!(matches!(
            role_of(&left, project).await,
            Err(AppError::Forbidden(_))
        ));
        assert!(matches!(
            role_of(&seatless, project).await,
            Err(AppError::Forbidden(_))
        ));
    }
    // The deleted project kept its people too: 410 for them.
    let had = try_open(&db, &editor, tenant, deleted).await.unwrap();
    assert!(
        had.deleted
            && matches!(
                projects::info(&db.app, &had).await,
                Err(AppError::Deleted(_))
            )
    );
    let editor_in = tenancy::access(&db.app, &editor, tenant).await.unwrap();
    assert_eq!(
        listing::list(&db.app, &editor_in)
            .await
            .unwrap()
            .projects
            .len(),
        2
    );
    // Recorded in each project's audit, and the members' grants are exact.
    let (audits, grants): (i64, i64) = sqlx::query_as(
        "select (select count(*) from kentos.audit_event where action = 'project.access.migrate'),
                (select count(*) from kentos.project_grant where granted_by is null)",
    )
    .fetch_one(&db.owner)
    .await
    .unwrap();
    // 3 projects × (editor, viewer, ayrilan, koltuksuz) + the project manager on the two projects not theirs.
    assert_eq!((audits, grants), (3, 3 * 4 + 1));

    // From now on a new project is its creator's (and the policy's) until shared.
    let pm_in = tenancy::access(&db.app, &pm, tenant).await.unwrap();
    let fresh = new_project(&db, &pm_in, "Ada 104").await;
    assert!(matches!(
        role_of(&editor, fresh).await,
        Err(AppError::NotFound(_))
    ));
    assert_eq!(
        role_of(&boss, fresh).await.unwrap(),
        (ProjectRole::Manager, Policy)
    );
    db.close().await;
}
