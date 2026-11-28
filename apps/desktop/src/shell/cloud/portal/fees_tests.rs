//! Harç in Portal dosyaları (docs/adr/9003), without a server: an application's assessments read after its page, the
//! tab offered once there is one, its lines, total, state and payment note drawn; a picture for the owner.

use kentos_commands::{
    DossierAction, DossierDirection, DossierStatus, FeeAssessment, FeeAssessmentState, FeeLine,
};

use super::fixtures::{APPLICATION, detail, summary};
use super::tests::{answer_detail, loaded, portal};
use super::{Event, Tab, msg};
use crate::app::App;

fn assessment(state: FeeAssessmentState) -> FeeAssessment {
    let line = |code: &str, label: &str, basis: &str, legal: &str, amount: i64| FeeLine {
        code: code.into(),
        label: label.into(),
        group: "Harç".into(),
        legal_basis: legal.into(),
        basis: basis.into(),
        amount,
        vat: 0,
        before_decision: true,
    };
    FeeAssessment {
        id: "0fee0000-0000-7000-8000-000000000001".into(),
        schedule_id: "0fee0000-0000-7000-8000-0000000000aa".into(),
        schedule_name: "2026 Ücret Tarifesi".into(),
        inputs: Default::default(),
        lines: vec![
            line(
                "bina_insaat_harci",
                "Bina inşaat harcı",
                "452,6 m² × 50 TL (Konut, 201 m² ve üstü)",
                "2464 sayılı Kanun Ek Madde 1, 5 ve 6",
                2_263_000,
            ),
            line(
                "proje_tasdik_harci",
                "Plan ve proje tasdik harcı",
                "452,6 m² × 1,25 TL (Konut bölgesi)",
                "2464 sayılı Kanun m.80/c ve m.84",
                56_575,
            ),
        ],
        total: 2_319_575,
        state,
        due_on: Some("2026-12-02".into()),
        issued_at: "2026-11-18T08:44:00Z".into(),
        issued_by_name: "Zeynep Demir".into(),
        paid_at: None,
        receipt_no: None,
        paid_recorded_by_name: None,
        cancel_reason: None,
        payment_note: Some(
            "Belediye veznesine ya da banka hesabına dosya numarasıyla ödenir.".into(),
        ),
    }
}

/// Ayşe's application chosen, its page answered with one assessment.
fn with_fees() -> App {
    let s = summary(
        APPLICATION,
        DossierDirection::Application,
        DossierStatus::Submitted,
    );
    let mut app = loaded(vec![s.clone()]);
    let _ = app.update(msg(Event::Choose(APPLICATION.to_owned())));
    let mut d = detail(s, vec![DossierAction::Withdraw]);
    d.fee_count = 1;
    answer_detail(&mut app, d);
    portal(
        &mut app,
        Event::Fees {
            dossier: APPLICATION.to_owned(),
            result: Ok(vec![assessment(FeeAssessmentState::Issued)]),
        },
    );
    app
}

#[test]
fn an_application_s_fees_are_read_after_its_page_and_drawn_in_their_tab() {
    let mut app = with_fees();
    let p = app.cloud.portal.as_ref().expect("open");
    let (dossier, list) = p.fees.as_ref().expect("the fees came");
    assert_eq!(
        (dossier.as_str(), list.len(), list[0].total),
        (APPLICATION, 1, 2_319_575)
    );
    portal(&mut app, Event::Tab(Tab::Fees));
    assert_eq!(app.cloud.portal.as_ref().expect("open").tab, Tab::Fees);
    // The tab draws: its lines, total and payment note (built, not asserted pixel by pixel).
    let _ = app.view();
    // Another dossier's late answer does not land on this one.
    portal(
        &mut app,
        Event::Fees {
            dossier: "baska".into(),
            result: Ok(Vec::new()),
        },
    );
    assert_eq!(
        app.cloud
            .portal
            .as_ref()
            .expect("open")
            .fees
            .as_ref()
            .map(|f| f.1.len()),
        Some(1)
    );
}

/// Pictures for the owner: Harç over an application in dark and light at 1440×900. Not run by default:
/// `cargo test -p kentos-desktop --lib cloud::portal::fees_tests::fees_screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn fees_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/portal");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", "koyu"), ("light", "acik")] {
        let mut app = with_fees();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        portal(&mut app, Event::Tab(Tab::Fees));
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("masaustu-harc-{suffix}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}
