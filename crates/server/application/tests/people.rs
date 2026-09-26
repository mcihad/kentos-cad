//! The people of a project's share dialog against a real PostgreSQL/PostGIS
//! (docs/adr/0015, TODOS.md CLOUD-12, CLOUD-21): the access list says what
//! each person works with and why, exactly as the server decides when that
//! person opens the project; a search for people to share with finds only
//! those the caller may see, never another organisation's members or a
//! stranger's account.

mod common;

use common::{
    PASSWORD, account, member, new_project, open, personal, share, share_envelope, try_open,
};
use kentos_application::identity::{self, Actor};
use kentos_application::tenancy::Access;
use kentos_application::{AppError, admin, lifecycle, people, sharing, tenancy};
use kentos_contracts::{
    AccessBlock, AccessSource, GrantRole, ProjectAccessHolder, ProjectRole, ProjectStorage,
    SignInMethod, TenantKind, TenantRole,
};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

/// A new account with a name and e-mail of its own and this role (and a seat) in `tenant`.
async fn named(
    db: &TestDb,
    tenant: &str,
    login: &str,
    name: &str,
    email: Option<&str>,
    role: TenantRole,
) -> Access {
    let user = admin::create_local_user(&db.owner, login, name, email, PASSWORD)
        .await
        .unwrap();
    admin::set_membership(&db.owner, tenant, login, role, true)
        .await
        .unwrap();
    let actor = identity::actor_of(&db.app, user, SignInMethod::Local)
        .await
        .unwrap()
        .unwrap();
    let tenant_id: Uuid = sqlx::query_scalar("select id from kentos.tenant where slug = $1")
        .bind(tenant)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    tenancy::access(&db.app, &actor, tenant_id).await.unwrap()
}

fn holder<'a>(list: &'a [ProjectAccessHolder], who: &Actor) -> Option<&'a ProjectAccessHolder> {
    list.iter().find(|p| p.user_id == who.user_id.to_string())
}

fn names(found: &kentos_contracts::ShareCandidates) -> Vec<&str> {
    found
        .candidates
        .iter()
        .map(|c| c.display_name.as_str())
        .collect()
}

/// Every listed person's role is the one the server gives them when they open the project.
async fn agrees_with_opening(
    db: &TestDb,
    tenant: Uuid,
    project: Uuid,
    list: &[ProjectAccessHolder],
) {
    for p in list {
        let user = Uuid::parse_str(&p.user_id).unwrap();
        let Some(actor) = identity::actor_of(&db.app, user, SignInMethod::Local)
            .await
            .unwrap()
        else {
            // A disabled account cannot sign in at all.
            assert_eq!(p.blocked, Some(AccessBlock::Inactive), "{}", p.display_name);
            continue;
        };
        match try_open(db, &actor, tenant, project).await {
            Ok(a) => assert_eq!(
                (p.role, p.via, p.blocked),
                (Some(a.role), Some(a.via), None),
                "{}",
                p.display_name
            ),
            Err(AppError::NotFound(_)) => {
                assert!(p.role.is_none(), "{}", p.display_name);
                assert!(
                    matches!(
                        p.blocked,
                        Some(AccessBlock::Expired | AccessBlock::NotMember)
                    ),
                    "{}: {:?}",
                    p.display_name,
                    p.blocked
                );
            }
            Err(AppError::Forbidden(_)) => {
                assert!(p.role.is_none(), "{}", p.display_name);
                assert!(
                    matches!(p.blocked, Some(AccessBlock::Inactive | AccessBlock::NoSeat)),
                    "{}: {:?}",
                    p.display_name,
                    p.blocked
                );
            }
            Err(e) => panic!("{}: {e}", p.display_name),
        }
    }
}

