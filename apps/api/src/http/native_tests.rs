//! The desktop's cloud client (`kentos-cloud`, docs/adr/0040) against this
//! server over a real TCP socket, with a throwaway database: signing in,
//! the catalog, opening both kinds of project, a file project's revisions
//! and their conflict, a drawing as a new project of either kind, and a
//! database project's edits sent with the web's tracker rules.

use std::collections::BTreeMap;

use kentos_application::admin;
use kentos_cloud::api::hex_sha256;
use kentos_cloud::follow;
use kentos_cloud::saving::envelope;
use kentos_cloud::{
    After, ApiFailure, CatalogQuery, Cloud, GivenBack, Incoming, Opened, ProjectSync, SaveState,
    Source, Uploaded, conflicting_revision, open, project_create, save_revision, upload_new,
};
use kentos_contracts::{
    AuthConfig, BlockDefinition, CatalogView, CommitResult, DocumentSnapshotV1, DocumentSnapshotV2, Entity,
    EntityId, FileUploadBegin, GrantRole, PointEntity, ProjectAccessChange, ProjectState,
    ProjectStorage, TenantRole,
};
use kentos_domain::{Document, Slot};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};
use uuid::Uuid;

use super::tests::app;

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");
const DRAWING: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/drawing.kcad");

/// The router on a port of this computer, and its address.
async fn serve(db: &TestDb) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = app(Some(db.app.clone()));
    tokio::spawn(async move { axum::serve(listener, router).await });
    format!("http://{addr}")
}

/// A workspace with Ayşe (project manager) and Dilek (editor), both local accounts.
async fn office(db: &TestDb) -> Uuid {
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    for (login, role) in [
        ("ayse", TenantRole::ProjectManager),
        ("dilek", TenantRole::Editor),
    ] {
        admin::create_local_user(&db.owner, login, login, None, "dogru-parola-1")
            .await
            .unwrap();
        admin::set_membership(&db.owner, "buro", login, role, true)
            .await
            .unwrap();
    }
    tenant
}

async fn signed_in(base: &str, login: &str) -> Cloud {
    let cloud = Cloud::new(base).unwrap();
    cloud.sign_in(login, "dogru-parola-1").await.unwrap();
    cloud
}

fn sample() -> Document {
    Document::from_snapshot(DocumentSnapshotV1::from_json(SAMPLE).unwrap()).unwrap()
}

fn kcad(doc: &Document) -> Vec<u8> {
    kentos_kcad::encode(&doc.to_snapshot_v2()).unwrap()
}

/// Every object by its persistent id, without its slot.
fn by_uid(doc: &DocumentSnapshotV2) -> BTreeMap<EntityId, Value> {
    doc.uids
        .iter()
        .zip(&doc.entities)
        .map(|(uid, e)| {
            let mut v = serde_json::to_value(e).unwrap();
            v.as_object_mut().unwrap().remove("id");
            (*uid, v)
        })
        .collect()
}

fn point(x: f64) -> Entity {
    let mut e = DocumentSnapshotV1::from_json(SAMPLE).unwrap().entities[0].clone();
    if let Entity::Point(PointEntity { p, .. }) = &mut e {
        p.x = x;
    }
    e
}

/// The slot of the drawing's first object of this kind (a server's drawing is in id order).
fn slot_of(doc: &Document, kind: &str) -> Slot {
    doc.entities()
        .find(|e| e.kind() == kind)
        .map(|e| Slot(e.base().id))
        .unwrap()
}

