//! GNSS baseline sessions, new points' velocities and the kept place limits on the desktop (survey_network_window.rs,
//! map_survey_panel.rs; plan/map-cad.md MAP-03, docs/adr/4920), on the shared cases of `map.survey.network`:
//! Oturum korelasyonları puts both sessions in and the summary says so, Bazlar shows each baseline's session; Yeni
//! noktaların hızı reads the new marks' velocities from the campaign's grid (Noktalar's Hız) and Kabul et keeps them;
//! GNSS arşivi's Projeye yaz keeps the typed limits with their source, and they judge the known points' places. And the
//! pictures.

use kentos_commands::NetworkCorrelations;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message, Panel};
use crate::map_survey_panel::Event as P;
use crate::survey_network_window::{Event as N, View};

const SESSIONS: &str = "İki baz oturumu";
const MOVING: &str = "Yeni noktaların hızı arazi ölçümünün";

/// A shared case's drawing, GNSS ağ dengelemesi open on it.
fn opened(starts: &str) -> App {
    crate::survey_epoch_tests::grids();
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/map.survey.network.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(starts)))
        .expect("the case")
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    assert_eq!(app.dialog, Some(Dialog::SurveyNetwork));
    app
}

fn send(app: &mut App, e: N) {
    let _ = app.update(Message::SurveyNetwork(e));
}

fn answer(app: &App) -> kentos_commands::MapSurveyNetworked {
    app.survey_network.answer.clone().expect("an answer").expect("adjusted")
}

#[test]
fn the_sessions_go_in_when_asked_and_the_summary_says_which() {
    let mut app = opened(SESSIONS);
    let plain = answer(&app);
    let c = plain.correlations.as_ref().expect("said");
    assert_eq!((c.used, c.unused.clone()), (NetworkCorrelations::None, vec!["bo-1".to_owned(), "bo-2".to_owned()]));
    let (_, text) = crate::survey_network_window::sessions_line(&plain).expect("a line");
    assert!(text.contains("kullanılmadı"), "{text}");
    send(&mut app, N::Correlated(true));
    let with = answer(&app);
    let c = with.correlations.as_ref().expect("said");
    assert_eq!((c.used, c.sessions.len()), (NetworkCorrelations::Session, 2));
    let (_, text) = crate::survey_network_window::sessions_line(&with).expect("a line");
    assert!(text.starts_with("2 baz oturumu (bo-1, bo-2) tam kovaryansıyla girdi"), "{text}");
    assert!(with.acceptable, "{:?}", with.why);
    // Bazlar: each baseline's session, from the archive.
    send(&mut app, N::View(View::Baselines));
    let survey = app.document.as_ref().and_then(|d| d.model.map_survey()).expect("an archive");
    assert_eq!(survey.session_of("bz-6").map(|s| s.id.as_str()), Some("bo-2"));
    // Kabul et keeps the sessions in the adjustment's record, in one step.
    send(&mut app, N::Accept);
    let survey = app.document.as_ref().and_then(|d| d.model.map_survey()).expect("an archive");
    assert_eq!(survey.adjustments.last().map(|a| a.sessions.clone()), Some(vec!["bo-1".to_owned(), "bo-2".to_owned()]));
}

#[test]
fn the_new_marks_velocities_come_from_the_grid_and_are_kept() {
    let mut app = opened(MOVING);
    send(&mut app, N::Velocities(true));
    let a = answer(&app);
    assert_eq!(a.velocity_grid.as_ref().map(|g| g.name.as_str()), Some("Hız alanı (sentetik)"));
    let n1 = a.stations.iter().find(|s| s.mark == "N1").expect("N1");
    let v = n1.velocity.expect("its velocity");
    // ~2.4 cm a year west, 0.8 cm north on the synthetic grid.
    assert!(v[0] < -0.02 && v[1] > 0.005, "{v:?}");
    assert!(a.stations.iter().filter(|s| s.known.is_some()).all(|s| s.velocity.is_none()), "known points have none");
    send(&mut app, N::Accept);
    let survey = app.document.as_ref().and_then(|d| d.model.map_survey()).expect("an archive");
    let kept = survey.points.iter().find(|p| p.mark == "N1").and_then(|p| p.result.velocity.clone()).expect("kept");
    assert_eq!(kept.enu, v);
}

#[test]
fn the_limits_are_kept_with_their_source_and_judge_the_places() {
    let mut app = opened(SESSIONS);
    send(&mut app, N::Close);
    let mp = |app: &mut App, e: P| {
        let _ = app.update(Message::MapSurvey(e));
    };
    mp(&mut app, P::LimitHorizontal("0,02".into()));
    mp(&mut app, P::LimitSave);
    let kept = |app: &App| app.document.as_ref().and_then(|d| d.model.map_survey()).and_then(|s| s.tolerance.clone());
    assert_eq!(kept(&app), None, "no source: not written");
    mp(&mut app, P::LimitSource("İdarenin şartnamesi".into()));
    mp(&mut app, P::LimitSave);
    let t = kept(&app).expect("written");
    assert_eq!((t.horizontal, t.vertical, t.source.as_str()), (Some(0.02), None, "İdarenin şartnamesi"));
    // The typed boxes emptied: the kept limits judge the places (the tree says them).
    mp(&mut app, P::LimitHorizontal(String::new()));
    let tree = app.map_survey_read().expect("a drawing").expect("an archive");
    assert_eq!(tree.tolerance.map(|t| t.horizontal), Some(Some(0.02)));
    // Nothing typed: Projeye yaz takes the kept limits away; one undo brings them back.
    mp(&mut app, P::LimitSave);
    assert_eq!(kept(&app), None);
    let _ = app.update(Message::Run("edit.undo"));
    assert!(kept(&app).is_some());
}

/// `.run/gui-evidence/gnss-oturum-hiz-4920/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib gnss_sessions_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/gnss-oturum-hiz-4920");
    std::fs::create_dir_all(&out).expect("a folder");
    let shoot = |app: &mut App, size: Size, file: &str| {
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(file);
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    };
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let setup = |app: &mut App| {
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
        };
        let mut app = opened(SESSIONS);
        setup(&mut app);
        send(&mut app, N::Correlated(true));
        shoot(&mut app, size, &format!("1-oturum-korelasyonlari-{name}.png"));
        send(&mut app, N::View(View::Baselines));
        shoot(&mut app, size, &format!("2-bazlarin-oturumu-{name}.png"));
        let mut app = opened(MOVING);
        setup(&mut app);
        send(&mut app, N::Velocities(true));
        shoot(&mut app, size, &format!("3-yeni-noktalarin-hizi-{name}.png"));
        send(&mut app, N::Close);
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.68, 0.32]));
        let _ = app.update(Message::MapSurvey(P::LimitHorizontal("0,02".into())));
        let _ = app.update(Message::MapSurvey(P::LimitVertical("0,03".into())));
        let _ = app.update(Message::MapSurvey(P::LimitSource("İdarenin şartnamesi (örnek)".into())));
        let _ = app.update(Message::MapSurvey(P::LimitSave));
        shoot(&mut app, size, &format!("4-kabul-farki-projede-{name}.png"));
    }
}
