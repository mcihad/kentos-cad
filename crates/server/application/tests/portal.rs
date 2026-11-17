//! The cloud portal against a real PostgreSQL/PostGIS (docs/adr/9000): signing up waits for an administrator's check;
//! an application goes round by round between a professional and a municipality under row security; a transfer goes
//! to a professional and comes back with a project opened in their space; a stale step is a conflict; the platform's
//! and the organisation's administration refuse anyone else.

mod common;

use common::{PASSWORD, account, blobs, new_project};
use kentos_application::blobs::Blobs;
use kentos_application::identity::Actor;
use kentos_application::portal::dossiers::{self, ListFilter};
use kentos_application::portal::files::{self, Purpose};
use kentos_application::portal::{accounts, detail, notifications, organization, platform, steps};
use kentos_application::{AppError, admin, tenancy};
use kentos_contracts::{
    ApplicationCreate, DossierAction, DossierActionRequest, DossierSite, DossierStatus,
    FromProject, MemberCreate, MemberEdit, MessagePost, NewOwner, OrganizationCategory,
    PlatformTenantCreate, Profession, ProfessionalReview, ProfessionalStatus, Signup, TenantRole,
    TransferCreate,
};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

const KCAD: &[u8] = include_bytes!("../../../../samples/Konut100m2.kcad");

/// The platform administrator.
async fn sistem(db: &TestDb) -> Actor {
    let a = account(db, "sistem").await;
    admin::set_platform_admin(&db.owner, "sistem", true)
        .await
        .unwrap();
    a
}

/// A municipality opened by the administrator with `owner` (a new account) as its owner; its id.
async fn municipality(db: &TestDb, by: &Actor, slug: &str, owner: &str) -> Uuid {
    platform::create_tenant(
        &db.app,
        by,
        &PlatformTenantCreate {
            slug: slug.into(),
            name: format!("{slug} belediyesi"),
            category: OrganizationCategory::Municipality,
            province: Some("Örnek".into()),
            district: Some("Merkez".into()),
            seats: 10,
            owner: Some(NewOwner {
                user_id: None,
                login: Some(owner.into()),
                password: Some(PASSWORD.into()),
                display_name: Some(format!("{owner} kişi")),
                email: Some(format!("{owner}@ornek.test")),
            }),
        },
    )
    .await
    .unwrap()
}

async fn signed_in(db: &TestDb, login: &str) -> Actor {
    let user = kentos_application::identity::check_local_login(&db.app, login, PASSWORD)
        .await
        .unwrap()
        .expect("signs in");
    kentos_application::identity::actor_of(&db.app, user, kentos_contracts::SignInMethod::Local)
        .await
        .unwrap()
        .unwrap()
}

/// A member added by the organisation's owner, signed in, with their access.
async fn staff(
    db: &TestDb,
    owner: &Actor,
    tenant: Uuid,
    login: &str,
    role: TenantRole,
) -> (Actor, tenancy::Access) {
    let by = tenancy::access(&db.app, owner, tenant).await.unwrap();
    organization::add_member(
        &db.app,
        &by,
        &MemberCreate {
            login: login.into(),
            password: PASSWORD.into(),
            display_name: format!("{login} kişi"),
            email: None,
            role,
        },
    )
    .await
    .unwrap();
    let actor = signed_in(db, login).await;
    let access = tenancy::access(&db.app, &actor, tenant).await.unwrap();
    (actor, access)
}

async fn professional(
    db: &TestDb,
    admin_actor: &Actor,
    login: &str,
    profession: Profession,
) -> Actor {
    let id = accounts::sign_up(
        &db.app,
        &Signup {
            login: login.into(),
            password: PASSWORD.into(),
            display_name: format!("{login} kişi"),
            email: format!("{login}@posta.test"),
            profession,
            chamber_no: "41234".into(),
            office_name: None,
            phone: None,
            province: None,
        },
        // Opened by the administrator: signing up may be closed.
        Some(admin_actor),
    )
    .await
    .unwrap();
    platform::review(
        &db.app,
        admin_actor,
        id,
        &ProfessionalReview {
            status: ProfessionalStatus::Verified,
            note: String::new(),
        },
    )
    .await
    .unwrap();
    signed_in(db, login).await
}

fn site() -> DossierSite {
    DossierSite {
        neighbourhood: Some("Cumhuriyet Mahallesi".into()),
        block: Some("412".into()),
        parcel: Some("7".into()),
        ..DossierSite::default()
    }
}

/// A file of `bytes` added to the dossier as `who`, as the upload route adds one.
async fn add_file(
    db: &TestDb,
    store: &Blobs,
    who: &Actor,
    dossier: Uuid,
    (name, bytes, doc_type): (&str, &[u8], &str),
    purpose: Purpose,
) -> Result<kentos_contracts::DossierFileView, AppError> {
    let prepared = files::prepare(&db.app, who, dossier, purpose).await?;
    let mut w = store
        .create(&prepared.key, bytes.len() as u64)
        .await
        .unwrap();
    w.write(bytes).await.unwrap();
    let (size, sha) = w.finish().await.unwrap();
    files::record_file(
        &db.app,
        who,
        &prepared,
        name,
        "application/pdf",
        (size, &sha),
        Some(doc_type),
        None,
        None,
    )
    .await
}

