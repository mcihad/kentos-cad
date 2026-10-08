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

pub(super) fn opened(permissions: Vec<ProjectPermission>, state: ProjectState) -> Opened {
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
            blocks: Vec::new(),
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
        source: Source::Database {
            versions,
            blocks: Vec::new(),
        },
    }
}

pub(super) fn editor() -> Opened {
    opened(
        vec![
            ProjectPermission::Read,
            ProjectPermission::FeatureWrite,
            ProjectPermission::Edit,
        ],
        ProjectState::Active,
    )
}

pub(super) fn point(x: f64) -> Entity {
    let mut e = DocumentSnapshotV1::from_json(SAMPLE).unwrap().entities[0].clone();
    if let Entity::Point(PointEntity { p, .. }) = &mut e {
        p.x = x;
    }
    e
}

pub(super) fn input(env: &CommandEnvelope) -> ProjectChanges {
    serde_json::from_value(env.input.clone()).unwrap()
}

/// The server's answer: every created or updated object at `revision`.
pub(super) fn committed(env: &CommandEnvelope, revision: u64) -> CommitResult {
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
    sync.keep_mine(&o.document, None);
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

/// A layer removed with more objects than a command takes (Katmanlar →
/// Sil): the deletions go first, the tree with the last of them, since the
/// server keeps a layer that still holds an object; a new object on a layer
/// the tree adds goes with the tree (the web's `leavingObjects`).
#[test]
fn a_removed_layer_goes_after_its_objects() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let on_cizim = |x: f64| {
        let mut e = point(x);
        e.base_mut().layer_id = "cizim".into();
        e
    };
    let many: Vec<Entity> = (0..BATCH + 2)
        .map(|i| on_cizim(486000.0 + i as f64))
        .collect();
    o.document.add_many(many, "ekle").unwrap();
    let mut revision = 2;
    while let Some(env) = sync.next(&o.document) {
        sync.answered(&o.document, &committed(&env, revision));
        revision += 1;
    }
    let on_cizim_now = o.document.by_layer("cizim").count();
    let new = o
        .document
        .add_layer(kentos_domain::NewLayer::layer("Yeni"), None, false)
        .unwrap();
    let mut e = point(486700.0);
    e.base_mut().layer_id = new.clone();
    o.document.add(e).unwrap();
    assert_eq!(o.document.remove_layer("cizim"), Ok(on_cizim_now));

    let first = sync.next(&o.document).unwrap();
    let c = input(&first);
    assert_eq!(c.features.len(), BATCH);
    assert!(
        c.features
            .iter()
            .all(|f| matches!(f, FeatureChange::Delete { .. }))
    );
    assert!(c.project.is_none() && !first.expected_versions.contains_key(PROJECT_KEY));
    sync.answered(&o.document, &committed(&first, revision));

    let second = sync.next(&o.document).unwrap();
    let c = input(&second);
    let deletes = c
        .features
        .iter()
        .filter(|f| matches!(f, FeatureChange::Delete { .. }))
        .count();
    assert_eq!(deletes, on_cizim_now - BATCH);
    assert!(matches!(
        c.features.last(),
        Some(FeatureChange::Create { .. })
    ));
    let tree = c.project.and_then(|p| p.layers).expect("the tree");
    assert!(tree.iter().all(|n| n.id != "cizim") && tree.iter().any(|n| n.id == new));
    assert_eq!(second.expected_versions[PROJECT_KEY], "4");
    sync.answered(&o.document, &committed(&second, revision + 1));
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

// ── Other editors' commits (remote.rs) ─────────────────────────────────────

use kentos_contracts::{EventFeature, EventPage, EventRecord, FeatureOp};

pub(super) fn event(
    seq: u64,
    request: Option<&str>,
    features: &[(Uuid, FeatureOp)],
    meta: bool,
) -> EventRecord {
    EventRecord {
        blocks: Vec::new(),
        seq: seq.to_string(),
        data_revision: seq.to_string(),
        kind: "project.changes".into(),
        actor: None,
        request_id: request.map(str::to_string),
        features: features
            .iter()
            .map(|(id, op)| EventFeature {
                id: id.to_string(),
                op: *op,
                version: None,
            })
            .collect(),
        meta,
    }
}

pub(super) fn page(events: Vec<EventRecord>) -> EventPage {
    let next = events.last().map_or("9".to_string(), |e| e.seq.clone());
    EventPage { events, next }
}

pub(super) fn record(id: Uuid, version: &str, entity: Entity) -> FeatureRecord {
    FeatureRecord {
        id: id.to_string(),
        version: version.into(),
        entity,
    }
}

fn x_of(doc: &Document, id: Uuid) -> Option<f64> {
    match doc.slot_of(id).and_then(|s| doc.get(s)) {
        Some(Entity::Point(p)) => Some(p.p.x),
        _ => None,
    }
}

#[test]
fn others_objects_come_in_and_this_syncs_own_are_skipped() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    // One of ours on its way and answered: its event comes back and is skipped.
    o.document.add(point(486600.0)).unwrap();
    let mine = sync.next(&o.document).unwrap();
    sync.answered(&o.document, &committed(&mine, 2));
    let first = o.document.uid(Slot(1)).unwrap();
    let second = o.document.uid(Slot(2)).unwrap();
    let theirs = Uuid::now_v7();
    let events = page(vec![
        event(
            10,
            Some(&mine.request_id),
            &[(Uuid::now_v7(), FeatureOp::Create)],
            false,
        ),
        event(
            11,
            Some("web-baska"),
            &[(first, FeatureOp::Update), (theirs, FeatureOp::Create)],
            false,
        ),
        event(12, None, &[(second, FeatureOp::Delete)], false),
    ]);
    let incoming = sync.incoming(&events);
    assert_eq!(incoming.fetch, vec![first, theirs]);
    assert_eq!(incoming.removed, vec![second]);
    assert_eq!(incoming.cursor, "12");
    let taken = sync
        .take_remote(
            &mut o.document,
            incoming,
            Remote {
                records: vec![
                    record(first, "3", point(486900.0)),
                    record(theirs, "3", point(486950.0)),
                ],
                info: None,
                blocks: None,
            },
        )
        .unwrap();
    assert_eq!((taken.changed, taken.conflicts), (3, 0));
    assert_eq!(x_of(&o.document, first), Some(486900.0));
    assert_eq!(x_of(&o.document, theirs), Some(486950.0));
    assert!(o.document.slot_of(second).is_none());
    assert_eq!(
        (
            sync.version_of(first),
            sync.version_of(theirs),
            sync.version_of(second)
        ),
        (Some("3"), Some("3"), None)
    );
    assert_eq!(sync.cursor(), "12");
    // What came from outside is not sent back, and is not this user's to undo.
    assert_eq!(sync.next(&o.document), None);
    assert_eq!(sync.state(), SaveState::Saved);
    // An edit of the object that came in goes over the version it came with.
    let slot = o.document.slot_of(theirs).unwrap();
    assert!(o.document.update(slot, point(486951.0)));
    let env = sync.next(&o.document).unwrap();
    assert_eq!(
        env.expected_versions
            .get(&theirs.to_string())
            .map(String::as_str),
        Some("3")
    );
}

