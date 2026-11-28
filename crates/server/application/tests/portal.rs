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
            ticket: None,
            website: None,
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

/// The days left of a deadline `days` from today as the law counts it: a last day the offices do not work runs to
/// the first working day after it (docs/adr/9000 §19), so the figure depends on the calendar.
fn legal_days(days: i64) -> i32 {
    let today = time::OffsetDateTime::now_utc()
        .to_offset(time::UtcOffset::from_hms(3, 0, 0).unwrap())
        .date();
    let (due, _) = kentos_application::portal::holidays::current()
        .extended(today + time::Duration::days(days));
    i32::try_from((due - today).whole_days()).unwrap()
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
        ticket: None,
        website: None,
    };
    // Closed until an administrator opens it (docs/adr/0007).
    let closed = accounts::sign_up(&db.app, &form, None).await.unwrap_err();
    assert!(matches!(closed, AppError::Forbidden(_)), "{closed:?}");
    platform::set_settings(
        &db.app,
        &root,
        &kentos_contracts::PlatformSettingsEdit {
            signup_open: Some(true),
            ..Default::default()
        },
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
        &kentos_contracts::PlatformSettingsEdit {
            signup_open: Some(true),
            ..Default::default()
        },
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
        ("Eksikliklerin bildirimi", legal_days(15))
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
        legal_days(15),
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
async fn uploaded(
    db: &TestDb,
    store: &Blobs,
    who: &Actor,
    name: &str,
    bytes: &[u8],
) -> Result<String, AppError> {
    use sha2::Digest as _;
    let sha: String = sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
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
    let done =
        kentos_application::uploads::receive(&db.app, store, &scanner, who, id, 0, bytes).await?;
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
    let upload = uploaded(
        &db,
        &store,
        &ayse,
        "zemin-etudu.pdf",
        b"%PDF-1.7 zemin etudu",
    )
    .await
    .unwrap();
    let take = |purpose: &str, upload: &str| kentos_contracts::DossierFileFromUpload {
        upload_id: upload.into(),
        purpose: purpose.into(),
        doc_type: Some("soil_report".into()),
    };
    // A wrong purpose is refused before anything moves: the upload is still there.
    let e = files::from_upload(&db.app, &store, &ayse, id, &take("hepsi", &upload))
        .await
        .unwrap_err();
    assert!(matches!(e, AppError::Invalid { .. }), "{e:?}");
    let file = files::from_upload(&db.app, &store, &ayse, id, &take("package", &upload))
        .await
        .unwrap();
    assert_eq!(
        (file.media_type.as_str(), file.scan, file.name.as_str()),
        (
            "application/pdf",
            kentos_contracts::ScanState::Unscanned,
            "zemin-etudu.pdf"
        )
    );
    let (_, mut opened) = files::download(
        &db.app,
        &store,
        &ayse,
        id,
        Uuid::parse_str(&file.id).unwrap(),
    )
    .await
    .unwrap();
    let mut back = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut opened, &mut back)
        .await
        .unwrap();
    assert_eq!(back, b"%PDF-1.7 zemin etudu");
    // Taken once; another account's upload is no one's here.
    let e = files::from_upload(&db.app, &store, &ayse, id, &take("package", &upload))
        .await
        .unwrap_err();
    assert!(matches!(e, AppError::NotFound(_)), "{e:?}");
    let mehmet = signed_in(&db, "mehmet").await;
    let his = uploaded(&db, &store, &mehmet, "kroki.png", b"\x89PNG\r\n\x1a\nkroki")
        .await
        .unwrap();
    let e = files::from_upload(&db.app, &store, &ayse, id, &take("package", &his))
        .await
        .unwrap_err();
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
    let e = dossiers::create_application(&db.app, &can, &apply(Some(office)))
        .await
        .unwrap_err();
    assert!(matches!(e, AppError::Invalid { .. }), "{e:?}");
    let id = dossiers::create_application(&db.app, &ayse, &apply(Some(office)))
        .await
        .unwrap();
    // The editor prepares the package but does not send; the office's manager sends for her.
    add_file(
        &db,
        &store,
        &selin,
        id,
        ("mimari.pdf", b"%PDF mimari", "architectural"),
        Purpose::Package,
    )
    .await
    .unwrap();
    let seen = detail::detail(&db.app, &selin, id).await.unwrap();
    assert_eq!(seen.summary.office_name.as_deref(), Some("Yılmaz Mimarlık"));
    assert!(seen.can_add_files && seen.actions.is_empty() && !seen.can_share_with_office);
    let e = act(&db, &selin, id, DossierAction::Send, None)
        .await
        .unwrap_err();
    assert!(matches!(e, AppError::Forbidden(_)), "{e:?}");
    let viewer = detail::detail(&db.app, &veli, id).await.unwrap();
    assert!(!viewer.can_add_files && viewer.actions.is_empty());
    act(&db, &emre, id, DossierAction::Send, None)
        .await
        .unwrap();
    // The municipality asks for changes: the professional and the office's editors and above are told, the viewer not.
    act(
        &db,
        &mehmet,
        id,
        DossierAction::RequestChanges,
        Some("Otopark hesabı eksik."),
    )
    .await
    .unwrap();
    for (who, told) in [(&ayse, true), (&selin, true), (&emre, true), (&veli, false)] {
        let kinds = notified(&db, who).await;
        assert_eq!(
            kinds.iter().any(|k| k.contains("changes")),
            told,
            "{}: {kinds:?}",
            who.display_name
        );
    }
    // The office's list holds it; a stranger finds nothing.
    let listed = dossiers::list_for_tenant(
        &db.app,
        &emre_in,
        &ListFilter {
            limit: 50,
            ..ListFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        listed
            .items
            .iter()
            .map(|d| d.id.clone())
            .collect::<Vec<_>>(),
        vec![id.to_string()]
    );
    assert!(
        listed.items[0].awaiting_me,
        "changes requested: the professional's side, the office's too"
    );
    assert!(matches!(
        detail::detail(&db.app, &can, id).await.unwrap_err(),
        AppError::NotFound(_)
    ));
    // Only she takes the sharing back; then her colleagues no longer see it.
    let off = kentos_contracts::DossierOfficeSet { office_id: None };
    let e = dossiers::set_office(&db.app, &emre, id, &off)
        .await
        .unwrap_err();
    assert!(matches!(e, AppError::Forbidden(_)), "{e:?}");
    dossiers::set_office(&db.app, &ayse, id, &off)
        .await
        .unwrap();
    assert!(matches!(
        detail::detail(&db.app, &selin, id).await.unwrap_err(),
        AppError::NotFound(_)
    ));
    let timeline = detail::detail(&db.app, &ayse, id).await.unwrap().events;
    assert!(
        timeline
            .iter()
            .any(|e| e.text.contains("Yılmaz Mimarlık ile paylaşımını kaldırdı")),
        "{timeline:?}"
    );
}

/// A tariff of 2464's structure with example rates.
fn priced_tariff() -> kentos_contracts::FeeDefinition {
    let mut d = kentos_application::portal::fees::template();
    for (row, rate) in d.tables[0]
        .rows
        .iter_mut()
        .zip([12.0, 24.0, 36.0, 48.0, 60.0, 48.0, 72.0, 96.0, 120.0])
    {
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
    use kentos_contracts::{
        FeeAssess, FeeAssessmentState, FeeCancel, FeePaid, FeeScheduleCopy, FeeScheduleEdit,
    };

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
    act(&db, &ayse, id, DossierAction::Send, None)
        .await
        .unwrap();

    // The tariff is the admins'; an editor reads it but does not write it.
    let edit = FeeScheduleEdit {
        name: "2026 Ücret Tarifesi".into(),
        year: 2026,
        valid_from: "2026-01-01".into(),
        valid_to: None,
        decision: Some("Meclisin 2025/214 sayılı kararı".into()),
        definition: priced_tariff(),
    };
    let refused = fees::create_schedule(&db.app, &zeynep_access, &edit)
        .await
        .unwrap_err();
    assert!(matches!(refused, AppError::Forbidden(_)), "{refused:?}");
    let draft = fees::create_schedule(&db.app, &owner, &edit).await.unwrap();
    // No tariff in force yet: nothing to issue, nothing holds the decision back.
    let none = fees::preview(
        &db.app,
        &zeynep,
        id,
        &FeeAssess {
            inputs: Default::default(),
            due_on: None,
        },
    )
    .await
    .unwrap_err();
    assert!(
        none.to_string()
            .contains("yürürlükte bir ücret tarifesi yok"),
        "{none}"
    );
    let draft_id = Uuid::parse_str(&draft.id).unwrap();
    let active = fees::activate(&db.app, &owner, draft_id).await.unwrap();
    assert_eq!(active.state, kentos_contracts::FeeScheduleState::Active);
    assert_eq!(
        fees::schedules(&db.app, &zeynep_access)
            .await
            .unwrap()
            .len(),
        1
    );

    // The permit waits for its fee: first to be issued, then to be paid (2464 Ek Madde 5).
    let early = act(&db, &ali, id, DossierAction::Approve, None)
        .await
        .unwrap_err();
    assert!(
        early.to_string().contains("tahakkuk ettirilmeli"),
        "{early}"
    );
    // The professional reads fees but does not issue them.
    let theirs = fees::preview(
        &db.app,
        &ayse,
        id,
        &FeeAssess {
            inputs: Default::default(),
            due_on: None,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(theirs, AppError::Forbidden(_)), "{theirs:?}");
    // An empty preview asks what the kind needs.
    let asked = fees::preview(
        &db.app,
        &zeynep,
        id,
        &FeeAssess {
            inputs: Default::default(),
            due_on: None,
        },
    )
    .await
    .unwrap();
    let keys: Vec<&str> = asked.parameters.iter().map(|p| p.key.as_str()).collect();
    assert_eq!(
        keys,
        ["yapi_turu", "insaat_alani", "bolge", "kamu_istisnasi"]
    );
    assert!(asked.missing.contains_key("insaat_alani") && asked.lines.is_empty());
    let inputs: kentos_contracts::FeeInputs = [
        ("yapi_turu".to_owned(), serde_json::json!("konut")),
        ("insaat_alani".to_owned(), serde_json::json!(152.5)),
        ("bolge".to_owned(), serde_json::json!("konut")),
    ]
    .into_iter()
    .collect();
    let fee = fees::issue(
        &db.app,
        &zeynep,
        id,
        &FeeAssess {
            inputs: inputs.clone(),
            due_on: None,
        },
    )
    .await
    .unwrap();
    // 152,5 m² × 48 TL (151–200 m²) + 152,5 m² × 1,25 TL (konut bölgesi) = 7.320 + 190,63.
    assert_eq!(fee.total, 732_000 + 19_063);
    assert_eq!(fee.lines[0].basis, "152,5 m² × 48 TL (Konut, 151–200 m²)");
    assert_eq!(fee.schedule_name, "2026 Ücret Tarifesi");
    assert!(
        notified(&db, &ayse)
            .await
            .contains(&"dossier.fee_issued".to_owned())
    );
    let seen = fees::dossier_fees(&db.app, &ayse, id).await.unwrap();
    assert_eq!((seen.len(), seen[0].state), (1, FeeAssessmentState::Issued));
    assert!(
        seen[0]
            .payment_note
            .as_deref()
            .is_some_and(|n| n.contains("Vezneye"))
    );
    let unpaid = act(&db, &ali, id, DossierAction::Approve, None)
        .await
        .unwrap_err();
    assert!(unpaid.to_string().contains("ödenmedi"), "{unpaid}");

    let fee_id = Uuid::parse_str(&fee.id).unwrap();
    let paid = fees::paid(
        &db.app,
        &zeynep,
        id,
        fee_id,
        &FeePaid {
            receipt_no: "MKB-2026-118".into(),
            paid_on: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(paid.state, FeeAssessmentState::Paid);
    assert!(
        notified(&db, &ayse)
            .await
            .contains(&"dossier.fee_paid".to_owned())
    );
    let twice = fees::paid(
        &db.app,
        &zeynep,
        id,
        fee_id,
        &FeePaid {
            receipt_no: "X".into(),
            paid_on: None,
        },
    )
    .await
    .unwrap_err();
    assert!(
        twice.to_string().contains("ödenecek durumda değil"),
        "{twice}"
    );
    let approved = act(&db, &ali, id, DossierAction::Approve, None)
        .await
        .unwrap();
    assert_eq!(approved.summary.status, DossierStatus::Approved);
    let texts: Vec<String> = approved.events.iter().map(|e| e.text.clone()).collect();
    assert!(
        texts
            .iter()
            .any(|t| t.contains("harç ve ücret tahakkuk ettirdi: 7.510,63 TL")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("makbuz MKB-2026-118")),
        "{texts:?}"
    );

    // Cancelling is a manager's, with a reason; a paid one too.
    let by_editor = fees::cancel(
        &db.app,
        &zeynep,
        id,
        fee_id,
        &FeeCancel {
            reason: "Yanlış alan".into(),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(by_editor, AppError::Forbidden(_)), "{by_editor:?}");
    let cancelled = fees::cancel(
        &db.app,
        &ali,
        id,
        fee_id,
        &FeeCancel {
            reason: "Alan yanlış girildi".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        (cancelled.state, cancelled.receipt_no),
        (FeeAssessmentState::Cancelled, None)
    );

    // Next year's draft: every amount revalued; the issued assessment keeps its own lines.
    let next = fees::copy_schedule(
        &db.app,
        &owner,
        draft_id,
        &FeeScheduleCopy {
            year: 2027,
            valid_from: "2027-01-01".into(),
            factor: 1.5,
        },
    )
    .await
    .unwrap();
    assert_eq!(next.definition.tables[0].rows[3].rate, 72.0);
    assert_eq!(next.state, kentos_contracts::FeeScheduleState::Draft);
    let frozen = fees::update_schedule(&db.app, &owner, draft_id, &edit)
        .await
        .unwrap_err();
    assert!(frozen.to_string().contains("değişmez"), "{frozen}");
    fees::delete_schedule(&db.app, &owner, Uuid::parse_str(&next.id).unwrap())
        .await
        .unwrap();
    assert_eq!(
        fees::dossier_fees(&db.app, &ayse, id).await.unwrap()[0].lines[0].amount,
        732_000
    );
}

/// What the signing service does with a request: checks the server's envelope, opens it, signs with the (software)
/// card and gives its result.
fn signed_on_the_card(
    created: &kentos_contracts::SigningRequestCreated,
    trust: &kentos_contracts::SigningTrust,
) -> kentos_imza_domain::BatchResult {
    use kentos_imza_signature::mock_token::{MOCK_PIN, MockToken};
    use kentos_imza_signature::{Choice, RecordSink, sign_batch};
    let guard = kentos_imza_security::TrustConfig::from_base64(
        &trust.public_key,
        trust.issuer.clone(),
        trust.audience.clone(),
    )
    .unwrap();
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    let payload = kentos_imza_security::verify(&created.signed_request, &guard, &now).unwrap();
    let mut record = kentos_imza_domain::BatchRecord::new(payload.to_batch().unwrap());
    record.open().unwrap();
    let batch = record.batch.clone();
    let mut ignore = |_: &str, _: &serde_json::Value| {};
    let mut sink = RecordSink {
        record: &mut record,
        on_event: &mut ignore,
    };
    sign_batch(
        &batch,
        &MockToken::new(),
        &Choice {
            slot_id: 1,
            certificate_id: "01".into(),
        },
        MOCK_PIN,
        &mut sink,
    )
    .unwrap();
    record.result()
}

#[tokio::test]
async fn a_dossier_s_files_are_signed_on_the_card_and_checked_against_the_server_s_own_record() {
    use kentos_application::portal::signing::{self, Signer};
    use kentos_contracts::{SigningClientType, SigningOutcome, SigningRequestCreate};

    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    let tenant = municipality(&db, &root, "imzali", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let (zeynep, _) = staff(&db, &mehmet, tenant, "zeynep", TenantRole::Editor).await;
    let (selin, _) = staff(&db, &mehmet, tenant, "selin", TenantRole::Viewer).await;
    let ayse = professional(&db, &root, "ayse", Profession::Architect).await;
    let id = dossiers::create_application(
        &db.app,
        &ayse,
        &ApplicationCreate {
            tenant_id: tenant.to_string(),
            kind: "building_permit".into(),
            title: "412 ada 7 parsel konut yapı ruhsatı".into(),
            description: String::new(),
            site: site(),
            owner_name: None,
            office_id: None,
        },
    )
    .await
    .unwrap();
    let a = add_file(
        &db,
        &store,
        &ayse,
        id,
        ("mimari.pdf", b"%PDF-1.4 mimari proje", "architectural"),
        Purpose::Package,
    )
    .await
    .unwrap();
    let b = add_file(
        &db,
        &store,
        &ayse,
        id,
        ("statik.pdf", b"%PDF-1.4 statik proje", "structural"),
        Purpose::Package,
    )
    .await
    .unwrap();
    act(&db, &ayse, id, DossierAction::Send, None)
        .await
        .unwrap();
    let ask = SigningRequestCreate {
        client_type: SigningClientType::Desktop,
        file_ids: vec![a.id.clone(), b.id.clone()],
    };

    // Without the server's key there is nothing to sign with.
    let off = signing::request(&db.app, &store, None, &ayse, id, &ask)
        .await
        .unwrap_err();
    assert!(off.to_string().contains("yapılandırılmamış"), "{off}");
    let signer = Signer::generated().unwrap();
    let trust = signer.trust();
    // A viewer reads; signing is an editor's.
    let viewer = signing::request(&db.app, &store, Some(&signer), &selin, id, &ask)
        .await
        .unwrap_err();
    assert!(matches!(viewer, AppError::Forbidden(_)), "{viewer:?}");

    let created = signing::request(&db.app, &store, Some(&signer), &ayse, id, &ask)
        .await
        .unwrap();
    assert_eq!(created.files.len(), 2);
    assert_eq!(
        created.files[0].digest,
        kentos_imza_security::digest::compute(
            kentos_imza_domain::DigestAlgorithm::Sha384,
            b"%PDF-1.4 mimari proje"
        ),
        "the digest is of the bytes the server keeps, as the signing service computes it"
    );
    let result = signed_on_the_card(&created, &trust);
    let tx = Uuid::parse_str(&created.transaction_id).unwrap();
    // Only the one who asked delivers.
    let other = signing::deliver(&db.app, &store, &zeynep, tx, &result)
        .await
        .unwrap_err();
    assert!(matches!(other, AppError::NotFound(_)), "{other:?}");
    let verdict = signing::deliver(&db.app, &store, &ayse, tx, &result)
        .await
        .unwrap();
    assert!(verdict.accepted, "{:?}", verdict.reason);
    assert_eq!(
        (verdict.status, verdict.signed, verdict.required),
        (SigningOutcome::Completed, 2, 2)
    );
    assert!(
        verdict
            .signer
            .as_ref()
            .is_some_and(|s| s.fingerprint_sha256.len() == 64)
    );
    // The first verdict stays.
    let again = signing::deliver(&db.app, &store, &ayse, tx, &result)
        .await
        .unwrap_err();
    assert!(again.to_string().contains("ilk teslim geçerli"), "{again}");

    // Both sides read the signatures; the timeline says who signed.
    let seen = signing::signatures(&db.app, &zeynep, id).await.unwrap();
    assert_eq!(seen.len(), 2);
    assert!(seen.iter().all(|s| s.still_matches
        && s.signer_name == "ayse kişi"
        && s.digest_algorithm == "SHA-384"));
    assert_eq!(signing::signatures(&db.app, &ayse, id).await.unwrap(), seen);
    let page = detail::detail(&db.app, &zeynep, id).await.unwrap();
    assert!(
        page.events
            .iter()
            .any(|e| e.action == "signed" && e.text.contains("2 dosyayı e-imzaladı")),
        "{:?}",
        page.events
    );

    // A signature that does not match is refused, and nothing is kept for it.
    let created = signing::request(&db.app, &store, Some(&signer), &zeynep, id, &ask)
        .await
        .unwrap();
    let mut broken = signed_on_the_card(&created, &trust);
    let value = &mut broken.results[0].signature.signature_value;
    let flipped = if value.starts_with('A') { 'B' } else { 'A' };
    value.replace_range(0..1, &flipped.to_string());
    let tx = Uuid::parse_str(&created.transaction_id).unwrap();
    let verdict = signing::deliver(&db.app, &store, &zeynep, tx, &broken)
        .await
        .unwrap();
    assert_eq!(
        (verdict.status, verdict.signed),
        (SigningOutcome::Partially, 1)
    );
    assert!(
        verdict
            .files
            .iter()
            .any(|f| !f.verified && f.error.is_some())
    );
    assert_eq!(
        signing::signatures(&db.app, &ayse, id).await.unwrap().len(),
        3
    );

    // A request given up is not delivered; another dossier's request is not this one's.
    let created = signing::request(&db.app, &store, Some(&signer), &ayse, id, &ask)
        .await
        .unwrap();
    let tx = Uuid::parse_str(&created.transaction_id).unwrap();
    let result = signed_on_the_card(&created, &trust);
    signing::cancel(&db.app, &ayse, tx).await.unwrap();
    let late = signing::deliver(&db.app, &store, &ayse, tx, &result)
        .await
        .unwrap_err();
    assert!(late.to_string().contains("iptal edildi"), "{late}");
    let wrong = signing::deliver(&db.app, &store, &ayse, Uuid::now_v7(), &result)
        .await
        .unwrap_err();
    assert!(wrong.to_string().contains("ait değil"), "{wrong}");
}

#[tokio::test]
async fn the_platform_keeps_the_religious_holidays_the_deadlines_count_over() {
    use kentos_application::portal::holidays;
    use kentos_contracts::HolidayCreate;

    let Some(db) = TestDb::create().await else {
        return;
    };
    let root = sistem(&db).await;
    let someone = account(&db, "biri").await;
    // The table comes with 2026's and 2027's bayrams; a year lists them with the law's fixed days.
    let year = holidays::list(&db.app, &someone, 2026).await.unwrap();
    assert!(
        year.iter().any(|h| h.day == "2026-05-27"
            && h.name.starts_with("Kurban")
            && !h.fixed
            && !h.half_day)
    );
    assert!(
        year.iter().any(|h| h.day == "2026-05-26" && h.half_day),
        "the arife is half a day"
    );
    assert!(year.iter().any(|h| h.day == "2026-10-29" && h.fixed));
    // Only the platform's administrators write it; a far year keeps the other tests' deadlines as they are.
    let add = HolidayCreate {
        day: "2031-01-24".into(),
        name: "Ramazan Bayramı 1. gün".into(),
        half_day: false,
    };
    let refused = holidays::add(&db.app, &someone, &add).await.unwrap_err();
    assert!(matches!(refused, AppError::Forbidden(_)), "{refused:?}");
    holidays::add(&db.app, &root, &add).await.unwrap();
    let day = time::Date::from_calendar_date(2031, time::Month::January, 24).unwrap();
    assert_eq!(
        holidays::current().closed(day).as_deref(),
        Some("Ramazan Bayramı 1. gün")
    );
    assert_eq!(
        holidays::current().extended(day).0,
        day + time::Duration::days(3),
        "Friday to Monday"
    );
    let fixed = HolidayCreate {
        day: "2031-10-29".into(),
        name: "Cumhuriyet".into(),
        half_day: false,
    };
    let law = holidays::add(&db.app, &root, &fixed).await.unwrap_err();
    assert!(law.to_string().contains("zaten tatil"), "{law}");
    holidays::remove(&db.app, &root, "2031-01-24")
        .await
        .unwrap();
    assert_eq!(holidays::current().closed(day), None);
    assert!(matches!(
        holidays::remove(&db.app, &root, "2031-01-24")
            .await
            .unwrap_err(),
        AppError::NotFound(_)
    ));
    // The platform's record says who changed the calendar.
    let events = platform::events(&db.app, &root, 20).await.unwrap();
    assert!(
        events.iter().any(|e| e.action == "holiday_set")
            && events.iter().any(|e| e.action == "holiday_removed")
    );
}

#[tokio::test]
async fn personal_data_past_its_period_is_archived_or_destroyed_and_each_run_is_recorded() {
    use kentos_application::portal::retention;
    use kentos_contracts::RetentionSettings;

    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    let tenant = municipality(&db, &root, "saklayan", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let mehmet_in = tenancy::access(&db.app, &mehmet, tenant).await.unwrap();
    let ayse = professional(&db, &root, "ayse", Profession::Architect).await;
    let open = |title: &'static str| ApplicationCreate {
        tenant_id: tenant.to_string(),
        kind: "architectural_project".into(),
        title: title.into(),
        description: String::new(),
        site: site(),
        owner_name: Some("Hasan Er".into()),
        office_id: None,
    };
    // A decided application, a draft left for months with a file, a draft of this week.
    let decided = dossiers::create_application(&db.app, &ayse, &open("Karara bağlanmış başvuru"))
        .await
        .unwrap();
    add_file(
        &db,
        &store,
        &ayse,
        decided,
        ("proje.pdf", b"%PDF-1.4 proje", "architectural"),
        Purpose::Package,
    )
    .await
    .unwrap();
    act(&db, &ayse, decided, DossierAction::Send, None)
        .await
        .unwrap();
    act(&db, &mehmet, decided, DossierAction::Approve, None)
        .await
        .unwrap();
    let left = dossiers::create_application(&db.app, &ayse, &open("Unutulmuş taslak"))
        .await
        .unwrap();
    let file = add_file(
        &db,
        &store,
        &ayse,
        left,
        ("eski.pdf", b"%PDF-1.4 eski", "architectural"),
        Purpose::Package,
    )
    .await
    .unwrap();
    let key: String =
        sqlx::query_scalar("select blob_key from kentos.dossier_file where id = $1::uuid")
            .bind(&file.id)
            .fetch_one(&db.owner)
            .await
            .unwrap();
    let fresh = dossiers::create_application(&db.app, &ayse, &open("Bu haftanın taslağı"))
        .await
        .unwrap();
    let back = |table: &'static str, column: &'static str, days: i32| {
        let owner = db.owner.clone();
        async move {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "update kentos.{table} set {column} = now() - make_interval(days => $1)"
            )))
            .bind(days)
            .execute(&owner)
            .await
            .unwrap();
        }
    };
    back("notification", "at", 200).await;
    for (id, days) in [(decided, 400), (left, 200)] {
        sqlx::query("update kentos.dossier set updated_at = now() - make_interval(days => $2) where id = $1")
            .bind(id)
            .bind(days)
            .execute(&db.owner)
            .await
            .unwrap();
    }

    // Only the platform's administrators run it by hand and set its periods.
    let refused = retention::run_now(&db.app, &store, &ayse)
        .await
        .unwrap_err();
    assert!(matches!(refused, AppError::Forbidden(_)), "{refused:?}");
    let short = RetentionSettings {
        archive_after_days: 365,
        draft_keep_days: 5,
        notification_keep_days: 180,
        technical_keep_days: 90,
    };
    let bad = retention::set(&db.app, &root, &short).await.unwrap_err();
    assert_eq!(bad.path(), Some("draftKeepDays"));
    let run = retention::run_now(&db.app, &store, &root).await.unwrap();
    assert_eq!(
        (
            run.trigger.as_str(),
            run.archived,
            run.drafts_deleted,
            run.files_deleted
        ),
        ("manual", 1, 1, 1)
    );
    assert!(run.notifications_deleted >= 1, "{run:?}");
    assert_eq!(
        run.actor_name.as_deref(),
        Some("sistem"),
        "who ran it by hand"
    );
    assert_eq!(run.settings.draft_keep_days, 180);

    // The draft and its bytes are gone; the decided one is in the archive, out of the working list; this week's stays.
    assert!(matches!(
        detail::detail(&db.app, &ayse, left).await.unwrap_err(),
        AppError::NotFound(_)
    ));
    assert!(store.open(&key).await.is_err(), "the file left the store");
    detail::detail(&db.app, &ayse, fresh).await.unwrap();
    let working = dossiers::list_for_tenant(
        &db.app,
        &mehmet_in,
        &ListFilter {
            limit: 50,
            ..ListFilter::default()
        },
    )
    .await
    .unwrap();
    assert!(working.items.iter().all(|d| d.id != decided.to_string()));
    let archive = dossiers::list_for_tenant(
        &db.app,
        &mehmet_in,
        &ListFilter {
            limit: 50,
            archived: true,
            ..ListFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(archive.items.len(), 1);
    assert!(archive.items[0].archived && archive.items[0].id == decided.to_string());

    // The server's daily run finds nothing more; both runs are on record, newest first.
    assert_eq!(retention::sweep(&db.app, &store).await.unwrap(), 0);
    let page = retention::page(&db.app, &root).await.unwrap();
    assert_eq!(
        page.runs
            .iter()
            .map(|r| r.trigger.as_str())
            .collect::<Vec<_>>(),
        ["schedule", "manual"]
    );
    let longer = RetentionSettings {
        archive_after_days: 730,
        ..page.settings
    };
    assert_eq!(
        retention::set(&db.app, &root, &longer)
            .await
            .unwrap()
            .settings
            .archive_after_days,
        730
    );
}

#[tokio::test]
async fn a_municipality_s_figures_for_a_period_and_its_workbook() {
    use kentos_application::portal::statistics;
    use kentos_contracts::WorkbookFormat;

    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = sistem(&db).await;
    let tenant = municipality(&db, &root, "sayan", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let (zeynep, zeynep_in) = staff(&db, &mehmet, tenant, "zeynep", TenantRole::Viewer).await;
    let _ = zeynep;
    let ayse = professional(&db, &root, "ayse", Profession::Architect).await;
    let mut ids = Vec::new();
    for (title, kind) in [
        ("Onaylanacak", "architectural_project"),
        ("Reddedilecek", "architectural_project"),
        ("Bekleyen", "building_permit"),
    ] {
        let id = dossiers::create_application(
            &db.app,
            &ayse,
            &ApplicationCreate {
                tenant_id: tenant.to_string(),
                kind: kind.into(),
                title: title.into(),
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
            ("p.pdf", b"%PDF-1.4 p", "architectural"),
            Purpose::Package,
        )
        .await
        .unwrap();
        act(&db, &ayse, id, DossierAction::Send, None)
            .await
            .unwrap();
        ids.push(id);
    }
    act(&db, &mehmet, ids[0], DossierAction::Approve, None)
        .await
        .unwrap();
    act(
        &db,
        &mehmet,
        ids[1],
        DossierAction::Reject,
        Some("Plana aykırı."),
    )
    .await
    .unwrap();

    // Any member reads them, a viewer too.
    let (from, to) = statistics::period(None, None).unwrap();
    let s = statistics::statistics(&db.app, &zeynep_in, from, to)
        .await
        .unwrap();
    assert_eq!(
        (s.received, s.decided, s.approved, s.rejected, s.open_now),
        (3, 2, 1, 1, 1)
    );
    assert_eq!(s.overdue_now, 0);
    assert!(
        s.average_decision_days.is_some_and(|d| d < 1.0) && s.average_rounds == Some(1.0),
        "{s:?}"
    );
    let architectural = s
        .by_kind
        .iter()
        .find(|k| k.kind == "architectural_project")
        .unwrap();
    assert_eq!(
        (
            architectural.received,
            architectural.approved,
            architectural.rejected
        ),
        (2, 1, 1)
    );
    let this_month = s.by_month.last().unwrap();
    assert_eq!((this_month.received, this_month.decided), (3, 2));
    assert_eq!(
        s.by_month.first().unwrap().month,
        format!("{}-01", from.year())
    );
    assert!(
        s.by_assignee
            .iter()
            .any(|p| p.name == "Atanmamış" && p.open == 1)
    );

    // The workbook: five sheets in an XLSX; the period's dossiers alone in the CSV.
    let (xlsx, name, media) =
        statistics::export(&db.app, &zeynep_in, from, to, WorkbookFormat::Xlsx)
            .await
            .unwrap();
    assert!(xlsx.starts_with(b"PK") && name.ends_with(".xlsx") && media.contains("spreadsheetml"));
    let (csv, _, _) = statistics::export(&db.app, &zeynep_in, from, to, WorkbookFormat::Csv)
        .await
        .unwrap();
    let csv = String::from_utf8(csv).unwrap();
    assert!(
        csv.trim_start_matches('\u{feff}')
            .starts_with("Dosya no;Yön;Tür;Başlık"),
        "{csv}"
    );
    assert_eq!(csv.lines().count(), 4, "a header and three dossiers: {csv}");
    assert!(
        csv.contains("Reddedildi") && csv.contains("ayse kişi"),
        "{csv}"
    );
}

/// The professional's role on a project of the municipality, as the project's rules count it.
async fn role_on(db: &TestDb, who: &Actor, tenant: Uuid, project: Uuid) -> Option<String> {
    let owner: Option<Uuid> = sqlx::query_scalar(
        "select owner_user_id from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(tenant)
    .bind(project)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    let mut tx = db
        .app
        .scoped(kentos_postgres::Scope {
            tenant: None,
            user: Some(who.user_id),
            project: None,
        })
        .await
        .unwrap();
    let role: Option<String> = sqlx::query_scalar("select kentos.project_role($1, $2, $3)")
        .bind(tenant)
        .bind(project)
        .bind(owner)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    role
}

#[tokio::test]
async fn a_transfer_in_the_live_project_makes_its_professional_a_guest_editor_until_it_closes() {
    use kentos_contracts::DossierLiveProjectSet;

    let Some(db) = TestDb::create().await else {
        return;
    };
    let root = sistem(&db).await;
    let tenant = municipality(&db, &root, "canli", "mehmet").await;
    let mehmet = signed_in(&db, "mehmet").await;
    let mehmet_in = tenancy::access(&db.app, &mehmet, tenant).await.unwrap();
    let (zeynep, _) = staff(&db, &mehmet, tenant, "zeynep", TenantRole::Editor).await;
    let can = professional(&db, &root, "can", Profession::Surveyor).await;
    let project = new_project(&db, &mehmet_in, "İmar uygulaması").await;
    let ask = TransferCreate {
        professional_id: can.user_id.to_string(),
        kind: "survey".into(),
        title: "501 ada aplikasyonu".into(),
        description: String::new(),
        site: site(),
        due_on: None,
    };
    let id = dossiers::create_transfer(&db.app, &mehmet_in, &ask)
        .await
        .unwrap();
    let link = |p: Option<Uuid>| DossierLiveProjectSet {
        project_id: p.map(|p| p.to_string()),
    };
    // Linking is a project manager's; a project of another organisation is not found.
    let refused = dossiers::set_live_project(&db.app, &zeynep, id, &link(Some(project)))
        .await
        .unwrap_err();
    assert!(matches!(refused, AppError::Forbidden(_)), "{refused:?}");
    let missing = dossiers::set_live_project(&db.app, &mehmet, id, &link(Some(Uuid::now_v7())))
        .await
        .unwrap_err();
    assert!(matches!(missing, AppError::NotFound(_)), "{missing:?}");
    dossiers::set_live_project(&db.app, &mehmet, id, &link(Some(project)))
        .await
        .unwrap();
    let page = detail::detail(&db.app, &mehmet, id).await.unwrap();
    let live = page.live_project.clone().unwrap();
    assert!(page.can_link_live_project && live.name == "İmar uygulaması" && !live.granted);

    // Sent with no copy: nothing on the project yet; taken on: a guest editor (not a member of the municipality).
    act(&db, &mehmet, id, DossierAction::Send, None)
        .await
        .unwrap();
    assert_eq!(role_on(&db, &can, tenant, project).await, None);
    let theirs = detail::detail(&db.app, &can, id).await.unwrap();
    assert_eq!(
        theirs.live_project.as_ref().map(|l| l.name.as_str()),
        Some("İmar uygulaması"),
        "the professional reads its name"
    );
    assert!(!theirs.can_link_live_project);
    act(&db, &can, id, DossierAction::Accept, None)
        .await
        .unwrap();
    assert_eq!(
        role_on(&db, &can, tenant, project).await.as_deref(),
        Some("editor")
    );
    let (guest, audits): (bool, i64) = sqlx::query_as(
        "select g.guest, (select count(*) from kentos.audit_event a where a.project_id = g.project_id
                            and a.action = 'dossier.live_project.grant')
           from kentos.project_grant g where g.project_id = $1 and g.user_id = $2",
    )
    .bind(project)
    .bind(can.user_id)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert!(
        guest && audits == 1,
        "a guest's grant, in the project's record"
    );
    assert!(
        detail::detail(&db.app, &can, id)
            .await
            .unwrap()
            .live_project
            .unwrap()
            .granted
    );

    // Delivered and completed: the dossier's grant goes back.
    act(&db, &can, id, DossierAction::Deliver, None)
        .await
        .unwrap();
    act(&db, &mehmet, id, DossierAction::Complete, None)
        .await
        .unwrap();
    assert_eq!(role_on(&db, &can, tenant, project).await, None);
    let closed = dossiers::set_live_project(&db.app, &mehmet, id, &link(None))
        .await
        .unwrap_err();
    assert!(closed.to_string().contains("Kapanmış"), "{closed}");
}
