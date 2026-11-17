//! Teslim kaydı (docs/adr/8099; plan/bim-plan.md BIM-15) worked by hand on the house of `dated_sizes_tests`: its first
//! package (two sheets' PDFs, the model under EK-2's name with revision 000, the drawing under kaynaklar) goes to the
//! municipality on 2.11.2026 by e-PYS and comes back returned on 9.11.2026; the reason — the WC's door too narrow, the
//! entrance's too — is a review note bound to those two doors. The WC's door is widened to 1.10 m, the second package
//! (the ground floor's sheet changed, the section's the same, the model renamed 001 and changed) goes on 16.11.2026 as
//! revision 001, and set beside the first: 1 sheet changed, 1 the same, the model changed under its new name; in the
//! drawings one object changed (the WC's door) and none added or removed; of the reason's two doors one changed.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib deliveries_tests::screens -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{Delivery, DeliveryChannel, DeliveryOutcome};
use kentos_native_application::delivery_compare::{Compared, FileChange};
use serde_json::{Value, json};

use crate::app::{App, Dialog, Message, Picker};
use crate::arch::ArchDialogKind;
use crate::deliveries_window::{Event, Outcome};
use crate::files_testing::{drive, scratch};

const TO: &str = "Sivas Belediyesi İmar ve Şehircilik Müdürlüğü";
const REASON: &str = "Islak hacim ve giriş kapılarının temiz geçişi yetersiz.";
const NOTE: &str = "WC kapısı 0,86 m, giriş kapısı 1,06 m: genişletin.";

fn send(app: &mut App, e: Event) {
    let task = app.update(Message::Arch(crate::arch::Event::Deliveries(e)));
    drive(app, task);
}

fn deliveries(app: &App) -> Vec<Delivery> {
    app.document.as_ref().expect("a drawing").model.deliveries().to_vec()
}

fn said(app: &App) -> Option<String> {
    app.arch.deliveries.as_ref().and_then(|w| w.said.as_ref()).map(|(_, s)| s.clone())
}