#[test]
fn an_object_changed_here_and_elsewhere_is_a_conflict_until_theirs_is_taken() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let id = o.document.uid(Slot(1)).unwrap();
    let gone = o.document.uid(Slot(2)).unwrap();
    assert!(o.document.update(Slot(1), point(486700.0)));
    let removed_line = o.document.get(Slot(2)).unwrap().clone();
    let mut changed_line = removed_line.clone();
    changed_line.base_mut().label = Some("benim".into());
    assert!(o.document.update(Slot(2), changed_line));
    sync.observe(&o.document);
    let incoming = sync.incoming(&page(vec![event(
        10,
        None,
        &[(id, FeatureOp::Update), (gone, FeatureOp::Delete)],
        false,
    )]));
    let taken = sync
        .take_remote(
            &mut o.document,
            incoming,
            Remote {
                records: vec![record(id, "5", point(486800.0))],
                info: None,
                blocks: None,
            },
        )
        .unwrap();
    // Neither is overwritten: both wait for the user's choice, and nothing is sent.
    assert_eq!((taken.changed, taken.conflicts), (0, 2));
    assert_eq!(x_of(&o.document, id), Some(486700.0));
    assert_eq!(sync.state(), SaveState::Conflict);
    assert_eq!(sync.next(&o.document), None);
    let reasons: Vec<_> = sync
        .conflicts()
        .iter()
        .map(|c| (c.id.clone(), c.reason))
        .collect();
    assert_eq!(
        reasons,
        vec![
            (id.to_string(), ConflictReason::Changed),
            (gone.to_string(), ConflictReason::Deleted)
        ]
    );
    // Theirs: the server's copy comes in, the removed one goes, nothing is left to send.
    sync.take_theirs(&mut o.document, None, None).unwrap();
    assert_eq!(x_of(&o.document, id), Some(486800.0));
    assert!(o.document.slot_of(gone).is_none());
    assert_eq!(sync.version_of(id), Some("5"));
    assert_eq!(sync.state(), SaveState::Saved);
    assert_eq!(sync.next(&o.document), None);
}

#[test]
fn new_metadata_comes_first_so_objects_on_its_new_layer_come_in() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let mut info = o.info.clone();
    let mut layer = info.layers[1].clone();
    layer.id = "yeni".into();
    layer.name = "Yeni katman".into();
    info.layers.push(layer);
    info.meta_version = "6".into();
    let on_new = {
        let mut e = point(486600.0);
        e.base_mut().layer_id = "yeni".into();
        e
    };
    let (a, b) = (Uuid::now_v7(), Uuid::now_v7());
    // Without the metadata the object's layer is not in the drawing: it waits for it, unsaid.
    let incoming = sync.incoming(&page(vec![event(
        10,
        None,
        &[(a, FeatureOp::Create)],
        false,
    )]));
    let taken = sync
        .take_remote(
            &mut o.document,
            incoming,
            Remote {
                records: vec![record(a, "2", on_new.clone())],
                info: None,
                blocks: None,
            },
        )
        .unwrap();
    assert_eq!(taken, Taken::default());
    assert!(o.document.slot_of(a).is_none());
    assert!(sync.is_waiting() && sync.arrived(&o.document).is_empty());
    // With it, the layer comes first and the object with it; not an edit to send back.
    let incoming = sync.incoming(&page(vec![event(
        11,
        None,
        &[(b, FeatureOp::Create)],
        true,
    )]));
    assert!(incoming.meta && incoming.needs_fetch());
    sync.take_remote(
        &mut o.document,
        incoming,
        Remote {
            records: vec![record(b, "3", on_new.clone())],
            info: Some(info),
            blocks: None,
        },
    )
    .unwrap();
    assert!(o.document.layers().get("yeni").is_some());
    assert!(o.document.slot_of(b).is_some());
    // The one that waited: its layer came, so it is fetched and taken in.
    assert_eq!(sync.arrived(&o.document), [a]);
    take_arrived(&mut sync, &mut o.document, vec![record(a, "2", on_new)]);
    assert!(o.document.slot_of(a).is_some() && !sync.is_waiting());
    assert_eq!(sync.next(&o.document), None);
    // The next metadata change here goes over the version that came in.
    o.document.rename_layer("yeni", "Yeni katman 2");
    let env = sync.next(&o.document).unwrap();
    assert_eq!(
        env.expected_versions.get(PROJECT_KEY).map(String::as_str),
        Some("6")
    );
}

