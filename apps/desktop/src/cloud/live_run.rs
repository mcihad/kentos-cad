//! The desktop's cloud interface against a real KentOS server (docs/adr/0041),
//! as a person would use it and as the web's cloud e2e plays its second
//! editor (apps/web/scripts/e2e/cloud.mjs): `kentosd` runs on the development
//! database under this test's control (it is stopped to take the connection
//! away), the app runs its own tasks on threads the way Iced would, a clock
//! ticks, and the windows are drawn into `.run/shots/` with KentOS's own
//! renderer.
//!
//! `apps/desktop/scripts/cloud-live.sh` builds `kentosd` and runs this test
//! with `KENTOS_DEV_PASSWORD` from `.env.local` (never printed). Another
//! editor, `mehmet`, speaks the web's HTTP (`apps/desktop/scripts/web-client.mjs`).
//! Copies and drafts go to `.run/cloud-live/`, never to the user's folders.
//! Projects it makes stay in the development database, named “Masaüstü canlı …”.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use iced::futures::StreamExt as _;
use iced::futures::executor::block_on;
use iced::{Size, Task};
use kentos_cloud::SaveState;
use kentos_contracts::{Entity, ProjectStorage};
use kentos_domain::{Slot, Uuid};
use kentos_ui::snapshot::Snapshot;
use kentos_ui::widget::command_line::Entry;

use crate::app::{App, Dialog, Message};
use crate::cloud::{Event, TICK, msg};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_dir() -> PathBuf {
    repo().join(".run")
}

/// `kentosd serve` on one port, started and stopped by the test.
struct Server {
    port: u16,
    child: Option<Child>,
}