/// Sends everything the autosave has, as the desktop's timer would.
async fn send_all(cloud: &Cloud, sync: &mut ProjectSync, doc: &Document) -> Result<(), ApiFailure> {
    while let Some(command) = sync.next(doc) {
        match cloud.command::<CommitResult>(command).await {
            Ok(result) => sync.answered(doc, &result),
            Err(f) => {
                assert_eq!(sync.failed(&f), After::Stop, "{f:?}");
                return Err(f);
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn the_desktop_signs_in_with_its_own_header_and_forgets_the_session() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    office(&db).await;
    let base = serve(&db).await;
    let cloud = Cloud::new(&base).unwrap();
    assert_eq!(
        cloud.auth_config().await.unwrap(),
        AuthConfig {
            local: true,
            oidc: None,
            public_url: Some("http://app.test/".into()),
        }
    );
    let wrong = cloud.sign_in("ayse", "yanlis-parola").await.unwrap_err();
    assert_eq!(
        (wrong.status, wrong.code.as_str()),
        (401, "unauthenticated")
    );
    assert!(!cloud.signed_in());
    let me = cloud.sign_in("ayse", "dogru-parola-1").await.unwrap();
    assert!(cloud.signed_in());
    assert!(me.memberships.iter().any(|m| m.tenant_slug == "buro"));
    assert_eq!(cloud.me().await.unwrap().user.id, me.user.id);
    // A command with the session goes because the desktop names itself; a
    // browser page of another site cannot add that header.
    let tenant = Uuid::parse_str(
        &me.memberships
            .iter()
            .find(|m| m.tenant_slug == "buro")
            .unwrap()
            .tenant_id,
    )
    .unwrap();
    let made = cloud
        .create_project(
            tenant,
            project_create(&sample(), "Ada 1", ProjectStorage::Database),
            Uuid::new_v4(),
        )
        .await
        .unwrap();
    assert_eq!(made.storage, ProjectStorage::Database);
    // Signing out forgets the session here and ends it on the server.
    cloud.sign_out().await.unwrap();
    assert!(!cloud.signed_in());
    let out = cloud.me().await.unwrap_err();
    assert!(out.signed_out(), "{out:?}");
    db.close().await;
}

#[tokio::test]
async fn a_file_project_saves_revisions_and_a_stale_save_writes_nothing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let drawing = sample();

    // A drawing becomes a new file project, its first revision.
    let key = Uuid::new_v4();
    let create = project_create(&drawing, "Ada 101 dosyası", ProjectStorage::File);
    let (info, uploaded) = upload_new(&ayse, tenant, create.clone(), kcad(&drawing), key)
        .await
        .unwrap();
    assert_eq!(info.storage, ProjectStorage::File);
    match &uploaded {
        Uploaded::File(c) => assert_eq!(
            (c.revision.as_str(), c.objects.as_deref()),
            ("1", Some("13"))
        ),
        other => panic!("{other:?}"),
    }
    let project = Uuid::parse_str(&info.id).unwrap();
    // The same upload again (its answer lost, say): the same project, not filled twice.
    let (again, replayed) = upload_new(&ayse, tenant, create, kcad(&drawing), key)
        .await
        .unwrap();
    assert_eq!(again.id, info.id);
    assert!(matches!(replayed, Uploaded::File(c) if c.replayed && c.revision == "1"));

    // It opens from that revision, object by object the drawing.
    let mut opened = open(&ayse, tenant, project, None).await.unwrap();
    let Source::File { revision: Some(r) } = &opened.source else {
        panic!("{:?}", opened.source)
    };
    assert_eq!(r.number, 1);
    assert_eq!(
        by_uid(&opened.document.to_snapshot_v2()),
        by_uid(&drawing.to_snapshot_v2())
    );
    assert!(opened.can_write());

    // Saved on top of revision 1: revision 2.
    opened.document.add(point(486600.0)).unwrap();
    let saved = save_revision(&ayse, tenant, project, kcad(&opened.document), 1)
        .await
        .unwrap();
    assert_eq!(
        (saved.revision.as_str(), saved.objects.as_deref()),
        ("2", Some("14"))
    );

    // Dilek, who opened revision 1 too, saves over it: a conflict, nothing written.
    ayse.command::<ProjectAccessChange>(envelope(
        tenant,
        project,
        "project.share",
        1,
        Uuid::new_v4(),
        BTreeMap::new(),
        json!({ "userId": admin::user_id(&db.owner, "dilek").await.unwrap().to_string(), "role": GrantRole::Editor }),
    ))
    .await
    .unwrap();
    let dilek = signed_in(&base, "dilek").await;
    let stale = save_revision(&dilek, tenant, project, kcad(&drawing), 1)
        .await
        .unwrap_err();
    assert!(stale.conflict(), "{stale:?}");
    assert_eq!(conflicting_revision(&stale), Some(2));
    assert_eq!(
        ayse.file_revisions(tenant, project)
            .await
            .unwrap()
            .revisions
            .len(),
        2
    );
    // Reopened, it is revision 2 with the new object.
    let reopened = open(&dilek, tenant, project, None).await.unwrap();
    assert_eq!(reopened.document.len(), 14);
    assert_eq!(
        reopened.source,
        Source::File {
            revision: Some(kentos_cloud::Revision {
                number: 2,
                sha256: saved.sha256
            })
        }
    );
    db.close().await;
}

#[tokio::test]
async fn an_upload_says_whether_its_bytes_arrived() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let made = ayse
        .create_project(
            tenant,
            project_create(&sample(), "Ada 2", ProjectStorage::File),
            Uuid::new_v4(),
        )
        .await
        .unwrap();
    let project = Uuid::parse_str(&made.id).unwrap();
    let bytes = kcad(&sample());
    let begun = ayse
        .begin_upload(
            tenant,
            project,
            FileUploadBegin {
                size: bytes.len() as u32,
                sha256: hex_sha256(&bytes),
            },
        )
        .await
        .unwrap();
    let upload = Uuid::parse_str(&begun.id).unwrap();
    assert!(
        !ayse
            .upload_status(tenant, project, upload)
            .await
            .unwrap()
            .received
    );
    let sent = ayse
        .send_upload(tenant, project, upload, bytes.clone().into())
        .await
        .unwrap();
    assert!(sent.received);
    let status = ayse.upload_status(tenant, project, upload).await.unwrap();
    assert_eq!(
        (
            status.received,
            status.objects.as_deref(),
            status.sha256.as_str()
        ),
        (true, Some("13"), hex_sha256(&bytes).as_str())
    );
    // Received bytes are not taken twice: why the client asks first.
    let twice = ayse
        .send_upload(tenant, project, upload, bytes.into())
        .await
        .unwrap_err();
    assert_eq!(twice.code, "invalid");
    // Someone else's upload is not there for them.
    ayse.command::<ProjectAccessChange>(envelope(
        tenant,
        project,
        "project.share",
        1,
        Uuid::new_v4(),
        BTreeMap::new(),
        json!({ "userId": admin::user_id(&db.owner, "dilek").await.unwrap().to_string(), "role": GrantRole::Editor }),
    ))
    .await
    .unwrap();
    let dilek = signed_in(&base, "dilek").await;
    assert!(
        dilek
            .upload_status(tenant, project, upload)
            .await
            .unwrap_err()
            .not_found()
    );
    db.close().await;
}

#[tokio::test]
async fn a_drawing_becomes_a_database_project_and_edits_go_back_object_by_object() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let drawing = sample();
    let (info, uploaded) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 102", ProjectStorage::Database),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    assert!(matches!(&uploaded, Uploaded::Database(i) if i.objects == "13"));
    let project = Uuid::parse_str(&info.id).unwrap();
    // Listed in “Projelerim” as a database project.
    let mine = ayse
        .catalog(&CatalogQuery::new(CatalogView::Mine))
        .await
        .unwrap();
    let listed = mine.projects.iter().find(|p| p.id == info.id).unwrap();
    assert_eq!(listed.storage, ProjectStorage::Database);

    // Opened object by object: the drawing, every object at version 1.
    let mut opened: Opened = open(&ayse, tenant, project, None).await.unwrap();
    assert_eq!(
        by_uid(&opened.document.to_snapshot_v2()),
        by_uid(&drawing.to_snapshot_v2())
    );
    let Source::Database { versions, .. } = &opened.source else {
        panic!("{:?}", opened.source)
    };
    assert!(versions.len() == 13 && versions.iter().all(|(_, v)| v == "1"));

    // A change, a removal, a new object and a layer renamed: one command.
    let mut sync = ProjectSync::new(&opened).unwrap();
    let doc = &mut opened.document;
    let (the_point, the_line) = (slot_of(doc, "point"), slot_of(doc, "line"));
    assert!(doc.update(the_point, point(486700.0)));
    doc.remove(&[the_line]);
    let added = doc.add(point(486600.0)).unwrap();
    doc.rename_layer("cizim", "Çizim (düzenlendi)");
    sync.observe(doc);
    assert_eq!(sync.pending(), 4);
    send_all(&ayse, &mut sync, doc).await.unwrap();
    assert!(sync.all_sent());
    assert_eq!(sync.state(), SaveState::Saved);
    assert_eq!(sync.version_of(doc.uid(added).unwrap()), Some("2"));

    // Opened again, the server has exactly this drawing.
    let back = open(&ayse, tenant, project, None).await.unwrap();
    assert_eq!(
        by_uid(&back.document.to_snapshot_v2()),
        by_uid(&doc.to_snapshot_v2())
    );
    assert_eq!(back.document.layers().nodes(), doc.layers().nodes());

    // An object on a locked layer is refused for good; the change still waits.
    let locked = doc
        .entities()
        .find(|e| e.base().layer_id == "bina")
        .map(|e| Slot(e.base().id))
        .unwrap();
    let mut moved = doc.get(locked).unwrap().clone();
    moved.base_mut().label = Some("taşındı".into());
    assert!(doc.update(locked, moved));
    let refused = send_all(&ayse, &mut sync, doc).await.unwrap_err();
    assert!(refused.message.contains("kilitli"), "{}", refused.message);
    assert_eq!((sync.state(), sync.pending()), (SaveState::Error, 1));
    doc.undo();
    assert_eq!(sync.next(doc), None);
    assert!(sync.all_sent());
    db.close().await;
}

