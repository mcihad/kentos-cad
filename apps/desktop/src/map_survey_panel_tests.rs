//! GNSS arşivi (map_survey_panel.rs, plan/map-cad.md MAP-03, docs/adr/1340): one mark measured on two days and
//! accepted as their mean; its second day's antenna height corrected, the point is out of date and the surface resting
//! on it too; made again from the panel its old result stays, its object moves, one undo takes it back; a solution
//! excluded with a reason and taken back; a recorded NMEA log added, solved and accepted; a closed file dialog changes
//! nothing; saved and opened again (its file gone) the raw measurements are all there and reading writes nothing;
//! KentOS Harita docks the panel, KentOS GIS reads the accepted point in Öznitelikler, Mimari and Yol show neither.
//! And its pictures.

use kentos_commands::{MapSurveyOperation as Op, SurveyAntenna, SurveyState as S};
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message, Panel};
use crate::map_survey_panel::Event as P;

fn fixtures() -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .canonicalize()
        .expect("the fixtures")
        .to_string_lossy()
        .into_owned()
}

/// `map.survey.query`'s drawing (map_survey_cases.py), its paths made absolute.
fn snapshot() -> kentos_commands::DocumentSnapshotV2 {
    let text = include_str!("../../../fixtures/commands/v1/map.survey.query.json")
        .replace("$fixture:", &format!("{}/", fixtures()));
    let cases: Value = serde_json::from_str(&text).expect("cases");
    serde_json::from_value(cases["setup"].clone()).expect("the drawing reads")
}

fn opened(snapshot: kentos_commands::DocumentSnapshotV2, program: Option<Module>) -> App {
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::MapSurvey(e));
}

fn tree(app: &App) -> kentos_commands::MapSurveyTree {
    app.map_survey_read().expect("a drawing").expect("a map project")
}

/// Öznitelikler's GNSS ölçümü rows of the object in `slot`, selected alone.
fn survey_rows(app: &mut App, slot: u32) -> Vec<(String, String)> {
    app.selection.set(vec![Slot(slot)]);
    let doc = app.document.as_ref().expect("a drawing");
    let panel = app.properties_panel(doc);
    panel
        .sections
        .iter()
        .find(|s| s.id == "mapSurvey")
        .map(|s| s.rows.iter().map(|r| (r.label.to_string(), r.value.clone())).collect())
        .unwrap_or_default()
}

fn row<'a>(rows: &'a [(String, String)], label: &str) -> Option<&'a str> {
    rows.iter().find(|(l, _)| l == label).map(|(_, v)| v.as_str())
}

fn z_of(app: &App, slot: u32) -> Option<f64> {
    match app.document.as_ref()?.model.get(Slot(slot))? {
        kentos_commands::Entity::Point(p) => p.z,
        _ => None,
    }
}

/// The second day's antenna, its height corrected from the field book.
fn corrected(app: &App) -> SurveyAntenna {
    let doc = app.document.as_ref().expect("a drawing");
    let mut a = doc.model.map_survey().expect("an archive").session("os-2").and_then(|s| s.antenna.clone()).expect("an antenna");
    a.height = 1.81;
    a
}

