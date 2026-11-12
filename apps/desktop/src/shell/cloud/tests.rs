//! The cloud interface's state, message by message, without a server
//! (docs/adr/0041): what a request would bring is handed to the app as its
//! answer; the tasks that would reach the network are dropped unrun (a
//! dropped request is stopped). Timers run on the tests' own clock.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use kentos_cloud::{ApiFailure, Cloud as Client, DraftStore, Opened, Revision, SaveState, Source};
use kentos_commands::{
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
const SAMPLE: &str = include_str!("../../../../../fixtures/document/v1/sample.json");
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
    let mut s = DocumentSnapshotV1::from_json(SAMPLE).expect("reads");
    // A CBS project: one without a type would be asked it on opening (docs/adr/0165 §1).
    s.settings.workspace = Some(kentos_commands::Workspace::Gis);
    ProjectInfo {
        dimension_sets: Vec::new(),
        casework: Vec::new(),
        constraints: Default::default(),
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
        sheets: None,
        arrays: Vec::new(),
        meta_version: "4".into(),
        data_revision: "7".into(),
        feature_count: "13".into(),
        event_cursor: "9".into(),
        storage,
    }
}

/// The sample drawing as the server would send it: the same persistent ids every time.
fn server_drawing() -> kentos_domain::Document {
    let mut s = DocumentSnapshotV1::from_json(SAMPLE).expect("reads");
    s.settings.workspace = Some(kentos_commands::Workspace::Gis);
    let mut doc = kentos_domain::Document::from_snapshot(s).expect("opens");
    let mut v2 = doc.to_snapshot_v2();
    for (i, uid) in v2.uids.iter_mut().enumerate() {
        let mut id = [0x77u8; 16];
        id[8..].copy_from_slice(&(i as u64 + 1).to_be_bytes());
        *uid = kentos_commands::EntityId(id);
    }
    v2.name = "Ada 101".into();
    doc = kentos_domain::Document::from_snapshot_v2(v2).expect("opens");
    doc
}

fn opened(info: ProjectInfo) -> Opened {
    opened_with(info, server_drawing())
}

fn opened_with(info: ProjectInfo, document: kentos_domain::Document) -> Opened {
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

/// [`open`] with a drawing of the test's own as the server's content (the atlas window needs a book of sheets).
pub(crate) fn open_with(app: &mut App, info: ProjectInfo, document: kentos_domain::Document) {
    app.cloud.open_hint = Some((info.name.clone(), info.storage));
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let id = app.cloud.opening.as_ref().expect("opening").id;
    let task = app.update(crate::cloud::msg(Event::Opened {
        id,
        result: Ok(Once::new(opened_with(info, document))),
    }));
    drive(app, task);
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
                kentos_commands::FeatureChange::Create { id, .. }
                | kentos_commands::FeatureChange::Update { id, .. }
                | kentos_commands::FeatureChange::Edit { id, .. }
                | kentos_commands::FeatureChange::Delete { id } => id.clone(),
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
        merged: Vec::new(),
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
        area_unit: kentos_commands::AreaUnit::M2,
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

#[cfg(test)]
mod signing_in;

mod sending;
pub(crate) use sending::*;

mod drafts;
pub(crate) use drafts::*;

mod offline;

mod renaming;