#[tokio::test]
async fn two_editors_of_one_object_meet_in_a_conflict() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let drawing = sample();
    let (info, _) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 103", ProjectStorage::Database),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&info.id).unwrap();
    ayse.command::<ProjectAccessChange>(envelope(
        tenant,
        project,
        "project.share",
        1,
        Uuid::new_v4(),
        BTreeMap::new(),
        json!({ "userId": admin::user_id(&db.owner, "dilek").await.unwrap().to_string(), "role": GrantRole::Editor }),
    ))
    .await
    .unwrap();
    let dilek = signed_in(&base, "dilek").await;
    let mut a = open(&ayse, tenant, project, None).await.unwrap();
    let mut d = open(&dilek, tenant, project, None).await.unwrap();
    assert_eq!(d.info.state, ProjectState::Active);
    let mut sa = ProjectSync::new(&a).unwrap();
    let mut sd = ProjectSync::new(&d).unwrap();
    let slot = slot_of(&a.document, "point");
    let uid = a.document.uid(slot).unwrap();
    // Both drawings came in the server's order: the same object in the same slot.
    assert_eq!(d.document.uid(slot), Some(uid));

    assert!(a.document.update(slot, point(486700.0)));
    send_all(&ayse, &mut sa, &a.document).await.unwrap();
    assert!(d.document.update(slot, point(486800.0)));
    let clash = send_all(&dilek, &mut sd, &d.document).await.unwrap_err();
    assert!(clash.conflict());
    assert_eq!(sd.state(), SaveState::Conflict);
    let c = &sd.conflicts()[0];
    assert_eq!(
        (c.id.clone(), c.actual.as_deref()),
        (uid.to_string(), Some("2"))
    );
    // Dilek keeps hers: it goes over Ayşe's version, and the server has it.
    sd.keep_mine(&d.document, None);
    send_all(&dilek, &mut sd, &d.document).await.unwrap();
    let back = open(&ayse, tenant, project, None).await.unwrap();
    let slot = back.document.slot_of(uid).unwrap();
    assert!(matches!(back.document.get(slot), Some(Entity::Point(p)) if p.p.x == 486800.0));
    db.close().await;
}

#[tokio::test]
async fn a_drawing_the_server_does_not_keep_leaves_no_project() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let extreme = Document::from_snapshot_v2(kentos_kcad::decode(DRAWING).unwrap()).unwrap();
    let refused = upload_new(
        &ayse,
        tenant,
        project_create(&extreme, "Uç değerler", ProjectStorage::Database),
        DRAWING.to_vec(),
        Uuid::new_v4(),
    )
    .await
    .unwrap_err();
    assert_eq!(
        (refused.code.as_str(), refused.path.as_deref()),
        ("invalid", Some("entities[13]"))
    );
    // The empty project went to the trash.
    let mine = ayse
        .catalog(&CatalogQuery::new(CatalogView::Mine))
        .await
        .unwrap();
    assert!(mine.projects.iter().all(|p| p.name != "Uç değerler"));
    let trash = ayse
        .catalog(&CatalogQuery::new(CatalogView::Trash))
        .await
        .unwrap();
    assert!(trash.projects.iter().any(|p| p.name == "Uç değerler"));
    db.close().await;
}

#[tokio::test]
async fn other_editors_changes_come_in_by_following_the_events() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let drawing = sample();
    let (info, _) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 104", ProjectStorage::Database),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&info.id).unwrap();
    ayse.command::<ProjectAccessChange>(envelope(
        tenant,
        project,
        "project.share",
        1,
        Uuid::new_v4(),
        BTreeMap::new(),
        json!({ "userId": admin::user_id(&db.owner, "dilek").await.unwrap().to_string(), "role": GrantRole::Editor }),
    ))
    .await
    .unwrap();
    let dilek = signed_in(&base, "dilek").await;
    let mut a = open(&ayse, tenant, project, None).await.unwrap();
    let mut d = open(&dilek, tenant, project, None).await.unwrap();
    let mut sa = ProjectSync::new(&a).unwrap();
    let mut sd = ProjectSync::new(&d).unwrap();

    // Ayşe moves the point, removes the line, adds a point and renames a layer.
    let (the_point, the_line) = (slot_of(&a.document, "point"), slot_of(&a.document, "line"));
    assert!(a.document.update(the_point, point(486700.0)));
    a.document.remove(&[the_line]);
    a.document.add(point(486600.0)).unwrap();
    a.document.rename_layer("cizim", "Çizim (Ayşe)");
    send_all(&ayse, &mut sa, &a.document).await.unwrap();

    // Dilek follows the events: her drawing becomes Ayşe's, and nothing goes back.
    let page = follow::events(&dilek, tenant, project, sd.cursor())
        .await
        .unwrap();
    let incoming = sd.incoming(&page);
    assert!(incoming.meta && incoming.needs_fetch());
    let remote = follow::fetch(&dilek, tenant, project, &incoming)
        .await
        .unwrap();
    let taken = sd.take_remote(&mut d.document, incoming, remote).unwrap();
    assert_eq!((taken.changed, taken.conflicts), (3, 0));
    assert!(!sd.is_waiting(), "every object's layer is in the drawing");
    assert_eq!(
        by_uid(&d.document.to_snapshot_v2()),
        by_uid(&a.document.to_snapshot_v2())
    );
    assert_eq!(
        d.document.layers().get("cizim").unwrap().name,
        "Çizim (Ayşe)"
    );
    assert!(!d.document.is_dirty());
    assert_eq!(sd.next(&d.document), None);

    // Dilek's own commit comes back in the events and is skipped; Ayşe takes it in.
    let added = d.document.add(point(486650.0)).unwrap();
    send_all(&dilek, &mut sd, &d.document).await.unwrap();
    let mine = follow::events(&dilek, tenant, project, sd.cursor())
        .await
        .unwrap();
    assert!(!sd.incoming(&mine).needs_fetch());
    let page = follow::events(&ayse, tenant, project, sa.cursor())
        .await
        .unwrap();
    let incoming = sa.incoming(&page);
    let remote = follow::fetch(&ayse, tenant, project, &incoming)
        .await
        .unwrap();
    sa.take_remote(&mut a.document, incoming, remote).unwrap();
    let uid = d.document.uid(added).unwrap();
    assert!(a.document.slot_of(uid).is_some());
    assert_eq!(
        by_uid(&a.document.to_snapshot_v2()),
        by_uid(&d.document.to_snapshot_v2())
    );

    // A cursor the log cannot continue from asks for the project to be opened again.
    let far = follow::events(&ayse, tenant, project, "999999")
        .await
        .unwrap_err();
    assert!(far.resync(), "{far:?}");
    db.close().await;
}

/// Takes in the events after the sync's cursor, as the desktop's following does.
async fn follow_up(cloud: &Cloud, tenant: Uuid, project: Uuid, sync: &mut ProjectSync, doc: &mut Document) -> kentos_cloud::Taken {
    let page = follow::events(cloud, tenant, project, sync.cursor())
        .await
        .unwrap();
    let incoming = sync.incoming(&page);
    let remote = follow::fetch(cloud, tenant, project, &incoming)
        .await
        .unwrap();
    sync.take_remote(doc, incoming, remote).unwrap()
}

