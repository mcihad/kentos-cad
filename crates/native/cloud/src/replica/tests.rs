//! The local copy without a server: what it keeps, what a crash leaves, and
//! that it opens as the online opening did. The same against the real server
//! is in apps/api/src/http/native_tests.rs.

use super::*;
use kentos_contracts::{
    AccessSource, DocumentSnapshotV1, Entity, PointEntity, ProjectAccessView, ProjectPermission,
    ProjectRole, ProjectState, TenantKind,
};
use kentos_domain::Slot;

use crate::sync::{BaseMeta, BaseObject, ProjectSync};

const SAMPLE: &str = include_str!("../../../../../fixtures/document/v1/sample.json");

fn temp() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-replica-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn opened(storage: ProjectStorage) -> Opened {
    let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    let document = Document::from_snapshot(s.clone()).unwrap();
    let versions: Vec<(Uuid, String)> = document
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
                permissions: vec![
                    ProjectPermission::Read,
                    ProjectPermission::FeatureWrite,
                    ProjectPermission::Edit,
                ],
            },
            state: ProjectState::Active,
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
            storage,
        },
        document,
        source: match storage {
            ProjectStorage::Database => Source::Database { versions },
            ProjectStorage::File => Source::File {
                revision: Some(Revision {
                    number: 3,
                    sha256: "ab".repeat(32),
                }),
            },
        },
    }
}

fn point(x: f64) -> Entity {
    let mut e = DocumentSnapshotV1::from_json(SAMPLE).unwrap().entities[0].clone();
    if let Entity::Point(PointEntity { p, .. }) = &mut e {
        p.x = x;
    }
    e
}

/// Every object by persistent id, without its slot.
fn by_uid(doc: &Document) -> BTreeMap<Uuid, Entity> {
    doc.entities()
        .map(|e| {
            let mut e = e.clone();
            let uid = doc.uid(Slot(e.base().id)).unwrap();
            e.base_mut().id = 0;
            (uid, e)
        })
        .collect()
}

fn store() -> (PathBuf, ReplicaStore) {
    let dir = temp();
    (dir.clone(), ReplicaStore::new(dir))
}

fn open(store: &ReplicaStore, o: &Opened) -> Replica {
    store
        .open("http://127.0.0.1:8787", "ayse", o.tenant, o.project)
        .unwrap()
}

