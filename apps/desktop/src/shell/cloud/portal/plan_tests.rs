//! What Portal dosyaları says and offers (plan.rs, docs/adr/9001 §3), read against the web portal's words
//! (apps/portal/src/pages/dossier/): the steps and the one filled button, the questions, whose turn it is, the
//! notices, the packages' titles, the place, the days.

use kentos_commands::{
    DossierAction, DossierDirection, DossierMessageView, DossierRound, DossierSide, DossierSite,
    DossierStatus, LegalClock,
};

use super::fixtures::{TRANSFER, catalog, detail, summary};
use super::plan::{self, Look, Note, Tone};

fn message(kind: &str, body: &str) -> DossierMessageView {
    DossierMessageView {
        id: "m1".into(),
        round: 1,
        side: DossierSide::Municipality,
        kind: kind.into(),
        author_name: "Zeynep Demir".into(),
        body: body.into(),
        at: "2026-11-17T17:40:00Z".into(),
        attachments: Vec::new(),
    }
}

#[test]
fn only_the_servers_steps_are_offered_and_one_of_them_is_filled() {
    let s = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
    let d = detail(s, vec![DossierAction::Accept, DossierAction::Decline]);
    assert_eq!(
        plan::steps(&d),
        [
            (DossierAction::Decline, Look::Danger),
            (DossierAction::Accept, Look::Primary)
        ],
        "the step forward last, at the right edge"
    );
    assert_eq!(plan::step_label(DossierAction::Accept, &d), "İşi üstlen");
    assert_eq!(plan::step_label(DossierAction::Decline, &d), "Üstlenme");
    let none = detail(
        summary(
            TRANSFER,
            DossierDirection::Transfer,
            DossierStatus::Delivered,
        ),
        Vec::new(),
    );
    assert!(
        plan::steps(&none).is_empty(),
        "nothing the server does not allow"
    );
    // An application sent once before goes again as its corrected version.
    let mut again = summary(
        TRANSFER,
        DossierDirection::Application,
        DossierStatus::ChangesRequested,
    );
    again.round = 1;
    let d = detail(again, vec![DossierAction::Send, DossierAction::Withdraw]);
    assert_eq!(
        plan::step_label(DossierAction::Send, &d),
        "Düzeltilmiş başvuruyu gönder"
    );
    assert_eq!(
        plan::steps(&d),
        [
            (DossierAction::Withdraw, Look::Danger),
            (DossierAction::Send, Look::Primary)
        ]
    );
}

#[test]
fn a_refusal_and_a_deficiency_list_ask_for_their_words() {
    let s = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
    let d = detail(s, vec![DossierAction::Accept, DossierAction::Decline]);
    let decline = plan::ask_of(DossierAction::Decline, &d).expect("a question");
    assert_eq!(decline.note, Some((Note::Required, "Neden")));
    assert!(decline.danger);
    let accept = plan::ask_of(DossierAction::Accept, &d).expect("a question");
    assert_eq!(accept.note, Some((Note::Optional, "Belediyeye not")));
    assert!(
        accept
            .lead
            .as_deref()
            .is_some_and(|l| l.starts_with("Örnek Belediyesi işi üstlendiğinizi"))
    );
    let changes = plan::ask_of(DossierAction::RequestChanges, &d).expect("a question");
    assert_eq!(changes.note, Some((Note::Required, "Eksik ve yanlışlar")));
    assert!(changes.note_hint.is_some_and(|h| h.contains("m.22")));
    let approve = plan::ask_of(DossierAction::Approve, &d).expect("a question");
    assert_eq!(approve.caution, Some(plan::DECISION_CAUTION));
    assert!(
        plan::ask_of(DossierAction::StartReview, &d).is_none(),
        "taken at once"
    );
}

#[test]
fn whose_turn_it_is_is_said_for_each_side() {
    let s = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
    let mut d = detail(s, vec![DossierAction::Accept]);
    assert_eq!(plan::waiting(&d), "Sıra sizde.");
    d.summary.awaiting_me = false;
    assert_eq!(plan::waiting(&d), "Sıra belediyede.");
    d.side = DossierSide::Municipality;
    assert_eq!(plan::waiting(&d), "Sıra meslek mensubunda.");
    d.summary.awaiting_me = true;
    assert_eq!(plan::waiting(&d), "Sıra belediyenizde.");
    d.summary.status = DossierStatus::Completed;
    assert_eq!(plan::waiting(&d), "Dosya kapandı.");
}

