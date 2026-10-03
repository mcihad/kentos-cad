//! The sheet template library's routes end to end (docs/sheet/design.md §13),
//! with a throwaway database: the commands through the personal space's
//! command route, the list, a template with its `ETag`, its sharing and the
//! people to share with, the events and their long poll; a template others
//! may not see answers 404 word for word as a guessed or malformed id; and
//! the desktop's client (`kentos-cloud`) over a real socket.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use kentos_application::admin;
use kentos_contracts::{ApiError, Me, TenantKind, TenantRole};
use kentos_postgres::testing::TestDb;
use kentos_sheet::cloud::{
    SheetTemplateAccess, SheetTemplateCandidates, SheetTemplateChanged, SheetTemplateDetail,
    SheetTemplateEventKind, SheetTemplateEventPage, SheetTemplateList, TemplateGrantRole,
    TemplateRole,
};
use serde_json::{Value, json};
use uuid::Uuid;

use super::tests::{app, get, json_req, not_found_body, send, signed_in};

fn content(name: &str) -> Value {
    let t = kentos_sheet::template::system_template("sys:genel-a3-yatay").unwrap();
    let mut v = serde_json::to_value(t).unwrap();
    v["meta"]["id"] = json!("u:http");
    v["meta"]["name"] = json!(name);
    v
}