#[test]
fn a_mark_measured_on_two_days_is_accepted_and_made_again() {
    let mut app = opened(snapshot(), Some(Module::Map));
    assert!(app.shows_panel(Panel::MapSurvey));
    let t = tree(&app);
    assert_eq!((t.campaigns.len(), t.sessions.len(), t.marks.len()), (1, 3, 2));
    let n1 = &t.marks[0];
    assert_eq!((n1.mark.as_str(), n1.sessions.len(), n1.point.as_deref()), ("N1", 2, Some("kn-1")));
    let roles: Vec<_> = n1.solutions.iter().map(|s| (s.id.as_str(), s.latest, s.accepted, s.excluded.is_some())).collect();
    assert_eq!(roles, [("cz-1", false, true, false), ("cz-2", true, true, false), ("cz-4", false, false, true)]);
    assert_eq!((t.points[0].state, t.uses[0].state, t.stale), (S::Current, S::Current, 0));
    let rows = survey_rows(&mut app, 1);
    assert_eq!(row(&rows, "İşaret"), Some("N1"));
    assert_eq!(row(&rows, "Kabul edilen nokta"), Some("kn-1 · 1. revizyon"));
    assert_eq!(row(&rows, "Durum"), Some("Güncel"));
    assert_eq!(row(&rows, "Yöntem"), Some("Ortalama"));
    assert_eq!(row(&rows, "Koordinat zinciri"), Some("Tam"));
    assert_eq!(row(&survey_rows(&mut app, 2), "Dayandığı kayıt"), Some("kb-1 · Yüzey üret"));
    // The field book says 1.810 m on the second day: the accepted point and the surface on it are out of date.
    let z0 = z_of(&app, 1).expect("a height");
    let antenna = corrected(&app);
    let op = Op::SetSession {
        session: "os-2".into(),
        mark: None,
        start: None,
        end: None,
        time_system: None,
        receiver: None,
        antenna: Some(antenna),
        operator: None,
    };
    assert!(app.map_survey_edit(op).is_some());
    let t = tree(&app);
    assert_eq!(t.points[0].state, S::Stale);
    assert_eq!(t.points[0].why, ["“os-2” oturumunun anten yüksekliği 1.610 m iken 1.810 m oldu."]);
    assert_eq!(row(&survey_rows(&mut app, 1), "Durum"), Some("Güncel değil"));
    // Made again from the panel: the second revision, the old result kept, the object 0.1 m lower (half of 0.2 m),
    // the surface out of date.
    send(&mut app, P::Regenerate("kn-1".into()));
    let t = tree(&app);
    assert_eq!((t.points[0].state, t.points[0].result.revision, t.points[0].history), (S::Current, 2, 1));
    assert_eq!(t.uses[0].state, S::Stale);
    let z1 = z_of(&app, 1).expect("a height");
    assert!((z0 - z1 - 0.1).abs() < 1e-9, "{z0} → {z1}");
    assert_eq!(row(&survey_rows(&mut app, 1), "Eski sonuç"), Some("1 (raporları korunur)"));
    let _ = app.update(Message::Run("edit.undo"));
    let t = tree(&app);
    assert_eq!((t.points[0].result.revision, t.points[0].state), (1, S::Stale), "undone: the first revision");
    assert_eq!(z_of(&app, 1), Some(z0));
}

#[test]
fn a_solution_is_excluded_with_a_reason_and_a_recorded_log_is_accepted() {
    let mut app = opened(snapshot(), Some(Module::Map));
    // No reason: nothing is excluded.
    let before = app.document.as_ref().expect("a drawing").model.revision();
    send(&mut app, P::Exclude("cz-2".into()));
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision(), before);
    send(&mut app, P::Text("Ağaç altında; float'a düştü".into()));
    send(&mut app, P::Exclude("cz-2".into()));
    let t = tree(&app);
    let cz2 = t.marks[0].solutions.iter().find(|s| s.id == "cz-2").expect("kept in the record");
    assert_eq!(cz2.excluded.as_deref(), Some("Ağaç altında; float'a düştü"));
    assert_eq!(t.points[0].state, S::Stale);
    send(&mut app, P::Include("cz-2".into()));
    assert_eq!(tree(&app).points[0].state, S::Current);
    // A closed dialog changes nothing.
    let before = app.document.as_ref().expect("a drawing").model.revision();
    send(&mut app, P::Imported(None));
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision(), before);
    // A recorded log of mark N5: six raw measurements, its replay, solved from its RTK fixed ones and accepted.
    send(&mut app, P::Text("N5".into()));
    send(&mut app, P::Imported(Some(format!("{}/map-survey/v1/n2.nmea", fixtures()).into())));
    let t = tree(&app);
    let s = t.sessions.iter().find(|s| s.mark == "N5").expect("added");
    assert_eq!((s.id.as_str(), s.observations), ("os-4", 6));
    let replay = s.replay.as_ref().expect("a replay");
    assert_eq!((replay.float_drops, replay.time_jumps, replay.nominal), (1, 1, Some(1.0)));
    send(&mut app, P::Solve("os-4".into()));
    // No antenna yet: the mark's height is not known, nothing is accepted; the pole's 2.000 m typed, it is.
    send(&mut app, P::Accept("N5".into()));
    assert!(tree(&app).points.iter().all(|p| p.mark != "N5"));
    send(&mut app, P::Text("2.000".into()));
    send(&mut app, P::Antenna("os-4".into()));
    send(&mut app, P::Accept("N5".into()));
    let t = tree(&app);
    let p = t.points.iter().find(|p| p.mark == "N5").expect("accepted");
    assert_eq!((p.id.as_str(), p.result.solutions.as_slice()), ("kn-2", ["cz-5".to_owned()].as_slice()));
    assert_eq!(p.result.chain, "limited", "a WGS 84 receiver solution is not a precise transformation");
    send(&mut app, P::Text(String::new()));
    let _ = app.update(Message::Run("map.survey.select"));
    assert_eq!(app.selection.ids().len(), 2, "both accepted points");
}