#[test]
fn a_correction_asked_for_shows_its_list_first() {
    let s = summary(
        TRANSFER,
        DossierDirection::Application,
        DossierStatus::ChangesRequested,
    );
    let mut d = detail(s, vec![DossierAction::Send]);
    d.messages = vec![
        message("deficiency", "Eski liste."),
        message("message", "Bir soru."),
        message("deficiency", "İş bitirme tutanağı eklenmeli."),
    ];
    let n = plan::notices(&d);
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].tone, Tone::Warning);
    assert_eq!(n[0].title, "Belediye düzeltme istedi.");
    assert_eq!(
        n[0].body.as_deref(),
        Some("İş bitirme tutanağı eklenmeli."),
        "the last list"
    );
}

#[test]
fn the_packages_are_named_as_each_side_calls_them() {
    let s = summary(
        TRANSFER,
        DossierDirection::Transfer,
        DossierStatus::Accepted,
    );
    let d = detail(s, vec![DossierAction::Deliver]);
    assert_eq!(plan::pending_title(&d), "Hazırlanan 1. teslim");
    let round = |n| DossierRound {
        round: n,
        side: DossierSide::Municipality,
        sent_at: None,
        files: Vec::new(),
    };
    assert_eq!(
        plan::round_title(&d, &round(1)),
        "Belediyenin gönderdiği paket"
    );
    assert_eq!(plan::round_title(&d, &round(3)), "2. teslim");
    let mut app = summary(
        TRANSFER,
        DossierDirection::Application,
        DossierStatus::ChangesRequested,
    );
    app.round = 1;
    let d = detail(app, vec![DossierAction::Send]);
    assert_eq!(plan::pending_title(&d), "Hazırlanan 2. sürüm");
    assert_eq!(plan::round_title(&d, &round(2)), "2. sürüm (düzeltilmiş)");
}

#[test]
fn the_place_and_the_days_are_said_as_people_say_them() {
    let mut site = DossierSite {
        neighbourhood: Some("Cumhuriyet Mahallesi".into()),
        block: Some("412".into()),
        parcel: Some("7".into()),
        ..DossierSite::default()
    };
    assert_eq!(
        plan::site_text(&site),
        "Cumhuriyet Mahallesi, 412 ada 7 parsel"
    );
    site = DossierSite {
        province: Some("Örnek".into()),
        district: Some("Merkez".into()),
        ..DossierSite::default()
    };
    assert_eq!(plan::site_text(&site), "Merkez, Örnek");
    // A block alone (a plan change): “220 ada”, never “220 ada — parsel”.
    let mut d = detail(
        summary(
            TRANSFER,
            DossierDirection::Application,
            DossierStatus::Draft,
        ),
        Vec::new(),
    );
    d.summary.site.block = Some("220".into());
    let parcel = plan::facts(&d)
        .into_iter()
        .find(|(name, _)| *name == "Ada / parsel");
    assert_eq!(parcel.map(|(_, v)| v).as_deref(), Some("220 ada"));
    assert_eq!(plan::days_left(5), "5 gün kaldı");
    assert_eq!(plan::days_left(0), "bugün son gün");
    assert_eq!(plan::days_left(-2), "2 gün geçti");
    assert_eq!(plan::day_text("2026-11-27"), "27.11.2026");
    // 17 November 2026, local noon: ten days to the 27th.
    let today = crate::cloud::local_time::seconds_of(2026, 11, 17) + 12 * 3600;
    assert_eq!(plan::days_until("2026-11-27", today), Some(10));
    let s = summary(
        TRANSFER,
        DossierDirection::Transfer,
        DossierStatus::Accepted,
    );
    assert_eq!(
        plan::due_line(&s, today).as_deref(),
        Some("İstenen teslim: 27.11.2026, 10 gün kaldı.")
    );
    let c = LegalClock {
        label: "Karar".into(),
        due_on: "2026-12-02".into(),
        days_left: 2,
        basis: "İmar Kanunu m.22".into(),
        extended_from: None,
    };
    assert_eq!(
        plan::clock_line(&c),
        (Tone::Warning, "Karar: 02.12.2026, 2 gün kaldı.".to_owned())
    );
}

#[test]
fn the_other_side_and_the_tones_come_from_the_servers_catalog() {
    let c = catalog();
    let s = summary(TRANSFER, DossierDirection::Transfer, DossierStatus::Sent);
    assert_eq!(
        plan::counterpart(&s, DossierSide::Professional, Some(&c)),
        "Örnek Belediyesi"
    );
    assert_eq!(
        plan::counterpart(&s, DossierSide::Municipality, Some(&c)),
        "Ayşe Yılmaz, Mimar"
    );
    assert_eq!(
        plan::status_tone(Some(&c), DossierStatus::Declined),
        Tone::Danger
    );
    assert_eq!(plan::status_tone(Some(&c), DossierStatus::Sent), Tone::Info);
    assert_eq!(
        plan::status_tone(None, DossierStatus::Sent),
        Tone::Neutral,
        "unnamed: quiet"
    );
    assert_eq!(plan::number_line(&s), "Aktarım 2026/0007, Proje düzeltme");
}
