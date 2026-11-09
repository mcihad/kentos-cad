//! GNSS arşivi's baselines, known points and adjustments (map_survey_network_rows.rs, plan/map-cad.md MAP-03,
//! docs/adr/2920) on the shared cases' drawing of `map.survey.network`: the network accepted from Ağ dengelemesi is a
//! row under Dengelemeler that selects the three points it made; a baseline left out from its row with the toolbar's
//! reason turns the adjustment out of date and Geri al takes it back; a known point leaves and comes back the same way;
//! the adjustment's button opens Ağ dengelemesi. And the pictures.

use kentos_launch::Module;
use kentos_ui::widget::Status;
use serde_json::Value;

use crate::app::{App, Dialog, Message, Panel};
use crate::map_survey_network_rows::{adjustment_state, made_by};
use crate::map_survey_panel::Event as P;
use crate::survey_network_window::{Class, Event as N};

fn opened() -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/map.survey.network.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The network adjusted in class C2 and accepted from Ağ dengelemesi, the window closed.
fn accepted() -> App {
    let mut app = opened();
    let _ = app.update(Message::Run("map.survey.adjustment"));
    let _ = app.update(Message::SurveyNetwork(N::Class(Class::C2)));
    let _ = app.update(Message::SurveyNetwork(N::Accept));
    app.dialog = None;
    app
}

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::MapSurvey(e));
}

fn tree(app: &App) -> kentos_commands::MapSurveyTree {
    app.map_survey_read().expect("a drawing").expect("a map project")
}

#[test]
fn the_accepted_adjustment_selects_its_points_and_goes_out_of_date_with_a_baseline() {
    let mut app = accepted();
    let t = tree(&app);
    assert_eq!((t.baselines.len(), t.known.len(), t.adjustments.len()), (8, 2, 1));
    let a = &t.adjustments[0];
    let (tone, words, out) = adjustment_state(&t, a);
    assert_eq!((tone, words, out.len()), (Status::Info, "Kabul edildi", 0));
    // Its row selects the three points it made.
    let made = made_by(&t, a);
    assert_eq!(made.len(), 3);
    send(&mut app, P::Objects(made));
    assert_eq!(app.selection.len(), 3);
    // A baseline left out without a reason is not; with one it is, and the adjustment is out of date.
    send(&mut app, P::Exclude("bz-8".into()));
    assert!(tree(&app).baselines.iter().find(|b| b.id == "bz-8").is_some_and(|b| b.excluded.is_none()));
    send(&mut app, P::Text("Çok bağlantılı, şüpheli".into()));
    send(&mut app, P::Exclude("bz-8".into()));
    let t = tree(&app);
    assert_eq!(t.baselines.iter().find(|b| b.id == "bz-8").and_then(|b| b.excluded.clone()).as_deref(), Some("Çok bağlantılı, şüpheli"));
    let (tone, words, out) = adjustment_state(&t, &t.adjustments[0]);
    assert_eq!((tone, words, out), (Status::Warning, "Güncel değil", vec!["bz-8"]));
    // Geri al: accepted again.
    send(&mut app, P::Include("bz-8".into()));
    let t = tree(&app);
    assert_eq!(adjustment_state(&t, &t.adjustments[0]).1, "Kabul edildi");
    // A known point leaves and comes back the same way.
    let known = t.known[1].id.clone();
    send(&mut app, P::Text("Röper bozulmuş".into()));
    send(&mut app, P::Exclude(known.clone()));
    let t = tree(&app);
    assert!(t.known.iter().find(|k| k.id == known).is_some_and(|k| k.excluded.is_some()));
    assert_eq!(adjustment_state(&t, &t.adjustments[0]).2, vec![known.as_str()]);
    send(&mut app, P::Include(known.clone()));
    assert!(tree(&app).known.iter().all(|k| k.excluded.is_none()));
    // The adjustment's button opens Ağ dengelemesi.
    send(&mut app, P::Network);
    assert_eq!(app.dialog, Some(Dialog::SurveyNetwork));
}

/// The archive panel with Bazlar, Bilinen noktalar and Dengelemeler, a baseline left out (the adjustment out of
/// date); light wide and dark narrow, `.run/gui-evidence/arsivde-bazlar-2920/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_survey_network_rows_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/arsivde-bazlar-2920");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = accepted();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.show_map_survey();
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.72, 0.28]));
        send(&mut app, P::Text("Çok bağlantılı, şüpheli".into()));
        send(&mut app, P::Exclude("bz-8".into()));
        // The campaigns', the marks' and the uses' branches folded, the baselines' opened: the network's rows in sight.
        for key in ["campaigns", "marks", "uses", "baselines"] {
            send(&mut app, P::Fold(key.into()));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("arsiv-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// docs/adr/3320: a baseline's row draws it on the map between its ends' accepted points, with its id and length,
/// besides selecting them; the drawing's points are its ends (a point moved, the line follows); İşareti kaldır takes
/// it away, and is offered only while something is marked.
#[test]
fn a_baseline_s_row_draws_it_between_its_ends() {
    let mut app = accepted();
    let t = tree(&app);
    let accepted_end = |m: &str| !crate::map_survey_network_rows::objects_of(&t, &[m]).is_empty();
    // A baseline from a known point without an accepted point of its own is not drawn, and the window says why.
    let lone = t.baselines.iter().find(|b| !(accepted_end(&b.from) && accepted_end(&b.to))).cloned().expect("one");
    send(&mut app, P::Baseline(lone.id.clone()));
    assert!(app.baseline_marks().is_none());
    let said = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains(&format!("{} çizilemedi", lone.id)), "{said}");
    // One between two accepted points is.
    let b = t.baselines.iter().find(|b| accepted_end(&b.from) && accepted_end(&b.to)).cloned().expect("one");
    send(&mut app, P::Baseline(b.id.clone()));
    assert_eq!(app.selection.len(), 2, "its two ends selected");
    let mark = app.baseline_marks().expect("drawn");
    assert_eq!(mark.words, format!("{} · {:.3} m", b.id, b.length));
    // The ends are the accepted points' objects where the drawing holds them.
    let place = |mark_name: &str| {
        let uid = crate::map_survey_network_rows::objects_of(&t, &[mark_name]).into_iter().next().expect("accepted");
        let doc = app.document.as_ref().expect("open");
        let slot = doc.model.slot_of(kentos_domain::Uuid::parse_str(&uid).expect("a uid")).expect("drawn");
        match doc.model.get(slot) {
            Some(kentos_commands::Entity::Point(p)) => [p.p.x, p.p.y],
            _ => panic!("a point"),
        }
    };
    assert_eq!((mark.from, mark.to), (place(&b.from), place(&b.to)));
    assert!(app.available("data.unmark"), "something is marked");
    let _ = app.update(Message::Run("data.unmark"));
    assert!(app.baseline_marks().is_none());
}

/// The baseline drawn from its row, light wide and dark narrow, `.run/gui-evidence/baz-haritada-3320/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_survey_network_rows_tests::baseline_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn baseline_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/baz-haritada-3320");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = accepted();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.show_map_survey();
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.72, 0.28]));
        for key in ["campaigns", "marks", "uses", "baselines"] {
            send(&mut app, P::Fold(key.into()));
        }
        let _ = app.update(Message::Run("view.zoomExtents"));
        let t = tree(&app);
        let accepted_end = |m: &str| !crate::map_survey_network_rows::objects_of(&t, &[m]).is_empty();
        let b = t.baselines.iter().find(|b| accepted_end(&b.from) && accepted_end(&b.to)).expect("one").id.clone();
        send(&mut app, P::Baseline(b));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("baz-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
