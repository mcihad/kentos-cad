//! GNSS ağ dengelemesi with known points' own velocities (survey_network_window.rs, plan/map-cad.md MAP-03,
//! docs/adr/2520): on the shared case whose known points carry their Earth-centred velocities and whose campaign has
//! neither a velocity grid nor an epoch decision, both go to the measurement epoch by their velocity, the summary says
//! so and Bilinen noktalar's Taşınma names it; with the grid for one of them the summary says both ways. The known point
//! form takes a velocity (all three components or none). And the pictures.

use kentos_commands::{KnownMovedBy, MapSurveyNetworked};
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::survey_network_window::{Event as N, Field, View};

const OWN: &str = "Bilinen noktaların kendi hızları, arazi ölçümünün epok kararı yok: yeni noktalar ölçme epokunda \
                   projeye taşınır, ağ kabul edilebilir";
const BOTH: &str = "BN1 kendi hızıyla, BN2 arazi ölçümünün hız ızgarasıyla taşınır";

fn case(name: &str) -> Value {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.network.json")).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone()
}

/// The window on a shared case's drawing.
fn opened(name: &str) -> App {
    crate::survey_epoch_tests::grids();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(case(name)["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    assert_eq!(app.dialog, Some(Dialog::SurveyNetwork));
    app
}

fn answer(app: &App) -> MapSurveyNetworked {
    app.survey_network.answer.clone().expect("an answer").expect("adjusted")
}

/// Whether the window draws `text` (Bilinen noktalar's table cell).
fn drawn(app: &mut App, text: &str) -> bool {
    let mut shot = crate::files_testing::offscreen(iced::Size::new(1440.0, 900.0));
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    shot.settle(app, App::view, &mut update);
    crate::files_testing::find_text(&mut shot, app, text).is_some()
}

#[test]
fn own_velocities_carry_the_known_points_without_a_grid() {
    let mut app = opened(OWN);
    let a = answer(&app);
    let by: Vec<(&str, KnownMovedBy)> = a.moved.iter().map(|m| (m.id.as_str(), m.by)).collect();
    assert_eq!(by, [("bn-1", KnownMovedBy::Velocity), ("bn-2", KnownMovedBy::Velocity)]);
    // The case's own answer, within a tenth of a millimetre.
    let want: Vec<Value> = case(OWN)["steps"][0]["result"]["output"]["moved"].as_array().expect("moved").clone();
    for (m, w) in a.moved.iter().zip(&want) {
        for (i, s) in m.shift.iter().enumerate() {
            let w = w["shift"][i].as_f64().expect("a shift");
            assert!((s - w).abs() < 1e-4, "{}: {s} ≠ {w}", m.id);
        }
    }
    assert!(a.acceptable, "{:?}", a.why);
    let line = crate::survey_network_window::epoch_line(&a);
    assert!(line.contains("2 bilinen nokta kendi hızıyla oraya taşındı"), "{line}");
    // Bilinen noktalar's Taşınma names the way.
    let _ = app.update(Message::SurveyNetwork(N::View(View::Known)));
    let mm = |v: f64| format!("{:.1}", v * 1000.0);
    let m = &a.moved[0];
    let cell = format!("hız {}/{}/{}", mm(m.shift[0]), mm(m.shift[1]), mm(m.shift[2]));
    assert!(drawn(&mut app, &cell), "{cell}");
    // One by its velocity, one along the grid: the summary says both.
    let app = opened(BOTH);
    let a = answer(&app);
    assert_eq!(a.moved.iter().map(|m| m.by).collect::<Vec<_>>(), [KnownMovedBy::Velocity, KnownMovedBy::Grid]);
    let line = crate::survey_network_window::epoch_line(&a);
    assert!(line.contains("2 bilinen nokta oraya taşındı: 1 nokta kendi hızıyla, 1 nokta hız ızgarasıyla"), "{line}");
}

#[test]
fn the_known_point_form_takes_a_velocity() {
    let mut app = opened(OWN);
    let _ = app.update(Message::SurveyNetwork(N::View(View::Known)));
    let type_ = |app: &mut App, field: Field, text: &str| {
        let _ = app.update(Message::SurveyNetwork(N::Known(field, text.into())));
    };
    // A third known point on N4 with two velocity components only: refused, nothing added.
    for (field, text) in [
        (Field::Mark, "N4"),
        (Field::Lat, "39.8"),
        (Field::Lon, "30.5"),
        (Field::Height, "850.0"),
        (Field::Epoch, "2005.0"),
        (Field::Velocity(0), "-0.0123"),
        (Field::Velocity(1), "0.0101"),
    ] {
        type_(&mut app, field, text);
    }
    let before = app.document.as_ref().and_then(|d| d.model.map_survey()).map(|s| s.known.len());
    let _ = app.update(Message::SurveyNetwork(N::AddKnown));
    assert_eq!(app.document.as_ref().and_then(|d| d.model.map_survey()).map(|s| s.known.len()), before);
    assert!(crate::files_testing::last_said(&app).contains("Hızın üç bileşenini birden yazın"));
    // The third component: added with its velocity.
    type_(&mut app, Field::Velocity(2), "0.0089");
    let _ = app.update(Message::SurveyNetwork(N::AddKnown));
    let survey = app.document.as_ref().and_then(|d| d.model.map_survey()).cloned().expect("the archive");
    let k = survey.known.iter().find(|k| k.mark == "N4").expect("the new known point");
    assert_eq!(k.velocity, Some([-0.0123, 0.0101, 0.0089]));
    assert_eq!(k.epoch, Some(2005.0));
}

/// The window with own velocities: the summary and Bilinen noktalar (Taşınma by velocity, the form's Hız row), light
/// wide and dark narrow, `.run/gui-evidence/kendi-hizi-2520/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib survey_velocity_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kendi-hizi-2520");
    std::fs::create_dir_all(&out).expect("a folder");
    for (case, stem) in [(OWN, "1-kendi-hizi"), (BOTH, "2-hiz-ve-izgara")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = opened(case);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::SurveyNetwork(N::View(View::Known)));
            let _ = app.update(Message::SurveyNetwork(N::Known(Field::Velocity(0), "-0.0123".into())));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
