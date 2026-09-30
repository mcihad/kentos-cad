//! The cloud interface's state, message by message, without a server
//! (docs/adr/0041): what a request would bring is handed to the app as its
//! answer; the tasks that would reach the network are dropped unrun (a
//! dropped request is stopped). Timers run on the tests' own clock.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use kentos_cloud::{ApiFailure, Cloud as Client, DraftStore, Opened, Revision, SaveState, Source};
use kentos_contracts::{
    AccessSource, CatalogView, CommitResult, ConflictReason, DocumentSnapshotV1, Entity,
    EventFeature, EventPage, EventRecord, FeatureConflict, FeatureOp, FeatureRecord, FileCommitted,
    Me, MembershipView, PointEntity, ProjectAccessView, ProjectChanges, ProjectInfo, ProjectPage,
    ProjectPermission, ProjectRole, ProjectState, ProjectStorage, ProjectSummary, SignInMethod,
    TenantKind, TenantRole, UserView,
};
use kentos_domain::{Slot, Uuid};

use crate::app::{App, Dialog, Message, Then};
use crate::cloud::catalog::List;
use crate::cloud::{Event, Once, words};
use crate::files_testing::{drive, scratch};

pub(crate) const TENANT: &str = "0199aaaa-0000-7000-8000-000000000002";
const PERSONAL: &str = "0199aaaa-0000-7000-8000-000000000003";
pub(crate) const PROJECT: &str = "0199aaaa-0000-7000-8000-000000000001";
const OTHER: &str = "0199aaaa-0000-7000-8000-000000000009";
const USER: &str = "0199aaaa-0000-7000-8000-00000000a1a1";
const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");
/// A password the tests type; it must never be written anywhere.
const SECRET: &str = "çok-gizli-parola-42";

fn member(tenant: &str, name: &str, kind: TenantKind, create: bool) -> MembershipView {
    MembershipView {
        tenant_id: tenant.into(),
        tenant_slug: name.to_lowercase(),
        tenant_name: name.into(),
        tenant_kind: kind,
        role: TenantRole::Editor,
        seat: true,
        active: true,
        capabilities: if create {
            vec!["project.create".into()]
        } else {
            Vec::new()
        },
    }
}

fn me() -> Me {
    Me {
        user: UserView {
            id: USER.into(),
            display_name: "Ayşe Yılmaz".into(),
            email: None,
            method: SignInMethod::Local,
        },
        memberships: vec![
            member(TENANT, "Harita Bürosu", TenantKind::Organization, true),
            member(PERSONAL, "Ayşe Yılmaz", TenantKind::Personal, true),
        ],
    }
}

/// The app signed in (a connection that never reaches a server: nothing is driven).
pub(crate) fn signed_in() -> App {
    let (mut app, _) = App::boot(None);
    app.cloud.client = Some(Client::new("http://127.0.0.1:9").expect("a local address"));
    app.cloud.me = Some(me());
    app
}

pub(crate) fn cloud(app: &mut App, event: Event) {
    let _ = app.update(crate::cloud::msg(event));
}