#[tokio::test]
async fn the_access_list_says_who_works_with_what_and_why() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 20)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 3)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let boss = member(&db, "buro", "patron", TenantRole::Owner).await;
    let admin_ = member(&db, "buro", "mudur", TenantRole::Admin).await;
    let admin_viewer = member(&db, "buro", "mudur2", TenantRole::Admin).await;
    let away_admin = member(&db, "buro", "mudur3", TenantRole::Admin).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let timed = member(&db, "buro", "sureli", TenantRole::Editor).await;
    let ended = member(&db, "buro", "bitmis", TenantRole::Viewer).await;
    let left = member(&db, "buro", "ayrilan", TenantRole::Editor).await;
    let seatless = member(&db, "buro", "koltuksuz", TenantRole::Editor).await;
    let disabled = member(&db, "buro", "kapali", TenantRole::Editor).await;
    let unshared = member(&db, "buro", "komsu", TenantRole::Editor).await;
    let stranger = member(&db, "diger", "yabanci", TenantRole::Owner).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    for (who, role) in [
        (&editor, GrantRole::Editor),
        (&admin_viewer, GrantRole::Viewer),
        (&ended, GrantRole::Viewer),
        (&left, GrantRole::Editor),
        (&seatless, GrantRole::Editor),
        (&disabled, GrantRole::Commenter),
    ] {
        share(&db, &owner, &who.actor, role).await;
    }
    sharing::share(
        &db.app,
        &owner,
        share_envelope(
            &owner,
            timed.actor.user_id,
            GrantRole::Manager,
            Some("2999-01-01T00:00:00Z".into()),
        ),
    )
    .await
    .unwrap();
    // What the operator and time did meanwhile.
    for (sql, who) in [
        (
            "update kentos.project_grant set expires_at = now() - interval '1 minute' where user_id = $1",
            &ended,
        ),
        (
            "update kentos.membership set status = 'disabled' where user_id = $1",
            &left,
        ),
        (
            "delete from kentos.seat_allocation where user_id = $1",
            &seatless,
        ),
        (
            "update kentos.app_user set status = 'disabled' where id = $1",
            &disabled,
        ),
        (
            "update kentos.membership set status = 'disabled' where user_id = $1",
            &away_admin,
        ),
    ] {
        sqlx::query(sql)
            .bind(who.actor.user_id)
            .execute(&db.owner)
            .await
            .unwrap();
    }

    let list = people::list(&db.app, &owner).await.unwrap();
    assert_eq!(
        (
            list.tenant_kind,
            list.storage,
            list.admins_access_all_projects,
            list.owner_id.as_str()
        ),
        (
            TenantKind::Organization,
            ProjectStorage::Database,
            true,
            pm.actor.user_id.to_string().as_str()
        )
    );
    // The owner first.
    assert_eq!(list.people[0].user_id, pm.actor.user_id.to_string());
    let expect = |who: &Access,
                  role: Option<ProjectRole>,
                  via: Option<AccessSource>,
                  blocked: Option<AccessBlock>,
                  grant: Option<GrantRole>| {
        let p = holder(&list.people, &who.actor)
            .unwrap_or_else(|| panic!("{} is not listed", who.actor.display_name));
        assert_eq!(
            (p.role, p.via, p.blocked, p.grant),
            (role, via, blocked, grant),
            "{}",
            who.actor.display_name
        );
    };
    use AccessSource::{Grant, Owner, Policy};
    use ProjectRole as R;
    expect(&pm, Some(R::Owner), Some(Owner), None, None);
    expect(&boss, Some(R::Manager), Some(Policy), None, None);
    expect(&admin_, Some(R::Manager), Some(Policy), None, None);
    // An admin with a grant works through the policy; the grant is still theirs to change.
    expect(
        &admin_viewer,
        Some(R::Manager),
        Some(Policy),
        None,
        Some(GrantRole::Viewer),
    );
    expect(
        &editor,
        Some(R::Editor),
        Some(Grant),
        None,
        Some(GrantRole::Editor),
    );
    expect(
        &timed,
        Some(R::Manager),
        Some(Grant),
        None,
        Some(GrantRole::Manager),
    );
    expect(
        &ended,
        None,
        None,
        Some(AccessBlock::Expired),
        Some(GrantRole::Viewer),
    );
    expect(
        &left,
        None,
        None,
        Some(AccessBlock::Inactive),
        Some(GrantRole::Editor),
    );
    expect(
        &seatless,
        None,
        None,
        Some(AccessBlock::NoSeat),
        Some(GrantRole::Editor),
    );
    expect(
        &disabled,
        None,
        None,
        Some(AccessBlock::Inactive),
        Some(GrantRole::Commenter),
    );
    let t = holder(&list.people, &timed.actor).unwrap();
    assert_eq!(
        (t.expires_at.as_deref(), t.expired),
        (Some("2999-01-01T00:00:00Z"), false)
    );
    assert!(holder(&list.people, &ended.actor).unwrap().expired);
    // Nobody the project has nothing to do with: an unshared member, an admin the policy does not
    // reach now (membership off, no grant), another organisation's member.
    for who in [&unshared, &away_admin, &stranger] {
        assert!(
            holder(&list.people, &who.actor).is_none(),
            "{}",
            who.actor.display_name
        );
    }
    assert_eq!(list.people.len(), 10);
    agrees_with_opening(&db, pm.tenant, project, &list.people).await;

    // The policy off: owners and admins only through what is shared with them.
    admin::set_tenant_policy(&db.owner, "buro", Some(false), None)
        .await
        .unwrap();
    let list = people::list(&db.app, &owner).await.unwrap();
    assert!(!list.admins_access_all_projects);
    assert!(
        holder(&list.people, &boss.actor).is_none()
            && holder(&list.people, &admin_.actor).is_none()
    );
    expect_after_policy(&list.people, &admin_viewer.actor);
    agrees_with_opening(&db, pm.tenant, project, &list.people).await;

    // Only those who may share see it; a deleted project answers 410 to them.
    let edits = open(&db, &editor, project).await;
    assert!(matches!(
        people::list(&db.app, &edits).await,
        Err(AppError::Forbidden(m)) if m.contains("project.share")
    ));
    lifecycle::delete(&db.app, &owner, None).await.unwrap();
    let after = open(&db, &pm, project).await;
    assert!(matches!(
        people::list(&db.app, &after).await,
        Err(AppError::Deleted(_))
    ));
    db.close().await;
}