async fn personal(router: &axum::Router, cookie: &str) -> Uuid {
    let (status, _, body) = send(router, get("/v1/me", cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let me: Me = serde_json::from_slice(&body).unwrap();
    me.memberships
        .iter()
        .find(|m| m.tenant_kind == TenantKind::Personal)
        .unwrap()
        .tenant_id
        .parse()
        .unwrap()
}

fn command(tenant: Uuid, name: &str, key: &str, input: Value) -> Value {
    json!({ "commandName": name, "version": 1, "tenantId": tenant, "projectId": "", "requestId": format!("istek-{key}"),
            "idempotencyKey": format!("anahtar-{key}"), "expectedVersions": {}, "input": input })
}

/// Büro: Ayşe (project manager) and Bora (editor); Diğer: Can, whose name looks like Bora's.
async fn office(db: &TestDb) {
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 6)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 3)
        .await
        .unwrap();
    for (login, name, tenant, role) in [
        ("ayse", "Ayşe Yılmaz", "buro", TenantRole::ProjectManager),
        ("bora", "Bora Tan", "buro", TenantRole::Editor),
        ("can", "Bora Can", "diger", TenantRole::Owner),
    ] {
        admin::create_local_user(&db.owner, login, name, None, "dogru-parola-1")
            .await
            .unwrap();
        admin::set_membership(&db.owner, tenant, login, role, true)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn the_template_routes_answer_only_who_may_see() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    office(&db).await;
    let router = app(Some(db.app.clone()));
    let (ayse, bora, can) = (
        signed_in(&router, "ayse").await,
        signed_in(&router, "bora").await,
        signed_in(&router, "can").await,
    );
    let space = personal(&router, &ayse).await;
    let commands = format!("/v1/tenants/{space}/commands");
    let create = command(
        space,
        "sheet.template.create",
        "1",
        json!({ "content": content("Büro A3") }),
    );
    let (status, _, body) = send(&router, json_req("POST", &commands, &ayse, create)).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let made: SheetTemplateChanged = serde_json::from_slice(&body).unwrap();
    let uri = format!("/v1/sheet-templates/{}", made.template_id);
    // Someone else's space takes no command of hers.
    let bora_space = personal(&router, &bora).await;
    let theirs = command(
        bora_space,
        "sheet.template.create",
        "2",
        json!({ "content": content("X") }),
    );
    let (status, _, _) = send(
        &router,
        json_req(
            "POST",
            &format!("/v1/tenants/{bora_space}/commands"),
            &ayse,
            theirs,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The list and the template, with its revision as the entity tag.
    let (status, _, body) = send(&router, get("/v1/me/sheet-templates", &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    let list: SheetTemplateList = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        list.templates
            .iter()
            .map(|t| (t.name.as_str(), t.role))
            .collect::<Vec<_>>(),
        [("Büro A3", TemplateRole::Owner)]
    );
    let (status, headers, body) = send(&router, get(&uri, &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get(header::ETAG).unwrap(), "\"1\"");
    let d: SheetTemplateDetail = serde_json::from_slice(&body).unwrap();
    assert_eq!(d.content.meta.id, made.template_id);
    let again = Request::get(&uri)
        .header(header::COOKIE, &ayse)
        .header(header::IF_NONE_MATCH, "\"1\"")
        .body(Body::empty())
        .unwrap();
    let (status, _, body) = send(&router, again).await;
    assert_eq!((status, body.len()), (StatusCode::NOT_MODIFIED, 0));

    // Every 404 reads the same: a template not shared with Bora, another organisation's Can, a guessed and a malformed id.
    let reference = not_found_body(
        &router,
        get(&format!("/v1/sheet-templates/{}", Uuid::now_v7()), &ayse),
    )
    .await;
    for (path, cookie) in [
        (uri.clone(), &bora),
        (format!("{uri}/access"), &bora),
        (format!("{uri}/access/candidates?q=bora"), &can),
        (uri.clone(), &can),
        ("/v1/sheet-templates/bozuk".to_owned(), &ayse),
        (
            format!("/v1/sheet-templates/{}/access", Uuid::now_v7()),
            &ayse,
        ),
    ] {
        assert_eq!(
            not_found_body(&router, get(&path, cookie)).await,
            reference,
            "{path}"
        );
    }

    // Sharing with Bora (Büro); never with Can (another organisation), whom the search does not find either.
    let bora_id = admin::user_id(&db.owner, "bora").await.unwrap();
    let can_id = admin::user_id(&db.owner, "can").await.unwrap();
    let (status, _, body) = send(
        &router,
        get(&format!("{uri}/access/candidates?q=bora"), &ayse),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let found: SheetTemplateCandidates = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        found
            .candidates
            .iter()
            .map(|c| c.display_name.as_str())
            .collect::<Vec<_>>(),
        ["Bora Tan"]
    );
    let share = |user: Uuid, key: &str| {
        command(
            space,
            "sheet.template.share",
            key,
            json!({ "templateId": made.template_id, "userId": user, "role": "viewer" }),
        )
    };
    let (status, _, body) = send(
        &router,
        json_req("POST", &commands, &ayse, share(can_id, "3")),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let refused: ApiError = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        (refused.error.as_str(), refused.path.as_deref()),
        ("invalid", Some("userId"))
    );
    let (status, _, _) = send(
        &router,
        json_req("POST", &commands, &ayse, share(bora_id, "4")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _, body) = send(&router, get(&format!("{uri}/access"), &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    let access: SheetTemplateAccess = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        access
            .grants
            .iter()
            .map(|g| (g.display_name.as_str(), g.role))
            .collect::<Vec<_>>(),
        [("Bora Tan", TemplateGrantRole::Viewer)]
    );
    // Bora sees it now, and its sharing is the owner's.
    let (status, _, _) = send(&router, get(&uri, &bora)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(&router, get(&format!("{uri}/access"), &bora)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // A viewer writes nothing.
    let update = command(
        bora_space,
        "sheet.template.update",
        "5",
        json!({ "templateId": made.template_id, "expectedRevision": 1, "content": content("Bora") }),
    );
    let (status, _, _) = send(
        &router,
        json_req(
            "POST",
            &format!("/v1/tenants/{bora_space}/commands"),
            &bora,
            update,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // A stale revision is a conflict with the revision now.
    let v2 = command(
        space,
        "sheet.template.update",
        "6",
        json!({ "templateId": made.template_id, "expectedRevision": 1, "content": content("Büro A3 v2") }),
    );
    assert_eq!(
        send(&router, json_req("POST", &commands, &ayse, v2))
            .await
            .0,
        StatusCode::CREATED
    );
    let stale = command(
        space,
        "sheet.template.update",
        "7",
        json!({ "templateId": made.template_id, "expectedRevision": 1, "content": content("Eski") }),
    );
    let (status, _, body) = send(&router, json_req("POST", &commands, &ayse, stale)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        serde_json::from_slice::<ApiError>(&body)
            .unwrap()
            .revision
            .as_deref(),
        Some("2")
    );

    // Bora's events: shared, then the new revision; a long poll with nothing new waits and answers empty.
    let (status, _, body) =
        send(&router, get("/v1/me/sheet-templates/events?after=0", &bora)).await;
    assert_eq!(status, StatusCode::OK);
    let page: SheetTemplateEventPage = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        page.events.iter().map(|e| e.kind).collect::<Vec<_>>(),
        [
            SheetTemplateEventKind::Shared,
            SheetTemplateEventKind::Updated
        ]
    );
    let started = std::time::Instant::now();
    let (status, _, body) = send(
        &router,
        get(
            &format!("/v1/me/sheet-templates/events?after={}&wait=1", page.next),
            &bora,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        serde_json::from_slice::<SheetTemplateEventPage>(&body)
            .unwrap()
            .events
            .is_empty()
    );
    assert!(started.elapsed() >= std::time::Duration::from_millis(900));
    let (status, _, _) = send(
        &router,
        get("/v1/me/sheet-templates/events?after=yarin", &bora),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    db.close().await;
}

/// The desktop's client over a real socket: its personal space, a template saved, listed, read, revised,
/// shared and followed, as the web does it.
#[tokio::test]
async fn the_desktop_client_keeps_a_template_library() {
    use kentos_cloud::{Cloud, sheet_templates as lib};
    let Some(db) = TestDb::create().await else {
        return;
    };
    office(&db).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = app(Some(db.app.clone()));
    tokio::spawn(async move { axum::serve(listener, router).await });
    let base = format!("http://{addr}");
    let ayse = Cloud::new(&base).unwrap();
    ayse.sign_in("ayse", "dogru-parola-1").await.unwrap();
    let bora = Cloud::new(&base).unwrap();
    bora.sign_in("bora", "dogru-parola-1").await.unwrap();

    let space = lib::personal_space(&ayse).await.unwrap();
    let template: kentos_sheet::template::Template =
        serde_json::from_value(content("Masaüstünden")).unwrap();
    let key = Uuid::now_v7();
    let made = lib::create(&ayse, space, &template, key).await.unwrap();
    // A retry with the same key: the same template.
    let again = lib::create(&ayse, space, &template, key).await.unwrap();
    assert_eq!(
        (again.template_id.as_str(), again.replayed),
        (made.template_id.as_str(), true)
    );
    let id: Uuid = made.template_id.parse().unwrap();
    let list = lib::list(&ayse).await.unwrap();
    assert_eq!(list.templates.len(), 1);
    let d = lib::detail(&ayse, id).await.unwrap();
    assert_eq!(d.summary.revision, 1);
    let v2 = lib::update(&ayse, space, id, 1, &d.content, Uuid::now_v7())
        .await
        .unwrap();
    assert_eq!(v2.revision, 2);
    let stale = lib::update(&ayse, space, id, 1, &d.content, Uuid::now_v7())
        .await
        .unwrap_err();
    assert_eq!((stale.status, stale.code.as_str()), (409, "conflict"));
    let bora_id = admin::user_id(&db.owner, "bora").await.unwrap();
    assert_eq!(
        lib::candidates(&ayse, id, "tan")
            .await
            .unwrap()
            .candidates
            .len(),
        1
    );
    lib::share(
        &ayse,
        space,
        id,
        bora_id,
        TemplateGrantRole::Editor,
        Uuid::now_v7(),
    )
    .await
    .unwrap();
    assert_eq!(lib::access(&ayse, id).await.unwrap().grants.len(), 1);
    // Bora's device: the list as the remote side of a sync, and the events after its cursor.
    let theirs = lib::list(&bora).await.unwrap();
    let plan = kentos_sheet::sync::plan_sync(&[], &lib::remote(&theirs));
    let first = serde_json::to_value(&plan.actions).unwrap()[0].clone();
    assert_eq!(
        (first["type"].as_str(), first["revision"].as_u64()),
        (Some("download"), Some(2))
    );
    let page = lib::events(&bora, "0", false).await.unwrap();
    assert_eq!(
        page.events.iter().map(|e| e.kind).collect::<Vec<_>>(),
        [SheetTemplateEventKind::Shared]
    );
    // The owner deletes it; Bora's long poll, already waiting, hears it at once (the command wakes it, not the next look).
    let wait = lib::events(&bora, &page.next, true);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let gone = lib::delete(&ayse, space, id, Some(2), Uuid::now_v7())
        .await
        .unwrap();
    assert!(gone.deleted);
    let deleted = std::time::Instant::now();
    let heard = wait.await.unwrap();
    assert!(
        deleted.elapsed() < std::time::Duration::from_secs(3),
        "{:?}",
        deleted.elapsed()
    );
    assert_eq!(
        heard.events.iter().map(|e| e.kind).collect::<Vec<_>>(),
        [SheetTemplateEventKind::Deleted]
    );
    assert!(lib::list(&bora).await.unwrap().templates.is_empty());
    // Bora's own space is another one.
    assert_ne!(lib::personal_space(&bora).await.unwrap(), space);
    db.close().await;
}

/// The desktop's sync (`kentos_cloud::sheet_library`, the web's `templateSync`)
/// against the real routes, three devices: Ayşe's two and Bora's. A template
/// made on one device goes up and reaches the other; shared, it reaches Bora
/// as his editor's copy; an edit travels, heard by the long poll; a conflict
/// keeps both; a right taken away keeps the changes as a template of the
/// device; a deletion leaves every device.
#[tokio::test]
async fn the_desktop_syncs_a_template_library_across_devices_and_accounts() {
    use kentos_cloud::sheet_library::{
        Account, DeviceLibrary, DeviceRecord, MemoryLibrary, SyncEnd, personal_of, sync_once, texts,
    };
    use kentos_cloud::{Cloud, sheet_templates as lib};
    use kentos_sheet::cloud::DeviceCloudState;

    let Some(db) = TestDb::create().await else {
        return;
    };
    office(&db).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = app(Some(db.app.clone()));
    tokio::spawn(async move { axum::serve(listener, router).await });
    let base = format!("http://{addr}");
    let sign = |login: &'static str| {
        let base = base.clone();
        async move {
            let c = Cloud::new(&base).unwrap();
            let me = c.sign_in(login, "dogru-parola-1").await.unwrap();
            (c, Account::of(&me), personal_of(&me))
        }
    };
    let (a1, ayse, ayse_space) = sign("ayse").await;
    let (a2, _, _) = sign("ayse").await;
    let (b, bora, bora_space) = sign("bora").await;
    let (dev_a1, dev_a2, dev_b) = (
        MemoryLibrary::default(),
        MemoryLibrary::default(),
        MemoryLibrary::default(),
    );
    let quiet = |_: &str, _: bool| {};
    let one = |lib: &MemoryLibrary| {
        let all = lib.records();
        assert_eq!(
            all.len(),
            1,
            "{:?}",
            all.iter().map(|r| &r.id).collect::<Vec<_>>()
        );
        all.into_iter().next().unwrap()
    };

    // Ayşe saves a template on her first device, then “Buluta eşitle”.
    let mut t: kentos_sheet::template::Template =
        serde_json::from_value(content("Belediye ifraz paftası")).unwrap();
    t.meta.id = "0199a1b2-0000-7000-8000-00000000aa01".into();
    dev_a1
        .save(&DeviceRecord {
            id: t.meta.id.clone(),
            template: t.clone(),
            cloud: Some(DeviceCloudState::to_upload(&ayse.id)),
        })
        .unwrap();
    let r = sync_once(&a1, ayse_space, &ayse, &dev_a1, &quiet).await;
    assert_eq!(r.end, SyncEnd::Synced);
    assert_eq!(r.created.len(), 1);
    let id = r.created[0].1.clone();
    let mine = one(&dev_a1);
    let c = mine.cloud.clone().unwrap();
    assert_eq!(
        (mine.id.as_str(), c.revision, c.changed),
        (id.as_str(), 1, false)
    );
    assert_eq!(c.former_ids, [t.meta.id.clone()]);
    assert_eq!(mine.template.meta.id, id, "the cloud's id in the copy too");

    // Her second device downloads it: the same template on both (the cloud wrote its own dates
    // into its copy; the device that sent it keeps the ones it sent, as the web's does).
    let r = sync_once(&a2, ayse_space, &ayse, &dev_a2, &quiet).await;
    assert_eq!(r.end, SyncEnd::Synced);
    let undated = |mut t: kentos_sheet::template::Template| {
        t.meta.created.clear();
        t.meta.updated.clear();
        t
    };
    assert_eq!(
        undated(one(&dev_a2).template),
        undated(one(&dev_a1).template)
    );

    // Shared with Bora as an editor: his device lists it as shared with him.
    let bora_id = admin::user_id(&db.owner, "bora").await.unwrap();
    let uuid: Uuid = id.parse().unwrap();
    lib::share(
        &a1,
        ayse_space.unwrap(),
        uuid,
        bora_id,
        TemplateGrantRole::Editor,
        Uuid::now_v7(),
    )
    .await
    .unwrap();
    let r = sync_once(&b, bora_space, &bora, &dev_b, &quiet).await;
    assert_eq!(r.end, SyncEnd::Synced);
    let his = one(&dev_b);
    let c = his.cloud.clone().unwrap();
    assert_eq!(
        (c.role, c.owner_name.as_deref(), c.revision),
        (TemplateRole::Editor, Some("Ayşe Yılmaz"), 1)
    );
    // Ayşe's copies learn it is shared (the gallery's “Paylaşıldı”).
    sync_once(&a1, ayse_space, &ayse, &dev_a1, &quiet).await;
    assert!(one(&dev_a1).cloud.unwrap().shared);

    // Bora's device waits for events; Ayşe edits on her first device; the wait hears it at once.
    let cursor = kentos_cloud::sheet_library::newest_cursor(&b)
        .await
        .unwrap();
    let wait = lib::events(&b, &cursor, true);
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    let mut edited = one(&dev_a1);
    edited.template.meta.description = "Kızılırmak mahallesi, 2026 düzeni.".into();
    edited.cloud.as_mut().unwrap().changed = true;
    dev_a1.save(&edited).unwrap();
    let started = std::time::Instant::now();
    let r = sync_once(&a1, ayse_space, &ayse, &dev_a1, &quiet).await;
    assert_eq!(r.end, SyncEnd::Synced);
    assert_eq!(one(&dev_a1).cloud.unwrap().revision, 2);
    let heard = wait.await.unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    assert!(!heard.events.is_empty());
    for (cloud, dev, account, space) in [
        (&b, &dev_b, &bora, bora_space),
        (&a2, &dev_a2, &ayse, ayse_space),
    ] {
        sync_once(cloud, space, account, dev, &quiet).await;
        let r = one(dev);
        assert_eq!(
            (
                r.cloud.unwrap().revision,
                r.template.meta.description.as_str()
            ),
            (2, "Kızılırmak mahallesi, 2026 düzeni.")
        );
    }

    // Both of Ayşe's devices change it before either syncs: the first goes up, the second keeps both.
    for (dev, name) in [(&dev_a1, "İlk cihaz"), (&dev_a2, "İkinci cihaz")] {
        let mut r = one(dev);
        r.template.meta.description = name.into();
        r.cloud.as_mut().unwrap().changed = true;
        dev.save(&r).unwrap();
    }
    sync_once(&a1, ayse_space, &ayse, &dev_a1, &quiet).await;
    let r = sync_once(&a2, ayse_space, &ayse, &dev_a2, &quiet).await;
    let copy = format!("Belediye ifraz paftası{}", kentos_sheet::sync::COPY_SUFFIX);
    assert!(
        r.said
            .iter()
            .any(|(_, s)| *s == texts::conflict("Belediye ifraz paftası", &copy)),
        "{:?}",
        r.said
    );
    let all = dev_a2.records();
    assert_eq!(all.len(), 2);
    let theirs = all.iter().find(|x| x.id == id).unwrap();
    let kept = all.iter().find(|x| x.id != id).unwrap();
    assert_eq!(theirs.template.meta.description, "İlk cihaz");
    assert_eq!(
        (
            kept.template.meta.name.as_str(),
            kept.template.meta.description.as_str()
        ),
        (copy.as_str(), "İkinci cihaz")
    );
    assert_eq!(
        kept.cloud.as_ref().unwrap().conflict_of.as_deref(),
        Some(id.as_str())
    );

    // Bora's right to write is taken away while he edits: his changes stay on his device as a copy.
    sync_once(&b, bora_space, &bora, &dev_b, &quiet).await;
    lib::share(
        &a1,
        ayse_space.unwrap(),
        uuid,
        bora_id,
        TemplateGrantRole::Viewer,
        Uuid::now_v7(),
    )
    .await
    .unwrap();
    let mut his = one(&dev_b);
    his.template.meta.description = "Bora'nın değişikliği".into();
    his.cloud.as_mut().unwrap().changed = true;
    dev_b.save(&his).unwrap();
    let r = sync_once(&b, bora_space, &bora, &dev_b, &quiet).await;
    assert!(
        r.said
            .iter()
            .any(|(_, s)| *s == texts::read_only("Belediye ifraz paftası", &copy)),
        "{:?}",
        r.said
    );
    let all = dev_b.records();
    assert_eq!(all.len(), 2);
    let device_only = all.iter().find(|x| x.cloud.is_none()).unwrap();
    assert_eq!(
        device_only.template.meta.description,
        "Bora'nın değişikliği"
    );
    let shared = all.iter().find(|x| x.id == id).unwrap();
    assert_eq!(shared.cloud.as_ref().unwrap().role, TemplateRole::Viewer);

    // Ayşe deletes it on her first device: it leaves the cloud, her other device and Bora's list.
    let mut gone = one_by(&dev_a1, &id);
    gone.cloud.as_mut().unwrap().deleted = true;
    dev_a1.save(&gone).unwrap();
    let r = sync_once(&a1, ayse_space, &ayse, &dev_a1, &quiet).await;
    assert_eq!(r.end, SyncEnd::Synced);
    assert!(dev_a1.get(&id).is_none());
    let r = sync_once(&a2, ayse_space, &ayse, &dev_a2, &quiet).await;
    assert!(
        r.said
            .iter()
            .any(|(_, s)| *s == texts::removed("Belediye ifraz paftası"))
    );
    assert!(dev_a2.get(&id).is_none());
    let r = sync_once(&b, bora_space, &bora, &dev_b, &quiet).await;
    assert!(
        r.said
            .iter()
            .any(|(_, s)| *s == texts::unshared("Belediye ifraz paftası"))
    );
    assert!(dev_b.get(&id).is_none());
    // What was kept as copies stays.
    assert_eq!(dev_b.records().len(), 1);
    assert_eq!(dev_a2.records().len(), 1);
    db.close().await;
}

/// One record of a device by its id.
fn one_by(
    lib: &kentos_cloud::sheet_library::MemoryLibrary,
    id: &str,
) -> kentos_cloud::sheet_library::DeviceRecord {
    lib.get(id)
        .unwrap_or_else(|| panic!("{id} is not on the device"))
}

/// An organisation's library (design §13 “Kurum şablonları”) over HTTP and on a device: Ayşe
/// (project manager) publishes one of hers into Büro through Büro's command route; Bora (editor)
/// sees it in Büro's library and may neither publish nor edit it (403); Can, of another
/// organisation, gets 404 from Büro's route and from its template, word for word as a guessed id;
/// Emre (administrator) edits it. Bora's device syncs it as Büro's: his changes, refused, stay
/// as a template of his device; once he leaves Büro the copy leaves his device, said.
#[tokio::test]
async fn an_organisation_s_library_over_http_and_on_a_device() {
    use kentos_cloud::sheet_library::{
        Account, DeviceLibrary, MemoryLibrary, SyncEnd, personal_of, sync_once, texts,
    };
    use kentos_cloud::{Cloud, sheet_templates as lib};

    let Some(db) = TestDb::create().await else {
        return;
    };
    office(&db).await;
    admin::create_local_user(&db.owner, "emre", "Emre Kaya", None, "dogru-parola-1")
        .await
        .unwrap();
    admin::set_membership(&db.owner, "buro", "emre", TenantRole::Admin, true)
        .await
        .unwrap();
    let buro: Uuid = sqlx::query_scalar("select id from kentos.tenant where slug = 'buro'")
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let router = app(Some(db.app.clone()));
    let (ayse, bora, can, emre) = (
        signed_in(&router, "ayse").await,
        signed_in(&router, "bora").await,
        signed_in(&router, "can").await,
        signed_in(&router, "emre").await,
    );
    let ayse_space = personal(&router, &ayse).await;
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("/v1/tenants/{ayse_space}/commands"),
            &ayse,
            command(
                ayse_space,
                "sheet.template.create",
                "o1",
                json!({ "content": content("Belediye ifraz paftası") }),
            ),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let own: SheetTemplateChanged = serde_json::from_slice(&body).unwrap();

    let org_commands = format!("/v1/tenants/{buro}/commands");
    let publish = |key: &str| {
        command(
            buro,
            "sheet.template.publish",
            key,
            json!({ "templateId": own.template_id, "tenantId": buro }),
        )
    };
    // Can is not in Büro: its route does not exist for him.
    let (status, _, body) = send(
        &router,
        json_req("POST", &org_commands, &can, publish("o2")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "{}",
        String::from_utf8_lossy(&body)
    );
    // Bora is, but may not publish.
    let (status, _, body) = send(
        &router,
        json_req("POST", &org_commands, &bora, publish("o3")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let (status, _, body) = send(
        &router,
        json_req("POST", &org_commands, &ayse, publish("o4")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let published: SheetTemplateChanged = serde_json::from_slice(&body).unwrap();
    let uri = format!("/v1/sheet-templates/{}", published.template_id);

    // Bora's list: Büro's library with it, as a viewer, its publisher named.
    let (status, _, body) = send(&router, get("/v1/me/sheet-templates", &bora)).await;
    assert_eq!(status, StatusCode::OK);
    let list: SheetTemplateList = serde_json::from_slice(&body).unwrap();
    assert!(list.templates.is_empty());
    let b = &list.organizations[0];
    assert_eq!(
        (b.tenant_id.as_str(), b.name.as_str(), b.can_publish),
        (buro.to_string().as_str(), "Harita Bürosu", false)
    );
    assert_eq!(
        b.templates
            .iter()
            .map(|t| (
                t.id.as_str(),
                t.role,
                t.owner_name.as_str(),
                t.published_from.as_deref()
            ))
            .collect::<Vec<_>>(),
        [(
            published.template_id.as_str(),
            TemplateRole::Viewer,
            "Ayşe Yılmaz",
            Some(own.template_id.as_str())
        )]
    );
    let (status, _, body) = send(&router, get("/v1/me/sheet-templates", &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    let list: SheetTemplateList = serde_json::from_slice(&body).unwrap();
    assert!(list.organizations[0].can_publish);
    // Can: 404 for the template, word for word as a guessed id; no Büro in his list.
    let seen = not_found_body(&router, get(&uri, &can)).await;
    let guessed = format!("/v1/sheet-templates/{}", Uuid::now_v7());
    assert_eq!(seen, not_found_body(&router, get(&guessed, &can)).await);
    let (_, _, body) = send(&router, get("/v1/me/sheet-templates", &can)).await;
    let list: SheetTemplateList = serde_json::from_slice(&body).unwrap();
    assert!(
        list.organizations
            .iter()
            .all(|o| o.tenant_id != buro.to_string())
    );

    // Editing: 403 for Bora, a new revision for Emre.
    let (_, _, body) = send(&router, get(&uri, &bora)).await;
    let detail: SheetTemplateDetail = serde_json::from_slice(&body).unwrap();
    let update = |key: &str| {
        command(
            buro,
            "sheet.template.update",
            key,
            json!({ "templateId": published.template_id, "expectedRevision": 1, "content": detail.content }),
        )
    };
    let (status, _, _) = send(
        &router,
        json_req("POST", &org_commands, &bora, update("o5")),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, body) = send(
        &router,
        json_req("POST", &org_commands, &emre, update("o6")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    // Not shared one by one.
    let bora_id = admin::user_id(&db.owner, "bora").await.unwrap();
    let share = command(
        buro,
        "sheet.template.share",
        "o7",
        json!({ "templateId": published.template_id, "userId": bora_id, "role": "editor" }),
    );
    let (status, _, _) = send(&router, json_req("POST", &org_commands, &ayse, share)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Bora's device, through the desktop's client over a real socket.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let served = app(Some(db.app.clone()));
    tokio::spawn(async move { axum::serve(listener, served).await });
    let client = Cloud::new(&format!("http://{addr}")).unwrap();
    let me = client.sign_in("bora", "dogru-parola-1").await.unwrap();
    let (account, space) = (Account::of(&me), personal_of(&me));
    let device = MemoryLibrary::default();
    let quiet = |_: &str, _: bool| {};
    let r = sync_once(&client, space, &account, &device, &quiet).await;
    assert_eq!(r.end, SyncEnd::Synced);
    assert_eq!(
        r.organizations
            .iter()
            .flatten()
            .map(|o| (o.name.as_str(), o.can_publish))
            .collect::<Vec<_>>(),
        [("Harita Bürosu", false)]
    );
    let copy = device
        .get(&published.template_id)
        .expect("Büro's template on his device");
    let c = copy.cloud.clone().unwrap();
    assert_eq!(
        (
            c.role,
            c.revision,
            c.owner_name.as_deref(),
            c.organization.as_ref().map(|o| o.tenant_id.clone())
        ),
        (
            TemplateRole::Viewer,
            2,
            Some("Ayşe Yılmaz"),
            Some(buro.to_string())
        )
    );
    assert_eq!(c.published_from.as_deref(), Some(own.template_id.as_str()));
    // He changes it on his device: Büro refuses (403); his changes stay as a template of his device.
    let mut his = copy.clone();
    his.template.meta.description = "Bora'nın değişikliği".into();
    his.cloud.as_mut().unwrap().changed = true;
    device.save(&his).unwrap();
    let r = sync_once(&client, space, &account, &device, &quiet).await;
    let kept = format!("Belediye ifraz paftası{}", kentos_sheet::sync::COPY_SUFFIX);
    assert!(
        r.said
            .iter()
            .any(|(_, s)| *s == texts::read_only("Belediye ifraz paftası", &kept)),
        "{:?}",
        r.said
    );
    assert_eq!(device.records().len(), 2);
    assert_eq!(
        device
            .get(&published.template_id)
            .unwrap()
            .cloud
            .unwrap()
            .revision,
        2
    );
    // He leaves Büro: the copy of Büro's template leaves his device, said; his own copy stays.
    sqlx::query("delete from kentos.membership where user_id = $1 and tenant_id = $2")
        .bind(bora_id)
        .bind(buro)
        .execute(&db.owner)
        .await
        .unwrap();
    let r = sync_once(&client, space, &account, &device, &quiet).await;
    assert_eq!(r.end, SyncEnd::Synced);
    assert_eq!(r.organizations, Some(Vec::new()));
    assert!(
        r.said.iter().any(|(_, s)| *s
            == texts::left_organisation("Belediye ifraz paftası", "Harita Bürosu")),
        "{:?}",
        r.said
    );
    assert!(device.get(&published.template_id).is_none());
    assert_eq!(device.records().len(), 1);
    // The client's own publish: Ayşe publishes again from her desktop, into Büro.
    let a = Cloud::new(&format!("http://{addr}")).unwrap();
    a.sign_in("ayse", "dogru-parola-1").await.unwrap();
    let again = lib::publish(&a, buro, own.template_id.parse().unwrap(), Uuid::now_v7())
        .await
        .unwrap();
    assert_ne!(again.template_id, published.template_id);
    assert_eq!(
        lib::list(&a).await.unwrap().organizations[0]
            .templates
            .len(),
        2
    );
    let _ = &can;
    db.close().await;
}