pub(crate) fn said(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

/// The status bar's save cell's words (cells.rs).
pub(crate) fn cell_text(app: &App) -> String {
    app.save_cell().map(|c| c.view.text).unwrap_or_default()
}

pub(crate) fn last_said(app: &App) -> String {
    said(app).pop().unwrap_or_default()
}

fn uuid(text: &str) -> Uuid {
    Uuid::parse_str(text).expect("a uuid")
}

fn permissions(write: bool) -> Vec<ProjectPermission> {
    let mut p = vec![ProjectPermission::Read];
    if write {
        p.extend([ProjectPermission::FeatureWrite, ProjectPermission::Edit]);
    }
    p
}

pub(crate) fn info(storage: ProjectStorage, write: bool, state: ProjectState) -> ProjectInfo {
    let s = DocumentSnapshotV1::from_json(SAMPLE).expect("reads");
    ProjectInfo {
        blocks: Vec::new(),
        id: PROJECT.into(),
        tenant_id: TENANT.into(),
        tenant_name: "Harita Bürosu".into(),
        tenant_kind: TenantKind::Organization,
        access: ProjectAccessView {
            role: if write {
                ProjectRole::Editor
            } else {
                ProjectRole::Viewer
            },
            via: AccessSource::Grant,
            permissions: permissions(write),
        },
        state,
        name: "Ada 101".into(),
        settings: s.settings,
        origin: s.origin,
        home_view: s.home_view,
        layers: s.layers,
        active_layer: s.active_layer,
        styles: s.styles,
        meta_version: "4".into(),
        data_revision: "7".into(),
        feature_count: "13".into(),
        event_cursor: "9".into(),
        storage,
    }
}

/// The sample drawing as the server would send it: the same persistent ids every time.
fn server_drawing() -> kentos_domain::Document {
    let s = DocumentSnapshotV1::from_json(SAMPLE).expect("reads");
    let mut doc = kentos_domain::Document::from_snapshot(s).expect("opens");
    let mut v2 = doc.to_snapshot_v2();
    for (i, uid) in v2.uids.iter_mut().enumerate() {
        let mut id = [0x77u8; 16];
        id[8..].copy_from_slice(&(i as u64 + 1).to_be_bytes());
        *uid = kentos_contracts::EntityId(id);
    }
    v2.name = "Ada 101".into();
    doc = kentos_domain::Document::from_snapshot_v2(v2).expect("opens");
    doc
}

fn opened(info: ProjectInfo) -> Opened {
    let document = server_drawing();
    let source = match info.storage {
        ProjectStorage::Database => Source::Database {
            versions: document
                .entities()
                .map(|e| {
                    (
                        document.uid(Slot(e.base().id)).expect("an id"),
                        "1".to_owned(),
                    )
                })
                .collect(),
            blocks: Vec::new(),
        },
        ProjectStorage::File => Source::File {
            revision: Some(Revision {
                number: 3,
                sha256: "ab".repeat(32),
            }),
        },
    };
    Opened {
        tenant: uuid(TENANT),
        project: uuid(PROJECT),
        info,
        document,
        source,
    }
}

/// Opens `info`'s project as the catalog would, the server's answer handed
/// over; the copy is written (when there is a place for copies) and the
/// drawing replaced, off the UI thread as in the app.
pub(crate) fn open(app: &mut App, info: ProjectInfo) {
    app.cloud.open_hint = Some((info.name.clone(), info.storage));
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let id = app.cloud.opening.as_ref().expect("opening").id;
    answer_open(app, id, info);
}

/// The server's answer to open `id`, driven through the copy's step.
fn answer_open(app: &mut App, id: u64, info: ProjectInfo) {
    let task = app.update(crate::cloud::msg(Event::Opened {
        id,
        result: Ok(Once::new(opened(info))),
    }));
    drive(app, task);
}

pub(crate) fn database(app: &mut App) {
    open(
        app,
        info(ProjectStorage::Database, true, ProjectState::Active),
    );
}

/// The first object (a point on “Çizim”), moved east by `dx`: an edit of this user.
pub(crate) fn edit(app: &mut App, dx: f64) -> Uuid {
    let doc = app.document.as_mut().expect("open");
    let slot = Slot(doc.model.entities().next().expect("an object").base().id);
    let mut e = doc.model.get(slot).expect("there").clone();
    if let Entity::Point(PointEntity { p, .. }) = &mut e {
        p.x += dx;
    }
    assert!(doc.model.update(slot, e));
    doc.model.uid(slot).expect("an id")
}

/// The server's answer to the command on its way: every object at `version`.
fn answer(app: &App, version: &str) -> CommitResult {
    let envelope = app
        .cloud
        .live
        .as_ref()
        .and_then(|l| l.sent.clone())
        .expect("a command on its way");
    let input: ProjectChanges = serde_json::from_value(envelope.input).expect("its input");
    let versions = input
        .features
        .iter()
        .map(|c| {
            let id = match c {
                kentos_contracts::FeatureChange::Create { id, .. }
                | kentos_contracts::FeatureChange::Update { id, .. }
                | kentos_contracts::FeatureChange::Delete { id } => id.clone(),
            };
            (id, version.to_owned())
        })
        .collect::<BTreeMap<_, _>>();
    CommitResult {
        data_revision: "8".into(),
        meta_version: "4".into(),
        versions,
        deleted: Vec::new(),
        event_seq: "10".into(),
        replayed: false,
    }
}

fn session(app: &App) -> u64 {
    app.document.as_ref().expect("open").session
}

fn commit(app: &mut App, result: Result<CommitResult, ApiFailure>) {
    let session = session(app);
    cloud(app, Event::Committed { session, result });
}

fn events(app: &mut App, result: Result<EventPage, ApiFailure>) {
    let session = session(app);
    cloud(app, Event::Events { session, result });
}

fn state(app: &App) -> SaveState {
    app.cloud
        .live
        .as_ref()
        .expect("a database project")
        .sync
        .state()
}

pub(crate) fn summary(id: &str, name: &str, storage: ProjectStorage) -> ProjectSummary {
    ProjectSummary {
        id: id.into(),
        name: name.into(),
        srid: 5256,
        data_revision: "1".into(),
        updated_at: "2026-09-26T09:00:00Z".into(),
        tenant_id: TENANT.into(),
        tenant_name: "Harita Bürosu".into(),
        tenant_kind: TenantKind::Organization,
        owner_name: "Ayşe Yılmaz".into(),
        access: ProjectAccessView {
            role: ProjectRole::Owner,
            via: AccessSource::Owner,
            permissions: permissions(true),
        },
        project_type: Default::default(),
        description: String::new(),
        tags: Vec::new(),
        state: ProjectState::Active,
        catalog_version: "1".into(),
        created_at: "2026-09-20T09:00:00Z".into(),
        creator_name: "Ayşe Yılmaz".into(),
        area_unit: kentos_contracts::AreaUnit::M2,
        storage,
        favorite: false,
        opened_at: None,
        archived_at: None,
        trashed_at: None,
        trashed_by_name: None,
        purge_after: None,
    }
}

pub(crate) fn page(projects: Vec<ProjectSummary>, total: u32, next: Option<&str>) -> ProjectPage {
    ProjectPage {
        projects,
        total,
        next: next.map(str::to_owned),
        trash_retention_days: 30,
    }
}

// ── Signing in ──────────────────────────────────────────────────────────

#[test]
fn sign_in_checks_the_fields_and_the_address_before_anything_is_sent() {
    let (mut app, _) = App::boot(None);
    let _ = app.run("cloud.signIn");
    assert_eq!(app.dialog, Some(Dialog::SignIn));
    let s = app.cloud.sign_in.as_ref().expect("the window");
    assert_eq!(
        s.server, "http://127.0.0.1:8787",
        "the preference's default"
    );

    cloud(&mut app, Event::SignInSubmit);
    let s = app.cloud.sign_in.as_ref().expect("the window");
    assert_eq!(s.error.as_deref(), Some("Giriş adını ve parolayı yazın."));
    assert!(!s.busy());

    cloud(&mut app, Event::SignInLogin("ayse".into()));
    cloud(&mut app, Event::SignInPassword(SECRET.into()));
    cloud(
        &mut app,
        Event::SignInServer("http://192.168.1.5:8787".into()),
    );
    cloud(&mut app, Event::SignInSubmit);
    let s = app.cloud.sign_in.as_ref().expect("the window");
    let error = s.error.as_deref().unwrap_or_default();
    assert!(
        error.contains("şifresiz http yalnız bu bilgisayardaki sunucuya"),
        "{error}"
    );
    assert!(!s.busy(), "nothing was sent");

    cloud(&mut app, Event::SignInServer("http://127.0.0.1:9".into()));
    cloud(&mut app, Event::SignInSubmit);
    let first = app
        .cloud
        .sign_in
        .as_ref()
        .and_then(|s| s.request())
        .expect("sent");

    // The server's own message goes into the window.
    cloud(
        &mut app,
        Event::SignedIn {
            id: first,
            result: Err(ApiFailure::new(
                401,
                "unauthenticated",
                "Giriş adı ya da parola yanlış.",
            )),
        },
    );
    let s = app.cloud.sign_in.as_ref().expect("the window");
    assert_eq!(s.error.as_deref(), Some("Giriş adı ya da parola yanlış."));
    assert!(app.cloud.me.is_none());

    cloud(&mut app, Event::SignInSubmit);
    let second = app
        .cloud
        .sign_in
        .as_ref()
        .and_then(|s| s.request())
        .expect("sent");
    // A late answer to the first request changes nothing.
    cloud(
        &mut app,
        Event::SignedIn {
            id: first,
            result: Ok(me()),
        },
    );
    assert!(app.cloud.me.is_none());
    cloud(
        &mut app,
        Event::SignedIn {
            id: second,
            result: Ok(me()),
        },
    );
    assert_eq!(
        app.cloud.me.as_ref().map(|m| m.user.display_name.as_str()),
        Some("Ayşe Yılmaz")
    );
    assert_eq!(app.dialog, None);
    assert!(
        app.cloud.sign_in.is_none(),
        "the password went with the window"
    );
    assert_eq!(last_said(&app), "Ayşe Yılmaz olarak giriş yapıldı.");
    // The address that worked is the preference now; the password is nowhere.
    assert_eq!(app.settings.text("cloud.server"), "http://127.0.0.1:9");
    assert!(!app.settings.export_text().contains(SECRET));
    assert!(!said(&app).iter().any(|t| t.contains(SECRET)));
}

#[test]
fn a_command_that_needs_an_account_signs_in_first_and_goes_on() {
    let (mut app, _) = App::boot(None);
    let _ = app.run("cloud.open");
    assert_eq!(app.dialog, Some(Dialog::SignIn));
    app.cloud.sign_in.as_mut().expect("the window").server = "http://127.0.0.1:9".into();
    cloud(&mut app, Event::SignInLogin("ayse".into()));
    cloud(&mut app, Event::SignInPassword(SECRET.into()));
    cloud(&mut app, Event::SignInSubmit);
    let id = app
        .cloud
        .sign_in
        .as_ref()
        .and_then(|s| s.request())
        .expect("sent");
    cloud(
        &mut app,
        Event::SignedIn {
            id,
            result: Ok(me()),
        },
    );
    assert_eq!(
        app.dialog,
        Some(Dialog::Catalog),
        "the catalog opens by itself"
    );
    assert!(app.cloud.catalog.as_ref().is_some_and(|c| c.loading()));
}

#[test]
fn esc_closes_the_sign_in_window_and_its_password_goes_with_it() {
    let (mut app, _) = App::boot(None);
    let _ = app.run("cloud.signIn");
    cloud(&mut app, Event::SignInPassword(SECRET.into()));
    app.close_dialog();
    assert_eq!(app.dialog, None);
    assert!(app.cloud.sign_in.is_none());
}

// ── The catalog ─────────────────────────────────────────────────────────

#[test]
fn the_catalog_pages_its_lists_and_drops_late_answers() {
    let mut app = signed_in();
    let _ = app.run("cloud.open");
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    let first = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    cloud(
        &mut app,
        Event::CatalogView(List::View(CatalogView::Shared)),
    );
    let second = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    assert_ne!(first, second);
    cloud(
        &mut app,
        Event::CatalogPage {
            id: first,
            more: false,
            result: Ok(page(
                vec![summary(OTHER, "Eski liste", ProjectStorage::File)],
                1,
                None,
            )),
        },
    );
    let c = app.cloud.catalog.as_ref().expect("open");
    assert!(c.projects.is_empty(), "the older list's answer is dropped");

    cloud(
        &mut app,
        Event::CatalogPage {
            id: second,
            more: false,
            result: Ok(page(
                vec![
                    summary(PROJECT, "Ada 101", ProjectStorage::Database),
                    summary(OTHER, "Yol projesi", ProjectStorage::File),
                ],
                3,
                Some("sonraki"),
            )),
        },
    );
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_eq!((c.projects.len(), c.total), (2, 3));
    cloud(&mut app, Event::CatalogMore);
    let third = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    let mut last = summary(
        "0199aaaa-0000-7000-8000-00000000000a",
        "Parsel",
        ProjectStorage::File,
    );
    last.state = ProjectState::Archived;
    cloud(
        &mut app,
        Event::CatalogPage {
            id: third,
            more: true,
            result: Ok(page(vec![last], 3, None)),
        },
    );
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_eq!(c.projects.len(), 3, "the next page comes below");
    assert!(c.next.is_none());

    // The server's failure says why, with a retry.
    cloud(&mut app, Event::CatalogRetry);
    let fourth = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    cloud(
        &mut app,
        Event::CatalogPage {
            id: fourth,
            more: false,
            result: Err(ApiFailure::new(
                503,
                "unavailable",
                "Sunucu şu an yanıt veremiyor.",
            )),
        },
    );
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_eq!(c.error.as_deref(), Some("Sunucu şu an yanıt veremiyor."));
}

#[test]
fn the_search_goes_to_the_server_once_typing_rests() {
    let mut app = signed_in();
    let _ = app.run("cloud.open");
    let first = app.cloud.catalog.as_ref().and_then(|c| c.request());
    cloud(&mut app, Event::CatalogSearch("ada".into()));
    let now = Instant::now();
    let _ = app.cloud_tick(now);
    assert_eq!(
        app.cloud.catalog.as_ref().and_then(|c| c.request()),
        first,
        "not yet"
    );
    let _ = app.cloud_tick(now + Duration::from_millis(300));
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_ne!(c.request(), first, "asked with the search");
    assert!(c.search_at.is_none());
}

#[test]
fn kurum_projeleri_lists_the_chosen_organisation() {
    let mut app = signed_in();
    let orgs = app.organizations();
    assert_eq!(orgs, [(TENANT.to_owned(), "Harita Bürosu".to_owned())]);
    let _ = app.run("cloud.open");
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_eq!(c.org.as_deref(), Some(TENANT), "the first organisation");
    cloud(
        &mut app,
        Event::CatalogView(List::View(CatalogView::Organization)),
    );
    assert!(app.cloud.catalog.as_ref().is_some_and(|c| c.loading()));
    // Without an organisation the list says so and asks nothing.
    app.cloud.catalog.as_mut().expect("open").org = None;
    cloud(&mut app, Event::CatalogRetry);
    assert!(app.cloud.catalog.as_ref().is_some_and(|c| !c.loading()));
}

// ── Opening ─────────────────────────────────────────────────────────────

#[test]
fn a_database_project_opens_as_the_drawing_and_says_where_it_is() {
    let mut app = signed_in();
    let _ = app.run("cloud.open");
    database(&mut app);
    let doc = app.document.as_ref().expect("open");
    let source = doc.cloud_source().expect("a cloud project");
    assert_eq!(
        (source.workspace.as_str(), source.storage()),
        ("Harita Bürosu", ProjectStorage::Database)
    );
    assert!(!doc.dirty());
    assert_eq!(app.title(), "Ada 101 — Harita Bürosu — KentOS CAD");
    assert_eq!(
        last_said(&app),
        "“Ada 101” bulut projesi açıldı (Harita Bürosu, Yönetilen PostGIS veritabanı): 13 nesne."
    );
    assert_eq!(app.dialog, None, "the catalog closed");
    assert!(app.cloud.catalog.is_none());
    assert_eq!(state(&app), SaveState::Saved);
    assert_eq!(cell_text(&app), "Buluta kaydedildi");
}

#[test]
fn a_late_open_never_replaces_a_newer_one_or_a_changed_drawing() {
    let mut app = signed_in();
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let first = app.cloud.opening.as_ref().expect("opening").id;
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let second = app.cloud.opening.as_ref().expect("opening").id;
    answer_open(
        &mut app,
        first,
        info(ProjectStorage::Database, true, ProjectState::Active),
    );
    assert!(app.document.is_none(), "the overtaken open is dropped");
    assert!(app.cloud.opening.is_some());
    cloud(&mut app, Event::OpenCancel);
    assert!(app.cloud.opening.is_none());
    assert!(last_said(&app).contains("açılışı durduruldu"));
    answer_open(
        &mut app,
        second,
        info(ProjectStorage::Database, true, ProjectState::Active),
    );
    assert!(app.document.is_none(), "a stopped open changes nothing");

    // The drawing on screen changed while the project was read: it stays.
    database(&mut app);
    let before = session(&app);
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let id = app.cloud.opening.as_ref().expect("opening").id;
    edit(&mut app, 1.0);
    answer_open(
        &mut app,
        id,
        info(ProjectStorage::Database, true, ProjectState::Active),
    );
    assert_eq!(session(&app), before);
    assert!(last_said(&app).contains("açılış sürerken ekrandaki çizim değişti"));
}

#[test]
fn an_archived_project_opens_read_only_with_the_web_s_notice() {
    let mut app = signed_in();
    open(
        &mut app,
        info(ProjectStorage::Database, true, ProjectState::Archived),
    );
    assert_eq!(state(&app), SaveState::Archived);
    assert_eq!(cell_text(&app), "Proje arşivde");
    assert!(said(&app).iter().any(|t| t
        == "“Ada 101” arşivlenmiş bir proje: salt okunur açıldı; değişiklikler buluta kaydedilmez. Düzenlemek için arşivden çıkarılmalı ya da kopyası oluşturulmalı."));
}

#[test]
fn opening_asks_first_about_the_drawing_on_screen() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    app.document.as_mut().expect("open").model.mark_unsaved();
    let _ = app.run("cloud.open");
    let id = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    cloud(
        &mut app,
        Event::CatalogPage {
            id,
            more: false,
            result: Ok(page(
                vec![summary(PROJECT, "Ada 101", ProjectStorage::Database)],
                1,
                None,
            )),
        },
    );
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogOpen);
    let then = Then::OpenCloud {
        tenant: uuid(TENANT),
        project: uuid(PROJECT),
    };
    assert_eq!(app.dialog, Some(Dialog::Unsaved(then)));
    assert!(app.cloud.opening.is_none(), "not before the answer");
    // Vazgeç goes back to the catalog.
    app.close_dialog();
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    cloud(&mut app, Event::CatalogOpen);
    let _ = app.update(Message::DialogConfirmed);
    assert!(app.cloud.opening.is_some(), "Kaydetmeden aç opens it");
}

