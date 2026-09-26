//! The autosave's rules without a server: what each edit sends, what an
//! answer or a failure does. The same rules against the real server are in
//! apps/api/src/http/native_tests.rs.

use super::*;
use kentos_contracts::{
    AccessSource, DocumentSnapshotV1, FeatureConflict, PointEntity, ProjectAccessView, ProjectInfo,
    ProjectPermission, ProjectRole, ProjectState, ProjectStorage, TenantKind,
};

use crate::open::{Opened, Source};

const SAMPLE: &str = include_str!("../../../../../fixtures/document/v1/sample.json");

fn opened(permissions: Vec<ProjectPermission>, state: ProjectState) -> Opened {
    let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    let document = Document::from_snapshot(s.clone()).unwrap();
    let versions = document
        .entities()
        .map(|e| (document.uid(Slot(e.base().id)).unwrap(), "1".to_string()))
        .collect();
    Opened {
        tenant: Uuid::parse_str("0199aaaa-0000-7000-8000-000000000002").unwrap(),
        project: Uuid::parse_str("0199aaaa-0000-7000-8000-000000000001").unwrap(),
        info: ProjectInfo {
            id: "0199aaaa-0000-7000-8000-000000000001".into(),
            tenant_id: "0199aaaa-0000-7000-8000-000000000002".into(),
            tenant_name: "Harita Bürosu".into(),
            tenant_kind: TenantKind::Organization,
            access: ProjectAccessView {
                role: ProjectRole::Editor,
                via: AccessSource::Grant,
                permissions,
            },
            state,
            name: s.name,
            settings: s.settings,
            origin: s.origin,
            home_view: s.home_view,
            layers: s.layers,
            active_layer: s.active_layer,
            styles: s.styles,
            meta_version: "4".into(),
            data_revision: "1".into(),
            feature_count: "13".into(),
            event_cursor: "9".into(),
            storage: ProjectStorage::Database,
        },
        document,
        source: Source::Database { versions },
    }
}

fn editor() -> Opened {
    opened(
        vec![
            ProjectPermission::Read,
            ProjectPermission::FeatureWrite,
            ProjectPermission::Edit,
        ],
        ProjectState::Active,
    )
}

fn point(x: f64) -> Entity {
    let mut e = DocumentSnapshotV1::from_json(SAMPLE).unwrap().entities[0].clone();
    if let Entity::Point(PointEntity { p, .. }) = &mut e {
        p.x = x;
    }
    e
}

fn input(env: &CommandEnvelope) -> ProjectChanges {
    serde_json::from_value(env.input.clone()).unwrap()
}

/// The server's answer: every created or updated object at `revision`.
fn committed(env: &CommandEnvelope, revision: u64) -> CommitResult {
    let changes = input(env);
    let mut versions = BTreeMap::new();
    let mut deleted = Vec::new();
    for c in changes.features {
        match c {
            FeatureChange::Create { id, .. } | FeatureChange::Update { id, .. } => {
                versions.insert(id, revision.to_string());
            }
            FeatureChange::Delete { id } => deleted.push(id),
        }
    }
    CommitResult {
        data_revision: revision.to_string(),
        meta_version: if changes.project.is_some() { "5" } else { "4" }.into(),
        versions,
        deleted,
        event_seq: "10".into(),
        replayed: false,
    }
}

#[test]
fn a_new_object_goes_under_its_persistent_id_and_takes_the_servers_version() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    assert_eq!(sync.state(), SaveState::Saved);
    let slot = o.document.add(point(486600.0)).unwrap();
    let uid = o.document.uid(slot).unwrap();
    sync.observe(&o.document);
    assert_eq!((sync.state(), sync.pending()), (SaveState::Pending, 1));
    let env = sync.next(&o.document).unwrap();
    assert_eq!(
        (env.command_name.as_str(), env.version),
        ("project.changes", 1)
    );
    assert!(env.expected_versions.is_empty());
    assert!(env.request_id.starts_with("desktop-"));
    match &input(&env).features[..] {
        [FeatureChange::Create { id, entity }] => {
            assert_eq!(id, &uid.to_string());
            assert_eq!(entity, o.document.get(slot).unwrap());
        }
        other => panic!("{other:?}"),
    }
    sync.answered(&o.document, &committed(&env, 2));
    assert_eq!(sync.version_of(uid), Some("2"));
    assert!(sync.all_sent());
    assert_eq!(sync.state(), SaveState::Saved);
    assert_eq!(sync.next(&o.document), None);
}

