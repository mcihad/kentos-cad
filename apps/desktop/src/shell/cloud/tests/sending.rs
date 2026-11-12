//! Cloud tests: edits sent and retried, refusals, conflicts, others' changes followed, refused trees,
//! lost cursors and deletions.

use super::*;

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

pub(crate) fn conflict_on(app: &mut App, uid: Uuid) {
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

/// docs/adr/0737 §3: Kopyasını dosyaya yaz in the conflict writes the drawing with the draft to a file of its own; the
/// drawing stays the cloud project with its unsent work, and the question stays to be answered.
#[test]
fn the_conflict_writes_a_copy_of_the_draft_and_leaves_the_question() {
    let mut app = signed_in();
    database(&mut app);
    let uid = edit(&mut app, 3.0);
    app.cloud_after(Instant::now());
    let _ = app.run("file.save");
    conflict_on(&mut app, uid);
    let _ = app.run("cloud.conflicts");
    assert_eq!(app.dialog, Some(Dialog::Conflicts));
    let path = scratch("taslak").join("Proje (taslak).kcad");
    app.picker = crate::app::Picker::File(path.clone());
    let task = app.update(crate::cloud::msg(Event::DraftCopy));
    drive(&mut app, task);
    // The copy holds my move.
    let bytes = std::fs::read(&path).expect("the copy is written");
    let copy = kentos_kcad::decode(&bytes).expect("a KCAD v2 file");
    let mine = app.document.as_ref().expect("open");
    let moved = mine
        .model
        .get(mine.model.slot_of(uid).expect("there"))
        .cloned();
    let written = copy
        .entities
        .iter()
        .zip(copy.uids.iter())
        .find(|(_, u)| u.0 == *uid.as_bytes())
        .map(|(e, _)| e.clone());
    assert_eq!(written, moved, "the copy holds the drawing as it is");
    // The drawing is still the project, with its draft and the question.
    let doc = app.document.as_ref().expect("open");
    assert!(doc.cloud_source().is_some() && doc.path.is_none());
    assert_eq!(state(&app), SaveState::Conflict);
    assert_eq!(app.dialog, Some(Dialog::Conflicts));
    assert!(
        last_said(&app).starts_with("Kopyası yazıldı:"),
        "{}",
        last_said(&app)
    );
}

// ── Others' changes ─────────────────────────────────────────────────────

pub(super) fn event(seq: &str, request: &str, features: Vec<EventFeature>) -> EventRecord {
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
pub(super) fn on_cizim(x: f64) -> Entity {
    let mut e = server_drawing().get(Slot(1)).expect("a point").clone();
    assert_eq!(e.base().layer_id, "cizim");
    if let Entity::Point(PointEntity { p, .. }) = &mut e {
        p.x = x;
    }
    e
}

/// What the server had of `ids` when asked, all on “Çizim”, handed over.
pub(super) fn fetched(app: &mut App, ids: &[Uuid], cursor: &str) {
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
    deleted.kind = kentos_commands::PROJECT_DELETED.into();
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