async fn act(
    db: &TestDb,
    who: &Actor,
    dossier: Uuid,
    action: DossierAction,
    note: Option<&str>,
) -> Result<kentos_contracts::DossierDetail, AppError> {
    let now = detail::detail(&db.app, who, dossier).await?;
    steps::act(
        &db.app,
        who,
        dossier,
        &DossierActionRequest {
            action,
            expected_status: now.summary.status,
            note: note.map(str::to_owned),
            assignee_id: None,
            attachments: Vec::new(),
        },
    )
    .await
}

async fn notified(db: &TestDb, who: &Actor) -> Vec<String> {
    notifications::list(&db.app, who, false, 50)
        .await
        .unwrap()
        .items
        .into_iter()
        .map(|n| n.kind)
        .collect()
}

#[tokio::test]
async fn signing_up_waits_for_the_check_and_the_check_attests_the_email() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let root = sistem(&db).await;
    let form = Signup {
        login: "ayse".into(),
        password: PASSWORD.into(),
        display_name: "Ayşe Yılmaz".into(),
        email: "ayse@posta.test".into(),
        profession: Profession::Architect,
        chamber_no: "41234".into(),
        office_name: None,
        phone: None,
        province: None,
    };
    // Closed until an administrator opens it (docs/adr/0007).
    let closed = accounts::sign_up(&db.app, &form, None).await.unwrap_err();
    assert!(matches!(closed, AppError::Forbidden(_)), "{closed:?}");
    platform::set_settings(
        &db.app,
        &root,
        &kentos_contracts::PlatformSettingsEdit { signup_open: Some(true), ..Default::default() },
    )
    .await
    .unwrap();
    let id = accounts::sign_up(&db.app, &form, None).await.unwrap();
    let verified: bool =
        sqlx::query_scalar("select email_verified from kentos.app_user where id = $1")
            .bind(id)
            .fetch_one(&db.owner)
            .await
            .unwrap();
    assert!(!verified, "a typed e-mail is not anyone's until checked");
    // Not yet verified: no application.
    let ayse = signed_in(&db, "ayse").await;
    let tenant = municipality(&db, &root, "ornek", "mehmet").await;
    let draft = ApplicationCreate {
        tenant_id: tenant.to_string(),
        kind: "building_permit".into(),
        title: "412 ada 7 parsel".into(),
        description: String::new(),
        site: site(),
        owner_name: None,
        office_id: None,
    };
    let refused = dossiers::create_application(&db.app, &ayse, &draft)
        .await
        .unwrap_err();
    assert!(refused.to_string().contains("doğrulanması"), "{refused}");
    // The administrators were told; the check attests the account.
    assert!(
        notified(&db, &root)
            .await
            .contains(&"professional.registered".to_owned())
    );
    platform::review(
        &db.app,
        &root,
        id,
        &ProfessionalReview {
            status: ProfessionalStatus::Verified,
            note: String::new(),
        },
    )
    .await
    .unwrap();
    let verified: bool =
        sqlx::query_scalar("select email_verified from kentos.app_user where id = $1")
            .bind(id)
            .fetch_one(&db.owner)
            .await
            .unwrap();
    assert!(verified);
    assert!(
        notified(&db, &ayse)
            .await
            .contains(&"professional.verified".to_owned())
    );
    dossiers::create_application(&db.app, &ayse, &draft)
        .await
        .unwrap();
}

