//! Başvuru tarihine göre kural seti (docs/adr/8078; plan/bim-plan.md BIM-13): the same drawing audited with an
//! application on 30.6.2026 and on 2.7.2026 is read by two versions of Planlı Alanlar İmar Yönetmeliği 34/1 and 34/8
//! (RG 1.7.2026/33297, Geçici 3/11); without a date the outcome turns on is eksik veri. The Denetim tab says the rule
//! set over its table and lists the standards package's findings with their state.

use kentos_commands::{ArchAuditOutcome, ArchAuditState};
use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

fn run(app: &mut App, name: &str, input: Value) {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
}

/// Three storeys of flats over a basement holding two storage rooms (each a flat's annex) and the common boiler room;
/// the elevator's shaft (a void named Asansör) on the three storeys, not in the basement.
pub(crate) fn basement_with_annexes() -> App {
    building(&[("Depo", "e1"), ("Depo", "e2"), ("Kazan dairesi", "o")], 3)
}

/// `upper` storeys of flats (a living room and a bedroom of one flat each, the common stair hall, the elevator's shaft)
/// over a basement holding `basement`'s rooms, each named and in its program group (`e1`, `e2`: the first two flats'
/// annexes; `o`: the common area).
fn building(basement: &[(&str, &str)], upper: usize) -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    let level = |id: &str, name: &str, z: f64| json!({ "id": id, "name": name, "elevation": z, "height": 3.0 });
    let mut levels = vec![level("b01", "Bodrum kat", -3.0)];
    for k in 0..upper {
        let name = if k == 0 { "Zemin kat".to_owned() } else { format!("{k}. kat") };
        levels.push(level(&format!("k{k:02}"), &name, 3.0 * k as f64));
    }
    run(&mut app, "cad.settings.set", json!({ "arch": { "levels": levels } }));
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "nature": "Daire"},
            {"id": "d2", "number": "2", "nature": "Daire"},
            {"id": "d3", "number": "3", "nature": "Daire"},
            {"id": "d4", "number": "4", "nature": "Daire"},
            {"id": "e1", "number": "E1", "kind": "annex", "annexOf": "d1", "nature": "Depo"},
            {"id": "e2", "number": "E2", "kind": "annex", "annexOf": "d2", "nature": "Depo"},
            {"id": "o", "number": "O", "name": "Ortak alan", "kind": "common"}
        ]}}),
    );
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let room = |x0: f64, name: &str, level: &str, unit: &str, void: bool| {
        json!({"parts": [{"outer": {"pts": [p(x0, 0.0), p(x0 + 4.0, 0.0), p(x0 + 4.0, 4.0), p(x0, 4.0)]}}],
               "name": name, "unit": unit, "void": void, "arch": {"level": level}})
    };
    let mut spaces: Vec<Value> =
        basement.iter().enumerate().map(|(i, (name, unit))| room(4.0 * i as f64, name, "b01", unit, false)).collect();
    for k in 0..upper {
        let (level, unit) = (format!("k{k:02}"), format!("d{}", k + 1));
        spaces.push(room(0.0, "Salon", &level, &unit, false));
        spaces.push(room(4.0, "Yatak odası", &level, &unit, false));
        spaces.push(room(8.0, "Merdiven holü", &level, "o", false));
        spaces.push(room(12.0, "Asansör", &level, "o", true));
    }
    run(&mut app, "cad.spaces.create", json!({"layerId": layer, "spaces": spaces}));
    app
}

