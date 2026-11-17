//! Portal dosyaları message by message, without a server (docs/adr/9001): what a request would bring is handed to the
//! app as its answer; the tasks that would reach the network are dropped unrun. The account, the list and a dossier;
//! a step's question, its note and the status it goes with; a stale step; a project file opened as a project; the
//! open project into the package; a message; the window closed and opened where it was.

use std::sync::Arc;

use iced::widget::text_editor;
use kentos_cloud::ApiFailure;
use kentos_commands::{
    DossierAction, DossierDetail, DossierDirection, DossierFileImported, DossierList, DossierSide,
    DossierStatus, DossierSummary, OrganizationCategory, ProjectPermission, ProjectState,
    ProjectStorage,
};

use super::fixtures::{
    APPLICATION, KCAD, MUNICIPALITY, PDF, TRANSFER, added_file, catalog, detail, membership,
    professional_me, summary,
};
use super::{Direction, Event, Place, Question, Said, Tab, places};
use crate::app::{App, Dialog, Message};
use crate::cloud::tests::{PROJECT, TENANT, info, open, signed_in};

pub(super) fn portal(app: &mut App, event: Event) -> Message {
    let message = super::msg(event);
    let _ = app.update(message.clone());
    message
}

/// The window open and loaded for Ayşe, a professional: her list answered with these, the waiting one chosen.
pub(super) fn loaded(items: Vec<DossierSummary>) -> App {
    let mut app = signed_in();
    let _ = app.run("cloud.portal");
    let id = app.cloud.portal.as_ref().expect("open").me_request();
    portal(
        &mut app,
        Event::Loaded {
            id,
            result: Ok(Box::new((professional_me(), catalog()))),
        },
    );
    let id = app.cloud.portal.as_ref().expect("open").list_request();
    let total = items.len() as u32;
    portal(
        &mut app,
        Event::Listed {
            id,
            result: Ok(DossierList { items, total }),
        },
    );
    app
}

/// The chosen dossier's page answered.
pub(super) fn answer_detail(app: &mut App, d: DossierDetail) {
    let id = app.cloud.portal.as_ref().expect("open").detail_request();
    portal(
        app,
        Event::Detail {
            id,
            result: Ok(Box::new(d)),
        },
    );
}

fn typed(app: &mut App, words: &str) {
    let paste = text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(words.to_owned())));
    portal(app, Event::AskNote(paste));
}

fn busy(app: &App) -> u64 {
    app.cloud
        .portal
        .as_ref()
        .and_then(|p| p.busy)
        .expect("a change on its way")
}

fn said(app: &App) -> Option<Said> {
    app.cloud.portal.as_ref().and_then(|p| p.said.clone())
}

fn refused(status: u16, code: &str, message: &str) -> ApiFailure {
    ApiFailure::new(status, code, message)
}

#[test]
fn the_window_asks_the_account_then_the_list_and_chooses_the_one_waiting() {
    let transfer = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
    let mut done = summary(
        APPLICATION,
        DossierDirection::Application,
        DossierStatus::ChangesRequested,
    );
    done.awaiting_me = false;
    let app = loaded(vec![done, transfer]);
    assert_eq!(app.dialog, Some(Dialog::Portal));
    let p = app.cloud.portal.as_ref().expect("open");
    assert_eq!(p.place, Some(Place::Own), "a professional's own work");
    assert_eq!(
        p.chosen.as_deref(),
        Some(TRANSFER),
        "the one waiting for her"
    );
    assert!(p.detail_request() > p.list_request(), "its page is asked");
}

#[test]
fn the_portal_command_signs_in_first() {
    let (mut app, _) = App::boot(None);
    let _ = app.run("cloud.portal");
    assert_eq!(app.dialog, Some(Dialog::SignIn));
    assert_eq!(
        app.cloud.sign_in.as_ref().and_then(|s| s.next),
        Some(crate::cloud::account::Next::Portal)
    );
}

