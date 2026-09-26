//! Invitations by link and guests (docs/adr/0035): an invitation opens a
//! project to the account of its verified e-mail once; someone outside the
//! organisation becomes a guest who reaches only that project, while the
//! organisation takes guests; members get plain grants and keep a stronger
//! role. Real database (`KENTOS_TEST_DB=required` makes a missing one a
//! failure).

mod common;

use common::{PASSWORD, a_point, catalog_envelope, member, new_project, open, personal};
use kentos_application::access::ProjectAccess;
use kentos_application::commands::{CatalogPolicy, CommandOutcome, run};
use kentos_application::identity::{self, Actor};
use kentos_application::{AppError, admin, changes, invitations, listing, people, tenancy};
use kentos_contracts::{
    AccessBlock, GrantRole, InvitationAccept, InvitationChange, InvitationState,
    PROJECT_INVITATION_REVOKE, PROJECT_INVITE, ProjectRole, SignInMethod, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

/// A local account with an e-mail (an administrator's, so verified), a
/// member of `tenant` with `role` when one is given.
async fn person(
    db: &TestDb,
    login: &str,
    email: &str,
    tenant: Option<(&str, TenantRole)>,
) -> Actor {
    let user = admin::create_local_user(&db.owner, login, login, Some(email), PASSWORD)
        .await
        .unwrap();
    if let Some((slug, role)) = tenant {
        admin::set_membership(&db.owner, slug, login, role, true)
            .await
            .unwrap();
    }
    identity::actor_of(&db.app, user, SignInMethod::Local)
        .await
        .unwrap()
        .unwrap()
}

async fn invite(
    db: &TestDb,
    by: &ProjectAccess,
    input: serde_json::Value,
) -> Result<InvitationChange, AppError> {
    let envelope = catalog_envelope(by, PROJECT_INVITE, input, &[]);
    match run(
        &db.app,
        &common::blobs(),
        &CatalogPolicy::default(),
        by,
        envelope,
    )
    .await?
    {
        CommandOutcome::Invitation(c) => Ok(c),
        other => panic!("not an invitation: {other:?}"),
    }
}

async fn accept(
    db: &TestDb,
    who: &Actor,
    token: &str,
) -> Result<kentos_contracts::InvitationAccepted, AppError> {
    invitations::accept(
        &db.app,
        who,
        InvitationAccept {
            token: token.into(),
        },
    )
    .await
}

#[tokio::test]
async fn an_invitation_opens_one_project_to_a_guest() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let buro = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 2)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = new_project(&db, &ayse, "Ada 101").await;
    let other = new_project(&db, &ayse, "Başka proje").await;
    let by = open(&db, &ayse, project).await;
    let can = person(
        &db,
        "can",
        "can@diger.org",
        Some(("diger", TenantRole::Owner)),
    )
    .await;

    let made = invite(
        &db,
        &by,
        json!({ "email": " Can@Diger.ORG ", "role": "editor" }),
    )
    .await
    .unwrap();
    let token = made
        .token
        .clone()
        .expect("the first answer carries the token");
    assert_eq!(token.len(), 64);
    let i = &made.invitation;
    assert_eq!(
        (i.email.as_str(), i.role, i.state),
        ("can@diger.org", GrantRole::Editor, InvitationState::Pending)
    );

    // Accepted by the account of that e-mail, from outside the organisation: a guest.
    let got = accept(&db, &can, &token).await.unwrap();
    assert_eq!(
        (got.project_name.as_str(), got.role, got.guest),
        ("Ada 101", ProjectRole::Editor, true)
    );
    assert_eq!(got.project_id, project.to_string());
    let c = common::try_open(&db, &can, buro, project).await.unwrap();
    changes::commit(
        &db.app,
        &c,
        common::envelope_in(buro, project, a_point(486500.0), &[]),
    )
    .await
    .unwrap();
    // The guest reaches that project only: not the organisation's list, not its other projects.
    assert!(tenancy::access(&db.app, &can, buro).await.is_err());
    assert!(common::try_open(&db, &can, buro, other).await.is_err());
    let mine = listing::mine(&db.app, &can).await.unwrap();
    assert_eq!(
        mine.projects
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["Ada 101"]
    );
    // Those who share it see the guest as such, and the invitation as used, once.
    let listed = people::list(&db.app, &by).await.unwrap();
    let guest = listed
        .people
        .iter()
        .find(|p| p.user_id == can.user_id.to_string())
        .unwrap();
    assert!(guest.guest && guest.role == Some(ProjectRole::Editor));
    assert!(matches!(
        accept(&db, &can, &token).await,
        Err(AppError::NotFound(_))
    ));
    let invitations = invitations::list(&db.app, &by).await.unwrap().invitations;
    assert_eq!(invitations[0].state, InvitationState::Accepted);
    assert_eq!(invitations[0].accepted_by_name.as_deref(), Some("can"));

    // The organisation stops taking guests: the grant stops working, and works again when it takes them.
    admin::set_tenant_guests(&db.owner, "buro", false)
        .await
        .unwrap();
    assert!(common::try_open(&db, &can, buro, project).await.is_err());
    let listed = people::list(&db.app, &by).await.unwrap();
    let guest = listed
        .people
        .iter()
        .find(|p| p.user_id == can.user_id.to_string())
        .unwrap();
    assert_eq!(guest.blocked, Some(AccessBlock::GuestsOff));
    admin::set_tenant_guests(&db.owner, "buro", true)
        .await
        .unwrap();
    assert!(common::try_open(&db, &can, buro, project).await.is_ok());
    db.close().await;
}