/// The künye's application date (`basvuru_tarihi`), as Ruhsat takımı writes it into the project's variables; none
/// takes it away.
pub(crate) fn dated(app: &mut App, date: Option<&str>) {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut settings = doc.settings().clone();
    settings.variables.retain(|v| v.name != "basvuru_tarihi");
    if let Some(d) = date {
        settings.variables.push(kentos_commands::ProjectVariable {
            name: "basvuru_tarihi".into(),
            label: "Başvuru tarihi".into(),
            kind: kentos_commands::VariableKind::Text,
            value: kentos_commands::VariableValue::Text(d.into()),
        });
    }
    doc.set_settings(settings);
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The rule set's outcomes by article: `(rule, version, outcome)`.
fn outcomes(app: &App) -> Vec<(String, Option<String>, ArchAuditOutcome)> {
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    set.rules.iter().map(|r| (r.rule.clone(), r.version.clone(), r.state)).collect()
}

/// The standards package's findings in the tab: code, state, message.
fn regulation(app: &App) -> Vec<(String, Option<ArchAuditState>, String)> {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    audited
        .findings
        .iter()
        .filter(|f| matches!(f.code.as_str(), "paiy_elevator_missing" | "paiy_elevator_not_serving" | "regulation_date_needed"))
        .map(|f| (f.code.clone(), f.state, f.message.clone()))
        .collect()
}

#[test]
fn the_application_date_chooses_the_version_of_the_elevator_articles() {
    let mut app = basement_with_annexes();
    let v = |s: &str| Some(s.to_owned());
    // Applied for on 30.6.2026: 34/1 as of 11.7.2021 counts the storeys by the plan (three: a place is enough, the
    // shaft is one) and not the basement of annexes; no elevator is asked, so 34/8 does not apply.
    dated(&mut app, Some("30.06.2026"));
    checked(&mut app);
    assert_eq!(
        outcomes(&app),
        [
            ("PAİY 34/1".to_owned(), v("11.7.2021"), ArchAuditOutcome::Passed),
            ("PAİY 34/8".to_owned(), v("11.7.2021"), ArchAuditOutcome::OutOfScope),
        ]
    );
    assert_eq!(regulation(&app), Vec::new());
    let line = app.arch.arch_audit.rule_set.as_ref().expect("the rule set").line();
    assert_eq!(
        line,
        "Kural seti — Başvuru tarihi künyeden: 30.6.2026 · PAİY 34/1 (11.7.2021) geçti · PAİY 34/8 (11.7.2021) kapsam dışı"
    );
    // Applied for on 2.7.2026: as of 1.7.2026 the basement of annexes counts, four storeys ask for an elevator and
    // 34/8 for it to reach the basement, which it does not.
    dated(&mut app, Some("02.07.2026"));
    checked(&mut app);
    assert_eq!(
        outcomes(&app),
        [
            ("PAİY 34/1".to_owned(), v("1.7.2026"), ArchAuditOutcome::Passed),
            ("PAİY 34/8".to_owned(), v("1.7.2026"), ArchAuditOutcome::Failed),
        ]
    );
    assert_eq!(
        regulation(&app),
        [(
            "paiy_elevator_not_serving".to_owned(),
            None,
            "Bodrum kat: asansör bu kata ulaşmıyor (katta eklenti ve teknik hacim) — PAİY 34/8 (1.7.2026)".to_owned()
        )]
    );
    let rule = app.arch.arch_audit.audited.as_ref().expect("checked").findings.iter().find(|f| f.code == "paiy_elevator_not_serving").and_then(|f| f.rule.clone());
    assert_eq!(rule.as_deref(), Some("PAİY 34/8 · RG 1.7.2026/33297"));
    // No date: the two versions disagree, so the finding is eksik veri, never a guess.
    dated(&mut app, None);
    checked(&mut app);
    let found = regulation(&app);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!((found[0].0.as_str(), found[0].1), ("regulation_date_needed", Some(ArchAuditState::MissingData)));
    assert!(found[0].2.contains("11.7.2021–30.6.2026 başvurusunda bulgu yok"), "{}", found[0].2);
    assert!(outcomes(&app).iter().all(|(_, _, o)| *o == ArchAuditOutcome::MissingData));
    // The shaft drawn in the basement too: 34/8 holds by either version, and no date is needed.
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [{"parts": [{"outer": {"pts": [p(12.0, 0.0), p(16.0, 0.0), p(16.0, 4.0), p(12.0, 4.0)]}}],
            "name": "Asansör", "unit": "o", "void": true, "arch": {"level": "b01"}}]}),
    );
    checked(&mut app);
    assert_eq!(regulation(&app), Vec::new());
}

