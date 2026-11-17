//! Kataloğa gönder (submit.rs, docs/adr/8057 §107): an extension installed from its working folder is packed from it,
//! uploaded to the chosen catalog as a draft and sent to review with the user's note; without a session, or for one
//! installed from a file, the window says why. An agent opens the window, the user sends.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use kentos_commands::{Me, MembershipView, SignInMethod, TenantKind, TenantRole, UserView};

use super::*;
use crate::extensions::submit::SubmitEvent;
use crate::files_testing::drive;

const ALTLIK: &str = "tr.ornek.altlik";

/// What a request said: its head and its body.
type Seen = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

/// A catalog server answering `answers` in turn, on this computer.
fn catalog(answers: Vec<(u16, serde_json::Value)>) -> (String, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("http://{}", listener.local_addr().expect("its address"));
    let seen: Seen = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&seen);
    std::thread::spawn(move || {
        for (status, body) in answers {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
            let mut head = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim_end().is_empty() {
                    break;
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap_or(0);
                }
                head.push_str(&line);
            }
            let mut data = vec![0; length];
            let _ = reader.read_exact(&mut data);
            kept.lock().expect("kept").push((head, data));
            let body = body.to_string();
            let mut stream = stream;
            let _ = write!(
                stream,
                "HTTP/1.1 {status} Durum\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                 connection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (url, seen)
}

/// The account signed in: a personal catalog and an organisation's.
fn me() -> Me {
    Me {
        user: UserView {
            id: "ayse-1".into(),
            display_name: "Ayşe Yılmaz".into(),
            email: None,
            method: SignInMethod::Local,
        },
        memberships: vec![MembershipView {
            tenant_id: "t-1".into(),
            tenant_slug: "ornek-belediyesi".into(),
            tenant_name: "Örnek Belediyesi".into(),
            tenant_kind: TenantKind::Organization,
            role: TenantRole::Editor,
            seat: true,
            active: true,
            capabilities: Vec::new(),
        }],
    }
}

/// The example extension installed from a working folder of its own, as the workshop installs one.
fn developed(root: &Path) -> App {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/extensions/v1/ornek-altlik");
    let folder = root.join("gelistirme").join("ornek-altlik");
    std::fs::create_dir_all(&folder).unwrap();
    let files = kentos_extension_host::store::read_files(&fixture).unwrap();
    for (path, bytes) in &files {
        let to = folder.join(path);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::write(to, bytes).unwrap();
    }
    let package = root.join("ornek-altlik.kek");
    std::fs::write(
        &package,
        kentos_extensions::pack(&files, &Limits::default())
            .unwrap()
            .bytes,
    )
    .unwrap();
    let mut gis = app(&root.join("eklentiler"), Some(Module::Gis));
    gis.agent_review_workshop(&package, &folder, "Ajan")
        .expect("opened");
    let _ = gis.extensions_event(Event::Install);
    gis
}

#[test]
fn a_developed_extension_is_uploaded_and_sent_to_review() {
    let root = temp("katalog-gonder");
    let mut gis = developed(&root);
    // Without a session the window says so and sends nothing.
    let _ = gis.extensions_event(Event::Submit(SubmitEvent::Open(ALTLIK.into())));
    let _ = gis.extension_submit_window().expect("drawn");
    let _ = gis.extensions_event(Event::Submit(SubmitEvent::Send));
    let failed = gis
        .extensions
        .submit
        .as_ref()
        .and_then(|s| s.failed.clone());
    assert!(failed.is_some_and(|w| w.contains("oturum açın")));
    // Signed in: to the organisation's catalog, with a note.
    let draft = serde_json::json!({ "view": "insider", "release": { "state": "draft" } });
    let submitted = serde_json::json!({ "view": "insider", "release": { "state": "submitted" } });
    let (url, seen) = catalog(vec![(201, draft), (200, submitted)]);
    gis.cloud.client = Some(kentos_cloud::Cloud::new(&url).expect("a client"));
    gis.cloud.me = Some(me());
    let names: Vec<String> = gis.submit_catalogs().into_iter().map(|(_, w)| w).collect();
    assert_eq!(
        names,
        [
            "Kişisel kataloğum",
            "Örnek Belediyesi kurum kataloğu",
            "Genel katalog"
        ]
    );
    let _ = gis.extensions_event(Event::Submit(SubmitEvent::Catalog(1)));
    let _ = gis.extensions_event(Event::Submit(SubmitEvent::Note("İlk sürüm.".into())));
    let _ = gis.extension_submit_window().expect("drawn");
    let task = gis.extensions_event(Event::Submit(SubmitEvent::Send));
    drive(&mut gis, task);
    assert!(gis.extensions.submit.is_none(), "closed once sent");
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(
        seen[0]
            .0
            .starts_with("POST /v1/extension-catalogs/tenant:ornek-belediyesi/releases "),
        "{}",
        seen[0].0
    );
    let sent = kentos_extensions::inspect(&seen[0].1, &Limits::default());
    assert_eq!(
        sent.manifest.map(|m| m.id),
        Some(ALTLIK.to_owned()),
        "the folder, packed"
    );
    assert!(
        seen[1].0.contains(&format!("/releases/{ALTLIK}/")),
        "{}",
        seen[1].0
    );
    let moved: serde_json::Value = serde_json::from_slice(&seen[1].1).unwrap();
    assert_eq!(
        moved,
        serde_json::json!({ "action": "submit", "note": "İlk sürüm." })
    );
    let logged = gis
        .extensions
        .registry
        .as_ref()
        .unwrap()
        .journal()
        .tail(Some(ALTLIK), 20);
    assert!(
        logged.iter().any(|r| r.code == "release_submitted"),
        "{logged:#?}"
    );
}

/// One installed from a file goes to a catalog from the folder it is developed in: the window says so and does not
/// open; an agent's ask is answered with the same words.
#[test]
fn one_installed_from_a_file_is_not_sent_from_here() {
    let root = temp("katalog-dosya");
    let mut gis = app(&root, Some(Module::Gis));
    install(
        &mut gis,
        "ornek-altlik.kek",
        package("ornek-altlik", |_| {}),
    );
    let refused = gis.agent_submit(ALTLIK, "Asistan").expect_err("refused");
    assert!(
        refused.contains("çalışma klasöründen kurulmadı"),
        "{refused}"
    );
    assert!(gis.extensions.submit.is_none());
}