/// Block definitions between two desktops through the server (docs/adr/0144
/// §5): Ayşe's definition goes with its insert in one command and comes to
/// Dilek from the events; Dilek places it too, and Ayşe, not having heard,
/// removes it with her own insert: the server keeps it for Dilek's insert,
/// and once Ayşe's missed events are in it is back without a question, her
/// insert's removal gone. A rename on both sides is a conflict; Ayşe keeps
/// hers over Dilek's, read from the server. Opened again, the drawing has
/// the definition at its version.
#[tokio::test]
async fn block_definitions_go_between_two_desktops_through_the_server() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let drawing = sample();
    let (info, _) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 107", ProjectStorage::Database),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&info.id).unwrap();
    ayse.command::<ProjectAccessChange>(envelope(
        tenant,
        project,
        "project.share",
        1,
        Uuid::new_v4(),
        BTreeMap::new(),
        json!({ "userId": admin::user_id(&db.owner, "dilek").await.unwrap().to_string(), "role": GrantRole::Editor }),
    ))
    .await
    .unwrap();
    let dilek = signed_in(&base, "dilek").await;
    let mut a = open(&ayse, tenant, project, None).await.unwrap();
    let mut d = open(&dilek, tenant, project, None).await.unwrap();
    let mut sa = ProjectSync::new(&a).unwrap();
    let mut sd = ProjectSync::new(&d).unwrap();

    // Ayşe's definition and her insert of it, in one command.
    let kapi: BlockDefinition = serde_json::from_value(json!({
        "id": Uuid::now_v7().to_string(), "name": "Kapı", "base": { "x": 0, "y": 0 },
        "entities": [{ "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": { "x": -1, "y": 0 }, "b": { "x": 1, "y": 0 } }]
    }))
    .unwrap();
    let insert = |x: f64| -> Entity {
        serde_json::from_value(json!({ "kind": "insert", "id": 0, "layerId": "cizim", "attrs": {},
            "block": kapi.id.to_string(), "p": { "x": x, "y": 4420210.0 }, "scale": 1, "rotation": 0 }))
        .unwrap()
    };
    a.document.add_block(kapi.clone()).unwrap();
    let hers = a.document.add(insert(486600.0)).unwrap();
    let hers = a.document.uid(hers).unwrap();
    sa.observe(&a.document);
    send_all(&ayse, &mut sa, &a.document).await.unwrap();
    assert!(sa.all_sent() && sa.block_version_of(kapi.id).is_some());

    // Dilek takes both in from the events.
    let taken = follow_up(&dilek, tenant, project, &mut sd, &mut d.document).await;
    assert_eq!(taken.conflicts, 0);
    assert_eq!(d.document.blocks().len(), 1);
    assert!(d.document.slot_of(hers).is_some());
    assert!(!d.document.is_dirty());

    // Dilek places it too; Ayşe, not having heard, removes her insert and the definition.
    let his = d.document.add(insert(486700.0)).unwrap();
    let his = d.document.uid(his).unwrap();
    sd.observe(&d.document);
    send_all(&dilek, &mut sd, &d.document).await.unwrap();
    let hers_slot = a.document.slot_of(hers).unwrap();
    a.document.remove(&[hers_slot]);
    assert!(a.document.remove_block(kapi.id).unwrap());
    let refused = send_all(&ayse, &mut sa, &a.document).await.unwrap_err();
    assert!(refused.conflict(), "{refused:?}");
    assert!(sa.may_give_back_blocks(), "the guard: the version is the one expected");
    // Her missed events bring Dilek's insert, and with it the definition, back.
    let taken = follow_up(&ayse, tenant, project, &mut sa, &mut a.document).await;
    assert!(a.document.slot_of(his).is_some());
    assert_eq!(a.document.blocks().len(), 1, "{:?}", taken.notes);
    let list = follow::blocks_now(&ayse, tenant, project).await.unwrap();
    let notes = sa.give_back_blocks(&mut a.document, &list).unwrap();
    assert!(notes.is_empty(), "nothing was missing any more: {notes:?}");
    assert!(sa.conflicts().is_empty());
    send_all(&ayse, &mut sa, &a.document).await.unwrap();
    assert!(sa.all_sent());
    let server = follow::blocks_now(&ayse, tenant, project).await.unwrap();
    assert_eq!(server.len(), 1, "the definition stays");

    // Renamed on both sides: a conflict; Ayşe keeps hers over Dilek's version.
    follow_up(&dilek, tenant, project, &mut sd, &mut d.document).await;
    assert!(d.document.slot_of(hers).is_none(), "Ayşe's insert is gone for Dilek too");
    d.document
        .update_block(BlockDefinition {
            name: "Kapı (Dilek)".into(),
            ..kapi.clone()
        })
        .unwrap();
    sd.observe(&d.document);
    send_all(&dilek, &mut sd, &d.document).await.unwrap();
    a.document
        .update_block(BlockDefinition {
            name: "Kapı (Ayşe)".into(),
            ..kapi.clone()
        })
        .unwrap();
    sa.observe(&a.document);
    let refused = send_all(&ayse, &mut sa, &a.document).await.unwrap_err();
    assert!(refused.conflict() && sa.conflicts_blocks() && !sa.may_give_back_blocks());
    let list = follow::blocks_now(&ayse, tenant, project).await.unwrap();
    sa.keep_mine(&a.document, Some(&list));
    send_all(&ayse, &mut sa, &a.document).await.unwrap();
    let taken = follow_up(&dilek, tenant, project, &mut sd, &mut d.document).await;
    assert_eq!(taken.conflicts, 0);
    assert_eq!(d.document.blocks()[0].name, "Kapı (Ayşe)");

    // Opened again from the server: the definition at its version, the drawings alike.
    let back = open(&ayse, tenant, project, None).await.unwrap();
    assert_eq!(back.document.blocks()[0].name, "Kapı (Ayşe)");
    let Source::Database { blocks, .. } = &back.source else {
        panic!("a database project");
    };
    assert_eq!(blocks.len(), 1);
    assert_eq!(Some(blocks[0].1.as_str()), sa.block_version_of(kapi.id));
    assert_eq!(
        by_uid(&back.document.to_snapshot_v2()),
        by_uid(&d.document.to_snapshot_v2())
    );
    db.close().await;
}