#[test]
fn metadata_changed_here_and_elsewhere_is_a_conflict() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.rename_layer("bina", "Yapılar");
    sync.observe(&o.document);
    let mut info = o.info.clone();
    info.name = "Ada 101 (yeni ad)".into();
    info.meta_version = "7".into();
    let incoming = sync.incoming(&page(vec![event(10, None, &[], true)]));
    let taken = sync
        .take_remote(
            &mut o.document,
            incoming,
            Remote {
                records: vec![],
                info: Some(info.clone()),
                blocks: None,
            },
        )
        .unwrap();
    assert_eq!(taken.conflicts, 1);
    assert_eq!(sync.conflicts()[0].id, PROJECT_KEY);
    assert_eq!(o.document.layers().get("bina").unwrap().name, "Yapılar");
    // Mine: the renamed layer goes over the server's metadata version.
    sync.keep_mine(&o.document, None);
    let env = sync.next(&o.document).unwrap();
    assert_eq!(
        env.expected_versions.get(PROJECT_KEY).map(String::as_str),
        Some("7")
    );
    // Or theirs: the server's metadata replaces the drawing's.
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.rename_layer("bina", "Yapılar");
    let incoming = sync.incoming(&page(vec![event(10, None, &[], true)]));
    sync.take_remote(
        &mut o.document,
        incoming,
        Remote {
            records: vec![],
            info: Some(info.clone()),
            blocks: None,
        },
    )
    .unwrap();
    sync.take_theirs(&mut o.document, Some(&info), None)
        .unwrap();
    assert_eq!(o.document.name(), "Ada 101 (yeni ad)");
    assert_eq!(o.document.layers().get("bina").unwrap().name, "Bina");
    assert_eq!(sync.next(&o.document), None);
}

#[test]
fn a_deleted_project_ends_the_sync_and_an_archived_one_after_its_events() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.add(point(486600.0)).unwrap();
    sync.observe(&o.document);
    let mut deleted = event(10, None, &[], false);
    deleted.kind = "project.deleted".into();
    let incoming = sync.incoming(&page(vec![deleted]));
    assert!(!incoming.needs_fetch());
    assert_eq!(sync.state(), SaveState::Deleted);
    assert_eq!(sync.next(&o.document), None);
    // The edit made here is not lost: it still waits.
    assert_eq!(sync.pending(), 1);

    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let id = o.document.uid(Slot(1)).unwrap();
    let mut archived = event(11, None, &[], false);
    archived.kind = "project.archived".into();
    let after = event(12, None, &[(Uuid::now_v7(), FeatureOp::Create)], false);
    let incoming = sync.incoming(&page(vec![
        event(10, None, &[(id, FeatureOp::Update)], false),
        archived,
        after,
    ]));
    assert!(incoming.archived);
    assert_eq!(
        (incoming.fetch.clone(), incoming.cursor.clone()),
        (vec![id], "11".to_string())
    );
    sync.take_remote(
        &mut o.document,
        incoming,
        Remote {
            records: vec![record(id, "4", point(486900.0))],
            info: None,
            blocks: None,
        },
    )
    .unwrap();
    assert_eq!(x_of(&o.document, id), Some(486900.0));
    assert_eq!(sync.state(), SaveState::Archived);
    o.document.add(point(486601.0)).unwrap();
    assert_eq!(sync.next(&o.document), None);
}

#[test]
fn changes_from_outside_wait_while_an_edit_is_open() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let id = o.document.uid(Slot(1)).unwrap();
    let incoming = sync.incoming(&page(vec![event(
        10,
        None,
        &[(id, FeatureOp::Update)],
        false,
    )]));
    let remote = Remote {
        records: vec![record(id, "2", point(486900.0))],
        info: None,
        blocks: None,
    };
    let group = o.document.begin_group("Taşı");
    assert!(
        sync.take_remote(&mut o.document, incoming.clone(), remote.clone())
            .is_err()
    );
    o.document.end_group(group);
    assert_eq!(sync.cursor(), "9");
    sync.take_remote(&mut o.document, incoming, remote).unwrap();
    assert_eq!(x_of(&o.document, id), Some(486900.0));
    assert_eq!(sync.cursor(), "10");
    // An access event says to ask the server what this account may do now.
    let mut access = event(11, None, &[], false);
    access.kind = "project.access".into();
    assert!(sync.incoming(&page(vec![access])).access);
}

// ── Device drafts (draft.rs) ───────────────────────────────────────────────

/// The same project opened again: a fresh sync of what the server has.
pub(super) fn reopened(o: &Opened) -> Opened {
    opened(o.info.access.permissions.clone(), o.info.state)
}

#[test]
fn unsent_work_goes_to_a_draft_and_comes_back_in_a_new_opening() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    assert_eq!(sync.draft(&o.document, "ayse"), None);
    let first = o.document.uid(Slot(1)).unwrap();
    let second = o.document.uid(Slot(2)).unwrap();
    assert!(o.document.update(Slot(1), point(486700.0)));
    o.document.remove(&[Slot(2)]);
    let added = o.document.add(point(486600.0)).unwrap();
    let new_id = o.document.uid(added).unwrap();
    o.document.rename_layer("bina", "Yapılar");
    let draft = sync.draft(&o.document, "ayse").unwrap();
    assert_eq!(
        (draft.version, draft.user_id.as_str(), draft.changes.len()),
        (2, "ayse", 3)
    );
    assert_eq!(
        draft.changes[&second.to_string()],
        DraftChange {
            base: Some("1".into()),
            entity: None
        }
    );
    assert_eq!(draft.changes[&new_id.to_string()].base, None);
    assert_eq!(draft.meta.as_ref().map(|m| m.base.as_str()), Some("4"));
    // Written as the web writes it: camelCase, nulls kept.
    let text = serde_json::to_string(&draft).unwrap();
    assert!(
        text.contains("\"userId\":\"ayse\"") && text.contains("\"base\":null"),
        "{text}"
    );

    // The program ends; the project opens again with what the server has.
    let mut again = reopened(&o);
    let mut sync = ProjectSync::new(&again).unwrap();
    let restored = sync.restore(&mut again.document, draft).unwrap();
    assert_eq!(
        (restored.changed, restored.conflicts, restored.resends),
        (4, 0, false)
    );
    assert!(again.document.is_dirty());
    assert_eq!(x_of(&again.document, first), Some(486700.0));
    assert!(again.document.slot_of(second).is_none());
    assert_eq!(x_of(&again.document, new_id), Some(486600.0));
    assert_eq!(again.document.layers().get("bina").unwrap().name, "Yapılar");
    // It is not this user's to undo (it came back, it was not drawn now), but it is to send.
    assert!(!again.document.can_undo());
    let env = sync.next(&again.document).unwrap();
    let input = input(&env);
    assert_eq!(input.features.len(), 3);
    assert!(input.project.is_some());
    assert_eq!(
        env.expected_versions
            .get(&first.to_string())
            .map(String::as_str),
        Some("1")
    );
}

