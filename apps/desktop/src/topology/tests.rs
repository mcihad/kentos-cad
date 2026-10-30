//! The Topoloji tab and the rules window over
//! `fixtures/interaction/v1/topology-rules.kcad` (docs/adr/0202): the check
//! finds what the core finds, a row shows its finding, a fix is one undo
//! step and the check runs again, exceptions go to the project's settings;
//! the window's rows, problems and Kaydet. The trace `topology-rules.json`
//! plays the flow with the mouse; the rules themselves are the core's
//! (fixtures/topology-rules/v1).

use kentos_commands::{TopologyRuleKind, TopologySettings};
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
        kentos_commands::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Topology(e));
}

/// The drawing with a block of 2 500 parcels beside its own, one of them overlapping its neighbour: enough objects
/// for the check to go to its own thread (docs/adr/0756).
fn big() -> App {
    let mut v: serde_json::Value = serde_json::from_str(DRAWING).expect("the drawing's JSON");
    let entities = v["entities"].as_array_mut().expect("entities");
    let mut id = entities
        .iter()
        .filter_map(|e| e["id"].as_u64())
        .max()
        .expect("ids")
        + 1;
    for i in 0..50 {
        for j in 0..50 {
            let (x, y) = (
                490_000.0 + f64::from(i) * 10.0,
                4_420_000.0 + f64::from(j) * 10.0,
            );
            let w = if (i, j) == (7, 7) { 10.5 } else { 10.0 };
            entities.push(serde_json::json!({
                "kind": "polygon", "id": id, "layerId": "parsel", "attrs": {},
                "pts": [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + 10.0}, {"x": x, "y": y + 10.0}]
            }));
            id += 1;
        }
    }
    let (mut app, _) = App::boot(None);
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&v.to_string()).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn a_large_check_goes_on_its_own_thread_and_finds_what_the_core_finds() {
    let mut app = big();
    let task = app.update(Message::Run("topology.check"));
    assert!(app.topology.running.is_some(), "on its own thread");
    assert!(app.topology_checked().is_none(), "not found yet");
    crate::files_testing::drive(&mut app, task);
    assert!(app.topology.running.is_none());
    let c = app.topology_checked().expect("checked");
    let g = kentos_native_application::topology_check::gather(
        &app.document.as_ref().expect("open").model,
        None,
    );
    let want = kentos_native_application::topology_check::run(&g);
    assert!(
        want.findings.len() > 4,
        "the block's overlap and the drawing's own"
    );
    assert_eq!(c.findings.as_slice(), want.findings.as_slice());
    assert!(
        crate::files_testing::last_said(&app).starts_with("Topoloji denetlendi"),
        "{}",
        crate::files_testing::last_said(&app)
    );
    // A fix: checked again over what changed, on the thread too; the row at its place chosen.
    let at = c
        .findings
        .iter()
        .position(|f| f.problem == "overlap" && f.at.x > 489_000.0)
        .expect("the block's overlap");
    let row = app
        .topology_shown()
        .iter()
        .position(|&i| i == at)
        .expect("shown");
    ev(&mut app, Event::Press(row));
    let task = app.update(Message::Topology(Event::Fix("subtractFirst")));
    assert!(app.topology.running.is_some(), "the recheck on its thread");
    crate::files_testing::drive(&mut app, task);
    let c = app.topology_checked().expect("rechecked");
    assert!(c.incremental.is_some(), "over what changed");
    assert!(
        !c.findings
            .iter()
            .any(|f| f.problem == "overlap" && f.at.x > 489_000.0),
        "the block's overlap fixed"
    );
    let g = kentos_native_application::topology_check::gather(
        &app.document.as_ref().expect("open").model,
        None,
    );
    let all = kentos_native_application::topology_check::run(&g);
    assert_eq!(
        c.findings.len(),
        all.findings.len(),
        "what checking all finds"
    );
}

#[test]
fn durdur_ends_a_check_and_keeps_what_was_found() {
    let mut app = big();
    let task = app.update(Message::Run("topology.check"));
    crate::files_testing::drive(&mut app, task);
    let before = app.topology_checked().expect("checked").findings.clone();
    let task = app.update(Message::Topology(Event::Check));
    assert!(app.topology.running.is_some());
    ev(&mut app, Event::Stop);
    assert!(app.topology.running.is_none(), "ended at once");
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Topoloji denetimi durduruldu."
    );
    // The thread's later words are not waited for: the last check stays.
    crate::files_testing::drive(&mut app, task);
    assert!(app.topology.running.is_none());
    assert_eq!(app.topology_checked().expect("kept").findings, before);
}

