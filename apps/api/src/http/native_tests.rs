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
    After, ApiFailure, CatalogQuery, Cloud, Opened, ProjectSync, SaveState, Source, Uploaded,
    conflicting_revision, open, project_create, save_revision, upload_new,
};
use kentos_contracts::{
    AuthConfig, CatalogView, CommitResult, DocumentSnapshotV1, DocumentSnapshotV2, Entity,
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
            oidc: None
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
    let Source::Database { versions } = &opened.source else {
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
    sd.keep_mine();
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
    assert_eq!(
        (taken.changed, taken.conflicts, taken.skipped.len()),
        (3, 0, 0)
    );
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