fn expect_after_policy(list: &[ProjectAccessHolder], who: &Actor) {
    let p = holder(list, who).unwrap();
    assert_eq!(
        (p.role, p.via, p.grant),
        (
            Some(ProjectRole::Viewer),
            Some(AccessSource::Grant),
            Some(GrantRole::Viewer)
        )
    );
}

#[tokio::test]
async fn a_personal_space_lists_its_person_and_the_people_it_is_shared_with() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let ayse = account(&db, "ayse").await;
    let bora = account(&db, "bora").await;
    let home = personal(&db, &ayse).await;
    let project = new_project(&db, &home, "Bahçe").await;
    let owner = open(&db, &home, project).await;
    share(&db, &owner, &bora, GrantRole::Editor).await;
    let list = people::list(&db.app, &owner).await.unwrap();
    assert_eq!(
        (
            list.tenant_kind,
            list.admins_access_all_projects,
            list.people.len()
        ),
        (TenantKind::Personal, false, 2)
    );
    assert_eq!(
        (list.people[0].display_name.as_str(), list.people[0].role),
        ("ayse", Some(ProjectRole::Owner))
    );
    // A guest needs no membership of the space.
    assert_eq!(
        (
            list.people[1].display_name.as_str(),
            list.people[1].role,
            list.people[1].via
        ),
        ("bora", Some(ProjectRole::Editor), Some(AccessSource::Grant))
    );
    agrees_with_opening(&db, home.tenant, project, &list.people).await;
    db.close().await;
}

