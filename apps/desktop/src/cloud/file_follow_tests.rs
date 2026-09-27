//! An open file project followed (file_follow.rs, docs/specs/file-revisions.md):
//! another's revision is heard, said once and offered, never loaded by
//! itself; this window's own commit is not another's; Kaydet over a newer
//! revision uploads nothing; the conflict's four answers; a resync never
//! reopens the drawing; a deleted project ends saving there.

use kentos_contracts::{
    ConflictReason, EventPage, EventRecord, FeatureConflict, FileCommitted, FileRevision,
    FileRevisions, ProjectState, ProjectStorage,
};

use iced::keyboard::Modifiers;
use iced::keyboard::key::{Key, Named, NativeCode, Physical};
use kentos_cloud::ApiFailure;

use super::revisions::{Answer, QuestionId};
use super::tests::{TENANT, cell_text, cloud, edit, file_project, info, said, signed_in};
use crate::app::{App, Dialog, Message};
use crate::cloud::{Event, Once};
use crate::keys::KeyPress;

fn key(app: &mut App, named: Named) {
    let _ = app.update(Message::Key(KeyPress {
        key: Key::Named(named),
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers: Modifiers::default(),
        text: None,
        repeat: false,
    }));
}

fn session(app: &App) -> u64 {
    app.document.as_ref().expect("a drawing").session
}

fn event(seq: &str, kind: &str, request: Option<&str>) -> EventRecord {
    EventRecord {
        seq: seq.to_owned(),
        data_revision: "0".to_owned(),
        kind: kind.to_owned(),
        actor: None,
        request_id: request.map(str::to_owned),
        features: Vec::new(),
        meta: false,
    }
}

fn events(app: &mut App, list: Vec<EventRecord>) {
    let session = session(app);
    let next = list
        .last()
        .map_or_else(|| "9".to_owned(), |e| e.seq.clone());
    cloud(
        app,
        Event::FileEvents {
            session,
            result: Ok(EventPage { events: list, next }),
        },
    );
}

/// The server's list with revision `n` its newest, saved by Mehmet Demir.
fn newest(app: &mut App, n: &str) {
    let session = session(app);
    cloud(
        app,
        Event::FileNewest {
            session,
            result: Ok(FileRevisions {
                current: Some(n.to_owned()),
                revisions: vec![FileRevision {
                    revision: n.to_owned(),
                    size: 3,
                    sha256: "ef".repeat(32),
                    created_by: "u-mehmet".to_owned(),
                    created_by_name: "Mehmet Demir".to_owned(),
                    created_at: "2026-09-27T11:32:00Z".to_owned(),
                    objects: Some("13".to_owned()),
                }],
            }),
        },
    );
}

fn tip(app: &App) -> String {
    app.save_cell()
        .map(|c| c.tip.description)
        .unwrap_or_default()
}

fn question_id(app: &App) -> Option<QuestionId> {
    app.cloud.question.as_ref().map(|q| q.id)
}

#[test]
fn a_file_project_is_followed_from_the_cursor_it_was_read_at() {
    let mut app = signed_in();
    file_project(&mut app);
    let f = app.cloud.file.as_ref().expect("followed");
    assert_eq!(f.cursor, "9");
    assert_eq!(f.session, session(&app));
    // The tip says the live link now that its events are followed.
    assert!(
        tip(&app).ends_with("Canlı bağlantı: canlı."),
        "{}",
        tip(&app)
    );

    // An archived project is not followed: nothing is saved there.
    let mut archived = signed_in();
    super::tests::open(
        &mut archived,
        info(ProjectStorage::File, true, ProjectState::Archived),
    );
    assert!(archived.cloud.file.is_none());
    assert_eq!(cell_text(&archived), "Proje arşivde");
}