/// Uzman onayı (docs/adr/8078): four storeys over a basement holding only the boiler room. 34/8 as of 1.7.2026 asks the
/// elevator to reach a basement with a common area, 34/1 does not count a technical room: the reviewer decides. Their
/// approval is İstisna yap on the package's finding with their words: hidden and counted, listed with İstisnalar:
/// Göster; once the basement holds a flat's storage too the finding is another one, open again.
#[test]
fn an_expert_s_approval_of_a_package_finding_is_kept_until_the_finding_changes() {
    use crate::arch_audit::Event;
    let event = |app: &mut App, e: Event| {
        let _ = app.update(Message::Arch(crate::arch::Event::ArchAudit(e)));
    };
    let mut app = building(&[("Kazan dairesi", "o")], 4);
    dated(&mut app, Some("02.07.2026"));
    checked(&mut app);
    let listed = |app: &App| {
        let a = app.arch.arch_audit.audited.as_ref().expect("checked");
        a.findings.iter().filter(|f| f.code == "paiy_elevator_not_serving").cloned().collect::<Vec<_>>()
    };
    let asked = listed(&app);
    assert_eq!(asked.len(), 1, "{asked:#?}");
    assert_eq!(asked[0].state, Some(ArchAuditState::ExpertReview));
    let row = crate::arch_audit_scenes::row_of(&app, "paiy_elevator_not_serving");
    event(&mut app, Event::Press(row));
    event(&mut app, Event::Except(None));
    event(&mut app, Event::Note("Kazan dairesi ortak alan sayılmaz: idarenin görüşü.".to_owned()));
    event(&mut app, Event::ExceptConfirm);
    app.arch_audit_standards_settle();
    let exceptions = app.document.as_ref().expect("a drawing").model.settings().arch.as_ref().expect("Mimari").audit_exceptions.clone();
    assert_eq!(exceptions.len(), 1, "{exceptions:#?}");
    assert_eq!(exceptions[0].code, "paiy_elevator_not_serving");
    assert!(listed(&app).is_empty(), "the approved finding is hidden");
    assert_eq!(app.arch.arch_audit.audited.as_ref().expect("checked").excepted, 1, "counted once");
    event(&mut app, Event::View(1));
    app.arch_audit_standards_settle();
    let shown = listed(&app);
    assert_eq!(shown.len(), 1);
    assert!(shown[0].exception && !shown[0].lapsed);
    assert_eq!(shown[0].note.as_deref(), Some("Kazan dairesi ortak alan sayılmaz: idarenin görüşü."));
    event(&mut app, Event::View(0));
    // A flat's storage in the basement: 34/8 asks for the elevator there plainly; that finding is open.
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [{"parts": [{"outer": {"pts": [p(20.0, 0.0), p(24.0, 0.0), p(24.0, 4.0), p(20.0, 4.0)]}}],
            "name": "Depo", "unit": "e1", "arch": {"level": "b01"}}]}),
    );
    checked(&mut app);
    let open = listed(&app);
    assert_eq!(open.len(), 1, "{open:#?}");
    assert_eq!((open[0].state, open[0].exception), (None, false));
    assert!(open[0].message.contains("katta eklenti ve teknik hacim"), "{}", open[0].message);
}

/// The Denetim tab with the rule set over its table: applied for on 30.6.2026, on 2.7.2026 and with no date, light and
/// dark; `.run/gui-evidence/mevzuat-tarih-8078/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib regulation_dates_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/mevzuat-tarih-8078");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1500.0, 1000.0), "acik"), ("dark", Size::new(1300.0, 950.0), "koyu")] {
        let mut app = basement_with_annexes();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        for (i, (date, words)) in [(Some("30.06.2026"), "30-haziran"), (Some("02.07.2026"), "2-temmuz"), (None, "tarihsiz")]
            .into_iter()
            .enumerate()
        {
            dated(&mut app, date);
            checked(&mut app);
            // The standards package's finding chosen, so the table shows its row.
            let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
            app.arch.arch_audit.selected = audited
                .findings
                .iter()
                .position(|f| matches!(f.code.as_str(), "paiy_elevator_not_serving" | "regulation_date_needed"));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{}-{words}-{name}.png", i + 1));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
        // The reviewer's approval of the basement holding only the boiler room, listed with İstisnalar: Göster.
        use crate::arch_audit::Event;
        let event = |app: &mut App, e: Event| {
            let _ = app.update(Message::Arch(crate::arch::Event::ArchAudit(e)));
        };
        let mut app = building(&[("Kazan dairesi", "o")], 4);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        dated(&mut app, Some("02.07.2026"));
        checked(&mut app);
        let row = crate::arch_audit_scenes::row_of(&app, "paiy_elevator_not_serving");
        event(&mut app, Event::Press(row));
        event(&mut app, Event::Except(None));
        event(&mut app, Event::Note("Kazan dairesi ortak alan sayılmaz: idarenin görüşü.".to_owned()));
        event(&mut app, Event::ExceptConfirm);
        app.arch_audit_standards_settle();
        event(&mut app, Event::View(1));
        app.arch_audit_standards_settle();
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_elevator_not_serving");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("4-uzman-onayi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
