//! The sheet template library end to end, by hand, against a running API
//! (`kentosd serve` on a throwaway database) with two accounts of one
//! organisation: the real desktop app, its sheet mode, gallery and share
//! window driven by their own messages, its tasks run to their end, its
//! sheet store in a scratch folder. Three devices: Ayşe's two and Bora's;
//! what one does reaches the others by the account's events (the long
//! poll), not by asking.
//!
//! ```text
//! KENTOS_E2E_SERVER=http://127.0.0.1:55441 KENTOS_E2E_PASSWORD=… \
//!   [KENTOS_DEMO_DRAWING=/path/demo.json] [KENTOS_E2E_SHOTS=.run/shots/sheet-desktop] \
//!   cargo test -p kentos-desktop sheet_library::tests -- --ignored --nocapture
//! ```

use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use iced::Size;
use iced::futures::StreamExt as _;
use kentos_sheet::cloud::{TemplateGrantRole, TemplateRole};
use kentos_sheet_ui::gallery::{GalleryMessage, Source};
use kentos_sheet_ui::library::{Badge, TemplateAction};
use kentos_sheet_ui::save_template::SaveMessage;
use kentos_sheet_ui::share_template::{Listed, ShareMessage};
use kentos_sheet_ui::{Message as Sheet, Store};

use super::LibraryMsg;
use crate::app::{App, Message};
use crate::cloud::copy::Link;
use crate::files_testing::{drive, scratch};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is needed (see the module's comment)"))
}

/// The drawing on screen: the web's demo drawing when given (its `.kcad` v1 JSON), else the sample.
fn app_with_demo() -> App {
    let Ok(path) = std::env::var("KENTOS_DEMO_DRAWING") else {
        return crate::files_testing::app_with_drawing();
    };
    let text = std::fs::read_to_string(&path).expect("the demo drawing");
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(&text).expect("reads");
    let mut doc = crate::document::Document::new(snapshot, None).expect("opens");
    doc.model.set_name("Ornek_1244-1249_Ada.kcad");
    // As the web's pictures: the CAD mode, the drawing area on the drawing's home view.
    let mut settings = doc.settings().clone();
    settings.workspace = Some(kentos_contracts::Workspace::Cad);
    doc.model.set_settings(settings);
    let home = doc.model.home_view();
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    if let Some(h) = home {
        app.viewport.camera.center =
            kentos_render_wgpu::Vec2::new((h.min_x + h.max_x) / 2.0, (h.min_y + h.max_y) / 2.0);
    }
    app
}

/// A device of an account: the app signed in, its sheet store its own; the
/// first message starts its library (the cursor, then a sync).
fn device(login: &str, name: &str) -> App {
    let mut app = app_with_demo();
    app.sheet_store = Some(Store::open(&scratch(&format!("pafta-{name}"))).expect("a store"));
    let client = kentos_cloud::Cloud::new(&env("KENTOS_E2E_SERVER")).expect("the server's address");
    let me = iced::futures::executor::block_on(client.sign_in(login, &env("KENTOS_E2E_PASSWORD")))
        .expect("signs in");
    app.cloud.client = Some(client);
    app.cloud.me = Some(me);
    app.cloud.link = Link::Online;
    step(&mut app, Message::SheetLibrary(LibraryMsg::Tick));
    assert!(app.sheet_library.cursor.is_some(), "the library started");
    app
}

/// A message and every task it starts, to their end.
fn step(app: &mut App, m: Message) {
    let task = app.update(m);
    drive(app, task);
}

fn sheet(app: &mut App, m: Sheet) {
    step(app, Message::Sheet(m));
}

fn gallery(app: &mut App, m: GalleryMessage) {
    sheet(app, Sheet::Gallery(m));
}

fn cards(app: &App) -> Vec<(String, String, Vec<Badge>)> {
    app.sheets
        .gallery_cards()
        .into_iter()
        .map(|c| (c.id, c.name, c.badges))
        .collect()
}