// ── A database project's autosave ───────────────────────────────────────

#[test]
fn an_edit_goes_a_second_after_the_last_and_at_most_five_after_the_first() {
    let mut app = signed_in();
    database(&mut app);
    let t0 = Instant::now();
    edit(&mut app, 1.0);
    app.cloud_after(t0);
    assert_eq!(state(&app), SaveState::Pending);
    assert_eq!(cell_text(&app), "Kaydedilecek: 1");
    assert!(app.document.as_ref().expect("open").dirty());
    let _ = app.cloud_tick(t0 + Duration::from_millis(900));
    assert!(
        app.cloud.live.as_ref().is_some_and(|l| l.sent.is_none()),
        "not yet"
    );

    // Edits every 800 ms keep it waiting, but not past five seconds.
    for i in 1..=6 {
        edit(&mut app, 1.0);
        app.cloud_after(t0 + Duration::from_millis(800 * i));
    }
    let _ = app.cloud_tick(t0 + Duration::from_millis(4900));
    assert!(app.cloud.live.as_ref().is_some_and(|l| l.sent.is_none()));
    let _ = app.cloud_tick(t0 + Duration::from_millis(5000));
    assert_eq!(state(&app), SaveState::Saving);
    assert_eq!(cell_text(&app), "Kaydediliyor…");
    let key = app
        .cloud
        .live
        .as_ref()
        .and_then(|l| l.sent.clone())
        .expect("sent")
        .idempotency_key;

    // One command at a time: an edit meanwhile waits for the answer.
    edit(&mut app, 1.0);
    app.cloud_after(t0 + Duration::from_millis(5100));
    let _ = app.cloud_tick(t0 + Duration::from_millis(9000));
    let live = app.cloud.live.as_ref().expect("live");
    assert_eq!(
        live.sent.as_ref().map(|e| e.idempotency_key.clone()),
        Some(key)
    );

    let first = answer(&app, "2");
    commit(&mut app, Ok(first));
    // The edit made meanwhile goes at once, and then everything is saved.
    let _ = app.cloud_tick(t0 + Duration::from_millis(9100));
    assert_eq!(state(&app), SaveState::Saving);
    let second = answer(&app, "3");
    commit(&mut app, Ok(second));
    assert_eq!(state(&app), SaveState::Saved);
    assert!(
        !app.document.as_ref().expect("open").dirty(),
        "saved when the server has it"
    );
    assert_eq!(cell_text(&app), "Buluta kaydedildi");
}

#[test]
fn a_passing_failure_waits_and_the_same_command_goes_again() {
    let mut app = signed_in();
    database(&mut app);
    let t0 = Instant::now();
    edit(&mut app, 2.0);
    app.cloud_after(t0);
    let _ = app.cloud_tick(t0 + Duration::from_secs(2));
    let first = app
        .cloud
        .live
        .as_ref()
        .and_then(|l| l.sent.clone())
        .expect("sent");
    commit(
        &mut app,
        Err(ApiFailure::new(
            0,
            "network",
            "Sunucuya ulaşılamadı; bağlantınızı ve sunucu adresini denetleyin.",
        )),
    );
    assert_eq!(state(&app), SaveState::Offline);
    assert_eq!(cell_text(&app), "Çevrimdışı: 1 bekliyor");
    let _ = app.cloud_tick(Instant::now());
    assert!(
        !app.cloud.live.as_ref().expect("live").sending(),
        "it waits first"
    );
    let _ = app.cloud_tick(Instant::now() + Duration::from_secs(2));
    let again = app
        .cloud
        .live
        .as_ref()
        .and_then(|l| l.sent.clone())
        .expect("sent again");
    assert_eq!(
        again.idempotency_key, first.idempotency_key,
        "the same command, the same key"
    );
    assert_eq!(again.input, first.input);
}

#[test]
fn ctrl_s_sends_at_once() {
    let mut app = signed_in();
    database(&mut app);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let _ = app.run("file.save");
    assert_eq!(state(&app), SaveState::Saving, "no waiting for the second");
    assert_eq!(last_said(&app), "Değişiklikler buluta gönderiliyor.");
}

#[test]
fn a_refusal_for_good_shows_the_server_s_message() {
    let mut app = signed_in();
    database(&mut app);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let _ = app.run("file.save");
    commit(
        &mut app,
        Err(ApiFailure::new(
            422,
            "locked_layer",
            "“Çizim” katmanı kilitli; nesneleri değiştirilemez.",
        )),
    );
    assert_eq!(state(&app), SaveState::Error);
    // The web's words in the cell, the server's reason in its tip.
    assert_eq!(cell_text(&app), "Kayıt hatası");
    assert!(
        app.save_cell()
            .expect("a cloud project")
            .tip
            .description
            .contains("Kaydedilmedi: “Çizim” katmanı kilitli; nesneleri değiştirilemez.")
    );
    assert_eq!(
        last_said(&app),
        "Bulut kaydı yapılamadı: “Çizim” katmanı kilitli; nesneleri değiştirilemez."
    );
}

#[test]
fn the_status_words_are_the_brief_s() {
    let reasons = [
        (ConflictReason::Changed, "başkası değiştirdi"),
        (ConflictReason::Deleted, "başkası sildi"),
        (ConflictReason::Exists, "sunucuda zaten var"),
        (ConflictReason::Project, "proje bilgileri değişti"),
    ];
    for (reason, text) in reasons {
        assert_eq!(words::reason(reason), text);
    }
}

// ── Conflicts ───────────────────────────────────────────────────────────

fn conflict_on(app: &mut App, uid: Uuid) {
    let mut server = server_drawing().get(Slot(1)).expect("an object").clone();
    if let Entity::Point(PointEntity { p, .. }) = &mut server {
        p.y += 5.0;
    }
    let record = FeatureRecord {
        id: uid.to_string(),
        version: "5".into(),
        entity: server,
    };
    let failure = ApiFailure::new(409, "conflict", "Başka biri daha önce kaydetti.")
        .with_conflicts(vec![FeatureConflict {
            id: uid.to_string(),
            reason: ConflictReason::Changed,
            expected: Some("1".into()),
            actual: Some("5".into()),
            current: Some(record),
        }]);
    commit(app, Err(failure));
}

#[test]
fn a_conflict_stops_sending_and_the_window_offers_both_choices() {
    let mut app = signed_in();
    database(&mut app);
    let uid = edit(&mut app, 3.0);
    app.cloud_after(Instant::now());
    let _ = app.run("file.save");
    conflict_on(&mut app, uid);
    assert_eq!(state(&app), SaveState::Conflict);
    assert_eq!(cell_text(&app), "Çakışma: 1");
    assert!(app.cloud_available("cloud.conflicts"));
    assert!(
        last_said(&app).starts_with("Kayıt çakışması: 1 nesneyi başka biri daha önce kaydetti.")
    );
    // Nothing more goes until the user chooses.
    let _ = app.cloud_tick(Instant::now() + Duration::from_secs(10));
    assert!(!app.cloud.live.as_ref().expect("live").sending());

    let _ = app.run("cloud.conflicts");
    assert_eq!(app.dialog, Some(Dialog::Conflicts));
    let doc = app.document.as_ref().expect("open");
    assert_eq!(
        crate::cloud::view::describe(doc, &uid.to_string(), None),
        "Nokta · Çizim · P1"
    );

    // Benimkini kaydet: mine goes over the server's version, at once.
    cloud(&mut app, Event::KeepMine);
    assert_eq!(app.dialog, None);
    assert_eq!(state(&app), SaveState::Saving);
    let sent = app
        .cloud
        .live
        .as_ref()
        .and_then(|l| l.sent.clone())
        .expect("sent");
    assert_eq!(
        sent.expected_versions
            .get(&uid.to_string())
            .map(String::as_str),
        Some("5")
    );

    // Another conflict; Sunucudakini al puts the server's copy in the drawing.
    conflict_on(&mut app, uid);
    cloud(&mut app, Event::TakeTheirs);
    assert_eq!(state(&app), SaveState::Saved);
    let doc = app.document.as_ref().expect("open");
    let slot = doc.model.slot_of(uid).expect("there");
    let server = server_drawing();
    let theirs = server.get(Slot(1)).expect("an object");
    match (doc.model.get(slot), theirs) {
        (Some(Entity::Point(mine)), Entity::Point(theirs)) => {
            assert_eq!(mine.p.x, theirs.p.x);
            assert_eq!(mine.p.y, theirs.p.y + 5.0);
        }
        other => panic!("{other:?}"),
    }
    assert!(!doc.dirty());
    assert_eq!(last_said(&app), "Sunucudaki hâller alındı.");
}

// ── Others' changes ─────────────────────────────────────────────────────

fn event(seq: &str, request: &str, features: Vec<EventFeature>) -> EventRecord {
    EventRecord {
        blocks: Vec::new(),
        seq: seq.into(),
        data_revision: seq.into(),
        kind: "project.changes".into(),
        actor: Some("mehmet".into()),
        request_id: Some(request.into()),
        features,
        meta: false,
    }
}

