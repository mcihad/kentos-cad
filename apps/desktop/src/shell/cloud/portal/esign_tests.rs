//! E-imza in Portal dosyaları without a card or a server (docs/adr/9002 §7): the software test card and the in-process
//! test server stand in for both, and every task the window gives is run to its end, so the whole way is taken: the
//! switch, E-imzayla gönder, the envelope verified, the cards read, a wrong PIN asked again in the same window, the
//! files signed and verified here, the delivery verified by the server's rules, then Gönder's question.

use iced::Task;
use iced::futures::StreamExt as _;
use kentos_commands::{DossierAction, DossierDirection, DossierStatus};
use kentos_imza_ui::{Msg, Outcome, Phase};

use super::esign::EsignEvent;
use super::fixtures::{APPLICATION, detail, file, summary};
use super::tests::{answer_detail, loaded};
use super::{Event, Question, msg};
use crate::app::{App, Message};

/// Every message of a task, and of the tasks those give, applied in turn (a guard against an endless chain).
fn run(app: &mut App, task: Task<Message>, depth: usize) {
    if depth > 24 {
        return;
    }
    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        return;
    };
    while let Some(action) = iced::futures::executor::block_on(stream.next()) {
        if let iced_runtime::Action::Output(m) = action {
            // Only the portal's own messages: a list or a dossier asked again would go to the network.
            let next = app.update(m);
            run(app, next, depth + 1);
        }
    }
}

fn esign(app: &mut App, e: EsignEvent) {
    let t = app.update(msg(Event::Esign(Box::new(e))));
    run(app, t, 0);
}

/// Ayşe's application with two files in the package being prepared, Gönder allowed.
fn ready() -> App {
    let s = summary(
        APPLICATION,
        DossierDirection::Application,
        DossierStatus::Draft,
    );
    let mut app = loaded(vec![s.clone()]);
    let _ = app.update(msg(Event::Choose(APPLICATION.to_owned())));
    let mut d = detail(s, vec![DossierAction::Send]);
    d.can_add_files = true;
    d.can_sign = true;
    d.pending = vec![
        file(
            "0b1e0000-0000-7000-8000-000000000001",
            "Vaziyet planı.pdf",
            false,
            "pending",
        ),
        file(
            "0b1e0000-0000-7000-8000-000000000002",
            "Statik rapor.pdf",
            false,
            "pending",
        ),
    ];
    answer_detail(&mut app, d);
    app
}

fn dialog(app: &App) -> &kentos_imza_ui::SigningDialog {
    &app.cloud
        .portal
        .as_ref()
        .and_then(|p| p.esign.as_ref())
        .expect("the signing window")
        .dialog
}

#[test]
fn esign_sends_after_signing_and_the_server_verifies() {
    let mut app = ready();
    // Off by default: the window is as it was without a card.
    assert!(!app.cloud.portal.as_ref().expect("open").esign_on);
    let t = app.update(msg(Event::EsignSwitch(true)));
    run(&mut app, t, 0);
    let t = app.update(msg(Event::EsignStart(true)));
    run(&mut app, t, 0);

    // The envelope verified, the batch opened, the cards read.
    let d = dialog(&app);
    assert_eq!(d.phase, Phase::Ready);
    assert_eq!(d.request.as_ref().map(|r| r.documents.len()), Some(2));
    assert!(d.mock, "the software card says so");
    assert!(d.chosen_certificate().is_some_and(|c| c.key == "EC P-384"));

    // A wrong PIN: asked again in the same window, the batch untouched.
    esign(&mut app, EsignEvent::Dialog(Msg::Pin("0000".into())));
    esign(&mut app, EsignEvent::Dialog(Msg::Sign));
    let d = dialog(&app);
    assert_eq!(d.phase, Phase::Ready);
    assert!(
        d.notice
            .as_ref()
            .is_some_and(|n| n.error && n.text.contains("PIN"))
    );

    // The right one: signed here, verified here, delivered and verified by the server.
    esign(&mut app, EsignEvent::Dialog(Msg::Pin("1234".into())));
    esign(&mut app, EsignEvent::Dialog(Msg::Sign));
    let d = dialog(&app);
    assert_eq!(
        d.phase,
        Phase::Done(Outcome::Completed { signed: 2 }),
        "{:?}",
        d.notice
    );
    let p = app.cloud.portal.as_ref().expect("open");
    let s = p.esign.as_ref().expect("still shown");
    assert!(s.verdict.as_ref().is_some_and(|v| v.accepted
        && v.signed == 2
        && v.legal_check == kentos_commands::SigningLegalCheck::NotDone));
    assert_eq!(
        s.record.as_ref().map(|r| r.batch.status),
        Some(kentos_imza_domain::BatchStatus::Completed),
        "COMPLETED only after the server's answer"
    );

    // Kapat: the window goes, the signatures are asked for the file lines, Gönder's question opens.
    esign(&mut app, EsignEvent::Dialog(Msg::Close));
    let p = app.cloud.portal.as_ref().expect("open");
    assert!(p.esign.is_none());
    assert!(matches!(
        p.asking.as_ref().map(|a| &a.question),
        Some(Question::Step(DossierAction::Send))
    ));
    let imzalar = p.signatures.get(APPLICATION).expect("asked");
    assert_eq!(imzalar.len(), 2);
    assert!(
        imzalar
            .iter()
            .all(|s| s.still_matches && s.subject.contains("gerçek değil"))
    );
}