#[test]
fn the_command_on_its_way_goes_again_with_its_key() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.add(point(486600.0)).unwrap();
    let sent = sync.next(&o.document).unwrap();
    // Edited again while the command is on its way, then the program ends.
    let slot = o
        .document
        .slot_of(
            Uuid::parse_str(&match &input(&sent).features[0] {
                FeatureChange::Create { id, .. } => id.clone(),
                other => panic!("{other:?}"),
            })
            .unwrap(),
        )
        .unwrap();
    assert!(o.document.update(slot, point(486605.0)));
    let draft = sync.draft(&o.document, "ayse").unwrap();
    assert_eq!(draft.inflight.as_ref(), Some(&sent));

    let mut again = reopened(&o);
    let mut sync = ProjectSync::new(&again).unwrap();
    let restored = sync.restore(&mut again.document, draft).unwrap();
    assert!(restored.resends);
    // First the same command, same key; its answer settles it.
    let resent = sync.next(&again.document).unwrap();
    assert_eq!(resent, sent);
    sync.answered(
        &again.document,
        &CommitResult {
            replayed: true,
            ..committed(&resent, 2)
        },
    );
    // Then the newer edit, over the version the command got: no conflict with oneself.
    let id = Uuid::parse_str(match &input(&sent).features[0] {
        FeatureChange::Create { id, .. } => id,
        other => panic!("{other:?}"),
    })
    .unwrap();
    assert_eq!(x_of(&again.document, id), Some(486605.0));
    let next = sync.next(&again.document).unwrap();
    assert_eq!(
        next.expected_versions
            .get(&id.to_string())
            .map(String::as_str),
        Some("2")
    );
    assert!(sync.conflicts().is_empty());
}

#[test]
fn a_draft_the_server_moved_past_is_a_conflict_and_a_lost_layer_is_held() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let id = o.document.uid(Slot(1)).unwrap();
    assert!(o.document.update(Slot(1), point(486700.0)));
    let on_gone_layer = {
        let mut e = point(486650.0);
        e.base_mut().layer_id = "yok".into();
        e
    };
    let mut draft = sync.draft(&o.document, "ayse").unwrap();
    let lost = Uuid::now_v7();
    draft.changes.insert(
        lost.to_string(),
        DraftChange {
            base: None,
            entity: Some(on_gone_layer),
        },
    );
    // Meanwhile someone else saved the object: the new opening has it at version 3.
    let mut again = reopened(&o);
    let versions = match &mut again.source {
        Source::Database { versions, .. } => versions,
        other => panic!("{other:?}"),
    };
    versions.iter_mut().find(|(v, _)| *v == id).unwrap().1 = "3".into();
    let mut sync = ProjectSync::new(&again).unwrap();
    let restored = sync.restore(&mut again.document, draft.clone()).unwrap();
    assert_eq!((restored.changed, restored.conflicts), (1, 1));
    assert_eq!(restored.held.len(), 1);
    assert!(restored.held[0].contains("yok"), "{:?}", restored.held);
    // The drawing shows this device's copy until the user chooses; nothing goes meanwhile.
    assert_eq!(x_of(&again.document, id), Some(486700.0));
    assert_eq!(sync.state(), SaveState::Conflict);
    assert_eq!(sync.next(&again.document), None);
    assert_eq!(sync.conflicts()[0].actual.as_deref(), Some("3"));
    // The held change is never sent, but the next draft keeps it.
    let kept = sync.draft(&again.document, "ayse").unwrap();
    assert!(kept.changes.contains_key(&lost.to_string()));
    sync.keep_mine(&o.document, None);
    let env = sync.next(&again.document).unwrap();
    assert!(
        input(&env)
            .features
            .iter()
            .all(|f| !matches!(f, FeatureChange::Create { id, .. } if *id == lost.to_string()))
    );
    assert_eq!(
        env.expected_versions
            .get(&id.to_string())
            .map(String::as_str),
        Some("3")
    );
}

#[test]
fn a_viewers_edits_never_go_to_a_draft() {
    let mut viewer = opened(vec![ProjectPermission::Read], ProjectState::Active);
    let mut sync = ProjectSync::new(&viewer).unwrap();
    viewer.document.add(point(486600.0)).unwrap();
    assert_eq!(sync.draft(&viewer.document, "dilek"), None);
}

// ── What the server has, for the local copy (base.rs) ─────────────────────

