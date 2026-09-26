//! Identity, tenancy and row-level security against a real PostgreSQL
//! (a throwaway database per test; skipped when no local server is set up).

use kentos_application::identity::{self, LOCAL_ISSUER};
use kentos_application::{AppError, admin, tenancy};
use kentos_contracts::{SignInMethod, TenantKind, TenantRole};
use kentos_postgres::Scope;
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

const PASSWORD: &str = "dogru-parola-123";

async fn two_tenants(db: &TestDb) -> (Uuid, Uuid, Uuid, Uuid) {
    let a = admin::create_tenant(&db.owner, "buro-a", "Büro A", 2)
        .await
        .unwrap();
    let b = admin::create_tenant(&db.owner, "buro-b", "Büro B", 2)
        .await
        .unwrap();
    let ayse = admin::create_local_user(&db.owner, "ayse", "Ayşe", None, PASSWORD)
        .await
        .unwrap();
    let bora = admin::create_local_user(
        &db.owner,
        "bora",
        "Bora",
        Some("bora@example.org"),
        PASSWORD,
    )
    .await
    .unwrap();
    admin::set_membership(&db.owner, "buro-a", "ayse", TenantRole::Editor, true)
        .await
        .unwrap();
    admin::set_membership(&db.owner, "buro-b", "bora", TenantRole::Editor, true)
        .await
        .unwrap();
    (a, b, ayse, bora)
}

