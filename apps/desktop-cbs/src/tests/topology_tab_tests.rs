//! topology › tests's tests that need the CBS module's surfaces (its project's bottom tabs and panels; docs/adr/9199
//! §4.14): moved from the core's gis/topology/tests.rs with the module's package; the work they test is the core's.

use crate::gis;
use kentos_commands::TopologySettings;
use kentos_domain::Slot;
use crate::topology::plan::Filter;
use crate::topology::Event;
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
    let _ = app.update(Message::Gis(gis::Event::Topology(e)));
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
    assert!(app.gis.topology.running.is_some(), "on its own thread");
    assert!(app.topology_checked().is_none(), "not found yet");
    crate::files_testing::drive(&mut app, task);
    assert!(app.gis.topology.running.is_none());
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
    let task = app.update(Message::Gis(gis::Event::Topology(Event::Fix(
        "subtractFirst",
    ))));
    assert!(
        app.gis.topology.running.is_some(),
        "the recheck on its thread"
    );
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
    let task = app.update(Message::Gis(gis::Event::Topology(Event::Check)));
    assert!(app.gis.topology.running.is_some());
    ev(&mut app, Event::Stop);
    assert!(app.gis.topology.running.is_none(), "ended at once");
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Topoloji denetimi durduruldu."
    );
    // The thread's later words are not waited for: the last check stays.
    crate::files_testing::drive(&mut app, task);
    assert!(app.gis.topology.running.is_none());
    assert_eq!(app.topology_checked().expect("kept").findings, before);
}

#[test]
fn the_bar_moves_on_smoothly_and_through_a_stretch_by_the_time() {
    use crate::topology::running::Reply;
    let mut app = big();
    let task = app.update(Message::Run("topology.check"));
    let id = app.gis.topology.running.as_ref().expect("running").id;
    let mut r = app.gis.topology.running.take().expect("running");
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

/// The review (docs/adr/1260 §1): Hata işaretle writes an error mark with Gerekçe's words; the row says its state and
/// reason; a quality profile refuses a mark without a reason and writes nothing.
#[test]
fn a_finding_is_marked_as_an_error_with_its_reason() {
    let mut app = checked();
    ev(&mut app, Event::Press(0));
    ev(&mut app, Event::Reason("Sayısallaştırma hatası".into()));
    ev(&mut app, Event::Error);
    let t = topology(&app);
    assert_eq!(t.exceptions.len(), 1);
    assert_eq!(t.exceptions[0].decision, Some(kentos_commands::TopologyDecision::Error));
    assert_eq!(t.exceptions[0].reason.as_deref(), Some("Sayısallaştırma hatası"));
    assert!(t.exceptions[0].rule_digest.is_some());
    let c = app.topology_checked().expect("checked again");
    let f = &c.findings[0];
    assert!(!f.exception, "an error is still open");
    assert_eq!(crate::topology::review::state_text(c, f), "Hata: Sayısallaştırma hatası");
    // A profile asks for a reason.
    let doc = &mut app.document.as_mut().expect("open").model;
    let mut settings = doc.settings().clone();
    if let Some(t) = settings.topology.as_mut() {
        t.profile = Some(kentos_commands::TopologyProfile {
            name: "Kadastro güncelleme".into(),
            edition: None,
            precision: 0.0001,
            acceptance: 0.05,
        });
    }
    doc.set_settings(settings);
    ev(&mut app, Event::Check);
    ev(&mut app, Event::Press(1));
    ev(&mut app, Event::Reason(String::new()));
    ev(&mut app, Event::Exception(true));
    assert_eq!(topology(&app).exceptions.len(), 1, "nothing written without a reason");
}

/// Tarifle onar (docs/adr/1260 §4): the chosen findings planned first, nothing written; Uygula writes the recipe as
/// one undo step; Vazgeç drops a recipe and the drawing stays.
#[test]
fn a_recipe_is_planned_then_applied_in_one_step() {
    let mut app = checked();
    let revision = app.document.as_ref().expect("open").model.revision();
    app.gis.topology.selected = vec![0];
    ev(&mut app, Event::Recipe);
    let r = app.topology_recipe().expect("planned").clone();
    assert_eq!(r.planned.recipe.len(), 1);
    assert_eq!(app.document.as_ref().expect("open").model.revision(), revision, "a plan writes nothing");
    assert!(app.topology_recipe_text(&r).starts_with("Onarım tarifi: 1 adım"));
    ev(&mut app, Event::Drop);
    assert!(app.topology_recipe().is_none());
    ev(&mut app, Event::Recipe);
    ev(&mut app, Event::Apply);
    let doc = &mut app.document.as_mut().expect("open").model;
    assert_ne!(doc.revision(), revision);
    assert_eq!(doc.undo().as_deref(), Some("Topoloji düzelt"));
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
        c.key(&c.findings[app.gis.topology.selected[0]])
    };
    move_road(&mut app);
    assert!(app.topology_stale(), "the findings are old");
    ev(&mut app, Event::Recheck);
    assert!(!app.topology_stale());
    let c = app.topology_checked().expect("rechecked");
    assert_eq!(c.incremental, Some(1), "one object changed");
    assert_eq!(
        c.key(&c.findings[app.gis.topology.selected[0]]),
        key,
        "the chosen row follows its finding"
    );
    let mut again = app.topology_seen().expect("the tab").1;
    ev(&mut app, Event::Check);
    let mut all = app.topology_seen().expect("the tab").1;
    again.sort();
    all.sort();
    assert_eq!(again, all);
}

/// UR2-034: the reason written for one finding stays with it: another finding chosen brings its own recorded reason or
/// an empty field, so marking it an error does not write the first one's words; chosen again, the first's reason
/// comes back.
#[test]
fn a_reason_stays_with_its_finding() {
    let mut app = checked();
    ev(&mut app, Event::Press(0));
    ev(&mut app, Event::Reason("QA: örnek hattın açık başlangıcı".into()));
    ev(&mut app, Event::Exception(true));
    ev(&mut app, Event::Filter(Filter::All));
    let other = app.topology_shown().len() - 1;
    ev(&mut app, Event::Press(other));
    assert_eq!(app.gis.topology.reason, "", "another finding: its own words, none yet");
    ev(&mut app, Event::Error);
    let reasons: Vec<Option<String>> = topology(&app).exceptions.iter().map(|x| x.reason.clone()).collect();
    assert_eq!(reasons.iter().filter(|r| r.as_deref() == Some("QA: örnek hattın açık başlangıcı")).count(), 1, "{reasons:?}");
    ev(&mut app, Event::Press(0));
    assert_eq!(app.gis.topology.reason, "QA: örnek hattın açık başlangıcı", "its own reason back");
}

/// GIS-032: Gerekçe is the finding's it was typed for and the drawing's: another finding or another drawing starts
/// it empty.
#[test]
fn the_reason_typed_stays_with_its_finding_and_drawing() {
    let mut app = checked();
    ev(&mut app, Event::Press(0));
    ev(&mut app, Event::Reason("QA: ortak sınır için geçici istisna".into()));
    ev(&mut app, Event::Press(1));
    assert!(app.gis.topology.reason.is_empty(), "another finding starts it empty");
    ev(&mut app, Event::Reason("taslak".into()));
    let _ = app.update(crate::app::Message::Opened(Some(Ok(Box::new(crate::files_testing::drawing(0))))));
    assert!(app.gis.topology.reason.is_empty() && app.gis.topology.checked.is_none(), "another drawing starts it again");
}