#[test]
fn places_are_her_own_work_and_the_municipalities_she_may_use() {
    let mut me = professional_me();
    me.organizations = vec![
        membership(
            MUNICIPALITY,
            "Örnek Belediyesi",
            OrganizationCategory::Municipality,
            true,
        ),
        membership(
            TENANT,
            "Yılmaz Mimarlık",
            OrganizationCategory::Office,
            true,
        ),
        membership(
            PROJECT,
            "Eski Belediye",
            OrganizationCategory::Municipality,
            false,
        ),
    ];
    let all = places(&me);
    assert_eq!(all.len(), 2, "{all:?}");
    assert_eq!(all[0], Place::Own);
    assert_eq!(all[1].label(), "Örnek Belediyesi");
    assert_eq!(Direction::Applications.label(&all[0]), "Başvurularım");
    assert_eq!(Direction::Applications.label(&all[1]), "Gelen başvurular");
    me.professional = None;
    me.organizations.clear();
    assert!(
        places(&me).is_empty(),
        "nothing to list: the window says why"
    );
}

#[test]
fn a_refusal_needs_its_note_and_a_step_goes_with_the_status_seen() {
    let s = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
    let mut app = loaded(vec![s.clone()]);
    answer_detail(
        &mut app,
        detail(
            s.clone(),
            vec![DossierAction::Accept, DossierAction::Decline],
        ),
    );
    portal(&mut app, Event::Step(DossierAction::Decline));
    portal(&mut app, Event::AskConfirm);
    let a = app
        .cloud
        .portal
        .as_ref()
        .and_then(|p| p.asking.as_ref())
        .expect("asked");
    assert_eq!(a.question, Question::Step(DossierAction::Decline));
    assert_eq!(
        a.missing.as_deref(),
        Some(super::plan::note_missing("Neden").as_str())
    );
    assert!(
        app.cloud.portal.as_ref().is_some_and(|p| p.busy.is_none()),
        "nothing went"
    );
    typed(&mut app, "Takvimim bu ay dolu.");
    portal(&mut app, Event::AskConfirm);
    let id = busy(&app);
    let mut after = detail(
        DossierSummary {
            status: DossierStatus::Declined,
            status_label: "Üstlenilmedi".into(),
            awaiting_me: false,
            ..s
        },
        Vec::new(),
    );
    after.can_message = false;
    let list_before = app.cloud.portal.as_ref().expect("open").list_request();
    portal(
        &mut app,
        Event::Acted {
            id,
            result: Ok(Box::new(after)),
        },
    );
    let p = app.cloud.portal.as_ref().expect("open");
    assert!(p.asking.is_none() && p.busy.is_none());
    assert_eq!(
        p.said,
        Some(Said::Done("İşi üstlenmediğiniz bildirildi.".into()))
    );
    assert_eq!(
        p.chosen_detail().map(|d| d.summary.status),
        Some(DossierStatus::Declined)
    );
    assert!(
        p.list_request() > list_before,
        "its row says the new status"
    );
}

#[test]
fn a_stale_step_is_said_in_the_servers_words_and_the_dossier_read_again() {
    let s = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
    let mut app = loaded(vec![s.clone()]);
    answer_detail(
        &mut app,
        detail(s, vec![DossierAction::Accept, DossierAction::Decline]),
    );
    portal(&mut app, Event::Step(DossierAction::Accept));
    portal(&mut app, Event::AskConfirm);
    let id = busy(&app);
    let before = app.cloud.portal.as_ref().expect("open").detail_request();
    let words =
        "Dosya siz bakarken değişti; şimdi “Üstlenildi”. Sayfayı yenileyip yeniden deneyin.";
    portal(
        &mut app,
        Event::Acted {
            id,
            result: Err(refused(409, "conflict", words)),
        },
    );
    let p = app.cloud.portal.as_ref().expect("open");
    let a = p.asking.as_ref().expect("the question stays");
    assert!(!a.busy);
    assert_eq!(a.error.as_deref(), Some(words));
    assert!(p.detail_request() > before, "the dossier is read again");
}