#[test]
fn others_changes_are_waited_for_and_asked_for_again_at_once() {
    let mut app = signed_in();
    database(&mut app);
    let first = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(Slot(1)))
        .expect("an id");
    let _ = app.cloud_tick(Instant::now());
    assert!(
        app.cloud.live.as_ref().expect("live").polling.is_some(),
        "waiting on the server at once"
    );

    let page = EventPage {
        events: vec![event(
            "10",
            "web-başkası",
            vec![EventFeature {
                id: first.to_string(),
                op: FeatureOp::Update,
                version: Some("2".into()),
            }],
        )],
        next: "10".into(),
    };
    events(&mut app, Ok(page));
    // The object is fetched; the server's copy comes into the drawing from outside.
    let mut theirs = server_drawing().get(Slot(1)).expect("an object").clone();
    if let Entity::Point(PointEntity { p, .. }) = &mut theirs {
        p.x += 12.5;
    }
    let incoming = kentos_cloud::Incoming {
        fetch: vec![first],
        cursor: "10".into(),
        ..Default::default()
    };
    let (revision, generation) = {
        let doc = app.document.as_ref().expect("open");
        (doc.model.revision(), doc.model.generation())
    };
    let now = session(&app);
    cloud(
        &mut app,
        Event::Fetched {
            session: now,
            incoming,
            full: false,
            result: Ok(kentos_cloud::Remote {
                records: vec![FeatureRecord {
                    id: first.to_string(),
                    version: "2".into(),
                    entity: theirs.clone(),
                }],
                info: None,
                blocks: None,
            }),
        },
    );
    let doc = app.document.as_ref().expect("open");
    let slot = doc.model.slot_of(first).expect("there");
    match (doc.model.get(slot), &theirs) {
        (Some(Entity::Point(now)), Entity::Point(theirs)) => assert_eq!(now.p.x, theirs.p.x),
        other => panic!("{other:?}"),
    }
    assert_eq!(doc.model.revision(), revision, "nothing to save");
    assert_ne!(
        doc.model.generation(),
        generation,
        "the screen and the store follow"
    );
    assert!(!doc.dirty());
    assert!(
        said(&app)
            .iter()
            .any(|t| t == "Başka bir düzenleyicinin 1 değişikliği çizime alındı.")
    );
    let live = app.cloud.live.as_ref().expect("live");
    assert_eq!(live.sync.cursor(), "10");
    assert!(live.polling.is_some(), "the next wait at once");

    // An empty answer (the wait ended): the next wait at once too.
    events(
        &mut app,
        Ok(EventPage {
            events: Vec::new(),
            next: "10".into(),
        }),
    );
    assert!(app.cloud.live.as_ref().expect("live").polling.is_some());

    // No answer: 1 s, doubling up to 30 s, then again from the cursor.
    events(
        &mut app,
        Err(ApiFailure::new(
            0,
            "network",
            "Sunucuya ulaşılamadı; bağlantınızı ve sunucu adresini denetleyin.",
        )),
    );
    let live = app.cloud.live.as_ref().expect("live");
    assert!(live.polling.is_none());
    assert!(live.poll_at > Instant::now() + Duration::from_millis(500));
    assert_eq!(app.cloud.link, crate::cloud::copy::Link::Offline);
    assert_eq!(
        app.link_state(),
        crate::cloud::cells_plan::LinkState::Offline
    );
}

/// A point of the sample on the hidden “Çizim” layer at `x`, as another editor made it.
fn on_cizim(x: f64) -> Entity {
    let mut e = server_drawing().get(Slot(1)).expect("a point").clone();
    assert_eq!(e.base().layer_id, "cizim");
    if let Entity::Point(PointEntity { p, .. }) = &mut e {
        p.x = x;
    }
    e
}

/// What the server had of `ids` when asked, all on “Çizim”, handed over.
fn fetched(app: &mut App, ids: &[Uuid], cursor: &str) {
    let session = session(app);
    cloud(
        app,
        Event::Fetched {
            session,
            incoming: kentos_cloud::Incoming {
                fetch: ids.to_vec(),
                cursor: cursor.into(),
                ..Default::default()
            },
            full: false,
            result: Ok(kentos_cloud::Remote {
                records: ids
                    .iter()
                    .map(|id| FeatureRecord {
                        id: id.to_string(),
                        version: "5".into(),
                        entity: on_cizim(486610.0),
                    })
                    .collect(),
                info: None,
                blocks: None,
            }),
        },
    );
}

/// Another editor's object on a layer removed here waits for it, unsaid;
/// when an undo brings the layer back the next tick fetches it; one still
/// waiting when the project is left is said, by layer (docs/adr/0081).
#[test]
fn others_objects_wait_for_their_layer_and_come_when_it_does() {
    let mut app = signed_in();
    database(&mut app);
    let removed = |app: &mut App| {
        let doc = app.document.as_mut().expect("open");
        doc.model.remove_layer("cizim").expect("removed");
        app.cloud_after(Instant::now());
    };
    removed(&mut app);
    let theirs = Uuid::now_v7();
    let before = said(&app).len();
    fetched(&mut app, &[theirs], "10");
    assert!(
        app.document
            .as_ref()
            .expect("open")
            .model
            .slot_of(theirs)
            .is_none()
    );
    assert_eq!(said(&app).len(), before, "unsaid while it waits");
    let live = app.cloud.live.as_ref().expect("live");
    assert!(live.polling.is_some() && live.long_poll, "the next wait");

    app.document
        .as_mut()
        .expect("open")
        .model
        .undo()
        .expect("undone");
    app.cloud_after(Instant::now());
    let _ = app.cloud_tick(Instant::now());
    let live = app.cloud.live.as_ref().expect("live");
    assert!(
        live.polling.is_some() && !live.long_poll,
        "a fetch in the wait's place"
    );
    fetched(&mut app, &[theirs], "10");
    let doc = app.document.as_ref().expect("open");
    let slot = doc.model.slot_of(theirs).expect("it came with its layer");
    assert_eq!(doc.model.get(slot).expect("there").base().layer_id, "cizim");

    removed(&mut app);
    fetched(&mut app, &[Uuid::now_v7()], "11");
    app.close_cloud_project();
    assert_eq!(
        last_said(&app),
        "“Çizim” katmanı bu çizimde olmadığı için başka birinin 1 nesnesi burada gösterilmedi; proje yeniden açılınca görünür."
    );
}

/// The server's guard refuses our tree without “Çizim”: someone drew on it
/// meanwhile. The missed events are asked for at once, then the server's
/// tree; the layer comes back, said once, no conflict is said, and our
/// deletions go at once; their object comes with the layer (docs/adr/0081).
#[test]
fn a_refused_tree_gives_back_the_layer_someone_drew_on() {
    let mut app = signed_in();
    database(&mut app);
    let removed = {
        let doc = app.document.as_mut().expect("open");
        doc.model.remove_layer("cizim").expect("removed")
    };
    app.cloud_after(Instant::now());
    let _ = app.run("file.save");
    assert!(
        app.cloud
            .live
            .as_ref()
            .and_then(|l| l.sent.as_ref())
            .is_some()
    );
    let guard = ApiFailure::new(
        409,
        "conflict",
        "“Çizim” katmanında hâlâ nesne var (başka biri eklemiş ya da taşımış olabilir); katman silinmedi. Sunucudaki hâli ile sizinkini karşılaştırın.",
    )
    .with_conflicts(vec![FeatureConflict {
        id: "@project".into(),
        reason: ConflictReason::Project,
        expected: Some("4".into()),
        actual: Some("4".into()),
        current: None,
    }]);
    commit(&mut app, Err(guard));
    assert_eq!(state(&app), SaveState::Conflict);
    let live = app.cloud.live.as_ref().expect("live");
    assert!(
        live.polling.is_some() && !live.long_poll,
        "the missed events at once"
    );

    let theirs = Uuid::now_v7();
    events(
        &mut app,
        Ok(EventPage {
            events: vec![event(
                "10",
                "web-başkası",
                vec![EventFeature {
                    id: theirs.to_string(),
                    op: FeatureOp::Create,
                    version: Some("5".into()),
                }],
            )],
            next: "10".into(),
        }),
    );
    fetched(&mut app, &[theirs], "10");
    assert!(
        app.document
            .as_ref()
            .expect("open")
            .model
            .slot_of(theirs)
            .is_none()
    );
    let session = session(&app);
    cloud(
        &mut app,
        Event::ServerTree {
            session,
            result: Ok(info(ProjectStorage::Database, true, ProjectState::Active)),
        },
    );
    let given: Vec<String> = said(&app)
        .into_iter()
        .filter(|t| t.contains("başkasının nesnesi"))
        .collect();
    assert_eq!(
        given,
        [
            "“Çizim” katmanında başkasının nesnesi olduğu için katman silinmedi; sizin nesneleriniz silindi."
        ]
    );
    assert!(!said(&app).iter().any(|t| t.starts_with("Kayıt çakışması")));
    assert!(
        app.document
            .as_ref()
            .expect("open")
            .model
            .layers()
            .get("cizim")
            .is_some()
    );
    let live = app.cloud.live.as_ref().expect("live");
    assert!(live.sync.conflicts().is_empty());
    let sent = live.sent.clone().expect("our deletions go at once");
    let input: ProjectChanges = serde_json::from_value(sent.input).expect("its input");
    assert!(input.project.is_none(), "the tree is the server's again");
    assert_eq!(input.features.len(), removed);
    assert!(
        live.polling.is_some() && !live.long_poll,
        "their object is fetched"
    );
    fetched(&mut app, &[theirs], "10");
    assert!(
        app.document
            .as_ref()
            .expect("open")
            .model
            .slot_of(theirs)
            .is_some()
    );
}

#[test]
fn a_cursor_the_server_no_longer_continues_opens_the_project_again() {
    let mut app = signed_in();
    database(&mut app);
    let _ = app.cloud_tick(Instant::now() + Duration::from_secs(4));
    events(
        &mut app,
        Err(ApiFailure::new(
            410,
            "resync_required",
            "Olaylar artık tutulmuyor.",
        )),
    );
    assert!(
        app.cloud.opening.is_some(),
        "opened again (nothing was unsent)"
    );
}

#[test]
fn a_deletion_seen_while_following_ends_sending() {
    let mut app = signed_in();
    database(&mut app);
    let _ = app.cloud_tick(Instant::now() + Duration::from_secs(4));
    let mut deleted = event("10", "web-başkası", Vec::new());
    deleted.kind = kentos_contracts::PROJECT_DELETED.into();
    events(
        &mut app,
        Ok(EventPage {
            events: vec![deleted],
            next: "10".into(),
        }),
    );
    assert_eq!(state(&app), SaveState::Deleted);
    assert_eq!(cell_text(&app), "Proje silindi");
    assert!(last_said(&app).starts_with("“Ada 101” bulut projesi silindi."));
    // The web's notice: what happened, what stays, a local copy offered.
    assert_eq!(app.dialog, Some(Dialog::Ended));
    let (title, _, detail) = app.ended_notice().expect("a notice");
    assert_eq!(title, "Proje çöp kutusuna taşındı");
    assert!(detail.contains("yerel bir .kcad"), "{detail}");
    cloud(&mut app, Event::SaveLocal);
    assert_eq!(app.dialog, None, "Farklı kaydet asks where");
    // Nothing is asked any more.
    let _ = app.cloud_tick(Instant::now() + Duration::from_secs(10));
    assert!(app.cloud.live.as_ref().expect("live").polling.is_none());
}

