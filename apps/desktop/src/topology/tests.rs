//! The Topoloji tab and the rules window over
//! `fixtures/interaction/v1/topology-rules.kcad` (docs/adr/0202): the check
//! finds what the core finds, a row shows its finding, a fix is one undo
//! step and the check runs again, exceptions go to the project's settings;
//! the window's rows, problems and Kaydet. The trace `topology-rules.json`
//! plays the flow with the mouse; the rules themselves are the core's
//! (fixtures/topology-rules/v1).

use kentos_contracts::{TopologyRuleKind, TopologySettings};
use kentos_domain::Slot;

use super::plan::{self, Filter};
use super::rules::{self, RuleRow};
use super::{Event, texts};
use crate::app::{App, Message};
use crate::bottom::BottomTab;

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/topology-rules.kcad");

fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Topology(e));
}

fn checked() -> App {
    let mut app = app();
    let _ = app.update(Message::Run("topology.check"));
    app
}

fn topology(app: &App) -> TopologySettings {
    app.document
        .as_ref()
        .and_then(|d| d.model.settings().topology.clone())
        .expect("the project has topology settings")
}

#[test]
fn the_check_opens_the_tab_and_lists_the_findings_in_the_rules_order() {
    let app = checked();
    assert_eq!(app.bottom_tab, BottomTab::Topology);
    let (count, rows) = app.topology_seen().expect("the tab is open");
    assert_eq!(count, "19 bulgu (açık 19, istisna 0)");
    assert_eq!(
        rows[0],
        "Parsel | Çakışmamalı | Çakışma | #2, #3 | 12.00 m²"
    );
    assert_eq!(
        rows[1],
        "Parsel | Boşluk olmamalı | Boşluk | #5, #6, #4, #7 | 200.00 m²"
    );
    assert_eq!(
        rows[12],
        "Bina | Parsel içinde kalmalı | Dışarıda kalan | #14 | 48.00 m²"
    );
    assert_eq!(
        rows[16],
        "Yol | Sarkan uç olmamalı | Sarkan uç | #17 | 0.600 m"
    );
}

#[test]
fn a_row_selects_its_objects_and_shows_its_finding() {
    let mut app = checked();
    ev(&mut app, Event::Press(1));
    let mut chosen: Vec<u32> = app.selection.ids().iter().map(|s| s.0).collect();
    chosen.sort_unstable();
    assert_eq!(chosen, [4, 5, 6, 7]);
    let mark = app.topology_problem().expect("the gap is shown");
    assert_eq!(mark.label, "Boşluk");
    assert_eq!(mark.regions.len(), 1);
}

#[test]
fn a_fix_is_one_undo_step_and_the_check_runs_again() {
    let mut app = checked();
    ev(&mut app, Event::Press(0));
    ev(&mut app, Event::Fix("subtractFirst"));
    let (count, rows) = app.topology_seen().expect("the tab is open");
    assert_eq!(count, "14 bulgu (açık 14, istisna 0)", "{rows:#?}");
    let doc = &mut app.document.as_mut().expect("open").model;
    // The second parcel ends where the third begins.
    let Some(kentos_contracts::Entity::Polygon(p)) = doc.get(Slot(2)) else {
        panic!("a polygon");
    };
    assert!(p.pts.iter().all(|q| q.x <= 487_040.0 + 1e-9), "{:?}", p.pts);
    assert_eq!(doc.undo().as_deref(), Some("Topoloji düzelt"));
    assert!(app.topology_stale(), "undone, the check is old");
}

#[test]
fn exceptions_are_kept_in_the_project_and_filtered() {
    let mut app = checked();
    let shown = app.topology_shown();
    let last = shown.len() - 1;
    ev(&mut app, Event::Press(last));
    ev(&mut app, Event::Exception(true));
    assert_eq!(topology(&app).exceptions.len(), 1);
    assert_eq!(
        app.topology_seen().expect("open").0,
        "18 bulgu (açık 18, istisna 1)"
    );
    ev(&mut app, Event::Filter(Filter::Exception));
    assert_eq!(app.topology_seen().expect("open").1.len(), 1);
    // Checked again, the finding is still an exception; taken off, it is open.
    ev(&mut app, Event::Check);
    assert_eq!(
        app.topology_seen().expect("open").0,
        "1 bulgu (açık 18, istisna 1)"
    );
    ev(&mut app, Event::Press(0));
    ev(&mut app, Event::Exception(false));
    assert!(topology(&app).exceptions.is_empty());
}