#[test]
fn the_bar_moves_on_smoothly_and_through_a_stretch_by_the_time() {
    use super::running::Reply;
    let mut app = big();
    let task = app.update(Message::Run("topology.check"));
    let id = app.topology.running.as_ref().expect("running").id;
    let mut r = app.topology.running.take().expect("running");
    let t0 = std::time::Instant::now();
    r.heard(&Reply::Progress(0.5));
    r.frame(t0 + std::time::Duration::from_millis(16));
    let first = r.shown;
    assert!(first > 0.0 && first < 0.5, "eased, not jumped: {first}");
    for k in 2..60 {
        r.frame(t0 + std::time::Duration::from_millis(16 * k));
    }
    assert!(
        (r.shown - 0.5).abs() < 0.01,
        "at what was said: {}",
        r.shown
    );
    // An overlay expected to take a second: on by the time, never to its end.
    r.heard(&Reply::Stretch(0.5, 0.9, 1.0));
    let mut last = r.shown;
    for k in 60..400 {
        r.frame(t0 + std::time::Duration::from_millis(16 * k));
        assert!(r.shown >= last, "never back");
        last = r.shown;
    }
    assert!(r.shown > 0.8 && r.shown < 0.9, "{}", r.shown);
    r.stop();
    drop(task);
    assert_eq!(id, r.id);
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
    let Some(kentos_commands::Entity::Polygon(p)) = doc.get(Slot(2)) else {
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
/// A part bound to `fixtures/geopackage/v1/topoloji-parca.gpkg` over `extent` (docs/adr/0589 §1), the rule
/// Çakışmamalı on its layer.
fn part(extent: [f64; 4]) -> App {
    let (mut app, _) = App::boot(None);
    let setup = serde_json::json!({
        "format": "kentos.document", "version": 1, "name": "Topoloji: bağlı parça",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis",
                     "topology": {"tolerance": 0.001,
                                  "rules": [{"id": "p1", "kind": "mustNotOverlap", "layer": "import-parsel"}]}},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false,
                    "expanded": true, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
                    "children": []}],
        "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []}
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/geopackage/v1/topoloji-parca.gpkg");
    let model = &mut app.document.as_mut().expect("open").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(model),
        "cad.sources.edit",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "edit": { "kind": "link", "path": file, "table": "parsel", "extent": extent } }),
    );
    assert!(answer.is_ok(), "{answer:?}");
    app
}

const E: f64 = 487_000.0;
const N: f64 = 4_420_000.0;
const AREA: [f64; 4] = [E - 1.0, N - 1.0, E + 10.5, N + 11.0];
const NARROW: [f64; 4] = [E - 1.0, N - 1.0, E + 2.0, N + 11.0];

/// A bound part's row overlaps one outside its area: the finding names it “dışarıda”; the outside rows' own
/// overlap is not the part's (docs/adr/0589 §1).
#[test]
fn a_parts_neighbour_outside_its_area_is_checked() {
    let mut app = part(AREA);
    let _ = app.update(Message::Run("topology.check"));
    let (count, rows) = app.topology_seen().expect("the tab is open");
    assert_eq!(count, "1 bulgu (açık 1, istisna 0)", "{rows:#?}");
    assert!(rows[0].contains("| #1, dışarıda |"), "{}", rows[0]);
    let c = app.topology_checked().expect("checked");
    assert!(c.g.incomplete.is_empty());
    // Düzelt ▾ says what each fix would do: the part's row may change, the outside row may not.
    ev(&mut app, Event::Press(0));
    let previews = app.topology.previews.clone();
    let first = &previews.iter().find(|(k, _)| *k == "subtractFirst").expect("a preview").1;
    assert_eq!(first.as_deref(), Ok("alan 110.00 m² → 107.50 m²; “parsel” tablosuna 1 satır"));
    let second = &previews.iter().find(|(k, _)| *k == "subtractSecond").expect("a preview").1;
    assert!(second.as_ref().is_err_and(|w| w.contains("alanın dışındaki bir satırı değiştirir")), "{second:?}");
}

/// A part's object reaching out of the band: the check is not complete and says so (docs/adr/0589 §1).
#[test]
fn a_part_reaching_out_of_its_band_is_not_said_clean() {
    let mut app = part(NARROW);
    let _ = app.update(Message::Run("topology.check"));
    let c = app.topology_checked().expect("checked");
    assert_eq!(c.g.incomplete.len(), 1);
    assert!(c.findings.is_empty());
    assert!(c.g.incomplete[0].reason.starts_with("parçanın 1 nesnesi okunan bandın dışına uzanıyor"));
    assert!(plan::not_clean(1, 1).starts_with("Bulgu yok ama denetim eksik"));
}