/// The server's guard refuses Ayşe's tree without “Çizim”: Dilek drew on
/// the layer before the removal went (docs/adr/0072, 0081). The refusal is
/// the metadata's conflict at the version Ayşe expected; once her missed
/// events are in, the layer comes back from the server's tree, the guard's
/// conflict ends, her deletions go, and Dilek's object comes to her too.
#[tokio::test]
async fn a_layer_someone_drew_on_is_given_back_when_its_removal_is_refused() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let drawing = sample();
    let (info, _) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 105", ProjectStorage::Database),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&info.id).unwrap();
    ayse.command::<ProjectAccessChange>(envelope(
        tenant,
        project,
        "project.share",
        1,
        Uuid::new_v4(),
        BTreeMap::new(),
        json!({ "userId": admin::user_id(&db.owner, "dilek").await.unwrap().to_string(), "role": GrantRole::Editor }),
    ))
    .await
    .unwrap();
    let dilek = signed_in(&base, "dilek").await;
    let mut a = open(&ayse, tenant, project, None).await.unwrap();
    let mut d = open(&dilek, tenant, project, None).await.unwrap();
    let mut sa = ProjectSync::new(&a).unwrap();
    let mut sd = ProjectSync::new(&d).unwrap();

    // Dilek draws on “Çizim”; Ayşe removes the layer before she hears of it.
    let drawn = d.document.add(point(486610.0)).unwrap();
    let theirs = d.document.uid(drawn).unwrap();
    send_all(&dilek, &mut sd, &d.document).await.unwrap();
    assert_eq!(a.document.remove_layer("cizim"), Ok(9));
    let refused = send_all(&ayse, &mut sa, &a.document).await.unwrap_err();
    assert!(refused.conflict(), "{refused:?}");
    assert!(sa.may_give_back(&a.document));

    // Her missed events: Dilek's object waits for its layer.
    let page = follow::events(&ayse, tenant, project, sa.cursor())
        .await
        .unwrap();
    let incoming = sa.incoming(&page);
    let remote = follow::fetch(&ayse, tenant, project, &incoming)
        .await
        .unwrap();
    sa.take_remote(&mut a.document, incoming, remote).unwrap();
    assert!(a.document.slot_of(theirs).is_none() && sa.is_waiting());

    // The server's tree: the layer comes back; the guard alone refused.
    let server = ayse.project(tenant, project).await.unwrap();
    let given = sa.give_back(&mut a.document, &server).unwrap();
    assert_eq!(
        given,
        GivenBack {
            names: vec!["Çizim".into()],
            guard_only: true,
        }
    );
    assert_eq!(sa.state(), SaveState::Pending);
    send_all(&ayse, &mut sa, &a.document).await.unwrap();
    assert!(sa.all_sent());
    let now = ayse.project(tenant, project).await.unwrap();
    assert!(now.layers.iter().any(|n| n.id == "cizim"), "the layer stays");

    // Dilek's object comes to Ayşe with its layer; Ayşe's deletions reach Dilek.
    let incoming = Incoming {
        fetch: sa.arrived(&a.document),
        cursor: sa.cursor().to_owned(),
        ..Incoming::default()
    };
    assert_eq!(incoming.fetch, [theirs]);
    let remote = follow::fetch(&ayse, tenant, project, &incoming)
        .await
        .unwrap();
    sa.take_remote(&mut a.document, incoming, remote).unwrap();
    assert!(a.document.slot_of(theirs).is_some() && !sa.is_waiting());
    let page = follow::events(&dilek, tenant, project, sd.cursor())
        .await
        .unwrap();
    let incoming = sd.incoming(&page);
    let remote = follow::fetch(&dilek, tenant, project, &incoming)
        .await
        .unwrap();
    sd.take_remote(&mut d.document, incoming, remote).unwrap();
    assert_eq!(d.document.by_layer("cizim").count(), 1, "only Dilek's is left");
    assert_eq!(
        by_uid(&a.document.to_snapshot_v2()),
        by_uid(&d.document.to_snapshot_v2())
    );
    db.close().await;
}

#[tokio::test]
async fn a_lost_answer_survives_the_program_ending_through_the_device_draft() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let me = ayse.me().await.unwrap().user.id;
    let drawing = sample();
    let (info, _) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 105", ProjectStorage::Database),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&info.id).unwrap();
    let mut a = open(&ayse, tenant, project, None).await.unwrap();
    let mut sync = ProjectSync::new(&a).unwrap();

    // A new point goes out and is committed, but its answer never arrives.
    let added = a.document.add(point(486600.0)).unwrap();
    let new_id = a.document.uid(added).unwrap();
    let lost = sync.next(&a.document).unwrap();
    ayse.command::<CommitResult>(lost.clone()).await.unwrap();
    // More work meanwhile, then the program ends; the draft was written as edits happened.
    let the_point = slot_of(&a.document, "point");
    assert!(a.document.update(the_point, point(486700.0)));
    let draft = sync.draft(&a.document, &me).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-native-drafts-{}", Uuid::now_v7()));
    let store = kentos_cloud::DraftStore::new(&dir);
    let key = store.key(ayse.server(), &me, tenant, project);
    store.save_later(key.clone(), draft).await.unwrap();

    // The program starts again: the project opens, and the draft goes back in.
    let mut b = open(&ayse, tenant, project, None).await.unwrap();
    let kentos_cloud::Loaded::Found(draft) = store.load(&key).unwrap() else {
        panic!("the draft was not found");
    };
    let mut sync = ProjectSync::new(&b).unwrap();
    let restored = sync.restore(&mut b.document, *draft).unwrap();
    assert!(restored.resends);
    assert_eq!(restored.conflicts, 0);
    // The same command, same key: the server answers from its log, and writes nothing twice.
    let again = sync.next(&b.document).unwrap();
    assert_eq!(again.idempotency_key, lost.idempotency_key);
    let answer = ayse.command::<CommitResult>(again).await.unwrap();
    assert!(answer.replayed);
    sync.answered(&b.document, &answer);
    send_all(&ayse, &mut sync, &b.document).await.unwrap();
    assert!(sync.all_sent());
    store.remove(&key).unwrap();

    // The server has the drawing of this device, the new point once.
    let back = open(&ayse, tenant, project, None).await.unwrap();
    assert_eq!(
        by_uid(&back.document.to_snapshot_v2()),
        by_uid(&b.document.to_snapshot_v2())
    );
    assert_eq!(back.document.len(), 14);
    assert!(back.document.slot_of(new_id).is_some());
    let _ = std::fs::remove_dir_all(dir);
    db.close().await;
}