#[test]
fn a_project_file_opens_as_a_project_in_the_drawing() {
    let s = summary(
        TRANSFER,
        DossierDirection::Transfer,
        DossierStatus::Accepted,
    );
    let mut app = loaded(vec![s.clone()]);
    answer_detail(&mut app, detail(s, vec![DossierAction::Deliver]));
    // Only a sent KentOS project file opens as a project.
    portal(&mut app, Event::Import(PDF.into()));
    assert!(
        app.cloud
            .portal
            .as_ref()
            .is_some_and(|p| p.asking.is_none())
    );
    portal(&mut app, Event::Import(KCAD.into()));
    portal(&mut app, Event::AskConfirm);
    let id = busy(&app);
    let made = DossierFileImported {
        tenant_id: TENANT.into(),
        project_id: PROJECT.into(),
        name: "Cephe (2026/0007)".into(),
    };
    portal(
        &mut app,
        Event::Imported {
            id,
            result: Ok(made),
        },
    );
    let opening = app.cloud.opening.as_ref().expect("the project opens");
    assert_eq!(opening.name, "Cephe (2026/0007)");
    assert_eq!(
        app.dialog, None,
        "the window steps aside for the open's progress"
    );
    let p = app
        .cloud
        .portal
        .as_ref()
        .expect("kept for when it is opened again");
    assert!(p.asking.is_none());
    assert_eq!(
        p.made.as_ref().map(|m| m.name.as_str()),
        Some("Cephe (2026/0007)")
    );
}

#[test]
fn the_open_project_goes_into_the_package_only_when_it_may() {
    let s = summary(
        TRANSFER,
        DossierDirection::Transfer,
        DossierStatus::Accepted,
    );
    let mut d = detail(
        s.clone(),
        vec![DossierAction::Deliver, DossierAction::Cancel],
    );
    d.can_add_files = true;
    // No cloud project open: nothing to add.
    let mut app = loaded(vec![s.clone()]);
    answer_detail(&mut app, d.clone());
    portal(&mut app, Event::AddOpen);
    assert!(app.cloud.portal.as_ref().is_some_and(|p| p.busy.is_none()));
    // A file project without the right to download it.
    let mut project = info(ProjectStorage::File, true, ProjectState::Active);
    open(&mut app, project.clone());
    assert!(app.portal_open_project().and_then(|o| o.refused).is_some());
    project.access.permissions.push(ProjectPermission::Download);
    open(&mut app, project);
    app.dialog = Some(Dialog::Portal);
    let open = app.portal_open_project().expect("a cloud project");
    assert_eq!((open.refused, open.unsaved), (None, false));
    portal(&mut app, Event::AddOpen);
    let id = busy(&app);
    let added = added_file();
    let before = app.cloud.portal.as_ref().expect("open").detail_request();
    portal(
        &mut app,
        Event::Added {
            id,
            result: Ok(added),
        },
    );
    let p = app.cloud.portal.as_ref().expect("open");
    assert_eq!(
        p.said,
        Some(Said::Done(
            "“Ada 101.kcad” (revizyon 3) pakete eklendi; gönderene kadar yalnız siz görürsünüz."
                .into()
        ))
    );
    assert!(p.detail_request() > before, "the package is read again");
    // The same revision is in the package now: it does not go in twice.
    d.pending = vec![added_file()];
    answer_detail(&mut app, d);
    portal(&mut app, Event::AddOpen);
    let p = app.cloud.portal.as_ref().expect("open");
    assert!(p.busy.is_none(), "nothing went");
    assert_eq!(p.said, Some(Said::Info(super::package::IN_PACKAGE.into())));
}

#[test]
fn a_database_projects_edits_must_reach_the_server_before_it_goes_in() {
    let s = summary(
        TRANSFER,
        DossierDirection::Transfer,
        DossierStatus::Accepted,
    );
    let mut d = detail(s.clone(), vec![DossierAction::Deliver]);
    d.can_add_files = true;
    let mut app = loaded(vec![s]);
    answer_detail(&mut app, d);
    if let Some(p) = app.cloud.portal.as_mut() {
        p.adding = true;
    }
    let _ = app.portal_add_settled(false);
    let p = app.cloud.portal.as_ref().expect("open");
    assert!(!p.adding && p.busy.is_none(), "nothing went");
    assert!(
        matches!(&p.said, Some(Said::Error(t)) if t.starts_with("Açık projenin değişiklikleri"))
    );
}

