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

// ── Other editors' commits (remote.rs) ─────────────────────────────────────

use kentos_contracts::{EventFeature, EventPage, EventRecord, FeatureOp};

fn event(
    seq: u64,
    request: Option<&str>,
    features: &[(Uuid, FeatureOp)],
    meta: bool,
) -> EventRecord {
    EventRecord {
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

fn page(events: Vec<EventRecord>) -> EventPage {
    let next = events.last().map_or("9".to_string(), |e| e.seq.clone());
    EventPage { events, next }
}

fn record(id: Uuid, version: &str, entity: Entity) -> FeatureRecord {
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
    sync.take_theirs(&mut o.document, None).unwrap();
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
    // Without the metadata the object's layer is not in the drawing: skipped and named.
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
            },
        )
        .unwrap();
    assert_eq!(taken.skipped.len(), 1);
    assert!(taken.skipped[0].contains("yeni"));
    assert!(o.document.slot_of(a).is_none());
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
            records: vec![record(b, "3", on_new)],
            info: Some(info),
        },
    )
    .unwrap();
    assert!(o.document.layers().get("yeni").is_some());
    assert!(o.document.slot_of(b).is_some());
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
            },
        )
        .unwrap();
    assert_eq!(taken.conflicts, 1);
    assert_eq!(sync.conflicts()[0].id, PROJECT_KEY);
    assert_eq!(o.document.layers().get("bina").unwrap().name, "Yapılar");
    // Mine: the renamed layer goes over the server's metadata version.
    sync.keep_mine();
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
        },
    )
    .unwrap();
    sync.take_theirs(&mut o.document, Some(&info)).unwrap();
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
fn reopened(o: &Opened) -> Opened {
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
        Source::Database { versions } => versions,
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
    sync.keep_mine();
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