#[tokio::test]
async fn a_project_goes_on_offline_and_catches_up_when_the_connection_returns() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let me = ayse.me().await.unwrap().user.id;
    let drawing = sample();
    let (info, _) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 106", ProjectStorage::Database),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&info.id).unwrap();
    ayse.command::<ProjectAccessChange>(envelope(
        tenant,
        project,
        "project.share",
        1,
        Uuid::new_v4(),
        BTreeMap::new(),
        json!({ "userId": admin::user_id(&db.owner, "dilek").await.unwrap().to_string(), "role": GrantRole::Editor }),
    ))
    .await
    .unwrap();
    let dilek = signed_in(&base, "dilek").await;
    let dir = std::env::temp_dir().join(format!("kentos-native-offline-{}", Uuid::now_v7()));
    let replicas = kentos_cloud::ReplicaStore::new(dir.join("kopya"));
    let drafts = kentos_cloud::DraftStore::new(dir.join("taslak"));
    let key = drafts.key(ayse.server(), &me, tenant, project);

    // 1. Online: the project opens, its copy is kept, an edit goes out.
    {
        let mut replica = replicas.open(ayse.server(), &me, tenant, project).unwrap();
        let mut a = open(&ayse, tenant, project, None).await.unwrap();
        replica.reset(&a).unwrap();
        let mut sync = ProjectSync::new(&a).unwrap();
        let the_point = slot_of(&a.document, "point");
        assert!(a.document.update(the_point, point(486700.0)));
        send_all(&ayse, &mut sync, &a.document).await.unwrap();
        replica.append(&sync.take_base_step().unwrap()).unwrap();
        assert!(sync.draft(&a.document, &me).is_none());
    }

    // Meanwhile Dilek, online, labels the line and adds a point.
    let mut d = open(&dilek, tenant, project, None).await.unwrap();
    let mut sd = ProjectSync::new(&d).unwrap();
    let the_line = slot_of(&d.document, "line");
    let mut line = d.document.get(the_line).unwrap().clone();
    line.base_mut().label = Some("Dilek".into());
    assert!(d.document.update(the_line, line));
    let dilek_point = d.document.add(point(486800.0)).unwrap();
    let dilek_uid = d.document.uid(dilek_point).unwrap();
    send_all(&dilek, &mut sd, &d.document).await.unwrap();

    // 2. No connection: the project opens from the copy; the work waits on this device.
    let unreachable = Cloud::new("http://127.0.0.1:9").unwrap();
    let offline_uid;
    let removed_uid;
    {
        let replica = replicas.open(ayse.server(), &me, tenant, project).unwrap();
        let mut a = replica.load().unwrap().unwrap();
        let the_point = slot_of(&a.document, "point");
        assert!(matches!(a.document.get(the_point), Some(Entity::Point(p)) if p.p.x == 486700.0));
        assert!(a.document.slot_of(dilek_uid).is_none());
        let mut sync = ProjectSync::new(&a).unwrap();
        let added = a.document.add(point(486650.0)).unwrap();
        offline_uid = a.document.uid(added).unwrap();
        let polyline = slot_of(&a.document, "polyline");
        removed_uid = a.document.uid(polyline).unwrap();
        a.document.remove(&[polyline]);
        let command = sync.next(&a.document).unwrap();
        // The draft reaches the disk before the command goes out.
        drafts
            .save(&key, &sync.draft(&a.document, &me).unwrap())
            .unwrap();
        let failure = unreachable
            .command::<CommitResult>(command)
            .await
            .unwrap_err();
        assert!(failure.transient(), "{failure:?}");
        assert!(matches!(sync.failed(&failure), After::Retry(_)));
        assert_eq!(sync.state(), SaveState::Offline);
        drafts
            .save(&key, &sync.draft(&a.document, &me).unwrap())
            .unwrap();
    }

    // 3. The connection is back: the copy opens, the draft goes back in, others' work comes in, this work goes out.
    {
        let mut replica = replicas.open(ayse.server(), &me, tenant, project).unwrap();
        let mut a = replica.load().unwrap().unwrap();
        let mut sync = ProjectSync::new(&a).unwrap();
        let kentos_cloud::Loaded::Found(draft) = drafts.load(&key).unwrap() else {
            panic!("the draft was not found");
        };
        let restored = sync.restore(&mut a.document, *draft).unwrap();
        assert!(restored.resends);
        assert_eq!(restored.conflicts, 0);
        loop {
            let page = follow::events(&ayse, tenant, project, sync.cursor())
                .await
                .unwrap();
            let full = page.events.len() >= follow::EVENTS_PAGE;
            let incoming = sync.incoming(&page);
            let remote = if incoming.needs_fetch() {
                follow::fetch(&ayse, tenant, project, &incoming)
                    .await
                    .unwrap()
            } else {
                kentos_cloud::Remote::default()
            };
            // Its own commit of the first session is known already: only Dilek's work is new, no conflict.
            let taken = sync.take_remote(&mut a.document, incoming, remote).unwrap();
            assert_eq!(taken.conflicts, 0, "{:?}", sync.conflicts());
            if let Some(step) = sync.take_base_step() {
                replica.append(&step).unwrap();
            }
            if !full {
                break;
            }
        }
        assert!(a.document.slot_of(dilek_uid).is_some());
        send_all(&ayse, &mut sync, &a.document).await.unwrap();
        if let Some(step) = sync.take_base_step() {
            replica.append(&step).unwrap();
        }
        assert!(sync.all_sent());
        assert!(sync.draft(&a.document, &me).is_none());
        drafts.remove(&key).unwrap();

        // The server has both editors' work, and so does the copy on this device.
        let server = open(&ayse, tenant, project, None).await.unwrap();
        let theirs = by_uid(&server.document.to_snapshot_v2());
        assert_eq!(by_uid(&a.document.to_snapshot_v2()), theirs);
        assert!(server.document.slot_of(offline_uid).is_some());
        assert!(server.document.slot_of(removed_uid).is_none());
        assert_eq!(
            by_uid(&replica.load().unwrap().unwrap().document.to_snapshot_v2()),
            theirs
        );
        replica.compact(&sync.base(&a.document)).unwrap();
        assert_eq!(replica.steps().unwrap(), 0);
        assert_eq!(
            by_uid(&replica.load().unwrap().unwrap().document.to_snapshot_v2()),
            theirs
        );
    }
    let _ = std::fs::remove_dir_all(dir);
    db.close().await;
}

#[tokio::test]
async fn a_long_offline_spell_past_the_kept_events_reopens_and_loses_nothing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let me = ayse.me().await.unwrap().user.id;
    let drawing = sample();
    let (info, _) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 107", ProjectStorage::Database),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&info.id).unwrap();
    ayse.command::<ProjectAccessChange>(envelope(
        tenant,
        project,
        "project.share",
        1,
        Uuid::new_v4(),
        BTreeMap::new(),
        json!({ "userId": admin::user_id(&db.owner, "dilek").await.unwrap().to_string(), "role": GrantRole::Editor }),
    ))
    .await
    .unwrap();
    let dilek = signed_in(&base, "dilek").await;
    let dir = std::env::temp_dir().join(format!("kentos-native-long-{}", Uuid::now_v7()));
    let replicas = kentos_cloud::ReplicaStore::new(dir.join("kopya"));
    let drafts = kentos_cloud::DraftStore::new(dir.join("taslak"));
    let key = drafts.key(ayse.server(), &me, tenant, project);

    // Ayşe opens the project once; the copy is kept.
    {
        let mut replica = replicas.open(ayse.server(), &me, tenant, project).unwrap();
        let a = open(&ayse, tenant, project, None).await.unwrap();
        replica.reset(&a).unwrap();
    }
    // While she is away, Dilek labels the line and adds a point, and weeks go by:
    // the server no longer keeps the events after Ayşe's cursor.
    let mut d = open(&dilek, tenant, project, None).await.unwrap();
    let mut sd = ProjectSync::new(&d).unwrap();
    let the_line = slot_of(&d.document, "line");
    let line_uid = d.document.uid(the_line).unwrap();
    let mut line = d.document.get(the_line).unwrap().clone();
    line.base_mut().label = Some("Dilek".into());
    assert!(d.document.update(the_line, line));
    let dilek_point = d.document.add(point(486800.0)).unwrap();
    let dilek_uid = d.document.uid(dilek_point).unwrap();
    send_all(&dilek, &mut sd, &d.document).await.unwrap();
    sqlx::query("update kentos.outbox_event set created_at = now() - interval '8 days' where project_id = $1")
        .bind(project)
        .execute(&db.owner)
        .await
        .unwrap();
    while kentos_application::events::prune(
        &db.app,
        std::time::Duration::from_secs(7 * 24 * 3600),
        100,
    )
    .await
    .unwrap()
        > 0
    {}

    // Offline, Ayşe edits the point and the same line; the work waits in the draft.
    let point_uid;
    {
        let replica = replicas.open(ayse.server(), &me, tenant, project).unwrap();
        let mut a = replica.load().unwrap().unwrap();
        let mut sync = ProjectSync::new(&a).unwrap();
        let the_point = slot_of(&a.document, "point");
        point_uid = a.document.uid(the_point).unwrap();
        assert!(a.document.update(the_point, point(486700.0)));
        let slot = a.document.slot_of(line_uid).unwrap();
        let mut line = a.document.get(slot).unwrap().clone();
        line.base_mut().label = Some("Ayşe".into());
        assert!(a.document.update(slot, line));
        drafts
            .save(&key, &sync.draft(&a.document, &me).unwrap())
            .unwrap();
    }

    // Back online: the copy's cursor is past what the server keeps.
    let mut replica = replicas.open(ayse.server(), &me, tenant, project).unwrap();
    let a = replica.load().unwrap().unwrap();
    let sync = ProjectSync::new(&a).unwrap();
    let gone = follow::events(&ayse, tenant, project, sync.cursor())
        .await
        .unwrap_err();
    assert!(gone.resync(), "{gone:?}");
    // So the project opens from the server, the copy starts again from it, and the draft goes on top.
    let mut a = open(&ayse, tenant, project, None).await.unwrap();
    replica.reset(&a).unwrap();
    let mut sync = ProjectSync::new(&a).unwrap();
    let kentos_cloud::Loaded::Found(draft) = drafts.load(&key).unwrap() else {
        panic!("the draft was not found");
    };
    let restored = sync.restore(&mut a.document, *draft).unwrap();
    // The line both changed is a conflict; the point is not; Dilek's new point is there.
    assert_eq!(restored.conflicts, 1);
    assert_eq!(sync.conflicts()[0].id, line_uid.to_string());
    assert!(a.document.slot_of(dilek_uid).is_some());
    let slot = a.document.slot_of(line_uid).unwrap();
    assert_eq!(
        a.document.get(slot).unwrap().base().label.as_deref(),
        Some("Ayşe")
    );
    // Ayşe keeps hers: everything goes out, nothing was lost.
    sync.keep_mine(&a.document, None);
    send_all(&ayse, &mut sync, &a.document).await.unwrap();
    replica.compact(&sync.base(&a.document)).unwrap();
    drafts.remove(&key).unwrap();
    let server = open(&ayse, tenant, project, None).await.unwrap();
    assert_eq!(
        by_uid(&server.document.to_snapshot_v2()),
        by_uid(&a.document.to_snapshot_v2())
    );
    let s = server.document.slot_of(line_uid).unwrap();
    assert_eq!(
        server.document.get(s).unwrap().base().label.as_deref(),
        Some("Ayşe")
    );
    assert!(
        matches!(server.document.get(server.document.slot_of(point_uid).unwrap()), Some(Entity::Point(p)) if p.p.x == 486700.0)
    );
    assert!(server.document.slot_of(dilek_uid).is_some());
    assert_eq!(
        by_uid(&replica.load().unwrap().unwrap().document.to_snapshot_v2()),
        by_uid(&server.document.to_snapshot_v2())
    );
    drop(replica);
    let _ = std::fs::remove_dir_all(dir);
    db.close().await;
}

