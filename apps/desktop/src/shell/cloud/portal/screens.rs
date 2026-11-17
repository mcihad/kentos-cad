//! Portal dosyaları' pictures for the owner (docs/adr/9001): a transfer waiting for its professional, the refusal's
//! question, a delivery being prepared with the open project, the messages and the facts, dark and light, at
//! 1440×900 and 1100×650. Not run by default:
//! `cargo test -p kentos-desktop --lib cloud::portal::screens -- --ignored --nocapture`.

use std::sync::Arc;

use iced::Size;
use iced::widget::text_editor;
use kentos_commands::{
    DocCheckItem, DocChecklist, DossierAction, DossierDirection, DossierEventView,
    DossierMessageView, DossierSide, DossierStatus, LegalClock, ProjectPermission, ProjectState,
    ProjectStorage,
};
use kentos_ui::snapshot::Snapshot;

use super::fixtures::{APPLICATION, TRANSFER, detail, file, summary};
use super::tests::{answer_detail, loaded, portal};
use super::{Event, Tab};
use crate::app::{App, Dialog};
use crate::cloud::tests::{info, open};

fn event(at: &str, text: &str) -> DossierEventView {
    DossierEventView {
        id: at.into(),
        at: at.into(),
        actor_name: "Ali Çelik".into(),
        side: DossierSide::Municipality,
        action: "send".into(),
        text: text.into(),
        round: 1,
    }
}

fn message(kind: &str, side: DossierSide, who: &str, body: &str) -> DossierMessageView {
    DossierMessageView {
        id: body.into(),
        round: 1,
        side,
        kind: kind.into(),
        author_name: who.into(),
        body: body.into(),
        at: "2026-11-17T18:05:00Z".into(),
        attachments: Vec::new(),
    }
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/desktop-portal/screens");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let scenes = [
        "aktarim", "soru", "teslim", "yazisma", "bilgiler", "basvuru",
    ];
    for (mode, suffix) in [("dark", "koyu"), ("light", "acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in scenes {
                let transfer = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
                let mut application = summary(
                    APPLICATION,
                    DossierDirection::Application,
                    DossierStatus::ChangesRequested,
                );
                application.awaiting_me = scene == "basvuru";
                let mut list = vec![transfer.clone(), application.clone()];
                if scene == "basvuru" {
                    list.reverse();
                }
                let mut app = loaded(list);
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut d = detail(
                    transfer.clone(),
                    vec![DossierAction::Accept, DossierAction::Decline],
                );
                d.events = vec![
                    event(
                        "2026-11-17T17:33:21Z",
                        "Ali Çelik aktarımı taslak olarak hazırlamaya başladı.",
                    ),
                    event(
                        "2026-11-17T17:33:22Z",
                        "Ali Çelik projeyi aktardı (2 dosya).",
                    ),
                ];
                match scene {
                    "soru" => {
                        answer_detail(&mut app, d);
                        portal(&mut app, Event::Step(DossierAction::Decline));
                        portal(&mut app, Event::AskConfirm);
                    }
                    "teslim" | "yazisma" | "bilgiler" => {
                        d.summary.status = DossierStatus::Accepted;
                        d.summary.status_label = "Üstlenildi".into();
                        d.actions = vec![DossierAction::Deliver];
                        d.can_add_files = true;
                        d.pending = vec![file(
                            "01a1ea92-0b91-777f-8c49-0a95082ec5f9",
                            "Cephe düzenlemesi.kcad",
                            true,
                            "pending",
                        )];
                        d.messages = vec![
                            message(
                                "message",
                                DossierSide::Municipality,
                                "Ali Çelik",
                                "Cephe paftalarını bu hafta içinde bekliyoruz.",
                            ),
                            message(
                                "message",
                                DossierSide::Professional,
                                "Ayşe Yılmaz",
                                "Pencere oranlarını düzelttim; teslim ediyorum.",
                            ),
                        ];
                        let mut project = info(ProjectStorage::File, true, ProjectState::Active);
                        project.name = "Cephe düzenlemesi".into();
                        project.access.permissions.push(ProjectPermission::Download);
                        open(&mut app, project);
                        app.dialog = Some(Dialog::Portal);
                        answer_detail(&mut app, d);
                        if scene == "yazisma" {
                            portal(&mut app, Event::Tab(Tab::Messages));
                            let words = "Teslim notunu da ekledim.";
                            let paste = text_editor::Action::Edit(text_editor::Edit::Paste(
                                Arc::new(words.into()),
                            ));
                            portal(&mut app, Event::Draft(paste));
                        }
                        if scene == "bilgiler" {
                            portal(&mut app, Event::Tab(Tab::Facts));
                        }
                    }
                    "basvuru" => {
                        let mut a = detail(
                            application,
                            vec![DossierAction::Send, DossierAction::Withdraw],
                        );
                        a.can_add_files = true;
                        a.summary.clock = Some(LegalClock {
                            label: "Karar".into(),
                            due_on: "2026-12-02".into(),
                            days_left: 15,
                            basis: "Eksikler giderildikten sonraki başvurudan itibaren en geç 15 günde karar \
                                    verilir (İmar Kanunu m.22)."
                                .into(),
                        });
                        a.messages = vec![message(
                            "deficiency",
                            DossierSide::Municipality,
                            "Zeynep Demir",
                            "Yapı denetim kuruluşunun iş bitirme tutanağı eklenmeli.",
                        )];
                        a.checklist = Some(DocChecklist {
                            basis: "Önerilen belgeler; neyin gerektiğine belediye karar verir."
                                .into(),
                            round: 2,
                            items: vec![
                                DocCheckItem {
                                    doc_type: "permit".into(),
                                    label: "Yapı ruhsatı".into(),
                                    present: true,
                                },
                                DocCheckItem {
                                    doc_type: "report".into(),
                                    label: "İş bitirme tutanağı".into(),
                                    present: false,
                                },
                            ],
                        });
                        answer_detail(&mut app, a);
                    }
                    _ => answer_detail(&mut app, d),
                }
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("portal-{scene}-{width}x{height}-{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