fn run(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// A publication folder of the drawing as it is now: `files` (name, sheets, bytes) written and listed in `yayin.json`,
/// the drawing under kaynaklar.
fn publish(app: &App, dir: &Path, files: &[(&str, &[&str], &[u8])]) -> PathBuf {
    std::fs::create_dir_all(dir.join("kaynaklar")).expect("a folder");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let listed: Vec<Value> = files
        .iter()
        .map(|(name, sheets, bytes)| {
            std::fs::write(dir.join(name), bytes).expect("writes");
            json!({"name": name, "sheets": sheets, "bytes": bytes.len(), "sha256": sha256_hex(bytes)})
        })
        .collect();
    let manifest = json!({
        "format": "kentos.publish/1",
        "source": {"revision": doc.revision().to_string(), "sha256": "0".repeat(64)},
        "files": listed
    });
    std::fs::write(dir.join("yayin.json"), serde_json::to_vec_pretty(&manifest).expect("json")).expect("writes");
    let kcad = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
    std::fs::write(dir.join("kaynaklar/cizim.kcad"), kcad).expect("writes");
    dir.to_path_buf()
}

/// The first package as published, before the return.
fn first_package(app: &App, root: &Path) -> PathBuf {
    publish(
        app,
        &root.join("paket-000"),
        &[
            ("001-A01-zemin-kat.pdf", &["plan-k00"], b"zemin kat 000"),
            ("002-A02-kesit.pdf", &["kesit-aa"], b"kesit 000"),
            ("260123_00_MM_0001_MD_MIM_001_01_000.ifc", &[], b"model 000"),
        ],
    )
}

/// The second package, after the WC's door was widened.
fn second_package(app: &App, root: &Path) -> PathBuf {
    publish(
        app,
        &root.join("paket-001"),
        &[
            ("001-A01-zemin-kat.pdf", &["plan-k00"], b"zemin kat 001"),
            ("002-A02-kesit.pdf", &["kesit-aa"], b"kesit 000"),
            ("260123_00_MM_0001_MD_MIM_001_01_001.ifc", &[], b"model 001"),
        ],
    )
}

/// Teslim ekle with `folder` chosen, the authority and the day typed, saved.
fn deliver(app: &mut App, folder: &Path, at: &str) {
    send(app, Event::Add);
    app.picker = Picker::File(folder.to_path_buf());
    send(app, Event::Pick);
    send(app, Event::To(TO.into()));
    send(app, Event::At(at.into()));
    send(app, Event::Save);
}

/// The house with its first package delivered and returned: the reason bound to the two doors.
fn returned() -> (App, PathBuf, String, String) {
    let (mut app, wc, entrance) = crate::dated_sizes_tests::house();
    let root = scratch("teslim");
    let first = first_package(&app, &root);
    let _ = app.update(Message::Run("arch.deliveries"));
    deliver(&mut app, &first, "2.11.2026");
    send(&mut app, Event::Outcome(Outcome(DeliveryOutcome::Returned)));
    send(&mut app, Event::AnswerAt("9.11.2026".into()));
    send(&mut app, Event::Reason(REASON.into()));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let slots: Vec<_> = [&wc, &entrance]
        .iter()
        .map(|u| doc.slot_of(kentos_domain::Uuid::parse_str(u).expect("a uuid")).expect("in the drawing"))
        .collect();
    app.selection.set(slots);
    send(&mut app, Event::Note(NOTE.into()));
    send(&mut app, Event::Bind);
    send(&mut app, Event::Respond);
    (app, root, wc, entrance)
}

#[test]
fn a_package_is_delivered_returned_with_its_reason_on_two_doors_and_the_corrected_one_compared() {
    let (mut app, wc, entrance) = crate::dated_sizes_tests::house();
    let root = scratch("teslim");
    let first = first_package(&app, &root);
    let _ = app.update(Message::Run("arch.deliveries"));
    assert_eq!(app.dialog, Some(Dialog::Arch(ArchDialogKind::Deliveries)));

    // Without its folder Teslimi kaydet waits; the folder read names the package by its manifest.
    send(&mut app, Event::Add);
    send(&mut app, Event::Save);
    assert_eq!(said(&app).as_deref(), Some("Önce teslim edilen paketin yayın klasörünü seçin."));
    app.picker = Picker::File(first.clone());
    send(&mut app, Event::Pick);
    {
        let draft = app.arch.deliveries.as_ref().and_then(|w| w.draft.as_ref()).expect("the new delivery's form");
        assert_eq!(draft.revision, "000", "the first delivery");
        let read = draft.read.as_ref().expect("read").as_ref().expect("the folder reads");
        let manifest = std::fs::read(first.join("yayin.json")).expect("the manifest");
        assert_eq!((read.package.manifest.as_str(), read.package.files), (sha256_hex(&manifest).as_str(), 3));
        assert!(read.drawing.is_some(), "the drawing is under kaynaklar");
    }
    send(&mut app, Event::To(TO.into()));
    send(&mut app, Event::At("2.11.2026".into()));
    send(&mut app, Event::Receipt("E-2026-118734".into()));
    send(&mut app, Event::Save);
    let list = deliveries(&app);
    assert_eq!(list.len(), 1, "{:?}", said(&app));
    let d = &list[0];
    assert_eq!(
        (d.number, d.revision.as_str(), d.to.as_str(), d.channel, d.at.as_str(), d.receipt.as_deref()),
        (1, "000", TO, DeliveryChannel::Epys, "2026-11-02", Some("E-2026-118734"))
    );
    assert_eq!(said(&app).as_deref(), Some("1. teslim kaydedildi."));

    // Returned: a return without its reason is refused by the command; the reason bound to the two doors.
    send(&mut app, Event::Outcome(Outcome(DeliveryOutcome::Returned)));
    send(&mut app, Event::AnswerAt("9.11.2026".into()));
    send(&mut app, Event::Respond);
    assert_eq!(said(&app).as_deref(), Some("1. teslimin iadesinin gerekçesini yazın."));
    assert!(deliveries(&app)[0].response.is_none());
    send(&mut app, Event::Reason(REASON.into()));
    send(&mut app, Event::Note(NOTE.into()));
    send(&mut app, Event::Bind);
    assert_eq!(said(&app).as_deref(), Some("Önce çizimde gerekçenin ilgili olduğu nesneleri seçin."));
    assert!(app.arch.deliveries.as_ref().expect("open").answer.notes.is_empty());
    let doc = &app.document.as_ref().expect("a drawing").model;
    let slots: Vec<_> = [&wc, &entrance]
        .iter()
        .map(|u| doc.slot_of(kentos_domain::Uuid::parse_str(u).expect("a uuid")).expect("in the drawing"))
        .collect();
    app.selection.set(slots);
    send(&mut app, Event::Bind);
    let notes = app.document.as_ref().expect("a drawing").model.settings().review_notes.clone();
    assert_eq!(notes.len(), 1);
    let anchors: Vec<&str> = notes[0].anchors.iter().map(|a| a.uid.as_str()).collect();
    assert_eq!((notes[0].text.as_str(), anchors), (NOTE, vec![wc.as_str(), entrance.as_str()]));
    assert_eq!(notes[0].revision.as_deref(), Some("000"), "written against the package's revision");
    assert_eq!(
        said(&app).as_deref(),
        Some("1. inceleme notu 2 nesneye bağlandı; cevapla birlikte yazın."),
    );
    send(&mut app, Event::Respond);
    let r = deliveries(&app)[0].response.clone().expect("answered");
    assert_eq!(
        (r.outcome, r.at.as_str(), r.reason.as_deref(), r.notes.clone()),
        (DeliveryOutcome::Returned, "2026-11-09", Some(REASON), vec![notes[0].id.clone()])
    );
    assert_eq!(said(&app).as_deref(), Some("1. teslimin cevabı yazıldı."));

    // The WC's door widened, the corrected package delivered as revision 001 to the same authority.
    run(&mut app, "cad.openings.edit", json!({"uids": [wc], "operation": {"kind": "set", "width": 1.1}}));
    let second = second_package(&app, &root);
    send(&mut app, Event::Add);
    {
        let draft = app.arch.deliveries.as_ref().and_then(|w| w.draft.as_ref()).expect("the form");
        assert_eq!((draft.revision.as_str(), draft.to.as_str(), draft.channel), ("001", TO, DeliveryChannel::Epys));
    }
    app.picker = Picker::File(second.clone());
    send(&mut app, Event::Pick);
    send(&mut app, Event::At("16.11.2026".into()));
    send(&mut app, Event::Save);
    let list = deliveries(&app);
    assert_eq!(list.iter().map(|d| (d.number, d.revision.as_str())).collect::<Vec<_>>(), [(1, "000"), (2, "001")]);
    assert_eq!(app.arch.deliveries.as_ref().and_then(|w| w.chosen.clone()), Some(list[1].id.clone()));

    // Öncekiyle karşılaştır: the files and the drawings of the two packages, the reason's doors.
    send(&mut app, Event::Compare);
    let c = compared(&app);
    assert_eq!(c.previous.1, 1);
    let files: Vec<(Option<&str>, Option<&str>, FileChange)> =
        c.files.iter().map(|f| (f.old.as_deref(), f.new.as_deref(), f.change)).collect();
    assert_eq!(
        files,
        [
            (Some("001-A01-zemin-kat.pdf"), Some("001-A01-zemin-kat.pdf"), FileChange::Changed),
            (Some("002-A02-kesit.pdf"), Some("002-A02-kesit.pdf"), FileChange::Same),
            (
                Some("260123_00_MM_0001_MD_MIM_001_01_000.ifc"),
                Some("260123_00_MM_0001_MD_MIM_001_01_001.ifc"),
                FileChange::Changed
            ),
        ]
    );
    let g = c.drawing.as_ref().expect("both packages carry the drawing");
    assert_eq!((g.changed, g.added, g.removed), (1, 0, 0), "{g:?}");
    let n: Vec<(u32, usize, usize, usize, usize)> =
        c.notes.iter().map(|n| (n.number, n.objects, n.changed, n.removed, n.unknown)).collect();
    assert_eq!(n, [(1, 2, 1, 0, 0)], "of the two doors the WC's changed");

    // The first delivery has no delivery before it to the same authority.
    send(&mut app, Event::Choose(list[0].id.clone()));
    send(&mut app, Event::Compare);
    assert_eq!(said(&app).as_deref(), Some(format!("“{TO}” kurumuna bundan önce teslim yok.").as_str()));
}

fn compared(app: &App) -> Compared {
    let w = app.arch.deliveries.as_ref().expect("open");
    let (_, result) = w.compared.as_ref().expect("compared");
    result.as_ref().clone().expect("the packages compare")
}

#[test]
fn a_renamed_file_shows_its_previous_name_from_where_it_differs() {
    use crate::deliveries_window::renamed;
    assert_eq!(renamed("260123_00_MM_0001_MD_MIM_001_01_000.ifc", "260123_00_MM_0001_MD_MIM_001_01_001.ifc"), "…_000.ifc");
    assert_eq!(renamed("001-A01-zemin-kat.pdf", "001-A01-zemin-kat-R1.pdf"), "…-kat.pdf");
    assert_eq!(renamed("notlar.txt", "aciklama.txt"), "notlar.txt", "a short common beginning: whole");
}

#[test]
fn the_answer_is_one_undo_step_and_a_changed_folder_is_not_compared() {
    let (mut app, root, wc, _) = returned();
    assert!(deliveries(&app)[0].response.is_some());
    let _ = app.update(Message::Run("edit.undo"));
    assert!(deliveries(&app)[0].response.is_none(), "Geri al takes the answer away");
    let _ = app.update(Message::Run("edit.redo"));
    assert!(deliveries(&app)[0].response.is_some(), "Yinele puts it back");

    run(&mut app, "cad.openings.edit", json!({"uids": [wc], "operation": {"kind": "set", "width": 1.1}}));
    let second = second_package(&app, &root);
    deliver(&mut app, &second, "16.11.2026");
    // The first package's folder written over after its delivery: not the package the record names.
    std::fs::write(root.join("paket-000/yayin.json"), b"{\"format\":\"kentos.publish/1\",\"files\":[]}").expect("writes");
    send(&mut app, Event::Compare);
    let w = app.arch.deliveries.as_ref().expect("open");
    let (_, result) = w.compared.as_ref().expect("answered");
    let why = result.as_ref().clone().expect_err("refused");
    assert!(why.contains("1. teslimin paketi değil (SHA-256 farklı)"), "{why}");

    // A record taken away is one undo step; its notes stay.
    let before = app.document.as_ref().expect("a drawing").model.settings().review_notes.len();
    send(&mut app, Event::Remove);
    assert_eq!(deliveries(&app).len(), 1);
    assert_eq!(said(&app).as_deref(), Some("2. teslim kaydı silindi; inceleme notları duruyor."));
    assert_eq!(app.document.as_ref().expect("a drawing").model.settings().review_notes.len(), before);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(deliveries(&app).len(), 2);
}

/// The window's pictures (`.run/gui-evidence/teslim-8099/`), light and dark: the new delivery's form, the returned
/// first delivery with its reason on the two doors, the second set beside the first, and the command's place.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/teslim-8099");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let (mut app, root, wc, _) = returned();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        app.toasts.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-iade-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());

        run(&mut app, "cad.openings.edit", json!({"uids": [wc], "operation": {"kind": "set", "width": 1.1}}));
        let second = second_package(&app, &root);
        send(&mut app, Event::Add);
        app.picker = Picker::File(second.clone());
        send(&mut app, Event::Pick);
        send(&mut app, Event::At("16.11.2026".into()));
        send(&mut app, Event::Receipt("E-2026-121905".into()));
        app.toasts.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-yeni-teslim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());

        send(&mut app, Event::Save);
        send(&mut app, Event::Compare);
        app.toasts.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        // Down the details to the comparison.
        let at = iced::Point::new(size.width * 0.62, size.height * 0.55);
        snapshot.input(&mut app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -30.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("3-karsilastirma-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());

        // Its place on the ribbon: Mimari › Proje ▾, after Model kalite kontrol formu.
        send(&mut app, Event::Close);
        let wide = Size::new(1440.0, 900.0);
        let _ = app.update(Message::WindowResized(wide));
        let mut snapshot = Snapshot::new(wide).expect("a renderer");
        let _ = app.update(Message::RibbonSearch("Teslim kaydı".into()));
        let _ = app.update(Message::RibbonSearchReveal(0));
        snapshot.settle(&mut app, App::view, &mut update);
        let more = iced::Point::new(1278.0, 133.0);
        snapshot.input(&mut app, App::view, &mut update, kentos_ui::snapshot::Input::Click(more));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("4-serit-yeri-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