#[test]
fn a_change_and_a_removal_name_the_version_they_are_based_on() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let first = Slot(1);
    let second = Slot(2);
    let (a, b) = (
        o.document.uid(first).unwrap(),
        o.document.uid(second).unwrap(),
    );
    assert!(o.document.update(first, point(486700.0)));
    o.document.remove(&[second]);
    let env = sync.next(&o.document).unwrap();
    assert_eq!(
        env.expected_versions,
        BTreeMap::from([(a.to_string(), "1".into()), (b.to_string(), "1".into())])
    );
    let changes = input(&env).features;
    assert!(changes.contains(&FeatureChange::Delete { id: b.to_string() }));
    assert!(
        changes
            .iter()
            .any(|c| matches!(c, FeatureChange::Update { id, .. } if *id == a.to_string()))
    );
    sync.answered(&o.document, &committed(&env, 2));
    assert_eq!((sync.version_of(a), sync.version_of(b)), (Some("2"), None));
    // The removal undone: the object comes back under the same id, as a new object.
    o.document.undo();
    let env = sync.next(&o.document).unwrap();
    assert_eq!(
        input(&env).features,
        vec![FeatureChange::Create {
            id: b.to_string(),
            entity: o.document.get(second).unwrap().clone(),
        }]
    );
}

#[test]
fn an_undo_back_to_what_the_server_has_sends_nothing() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    assert!(o.document.update(Slot(1), point(486700.0)));
    sync.observe(&o.document);
    assert_eq!(sync.pending(), 1);
    o.document.undo();
    assert_eq!(sync.next(&o.document), None);
    assert!(sync.all_sent());
    assert_eq!(sync.state(), SaveState::Saved);
}

#[test]
fn a_lost_answer_sends_the_same_command_again() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.add(point(486600.0)).unwrap();
    let env = sync.next(&o.document).unwrap();
    let lost = ApiFailure::new(0, "network", "Sunucuya ulaşılamadı.");
    assert_eq!(sync.failed(&lost), After::Retry(Duration::from_secs(1)));
    assert_eq!(sync.state(), SaveState::Offline);
    assert_eq!(sync.failed(&lost), After::Retry(Duration::from_secs(2)));
    // An edit meanwhile does not change the command on its way: same key, same content.
    o.document.add(point(486601.0)).unwrap();
    let again = sync.next(&o.document).unwrap();
    assert_eq!(again, env);
    sync.answered(&o.document, &committed(&again, 2));
    // The edit made meanwhile goes next.
    assert_eq!(sync.state(), SaveState::Pending);
    let next = sync.next(&o.document).unwrap();
    assert_ne!(next.idempotency_key, env.idempotency_key);
    assert_eq!(input(&next).features.len(), 1);
}

#[test]
fn a_conflict_stops_sending_until_mine_is_kept() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let uid = o.document.uid(Slot(1)).unwrap();
    assert!(o.document.update(Slot(1), point(486700.0)));
    sync.next(&o.document).unwrap();
    let theirs = point(486800.0);
    let conflict = ApiFailure::new(409, "conflict", "Nesne başkası tarafından değiştirildi.")
        .with_conflicts(vec![FeatureConflict {
            id: uid.to_string(),
            reason: ConflictReason::Changed,
            expected: Some("1".into()),
            actual: Some("3".into()),
            current: Some(FeatureRecord {
                id: uid.to_string(),
                version: "3".into(),
                entity: theirs,
            }),
        }]);
    assert_eq!(sync.failed(&conflict), After::Stop);
    assert_eq!(sync.state(), SaveState::Conflict);
    assert_eq!(sync.conflicts().len(), 1);
    assert!(!sync.wants_to_send());
    o.document.add(point(486601.0)).unwrap();
    assert_eq!(sync.next(&o.document), None);
    sync.keep_mine();
    let env = sync.next(&o.document).unwrap();
    assert_eq!(
        env.expected_versions
            .get(&uid.to_string())
            .map(String::as_str),
        Some("3")
    );
}