#[tokio::test]
async fn a_file_project_saved_offline_goes_out_when_connected_and_a_clash_keeps_both() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let me = ayse.me().await.unwrap().user.id;
    let drawing = sample();
    let (info, _) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 108 dosyası", ProjectStorage::File),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&info.id).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-native-file-{}", Uuid::now_v7()));
    let replicas = kentos_cloud::ReplicaStore::new(&dir);
    let mut replica = replicas.open(ayse.server(), &me, tenant, project).unwrap();
    let a = open(&ayse, tenant, project, None).await.unwrap();
    replica.reset(&a).unwrap();

    // Offline: the copy opens at revision 1; a save waits on this device.
    let mut a = replica.load().unwrap().unwrap();
    let Source::File { revision: Some(r) } = &a.source else {
        panic!("{:?}", a.source)
    };
    assert_eq!(r.number, 1);
    a.document.add(point(486600.0)).unwrap();
    replica.keep_save(&kcad(&a.document), 1).unwrap();
    // Connected again: it goes out as revision 2 and no longer waits.
    let (bytes, based_on) = replica.kept_save().unwrap().unwrap();
    let saved = save_revision(&ayse, tenant, project, bytes, based_on)
        .await
        .unwrap();
    assert_eq!(saved.revision, "2");
    replica.clear_save().unwrap();
    assert_eq!(replica.kept_save().unwrap(), None);

    // Offline again, while Dilek saves revision 3 on the server.
    let a2 = open(&ayse, tenant, project, None).await.unwrap();
    replica.reset(&a2).unwrap();
    let mut mine = replica.load().unwrap().unwrap();
    mine.document.add(point(486650.0)).unwrap();
    replica.keep_save(&kcad(&mine.document), 2).unwrap();
    ayse.command::<ProjectAccessChange>(envelope(
        tenant,
        project,
        "project.share",
        1,
        Uuid::new_v4(),
        BTreeMap::new(),
        json!({ "userId": admin::user_id(&db.owner, "dilek").await.unwrap().to_string(), "role": GrantRole::Editor }),
    ))
    .await
    .unwrap();
    let dilek = signed_in(&base, "dilek").await;
    let mut theirs = open(&dilek, tenant, project, None).await.unwrap();
    theirs.document.add(point(486900.0)).unwrap();
    save_revision(&dilek, tenant, project, kcad(&theirs.document), 2)
        .await
        .unwrap();
    // Connected: the waiting save meets Dilek's revision. Both are kept: hers stays the
    // project's newest, this device's work becomes a separate project, nothing is dropped.
    let (bytes, based_on) = replica.kept_save().unwrap().unwrap();
    let clash = save_revision(&ayse, tenant, project, bytes.clone(), based_on)
        .await
        .unwrap_err();
    assert_eq!(conflicting_revision(&clash), Some(3));
    let (copy, _) = upload_new(
        &ayse,
        tenant,
        project_create(
            &mine.document,
            "Ada 108 dosyası (kopya)",
            ProjectStorage::File,
        ),
        bytes,
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    replica.clear_save().unwrap();
    let copy = open(&ayse, tenant, Uuid::parse_str(&copy.id).unwrap(), None)
        .await
        .unwrap();
    assert_eq!(
        by_uid(&copy.document.to_snapshot_v2()),
        by_uid(&mine.document.to_snapshot_v2())
    );
    let newest = open(&ayse, tenant, project, None).await.unwrap();
    assert_eq!(
        by_uid(&newest.document.to_snapshot_v2()),
        by_uid(&theirs.document.to_snapshot_v2())
    );
    drop(replica);
    let _ = std::fs::remove_dir_all(dir);
    db.close().await;
}