fn said(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

/// The template kept on a device under `name`.
fn kept(app: &App, name: &str) -> kentos_sheet_ui::StoredTemplate {
    app.sheet_store
        .as_ref()
        .expect("a store")
        .templates()
        .into_iter()
        .find(|r| r.template.meta.name == name)
        .unwrap_or_else(|| panic!("“{name}” is not kept"))
}

/// The account's events waited for on the server, on a thread of their own: what it heard and when.
fn listen(app: &mut App) -> JoinHandle<(Instant, Vec<Message>)> {
    let task = app.library_listen();
    let stream = iced_runtime::task::into_stream(task);
    std::thread::spawn(move || {
        let Some(stream) = stream else {
            return (Instant::now(), Vec::new());
        };
        let heard: Vec<Message> = iced::futures::executor::block_on(stream.collect::<Vec<_>>())
            .into_iter()
            .filter_map(|a| match a {
                iced_runtime::Action::Output(m) => Some(m),
                _ => None,
            })
            .collect();
        (Instant::now(), heard)
    })
}

/// What the wait heard goes to the app; the sync it asks for runs (the settle time passed).
/// When the wait answered.
fn hear(app: &mut App, waiting: JoinHandle<(Instant, Vec<Message>)>) -> Instant {
    let (at, heard) = waiting.join().expect("the wait");
    assert!(!heard.is_empty(), "the wait answered");
    for m in heard {
        step(app, m);
    }
    assert!(
        app.sheet_library.settle_at.is_some(),
        "an event asks for a sync"
    );
    // The next wait is not started here (it would wait 25 s); the sync is.
    app.sheet_library.listen_at = None;
    let task = app.library_tick(Instant::now() + Duration::from_secs(1));
    drive(app, task);
    at
}

/// Pictures of the app as it stands, light (Pafta) and dark (Grafit), 1440 and 1100 wide, when asked for.
fn shots(app: &mut App, scene: &str) {
    let Ok(dir) = std::env::var("KENTOS_E2E_SHOTS") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        for (w, h) in [(1440.0, 900.0), (1100.0, 720.0)] {
            let mut snapshot =
                kentos_ui::snapshot::Snapshot::software(Size::new(w, h)).expect("a renderer");
            // Only what draws: the tasks of what the view says are not run.
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(app, App::view, &mut update);
            let file = dir.join(format!("masaustu-{scene}-{tag}-{}.png", w as u32));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
    let _ = app
        .settings
        .choose(&[("appearance.theme", serde_json::Value::from("light"))]);
    app.apply_settings();
}

/// An account's library emptied on the server (a run before may have left templates there):
/// its own, and those it published or administers in its organisations (on their routes).
fn empty_library(login: &str) {
    use kentos_cloud::sheet_templates as api;
    let client = kentos_cloud::Cloud::new(&env("KENTOS_E2E_SERVER")).expect("the server's address");
    use iced::futures::executor::block_on;
    let me = block_on(client.sign_in(login, &env("KENTOS_E2E_PASSWORD"))).expect("signs in");
    let space = kentos_cloud::sheet_library::personal_of(&me).expect("a personal space");
    let list = block_on(api::list(&client)).expect("the list");
    for t in list.every() {
        let route = match &t.organization {
            Some(o) if matches!(t.role, TemplateRole::Owner | TemplateRole::Admin) => {
                kentos_domain::Uuid::parse_str(&o.tenant_id).expect("an organisation")
            }
            None if t.role == TemplateRole::Owner => space,
            _ => continue,
        };
        let id = kentos_domain::Uuid::parse_str(&t.id).expect("an id");
        block_on(api::delete(
            &client,
            route,
            id,
            None,
            kentos_domain::Uuid::now_v7(),
        ))
        .expect("deleted");
    }
}

#[test]
#[ignore = "needs a running API and its accounts; run by hand"]
fn a_template_made_on_the_desktop_reaches_another_account_and_another_device() {
    empty_library("ayse");
    empty_library("bora");
    let mut a1 = device("ayse", "ayse-1");
    let mut b = device("bora", "bora");

    // Ayşe makes an ifraz sheet from the system template and saves it as her own template.
    gallery(&mut a1, GalleryMessage::Open);
    gallery(&mut a1, GalleryMessage::Pick("sys:ifraz-paftasi".into()));
    gallery(&mut a1, GalleryMessage::Use);
    // A drawing without attributes is asked about first (the parcels' table needs them) and
    // used anyway; the demo drawing has them, and the gallery over it knows.
    if a1.sheets.gallery_asking().is_some() {
        assert!(
            std::env::var_os("KENTOS_DEMO_DRAWING").is_none(),
            "the demo drawing's parcels have attributes: nothing to ask"
        );
        gallery(&mut a1, GalleryMessage::Answer(true));
    }
    // The template's questions (design §12), answered with its own values.
    if a1.sheets.questions().is_some() {
        sheet(
            &mut a1,
            Sheet::Questions(kentos_sheet_ui::questions::QuestionsMessage::Make),
        );
    }
    assert!(a1.sheets.is_active(), "the ifraz sheet in front");
    sheet(&mut a1, Sheet::SaveTemplate(SaveMessage::Open));
    sheet(
        &mut a1,
        Sheet::SaveTemplate(SaveMessage::Name("Belediye ifraz paftası".into())),
    );
    sheet(
        &mut a1,
        Sheet::SaveTemplate(SaveMessage::Category("kadastro".into())),
    );
    sheet(&mut a1, Sheet::SaveTemplate(SaveMessage::Save));
    let local = kept(&a1, "Belediye ifraz paftası");
    assert!(local.cloud.is_none(), "this device's only, first");

    // Buluta eşitle: it goes up, the cloud gives it its id.
    gallery(&mut a1, GalleryMessage::Open);
    gallery(&mut a1, GalleryMessage::Source(Source::Mine));
    gallery(&mut a1, GalleryMessage::Pick(local.id.clone()));
    gallery(&mut a1, GalleryMessage::Act(TemplateAction::Sync));
    let up = kept(&a1, "Belediye ifraz paftası");
    let c = up.cloud.clone().expect("in Ayşe's library");
    assert_ne!(up.id, local.id);
    assert_eq!(
        (c.revision, c.changed, c.role),
        (1, false, TemplateRole::Owner)
    );
    assert!(
        said(&a1).iter().any(|s| s.contains("buluta eşitlendi")),
        "{:?}",
        said(&a1)
    );
    let id = up.id.clone();
    assert!(
        cards(&a1).contains(&(
            id.clone(),
            "Belediye ifraz paftası".into(),
            vec![Badge::Synced]
        )),
        "{:?}",
        cards(&a1)
    );

    // Bora's device waits for his account's events; Ayşe shares it with him as an editor
    // from Şablonu paylaş, finding him by his name.
    let waiting = listen(&mut b);
    std::thread::sleep(Duration::from_millis(500));
    gallery(&mut a1, GalleryMessage::Pick(id.clone()));
    gallery(&mut a1, GalleryMessage::Act(TemplateAction::Share));
    let w = a1.sheets.sharing().expect("the window");
    assert_eq!(w.reason, None);
    assert!(matches!(w.access, Listed::Ready(_)), "{:?}", w.access);
    sheet(&mut a1, Sheet::Share(ShareMessage::Query("bora".into())));
    let found = a1.sheets.sharing().and_then(|w| w.found.clone());
    assert!(
        matches!(&found, Some((_, Listed::Ready(list))) if list.iter().any(|c| c.display_name == "Bora Tan")),
        "{found:?}"
    );
    sheet(&mut a1, Sheet::Share(ShareMessage::Pick(0)));
    sheet(
        &mut a1,
        Sheet::Share(ShareMessage::Role(TemplateGrantRole::Editor)),
    );
    let shared_at = Instant::now();
    sheet(&mut a1, Sheet::Share(ShareMessage::Add));
    let w = a1.sheets.sharing().expect("the window");
    assert_eq!(
        w.said,
        Some((true, "Bora Tan artık bu şablonu düzenleyebilir.".to_owned()))
    );
    assert!(matches!(&w.access, Listed::Ready(a) if a.grants.len() == 1));
    shots(&mut a1, "bulut-paylas");
    sheet(&mut a1, Sheet::Share(ShareMessage::Close));
    // The sync after the share: Ayşe's copy says it is shared.
    assert!(
        cards(&a1).contains(&(
            id.clone(),
            "Belediye ifraz paftası".into(),
            vec![Badge::Synced, Badge::Shared]
        )),
        "{:?}",
        cards(&a1)
    );
    shots(&mut a1, "bulut-esitlendi");

    // Bora's wait heard the share; his sync lists it under “Benimle paylaşılanlar”.
    let heard = hear(&mut b, waiting).duration_since(shared_at);
    println!("Bora duydu: paylaşımdan {heard:?} sonra");
    assert!(heard < Duration::from_secs(3));
    gallery(&mut b, GalleryMessage::Open);
    gallery(&mut b, GalleryMessage::Source(Source::Shared));
    assert_eq!(
        cards(&b),
        [(
            id.clone(),
            "Belediye ifraz paftası".into(),
            vec![Badge::Editor, Badge::Synced]
        )]
    );
    gallery(&mut b, GalleryMessage::Pick(id.clone()));
    shots(&mut b, "bulut-benimle-paylasilanlar");

    // Ayşe's second device has it; it and Bora's wait while her first device edits it.
    let mut a2 = device("ayse", "ayse-2");
    assert_eq!(kept(&a2, "Belediye ifraz paftası").id, id);
    let (waiting_a2, waiting_b) = (listen(&mut a2), listen(&mut b));
    std::thread::sleep(Duration::from_millis(500));
    gallery(&mut a1, GalleryMessage::Open);
    gallery(&mut a1, GalleryMessage::Source(Source::Mine));
    gallery(&mut a1, GalleryMessage::Pick(id.clone()));
    gallery(&mut a1, GalleryMessage::Act(TemplateAction::Edit));
    sheet(&mut a1, Sheet::SaveTemplate(SaveMessage::Open));
    sheet(
        &mut a1,
        Sheet::SaveTemplate(SaveMessage::Description(
            "Kızılırmak mahallesi, 1244–1249 ada".into(),
        )),
    );
    let sent = Instant::now();
    sheet(&mut a1, Sheet::SaveTemplate(SaveMessage::Save));
    assert_eq!(
        kept(&a1, "Belediye ifraz paftası")
            .cloud
            .map(|c| c.revision),
        Some(2)
    );
    let heard = hear(&mut a2, waiting_a2).duration_since(sent);
    println!("İkinci cihaz duydu: kayıttan {heard:?} sonra");
    assert!(heard < Duration::from_secs(3));
    hear(&mut b, waiting_b);
    for app in [&a2, &b] {
        let there = kept(app, "Belediye ifraz paftası");
        assert_eq!(
            (
                there.cloud.map(|c| c.revision),
                there.template.meta.description.as_str()
            ),
            (Some(2), "Kızılırmak mahallesi, 1244–1249 ada")
        );
    }
    assert_eq!(
        kept(&b, "Belediye ifraz paftası").cloud.map(|c| c.role),
        Some(TemplateRole::Editor)
    );
}

/// Kurumum with two people of one organisation (design §13 “Kurum şablonları”): Ayşe (project
/// manager in Büro) copies a system template to her own, takes it to the cloud and publishes it
/// into Büro; Bora (editor in Büro) hears it and finds it under Kurumum with Büro's badge,
/// uses it and copies it to his own, and may neither edit nor delete it; Ayşe changes her own
/// template and updates Büro's copy from it; Bora's device follows.
#[test]
#[ignore = "needs a running API, its accounts and their organisation; run by hand"]
fn an_organisation_s_library_is_live_for_two_people() {
    use kentos_sheet_ui::publish_template::PublishMessage;
    empty_library("ayse");
    empty_library("bora");
    let mut a = device("ayse", "ayse-kurum");
    let mut b = device("bora", "bora-kurum");
    let organisation = a.sheet_library.organizations.clone();
    assert!(
        organisation
            .iter()
            .any(|o| o.name == "Harita Bürosu" && o.can_publish),
        "Ayşe publishes into Büro: {organisation:?}"
    );
    assert!(
        b.sheet_library
            .organizations
            .iter()
            .any(|o| o.name == "Harita Bürosu" && !o.can_publish),
        "Bora is a member who does not publish"
    );

    // Ayşe: the system ifraz template copied to her own, then to the cloud.
    gallery(&mut a, GalleryMessage::Open);
    gallery(&mut a, GalleryMessage::Pick("sys:ifraz-paftasi".into()));
    gallery(&mut a, GalleryMessage::Act(TemplateAction::Duplicate));
    let name = "İfraz / tevhit paftası (kopya)";
    let local = kept(&a, name);
    gallery(&mut a, GalleryMessage::Pick(local.id.clone()));
    gallery(&mut a, GalleryMessage::Act(TemplateAction::Sync));
    let own = kept(&a, name);
    assert!(
        own.cloud
            .as_ref()
            .is_some_and(|c| c.revision == 1 && !c.changed)
    );

    // Ayşe publishes it into Büro from “Kuruma yayımla…” while Bora's device waits (its wait
    // starts after the pictures: one wait lasts 25 s).
    gallery(&mut a, GalleryMessage::Source(Source::Mine));
    gallery(&mut a, GalleryMessage::Pick(own.id.clone()));
    gallery(&mut a, GalleryMessage::Act(TemplateAction::Publish));
    let w = a.sheets.publishing().expect("the window");
    assert_eq!(
        w.targets
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["Harita Bürosu"]
    );
    shots(&mut a, "c7-kuruma-yayimla");
    let waiting = listen(&mut b);
    std::thread::sleep(Duration::from_millis(500));
    let published_at = Instant::now();
    sheet(&mut a, Sheet::Publish(PublishMessage::Publish));
    assert!(
        a.sheets.publishing().is_none(),
        "published: the window closed"
    );
    assert!(
        said(&a)
            .iter()
            .any(|s| s.contains("“Harita Bürosu” kurumunda yayımlandı")),
        "{:?}",
        said(&a)
    );
    // The sync after it brought Büro's copy: Kurumum, with Büro's badge, hers.
    let ours = a
        .sheet_store
        .as_ref()
        .expect("a store")
        .templates()
        .into_iter()
        .find(|r| {
            r.cloud
                .as_ref()
                .is_some_and(|c| c.published_from.as_deref() == Some(own.id.as_str()))
        })
        .expect("Büro's copy on Ayşe's device");
    assert_eq!(
        ours.cloud
            .as_ref()
            .and_then(|c| c.organization.as_ref())
            .map(|o| o.name.as_str()),
        Some("Harita Bürosu")
    );
    assert_eq!(
        cards(&a),
        [(
            ours.id.clone(),
            name.to_owned(),
            vec![Badge::Organisation, Badge::Published, Badge::Synced]
        )]
    );
    shots(&mut a, "c7-kurumum-ayse");

    // Bora heard it; under Kurumum it is Büro's, his to use and copy.
    let heard = hear(&mut b, waiting).duration_since(published_at);
    println!("Bora duydu: yayımdan {heard:?} sonra");
    assert!(heard < Duration::from_secs(3));
    gallery(&mut b, GalleryMessage::Open);
    gallery(&mut b, GalleryMessage::Source(Source::Organisation));
    assert_eq!(
        cards(&b),
        [(
            ours.id.clone(),
            name.to_owned(),
            vec![Badge::Organisation, Badge::Synced]
        )]
    );
    gallery(&mut b, GalleryMessage::Pick(ours.id.clone()));
    shots(&mut b, "c7-kurumum-bora");
    gallery(&mut b, GalleryMessage::Act(TemplateAction::Delete));
    assert_eq!(
        b.sheets.gallery_note(),
        Some(kentos_sheet_ui::library::texts::ORGANISATION_NO_DELETE)
    );
    gallery(&mut b, GalleryMessage::Act(TemplateAction::Duplicate));
    let copy = kept(&b, &format!("{name} (kopya)"));
    assert!(copy.cloud.is_none(), "Bora's own, on his device");

    // Ayşe changes her own and updates Büro's copy from it; Bora's device follows.
    let mut changed = kept(&a, name);
    changed.template.meta.description = "Büronun ifraz paftası, 2026".into();
    let mut c = changed.cloud.clone().expect("in the cloud");
    c.changed = true;
    a.sheet_store
        .as_ref()
        .expect("a store")
        .save_template(&changed.id, &changed.template, Some(&c))
        .expect("kept");
    // Her change goes up first (the next run), then Büro's copy takes it.
    let task = a.library_run();
    drive(&mut a, task);
    assert_eq!(kept(&a, name).cloud.map(|c| c.revision), Some(2));
    gallery(&mut a, GalleryMessage::Source(Source::Mine));
    gallery(&mut a, GalleryMessage::Pick(own.id.clone()));
    gallery(&mut a, GalleryMessage::Act(TemplateAction::Publish));
    let w = a.sheets.publishing().expect("the window");
    assert_eq!(
        w.targets[0].copy.as_ref().map(|c| c.0.as_str()),
        Some(ours.id.as_str()),
        "Büro has its copy: “Kurumdakini güncelle”"
    );
    shots(&mut a, "c7-kurumdakini-guncelle");
    let waiting = listen(&mut b);
    std::thread::sleep(Duration::from_millis(500));
    sheet(&mut a, Sheet::Publish(PublishMessage::Update));
    let there = a
        .sheet_store
        .as_ref()
        .expect("a store")
        .template(&ours.id)
        .expect("Büro's copy");
    assert_eq!(
        (
            there.cloud.as_ref().map(|c| (c.revision, c.changed)),
            there.template.meta.description.as_str()
        ),
        (Some((2, false)), "Büronun ifraz paftası, 2026")
    );
    hear(&mut b, waiting);
    let bora_sees = b
        .sheet_store
        .as_ref()
        .expect("a store")
        .template(&ours.id)
        .expect("Büro's copy on Bora's device");
    assert_eq!(
        (
            bora_sees.cloud.as_ref().map(|c| (c.revision, c.role)),
            bora_sees.template.meta.description.as_str()
        ),
        (
            Some((2, TemplateRole::Viewer)),
            "Büronun ifraz paftası, 2026"
        )
    );
    gallery(&mut b, GalleryMessage::Open);
    gallery(&mut b, GalleryMessage::Source(Source::Organisation));
    gallery(&mut b, GalleryMessage::Pick(ours.id.clone()));
    shots(&mut b, "c7-kurumum-bora-guncel");
}

/// Without the server the library keeps to itself: the copies stay, the
/// gallery says it is offline, a retry waits, and the rest of the cloud's
/// idea of the connection is left alone; signed out in this session, the
/// account's copies are hidden.
#[test]
fn without_the_server_the_library_waits_and_says_so() {
    let mut app = crate::cloud::tests::signed_in();
    app.sheet_store = Some(Store::open(&scratch("pafta-cevrimdisi")).expect("a store"));
    let account = app
        .cloud
        .me
        .as_ref()
        .map(|m| m.user.id.clone())
        .expect("an account");
    step(&mut app, Message::SheetLibrary(LibraryMsg::Tick));
    assert_eq!(
        app.sheet_library.status,
        kentos_sheet_ui::SyncStatus::Offline
    );
    assert!(app.sheet_library.retry_at.is_some(), "a retry waits");
    assert!(app.sheet_library.wants_ticks());
    assert_eq!(
        app.cloud.link,
        Link::Online,
        "the cloud's connection is the cloud's"
    );
    let seen = app.sheets.library();
    assert_eq!(
        (seen.viewer.as_deref(), seen.status.clone()),
        (Some(account.as_str()), kentos_sheet_ui::SyncStatus::Offline)
    );
    assert_eq!(seen.line(), kentos_sheet_ui::library::texts::OFFLINE);
    // Signed out here: nothing more is asked, and the copies of the account are hidden.
    app.cloud.me = None;
    step(&mut app, Message::SheetLibrary(LibraryMsg::Tick));
    assert!(!app.sheet_library.wants_ticks());
    assert_eq!(app.sheets.library().viewer, None);
    assert_eq!(
        app.sheets.library().status,
        kentos_sheet_ui::SyncStatus::SignedOut
    );
}