// ── Device drafts and leaving ───────────────────────────────────────────

fn drafts(app: &mut App, dir: PathBuf) {
    app.cloud.drafts = Some(DraftStore::new(dir));
}

#[test]
fn signing_out_keeps_unsent_work_in_the_draft_and_the_next_opening_puts_it_back() {
    let dir = scratch("bulut-taslak");
    let mut app = signed_in();
    drafts(&mut app, dir.clone());
    database(&mut app);
    let uid = edit(&mut app, 7.0);
    app.cloud_after(Instant::now());
    let task = app.run("cloud.signOut");
    assert!(app.cloud.leaving.is_some(), "the draft first");
    drive(&mut app, task);
    assert!(said(&app).iter().any(|t| t
        == "Gönderilmemiş 1 değişiklik bu cihazda saklandı; proje yeniden açılınca gönderilecek."));
    assert_eq!(last_said(&app), "Bulut oturumu kapatıldı.");
    assert!(app.cloud.me.is_none());
    let doc = app.document.as_ref().expect("the drawing stays");
    assert!(doc.cloud_source().is_none(), "a local drawing now");
    assert!(app.cloud.live.is_none());

    // Signed in again and opened: the edit is back, unsent, and goes.
    app.cloud.client = Some(Client::new("http://127.0.0.1:9").expect("local"));
    app.cloud.me = Some(me());
    database(&mut app);
    assert!(
        said(&app)
            .iter()
            .any(|t| t == "Bu cihazda gönderilmemiş değişiklikler vardı; çizime geri kondu.")
    );
    let doc = app.document.as_ref().expect("open");
    let back = doc
        .model
        .get(doc.model.slot_of(uid).expect("there"))
        .expect("an object");
    let server = server_drawing();
    match (back, server.get(Slot(1)).expect("an object")) {
        (Entity::Point(back), Entity::Point(server)) => assert_eq!(back.p.x, server.p.x + 7.0),
        other => panic!("{other:?}"),
    }
    assert!(doc.dirty());
    let live = app.cloud.live.as_ref().expect("live");
    assert_eq!(live.sync.pending(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_command_on_its_way_is_in_the_draft_before_it_leaves() {
    let dir = scratch("bulut-yoldaki");
    let mut app = signed_in();
    drafts(&mut app, dir.clone());
    database(&mut app);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let task = app.run("file.save");
    let live = app.cloud.live.as_ref().expect("live");
    assert!(live.sending(), "waiting for its draft");
    let key = live.sent.clone().expect("planned").idempotency_key;
    // Only the draft is written here: the command itself would reach the network.
    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        panic!("a task");
    };
    use iced::futures::StreamExt as _;
    let written = loop {
        match iced::futures::executor::block_on(stream.next()) {
            Some(iced_runtime::Action::Output(Message::Cloud(e)))
                if matches!(*e, Event::DraftWritten { .. }) =>
            {
                break *e;
            }
            Some(_) => {}
            None => panic!("no draft was written"),
        }
    };
    let Event::DraftWritten { send, result, .. } = &written else {
        unreachable!()
    };
    assert!(*send && result.is_ok(), "{written:?}");
    let files: Vec<PathBuf> = walk(&dir);
    assert_eq!(files.len(), 1, "{files:?}");
    let text = std::fs::read_to_string(&files[0]).expect("reads");
    assert!(text.contains(&key), "the command on its way, with its key");
    assert!(!text.contains(SECRET) && !text.contains("kentos_session"));
    let _ = std::fs::remove_dir_all(dir);
}

fn walk(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

#[test]
fn a_draft_that_cannot_be_written_asks_before_anything_is_lost() {
    let dir = scratch("bulut-yazilamaz");
    // A file where the draft folder should be: nothing can be written under it.
    let blocked = dir.join("dosya");
    std::fs::write(&blocked, b"x").expect("writes");
    let mut app = signed_in();
    drafts(&mut app, blocked);
    database(&mut app);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let task = app.run("cloud.signOut");
    drive(&mut app, task);
    assert_eq!(app.dialog, Some(Dialog::Unsaved(Then::SignOut)));
    assert!(app.cloud.me.is_some(), "not signed out before the answer");
    let q = app.unsaved_question(Then::SignOut);
    assert_eq!(
        q.message,
        "“Ada 101” projesinde buluta gönderilmemiş 1 değişiklik var."
    );
    assert!(
        q.detail.contains("Bu cihaza da yedeklenemediler"),
        "{}",
        q.detail
    );
    assert_eq!(q.confirm, "Göndermeden oturumu kapat");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_viewer_is_told_once_and_leaving_asks() {
    let dir = scratch("bulut-izleyici");
    let mut app = signed_in();
    drafts(&mut app, dir.clone());
    open(
        &mut app,
        info(ProjectStorage::Database, false, ProjectState::Active),
    );
    assert_eq!(state(&app), SaveState::ReadOnly);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let told = said(&app)
        .iter()
        .filter(|t| t.starts_with("Bu projeyi yalnız görüntüleyebilirsiniz"))
        .count();
    assert_eq!(told, 1);
    let _ = app.run("file.open");
    assert_eq!(app.dialog, Some(Dialog::Unsaved(Then::Open)));
    let q = app.unsaved_question(Then::Open);
    assert!(
        q.detail.contains("yalnız görüntüleme yetkiniz var"),
        "{}",
        q.detail
    );
    assert_eq!(q.confirm, "Göndermeden aç");
    let _ = std::fs::remove_dir_all(dir);
}

// ── A file project ──────────────────────────────────────────────────────

pub(crate) fn file_project(app: &mut App) {
    open(app, info(ProjectStorage::File, true, ProjectState::Active));
}

fn save_file(app: &mut App) -> u64 {
    let _ = app.run("file.save");
    let id = app.saving.as_ref().expect("saving").id;
    let _ = app.update(Message::Saving(crate::saving::Event::Encoded {
        id,
        result: Ok(Once::new(vec![1, 2, 3])),
    }));
    id
}

#[test]
fn a_file_project_saves_its_next_revision() {
    let mut app = signed_in();
    file_project(&mut app);
    assert!(app.cloud.live.is_none(), "no autosave for a file project");
    assert_eq!(cell_text(&app), "Buluta kaydedildi · r3");
    edit(&mut app, 1.0);
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::default()));
    assert_eq!(cell_text(&app), "Kaydedilmedi · r3 üstüne");
    let id = save_file(&mut app);
    let s = app.saving.as_ref().expect("saving");
    assert!(s.stage.starts_with("Yükleniyor %"), "{}", s.stage);
    assert!(
        matches!(&s.target, crate::saving::Target::Cloud { based_on: 3, .. }),
        "{:?}",
        s.target
    );
    cloud(
        &mut app,
        Event::FileSaved {
            id,
            result: Ok(FileCommitted {
                revision: "4".into(),
                size: 3,
                sha256: "cd".repeat(32),
                objects: Some("13".into()),
                replayed: false,
            }),
        },
    );
    assert!(app.saving.is_none());
    let doc = app.document.as_ref().expect("open");
    assert!(!doc.dirty());
    assert_eq!(
        doc.cloud_source()
            .and_then(|s| s.revision.as_ref())
            .map(|r| r.number),
        Some(4)
    );
    assert_eq!(last_said(&app), "“Ada 101” buluta kaydedildi: revizyon 4.");
    assert_eq!(cell_text(&app), "Buluta kaydedildi · r4");
}

// ── Buluta yükle ────────────────────────────────────────────────────────

#[test]
fn the_upload_offers_only_workspaces_where_projects_can_be_opened() {
    let mut app = signed_in();
    if let Some(me) = app.cloud.me.as_mut() {
        me.memberships[0].capabilities.clear();
    }
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.upload");
    assert_eq!(app.dialog, Some(Dialog::Upload));
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.tenants.len(), 1);
    assert_eq!(u.tenants[0].tenant_id, PERSONAL);
    assert_eq!(u.storage, ProjectStorage::Database, "the web's default");

    cloud(&mut app, Event::UploadName("  ".into()));
    cloud(&mut app, Event::UploadSubmit);
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(
        u.error.as_deref(),
        Some("Proje adı boş olamaz ve en çok 200 karakter olabilir.")
    );
    assert!(!u.working());
}

/// The catalog's fields go with the new project (the web's `catalogFields`,
/// parity-audit Y1): a change to them is a new upload, with a new key.
#[test]
fn the_upload_takes_the_catalog_s_type_description_and_tags() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.upload");
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(
        u.project_type,
        kentos_contracts::ProjectType::Cad,
        "the web's default"
    );
    let key = |app: &App| {
        app.cloud
            .upload
            .as_ref()
            .map(|u| format!("{:?}", u.key_for_tests()))
    };
    let before = key(&app);
    cloud(
        &mut app,
        Event::UploadType(kentos_contracts::ProjectType::Subdivision),
    );
    assert_ne!(key(&app), before, "a new upload");
    let before = key(&app);
    cloud(&mut app, Event::UploadTags("Kadıköy, 2026".into()));
    assert_ne!(key(&app), before);
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.project_type, kentos_contracts::ProjectType::Subdivision);
    assert_eq!(
        crate::cloud::forms_plan::parse_tags(&u.tags),
        ["Kadıköy", "2026"]
    );
}