#[tokio::test]
async fn a_waiting_request_answers_as_soon_as_another_editor_commits() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let drawing = sample();
    let (info, _) = upload_new(
        &ayse,
        tenant,
        project_create(&drawing, "Ada 109", ProjectStorage::Database),
        kcad(&drawing),
        Uuid::new_v4(),
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&info.id).unwrap();
    let a = open(&ayse, tenant, project, None).await.unwrap();
    let cursor = a.info.event_cursor.clone();
    // Nothing new: a short wait ends empty, at its end.
    let started = std::time::Instant::now();
    let empty = ayse
        .events_waiting(tenant, project, &cursor, std::time::Duration::from_secs(1))
        .await
        .unwrap();
    assert!(empty.events.is_empty());
    assert!(started.elapsed() >= std::time::Duration::from_millis(900));
    // A long wait answers as soon as someone commits.
    let waiting = follow::wait(&ayse, tenant, project, &cursor);
    let commit = async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let mut d = open(&ayse, tenant, project, None).await.unwrap();
        let mut sync = ProjectSync::new(&d).unwrap();
        d.document.add(point(486600.0)).unwrap();
        send_all(&ayse, &mut sync, &d.document).await.unwrap();
    };
    let started = std::time::Instant::now();
    let (page, ()) = tokio::join!(waiting, commit);
    let page = page.unwrap();
    assert_eq!(page.events.len(), 1);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
    // With something new already, it answers at once.
    let started = std::time::Instant::now();
    let now = follow::wait(&ayse, tenant, project, &cursor).await.unwrap();
    assert_eq!(now.events.len(), 1);
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    db.close().await;
}

#[tokio::test]
async fn a_file_goes_in_parts_and_a_cut_goes_on_from_what_arrived() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let made = ayse
        .create_project(
            tenant,
            project_create(&sample(), "Ada 110 dosyası", ProjectStorage::File),
            Uuid::new_v4(),
        )
        .await
        .unwrap();
    let project = Uuid::parse_str(&made.id).unwrap();
    let bytes = kcad(&sample());
    assert!(
        bytes.len() > 3 * 1024,
        "the sample should need several parts"
    );

    // Saved in parts of 1 KiB: the progress climbs to the whole file, then revision 1.
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = seen.clone();
    let progress: kentos_cloud::Progress = std::sync::Arc::new(move |done, total| {
        log.lock().unwrap().push((done, total));
    });
    let saved = kentos_cloud::save_revision_watched(
        &ayse,
        tenant,
        project,
        bytes.clone(),
        0,
        1024,
        Some(progress),
    )
    .await
    .unwrap();
    assert_eq!(
        (saved.revision.as_str(), saved.objects.as_deref()),
        ("1", Some("13"))
    );
    let seen = seen.lock().unwrap().clone();
    assert!(seen.len() >= 3, "{seen:?}");
    assert!(seen.windows(2).all(|w| w[0].0 < w[1].0), "{seen:?}");
    assert_eq!(seen.last().unwrap().1, bytes.len() as u64);
    assert_eq!(
        open(&ayse, tenant, project, None)
            .await
            .unwrap()
            .document
            .len(),
        13
    );

    // A cut: the first part arrived; a part out of step is refused and says where to go on.
    let begun = ayse
        .begin_upload(
            tenant,
            project,
            FileUploadBegin {
                size: bytes.len() as u32,
                sha256: hex_sha256(&bytes),
            },
        )
        .await
        .unwrap();
    let upload = Uuid::parse_str(&begun.id).unwrap();
    let first = ayse
        .send_part(tenant, project, upload, 0, bytes[..1024].to_vec().into())
        .await
        .unwrap();
    assert_eq!(
        (first.received, first.received_bytes.as_deref()),
        (false, Some("1024"))
    );
    let stale = ayse
        .send_part(tenant, project, upload, 0, bytes[..1024].to_vec().into())
        .await
        .unwrap_err();
    assert_eq!(stale.path.as_deref(), Some("offset"));
    let status = ayse.upload_status(tenant, project, upload).await.unwrap();
    assert_eq!(status.received_bytes.as_deref(), Some("1024"));
    // Going on from there completes and verifies the file; it commits as revision 2.
    let rest = ayse
        .send_part(tenant, project, upload, 1024, bytes[1024..].to_vec().into())
        .await
        .unwrap();
    assert!(rest.received);
    let committed = ayse
        .command::<kentos_contracts::FileCommitted>(envelope(
            tenant,
            project,
            "project.file.commit",
            1,
            Uuid::new_v4(),
            [("@file".to_string(), "1".to_string())].into(),
            json!({ "uploadId": upload.to_string() }),
        ))
        .await
        .unwrap();
    assert_eq!(committed.revision, "2");

    // A file whose parts do not add up to the declared hash keeps nothing.
    let wrong = ayse
        .begin_upload(
            tenant,
            project,
            FileUploadBegin {
                size: bytes.len() as u32,
                sha256: "0".repeat(64),
            },
        )
        .await
        .unwrap();
    let wrong_id = Uuid::parse_str(&wrong.id).unwrap();
    ayse.send_part(tenant, project, wrong_id, 0, bytes[..1024].to_vec().into())
        .await
        .unwrap();
    let refused = ayse
        .send_part(
            tenant,
            project,
            wrong_id,
            1024,
            bytes[1024..].to_vec().into(),
        )
        .await
        .unwrap_err();
    assert_eq!(refused.path.as_deref(), Some("sha256"));
    let after = ayse.upload_status(tenant, project, wrong_id).await.unwrap();
    assert_eq!(
        (after.received, after.received_bytes.as_deref()),
        (false, Some("0"))
    );
    db.close().await;
}

/// How long a large database project takes to open on the desktop: page by
/// page (`open`, what the sync needs: every object's version) against the
/// one-file snapshot (`GET …/snapshot`). A measurement, not run by default:
/// `KENTOS_TEST_DB=required cargo test --release -p kentos-api --bin kentosd
/// native_tests::opening_a_large -- --ignored --nocapture` (`KENTOS_PERF_SIZES`).
#[tokio::test]
#[ignore = "a measurement, run by hand in release mode"]
async fn opening_a_large_project_measured() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = office(&db).await;
    let base = serve(&db).await;
    let ayse = signed_in(&base, "ayse").await;
    let sizes: Vec<usize> = std::env::var("KENTOS_PERF_SIZES")
        .ok()
        .map(|s| s.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![10_000, 100_000]);
    println!("| Nesne | İçe aktarım | Açılış (sayfalar) | Görüntü (tek dosya) | Görüntü boyu |");
    println!("|---:|---:|---:|---:|---:|");
    for n in sizes {
        let mut doc = sample();
        let entities: Vec<Entity> = (0..n)
            .map(|i| point(486000.0 + (i % 1000) as f64 * 2.0 + (i / 1000) as f64 * 0.001))
            .collect();
        doc.add_many(entities, "ölçüm").unwrap();
        let bytes = kcad(&doc);
        let t = std::time::Instant::now();
        let (info, _) = upload_new(
            &ayse,
            tenant,
            project_create(&doc, &format!("Ölçüm {n}"), ProjectStorage::Database),
            bytes,
            Uuid::new_v4(),
        )
        .await
        .unwrap();
        let import = t.elapsed();
        let project = Uuid::parse_str(&info.id).unwrap();
        let t = std::time::Instant::now();
        let opened = open(&ayse, tenant, project, None).await.unwrap();
        let paged = t.elapsed();
        assert_eq!(opened.document.len(), n + 13);
        let t = std::time::Instant::now();
        let snap = ayse.snapshot(tenant, project, None).await.unwrap();
        let snapshot = t.elapsed();
        println!(
            "| {} | {:.1} s | {:.1} s | {:.1} s | {:.1} MB |",
            n + 13,
            import.as_secs_f64(),
            paged.as_secs_f64(),
            snapshot.as_secs_f64(),
            snap.bytes.len() as f64 / 1e6
        );
    }
    db.close().await;
}