#[test]
fn every_change_of_the_servers_side_is_a_base_step() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    assert_eq!(sync.take_base_step(), None);
    // A command answered: the created object at its version, the removed one gone.
    let second = o.document.uid(Slot(2)).unwrap();
    o.document.remove(&[Slot(2)]);
    let added = o.document.add(point(486600.0)).unwrap();
    let new_id = o.document.uid(added).unwrap();
    o.document.rename_layer("bina", "Yapılar");
    let env = sync.next(&o.document).unwrap();
    sync.answered(&o.document, &committed(&env, 2));
    let step = sync.take_base_step().unwrap();
    assert_eq!(
        step.put
            .iter()
            .map(|p| (p.id.clone(), p.version.clone()))
            .collect::<Vec<_>>(),
        vec![(new_id.to_string(), "2".to_string())]
    );
    assert_eq!(step.remove, vec![second.to_string()]);
    let meta = step.meta.unwrap();
    assert_eq!(meta.version, "5");
    assert!(
        meta.layers
            .iter()
            .any(|l| l.children.iter().any(|c| c.name == "Yapılar"))
            || meta.layers.iter().any(|l| l.name == "Yapılar")
    );
    assert_eq!(step.cursor, None);
    // Taken: nothing is left to take.
    assert_eq!(sync.take_base_step(), None);
    // Another editor's change and the cursor after it.
    let first = o.document.uid(Slot(1)).unwrap();
    let incoming = sync.incoming(&page(vec![event(
        12,
        None,
        &[(first, FeatureOp::Update)],
        false,
    )]));
    sync.take_remote(
        &mut o.document,
        incoming,
        Remote {
            records: vec![record(first, "7", point(486900.0))],
            info: None,
            blocks: None,
        },
    )
    .unwrap();
    let step = sync.take_base_step().unwrap();
    assert_eq!(
        (
            step.cursor.as_deref(),
            step.put.len(),
            step.put[0].version.as_str()
        ),
        (Some("12"), 1, "7")
    );
}

#[test]
fn a_version_this_device_knows_is_not_taken_again_nor_a_conflict() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let first = o.document.uid(Slot(1)).unwrap();
    // This device's own commit, answered before the program ended: its version is known.
    assert!(o.document.update(Slot(1), point(486700.0)));
    let env = sync.next(&o.document).unwrap();
    sync.answered(&o.document, &committed(&env, 5));
    // Edited again, unsent.
    assert!(o.document.update(Slot(1), point(486750.0)));
    sync.observe(&o.document);
    // After a restart this sync no longer knows the command's request id: its event comes back as anyone's.
    let incoming = sync.incoming(&page(vec![event(
        13,
        Some("desktop-onceki"),
        &[(first, FeatureOp::Update)],
        false,
    )]));
    let taken = sync
        .take_remote(
            &mut o.document,
            incoming,
            Remote {
                records: vec![record(first, "5", point(486700.0))],
                info: None,
                blocks: None,
            },
        )
        .unwrap();
    // The server's version is the one known here: no conflict, and the unsent edit stays.
    assert_eq!((taken.changed, taken.conflicts), (0, 0));
    assert!(sync.conflicts().is_empty());
    assert_eq!(x_of(&o.document, first), Some(486750.0));
    let next = sync.next(&o.document).unwrap();
    assert_eq!(
        next.expected_versions
            .get(&first.to_string())
            .map(String::as_str),
        Some("5")
    );
}

#[test]
fn a_removal_known_already_does_nothing() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let second = o.document.uid(Slot(2)).unwrap();
    o.document.remove(&[Slot(2)]);
    let env = sync.next(&o.document).unwrap();
    sync.answered(&o.document, &committed(&env, 5));
    let incoming = sync.incoming(&page(vec![event(
        13,
        None,
        &[(second, FeatureOp::Delete)],
        false,
    )]));
    let taken = sync
        .take_remote(&mut o.document, incoming, Remote::default())
        .unwrap();
    assert_eq!((taken.changed, taken.conflicts), (0, 0));
    assert_eq!(sync.state(), SaveState::Saved);
}

#[test]
fn what_the_command_on_its_way_carries_is_in_the_drawing_at_once() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let added = o.document.add(point(486600.0)).unwrap();
    let id = o.document.uid(added).unwrap();
    let gone = o.document.uid(Slot(2)).unwrap();
    o.document.remove(&[Slot(2)]);
    let sent = sync.next(&o.document).unwrap();
    let draft = sync.draft(&o.document, "ayse").unwrap();

    // Opened again without a connection: the drawing already shows this device's work.
    let mut again = reopened(&o);
    let mut sync = ProjectSync::new(&again).unwrap();
    sync.restore(&mut again.document, draft.clone()).unwrap();
    assert_eq!(x_of(&again.document, id), Some(486600.0));
    assert!(again.document.slot_of(gone).is_none());
    assert_eq!(sync.pending(), 2);
    // It goes as it went; once answered nothing else is left.
    assert_eq!(sync.next(&again.document).unwrap(), sent);
    sync.answered(&again.document, &committed(&sent, 2));
    assert_eq!(sync.next(&again.document), None);
    assert!(sync.all_sent());

    // Refused instead (a rule, for good): the work stays in the drawing and still waits.
    let mut again = reopened(&o);
    let mut sync = ProjectSync::new(&again).unwrap();
    sync.restore(&mut again.document, draft).unwrap();
    sync.next(&again.document).unwrap();
    assert_eq!(
        sync.failed(&ApiFailure::new(403, "forbidden", "Kilitli katman.")),
        After::Stop
    );
    assert_eq!(x_of(&again.document, id), Some(486600.0));
    assert_eq!(sync.pending(), 2);
}

/// The sample's tree without a node (another editor's removal).
fn without(nodes: &[LayerNode], id: &str) -> Vec<LayerNode> {
    nodes
        .iter()
        .filter(|n| n.id != id)
        .map(|n| LayerNode {
            children: without(&n.children, id),
            ..n.clone()
        })
        .collect()
}