#[test]
fn another_s_revision_is_heard_said_once_and_offered() {
    let mut app = signed_in();
    file_project(&mut app);
    events(
        &mut app,
        vec![event("10", "project.file", Some("web-baska"))],
    );
    assert_eq!(
        app.cloud.file.as_ref().map(|f| f.cursor.as_str()),
        Some("10")
    );
    assert_eq!(
        cell_text(&app),
        "Buluta kaydedildi · r3",
        "not known before the answer"
    );
    newest(&mut app, "4");
    assert_eq!(cell_text(&app), "Yeni revizyon: r4");
    let line = "“Ada 101” başka bir yerde kaydedildi: revizyon 4 (Mehmet Demir, 27.09.2026 14:32). Açık çizimin dayandığı revizyon: 3; yeni revizyonu açmak için durum çubuğundaki kayıt durumuna tıklayın. Kendiliğinden yeniden yüklenmez.";
    // The device's zone decides the time; the test's is the fixture's only when it is Istanbul's.
    let heard: Vec<String> = said(&app)
        .into_iter()
        .filter(|t| t.contains("başka bir yerde kaydedildi"))
        .collect();
    assert_eq!(heard.len(), 1, "{heard:?}");
    if crate::cloud::local_time::Zone::system().offset_at(0) == 3 * 3600 {
        assert_eq!(heard[0], line);
    }
    assert!(tip(&app).contains("Sunucuda daha yeni revizyon var: 4 (Mehmet Demir"));
    assert!(
        app.document.as_ref().is_some_and(|d| !d.dirty()),
        "nothing reloaded"
    );
    // Said once: the same revision again says nothing.
    newest(&mut app, "4");
    let again = said(&app)
        .iter()
        .filter(|t| t.contains("başka bir yerde kaydedildi"))
        .count();
    assert_eq!(again, 1);

    // The cell offers it: a plain question over a clean drawing.
    let action = app.save_cell().and_then(|c| c.action);
    assert_eq!(action, Some("cloud.openNewest"));
    let _ = app.run("cloud.openNewest");
    assert_eq!(app.dialog, Some(Dialog::Revision));
    assert_eq!(question_id(&app), Some(QuestionId::Newest));
    // Enter is its amber answer: the newest revision, from the server only.
    key(&mut app, Named::Enter);
    assert!(app.cloud.opening.as_ref().is_some_and(|o| o.newest));
    cloud(&mut app, Event::OpenCancel);

    // Over unsaved work the cell says so, and its click is the conflict's question.
    edit(&mut app, 1.0);
    assert_eq!(cell_text(&app), "Yeni revizyon: r4 · kaydedilmedi");
    let _ = app.run("cloud.openNewest");
    assert_eq!(question_id(&app), Some(QuestionId::Conflict));
}

#[test]
fn this_window_s_own_commit_is_not_another_s_revision() {
    let mut app = signed_in();
    file_project(&mut app);
    edit(&mut app, 1.0);
    let _ = app.run("file.save");
    let id = app.saving.as_ref().expect("saving").id;
    let _ = app.update(Message::Saving(crate::saving::Event::Encoded {
        id,
        result: Ok(Once::new(vec![1, 2, 3])),
    }));
    let own = app
        .cloud
        .file
        .as_ref()
        .and_then(|f| f.own_requests().next().map(str::to_owned))
        .expect("the commit's request id kept");
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
    events(&mut app, vec![event("10", "project.file", Some(&own))]);
    assert!(
        app.cloud.file.as_ref().is_some_and(|f| !f.asking()),
        "its own event asks nothing"
    );
    assert_eq!(cell_text(&app), "Buluta kaydedildi · r4");
    // A late, older answer does not lower what is known.
    newest(&mut app, "3");
    assert_eq!(cell_text(&app), "Buluta kaydedildi · r4");
}

#[test]
fn kaydet_over_a_known_newer_revision_uploads_nothing_and_asks() {
    let mut app = signed_in();
    file_project(&mut app);
    events(&mut app, vec![event("10", "project.file", None)]);
    newest(&mut app, "5");
    edit(&mut app, 1.0);
    let _ = app.run("file.save");
    assert!(app.saving.is_none(), "nothing encoded or uploaded");
    assert_eq!(app.dialog, Some(Dialog::Revision));
    assert_eq!(question_id(&app), Some(QuestionId::Conflict));
    let c = app.cloud.file_conflict.clone().expect("a conflict");
    assert_eq!((c.server, c.based_on), (5, 3));
    assert_eq!(cell_text(&app), "Çakışma: r5 kaydedilmiş");
    // Vazgeç changes nothing; Kaydet asks again, still without uploading.
    key(&mut app, Named::Escape);
    assert_eq!(app.dialog, None);
    assert!(app.cloud.file_conflict.is_some(), "the conflict stands");
    let _ = app.run("file.save");
    assert!(app.saving.is_none());
    assert_eq!(question_id(&app), Some(QuestionId::Conflict));
}