/// An exception bound to its objects' geometry lapses once it changes: the row says it (docs/adr/0589 §3).
#[test]
fn a_lapsed_exception_is_said_on_its_row() {
    let mut app = checked();
    ev(&mut app, Event::Press(0));
    ev(&mut app, Event::Exception(true));
    let x = &topology(&app).exceptions[0];
    assert_eq!(x.digest.as_ref().map(String::len), Some(16), "bound to the geometry");
    // Its digest made old as a change of the objects' geometry would.
    let doc = &mut app.document.as_mut().expect("open").model;
    let mut settings = doc.settings().clone();
    if let Some(t) = settings.topology.as_mut() {
        t.exceptions[0].digest = Some("0000000000000000".into());
    }
    doc.set_settings(settings);
    ev(&mut app, Event::Check);
    let (count, rows) = app.topology_seen().expect("the tab is open");
    assert_eq!(count, "19 bulgu (açık 19, istisna 0)");
    assert!(rows[0].contains("Çakışma (istisna düştü)"), "{}", rows[0]);
}

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
            for name in [
                "bulgular",
                "bosluk",
                "duzelt",
                "kurallar",
                "serit",
                "degisti",
                "yeniden",
                "dustu",
                "parca",
                "parca-duzelt",
                "eksik",
                "suruyor",
            ] {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let mut app = match name {
                    "parca" | "parca-duzelt" => part(AREA),
                    "eksik" => part(NARROW),
                    "suruyor" => big(),
                    _ => app(),
                };
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
                let task = app.update(Message::Run("topology.check"));
                if name == "suruyor" {
                    // A check on its own thread, its bar at 42 % (docs/adr/0756): drawn as it goes.
                    if let Some(r) = app.topology.running.as_mut() {
                        r.shown = 0.42;
                    }
                    snapshot.settle(&mut app, App::view, &mut update);
                    let file = out.join(format!("topoloji-{name}-{width}x{height}{suffix}.png"));
                    snapshot
                        .render(app.view(), &app.theme())
                        .save(&file)
                        .expect("writes the picture");
                    println!("{}", file.display());
                    if let Some(r) = app.topology.running.take() {
                        r.stop();
                    }
                    continue;
                }
                crate::files_testing::drive(&mut app, task);
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "dustu" {
                    // An exception whose objects' geometry changed (docs/adr/0589 §3).
                    ev(&mut app, Event::Press(0));
                    ev(&mut app, Event::Exception(true));
                    let doc = &mut app.document.as_mut().expect("open").model;
                    let mut settings = doc.settings().clone();
                    if let Some(t) = settings.topology.as_mut() {
                        t.exceptions[0].digest = Some("0000000000000000".into());
                    }
                    doc.set_settings(settings);
                    let _ = app.update(Message::Run("topology.check"));
                }
                if name != "eksik" {
                    ev(
                        &mut app,
                        Event::Press(if matches!(name, "bulgular" | "dustu" | "parca" | "parca-duzelt") { 0 } else { 1 }),
                    );
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "duzelt" || name == "parca-duzelt" {
                    // Düzelt ▾ opened: pressed where the bar shows it.
                    let at = crate::files_testing::find_text(&mut snapshot, &app, texts::FIX)
                        .expect("Düzelt on the bar")
                        .center();
                    snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
                }
                if name == "kurallar" {
                    let _ = app.update(Message::Run("topology.rules"));
                }
                if name == "degisti" || name == "yeniden" {
                    // The road moved after the check (docs/adr/0524): the findings old, then checked again.
                    move_road(&mut app);
                    if name == "yeniden" {
                        ev(&mut app, Event::Recheck);
                    }
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

/// The road (#17) moved 0.4 m east through `cad.entities.transform`.
fn move_road(app: &mut App) {
    let doc = &mut app.document.as_mut().expect("open").model;
    let uid = doc.uid(Slot(17)).expect("the road").to_string();
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        "cad.entities.transform",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "uids": [uid], "transform": { "kind": "move", "dx": 0.4, "dy": 0.0 } }),
    );
    assert!(answer.is_ok());
}

/// Değişenleri denetle (docs/adr/0524): after an object moves the findings are old; the recheck finds what checking
/// all finds, and the chosen row follows its finding by its lasting key.
#[test]
fn the_recheck_after_a_move_finds_what_checking_all_finds() {
    let mut app = checked();
    ev(&mut app, Event::Press(0));
    let key = {
        let c = app.topology_checked().expect("checked");
        c.key(&c.findings[app.topology.selected[0]])
    };
    move_road(&mut app);
    assert!(app.topology_stale(), "the findings are old");
    ev(&mut app, Event::Recheck);
    assert!(!app.topology_stale());
    let c = app.topology_checked().expect("rechecked");
    assert_eq!(c.incremental, Some(1), "one object changed");
    assert_eq!(c.key(&c.findings[app.topology.selected[0]]), key, "the chosen row follows its finding");
    let mut again = app.topology_seen().expect("the tab").1;
    ev(&mut app, Event::Check);
    let mut all = app.topology_seen().expect("the tab").1;
    again.sort();
    all.sort();
    assert_eq!(again, all);
}