#[tokio::test]
async fn a_search_finds_only_people_the_caller_may_see() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    for (slug, name) in [("buro", "Büro"), ("diger", "Diğer"), ("ucuncu", "Üçüncü")] {
        admin::create_tenant(&db.owner, slug, name, 10)
            .await
            .unwrap();
    }
    let pm = named(
        &db,
        "buro",
        "ayse",
        "Ayşe Yılmaz",
        None,
        TenantRole::ProjectManager,
    )
    .await;
    let mehmet = named(
        &db,
        "buro",
        "mehmet",
        "Mehmet Demir",
        Some("mehmet.demir@ornek.com.tr"),
        TenantRole::Editor,
    )
    .await;
    let seyma = named(&db, "buro", "seyma", "Şeyma IŞIK", None, TenantRole::Viewer).await;
    let off = named(
        &db,
        "buro",
        "mehmetali",
        "Mehmet Ali",
        None,
        TenantRole::Viewer,
    )
    .await;
    let zeynep_in = named(
        &db,
        "buro",
        "zeynep",
        "Zeynep Kaya",
        None,
        TenantRole::Admin,
    )
    .await;
    // Ayşe is also a member of another organisation; a third one she has nothing to do with.
    admin::set_membership(&db.owner, "diger", "ayse", TenantRole::Editor, true)
        .await
        .unwrap();
    let other = named(
        &db,
        "diger",
        "mbaska",
        "Mehmet Başka",
        None,
        TenantRole::Owner,
    )
    .await;
    named(
        &db,
        "ucuncu",
        "mucuncu",
        "Mehmet Üçüncü",
        None,
        TenantRole::Owner,
    )
    .await;
    // An account that shares no organisation with anyone.
    admin::create_local_user(
        &db.owner,
        "myalniz",
        "Mehmet Yalnız",
        Some("yalniz@ornek.com.tr"),
        PASSWORD,
    )
    .await
    .unwrap();
    sqlx::query("update kentos.membership set status = 'disabled' where user_id = $1")
        .bind(off.actor.user_id)
        .execute(&db.owner)
        .await
        .unwrap();

    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    let search = |a: &kentos_application::access::ProjectAccess, q: &'static str| {
        let db = &db;
        let a = a.clone();
        async move { people::candidates(&db.app, &a, q).await }
    };
    // An organisation's project: its active members only.
    assert_eq!(
        names(&search(&owner, "mehmet").await.unwrap()),
        ["Mehmet Demir"]
    );
    // By e-mail, and with Turkish letters folded both ways.
    assert_eq!(
        names(&search(&owner, "ornek.com").await.unwrap()),
        ["Mehmet Demir"]
    );
    assert_eq!(
        names(&search(&owner, "seyma isik").await.unwrap()),
        ["Şeyma IŞIK"]
    );
    assert_eq!(
        names(&search(&owner, "IŞIK").await.unwrap()),
        ["Şeyma IŞIK"]
    );
    assert_eq!(names(&search(&owner, "Şey").await.unwrap()), ["Şeyma IŞIK"]);
    let found = search(&owner, "demir").await.unwrap();
    assert_eq!(
        (
            found.candidates[0].user_id.clone(),
            found.candidates[0].email.as_deref()
        ),
        (
            mehmet.actor.user_id.to_string(),
            Some("mehmet.demir@ornek.com.tr")
        )
    );
    // Not the caller, not the owner (the same here), and wildcards are letters.
    assert!(search(&owner, "ayse").await.unwrap().candidates.is_empty());
    for q in ["%%", "__", "%_"] {
        assert!(
            search(&owner, q).await.unwrap().candidates.is_empty(),
            "{q}"
        );
    }
    // Too short to be a search.
    assert!(matches!(
        search(&owner, "m").await,
        Err(AppError::Invalid(_))
    ));

    // Someone else who may share (the admin, through the policy) finds neither the owner nor themselves.
    let zeynep = open(&db, &zeynep_in, project).await;
    assert!(
        search(&zeynep, "ayse").await.unwrap().candidates.is_empty(),
        "the owner"
    );
    assert!(
        search(&zeynep, "zeynep")
            .await
            .unwrap()
            .candidates
            .is_empty(),
        "the caller"
    );
    assert_eq!(
        names(&search(&zeynep, "mehmet").await.unwrap()),
        ["Mehmet Demir"]
    );

    // Who may not share may not search: an editor gets 403 (they see the project); a stranger never gets this far.
    share(&db, &owner, &mehmet.actor, GrantRole::Editor).await;
    let edits = open(&db, &mehmet, project).await;
    assert!(matches!(
        search(&edits, "zeynep").await,
        Err(AppError::Forbidden(m)) if m.contains("project.share")
    ));
    assert!(matches!(
        try_open(&db, &other.actor, pm.tenant, project).await,
        Err(AppError::NotFound(_))
    ));

    // A personal project: the members of the caller's own organisations, never a third one's or a stranger's.
    let home = personal(&db, &pm.actor).await;
    let own = new_project(&db, &home, "Kendi işim").await;
    let own = open(&db, &home, own).await;
    assert_eq!(
        names(&search(&own, "mehmet").await.unwrap()),
        ["Mehmet Başka", "Mehmet Demir"]
    );
    assert!(search(&own, "yalniz").await.unwrap().candidates.is_empty());
    assert!(search(&own, "ucuncu").await.unwrap().candidates.is_empty());
    // An organisation she may not use now (no seat) is not looked in.
    sqlx::query("delete from kentos.seat_allocation where user_id = $1 and tenant_id = (select id from kentos.tenant where slug = 'diger')")
        .bind(pm.actor.user_id)
        .execute(&db.owner)
        .await
        .unwrap();
    assert_eq!(
        names(&search(&own, "mehmet").await.unwrap()),
        ["Mehmet Demir"]
    );
    // Whoever it finds, sharing with them works (a personal project takes any account).
    let found = search(&own, "seyma").await.unwrap();
    let to = Uuid::parse_str(&found.candidates[0].user_id).unwrap();
    assert_eq!(to, seyma.actor.user_id);
    assert!(
        sharing::share(
            &db.app,
            &own,
            share_envelope(&own, to, GrantRole::Viewer, None)
        )
        .await
        .unwrap()
        .changed
    );
    // A deleted project is not searched.
    lifecycle::delete(&db.app, &owner, None).await.unwrap();
    let after = open(&db, &pm, project).await;
    assert!(matches!(
        search(&after, "mehmet").await,
        Err(AppError::Deleted(_))
    ));
    db.close().await;
}