#[test]
fn a_refused_kaydet_offers_a_copy_a_local_file_or_the_newest_revision() {
    let mut app = signed_in();
    file_project(&mut app);
    edit(&mut app, 1.0);
    let _ = app.run("file.save");
    let id = app.saving.as_ref().expect("saving").id;
    let _ = app.update(Message::Saving(crate::saving::Event::Encoded {
        id,
        result: Ok(Once::new(vec![1, 2, 3])),
    }));
    let failure = ApiFailure::new(409, "conflict", "Başka biri daha önce kaydetti.")
        .with_conflicts(vec![FeatureConflict {
            id: "@file".into(),
            reason: ConflictReason::Changed,
            expected: Some("3".into()),
            actual: Some("5".into()),
            current: None,
        }]);
    cloud(
        &mut app,
        Event::FileSaved {
            id,
            result: Err(failure),
        },
    );
    assert_eq!(app.dialog, Some(Dialog::Revision));
    let q = app.cloud.question.clone().expect("the question");
    assert_eq!(q.title, "Dosya başka biri tarafından kaydedildi");
    assert_eq!(
        q.message,
        "“Ada 101” siz çalışırken başka biri tarafından kaydedildi: sunucuda revizyon 5 var; çiziminizin dayandığı revizyon 3. Hiçbir şey yazılmadı; iki dosya birleştirilmez."
    );
    assert_eq!(
        q.answers.iter().map(|a| a.value).collect::<Vec<_>>(),
        [Answer::Latest, Answer::Stay, Answer::Local, Answer::Copy]
    );
    let c = app.cloud.file_conflict.clone().expect("a conflict");
    assert_eq!((c.server, c.based_on), (5, 3));
    assert_eq!(cell_text(&app), "Çakışma: r5 kaydedilmiş");
    assert!(
        app.document.as_ref().is_some_and(|d| d.dirty()),
        "nothing written"
    );
    // The refusal named only the number: who saved it and when are asked, and known after.
    assert!(app.cloud.file.as_ref().is_some_and(|f| f.asking()));
    newest(&mut app, "5");
    assert!(tip(&app).contains("5 (Mehmet Demir"), "{}", tip(&app));

    // Son revizyonu aç: the work is dropped, the newest revision opens from the server.
    cloud(&mut app, Event::RevisionAnswer(Answer::Latest));
    assert!(app.cloud.opening.as_ref().is_some_and(|o| o.newest));
    cloud(&mut app, Event::OpenCancel);
    assert!(
        app.cloud.file_conflict.is_some(),
        "stopped: the conflict stands"
    );

    // Yerel dosyaya kaydet: Farklı kaydet; once written, the drawing leaves the project.
    let dir = crate::files_testing::scratch("dosya-cakismasi-yerel");
    let path = dir.join("Ada 101.kcad");
    app.picker = crate::app::Picker::File(path.clone());
    let _ = app.run("cloud.conflicts");
    let task = app.update(crate::cloud::msg(Event::RevisionAnswer(Answer::Local)));
    crate::files_testing::drive(&mut app, task);
    let doc = app.document.as_ref().expect("the drawing stays");
    assert!(doc.cloud_source().is_none(), "left the project");
    assert_eq!(doc.path.as_deref(), Some(path.as_path()));
    assert!(said(&app).iter().any(|t| t
        == "Çizim yerel dosyaya kaydedildi ve “Ada 101” bulut projesinden ayrıldı; proje olduğu gibi duruyor."));
    assert!(app.cloud.file.is_none(), "nothing more is followed");
    let _ = std::fs::remove_dir_all(dir);

    // Ayrı kopya olarak kaydet: the upload window, on file storage, as a copy.
    let mut app = signed_in();
    file_project(&mut app);
    edit(&mut app, 1.0);
    events(&mut app, vec![event("10", "project.file", None)]);
    newest(&mut app, "5");
    let _ = app.run("file.save");
    cloud(&mut app, Event::RevisionAnswer(Answer::Copy));
    let u = app.cloud.upload.as_ref().expect("the upload");
    assert_eq!(
        (u.name.as_str(), u.storage),
        ("Ada 101 (kopya)", ProjectStorage::File)
    );
    assert_eq!(u.tenants[u.tenant].tenant_id, TENANT);
    assert!(u.working(), "straight up");
}