#[test]
fn saved_and_opened_again_the_raw_measurements_stay_even_without_their_file() {
    let mut app = opened(snapshot(), Some(Module::Map));
    let model = &app.document.as_ref().expect("a drawing").model;
    let bytes = kentos_kcad::encode(&model.to_snapshot_v2()).expect("encodes");
    let mut back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(back.settings.map_survey, model.map_survey().cloned(), "the archive is saved whole");
    let before = tree(&app);
    // The log moved away: its file is missing, its raw measurements are in the project.
    if let Some(s) = back.settings.map_survey.as_mut().and_then(|s| s.sessions.get_mut(2)) {
        s.file.as_mut().expect("a file").path = "/yok/n2.nmea".into();
    }
    app = opened(back, Some(Module::Map));
    let after = tree(&app);
    assert_eq!(after.marks, before.marks);
    assert_eq!(after.points, before.points);
    let s = &after.sessions[2];
    assert_eq!((s.observations, s.file.as_ref().map(|f| f.state)), (6, Some(kentos_commands::SurveyFileState::Missing)));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let (revision, dirty) = (doc.revision(), doc.is_dirty());
    send(&mut app, P::Deep(true));
    let _ = tree(&app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!((doc.revision(), doc.is_dirty()), (revision, dirty), "reading writes nothing");
}

#[test]
fn harita_edits_the_archive_gis_reads_the_point_mimari_and_yol_neither() {
    let mut map = opened(snapshot(), Some(Module::Map));
    assert!(map.offers("map.survey") && map.offers("map.survey.import") && map.offers("map.survey.select"));
    assert!(!survey_rows(&mut map, 1).is_empty());
    let mut gis = opened(snapshot(), Some(Module::Gis));
    assert!(!gis.shows_panel(Panel::MapSurvey));
    assert!(!gis.offers("map.survey") && !gis.offers("map.survey.import"));
    assert!(gis.offers("map.survey.select"));
    assert_eq!(row(&survey_rows(&mut gis, 1), "Kabul edilen nokta"), Some("kn-1 · 1. revizyon"));
    for module in [Module::Architecture, Module::Road] {
        let mut app = opened(snapshot(), Some(module));
        assert!(!app.shows_panel(Panel::MapSurvey), "{module:?}");
        assert!(!app.offers("map.survey") && !app.offers("map.survey.select"), "{module:?}");
        assert!(survey_rows(&mut app, 1).is_empty(), "{module:?}");
    }
}

/// The panel in the dock, wide and light, narrow and dark: the archive as it is, the second day's antenna corrected
/// (N1 out of date), and GIS's Öznitelikler of the accepted point; `.run/gui-evidence/gnss-arsivi-*`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_survey_panel_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        for (program, what) in [(Module::Map, "harita"), (Module::Gis, "gis")] {
            let mut app = opened(snapshot(), Some(program));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            let mut shot = |app: &mut App, file: &str| {
                snapshot.settle(app, App::view, &mut update);
                let file = out.join(format!("gnss-arsivi-{file}-{name}.png"));
                snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
                println!("{}", file.display());
            };
            if program == Module::Gis {
                app.selection.set(vec![Slot(1)]);
                shot(&mut app, "gis-oznitelik");
                continue;
            }
            let _ = what;
            app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
            for key in ["marks", "mark:N1", "mark:N2", "uses"] {
                send(&mut app, P::Fold(key.into()));
            }
            app.selection.set(vec![Slot(1)]);
            shot(&mut app, "guncel");
            let antenna = corrected(&app);
            let op = Op::SetSession {
                session: "os-2".into(),
                mark: None,
                start: None,
                end: None,
                time_system: None,
                receiver: None,
                antenna: Some(antenna),
                operator: None,
            };
            let _ = app.map_survey_edit(op);
            // N1 now wants attention and starts open (the user's earlier flips taken back); the sessions folded away.
            for key in ["marks", "mark:N1", "campaigns"] {
                send(&mut app, P::Fold(key.into()));
            }
            app.selection.set(vec![Slot(1)]);
            shot(&mut app, "anten-degisti");
        }
    }
}