/// A layer another editor removed while this device had an object on it
/// not sent yet (the web's 88ca558): the layer stays, in its group, and the
/// notice counts the object; the next command sends the tree with it over
/// the server's metadata version, and the object with it.
#[test]
fn a_removed_layer_stays_while_it_holds_unsent_objects() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let mut e = point(486700.0);
    e.base_mut().layer_id = "bina".into();
    o.document.add(e).unwrap();
    let mut info = o.info.clone();
    info.layers = without(&info.layers, "bina");
    info.meta_version = "6".into();
    let incoming = sync.incoming(&page(vec![event(10, None, &[], true)]));
    let taken = sync
        .take_remote(
            &mut o.document,
            incoming,
            Remote {
                records: vec![],
                info: Some(info),
                blocks: None,
            },
        )
        .unwrap();
    assert_eq!(
        taken.kept,
        [KeptLayer {
            id: "bina".into(),
            name: "Bina".into(),
            unsent: 1,
        }]
    );
    assert_eq!(
        taken.kept[0].text(),
        "“Bina” katmanını başka biri sildi; üzerinde gönderilmemiş 1 nesneniz olduğu için katman bu çizimde kaldı ve yeniden kaydedilecek."
    );
    assert_eq!(
        o.document.layers().parent("bina").map(|g| g.id.as_str()),
        Some("layer-g"),
        "back in its group"
    );
    let env = sync.next(&o.document).unwrap();
    let c = input(&env);
    let layers = c.project.and_then(|p| p.layers).expect("the tree goes");
    assert!(remote::is_layer(&layers, "bina"));
    assert_eq!(
        env.expected_versions.get(PROJECT_KEY).map(String::as_str),
        Some("6")
    );
    assert!(
        c.features
            .iter()
            .any(|f| matches!(f, FeatureChange::Create { .. }))
    );
}

/// A removed layer whose objects are all on the server goes, as the other
/// editor wants, with the objects the same events delete; nothing is said.
#[test]
fn a_removed_layer_with_everything_sent_goes() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let on_bina: Vec<(Uuid, FeatureOp)> = o
        .document
        .by_layer("bina")
        .map(|e| {
            (
                o.document.uid(Slot(e.base().id)).unwrap(),
                FeatureOp::Delete,
            )
        })
        .collect();
    assert!(!on_bina.is_empty());
    let mut info = o.info.clone();
    info.layers = without(&info.layers, "bina");
    info.meta_version = "6".into();
    let incoming = sync.incoming(&page(vec![event(10, None, &on_bina, true)]));
    let taken = sync
        .take_remote(
            &mut o.document,
            incoming,
            Remote {
                records: vec![],
                info: Some(info),
                blocks: None,
            },
        )
        .unwrap();
    assert!(taken.kept.is_empty());
    assert!(o.document.layers().get("bina").is_none());
    assert_eq!(o.document.by_layer("bina").count(), 0);
    assert_eq!(sync.next(&o.document), None);
}

/// Through a conflict of the metadata, “take the server's”: the name goes
/// back to the server's, the layer holding an unsent object stays, and the
/// next command sends it back.
#[test]
fn taking_the_servers_metadata_keeps_a_layer_with_unsent_objects() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.rename_layer("parsel", "Parseller");
    let mut e = point(486700.0);
    e.base_mut().layer_id = "bina".into();
    o.document.add(e).unwrap();
    sync.observe(&o.document);
    let mut info = o.info.clone();
    info.layers = without(&info.layers, "bina");
    info.meta_version = "7".into();
    let incoming = sync.incoming(&page(vec![event(10, None, &[], true)]));
    let taken = sync
        .take_remote(
            &mut o.document,
            incoming,
            Remote {
                records: vec![],
                info: Some(info.clone()),
                blocks: None,
            },
        )
        .unwrap();
    assert_eq!(taken.conflicts, 1);
    let kept = sync
        .take_theirs(&mut o.document, Some(&info), None)
        .unwrap()
        .kept;
    assert_eq!(kept.len(), 1);
    assert_eq!(o.document.layers().get("parsel").unwrap().name, "Parsel");
    assert!(o.document.layers().get("bina").is_some());
    let env = sync.next(&o.document).unwrap();
    let layers = input(&env)
        .project
        .and_then(|p| p.layers)
        .expect("the tree goes");
    assert!(remote::is_layer(&layers, "bina"));
    assert_eq!(
        env.expected_versions.get(PROJECT_KEY).map(String::as_str),
        Some("7")
    );
}

// ── Objects waiting for their layer, layers given back (docs/adr/0081) ──

/// Removes the hidden “Çizim” layer here with its objects, unsent (in the
/// sample “Parsel” is the active layer and “Bina” is locked).
fn remove_cizim(o: &mut Opened) -> usize {
    o.document.remove_layer("cizim").unwrap()
}

/// A point on `layer`, as another editor made it.
fn on_layer(layer: &str, x: f64) -> Entity {
    let mut e = point(x);
    e.base_mut().layer_id = layer.into();
    e
}

/// Another editor's commit of `features`, fetched as `records`, taken in.
fn take_commit(
    sync: &mut ProjectSync,
    doc: &mut Document,
    seq: u64,
    features: &[(Uuid, FeatureOp)],
    records: Vec<FeatureRecord>,
) -> Taken {
    let incoming = sync.incoming(&page(vec![event(seq, None, features, false)]));
    sync.take_remote(
        doc,
        incoming,
        Remote {
            records,
            info: None,
            blocks: None,
        },
    )
    .unwrap()
}

/// The waiting objects whose layer the drawing has now, fetched as
/// `records` and taken in (the desktop's `fetch_arrived`).
fn take_arrived(sync: &mut ProjectSync, doc: &mut Document, records: Vec<FeatureRecord>) -> Taken {
    let incoming = Incoming {
        fetch: sync.arrived(doc),
        cursor: sync.cursor().to_owned(),
        ..Incoming::default()
    };
    sync.take_remote(
        doc,
        incoming,
        Remote {
            records,
            info: None,
            blocks: None,
        },
    )
    .unwrap()
}