#[tokio::test]
async fn only_the_invited_verified_e_mail_accepts_a_waiting_invitation() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = new_project(&db, &ayse, "Ada 7").await;
    let by = open(&db, &ayse, project).await;
    let can = person(&db, "can", "can@diger.org", None).await;
    let mehmet = person(
        &db,
        "mehmet",
        "mehmet@buro.org",
        Some(("buro", TenantRole::Editor)),
    )
    .await;

    // Another account (a member, even) cannot take someone else's invitation; it keeps waiting.
    let for_can = invite(
        &db,
        &by,
        json!({ "email": "can@diger.org", "role": "viewer" }),
    )
    .await
    .unwrap();
    let token = for_can.token.unwrap();
    assert!(matches!(
        accept(&db, &mehmet, &token).await,
        Err(AppError::Forbidden(_))
    ));
    // An OpenID account whose provider did not verify the e-mail cannot either.
    let ece_id = identity::resolve_identity(
        &db.app,
        "https://kimlik.example.org",
        "ece",
        "Ece",
        Some("ece@x.org"),
        false,
    )
    .await
    .unwrap()
    .unwrap();
    let ece = identity::actor_of(&db.app, ece_id, SignInMethod::Oidc)
        .await
        .unwrap()
        .unwrap();
    let for_ece = invite(&db, &by, json!({ "email": "ece@x.org", "role": "viewer" }))
        .await
        .unwrap();
    match accept(&db, &ece, &for_ece.token.unwrap()).await {
        Err(AppError::Forbidden(m)) => assert!(m.contains("doğrulanmamış"), "{m}"),
        other => panic!("an unverified e-mail accepted: {other:?}"),
    }
    // A new invitation for the same e-mail replaces the waiting one: the old link stops working.
    let again = invite(
        &db,
        &by,
        json!({ "email": "can@diger.org", "role": "commenter" }),
    )
    .await
    .unwrap();
    assert!(matches!(
        accept(&db, &can, &token).await,
        Err(AppError::NotFound(_))
    ));
    // A withdrawn one does not open anything.
    let revoke = catalog_envelope(
        &by,
        PROJECT_INVITATION_REVOKE,
        json!({ "invitationId": again.invitation.id }),
        &[],
    );
    run(
        &db.app,
        &common::blobs(),
        &CatalogPolicy::default(),
        &by,
        revoke,
    )
    .await
    .unwrap();
    assert!(matches!(
        accept(&db, &can, &again.token.unwrap()).await,
        Err(AppError::NotFound(_))
    ));
    // Nor does one past its end.
    let late = invite(
        &db,
        &by,
        json!({ "email": "can@diger.org", "role": "viewer" }),
    )
    .await
    .unwrap();
    sqlx::query("update kentos.project_invitation set expires_at = now() - interval '1 minute' where id = $1")
        .bind(Uuid::parse_str(&late.invitation.id).unwrap())
        .execute(&db.owner)
        .await
        .unwrap();
    assert!(matches!(
        accept(&db, &can, &late.token.unwrap()).await,
        Err(AppError::NotFound(_))
    ));
    let states: Vec<InvitationState> = invitations::list(&db.app, &by)
        .await
        .unwrap()
        .invitations
        .iter()
        .map(|i| i.state)
        .collect();
    assert!(
        states.contains(&InvitationState::Expired) && states.contains(&InvitationState::Revoked)
    );
    // An organisation that takes no guests: an outsider's acceptance is refused.
    admin::set_tenant_guests(&db.owner, "buro", false)
        .await
        .unwrap();
    let off = invite(
        &db,
        &by,
        json!({ "email": "can@diger.org", "role": "viewer" }),
    )
    .await
    .unwrap();
    match accept(&db, &can, &off.token.unwrap()).await {
        Err(AppError::Forbidden(m)) => assert!(m.contains("misafir"), "{m}"),
        other => panic!("a guest was taken: {other:?}"),
    }
    // Nonsense tokens find nothing.
    assert!(matches!(
        accept(&db, &can, "abc").await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        accept(&db, &can, &"0".repeat(64)).await,
        Err(AppError::NotFound(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn invitations_are_checked_and_kept_to_those_who_share() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let dilek = member(&db, "buro", "dilek", TenantRole::Viewer).await;
    let project = new_project(&db, &ayse, "Kurallar").await;
    let by = open(&db, &ayse, project).await;
    common::share(&db, &by, &dilek.actor, GrantRole::Viewer).await;
    let path = |r: Result<InvitationChange, AppError>| match r {
        Err(e) => (e.code(), e.path().map(str::to_string)),
        Ok(c) => panic!("expected a refusal, got {c:?}"),
    };
    assert_eq!(
        path(invite(&db, &by, json!({ "email": "yok", "role": "viewer" })).await),
        ("invalid", Some("email".into()))
    );
    assert_eq!(
        path(invite(&db, &by, json!({ "email": "a@b.org", "role": "manager" })).await),
        ("invalid", Some("role".into()))
    );
    let far = (time::OffsetDateTime::now_utc() + time::Duration::days(91))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    assert_eq!(
        path(
            invite(
                &db,
                &by,
                json!({ "email": "a@b.org", "role": "viewer", "expiresAt": far })
            )
            .await
        ),
        ("invalid", Some("expiresAt".into()))
    );
    // A viewer does not invite, nor see the invitations.
    let d = open(&db, &dilek, project).await;
    assert!(matches!(
        invite(&db, &d, json!({ "email": "a@b.org", "role": "viewer" })).await,
        Err(AppError::Forbidden(_))
    ));
    assert!(matches!(
        invitations::list(&db.app, &d).await,
        Err(AppError::Forbidden(_))
    ));
    // A retry gets the stored answer, which has no token.
    let first = catalog_envelope(
        &by,
        PROJECT_INVITE,
        json!({ "email": "a@b.org", "role": "viewer" }),
        &[],
    );
    let answer = |o: CommandOutcome| match o {
        CommandOutcome::Invitation(c) => c,
        other => panic!("not an invitation: {other:?}"),
    };
    let made = answer(
        run(
            &db.app,
            &common::blobs(),
            &CatalogPolicy::default(),
            &by,
            first.clone(),
        )
        .await
        .unwrap(),
    );
    let retried = answer(
        run(
            &db.app,
            &common::blobs(),
            &CatalogPolicy::default(),
            &by,
            first,
        )
        .await
        .unwrap(),
    );
    assert!(made.token.is_some());
    assert_eq!(
        (retried.token, retried.replayed, retried.invitation.id),
        (None, true, made.invitation.id)
    );
    db.close().await;
}

#[tokio::test]
async fn members_get_plain_grants_and_keep_a_stronger_role() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = new_project(&db, &ayse, "Üyeler").await;
    let by = open(&db, &ayse, project).await;
    let mehmet = person(
        &db,
        "mehmet",
        "mehmet@buro.org",
        Some(("buro", TenantRole::Editor)),
    )
    .await;

    let viewer = invite(
        &db,
        &by,
        json!({ "email": "mehmet@buro.org", "role": "viewer" }),
    )
    .await
    .unwrap();
    let got = accept(&db, &mehmet, &viewer.token.unwrap()).await.unwrap();
    assert_eq!((got.role, got.guest), (ProjectRole::Viewer, false));
    // Made a manager by sharing, an invitation as viewer takes nothing away.
    common::share(&db, &by, &mehmet, GrantRole::Manager).await;
    let again = invite(
        &db,
        &by,
        json!({ "email": "mehmet@buro.org", "role": "viewer" }),
    )
    .await
    .unwrap();
    let got = accept(&db, &mehmet, &again.token.unwrap()).await.unwrap();
    assert_eq!(got.role, ProjectRole::Manager);

    // A personal space's project: everyone gets a plain grant, no membership needed.
    let deniz = person(&db, "deniz", "deniz@example.org", None).await;
    let own = personal(&db, &ayse.actor).await;
    let mine = new_project(&db, &own, "Kişisel").await;
    let p = open(&db, &own, mine).await;
    let made = invite(
        &db,
        &p,
        json!({ "email": "deniz@example.org", "role": "commenter" }),
    )
    .await
    .unwrap();
    let got = accept(&db, &deniz, &made.token.unwrap()).await.unwrap();
    assert_eq!((got.role, got.guest), (ProjectRole::Commenter, false));
    assert!(
        common::try_open(&db, &deniz, own.tenant, mine)
            .await
            .is_ok()
    );
    // Its owner opening their own invitation keeps being its owner.
    let own_mail = person(&db, "sahip", "sahip@example.org", None).await;
    let space = personal(&db, &own_mail).await;
    let theirs = new_project(&db, &space, "Sahibin").await;
    let t = open(&db, &space, theirs).await;
    let made = invite(
        &db,
        &t,
        json!({ "email": "sahip@example.org", "role": "viewer" }),
    )
    .await
    .unwrap();
    assert_eq!(
        accept(&db, &own_mail, &made.token.unwrap())
            .await
            .unwrap()
            .role,
        ProjectRole::Owner
    );
    db.close().await;
}