#[tokio::test]
async fn row_level_security_keeps_tenants_apart() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (a, b, ayse, _) = two_tenants(&db).await;
    let count = |scope: Scope, sql: &'static str| {
        let app = db.app.clone();
        async move {
            let mut tx = app.scoped(scope).await.unwrap();
            let n: i64 = sqlx::query_scalar(sql).fetch_one(&mut *tx).await.unwrap();
            tx.commit().await.unwrap();
            n
        }
    };
    // Scoped to A: only A's tenant row and memberships.
    let in_a = Scope {
        tenant: Some(a),
        user: Some(ayse),
        project: None,
    };
    assert_eq!(count(in_a, "select count(*) from kentos.tenant").await, 1);
    assert_eq!(
        count(in_a, "select count(*) from kentos.membership").await,
        1
    );
    // Project rows follow the tenant scope and the user's role in them (the owner here). (The
    // server sets a tenant scope only after checking the membership or the project access; one's
    // own tenant rows stay visible for /v1/me.)
    sqlx::query(
        "insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y, styles, created_by, owner_user_id)
         values ($1, $2, 'A projesi', 5256, '{}', '[]', 'x', 0, 0, '{}', $3, $3)",
    )
    .bind(a)
    .bind(Uuid::now_v7())
    .bind(ayse)
    .execute(&db.owner)
    .await
    .unwrap();
    assert_eq!(count(in_a, "select count(*) from kentos.project").await, 1);
    let in_b = Scope {
        tenant: Some(b),
        user: Some(ayse),
        project: None,
    };
    assert_eq!(count(in_b, "select count(*) from kentos.project").await, 0);
    assert_eq!(count(in_b, "select count(*) from kentos.membership where tenant_id <> kentos.current_tenant() and user_id <> kentos.current_user_id()").await, 0);
    // Writing a row for another tenant than the scope is refused by the policy.
    let mut tx = db.app.scoped(in_b).await.unwrap();
    let wrong = sqlx::query(
        "insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y, styles, created_by, owner_user_id)
         values ($1, $2, 'sızma', 5256, '{}', '[]', 'x', 0, 0, '{}', $3, $3)",
    )
    .bind(a)
    .bind(Uuid::now_v7())
    .bind(ayse)
    .execute(&mut *tx)
    .await;
    assert!(matches!(wrong, Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42501")));
    drop(tx);
    // Without a scope nothing tenant-bound is visible, and a scope ends with its transaction.
    assert_eq!(
        count(Scope::default(), "select count(*) from kentos.membership").await,
        0
    );
    let n: i64 = sqlx::query_scalar("select count(*) from kentos.membership")
        .fetch_one(&db.app.pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
    db.close().await;
}

#[tokio::test]
async fn the_server_role_cannot_read_hashes_or_administer() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    two_tenants(&db).await;
    let denied = |r: Result<sqlx::postgres::PgQueryResult, sqlx::Error>| matches!(r, Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42501"));
    let pool = &db.app.pool;
    assert!(denied(
        sqlx::query("select password_hash from kentos.local_credential")
            .execute(pool)
            .await
    ));
    assert!(denied(sqlx::query("insert into kentos.tenant (id, slug, name, seat_limit) values (gen_random_uuid(), 'x-y', 'X', 1)").execute(pool).await));
    assert!(denied(
        sqlx::query("update kentos.membership set role = 'owner'")
            .execute(pool)
            .await
    ));
    assert!(denied(
        sqlx::query("delete from kentos.audit_event")
            .execute(pool)
            .await
    ));
    // Deleting a project only marks it (lifecycle.rs): its rows can never be removed by the server.
    assert!(denied(
        sqlx::query("delete from kentos.project")
            .execute(pool)
            .await
    ));
    // Old events go only through kentos.prune_outbox (at least an hour kept), never by hand.
    assert!(denied(
        sqlx::query("delete from kentos.outbox_event")
            .execute(pool)
            .await
    ));
    assert!(denied(
        sqlx::query("update kentos.outbox_horizon set pruned_through = 0")
            .execute(pool)
            .await
    ));
    // Its role is not the owner and cannot bypass row-level security.
    let (bypass, owner): (bool, bool) = sqlx::query_as(
        "select rolbypassrls, pg_has_role(current_user, 'kentos_cad_owner', 'member') from pg_roles where rolname = current_user",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(!bypass && !owner);
    db.close().await;
}

#[tokio::test]
async fn local_login_and_sessions() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_, _, ayse, _) = two_tenants(&db).await;
    assert_eq!(
        identity::check_local_login(&db.app, "ayse", PASSWORD)
            .await
            .unwrap(),
        Some(ayse)
    );
    assert_eq!(
        identity::check_local_login(&db.app, "AYSE", PASSWORD)
            .await
            .unwrap(),
        Some(ayse)
    );
    assert_eq!(
        identity::check_local_login(&db.app, "ayse", "yanlis-parola-1")
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        identity::check_local_login(&db.app, "yok", PASSWORD)
            .await
            .unwrap(),
        None
    );

    let token = identity::open_session(&db.app, ayse, SignInMethod::Local)
        .await
        .unwrap();
    assert_eq!(token.len(), 64);
    let actor = identity::session_actor(&db.app, &token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (actor.user_id, actor.display_name.as_str(), actor.method),
        (ayse, "Ayşe", SignInMethod::Local)
    );
    // Only the hash is stored.
    let stored: i64 = sqlx::query_scalar(
        "select count(*) from kentos.auth_session where token_hash = convert_to($1, 'UTF8')",
    )
    .bind(&token)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert_eq!(stored, 0);
    assert!(
        identity::session_actor(&db.app, "bozuk")
            .await
            .unwrap()
            .is_none()
    );
    identity::close_session(&db.app, &token).await.unwrap();
    assert!(
        identity::session_actor(&db.app, &token)
            .await
            .unwrap()
            .is_none()
    );

    // An expired session and a disabled account are refused.
    let late = identity::open_session(&db.app, ayse, SignInMethod::Local)
        .await
        .unwrap();
    sqlx::query("update kentos.auth_session set expires_at = now() - interval '1 second'")
        .execute(&db.owner)
        .await
        .unwrap();
    assert!(
        identity::session_actor(&db.app, &late)
            .await
            .unwrap()
            .is_none()
    );
    let fresh = identity::open_session(&db.app, ayse, SignInMethod::Local)
        .await
        .unwrap();
    sqlx::query("update kentos.app_user set status = 'disabled' where id = $1")
        .bind(ayse)
        .execute(&db.owner)
        .await
        .unwrap();
    assert!(
        identity::session_actor(&db.app, &fresh)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        identity::check_local_login(&db.app, "ayse", PASSWORD)
            .await
            .unwrap(),
        None
    );

    // A new password ends open sessions.
    sqlx::query("update kentos.app_user set status = 'active' where id = $1")
        .bind(ayse)
        .execute(&db.owner)
        .await
        .unwrap();
    let before = identity::open_session(&db.app, ayse, SignInMethod::Local)
        .await
        .unwrap();
    admin::set_password(&db.owner, "ayse", "yeni-parola-456")
        .await
        .unwrap();
    assert!(
        identity::session_actor(&db.app, &before)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        identity::check_local_login(&db.app, "ayse", "yeni-parola-456")
            .await
            .unwrap(),
        Some(ayse)
    );
    db.close().await;
}

#[tokio::test]
async fn access_needs_an_active_membership_and_a_seat() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (a, b, ayse, _) = two_tenants(&db).await;
    let actor = identity::actor_of(&db.app, ayse, SignInMethod::Local)
        .await
        .unwrap()
        .unwrap();
    let access = tenancy::access(&db.app, &actor, a).await.unwrap();
    assert_eq!(access.role, TenantRole::Editor);
    // An editor creates no projects; what they do in a project is that project's (access.rs).
    assert!(matches!(
        access.require(tenancy::Capability::ProjectCreate),
        Err(AppError::Forbidden(m)) if m.contains("project.create")
    ));
    // Another tenant is "not found", not "forbidden": its existence is not revealed.
    assert!(matches!(
        tenancy::access(&db.app, &actor, b).await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        tenancy::access(&db.app, &actor, Uuid::now_v7()).await,
        Err(AppError::NotFound(_))
    ));

    let me = tenancy::memberships(&db.app, &actor).await.unwrap();
    assert_eq!(me.len(), 1);
    assert_eq!(
        (me[0].tenant_slug.as_str(), me[0].seat, me[0].active),
        ("buro-a", true, true)
    );
    // Tenant-level rights only: an editor has none of them.
    assert!(me[0].capabilities.is_empty());
    assert_eq!(me[0].tenant_kind, TenantKind::Organization);
    admin::set_membership(
        &db.owner,
        "buro-a",
        "ayse",
        TenantRole::ProjectManager,
        true,
    )
    .await
    .unwrap();
    assert_eq!(
        tenancy::memberships(&db.app, &actor).await.unwrap()[0].capabilities,
        vec!["project.create".to_string()]
    );

    // Without a seat, or with a disabled membership, there is no access.
    sqlx::query("delete from kentos.seat_allocation where user_id = $1")
        .bind(ayse)
        .execute(&db.owner)
        .await
        .unwrap();
    assert!(matches!(
        tenancy::access(&db.app, &actor, a).await,
        Err(AppError::Forbidden(_))
    ));
    assert!(
        tenancy::memberships(&db.app, &actor).await.unwrap()[0]
            .capabilities
            .is_empty()
    );
    admin::set_membership(&db.owner, "buro-a", "ayse", TenantRole::Editor, true)
        .await
        .unwrap();
    sqlx::query("update kentos.membership set status = 'disabled' where user_id = $1")
        .bind(ayse)
        .execute(&db.owner)
        .await
        .unwrap();
    assert!(matches!(
        tenancy::access(&db.app, &actor, a).await,
        Err(AppError::Forbidden(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn seats_stop_at_the_tenant_limit() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "kucuk", "Küçük büro", 1)
        .await
        .unwrap();
    for login in ["bir", "iki"] {
        admin::create_local_user(&db.owner, login, login, None, PASSWORD)
            .await
            .unwrap();
    }
    admin::set_membership(&db.owner, "kucuk", "bir", TenantRole::Owner, true)
        .await
        .unwrap();
    // The same seat again is fine; a second person does not fit, and nothing of that attempt stays.
    admin::set_membership(&db.owner, "kucuk", "bir", TenantRole::Owner, true)
        .await
        .unwrap();
    let full = admin::set_membership(&db.owner, "kucuk", "iki", TenantRole::Viewer, true).await;
    assert!(
        matches!(full, Err(AppError::Invalid { message: m, .. }) if m.contains("koltuğunun hepsi dolu"))
    );
    let members = admin::list_members(&db.owner, "kucuk").await.unwrap();
    assert_eq!(members.len(), 1);
    // A membership without a seat is allowed.
    admin::set_membership(&db.owner, "kucuk", "iki", TenantRole::Viewer, false)
        .await
        .unwrap();
    assert_eq!(
        admin::list_members(&db.owner, "kucuk").await.unwrap().len(),
        2
    );
    assert!(matches!(
        admin::create_tenant(&db.owner, "kucuk", "Aynı", 1).await,
        Err(AppError::Invalid { .. })
    ));
    assert!(matches!(
        admin::create_tenant(&db.owner, "Büyük", "Harf", 1).await,
        Err(AppError::Invalid { .. })
    ));
    assert!(matches!(
        admin::create_local_user(&db.owner, "bir", "Tekrar", None, PASSWORD).await,
        Err(AppError::Invalid { .. })
    ));
    db.close().await;
}

#[tokio::test]
async fn openid_identities_are_found_by_issuer_and_subject() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let first = identity::resolve_identity(
        &db.app,
        "https://kimlik.example.org",
        "abc",
        "Ayla",
        Some("ayla@example.org"),
    )
    .await
    .unwrap();
    let again = identity::resolve_identity(
        &db.app,
        "https://kimlik.example.org",
        "abc",
        "Ayla K.",
        None,
    )
    .await
    .unwrap();
    assert!(first.is_some() && first == again);
    // The same subject at another issuer is another person; e-mail is not an identity.
    let other = identity::resolve_identity(
        &db.app,
        "https://baska.example.org",
        "abc",
        "Ayla",
        Some("ayla@example.org"),
    )
    .await
    .unwrap();
    assert_ne!(other, first);
    assert!(
        identity::resolve_identity(&db.app, LOCAL_ISSUER, "x", "x", None)
            .await
            .is_err()
    );
    let name: String = sqlx::query_scalar("select display_name from kentos.app_user where id = $1")
        .bind(first)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    assert_eq!(name, "Ayla K.");
    db.close().await;
}