/// The server's guard refusing a tree that drops a layer still holding
/// objects (docs/adr/0072): the metadata's conflict at `expected`, `actual`.
fn guard(expected: &str, actual: &str) -> ApiFailure {
    ApiFailure::new(
        409,
        "conflict",
        "“Çizim” katmanında hâlâ nesne var (başka biri eklemiş ya da taşımış olabilir); katman silinmedi. Sunucudaki hâli ile sizinkini karşılaştırın.",
    )
    .with_conflicts(vec![FeatureConflict {
        id: PROJECT_KEY.into(),
        reason: ConflictReason::Project,
        expected: Some(expected.into()),
        actual: Some(actual.into()),
        current: None,
    }])
}

/// Another editor's object on a layer removed here, the removal not sent
/// yet, waits for it, unsaid; an undo brings the layer and the object comes
/// at its version (the web's b3c022f).
#[test]
fn an_object_on_a_layer_removed_here_waits_and_comes_with_the_layer() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    remove_cizim(&mut o);
    sync.observe(&o.document);
    let theirs = Uuid::now_v7();
    let taken = take_commit(
        &mut sync,
        &mut o.document,
        10,
        &[(theirs, FeatureOp::Create)],
        vec![record(theirs, "5", on_layer("cizim", 486610.0))],
    );
    assert_eq!(taken, Taken::default(), "nothing taken, nothing said");
    assert!(o.document.slot_of(theirs).is_none());
    assert!(sync.is_waiting() && sync.arrived(&o.document).is_empty());
    o.document.undo().unwrap();
    assert_eq!(sync.arrived(&o.document), [theirs]);
    let taken = take_arrived(
        &mut sync,
        &mut o.document,
        vec![record(theirs, "5", on_layer("cizim", 486610.0))],
    );
    assert_eq!(taken.changed, 1);
    assert_eq!(x_of(&o.document, theirs), Some(486610.0));
    assert_eq!(sync.version_of(theirs), Some("5"));
    assert!(!sync.is_waiting());
    assert_eq!(sync.next(&o.document), None, "not this user's to send");
}

/// A copy another editor moved onto a layer this drawing lacks leaves until
/// the layer comes; one deleted meanwhile waits no more, and nothing is
/// left to say when the project is left.
#[test]
fn a_copy_moved_onto_a_missing_layer_leaves_until_the_layer_comes() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let a = o.document.add(on_layer("parsel", 486620.0)).unwrap();
    let b = o.document.add(on_layer("parsel", 486630.0)).unwrap();
    let (moved, gone) = (o.document.uid(a).unwrap(), o.document.uid(b).unwrap());
    let env = sync.next(&o.document).unwrap();
    sync.answered(&o.document, &committed(&env, 2));
    remove_cizim(&mut o);
    sync.observe(&o.document);
    let taken = take_commit(
        &mut sync,
        &mut o.document,
        10,
        &[(moved, FeatureOp::Update), (gone, FeatureOp::Update)],
        vec![
            record(moved, "3", on_layer("cizim", 486620.0)),
            record(gone, "3", on_layer("cizim", 486630.0)),
        ],
    );
    assert_eq!(taken.changed, 2, "the copies here leave");
    assert!(o.document.slot_of(moved).is_none() && o.document.slot_of(gone).is_none());
    take_commit(
        &mut sync,
        &mut o.document,
        11,
        &[(gone, FeatureOp::Delete)],
        vec![],
    );
    o.document.undo().unwrap();
    assert_eq!(sync.arrived(&o.document), [moved]);
    take_arrived(
        &mut sync,
        &mut o.document,
        vec![record(moved, "3", on_layer("cizim", 486620.0))],
    );
    let slot = o.document.slot_of(moved).unwrap();
    assert_eq!(o.document.get(slot).unwrap().base().layer_id, "cizim");
    assert!(o.document.slot_of(gone).is_none());
    assert!(sync.waiting_texts().is_empty());
    assert_eq!(sync.next(&o.document), None);
}

/// Objects still waiting when the project is left are said once per layer,
/// by the name the server's tree gives it, with their count.
#[test]
fn objects_still_waiting_are_said_by_layer() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    remove_cizim(&mut o);
    sync.observe(&o.document);
    let (x, y) = (Uuid::now_v7(), Uuid::now_v7());
    take_commit(
        &mut sync,
        &mut o.document,
        10,
        &[(x, FeatureOp::Create), (y, FeatureOp::Create)],
        vec![
            record(x, "5", on_layer("cizim", 1.0)),
            record(y, "5", on_layer("cizim", 2.0)),
        ],
    );
    assert_eq!(
        sync.waiting_texts(),
        [
            "“Çizim” katmanı bu çizimde olmadığı için başka birinin 2 nesnesi burada gösterilmedi; proje yeniden açılınca görünür."
        ]
    );
}

/// The server's guard refused our tree without “Çizim”: someone drew on it
/// before the removal went (the web's 36d87de). Once the missed events are
/// in, the layer comes back from the server's tree, said once; the guard
/// alone refused, so its conflict ends, our deletions go and theirs comes.
#[test]
fn the_guard_refusing_our_removal_gives_the_layer_back_and_our_deletions_go() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let tree = o.document.layers().nodes().to_vec();
    let removed = remove_cizim(&mut o);
    let env = sync.next(&o.document).unwrap();
    assert!(input(&env).project.is_some_and(|p| p.layers.is_some()));
    assert_eq!(sync.failed(&guard("4", "4")), After::Stop);
    assert!(sync.may_give_back(&o.document));
    let theirs = Uuid::now_v7();
    take_commit(
        &mut sync,
        &mut o.document,
        10,
        &[(theirs, FeatureOp::Create)],
        vec![record(theirs, "5", on_layer("cizim", 486610.0))],
    );
    let given = sync.give_back(&mut o.document, &o.info).unwrap();
    assert_eq!(
        given,
        GivenBack {
            names: vec!["Çizim".into()],
            guard_only: true,
        }
    );
    assert_eq!(
        given_back_text(&given.names[0]),
        "“Çizim” katmanında başkasının nesnesi olduğu için katman silinmedi; sizin nesneleriniz silindi."
    );
    assert!(sync.conflicts().is_empty());
    assert_eq!(sync.state(), SaveState::Pending);
    assert_eq!(
        o.document.layers().nodes(),
        tree.as_slice(),
        "the server's tree again"
    );
    // Our deletions go, with no tree left to send.
    let env = sync.next(&o.document).unwrap();
    let c = input(&env);
    assert!(c.project.is_none());
    assert_eq!(c.features.len(), removed);
    assert!(
        c.features
            .iter()
            .all(|f| matches!(f, FeatureChange::Delete { .. }))
    );
    // And theirs comes with the layer.
    take_arrived(
        &mut sync,
        &mut o.document,
        vec![record(theirs, "5", on_layer("cizim", 486610.0))],
    );
    assert_eq!(x_of(&o.document, theirs), Some(486610.0));
    assert!(!sync.is_waiting());
}