#[test]
fn a_message_goes_and_its_box_empties() {
    let s = summary(
        TRANSFER,
        DossierDirection::Transfer,
        DossierStatus::Accepted,
    );
    let mut app = loaded(vec![s.clone()]);
    answer_detail(&mut app, detail(s, vec![DossierAction::Deliver]));
    portal(&mut app, Event::Tab(Tab::Messages));
    portal(&mut app, Event::Send);
    assert!(
        matches!(said(&app), Some(Said::Error(_))),
        "an empty message does not go"
    );
    let paste = text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(
        "Yarın yüklüyorum.".into(),
    )));
    portal(&mut app, Event::Draft(paste));
    portal(&mut app, Event::Send);
    let id = busy(&app);
    portal(
        &mut app,
        Event::Sent {
            id,
            result: Err(refused(
                0,
                "network",
                "Sunucuya ulaşılamadı; bağlantınızı ve sunucu adresini denetleyin.",
            )),
        },
    );
    let p = app.cloud.portal.as_ref().expect("open");
    assert_eq!(
        p.draft.text().trim(),
        "Yarın yüklüyorum.",
        "what was written stays"
    );
    portal(&mut app, Event::Send);
    let id = busy(&app);
    let sent = kentos_commands::DossierMessageView {
        id: "m1".into(),
        round: 1,
        side: DossierSide::Professional,
        kind: "message".into(),
        author_name: "Ayşe Yılmaz".into(),
        body: "Yarın yüklüyorum.".into(),
        at: "2026-11-17T18:00:00Z".into(),
        attachments: Vec::new(),
    };
    portal(
        &mut app,
        Event::Sent {
            id,
            result: Ok(sent),
        },
    );
    let p = app.cloud.portal.as_ref().expect("open");
    assert!(p.draft.text().trim().is_empty());
    assert_eq!(p.said, Some(Said::Done("İleti gönderildi.".into())));
}

#[test]
fn a_question_closes_before_the_window_and_the_window_keeps_where_it_was() {
    let s = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
    let mut app = loaded(vec![s.clone()]);
    answer_detail(
        &mut app,
        detail(s, vec![DossierAction::Accept, DossierAction::Decline]),
    );
    portal(&mut app, Event::Direction(Direction::Transfers));
    portal(&mut app, Event::Step(DossierAction::Accept));
    app.close_dialog();
    assert_eq!(
        app.dialog,
        Some(Dialog::Portal),
        "the question went, the window stays"
    );
    assert!(
        app.cloud
            .portal
            .as_ref()
            .is_some_and(|p| p.asking.is_none())
    );
    app.close_dialog();
    assert_eq!(app.dialog, None);
    assert!(app.cloud.portal.is_none());
    let _ = app.run("cloud.portal");
    let p = app.cloud.portal.as_ref().expect("open again");
    assert_eq!(p.chosen.as_deref(), Some(TRANSFER));
    assert_eq!(p.direction, Direction::Transfers);
    assert_eq!(p.place, Some(Place::Own));
}

#[test]
fn a_late_answer_is_dropped() {
    let s = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
    let mut app = loaded(vec![s.clone()]);
    let old = app.cloud.portal.as_ref().expect("open").list_request();
    portal(&mut app, Event::Direction(Direction::Applications));
    portal(
        &mut app,
        Event::Listed {
            id: old,
            result: Ok(DossierList {
                items: Vec::new(),
                total: 0,
            }),
        },
    );
    let p = app.cloud.portal.as_ref().expect("open");
    assert_eq!(
        p.list.as_ref().map(|l| l.items.len()),
        Some(1),
        "the old answer did not land"
    );
}