#[tokio::test]
async fn an_application_goes_round_by_round_under_the_law_s_deadline() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    platform::set_settings(
        &db.app,
        &root,
        &kentos_contracts::PlatformSettingsEdit { signup_open: Some(true), ..Default::default() },
    )
    .await
    .unwrap();
    let tenant = municipality(&db, &root, "ornek", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let (zeynep, _) = staff(&db, &mehmet, tenant, "zeynep", TenantRole::Editor).await;
    let (ali, _) = staff(&db, &mehmet, tenant, "ali", TenantRole::ProjectManager).await;
    let ayse = professional(&db, &root, "ayse", Profession::Architect).await;
    let id = dossiers::create_application(
        &db.app,
        &ayse,
        &ApplicationCreate {
            tenant_id: tenant.to_string(),
            kind: "building_permit".into(),
            title: "412 ada 7 parsel konut yapı ruhsatı".into(),
            description: "Bodrum, zemin ve 3 kat.".into(),
            site: site(),
            owner_name: Some("Hasan Er".into()),
            office_id: None,
        },
    )
    .await
    .unwrap();
    // A draft is the professional's alone.
    let hidden = detail::detail(&db.app, &zeynep, id).await.unwrap_err();
    assert!(matches!(hidden, AppError::NotFound(_)), "{hidden:?}");
    // Nothing to send yet.
    let empty = act(&db, &ayse, id, DossierAction::Send, None)
        .await
        .unwrap_err();
    assert!(
        empty.to_string().contains("Gönderilecek dosya yok"),
        "{empty}"
    );
    add_file(
        &db,
        &store,
        &ayse,
        id,
        ("mimari.pdf", b"%PDF-1.4 mimari", "architectural"),
        Purpose::Package,
    )
    .await
    .unwrap();
    add_file(
        &db,
        &store,
        &ayse,
        id,
        ("proje.kcad", KCAD, "kentos_project"),
        Purpose::Package,
    )
    .await
    .unwrap();
    // The municipality does not see what is not sent yet.
    let sent = act(&db, &ayse, id, DossierAction::Send, Some("Sunarız."))
        .await
        .unwrap();
    assert_eq!(sent.summary.status, DossierStatus::Submitted);
    let number = sent.summary.number.clone().unwrap();
    assert!(number.ends_with("/0001"), "{number}");
    // Under article 22: the deficiencies are due within fifteen days.
    let clock = sent
        .summary
        .clock
        .clone()
        .expect("a building permit's deadline");
    assert_eq!(
        (clock.label.as_str(), clock.days_left),
        ("Eksikliklerin bildirimi", 15)
    );
    assert!(
        notified(&db, &zeynep)
            .await
            .contains(&"dossier.submitted".to_owned())
    );
    // An editor reviews; a deficiency list needs its words.
    let review = act(&db, &zeynep, id, DossierAction::StartReview, None)
        .await
        .unwrap();
    assert_eq!(review.summary.assignee_name.as_deref(), Some("zeynep kişi"));
    let wordless = act(&db, &zeynep, id, DossierAction::RequestChanges, None)
        .await
        .unwrap_err();
    assert_eq!(wordless.path(), Some("note"));
    act(
        &db,
        &zeynep,
        id,
        DossierAction::RequestChanges,
        Some("Statik proje eksik."),
    )
    .await
    .unwrap();
    // An editor does not decide.
    let not_hers = act(&db, &zeynep, id, DossierAction::Approve, None)
        .await
        .unwrap_err();
    assert!(
        matches!(not_hers, AppError::Invalid { .. } | AppError::Forbidden(_)),
        "{not_hers:?}"
    );
    // The professional reads the deficiencies and sends the corrected round.
    let back = detail::detail(&db.app, &ayse, id).await.unwrap();
    assert!(
        back.messages
            .iter()
            .any(|m| m.kind == "deficiency" && m.body == "Statik proje eksik.")
    );
    assert!(
        notified(&db, &ayse)
            .await
            .contains(&"dossier.changes_requested".to_owned())
    );
    // A pending file of the professional is theirs only.
    add_file(
        &db,
        &store,
        &ayse,
        id,
        ("statik.pdf", b"%PDF-1.4 statik", "structural"),
        Purpose::Package,
    )
    .await
    .unwrap();
    assert!(
        detail::detail(&db.app, &zeynep, id)
            .await
            .unwrap()
            .pending
            .is_empty()
    );
    let resent = act(&db, &ayse, id, DossierAction::Send, None)
        .await
        .unwrap();
    assert_eq!(
        (resent.summary.round, resent.summary.status),
        (2, DossierStatus::Submitted)
    );
    assert_eq!(
        resent.summary.number.as_deref(),
        Some(number.as_str()),
        "the number stays"
    );
    assert_eq!(
        resent.summary.clock.unwrap().days_left,
        15,
        "fifteen days from the corrected round"
    );
    // Every round counts for the documents.
    let checklist = resent.checklist.unwrap();
    for present in ["architectural", "structural"] {
        assert!(
            checklist
                .items
                .iter()
                .any(|i| i.doc_type == present && i.present),
            "{present}"
        );
    }
    // The manager decides; a closed dossier takes no more steps or messages.
    let approved = act(&db, &ali, id, DossierAction::Approve, Some("Uygun."))
        .await
        .unwrap();
    assert_eq!(approved.summary.status, DossierStatus::Approved);
    assert!(approved.actions.is_empty() && !approved.can_message);
    let closed = steps::message(
        &db.app,
        &ayse,
        id,
        &MessagePost {
            body: "Teşekkürler".into(),
            attachments: Vec::new(),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(closed, AppError::Forbidden(_)), "{closed:?}");
    assert!(
        notified(&db, &ayse)
            .await
            .contains(&"dossier.approved".to_owned())
    );
}

#[tokio::test]
async fn each_side_sees_only_what_is_its_own() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    let ornek = municipality(&db, &root, "ornek", "mehmet").await;
    let diger = municipality(&db, &root, "diger", "hakan").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let hakan = signed_in(&db, "hakan").await;
    let mehmet_in = tenancy::access(&db.app, &mehmet, ornek).await.unwrap();
    // The municipality's own project, which no professional may see.
    new_project(&db, &mehmet_in, "Belediyenin altlığı").await;
    let ayse = professional(&db, &root, "ayse", Profession::Architect).await;
    let can = professional(&db, &root, "can", Profession::Surveyor).await;
    let id = dossiers::create_application(
        &db.app,
        &ayse,
        &ApplicationCreate {
            tenant_id: ornek.to_string(),
            kind: "architectural_project".into(),
            title: "398 ada 12 parsel".into(),
            description: String::new(),
            site: site(),
            owner_name: None,
            office_id: None,
        },
    )
    .await
    .unwrap();
    add_file(
        &db,
        &store,
        &ayse,
        id,
        ("mimari.pdf", b"%PDF mimari", "architectural"),
        Purpose::Package,
    )
    .await
    .unwrap();
    act(&db, &ayse, id, DossierAction::Send, None)
        .await
        .unwrap();
    // Another municipality, another professional: as if it did not exist.
    for stranger in [&hakan, &can] {
        let e = detail::detail(&db.app, stranger, id).await.unwrap_err();
        assert!(matches!(e, AppError::NotFound(_)), "{e:?}");
    }
    let hakan_in = tenancy::access(&db.app, &hakan, diger).await.unwrap();
    assert_eq!(
        dossiers::list_for_tenant(
            &db.app,
            &hakan_in,
            &ListFilter {
                limit: 50,
                ..ListFilter::default()
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
    // The professional's transactions never carry the municipality's scope: its projects and members stay unseen.
    let page = kentos_application::listing::page(
        &db.app,
        &ayse,
        &kentos_application::listing::CatalogQuery {
            view: kentos_contracts::CatalogView::Organization,
            tenant: Some(ornek),
            search: String::new(),
            project_type: None,
            sort: None,
            limit: None,
            after: None,
        },
        std::time::Duration::from_secs(86_400),
    )
    .await;
    assert!(
        page.is_err(),
        "not a member: the organisation's projects are not listed"
    );
    let ayse_sees = detail::detail(&db.app, &ayse, id).await.unwrap();
    assert!(
        ayse_sees.assignees.is_empty(),
        "the municipality's members are not listed to her"
    );
    // The municipality sees the professional it has a dossier with, though she is not listed in its directory.
    let mehmet_sees = detail::detail(&db.app, &mehmet, id).await.unwrap();
    assert_eq!(mehmet_sees.professional.unwrap().display_name, "ayse kişi");
}

#[tokio::test]
async fn a_transfer_goes_to_the_professional_and_comes_back_as_their_project() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    let tenant = municipality(&db, &root, "ornek", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let mehmet_in = tenancy::access(&db.app, &mehmet, tenant).await.unwrap();
    let (_, editor_in) = staff(&db, &mehmet, tenant, "zeynep", TenantRole::Editor).await;
    let can = professional(&db, &root, "can", Profession::Surveyor).await;
    let ask = TransferCreate {
        professional_id: can.user_id.to_string(),
        kind: "survey".into(),
        title: "501 ada aplikasyonu".into(),
        description: "Köşe noktalarının aplikasyonu.".into(),
        site: site(),
        due_on: None,
    };
    // An editor does not transfer work.
    assert!(matches!(
        dossiers::create_transfer(&db.app, &editor_in, &ask)
            .await
            .unwrap_err(),
        AppError::Forbidden(_)
    ));
    let id = dossiers::create_transfer(&db.app, &mehmet_in, &ask)
        .await
        .unwrap();
    // The municipality's project goes in as a KCAD file of its present state.
    let base = new_project(&db, &mehmet_in, "Altlık").await;
    let added = files::from_project(
        &db.app,
        &store,
        &mehmet,
        id,
        &FromProject {
            tenant_id: tenant.to_string(),
            project_id: base.to_string(),
            doc_type: None,
        },
    )
    .await
    .unwrap();
    assert!(added.is_project && added.source.is_some());
    // The professional sees nothing of the draft.
    assert!(matches!(
        detail::detail(&db.app, &can, id).await.unwrap_err(),
        AppError::NotFound(_)
    ));
    act(&db, &mehmet, id, DossierAction::Send, None)
        .await
        .unwrap();
    assert!(
        notified(&db, &can)
            .await
            .contains(&"dossier.transferred".to_owned())
    );
    act(&db, &can, id, DossierAction::Accept, None)
        .await
        .unwrap();
    // Opened as a file project of his own space, revision 1 the file sent.
    let sent = detail::detail(&db.app, &can, id).await.unwrap();
    let file = sent.rounds[0].files.iter().find(|f| f.is_project).unwrap();
    let made = files::import(
        &db.app,
        &store,
        &can,
        id,
        Uuid::parse_str(&file.id).unwrap(),
        &kentos_contracts::DossierFileImport { tenant_id: None },
    )
    .await
    .unwrap();
    let (storage, revision): (String, Option<i64>) =
        sqlx::query_as("select storage, file_revision from kentos.project where id = $1")
            .bind(Uuid::parse_str(&made.project_id).unwrap())
            .fetch_one(&db.owner)
            .await
            .unwrap();
    assert_eq!((storage.as_str(), revision), ("file", Some(1)));
    // He delivers that project back; the manager completes the work.
    let delivered = files::from_project(
        &db.app,
        &store,
        &can,
        id,
        &FromProject {
            tenant_id: made.tenant_id.clone(),
            project_id: made.project_id.clone(),
            doc_type: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        delivered.sha256, file.sha256,
        "the same bytes, shared in the store"
    );
    let done = act(
        &db,
        &can,
        id,
        DossierAction::Deliver,
        Some("Tutanak ektedir."),
    )
    .await
    .unwrap();
    assert_eq!(
        (done.summary.status, done.summary.round),
        (DossierStatus::Delivered, 2)
    );
    let completed = act(&db, &mehmet, id, DossierAction::Complete, None)
        .await
        .unwrap();
    assert_eq!(completed.summary.status, DossierStatus::Completed);
    assert!(
        notified(&db, &can)
            .await
            .contains(&"dossier.completed".to_owned())
    );
}

#[tokio::test]
async fn a_step_from_a_status_that_changed_is_a_conflict() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    let tenant = municipality(&db, &root, "ornek", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let ayse = professional(&db, &root, "ayse", Profession::Architect).await;
    let id = dossiers::create_application(
        &db.app,
        &ayse,
        &ApplicationCreate {
            tenant_id: tenant.to_string(),
            kind: "other".into(),
            title: "Bir başvuru".into(),
            description: String::new(),
            site: DossierSite::default(),
            owner_name: None,
            office_id: None,
        },
    )
    .await
    .unwrap();
    add_file(
        &db,
        &store,
        &ayse,
        id,
        ("a.pdf", b"%PDF a", "other"),
        Purpose::Package,
    )
    .await
    .unwrap();
    act(&db, &ayse, id, DossierAction::Send, None)
        .await
        .unwrap();
    // The owner saw it submitted; the professional withdrew it meanwhile.
    act(&db, &ayse, id, DossierAction::Withdraw, None)
        .await
        .unwrap();
    let stale = steps::act(
        &db.app,
        &mehmet,
        id,
        &DossierActionRequest {
            action: DossierAction::Approve,
            expected_status: DossierStatus::Submitted,
            note: None,
            assignee_id: None,
            attachments: Vec::new(),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(stale, AppError::Conflict { .. }), "{stale:?}");
}

#[tokio::test]
async fn administration_is_its_administrators_and_an_organisation_keeps_an_owner() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let root = sistem(&db).await;
    let tenant = municipality(&db, &root, "ornek", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    // A municipality's owner is no platform administrator.
    assert!(matches!(
        platform::tenants(&db.app, &mehmet).await.unwrap_err(),
        AppError::Forbidden(_)
    ));
    assert!(matches!(
        platform::summary(&db.app, &mehmet).await.unwrap_err(),
        AppError::Forbidden(_)
    ));
    // An admin of the organisation makes no owner; the last owner stays one.
    let (_, admin_in) = staff(&db, &mehmet, tenant, "zeynep", TenantRole::Admin).await;
    let owner_try = organization::add_member(
        &db.app,
        &admin_in,
        &MemberCreate {
            login: "yeni".into(),
            password: PASSWORD.into(),
            display_name: "Yeni kişi".into(),
            email: None,
            role: TenantRole::Owner,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(owner_try, AppError::Forbidden(_)), "{owner_try:?}");
    let owner_in = tenancy::access(&db.app, &mehmet, tenant).await.unwrap();
    let last = organization::update_member(
        &db.app,
        &owner_in,
        mehmet.user_id,
        &MemberEdit {
            role: TenantRole::Admin,
            status: "active".into(),
            seat: true,
        },
    )
    .await
    .unwrap_err();
    assert!(last.to_string().contains("son sahibi"), "{last}");
    // Seats are the administrator's: not fewer than used.
    let used = platform::update_tenant(
        &db.app,
        &root,
        tenant,
        &kentos_contracts::PlatformTenantEdit {
            name: "ornek belediyesi".into(),
            category: OrganizationCategory::Municipality,
            province: None,
            district: None,
            seats: 1,
            status: "active".into(),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(used.path(), Some("seats"));
    // Every change is in the platform's record.
    let record = platform::events(&db.app, &root, 50).await.unwrap();
    assert!(record.iter().any(|e| e.action == "tenant.create"));
    assert!(record.iter().any(|e| e.action == "user.platform_admin"));
}

/// Sends `bytes` as `name` through the account's own upload, in one part; the upload's id.
async fn uploaded(db: &TestDb, store: &Blobs, who: &Actor, name: &str, bytes: &[u8]) -> Result<String, AppError> {
    use sha2::Digest as _;
    let sha: String = sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
    let open = kentos_application::uploads::begin(
        &db.app,
        who,
        kentos_contracts::UploadBegin {
            name: name.into(),
            size: bytes.len() as u64,
            sha256: sha,
        },
    )
    .await?;
    let id = Uuid::parse_str(&open.id).unwrap();
    let scanner = kentos_application::scan::Scanner::rules_only();
    let done = kentos_application::uploads::receive(&db.app, store, &scanner, who, id, 0, bytes).await?;
    assert!(done.received);
    Ok(done.id)
}

#[tokio::test]
async fn a_dossier_takes_a_scanned_upload_and_a_refusal_leaves_it_to_try_again() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    let ornek = municipality(&db, &root, "ornek", "mehmet").await;
    let ayse = professional(&db, &root, "ayse", Profession::Architect).await;
    let id = dossiers::create_application(
        &db.app,
        &ayse,
        &ApplicationCreate {
            tenant_id: ornek.to_string(),
            kind: "architectural_project".into(),
            title: "398 ada 12 parsel".into(),
            description: String::new(),
            site: site(),
            owner_name: None,
            office_id: None,
        },
    )
    .await
    .unwrap();
    // A plan sent from a phone in parts becomes the package's file, kept with the type its bytes are.
    let upload = uploaded(&db, &store, &ayse, "zemin-etudu.pdf", b"%PDF-1.7 zemin etudu").await.unwrap();
    let take = |purpose: &str, upload: &str| kentos_contracts::DossierFileFromUpload {
        upload_id: upload.into(),
        purpose: purpose.into(),
        doc_type: Some("soil_report".into()),
    };
    // A wrong purpose is refused before anything moves: the upload is still there.
    let e = files::from_upload(&db.app, &store, &ayse, id, &take("hepsi", &upload)).await.unwrap_err();
    assert!(matches!(e, AppError::Invalid { .. }), "{e:?}");
    let file = files::from_upload(&db.app, &store, &ayse, id, &take("package", &upload)).await.unwrap();
    assert_eq!(
        (file.media_type.as_str(), file.scan, file.name.as_str()),
        ("application/pdf", kentos_contracts::ScanState::Unscanned, "zemin-etudu.pdf")
    );
    let (_, mut opened) = files::download(&db.app, &store, &ayse, id, Uuid::parse_str(&file.id).unwrap()).await.unwrap();
    let mut back = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut opened, &mut back).await.unwrap();
    assert_eq!(back, b"%PDF-1.7 zemin etudu");
    // Taken once; another account's upload is no one's here.
    let e = files::from_upload(&db.app, &store, &ayse, id, &take("package", &upload)).await.unwrap_err();
    assert!(matches!(e, AppError::NotFound(_)), "{e:?}");
    let mehmet = signed_in(&db, "mehmet").await;
    let his = uploaded(&db, &store, &mehmet, "kroki.png", b"\x89PNG\r\n\x1a\nkroki").await.unwrap();
    let e = files::from_upload(&db.app, &store, &ayse, id, &take("package", &his)).await.unwrap_err();
    assert!(matches!(e, AppError::NotFound(_)), "{e:?}");
}

#[tokio::test]
async fn an_office_works_on_its_professionals_dossier_by_role() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    let ornek = municipality(&db, &root, "ornek", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let ayse = professional(&db, &root, "ayse", Profession::Architect).await;
    // Ayşe's office, with an editor, a project manager and a viewer.
    let office = platform::create_tenant(
        &db.app,
        &root,
        &PlatformTenantCreate {
            slug: "yilmaz-mimarlik".into(),
            name: "Yılmaz Mimarlık".into(),
            category: OrganizationCategory::Office,
            province: Some("Örnek".into()),
            district: None,
            seats: 5,
            owner: Some(NewOwner {
                user_id: Some(ayse.user_id.to_string()),
                login: None,
                password: None,
                display_name: None,
                email: None,
            }),
        },
    )
    .await
    .unwrap();
    let (selin, _) = staff(&db, &ayse, office, "selin", TenantRole::Editor).await;
    let (emre, emre_in) = staff(&db, &ayse, office, "emre", TenantRole::ProjectManager).await;
    let (veli, _) = staff(&db, &ayse, office, "veli", TenantRole::Viewer).await;
    let can = professional(&db, &root, "can", Profession::Surveyor).await;
    let apply = |office: Option<Uuid>| ApplicationCreate {
        tenant_id: ornek.to_string(),
        kind: "architectural_project".into(),
        title: "412 ada 7 parsel konut".into(),
        description: String::new(),
        site: site(),
        owner_name: None,
        office_id: office.map(|o| o.to_string()),
    };
    // Only an office she is an editor of: not another's.
    let e = dossiers::create_application(&db.app, &can, &apply(Some(office))).await.unwrap_err();
    assert!(matches!(e, AppError::Invalid { .. }), "{e:?}");
    let id = dossiers::create_application(&db.app, &ayse, &apply(Some(office))).await.unwrap();
    // The editor prepares the package but does not send; the office's manager sends for her.
    add_file(&db, &store, &selin, id, ("mimari.pdf", b"%PDF mimari", "architectural"), Purpose::Package)
        .await
        .unwrap();
    let seen = detail::detail(&db.app, &selin, id).await.unwrap();
    assert_eq!(seen.summary.office_name.as_deref(), Some("Yılmaz Mimarlık"));
    assert!(seen.can_add_files && seen.actions.is_empty() && !seen.can_share_with_office);
    let e = act(&db, &selin, id, DossierAction::Send, None).await.unwrap_err();
    assert!(matches!(e, AppError::Forbidden(_)), "{e:?}");
    let viewer = detail::detail(&db.app, &veli, id).await.unwrap();
    assert!(!viewer.can_add_files && viewer.actions.is_empty());
    act(&db, &emre, id, DossierAction::Send, None).await.unwrap();
    // The municipality asks for changes: the professional and the office's editors and above are told, the viewer not.
    act(&db, &mehmet, id, DossierAction::RequestChanges, Some("Otopark hesabı eksik."))
        .await
        .unwrap();
    for (who, told) in [(&ayse, true), (&selin, true), (&emre, true), (&veli, false)] {
        let kinds = notified(&db, who).await;
        assert_eq!(kinds.iter().any(|k| k.contains("changes")), told, "{}: {kinds:?}", who.display_name);
    }
    // The office's list holds it; a stranger finds nothing.
    let listed = dossiers::list_for_tenant(&db.app, &emre_in, &ListFilter { limit: 50, ..ListFilter::default() })
        .await
        .unwrap();
    assert_eq!(listed.items.iter().map(|d| d.id.clone()).collect::<Vec<_>>(), vec![id.to_string()]);
    assert!(listed.items[0].awaiting_me, "changes requested: the professional's side, the office's too");
    assert!(matches!(detail::detail(&db.app, &can, id).await.unwrap_err(), AppError::NotFound(_)));
    // Only she takes the sharing back; then her colleagues no longer see it.
    let off = kentos_contracts::DossierOfficeSet { office_id: None };
    let e = dossiers::set_office(&db.app, &emre, id, &off).await.unwrap_err();
    assert!(matches!(e, AppError::Forbidden(_)), "{e:?}");
    dossiers::set_office(&db.app, &ayse, id, &off).await.unwrap();
    assert!(matches!(detail::detail(&db.app, &selin, id).await.unwrap_err(), AppError::NotFound(_)));
    let timeline = detail::detail(&db.app, &ayse, id).await.unwrap().events;
    assert!(timeline.iter().any(|e| e.text.contains("Yılmaz Mimarlık ile paylaşımını kaldırdı")), "{timeline:?}");
}

/// A tariff of 2464's structure with example rates.
fn priced_tariff() -> kentos_contracts::FeeDefinition {
    let mut d = kentos_application::portal::fees::template();
    for (row, rate) in d.tables[0].rows.iter_mut().zip([12.0, 24.0, 36.0, 48.0, 60.0, 48.0, 72.0, 96.0, 120.0]) {
        row.rate = rate;
    }
    for t in &mut d.tables[1..] {
        t.rows[0].rate = 1.25;
        t.rows[1].rate = 3.75;
    }
    d.payment_note = Some("Vezneye ya da belediyenin hesabına dosya numarasıyla ödenir.".into());
    d
}

#[tokio::test]
async fn fees_are_issued_by_the_tariff_and_paid_before_the_permit() {
    use kentos_application::portal::fees;
    use kentos_contracts::{FeeAssess, FeeAssessmentState, FeeCancel, FeePaid, FeeScheduleCopy, FeeScheduleEdit};

    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    let tenant = municipality(&db, &root, "harcli", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let owner = tenancy::access(&db.app, &mehmet, tenant).await.unwrap();
    let (zeynep, zeynep_access) = staff(&db, &mehmet, tenant, "zeynep", TenantRole::Editor).await;
    let (ali, _) = staff(&db, &mehmet, tenant, "ali", TenantRole::ProjectManager).await;
    let ayse = professional(&db, &root, "ayse", Profession::Architect).await;
    let id = dossiers::create_application(
        &db.app,
        &ayse,
        &ApplicationCreate {
            tenant_id: tenant.to_string(),
            kind: "building_permit".into(),
            title: "412 ada 7 parsel konut yapı ruhsatı".into(),
            description: "Bodrum, zemin ve 3 kat.".into(),
            site: site(),
            owner_name: Some("Hasan Er".into()),
            office_id: None,
        },
    )
    .await
    .unwrap();
    add_file(&db, &store, &ayse, id, ("mimari.pdf", b"%PDF-1.4 mimari", "architectural"), Purpose::Package)
        .await
        .unwrap();
    act(&db, &ayse, id, DossierAction::Send, None).await.unwrap();

    // The tariff is the admins'; an editor reads it but does not write it.
    let edit = FeeScheduleEdit {
        name: "2026 Ücret Tarifesi".into(),
        year: 2026,
        valid_from: "2026-01-01".into(),
        valid_to: None,
        decision: Some("Meclisin 2025/214 sayılı kararı".into()),
        definition: priced_tariff(),
    };
    let refused = fees::create_schedule(&db.app, &zeynep_access, &edit).await.unwrap_err();
    assert!(matches!(refused, AppError::Forbidden(_)), "{refused:?}");
    let draft = fees::create_schedule(&db.app, &owner, &edit).await.unwrap();
    // No tariff in force yet: nothing to issue, nothing holds the decision back.
    let none = fees::preview(&db.app, &zeynep, id, &FeeAssess { inputs: Default::default(), due_on: None })
        .await
        .unwrap_err();
    assert!(none.to_string().contains("yürürlükte bir ücret tarifesi yok"), "{none}");
    let draft_id = Uuid::parse_str(&draft.id).unwrap();
    let active = fees::activate(&db.app, &owner, draft_id).await.unwrap();
    assert_eq!(active.state, kentos_contracts::FeeScheduleState::Active);
    assert_eq!(fees::schedules(&db.app, &zeynep_access).await.unwrap().len(), 1);

    // The permit waits for its fee: first to be issued, then to be paid (2464 Ek Madde 5).
    let early = act(&db, &ali, id, DossierAction::Approve, None).await.unwrap_err();
    assert!(early.to_string().contains("tahakkuk ettirilmeli"), "{early}");
    // The professional reads fees but does not issue them.
    let theirs = fees::preview(&db.app, &ayse, id, &FeeAssess { inputs: Default::default(), due_on: None })
        .await
        .unwrap_err();
    assert!(matches!(theirs, AppError::Forbidden(_)), "{theirs:?}");
    // An empty preview asks what the kind needs.
    let asked = fees::preview(&db.app, &zeynep, id, &FeeAssess { inputs: Default::default(), due_on: None })
        .await
        .unwrap();
    let keys: Vec<&str> = asked.parameters.iter().map(|p| p.key.as_str()).collect();
    assert_eq!(keys, ["yapi_turu", "insaat_alani", "bolge", "kamu_istisnasi"]);
    assert!(asked.missing.contains_key("insaat_alani") && asked.lines.is_empty());
    let inputs: kentos_contracts::FeeInputs = [
        ("yapi_turu".to_owned(), serde_json::json!("konut")),
        ("insaat_alani".to_owned(), serde_json::json!(152.5)),
        ("bolge".to_owned(), serde_json::json!("konut")),
    ]
    .into_iter()
    .collect();
    let fee = fees::issue(&db.app, &zeynep, id, &FeeAssess { inputs: inputs.clone(), due_on: None })
        .await
        .unwrap();
    // 152,5 m² × 48 TL (151–200 m²) + 152,5 m² × 1,25 TL (konut bölgesi) = 7.320 + 190,63.
    assert_eq!(fee.total, 732_000 + 19_063);
    assert_eq!(fee.lines[0].basis, "152,5 m² × 48 TL (Konut, 151–200 m²)");
    assert_eq!(fee.schedule_name, "2026 Ücret Tarifesi");
    assert!(notified(&db, &ayse).await.contains(&"dossier.fee_issued".to_owned()));
    let seen = fees::dossier_fees(&db.app, &ayse, id).await.unwrap();
    assert_eq!((seen.len(), seen[0].state), (1, FeeAssessmentState::Issued));
    assert!(seen[0].payment_note.as_deref().is_some_and(|n| n.contains("Vezneye")));
    let unpaid = act(&db, &ali, id, DossierAction::Approve, None).await.unwrap_err();
    assert!(unpaid.to_string().contains("ödenmedi"), "{unpaid}");

    let fee_id = Uuid::parse_str(&fee.id).unwrap();
    let paid = fees::paid(&db.app, &zeynep, id, fee_id, &FeePaid { receipt_no: "MKB-2026-118".into(), paid_on: None })
        .await
        .unwrap();
    assert_eq!(paid.state, FeeAssessmentState::Paid);
    assert!(notified(&db, &ayse).await.contains(&"dossier.fee_paid".to_owned()));
    let twice = fees::paid(&db.app, &zeynep, id, fee_id, &FeePaid { receipt_no: "X".into(), paid_on: None })
        .await
        .unwrap_err();
    assert!(twice.to_string().contains("ödenecek durumda değil"), "{twice}");
    let approved = act(&db, &ali, id, DossierAction::Approve, None).await.unwrap();
    assert_eq!(approved.summary.status, DossierStatus::Approved);
    let texts: Vec<String> = approved.events.iter().map(|e| e.text.clone()).collect();
    assert!(texts.iter().any(|t| t.contains("harç ve ücret tahakkuk ettirdi: 7.510,63 TL")), "{texts:?}");
    assert!(texts.iter().any(|t| t.contains("makbuz MKB-2026-118")), "{texts:?}");

    // Cancelling is a manager's, with a reason; a paid one too.
    let by_editor = fees::cancel(&db.app, &zeynep, id, fee_id, &FeeCancel { reason: "Yanlış alan".into() })
        .await
        .unwrap_err();
    assert!(matches!(by_editor, AppError::Forbidden(_)), "{by_editor:?}");
    let cancelled = fees::cancel(&db.app, &ali, id, fee_id, &FeeCancel { reason: "Alan yanlış girildi".into() })
        .await
        .unwrap();
    assert_eq!((cancelled.state, cancelled.receipt_no), (FeeAssessmentState::Cancelled, None));

    // Next year's draft: every amount revalued; the issued assessment keeps its own lines.
    let next = fees::copy_schedule(
        &db.app,
        &owner,
        draft_id,
        &FeeScheduleCopy { year: 2027, valid_from: "2027-01-01".into(), factor: 1.5 },
    )
    .await
    .unwrap();
    assert_eq!(next.definition.tables[0].rows[3].rate, 72.0);
    assert_eq!(next.state, kentos_contracts::FeeScheduleState::Draft);
    let frozen = fees::update_schedule(&db.app, &owner, draft_id, &edit).await.unwrap_err();
    assert!(frozen.to_string().contains("değişmez"), "{frozen}");
    fees::delete_schedule(&db.app, &owner, Uuid::parse_str(&next.id).unwrap()).await.unwrap();
    assert_eq!(fees::dossier_fees(&db.app, &ayse, id).await.unwrap()[0].lines[0].amount, 732_000);
}