#[test]
fn a_viewer_and_an_archived_project_send_nothing() {
    let mut viewer = opened(vec![ProjectPermission::Read], ProjectState::Active);
    let mut sync = ProjectSync::new(&viewer).unwrap();
    assert_eq!(sync.state(), SaveState::ReadOnly);
    viewer.document.add(point(486600.0)).unwrap();
    sync.observe(&viewer.document);
    assert_eq!(sync.next(&viewer.document), None);
    assert_eq!(sync.state(), SaveState::ReadOnly);
    // Made an editor while it is open: what the viewer drew waits, now it may go.
    sync.set_access(true, false);
    assert_eq!(sync.state(), SaveState::Pending);
    assert!(sync.next(&viewer.document).is_some());

    let archived = opened(
        vec![ProjectPermission::Read, ProjectPermission::FeatureWrite],
        ProjectState::Archived,
    );
    let sync = ProjectSync::new(&archived).unwrap();
    assert_eq!(sync.state(), SaveState::Archived);
    assert!(!sync.wants_to_send());
}

#[test]
fn the_metadata_goes_with_its_version_only_for_who_may_change_it() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.rename_layer("bina", "Yapılar");
    let env = sync.next(&o.document).unwrap();
    assert_eq!(
        env.expected_versions,
        BTreeMap::from([(PROJECT_KEY.to_string(), "4".into())])
    );
    let patch = input(&env).project.unwrap();
    assert_eq!(patch.active_layer.as_deref(), Some("parsel"));
    assert!(patch.layers.is_some() && patch.name.is_none() && patch.settings.is_none());
    sync.answered(&o.document, &committed(&env, 2));
    assert!(sync.all_sent());

    // An editor without `project.edit`: the layer change stays here, objects still go.
    let mut e = opened(
        vec![ProjectPermission::Read, ProjectPermission::FeatureWrite],
        ProjectState::Active,
    );
    let mut sync = ProjectSync::new(&e).unwrap();
    e.document.rename_layer("bina", "Yapılar");
    assert_eq!(sync.next(&e.document), None);
    assert!(sync.keeps_meta_here() && sync.all_sent());
    e.document.add(point(486600.0)).unwrap();
    let env = sync.next(&e.document).unwrap();
    assert!(input(&env).project.is_none());
}

#[test]
fn many_objects_go_in_batches() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let many: Vec<Entity> = (0..BATCH + 5).map(|i| point(486000.0 + i as f64)).collect();
    o.document.add_many(many, "ekle").unwrap();
    let first = sync.next(&o.document).unwrap();
    assert_eq!(input(&first).features.len(), BATCH);
    sync.answered(&o.document, &committed(&first, 2));
    let second = sync.next(&o.document).unwrap();
    assert_eq!(input(&second).features.len(), 5);
    sync.answered(&o.document, &committed(&second, 3));
    assert!(sync.all_sent());
}

#[test]
fn refusals_end_or_stop_as_on_the_web() {
    let refused = |code: &str, status: u16| ApiFailure::new(status, code, "Reddedildi.");
    for (code, status, state) in [
        ("project_deleted", 410, SaveState::Deleted),
        ("project_archived", 409, SaveState::Archived),
        ("not_found", 404, SaveState::Revoked),
        ("forbidden", 403, SaveState::Error),
    ] {
        let mut o = editor();
        let mut sync = ProjectSync::new(&o).unwrap();
        o.document.add(point(486600.0)).unwrap();
        sync.next(&o.document).unwrap();
        assert_eq!(sync.failed(&refused(code, status)), After::Stop, "{code}");
        assert_eq!(sync.state(), state, "{code}");
        // The change is not lost: it still waits.
        assert_eq!(sync.pending(), 1, "{code}");
    }
}