/// Given back, the rest of our tree stays ours: a rename sent with the
/// removal goes again, with the layer.
#[test]
fn a_layer_given_back_leaves_the_rest_of_our_tree_ours() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.rename_layer("parsel", "Parseller");
    remove_cizim(&mut o);
    sync.next(&o.document).unwrap();
    sync.failed(&guard("4", "4"));
    let theirs = Uuid::now_v7();
    take_commit(
        &mut sync,
        &mut o.document,
        10,
        &[(theirs, FeatureOp::Create)],
        vec![record(theirs, "5", on_layer("cizim", 1.0))],
    );
    assert!(sync.give_back(&mut o.document, &o.info).unwrap().guard_only);
    let env = sync.next(&o.document).unwrap();
    let layers = input(&env)
        .project
        .and_then(|p| p.layers)
        .expect("the tree goes");
    assert!(remote::is_layer(&layers, "cizim"));
    let parsel = layers
        .iter()
        .flat_map(|n| n.children.iter())
        .find(|n| n.id == "parsel")
        .unwrap();
    assert_eq!(parsel.name, "Parseller");
    assert_eq!(
        env.expected_versions.get(PROJECT_KEY).map(String::as_str),
        Some("4")
    );
}

/// When someone else changed the metadata too, the conflict stays for the
/// user, the layer already given back; keeping mine goes over their version.
#[test]
fn with_their_metadata_changed_too_the_conflict_stays_with_the_layer_back() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let removed = remove_cizim(&mut o);
    sync.next(&o.document).unwrap();
    assert_eq!(sync.failed(&guard("4", "5")), After::Stop);
    let theirs = Uuid::now_v7();
    take_commit(
        &mut sync,
        &mut o.document,
        10,
        &[(theirs, FeatureOp::Create)],
        vec![record(theirs, "5", on_layer("cizim", 1.0))],
    );
    let mut info = o.info.clone();
    info.name = "Ada 102".into();
    info.meta_version = "5".into();
    let given = sync.give_back(&mut o.document, &info).unwrap();
    assert_eq!(
        given,
        GivenBack {
            names: vec!["Çizim".into()],
            guard_only: false,
        }
    );
    let open: Vec<&str> = sync.conflicts().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(open, [PROJECT_KEY]);
    assert_eq!(sync.state(), SaveState::Conflict);
    assert!(o.document.layers().get("cizim").is_some());
    sync.keep_mine(&o.document, None);
    let env = sync.next(&o.document).unwrap();
    let c = input(&env);
    assert!(c.project.is_none(), "our tree is the server's again");
    assert_eq!(c.features.len(), removed);
}

/// Without a refusal of the metadata nothing is given back, and a layer
/// that held only our objects goes as it is.
#[test]
fn without_a_refused_tree_nothing_is_given_back() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    remove_cizim(&mut o);
    sync.observe(&o.document);
    assert!(!sync.may_give_back(&o.document));
    assert_eq!(
        sync.give_back(&mut o.document, &o.info).unwrap(),
        GivenBack::default()
    );
    assert!(o.document.layers().get("cizim").is_none());
    // The deletions and the tree without the layer go together.
    let env = sync.next(&o.document).unwrap();
    let layers = input(&env)
        .project
        .and_then(|p| p.layers)
        .expect("the tree goes");
    assert!(!remote::is_layer(&layers, "cizim"));
    sync.answered(&o.document, &committed(&env, 2));
    assert!(sync.all_sent());
}

/// A device draft whose tree drops a layer someone drew on since it was
/// written: the layer stays, said once, and the draft's deletions go (the
/// web's `restoreDraft`, 36d87de).
#[test]
fn a_draft_dropping_a_layer_someone_drew_on_since_keeps_it() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    let removed = remove_cizim(&mut o);
    let draft = sync.draft(&o.document, "ayse").unwrap();
    assert!(
        draft
            .meta
            .as_ref()
            .is_some_and(|m| m.patch.layers.is_some())
    );
    // Before this device comes back, someone draws on the layer.
    let mut again = reopened(&o);
    let slot = again.document.add(on_layer("cizim", 486610.0)).unwrap();
    let theirs = again.document.uid(slot).unwrap();
    match &mut again.source {
        Source::Database { versions, .. } => versions.push((theirs, "5".into())),
        other => panic!("{other:?}"),
    }
    let mut sync = ProjectSync::new(&again).unwrap();
    let restored = sync.restore(&mut again.document, draft).unwrap();
    assert_eq!(restored.given_back, ["Çizim"]);
    assert!(again.document.layers().get("cizim").is_some());
    assert_eq!(x_of(&again.document, theirs), Some(486610.0));
    assert_eq!(
        again.document.by_layer("cizim").count(),
        1,
        "ours deleted, theirs kept"
    );
    let env = sync.next(&again.document).unwrap();
    let c = input(&env);
    assert!(c.project.is_none(), "the tree is the server's");
    assert_eq!(c.features.len(), removed);
    assert!(
        c.features
            .iter()
            .all(|f| matches!(f, FeatureChange::Delete { .. }))
    );
}