#[test]
fn a_refused_upload_names_the_object_and_a_good_one_opens_the_project() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.upload");
    cloud(&mut app, Event::UploadStorage(ProjectStorage::File));
    cloud(&mut app, Event::UploadName("Pafta 12".into()));
    let _ = app.update(crate::cloud::msg(Event::UploadSubmit));
    let (id, key) = app
        .cloud
        .upload
        .as_ref()
        .and_then(|u| u.request())
        .expect("working");
    cloud(
        &mut app,
        Event::UploadEncoded {
            id,
            result: Ok(Once::new(vec![1, 2, 3])),
        },
    );
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.stage.as_deref(), Some("Proje oluşturuluyor…"));
    // How far the file is, then what the server does with it (the web's words).
    cloud(
        &mut app,
        Event::UploadProgress {
            id,
            done: 4 * 1024 * 1024,
            total: 10 * 1024 * 1024,
        },
    );
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.stage.as_deref(), Some("Yükleniyor: %40 (4 MB / 10 MB)"));
    assert_eq!(u.fraction, Some(0.4));
    cloud(
        &mut app,
        Event::UploadProgress {
            id,
            done: 10 * 1024 * 1024,
            total: 10 * 1024 * 1024,
        },
    );
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.stage.as_deref(), Some("Sunucu dosyayı doğruluyor…"));
    let mut refused = ApiFailure::new(
        422,
        "invalid",
        "Dosyanın 1. nesnesi içe aktarılamadı: koordinat sınırın dışında.",
    );
    refused = refused.with_path("entities[0]");
    cloud(
        &mut app,
        Event::Uploaded {
            id,
            result: Err(refused),
        },
    );
    let u = app.cloud.upload.as_ref().expect("the window");
    let error = u.error.clone().unwrap_or_default();
    assert!(
        error.contains("Reddedilen nesne: 1. nesne: Nokta · Çizim · P1; çizimde seçildi."),
        "{error}"
    );
    assert_eq!(app.selection.len(), 1, "selected in the drawing");
    assert!(!u.working());

    // Refused for good, the empty project went to the trash: the next try is
    // a new upload, which a trashed project must not answer.
    cloud(&mut app, Event::UploadSubmit);
    let (id, again) = app
        .cloud
        .upload
        .as_ref()
        .and_then(|u| u.request())
        .expect("working");
    assert_ne!(again, key, "a new key after a refusal for good");
    // No answer: the project stays, and the same key finds it again.
    cloud(
        &mut app,
        Event::UploadEncoded {
            id,
            result: Ok(Once::new(vec![1, 2, 3])),
        },
    );
    cloud(
        &mut app,
        Event::Uploaded {
            id,
            result: Err(ApiFailure::new(0, "network", "Sunucuya ulaşılamadı.")),
        },
    );
    cloud(&mut app, Event::UploadSubmit);
    let (id, third) = app
        .cloud
        .upload
        .as_ref()
        .and_then(|u| u.request())
        .expect("working");
    assert_eq!(third, again, "the same key after no answer");
    cloud(
        &mut app,
        Event::UploadEncoded {
            id,
            result: Ok(Once::new(vec![1, 2, 3])),
        },
    );
    let mut made = info(ProjectStorage::File, true, ProjectState::Active);
    made.name = "Pafta 12".into();
    cloud(
        &mut app,
        Event::Uploaded {
            id,
            result: Ok((
                made,
                kentos_cloud::Uploaded::File(FileCommitted {
                    revision: "1".into(),
                    size: 3,
                    sha256: "ef".repeat(32),
                    objects: Some("13".into()),
                    replayed: false,
                }),
            )),
        },
    );
    assert_eq!(app.dialog, None);
    assert!(app.cloud.upload.is_none());
    assert!(said(&app).iter().any(|t| t.starts_with(
        "“Pafta 12” buluta yüklendi (Harita Bürosu, Dosya (KCAD revizyonları)): 13 nesne."
    )));
    assert_eq!(
        app.cloud.opening.as_ref().map(|o| o.name.as_str()),
        Some("Pafta 12")
    );
}

/// The drawing changed while it went up (the web's f7616e4): the project
/// holds it as it went up, the window closes, nothing opens over the change.
#[test]
fn a_drawing_changed_while_it_went_up_stays_as_it_is() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.upload");
    cloud(&mut app, Event::UploadStorage(ProjectStorage::Database));
    cloud(&mut app, Event::UploadName("Pafta 12".into()));
    let _ = app.update(crate::cloud::msg(Event::UploadSubmit));
    let (id, _) = app
        .cloud
        .upload
        .as_ref()
        .and_then(|u| u.request())
        .expect("working");
    cloud(
        &mut app,
        Event::UploadEncoded {
            id,
            result: Ok(Once::new(vec![1, 2, 3])),
        },
    );
    let doc = app.document.as_mut().expect("open");
    doc.model.remove(&[Slot(4)]);
    let revision = doc.model.revision();
    let mut made = info(ProjectStorage::Database, true, ProjectState::Active);
    made.name = "Pafta 12".into();
    cloud(
        &mut app,
        Event::Uploaded {
            id,
            result: Ok((
                made,
                kentos_cloud::Uploaded::Database(kentos_contracts::ProjectImported {
                    objects: "13".into(),
                    data_revision: "1".into(),
                    meta_version: "1".into(),
                    replayed: false,
                }),
            )),
        },
    );
    assert_eq!(app.dialog, None);
    assert!(app.cloud.upload.is_none());
    assert!(app.cloud.opening.is_none(), "nothing opens over the change");
    let doc = app.document.as_ref().expect("open");
    assert!(doc.cloud_source().is_none() && doc.model.revision() == revision);
    assert_eq!(
        last_said(&app),
        "“Pafta 12” bulut projesi oluşturuldu ve çizim içe aktarıldı (13 nesne), ama çizim yükleme sürerken değişti; ekrandaki çizim projeye bağlanmadı ve değişiklikleri yerinde duruyor. Projeyi Bulut projesi aç ile açın."
    );
}

/// An editor of a database project (objects, not the tree; the web's
/// b19ed6f): Yeni katman and Yeni grup are off with the reason; run anyway,
/// and the row's Yanına yeni katman, Yeniden adlandır and Sil, say it and
/// change nothing; the eye stays the user's own. A file project may.
#[test]
fn an_editor_may_not_change_the_layer_tree_of_a_database_project() {
    use crate::layering::{Event as LayerEvent, TREE_LOCKED};
    let mut app = signed_in();
    let mut project = info(ProjectStorage::Database, true, ProjectState::Active);
    project.access.permissions = vec![ProjectPermission::Read, ProjectPermission::FeatureWrite];
    open(&mut app, project);
    assert!(!app.available("layer.new") && !app.available("layer.newGroup"));
    assert_eq!(app.why_disabled("layer.new"), Some(TREE_LOCKED));
    let nodes = |app: &App| {
        let doc = app.document.as_ref().expect("open");
        (
            doc.model.layers().leaves().len(),
            doc.model.layers().nodes().len(),
        )
    };
    let before = nodes(&app);
    let _ = app.run("layer.new");
    assert_eq!(last_said(&app), TREE_LOCKED);
    let _ = app.run("layer.newGroup");
    assert_eq!(last_said(&app), TREE_LOCKED);
    for event in [
        LayerEvent::AddBeside("cizim".into()),
        LayerEvent::Rename("cizim".into()),
        LayerEvent::Remove("cizim".into()),
    ] {
        let _ = app.update(Message::Layer(event));
        assert_eq!(last_said(&app), TREE_LOCKED);
    }
    assert_eq!(nodes(&app), before);
    assert!(app.renaming.is_none() && app.dialog.is_none());
    let visible = |app: &App| {
        let doc = app.document.as_ref().expect("open");
        doc.model.layers().get("cizim").is_some_and(|l| l.visible)
    };
    let was = visible(&app);
    let _ = app.update(Message::LayerVisible("cizim".into()));
    assert_ne!(visible(&app), was, "the eye still works");

    let mut app = signed_in();
    let mut project = info(ProjectStorage::File, true, ProjectState::Active);
    project.access.permissions = vec![ProjectPermission::Read, ProjectPermission::FeatureWrite];
    open(&mut app, project);
    assert!(
        app.available("layer.new"),
        "a file project saves its tree with the file"
    );
}

#[test]
fn the_status_bar_offers_sign_in_or_says_who_and_where() {
    let (mut app, _) = App::boot(None);
    assert!(app.cloud_available("cloud.signIn") && !app.cloud_available("cloud.signOut"));
    let mut app2 = signed_in();
    database(&mut app2);
    assert!(app2.cloud_available("cloud.signOut") && !app2.cloud_available("cloud.signIn"));
    // Signing out with nothing unsent: at once, the drawing stays as a local one.
    let task = app2.run("cloud.signOut");
    drive(&mut app2, task);
    assert!(app2.cloud.me.is_none());
    assert!(
        app2.document
            .as_ref()
            .is_some_and(|d| d.cloud_source().is_none())
    );
    let _ = app.run("cloud.conflicts");
    assert_eq!(last_said(&app), "Çözülecek bir çakışma yok.");
}

// ── Without a connection: the local copy ────────────────────────────────

/// The app signed in, with places for copies and drafts in `dir`.
fn keeping(dir: &std::path::Path) -> App {
    let mut app = signed_in();
    app.cloud.replicas = Some(kentos_cloud::ReplicaStore::new(dir.join("kopya")));
    drafts(&mut app, dir.join("taslak"));
    app
}

/// The app after a restart without a session: the last account and the server
/// kept in its settings (the id is not a secret), the same places on disk.
fn restarted(dir: &std::path::Path) -> App {
    let (mut app, _) = App::boot(None);
    let _ = app.settings.choose(&[
        (
            "cloud.server",
            serde_json::Value::from("http://127.0.0.1:9"),
        ),
        ("cloud.account", serde_json::Value::from(USER)),
    ]);
    app.cloud.replicas = Some(kentos_cloud::ReplicaStore::new(dir.join("kopya")));
    drafts(&mut app, dir.join("taslak"));
    app
}