#[test]
fn a_resync_asks_the_project_and_never_reopens_the_drawing() {
    let mut app = signed_in();
    file_project(&mut app);
    edit(&mut app, 1.0);
    let before = session(&app);
    cloud(
        &mut app,
        Event::FileEvents {
            session: before,
            result: Err(ApiFailure::new(
                410,
                "resync_required",
                "Olaylar artık tutulmuyor.",
            )),
        },
    );
    assert!(app.cloud.opening.is_none(), "not opened again");
    assert!(app.cloud.file.as_ref().is_some_and(|f| f.resyncing()));
    // No answer: said once, asked again half a minute later; the drawing stays.
    for _ in 0..2 {
        cloud(
            &mut app,
            Event::FileResync {
                session: before,
                result: Err(ApiFailure::new(0, "network", "Sunucuya ulaşılamadı.")),
            },
        );
    }
    let failed = said(&app)
        .iter()
        .filter(|t| t.contains("kaçırılan olayları alınamadı"))
        .count();
    assert_eq!(failed, 1);
    // The project as it is: followed from its cursor now, its newest asked.
    let mut now = info(ProjectStorage::File, true, ProjectState::Active);
    now.event_cursor = "42".into();
    cloud(
        &mut app,
        Event::FileResync {
            session: before,
            result: Ok(now),
        },
    );
    let f = app.cloud.file.as_ref().expect("followed");
    assert_eq!(f.cursor, "42");
    assert!(f.asking(), "the newest revision asked");
    assert_eq!(session(&app), before, "the drawing is never replaced");
    assert!(app.document.as_ref().is_some_and(|d| d.dirty()));
}

#[test]
fn a_deleted_or_archived_project_ends_saving_there() {
    let mut app = signed_in();
    file_project(&mut app);
    events(&mut app, vec![event("10", "project.deleted", None)]);
    assert_eq!(cell_text(&app), "Proje silindi");
    assert_eq!(app.dialog, Some(Dialog::Ended));
    assert!(
        said(&app)
            .iter()
            .any(|t| t.starts_with("“Ada 101” bulut projesi silindi"))
    );
    let (title, _, _) = app.ended_notice().expect("the notice");
    assert_eq!(title, "Proje çöp kutusuna taşındı");
    // Son revizyonu aç says why it cannot, without a question.
    app.close_dialog();
    let _ = app.run("cloud.openNewest");
    assert_eq!(app.dialog, None);
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some(
            "“Ada 101” bulut projesi silindi; son revizyonu açılamaz. Çizimi saklamak için yerel bir dosyaya kaydedin."
        )
    );

    // Archived from this window: quiet.
    let mut app = signed_in();
    file_project(&mut app);
    events(&mut app, vec![event("10", "project.archived", None)]);
    assert_eq!(cell_text(&app), "Proje arşivde");
    assert!(said(&app).iter().any(|t| t.contains("arşivlendi")));
}

/// Pictures for the owner (docs/specs/file-revisions.md's scenes): the cell
/// over a newer revision and its tip, over unsaved work, and the three
/// questions; `.run/shots/bulut-revizyon-*`.
/// `cargo test -p kentos-desktop cloud::file_follow_tests::revision_screens -- --ignored --nocapture`
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn revision_screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "yeni",
                "yeni-ipucu",
                "yeni-kaydedilmedi",
                "son",
                "cakisma",
                "kaydedilmemis",
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
                if name != "kaydedilmemis" {
                    events(&mut app, vec![event("10", "project.file", None)]);
                    newest(&mut app, "4");
                }
                match name {
                    "yeni-kaydedilmedi" | "cakisma" | "kaydedilmemis" => {
                        edit(&mut app, 1.0);
                    }
                    _ => {}
                }
                match name {
                    "son" | "cakisma" | "kaydedilmemis" => {
                        let _ = app.run("cloud.openNewest");
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "yeni-ipucu" {
                    // The cell's words; a narrow window keeps only its lamp (docs/adr/0080).
                    let at =
                        crate::files_testing::find_text(&mut snapshot, &app, "Yeni revizyon: r4")
                            .map_or(Point::new(width - 118.0, height - 14.0), |cell| {
                                Point::new(cell.center_x(), cell.center_y())
                            });
                    snapshot.input(&mut app, App::view, &mut update, Input::Move(at));
                    std::thread::sleep(std::time::Duration::from_millis(600));
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!(
                    "bulut-revizyon-{name}-{width}x{height}{suffix}.png"
                ));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