#[test]
fn a_project_opened_online_opens_again_offline_as_it_was() {
    let (dir, store) = store();
    let o = opened(ProjectStorage::Database);
    let mut replica = open(&store, &o);
    assert!(replica.load().unwrap().is_none());
    replica.reset(&o).unwrap();
    let back = replica.load().unwrap().unwrap();
    assert_eq!(by_uid(&back.document), by_uid(&o.document));
    // The same versions; in the server's id order, as an opening from the server has them.
    let sorted = |s: &Source| match s {
        Source::Database { versions } => {
            let mut v = versions.clone();
            v.sort();
            v
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(sorted(&back.source), sorted(&o.source));
    assert_eq!(
        (
            back.info.event_cursor.as_str(),
            back.info.meta_version.as_str()
        ),
        ("9", "4")
    );
    assert_eq!(back.document.project_id(), Some(o.project));
    // The offline catalog lists it.
    let kept = store.list("http://127.0.0.1:8787", "ayse");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].info.name, o.info.name);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn what_came_to_be_known_is_kept_step_by_step() {
    let (dir, store) = store();
    let o = opened(ProjectStorage::Database);
    let mut replica = open(&store, &o);
    replica.reset(&o).unwrap();
    let first = o.document.uid(Slot(1)).unwrap();
    let second = o.document.uid(Slot(2)).unwrap();
    let new = Uuid::now_v7();
    let mut layers = o.info.layers.clone();
    layers[0].name = "Yeni ad".into();
    replica
        .append(&BaseStep {
            cursor: Some("12".into()),
            put: vec![
                BaseObject {
                    id: first.to_string(),
                    version: "5".into(),
                    entity: point(486900.0),
                },
                BaseObject {
                    id: new.to_string(),
                    version: "5".into(),
                    entity: point(486950.0),
                },
            ],
            remove: vec![second.to_string()],
            meta: None,
        })
        .unwrap();
    replica
        .append(&BaseStep {
            meta: Some(BaseMeta {
                version: "6".into(),
                name: "Ada 5".into(),
                settings: o.info.settings.clone(),
                layers,
                styles: o.info.styles.clone(),
            }),
            ..BaseStep::default()
        })
        .unwrap();
    assert_eq!(replica.steps().unwrap(), 2);
    let back = replica.load().unwrap().unwrap();
    let Source::Database { versions } = &back.source else {
        panic!()
    };
    let version: BTreeMap<Uuid, String> = versions.iter().cloned().collect();
    assert_eq!(
        (version[&first].as_str(), version[&new].as_str()),
        ("5", "5")
    );
    assert!(!version.contains_key(&second));
    assert_eq!(back.document.len(), 13);
    assert!(back.document.slot_of(second).is_none());
    assert_eq!(
        (
            back.info.event_cursor.as_str(),
            back.info.meta_version.as_str(),
            back.info.name.as_str()
        ),
        ("12", "6", "Ada 5")
    );
    assert_eq!(back.document.name(), "Ada 5");
    assert_eq!(back.document.layers().nodes()[0].name, "Yeni ad");
    // Objects come in the server's order, as an opening numbers them.
    let ids: Vec<Uuid> = back
        .document
        .entities()
        .map(|e| back.document.uid(Slot(e.base().id)).unwrap())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_step_cut_short_by_a_crash_is_dropped_and_a_damaged_one_refuses_the_copy() {
    let (dir, store) = store();
    let o = opened(ProjectStorage::Database);
    let mut replica = open(&store, &o);
    replica.reset(&o).unwrap();
    replica
        .append(&BaseStep {
            cursor: Some("10".into()),
            ..BaseStep::default()
        })
        .unwrap();
    let log = replica.files(replica.generation().unwrap().unwrap())[2].clone();
    // A crash in the middle of the next line.
    let mut f = OpenOptions::new().append(true).open(&log).unwrap();
    f.write_all(br#"{"cursor":"1"#).unwrap();
    drop(f);
    let back = replica.load().unwrap().unwrap();
    assert_eq!(back.info.event_cursor, "10");
    // A damaged line that is not the last: the copy is not trusted.
    fs::write(&log, b"{\"cursor\":\"10\"}\n}bozuk{\n{\"cursor\":\"11\"}\n").unwrap();
    assert!(matches!(replica.load(), Err(ReplicaError::Unreadable(_))));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_compaction_rewrites_the_copy_from_the_syncs_base() {
    let (dir, store) = store();
    let mut o = opened(ProjectStorage::Database);
    let mut replica = open(&store, &o);
    replica.reset(&o).unwrap();
    let mut sync = ProjectSync::new(&o).unwrap();
    // A command answered: the server has the new point at version 2.
    let added = o.document.add(point(486600.0)).unwrap();
    let new = o.document.uid(added).unwrap();
    let env = sync.next(&o.document).unwrap();
    let mut versions = std::collections::BTreeMap::new();
    versions.insert(new.to_string(), "2".to_string());
    sync.answered(
        &o.document,
        &kentos_contracts::CommitResult {
            data_revision: "2".into(),
            meta_version: "4".into(),
            versions,
            deleted: vec![],
            event_seq: "10".into(),
            replayed: false,
        },
    );
    let _ = env;
    let step = sync.take_base_step().unwrap();
    assert_eq!(step.put.len(), 1);
    assert!(sync.take_base_step().is_none());
    replica.append(&step).unwrap();
    let appended = replica.load().unwrap().unwrap();
    replica.compact(&sync.base(&o.document)).unwrap();
    assert_eq!(replica.steps().unwrap(), 0);
    let compacted = replica.load().unwrap().unwrap();
    assert_eq!(by_uid(&compacted.document), by_uid(&appended.document));
    assert_eq!(compacted.source, appended.source);
    assert_eq!(by_uid(&compacted.document), by_uid(&o.document));
    // One generation's files are left.
    let names: Vec<String> = fs::read_dir(&replica.dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("taban-"))
        .collect();
    assert_eq!(names.len(), 3, "{names:?}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn one_program_at_a_time_holds_a_copy() {
    let (dir, store) = store();
    let o = opened(ProjectStorage::Database);
    let held = open(&store, &o);
    assert!(matches!(
        store.open("http://127.0.0.1:8787", "ayse", o.tenant, o.project),
        Err(ReplicaError::InUse)
    ));
    drop(held);
    assert!(
        store
            .open("http://127.0.0.1:8787", "ayse", o.tenant, o.project)
            .is_ok()
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_file_project_keeps_its_revision_and_a_save_made_offline() {
    let (dir, store) = store();
    let o = opened(ProjectStorage::File);
    let mut replica = open(&store, &o);
    replica.reset(&o).unwrap();
    let back = replica.load().unwrap().unwrap();
    assert_eq!(back.source, o.source);
    assert_eq!(by_uid(&back.document), by_uid(&o.document));
    assert_eq!(replica.kept_save().unwrap(), None);
    replica.keep_save(b"KCAD-baytlari", 3).unwrap();
    assert_eq!(
        replica.kept_save().unwrap(),
        Some((b"KCAD-baytlari".to_vec(), 3))
    );
    replica.clear_save().unwrap();
    replica.clear_save().unwrap();
    assert_eq!(replica.kept_save().unwrap(), None);
    let _ = fs::remove_dir_all(dir);
}