#[test]
fn a_project_opens_from_this_device_s_copy_without_a_connection_and_its_work_waits() {
    let dir = scratch("bulut-cevrimdisi");
    let mut app = keeping(&dir);
    database(&mut app);
    assert!(app.cloud.held.is_some(), "the copy is kept and locked");
    let uid = edit(&mut app, 4.0);
    app.cloud_after(Instant::now());
    // The program ends with the edit unsent: its draft is written first.
    let task = app.update(Message::CloseRequested(iced::window::Id::unique()));
    drive(&mut app, task);
    drop(app);

    // Again, without a session: the catalog shows this device's copies.
    let mut app = restarted(&dir);
    let _ = app.run("cloud.open");
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    let c = app.cloud.catalog.as_ref().expect("the window");
    assert_eq!(c.list, List::Device);
    assert_eq!(c.device.len(), 1);
    assert_eq!(c.device[0].info.name, "Ada 101");
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    let task = app.update(crate::cloud::msg(Event::CatalogOpen));
    drive(&mut app, task);
    let doc = app.document.as_ref().expect("opened from the copy");
    assert_eq!(
        doc.cloud_source().map(|s| s.workspace.as_str()),
        Some("Harita Bürosu")
    );
    let back = doc
        .model
        .get(doc.model.slot_of(uid).expect("there"))
        .expect("an object");
    match (back, server_drawing().get(Slot(1)).expect("an object")) {
        (Entity::Point(back), Entity::Point(server)) => assert_eq!(back.p.x, server.p.x + 4.0),
        other => panic!("{other:?}"),
    }
    assert!(
        said(&app)
            .iter()
            .any(|t| t.contains("Çevrimdışı — değişiklikler bu cihazda saklanıyor"))
    );
    // Signed out, the work waits in the draft: said as the web says work waiting offline.
    assert_eq!(cell_text(&app), "Çevrimdışı: 1 bekliyor");
    assert_eq!(
        app.link_state(),
        crate::cloud::cells_plan::LinkState::AuthRequired
    );
    // Nothing goes without a session; the work waits in the draft.
    let _ = app.cloud_tick(Instant::now() + Duration::from_secs(10));
    assert!(!app.cloud.live.as_ref().expect("live").sending());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_second_window_cannot_open_a_project_this_one_holds() {
    let dir = scratch("bulut-kilit");
    let mut app = keeping(&dir);
    database(&mut app);
    let mut other = keeping(&dir);
    other.cloud.open_hint = Some(("Ada 101".into(), ProjectStorage::Database));
    let _ = other.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    assert!(other.cloud.opening.is_none());
    assert!(
        last_said(&other).contains("başka bir KentOS penceresinde açık"),
        "{}",
        last_said(&other)
    );
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_file_project_saved_without_a_connection_waits_in_the_copy_and_goes_later() {
    let dir = scratch("bulut-dosya-bekleyen");
    let mut app = keeping(&dir);
    file_project(&mut app);
    // The session ends: Kaydet keeps the save on this device.
    app.cloud.me = None;
    edit(&mut app, 1.0);
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::default()));
    let _ = save_file(&mut app);
    assert!(app.saving.is_none());
    assert!(
        !app.document.as_ref().expect("open").dirty(),
        "saved on this device"
    );
    assert_eq!(cell_text(&app), "Kaydedildi (bu cihazda) · gönderilecek");
    assert!(app.cloud.held.as_ref().is_some_and(|h| h.kept_save));

    // Signed in again: it goes, based on the revision it was made on.
    app.cloud.me = Some(me());
    app.cloud.link = crate::cloud::copy::Link::Offline;
    let _ = app.came_online();
    assert!(app.cloud.held.as_ref().is_some_and(|h| h.sending.is_some()));
    let session = session(&app);
    cloud(
        &mut app,
        Event::KeptSent {
            session,
            based_on: 3,
            result: Ok(FileCommitted {
                revision: "4".into(),
                size: 3,
                sha256: "cd".repeat(32),
                objects: Some("13".into()),
                replayed: false,
            }),
        },
    );
    assert!(app.cloud.held.as_ref().is_some_and(|h| !h.kept_save));
    assert_eq!(cell_text(&app), "Buluta kaydedildi · r4");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_kept_save_that_meets_a_newer_revision_keeps_both_until_the_choice() {
    let dir = scratch("bulut-dosya-iki");
    let mut app = keeping(&dir);
    file_project(&mut app);
    app.cloud.me = None;
    edit(&mut app, 1.0);
    let _ = save_file(&mut app);
    app.cloud.me = Some(me());
    app.cloud.link = crate::cloud::copy::Link::Offline;
    let _ = app.came_online();
    let session = session(&app);
    let clash =
        ApiFailure::new(409, "conflict", "Başka biri daha önce kaydetti.").with_conflicts(vec![
            FeatureConflict {
                id: "@file".into(),
                reason: ConflictReason::Changed,
                expected: Some("3".into()),
                actual: Some("6".into()),
                current: None,
            },
        ]);
    cloud(
        &mut app,
        Event::KeptSent {
            session,
            based_on: 3,
            result: Err(clash),
        },
    );
    assert_eq!(app.dialog, Some(Dialog::Revision));
    assert_eq!(
        app.cloud.question.as_ref().map(|q| q.id),
        Some(crate::cloud::revisions::QuestionId::Conflict)
    );
    assert!(
        app.cloud.held.as_ref().is_some_and(|h| h.kept_save),
        "never dropped without the choice"
    );
    assert_eq!(cell_text(&app), "Çakışma: r6 kaydedilmiş");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_ended_project_is_marked_in_the_copy_and_says_why_in_the_offline_list() {
    let dir = scratch("bulut-bitmis");
    let mut app = keeping(&dir);
    database(&mut app);
    let mut deleted = event("10", "web-başkası", Vec::new());
    deleted.kind = kentos_contracts::PROJECT_DELETED.into();
    events(
        &mut app,
        Ok(EventPage {
            events: vec![deleted],
            next: "10".into(),
        }),
    );
    assert_eq!(state(&app), SaveState::Deleted);
    let store = app.cloud.replicas.clone().expect("a place for copies");
    let kept = store.list("http://127.0.0.1:9", USER);
    assert_eq!(kept.len(), 1);
    let ended = kept[0].ended.expect("marked");
    assert_eq!(words::ended(ended), "Çöp kutusunda");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn removing_a_copy_asks_about_its_unsent_work_first() {
    let dir = scratch("bulut-kaldir");
    let mut app = keeping(&dir);
    database(&mut app);
    edit(&mut app, 2.0);
    app.cloud_after(Instant::now());
    let task = app.run("cloud.signOut");
    drive(&mut app, task);
    assert!(app.cloud.held.is_none(), "the copy was let go");
    // As a sign-in leaves them: the server and the account (not a secret).
    let _ = app.settings.choose(&[
        (
            "cloud.server",
            serde_json::Value::from("http://127.0.0.1:9"),
        ),
        ("cloud.account", serde_json::Value::from(USER)),
    ]);
    let _ = app.run("cloud.open");
    let c = app.cloud.catalog.as_ref().expect("the window");
    assert_eq!((c.list.clone(), c.device.len()), (List::Device, 1));
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogRemove);
    assert_eq!(app.dialog, Some(Dialog::RemoveCopy));
    assert_eq!(app.cloud.removing.as_ref().map(|r| r.unsent), Some(1));
    // Vazgeç: back to the list, nothing removed.
    app.close_dialog();
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    assert_eq!(app.cloud.catalog.as_ref().map(|c| c.device.len()), Some(1));
    cloud(&mut app, Event::CatalogRemove);
    cloud(&mut app, Event::RemoveConfirmed);
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    assert_eq!(app.cloud.catalog.as_ref().map(|c| c.device.len()), Some(0));
    assert!(walk(&dir.join("taslak")).is_empty(), "its draft went too");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_link_says_live_offline_or_signed_out() {
    use crate::cloud::cells_plan::LinkState;
    let mut app = signed_in();
    database(&mut app);
    assert_eq!(app.link_state(), LinkState::Online);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    assert_eq!(
        cell_text(&app),
        "Kaydedilecek: 1",
        "what waits, in the save cell"
    );
    app.cloud.link = crate::cloud::copy::Link::Offline;
    assert_eq!(app.link_state(), LinkState::Offline);
    app.cloud.me = None;
    assert_eq!(app.link_state(), LinkState::AuthRequired);
}

/// The connection's return with nothing waiting sends nothing, and the next
/// edit still waits its second (a bug the real-server run found).
#[test]
fn after_the_connection_returns_the_next_edit_still_waits_its_second() {
    let mut app = signed_in();
    database(&mut app);
    app.cloud.link = crate::cloud::copy::Link::Offline;
    let _ = app.came_online();
    assert!(
        said(&app)
            .iter()
            .any(|t| t.starts_with("Sunucuya yeniden ulaşıldı"))
    );
    let t0 = Instant::now();
    edit(&mut app, 1.0);
    app.cloud_after(t0);
    let _ = app.cloud_tick(t0 + Duration::from_millis(500));
    assert_eq!(state(&app), SaveState::Pending, "not before its second");
    let _ = app.cloud_tick(t0 + Duration::from_millis(1100));
    assert_eq!(state(&app), SaveState::Saving);
}

/// An answer goes to the copy's log first, then the draft says what is left
/// (docs/adr/0043): here nothing, so the draft is gone.
#[test]
fn an_answer_is_appended_to_the_copy_before_the_draft_is_written() {
    let dir = scratch("bulut-adim");
    let mut app = keeping(&dir);
    database(&mut app);
    assert_eq!(app.cloud.held.as_ref().map(|h| h.steps), Some(0));
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let task = app.run("file.save");
    // The draft with the command on its way reaches the disk (the command itself is not sent here).
    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        panic!("a task");
    };
    use iced::futures::StreamExt as _;
    while let Some(action) = iced::futures::executor::block_on(stream.next()) {
        if let iced_runtime::Action::Output(Message::Cloud(e)) = action
            && matches!(*e, Event::DraftWritten { .. })
        {
            // Written: the command would go now (its request is dropped here).
            let _ = app.update(Message::Cloud(e));
            break;
        }
    }
    assert_eq!(
        walk(&dir.join("taslak")).len(),
        1,
        "the draft holds the command"
    );
    let answer = answer(&app, "2");
    let session = session(&app);
    let task = app.update(crate::cloud::msg(Event::Committed {
        session,
        result: Ok(answer),
    }));
    assert_eq!(
        app.cloud.held.as_ref().map(|h| h.steps),
        Some(1),
        "the step is in the copy before the draft is written"
    );
    drive(&mut app, task);
    assert!(
        walk(&dir.join("taslak")).is_empty(),
        "nothing is left to keep"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// The server does not answer an open: the project opens from this device's copy.
#[test]
fn an_open_the_server_does_not_answer_comes_from_the_copy() {
    let dir = scratch("bulut-yedek-acilis");
    let mut app = keeping(&dir);
    database(&mut app);
    let before = session(&app);
    app.cloud.open_hint = Some(("Ada 101".into(), ProjectStorage::Database));
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let id = app.cloud.opening.as_ref().expect("opening").id;
    let task = app.update(crate::cloud::msg(Event::Opened {
        id,
        result: Err(ApiFailure::new(
            0,
            "network",
            "Sunucuya ulaşılamadı; bağlantınızı ve sunucu adresini denetleyin.",
        )),
    }));
    drive(&mut app, task);
    assert_ne!(session(&app), before, "opened again, from the copy");
    assert_eq!(app.cloud.link, crate::cloud::copy::Link::Offline);
    assert!(
        said(&app)
            .iter()
            .any(|t| t.contains("bu cihazdaki kopyasından açıldı"))
    );
    assert!(app.cloud.held.is_some(), "the copy stays locked");
    let _ = std::fs::remove_dir_all(dir);
}

// ── The open project's actions (actions.rs) ──────────────────────────────

/// The catalog's answer to a lifecycle command on the open project.
fn catalog_change(name: &str, purge_after: Option<&str>) -> kentos_contracts::ProjectCatalogChange {
    let mut project = summary(PROJECT, name, ProjectStorage::File);
    project.purge_after = purge_after.map(str::to_owned);
    kentos_contracts::ProjectCatalogChange {
        project,
        changed: true,
        event_seq: None,
        replayed: false,
    }
}

#[test]
fn buluta_dosya_olarak_kaydet_opens_the_upload_on_file_storage() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.uploadFile");
    assert_eq!(app.dialog, Some(Dialog::Upload));
    assert_eq!(
        app.cloud.upload.as_ref().expect("the window").storage,
        ProjectStorage::File
    );
    cloud(&mut app, Event::Close);
    let _ = app.run("cloud.upload");
    assert_eq!(
        app.cloud.upload.as_ref().expect("the window").storage,
        ProjectStorage::Database
    );
}

/// A file project's name is the catalog's: the drawing takes it quietly,
/// not as an edit; a refusal stays in the window in its words.
#[test]
fn a_file_project_is_renamed_through_the_catalog() {
    let mut app = signed_in();
    file_project(&mut app);
    assert!(app.cloud_available("cloud.rename"));
    let _ = app.run("cloud.rename");
    assert_eq!(app.dialog, Some(Dialog::CloudRename));
    cloud(&mut app, Event::RenameSubmit);
    assert!(
        !app.cloud.rename.as_ref().expect("the window").busy,
        "the same name is not sent"
    );
    cloud(&mut app, Event::RenameInput("  Ada 102 ".into()));
    cloud(&mut app, Event::RenameSubmit);
    let id = {
        let r = app.cloud.rename.as_ref().expect("the window");
        assert!(r.busy);
        r_id(r)
    };
    cloud(
        &mut app,
        Event::Renamed {
            id,
            result: Err(ApiFailure::new(409, "conflict", "Çakışma")),
        },
    );
    let r = app.cloud.rename.as_ref().expect("the window stays");
    assert!(!r.busy);
    assert_eq!(
        r.status,
        Some(crate::cloud::actions::Status::Error(
            "Proje bilgileri bu arada başka biri tarafından değiştirildi. Listeyi yenileyip yeniden deneyin.".into()
        ))
    );
    cloud(&mut app, Event::RenameSubmit);
    let id = r_id(app.cloud.rename.as_ref().expect("the window"));
    cloud(
        &mut app,
        Event::Renamed {
            id,
            result: Ok(catalog_change("Ada 102", None)),
        },
    );
    assert_eq!(app.dialog, None);
    let doc = app.document.as_ref().expect("open");
    assert_eq!(doc.name(), "Ada 102");
    assert!(!doc.dirty(), "taken quietly, not an edit");
    assert_eq!(
        last_said(&app),
        "Proje “Ada 102” olarak yeniden adlandırıldı."
    );
}

fn r_id(r: &crate::cloud::actions::Rename) -> u64 {
    r.id_for_tests()
}

/// A database project's new name goes through its autosave with whatever
/// else waits; said once the server has it, or that it waits on the device.
#[test]
fn a_database_project_is_renamed_through_its_autosave() {
    let mut app = signed_in();
    database(&mut app);
    let _ = app.run("cloud.rename");
    cloud(&mut app, Event::RenameInput("Ada 103".into()));
    cloud(&mut app, Event::RenameSubmit);
    let sent = app
        .cloud
        .live
        .as_ref()
        .and_then(|l| l.sent.clone())
        .expect("the name goes at once");
    let input: ProjectChanges = serde_json::from_value(sent.input).expect("its input");
    assert_eq!(
        input.project.and_then(|p| p.name).as_deref(),
        Some("Ada 103")
    );
    assert_eq!(
        app.dialog,
        Some(Dialog::CloudRename),
        "until the server has it"
    );
    let answered = answer(&app, "8");
    commit(&mut app, Ok(answered));
    assert_eq!(app.dialog, None);
    assert_eq!(
        last_said(&app),
        "Proje “Ada 103” olarak yeniden adlandırıldı."
    );

    // No connection: the name waits on this device.
    let _ = app.run("cloud.rename");
    cloud(&mut app, Event::RenameInput("Ada 104".into()));
    cloud(&mut app, Event::RenameSubmit);
    commit(
        &mut app,
        Err(ApiFailure::new(0, "network", "Sunucuya ulaşılamadı.")),
    );
    assert_eq!(app.dialog, None);
    assert!(
        said(&app).iter().any(
            |t| t == "Yeni ad (“Ada 104”) bu cihazda bekliyor; sunucuya ulaşılınca kaydedilir."
        ),
        "{:?}",
        said(&app)
    );
}

/// Çöpe taşı: asked, then the drawing stays as a local one and the
/// message says until when the project can come back.
#[test]
fn the_open_project_goes_to_the_trash_and_the_drawing_stays() {
    let mut app = signed_in();
    file_project(&mut app);
    // An editor may not move it to the trash; its owner may.
    assert!(!app.cloud_available("cloud.delete"));
    if let Some(source) = app.document.as_mut().and_then(|d| d.cloud_source_mut()) {
        source
            .info
            .access
            .permissions
            .push(ProjectPermission::Delete);
    }
    assert!(app.cloud_available("cloud.delete"));
    let _ = app.run("cloud.delete");
    assert_eq!(app.dialog, Some(Dialog::CloudTrash));
    cloud(&mut app, Event::TrashConfirm);
    let id = app.cloud.trashing.expect("the request on its way");
    cloud(
        &mut app,
        Event::Trashed {
            id,
            result: Ok(catalog_change("Ada 101", Some("2026-10-27T10:00:00Z"))),
        },
    );
    let doc = app.document.as_ref().expect("the drawing stays");
    assert!(doc.cloud_source().is_none(), "no longer a cloud project");
    assert_eq!(
        last_said(&app),
        "“Ada 101” bulut projesi çöp kutusuna taşındı (27.10.2026 tarihine kadar geri yüklenebilir). Çizim ekranda kaldı; saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin."
    );
}

/// Son revizyonu aç: a plain question over a clean drawing; over unsaved
/// work the web's question without “Kaydet ve aç” (4b393ae); a viewer of a
/// database project has no such command.
#[test]
fn son_revizyonu_ac_asks_first() {
    use crate::cloud::revisions::{Answer, QuestionId};
    let mut app = signed_in();
    file_project(&mut app);
    assert!(app.cloud_available("cloud.openNewest"));
    let _ = app.run("cloud.openNewest");
    assert_eq!(app.dialog, Some(Dialog::Revision));
    assert_eq!(
        app.cloud.question.as_ref().map(|q| q.id),
        Some(QuestionId::Newest)
    );
    cloud(&mut app, Event::Close);
    assert!(app.cloud.question.is_none(), "Vazgeç leaves nothing");
    edit(&mut app, 1.0);
    let _ = app.run("cloud.openNewest");
    assert_eq!(
        app.cloud.question.as_ref().map(|q| q.id),
        Some(QuestionId::Unsaved)
    );
    cloud(&mut app, Event::RevisionAnswer(Answer::Discard));
    assert!(
        app.cloud.opening.as_ref().is_some_and(|o| o.newest),
        "the newest revision opens from the server"
    );

    let mut db = signed_in();
    database(&mut db);
    assert!(
        !db.cloud_available("cloud.openNewest"),
        "a file project's command"
    );
}

/// The open project's windows, for the owner. Not run by default:
/// `cargo test -p kentos-desktop cloud::tests::action_screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn action_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "bulut-yeniden-adlandir",
                "bulut-cope-tasi",
                "bulut-son-revizyon",
                "bulut-son-revizyon-kaydedilmemis",
            ] {
                let mut app = signed_in();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                file_project(&mut app);
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "bulut-yeniden-adlandir" => {
                        let _ = app.run("cloud.rename");
                        cloud(
                            &mut app,
                            Event::RenameInput("Ada 101 (tevhit sonrası)".into()),
                        );
                    }
                    "bulut-cope-tasi" => {
                        if let Some(source) =
                            app.document.as_mut().and_then(|d| d.cloud_source_mut())
                        {
                            source
                                .info
                                .access
                                .permissions
                                .push(ProjectPermission::Delete);
                        }
                        let _ = app.run("cloud.delete");
                    }
                    "bulut-son-revizyon" => {
                        let _ = app.run("cloud.openNewest");
                    }
                    _ => {
                        edit(&mut app, 1.0);
                        let _ = app.run("cloud.openNewest");
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// Pictures for the owner: an editor's database project, the pointer
/// resting on Katmanlar's Yeni katman (off, its tip says why; docs/adr/0078);
/// `.run/shots/bulut-agac-kilitli-*`.
/// `cargo test -p kentos-desktop cloud::tests::tree_locked_screens -- --ignored --nocapture`
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn tree_locked_screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let mut app = signed_in();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let mut project = info(ProjectStorage::Database, true, ProjectState::Active);
            project.access.permissions =
                vec![ProjectPermission::Read, ProjectPermission::FeatureWrite];
            open(&mut app, project);
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            // Katmanlar's header: Yeni katman left of Yeni grup.
            let at = Point::new(width - 91.0, 160.0);
            snapshot.input(&mut app, App::view, &mut update, Input::Move(at));
            std::thread::sleep(std::time::Duration::from_millis(500));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("bulut-agac-kilitli-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// Buluta yükle for the owner (the web's UploadDialog, parity-audit Y1–Y4):
/// a database project, a file project with the catalog's fields filled, and
/// a file going up; `.run/shots/bulut-yukle-*`.
/// `cargo test -p kentos-desktop cloud::tests::upload_screens -- --ignored --nocapture`
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn upload_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["veritabani", "dosya", "yukleniyor"] {
                let mut app = signed_in();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::Opened(Some(Ok(Box::new(
                    crate::files_testing::drawing(0),
                )))));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let _ = app.run("cloud.upload");
                if name != "veritabani" {
                    cloud(&mut app, Event::UploadStorage(ProjectStorage::File));
                    cloud(&mut app, Event::UploadName("Kadıköy 1244 ada".into()));
                    cloud(
                        &mut app,
                        Event::UploadType(kentos_contracts::ProjectType::Subdivision),
                    );
                    cloud(&mut app, Event::UploadTags("Kadıköy, 2026".into()));
                }
                if name == "yukleniyor" {
                    let _ = app.update(crate::cloud::msg(Event::UploadSubmit));
                    let (id, _) = app
                        .cloud
                        .upload
                        .as_ref()
                        .and_then(|u| u.request())
                        .expect("working");
                    cloud(
                        &mut app,
                        Event::UploadEncoded {
                            id,
                            result: Ok(Once::new(vec![1, 2, 3])),
                        },
                    );
                    cloud(
                        &mut app,
                        Event::UploadProgress {
                            id,
                            done: 3 * 1024 * 1024,
                            total: 7 * 1024 * 1024,
                        },
                    );
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("bulut-yukle-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