#[test]
fn the_words_are_the_webs() {
    assert_eq!(
        plan::COLUMNS,
        ["Sıra", "Katman", "Kural", "Sorun", "Nesneler", "Ölçü"]
    );
    assert_eq!(plan::count_text(2, 2, 2), "2 bulgu (açık 2, istisna 2)");
    let b = kentos_geometry_core::geometry::Bounds {
        min_x: 5.0,
        min_y: 5.0,
        max_x: 5.0,
        max_y: 5.0,
    };
    let v = plan::finding_view(&b);
    assert_eq!((v.min_x, v.max_x), (-2.0, 12.0));
    assert_eq!(
        texts::NO_RULES,
        "Projede topoloji kuralı yok. Kurallar… ile katmanlara kural ekleyin."
    );
}

#[test]
fn the_rules_window_writes_the_rules_and_refuses_what_does_not_hold() {
    let mut app = app();
    let _ = app.update(Message::Run("topology.rules"));
    assert!(app.topology_rules.is_some());
    let _ = app.update(Message::TopologyRules(rules::Event::Add));
    // The new rule: on the active layer, Çakışmamalı; made a rule between layers without its other layer.
    let between = TopologyRuleKind::ALL
        .iter()
        .position(|k| *k == TopologyRuleKind::MustNotOverlapWith)
        .expect("a kind");
    let _ = app.update(Message::TopologyRules(rules::Event::Kind(6, between)));
    let _ = app.update(Message::TopologyRules(rules::Event::Save));
    assert_eq!(topology(&app).rules.len(), 6, "refused: no other layer");
    // Its other layer chosen (Yol, the third in the list), it is kept.
    let _ = app.update(Message::TopologyRules(rules::Event::Other(6, 2)));
    let _ = app.update(Message::TopologyRules(rules::Event::Save));
    let t = topology(&app);
    assert_eq!(t.rules.len(), 7);
    assert_eq!(t.rules[6].kind, TopologyRuleKind::MustNotOverlapWith);
    assert_eq!(t.rules[6].other.as_deref(), Some("yol"));
    assert!(app.topology_rules.is_none());
}

#[test]
fn a_row_types_its_value_in_the_projects_unit() {
    let app = app();
    let format = app.format();
    let r = RuleRow {
        id: "r".into(),
        kind: TopologyRuleKind::MustNotHaveSmallAngles,
        layer: "parsel".into(),
        other: String::new(),
        value: "10".into(),
    };
    let rule = rules::rule_of(&r, &format);
    // The project's angles are in grad: 10 grad.
    assert!((rule.value.expect("a value") - std::f64::consts::PI / 20.0).abs() < 1e-15);
    assert_eq!(rules::row_of(&rule, &format).value, "10");
    let name = |id: &str| (id == "parsel").then(|| "Kadastro / Parsel".to_owned());
    assert!(
        rules::rules_problem("2", std::slice::from_ref(&r), &format, name)
            .is_some_and(|p| p.contains("Tolerans"))
    );
    let wide = RuleRow {
        value: "100".into(),
        ..r
    };
    assert!(
        rules::rules_problem("0.001", &[wide], &format, name)
            .is_some_and(|p| p.contains("dik açıdan"))
    );
}

/// `cargo test -p kentos-desktop topology::tests::screens -- --ignored --nocapture`: the
/// Topoloji tab after Denetle with the overlap chosen, the gap chosen, Düzelt ▾ on it,
/// the rules window, the ribbon's Topoloji panel; `.run/shots/topoloji-*` (the web's
/// `shots.mjs topologyrules`).
#[test]
#[ignore]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["bulgular", "bosluk", "duzelt", "kurallar", "serit"] {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let mut app = app();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let _ = app.update(Message::Run("view.zoomExtents"));
                let _ = app.update(Message::Run("topology.check"));
                snapshot.settle(&mut app, App::view, &mut update);
                ev(
                    &mut app,
                    Event::Press(if name == "bulgular" { 0 } else { 1 }),
                );
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "duzelt" {
                    // Düzelt ▾ opened: pressed where the bar shows it.
                    let at = crate::files_testing::find_text(&mut snapshot, &app, texts::FIX)
                        .expect("Düzelt on the bar")
                        .center();
                    snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
                }
                if name == "kurallar" {
                    let _ = app.update(Message::Run("topology.rules"));
                }
                if name == "serit" {
                    // The CBS ribbon's Analiz: its Topoloji panel beside Karşılaştırma.
                    let _ = app.update(Message::RibbonTab("analysis"));
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("topoloji-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