#[test]
fn esign_cancelled_leaves_the_package_as_it_was() {
    let mut app = ready();
    let _ = app.update(msg(Event::EsignSwitch(true)));
    let t = app.update(msg(Event::EsignStart(false)));
    run(&mut app, t, 0);
    assert_eq!(super::esign::mock_server().open(), 1);
    esign(&mut app, EsignEvent::Dialog(Msg::Cancel));
    let p = app.cloud.portal.as_ref().expect("open");
    assert!(p.esign.is_none(), "Vazgeç closes the signing window");
    assert!(p.asking.is_none(), "and takes no step");
    assert_eq!(p.chosen_detail().map(|d| d.pending.len()), Some(2));
    assert_eq!(
        super::esign::mock_server().open(),
        0,
        "the server's request is given up, not left until it expires"
    );

    // Esc over the signing window is its Kapat: the request is given up too, Portal dosyaları stays open.
    let t = app.update(msg(Event::EsignStart(false)));
    run(&mut app, t, 0);
    assert_eq!(super::esign::mock_server().open(), 1);
    app.dialog = Some(crate::app::Dialog::Portal);
    let t = app.update(Message::Key(crate::keys::KeyPress {
        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
        physical: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        modifiers: iced::keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    }));
    run(&mut app, t, 0);
    let p = app.cloud.portal.as_ref().expect("Portal dosyaları stays");
    assert!(p.esign.is_none(), "Esc closes the signing window");
    assert_eq!(app.dialog, Some(crate::app::Dialog::Portal));
    assert_eq!(super::esign::mock_server().open(), 0);
}

#[test]
fn esign_offered_only_with_the_switch_to_a_signer_with_files() {
    let mut app = ready();
    let d = app
        .cloud
        .portal
        .as_ref()
        .and_then(|p| p.chosen_detail())
        .expect("chosen")
        .clone();
    // Off: nothing is offered, and the message alone starts nothing either.
    assert_eq!(super::esign::offer(false, &d), None);
    let t = app.update(msg(Event::EsignStart(true)));
    run(&mut app, t, 0);
    assert!(app.cloud.portal.as_ref().expect("open").esign.is_none());
    // On: E-imzayla gönder while Gönder is one of the steps, E-imzala otherwise.
    assert_eq!(super::esign::offer(true, &d), Some(true));
    let mut no_send = d.clone();
    no_send.actions.clear();
    assert_eq!(super::esign::offer(true, &no_send), Some(false));
    // A reader the server does not let sign these files (`can_sign`): nothing is offered.
    let mut reader = d.clone();
    reader.can_sign = false;
    assert_eq!(super::esign::offer(true, &reader), None);
    // Nothing in the package being prepared: nothing to sign.
    let mut empty = d;
    empty.pending.clear();
    assert_eq!(super::esign::offer(true, &empty), None);
    // The message alone starts nothing the row does not offer either.
    let _ = app.update(msg(Event::EsignSwitch(true)));
    if let Some(d) = app.cloud.portal.as_mut().and_then(|p| p.detail.as_mut()) {
        d.can_sign = false;
    }
    let t = app.update(msg(Event::EsignStart(false)));
    run(&mut app, t, 0);
    assert!(app.cloud.portal.as_ref().expect("open").esign.is_none());
    if let Some(d) = app.cloud.portal.as_mut().and_then(|p| p.detail.as_mut()) {
        d.can_sign = true;
        d.pending.clear();
    }
    let t = app.update(msg(Event::EsignStart(false)));
    run(&mut app, t, 0);
    assert!(app.cloud.portal.as_ref().expect("open").esign.is_none());
}

/// Pictures for the owner: the switch with E-imzayla gönder, the signing window over Portal dosyaları with the PIN
/// typed, its end, and the package's file lines once signed; dark and light at 1440×900. Not run by default:
/// `cargo test -p kentos-desktop --lib cloud::portal::esign_tests::esign_screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn esign_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/imza/masaustu");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", "koyu"), ("light", "acik")] {
        for scene in ["anahtar", "pencere", "bitti", "imzali"] {
            // Every scene starts from a dossier nobody has signed yet.
            super::esign::mock_server().forget();
            let mut app = ready();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let t = app.update(msg(Event::EsignSwitch(true)));
            run(&mut app, t, 0);
            if scene != "anahtar" {
                // E-imzala for the last one: Gönder's question would cover the file lines.
                let t = app.update(msg(Event::EsignStart(scene != "imzali")));
                run(&mut app, t, 0);
                esign(&mut app, EsignEvent::Dialog(Msg::Pin("1234".into())));
            }
            if scene == "bitti" || scene == "imzali" {
                esign(&mut app, EsignEvent::Dialog(Msg::Sign));
            }
            if scene == "imzali" {
                esign(&mut app, EsignEvent::Dialog(Msg::Close));
            }
            let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("masaustu-{scene}-{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