impl Server {
    fn new() -> Self {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .expect("a free port");
        let mut server = Self { port, child: None };
        server.start();
        server
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn start(&mut self) {
        let log = std::fs::File::create(run_dir().join("cloud-live-api.log")).expect("a log");
        let child = Command::new(repo().join("target/debug/kentosd"))
            .arg("serve")
            .current_dir(repo())
            .env("KENTOS_API_PORT", self.port.to_string())
            .env("KENTOS_LOG", "warn")
            .stdin(Stdio::null())
            .stdout(log.try_clone().expect("the log"))
            .stderr(log)
            .spawn()
            .expect("kentosd starts (cargo build -p kentos-api)");
        self.child = Some(child);
        let probe = kentos_cloud::Cloud::new(&self.url()).expect("a local address");
        let end = Instant::now() + Duration::from_secs(30);
        while block_on(probe.auth_config()).is_err() {
            assert!(Instant::now() < end, "kentosd did not answer");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// The connection taken away: the server is gone.
    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The app with its tasks run on threads, as Iced's executor would run them.
struct Runner {
    app: App,
    snapshot: Snapshot,
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
    next_tick: Instant,
}

impl Runner {
    fn new(app: App) -> Self {
        let (tx, rx) = mpsc::channel();
        let snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut runner = Self {
            app,
            snapshot,
            tx,
            rx,
            next_tick: Instant::now(),
        };
        runner.settle();
        runner
    }

    fn spawn(&self, task: Task<Message>) {
        let Some(mut stream) = iced_runtime::task::into_stream(task) else {
            return;
        };
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            while let Some(action) = block_on(stream.next()) {
                if let iced_runtime::Action::Output(message) = action
                    && tx.send(message).is_err()
                {
                    break;
                }
            }
        });
    }

    fn send(&mut self, message: Message) {
        let task = self.app.update(message);
        self.spawn(task);
    }

    fn cloud(&mut self, event: Event) {
        self.send(msg(event));
    }

    /// Runs the app until `done`, answering its tasks and ticking its clock.
    fn until(&mut self, what: &str, limit: Duration, done: impl Fn(&App) -> bool) {
        let end = Instant::now() + limit;
        loop {
            if done(&self.app) {
                return;
            }
            assert!(
                Instant::now() < end,
                "{what}: {} s içinde olmadı. Son iletiler: {:?}",
                limit.as_secs(),
                said(&self.app).iter().rev().take(6).collect::<Vec<_>>()
            );
            if let Ok(message) = self.rx.recv_timeout(Duration::from_millis(20)) {
                self.send(message);
            }
            if Instant::now() >= self.next_tick {
                self.next_tick = Instant::now() + TICK;
                self.cloud(Event::Tick);
            }
        }
    }

    /// Runs the app for a while (answers arriving, the clock ticking).
    fn run_for(&mut self, time: Duration) {
        let end = Instant::now() + time;
        self.until("süre", time + Duration::from_secs(1), |_| {
            Instant::now() >= end
        });
    }

    fn settle(&mut self) {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        self.snapshot.settle(&mut self.app, App::view, &mut update);
    }

    /// The window as it is now, into `.run/shots/bulut-<name>.png`.
    fn shot(&mut self, name: &str) {
        self.settle();
        let _ = self.snapshot.render(self.app.view(), &self.app.theme());
        self.app.sync_device();
        let path = run_dir().join("shots").join(format!("bulut-{name}.png"));
        self.snapshot
            .render(self.app.view(), &self.app.theme())
            .save(&path)
            .expect("the image is written");
        println!("görüntü: {}", path.display());
    }
}

fn said(app: &App) -> Vec<String> {
    app.history
        .iter()
        .map(|e| match e {
            Entry::Input(t) | Entry::Value(t) | Entry::Output(t) | Entry::Warning(t) | Entry::Error(t) => t.clone(),
        })
        .collect()
}

/// The other editor (the web's HTTP): one JSON line back.
fn web(server: &Server, args: &[&str]) -> serde_json::Value {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/web-client.mjs");
    let out = Command::new("node")
        .arg(script)
        .args(args)
        .env("KENTOS_LIVE_API", server.url())
        .output()
        .expect("node runs");
    assert!(
        out.status.success(),
        "web istemcisi: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("one JSON line")
}

/// A new app with this run's folders for copies and drafts, and `server` as its address.
fn app(dir: &Path, server: &str) -> App {
    let (mut app, _) = App::boot(None);
    let _ = app
        .settings
        .choose(&[("cloud.server", serde_json::Value::from(server))]);
    app.cloud.replicas = Some(kentos_cloud::ReplicaStore::new(dir.join("kopya")));
    app.cloud.drafts = Some(kentos_cloud::DraftStore::new(dir.join("taslak")));
    app
}

fn sign_in(r: &mut Runner, password: &str, shot: Option<&str>) {
    r.send(Message::Run("cloud.signIn"));
    r.cloud(Event::SignInLogin("ayse".into()));
    r.cloud(Event::SignInPassword(password.into()));
    if let Some(name) = shot {
        r.shot(name);
    }
    r.cloud(Event::SignInSubmit);
    r.until("giriş", Duration::from_secs(15), |a| a.cloud.me.is_some());
}

/// Opens `project` from the catalog (the list `list` shows it).
fn open_from_catalog(r: &mut Runner, project: &str, shot: Option<&str>) {
    r.send(Message::Run("cloud.open"));
    r.until("katalog", Duration::from_secs(10), |a| {
        a.cloud.catalog.as_ref().is_some_and(|c| !c.loading())
    });
    r.cloud(Event::CatalogPick(project.to_owned()));
    if let Some(name) = shot {
        r.shot(name);
    }
    let before = r.app.document.as_ref().map(|d| d.session);
    r.cloud(Event::CatalogOpen);
    r.until("katalogdan açılış", Duration::from_secs(30), |a| {
        a.cloud.opening.is_none() && a.document.as_ref().map(|d| d.session) != before
    });
}

/// The first point of the drawing (the sample's P1): its persistent id.
fn the_point(app: &App) -> Uuid {
    let doc = app.document.as_ref().expect("open");
    let e = doc
        .model
        .entities()
        .find(|e| matches!(e, Entity::Point(_)))
        .expect("a point");
    doc.model.uid(Slot(e.base().id)).expect("an id")
}

fn point_x(app: &App, uid: Uuid) -> f64 {
    let doc = app.document.as_ref().expect("open");
    match doc.model.slot_of(uid).and_then(|s| doc.model.get(s)) {
        Some(Entity::Point(p)) => p.p.x,
        other => panic!("{other:?}"),
    }
}

/// Moves the point by `dx`, as a tool's edit does.
fn move_point(r: &mut Runner, uid: Uuid, dx: f64) {
    let doc = r.app.document.as_mut().expect("open");
    let slot = doc.model.slot_of(uid).expect("there");
    let mut e = doc.model.get(slot).expect("there").clone();
    if let Entity::Point(p) = &mut e {
        p.p.x += dx;
    }
    assert!(doc.model.update(slot, e));
    r.send(Message::Modifiers(iced::keyboard::Modifiers::default()));
}

fn state(app: &App) -> Option<SaveState> {
    app.cloud.live.as_ref().map(|l| l.sync.state())
}

fn saved(app: &App) -> bool {
    app.cloud.live.as_ref().is_some_and(|l| l.sync.all_sent())
        && state(app) == Some(SaveState::Saved)
}

fn ids(app: &App) -> (String, String) {
    let s = app
        .document
        .as_ref()
        .and_then(|d| d.cloud_source())
        .expect("a cloud project");
    (s.tenant.to_string(), s.project.to_string())
}

/// Uploads the sample drawing as a new project kept `storage`-wise; it opens from the server.
fn upload(r: &mut Runner, name: &str, storage: ProjectStorage, shot: Option<&str>) {
    let mut doc = crate::files_testing::drawing(0);
    // The sample's “Çizim” layer is hidden; the point the run moves is on it.
    doc.model.set_layer_visible("cizim", true);
    let revision = doc.model.revision();
    doc.model.mark_saved(revision);
    r.send(Message::Opened(Some(Ok(Box::new(doc)))));
    r.send(Message::Run("cloud.upload"));
    r.cloud(Event::UploadName(name.into()));
    r.cloud(Event::UploadStorage(storage));
    if let Some(shot) = shot {
        r.shot(shot);
    }
    r.cloud(Event::UploadSubmit);
    r.until("yükleme ve açılış", Duration::from_secs(60), |a| {
        a.cloud.opening.is_none()
            && a.document
                .as_ref()
                .and_then(|d| d.cloud_source())
                .is_some_and(|s| s.info.name == name)
    });
}

/// The open project shared with `user` as an editor (ayşe's session).
fn share(r: &Runner, user: &str) {
    let (tenant, project) = ids(&r.app);
    let client = r.app.cloud.client.clone().expect("signed in");
    let envelope = kentos_cloud::saving::envelope(
        Uuid::parse_str(&tenant).expect("id"),
        Uuid::parse_str(&project).expect("id"),
        "project.share",
        1,
        Uuid::new_v4(),
        Default::default(),
        serde_json::json!({ "userId": user, "role": "editor" }),
    );
    block_on(client.command::<serde_json::Value>(envelope)).expect("shared with the other editor");
}

#[test]
#[ignore = "a real KentOS server: apps/desktop/scripts/cloud-live.sh"]
fn cloud_live() {
    let password = std::env::var("KENTOS_DEV_PASSWORD")
        .expect("KENTOS_DEV_PASSWORD (apps/desktop/scripts/cloud-live.sh)");
    let mut server = Server::new();
    let api = server.url();
    let dir = run_dir().join("cloud-live");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(run_dir().join("shots")).expect("shots");
    let stamp = crate::settings::rfc3339(std::time::SystemTime::now());
    let stamp = &stamp[..19];
    let mehmet = web(&server, &["whoami", "mehmet"])["id"]
        .as_str()
        .expect("his id")
        .to_owned();

    // 1. Signing in.
    let mut r = Runner::new(app(&dir, &api));
    sign_in(&mut r, &password, Some("01-giris"));
    let ayse = r
        .app
        .cloud
        .me
        .as_ref()
        .map(|m| m.user.id.clone())
        .expect("signed in");
    r.shot("02-giris-yapildi");

    // 2. The sample drawing becomes a database project, opened from the server.
    let db = format!("Masaüstü canlı {stamp}");
    upload(
        &mut r,
        &db,
        ProjectStorage::Database,
        Some("03-buluta-yukle"),
    );
    share(&r, &mehmet);
    let (tenant, project) = ids(&r.app);
    let uid = the_point(&r.app);

    // 3. The catalog lists it; opened again from there.
    open_from_catalog(&mut r, &project, Some("04-katalog"));
    r.run_for(Duration::from_millis(300));
    r.shot("05-veritabani-projesi-acildi");

    // 4. An edit reaches Kaydedildi.
    move_point(&mut r, uid, 3.0);
    r.shot("06-kaydedilmedi");
    r.until("kayıt", Duration::from_secs(15), saved);
    r.shot("07-kaydedildi");

    // 5. The other editor's change (the web's HTTP) arrives by the long poll.
    let x = point_x(&r.app, uid);
    let started = Instant::now();
    web(
        &server,
        &[
            "move-point",
            "mehmet",
            &tenant,
            &project,
            &uid.to_string(),
            "25",
        ],
    );
    r.until("web düzenlemesi", Duration::from_secs(15), |a| {
        (point_x(a, uid) - (x + 25.0)).abs() < 1e-6
    });
    println!(
        "web düzenlemesi {} ms'de geldi",
        started.elapsed().as_millis()
    );
    r.shot("08-web-duzenlemesi-geldi");

    // 6. Both change the point: the conflict window.
    move_point(&mut r, uid, 1.0);
    web(
        &server,
        &[
            "move-point",
            "mehmet",
            &tenant,
            &project,
            &uid.to_string(),
            "5",
        ],
    );
    r.until("çakışma", Duration::from_secs(20), |a| {
        a.cloud
            .live
            .as_ref()
            .is_some_and(|l| !l.sync.conflicts().is_empty())
    });
    r.send(Message::Run("cloud.conflicts"));
    assert_eq!(r.app.dialog, Some(Dialog::Conflicts));
    r.shot("09-cakisma-penceresi");
    r.cloud(Event::KeepMine);
    r.until("benimkini koru", Duration::from_secs(15), saved);

    // 7. The server goes away while signed in: Bağlantı yok, then it comes back.
    server.stop();
    move_point(&mut r, uid, 0.5);
    r.until("bağlantı yok", Duration::from_secs(20), |a| {
        state(a) == Some(SaveState::Offline)
    });
    r.shot("10-baglanti-yok");
    server.start();
    r.until("bağlantı dönünce kayıt", Duration::from_secs(60), saved);
    r.shot("11-baglanti-dondu");

    // 8. A forced quit while Kaydedilmedi shows: the draft was written, nothing sent.
    move_point(&mut r, uid, 7.0);
    r.run_for(Duration::from_millis(500));
    assert_eq!(state(&r.app), Some(SaveState::Pending));
    r.shot("12-kaydedilmedi-zorla-kapatmadan-once");
    let unsent = point_x(&r.app, uid);
    // As a killed process: no closing path runs; its locks go with it.
    drop(r);
    // Meanwhile the other editor moves the same point.
    web(
        &server,
        &[
            "move-point",
            "mehmet",
            &tenant,
            &project,
            &uid.to_string(),
            "11",
        ],
    );

    // 9. Opened again: the draft goes back in; its base moved: a conflict.
    let mut r = Runner::new(app(&dir, &api));
    sign_in(&mut r, &password, None);
    open_from_catalog(&mut r, &project, None);
    assert!(said(&r.app).iter().any(|t| t.contains("çizime geri kondu")));
    assert!(
        (point_x(&r.app, uid) - unsent).abs() < 1e-6,
        "this device's work is back"
    );
    r.shot("13-taslak-geri-kondu");
    r.send(Message::Run("cloud.conflicts"));
    assert_eq!(r.app.dialog, Some(Dialog::Conflicts));
    r.shot("14-taslaktan-cakisma");
    r.cloud(Event::KeepMine);
    r.until("taslağın gönderilmesi", Duration::from_secs(15), saved);
    r.send(Message::CloseRequested(iced::window::Id::unique()));
    drop(r);

    // 10. No server at all: this device's projects open, the work waits.
    server.stop();
    let mut offline = app(&dir, &api);
    let _ = offline
        .settings
        .choose(&[("cloud.account", serde_json::Value::from(ayse.as_str()))]);
    let mut r = Runner::new(offline);
    r.send(Message::Run("cloud.open"));
    assert_eq!(r.app.dialog, Some(Dialog::Catalog));
    r.cloud(Event::CatalogPick(project.clone()));
    r.shot("15-bu-cihazdaki-projeler");
    r.cloud(Event::CatalogOpen);
    r.until("çevrimdışı açılış", Duration::from_secs(20), |a| {
        a.cloud.opening.is_none() && a.document.is_some()
    });
    move_point(&mut r, uid, 2.0);
    r.run_for(Duration::from_millis(600));
    let offline_x = point_x(&r.app, uid);
    r.shot("16-cevrimdisi");
    r.send(Message::CloseRequested(iced::window::Id::unique()));
    r.run_for(Duration::from_millis(300));
    drop(r);

    // 11. The server is back: signed in, opened, the offline work goes; the server has it.
    server.start();
    let mut r = Runner::new(app(&dir, &api));
    sign_in(&mut r, &password, None);
    open_from_catalog(&mut r, &project, None);
    r.until(
        "çevrimdışı işin gönderilmesi",
        Duration::from_secs(30),
        saved,
    );
    assert!((point_x(&r.app, uid) - offline_x).abs() < 1e-6);
    r.shot("17-cevrimdisi-is-gitti");
    let client = r.app.cloud.client.clone().expect("signed in");
    let fresh = block_on(kentos_cloud::open(
        &client,
        Uuid::parse_str(&tenant).expect("id"),
        Uuid::parse_str(&project).expect("id"),
        None,
    ))
    .expect("opened");
    let server_x = match fresh
        .document
        .slot_of(uid)
        .and_then(|s| fresh.document.get(s))
    {
        Some(Entity::Point(p)) => p.p.x,
        other => panic!("{other:?}"),
    };
    assert!(
        (server_x - offline_x).abs() < 1e-6,
        "the server has the offline work"
    );

    // 12. A file project: Kaydet makes revision 2; the other editor's save first makes a conflict.
    let file = format!("Masaüstü canlı dosya {stamp}");
    upload(&mut r, &file, ProjectStorage::File, None);
    share(&r, &mehmet);
    let uid = the_point(&r.app);
    move_point(&mut r, uid, 4.0);
    r.send(Message::Run("file.save"));
    let revision = |a: &App| {
        a.document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .and_then(|s| s.revision.as_ref())
            .map(|rev| rev.number)
    };
    r.until("dosya kaydı", Duration::from_secs(30), |a| {
        a.saving.is_none() && revision(a) == Some(2)
    });
    r.shot("18-dosya-kaydedildi");
    let (tenant, project) = ids(&r.app);
    web(&server, &["save-file", "mehmet", &tenant, &project]);
    move_point(&mut r, uid, 4.0);
    r.send(Message::Run("file.save"));
    r.until("dosya çakışması", Duration::from_secs(30), |a| {
        a.cloud.file_conflict.is_some()
    });
    assert_eq!(r.app.dialog, Some(Dialog::FileConflict));
    r.shot("19-dosya-cakismasi");
    println!("{}", said(&r.app).join("\n"));
}
