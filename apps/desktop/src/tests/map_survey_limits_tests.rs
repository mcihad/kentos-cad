//! GNSS arşivi's limits of the known points' places against their accepted points (map_survey_panel.rs,
//! map_survey_network_rows.rs, plan/map-cad.md MAP-03, docs/adr/4520), on the shared case's drawing of
//! `map.survey.query` where N1 is both a known point and an accepted one (its own place 6 mm east, 4 mm south and 12 mm
//! up of its accepted point). The panel reads the places judged against the limits typed: 5 mm horizontally, N1 is
//! beyond and its row says so; 1 cm, it is within; a text that is no length above 0 is not a limit. And the pictures.

use crate::map;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message, Panel};
use crate::map_survey_panel::Event as P;

fn opened() -> App {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.query.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Kabul farkı sınırı: yatay 5 mm")))
        .cloned()
        .expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::Map(map::Event::MapSurvey(e)));
}

/// N1's place as the panel reads it: beyond or not (none: nothing judged).
fn n1_beyond(app: &App) -> Option<bool> {
    let tree = app.map_survey_read().expect("a drawing").expect("an archive");
    tree.known_places.iter().find(|p| p.mark == "N1").and_then(|p| p.beyond)
}

#[test]
fn the_limits_typed_judge_the_known_points_places() {
    let mut app = opened();
    assert_eq!(n1_beyond(&app), None, "no limit: nothing judged");
    send(&mut app, P::LimitHorizontal("0,005".into()));
    assert_eq!(n1_beyond(&app), Some(true), "7.2 mm beyond 5 mm");
    let tree = app.map_survey_read().expect("a drawing").expect("an archive");
    let n1 = tree.known_places.iter().find(|p| p.mark == "N1").expect("N1's place");
    let said = crate::map_survey_network_rows::place_words(n1);
    assert!(said.iter().any(|w| w == "Kabul farkı sınırını aşıyor."), "{said:?}");
    send(&mut app, P::LimitHorizontal("0,01".into()));
    assert_eq!(n1_beyond(&app), Some(false), "within 1 cm");
    send(&mut app, P::LimitVertical("0,01".into()));
    assert_eq!(n1_beyond(&app), Some(true), "12 mm up beyond 1 cm");
    // A text that is no length above 0 is no limit.
    send(&mut app, P::LimitHorizontal("abc".into()));
    send(&mut app, P::LimitVertical("0".into()));
    assert_eq!(app.map_survey_limits(), None);
    assert_eq!(n1_beyond(&app), None);
}

/// `.run/gui-evidence/kabul-farki-siniri-4520/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_survey_limits_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kabul-farki-siniri-4520");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.68, 0.32]));
        send(&mut app, P::LimitHorizontal("0,005".into()));
        send(&mut app, P::LimitVertical("0,02".into()));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-kabul-farki-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
