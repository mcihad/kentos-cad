//! Ruhsat takımı's Dijital teslim kapsamı (docs/adr/8063; plan/bim-plan.md BIM-02): the window asks the dated scope
//! decision's particulars, the standards package's Python rule decides from them and the drawing's measured area and
//! storeys, the set writes them into the drawing's variables and the audit facts carry them.

use crate::app::{App, Message};
use crate::exchange::Window;
use crate::exchange::permit_scope::{Scope, Verdict};
use crate::exchange::permit_set::Event;
use kentos_digital_project::scope::Kind;

/// The day the decisions are taken on: a fixed one, so the rows do not move with the calendar.
const TODAY: &str = "2026-11-15";

fn scope(app: &App) -> &Scope {
    match &app.exchange {
        Some(Window::PermitSet(s)) => &s.scope,
        _ => panic!("Ruhsat takımı is open"),
    }
}

/// Writes a box or picks a choice, as the window's controls do.
fn set(app: &mut App, name: &str, value: &str) {
    let s = scope(app);
    let i = s.fields.iter().position(|f| f.name == name).expect(name);
    let f = &s.fields[i];
    let e = if f.kind == Kind::Choice {
        Event::ScopeChoice(i, Some(f.choices.iter().position(|c| c == value).expect(value)))
    } else {
        Event::ScopeField(i, value.into())
    };
    let _ = app.permit_set_event(e);
}

/// The rows as (title, state words, from, until).
fn rows(app: &App) -> Vec<(String, String, Option<String>, Option<String>)> {
    match &scope(app).verdict {
        Verdict::Given(g) => g.rows.iter().map(|l| (l.title.clone(), l.words.clone(), l.from.clone(), l.until.clone())).collect(),
        other => panic!("no decision: {other:?}"),
    }
}

fn residential(app: &mut App, province: &str, municipality: &str) {
    for (n, v) in [
        ("plan_alani", "evet"),
        ("yapi_turu", "yeni bina"),
        ("kullanim", "konut"),
        ("mustakil", "hayır"),
        ("kanun_istisnasi", "yok"),
        ("il_nufusu", province),
        ("belediye_nufusu", municipality),
    ] {
        set(app, n, v);
    }
}

#[test]
fn the_window_says_which_obligation_starts_when_and_the_set_keeps_its_particulars() {
    let mut app = crate::permit_set_tests::building();
    let _ = app.update(Message::Run("arch.permitSet"));
    let area = scope(&app).area.expect("the building's area, measured");
    assert!(area > 0.0 && area < 5000.0, "{area}");
    assert!(matches!(scope(&app).verdict, Verdict::Asking), "asked on opening");
    residential(&mut app, "650.000", "400000");
    let generation = scope(&app).generation;
    app.permit_scope_settle(TODAY);
    let d = |s: &str| Some(s.to_owned());
    let got = rows(&app);
    let words: Vec<_> = got.iter().map(|r| (r.0.as_str(), r.1.as_str())).collect();
    assert_eq!(
        words,
        [
            ("Yönetmeliğin kapsamı", "Henüz başlamadı"),
            ("2B çizimler ve belgeler PDF/A", "Henüz başlamadı"),
            ("BIM modeli IFC biçiminde", "Henüz başlamadı"),
            ("Modelden üretilen çizimler PDF/A", "Henüz başlamadı"),
            ("Projelerin e-PYS'de teslimi", "Teyit gerekli"),
        ]
    );
    assert_eq!((got[1].2.clone(), got[1].3.clone()), (d("2029-03-01"), d("2032-09-01")), "PDF/A until IFC takes over");
    assert_eq!(got[2].2, d("2032-09-01"), "the remaining group: the province under 1 000 000");
    // A late answer to an older edit is dropped.
    let _ = app.permit_set_event(Event::ScopeGiven(generation - 1, Err("eski".into())));
    assert!(matches!(scope(&app).verdict, Verdict::Given(_)));
    // The set writes the particulars into the drawing's variables, and the audit facts carry them with the area.
    let _ = app.permit_set_event(Event::Make);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let var = |n: &str| {
        doc.settings().variables.iter().find(|v| v.name == n).map(|v| v.value.clone())
    };
    assert_eq!(var("kullanim"), Some(kentos_commands::VariableValue::Text("konut".into())));
    assert_eq!(var("il_nufusu"), Some(kentos_commands::VariableValue::Text("650.000".into())));
    let facts = kentos_processing::audit_facts::project_facts(doc, kentos_processing::audit_facts::MOST_OBJECTS, &|| false)
        .expect("facts");
    let permit = facts.permit.expect("the permit section");
    assert_eq!(permit.particulars.get("belediye_nufusu").map(String::as_str), Some("400000"));
    assert_eq!(permit.construction_area, Some((area * 1e4).round() / 1e4));
}

#[test]
fn a_missing_population_that_could_change_it_asks_for_confirmation() {
    let mut app = crate::permit_set_tests::building();
    let _ = app.update(Message::Run("arch.permitSet"));
    residential(&mut app, "", "");
    set(&mut app, "yapi_insaat_alani", "12000");
    app.permit_scope_settle(TODAY);
    let got = rows(&app);
    assert_eq!(got[1].1, "Teyit gerekli", "PDF/A's day turns on the populations");
    match &scope(&app).verdict {
        Verdict::Given(g) => assert!(g.rows[1].reason.contains("İl nüfusu"), "{}", g.rows[1].reason),
        _ => unreachable!(),
    }
    // An area written so that it could be 5 or 5000 is not read; the rule says what to write.
    set(&mut app, "yapi_insaat_alani", "5.000");
    app.permit_scope_settle(TODAY);
    match &scope(&app).verdict {
        Verdict::Given(g) => assert!(g.problems.iter().any(|p| p.contains("5000 ya da 5000.5")), "{:?}", g.problems),
        _ => unreachable!(),
    }
}

/// The section and its place, light and dark; `.run/gui-evidence/dijital-kapsam-8063/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib permit_scope_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dijital-kapsam-8063");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1600.0, 1000.0), "acik"), ("dark", Size::new(1440.0, 900.0), "koyu")] {
        let mut app = crate::permit_set_tests::building();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |snapshot: &mut Snapshot, app: &mut App, file: String| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(file);
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        // Its place: Mimari › Proje › Ruhsat takımı.
        let _ = app.update(Message::RibbonTab("arch"));
        shot(&mut snapshot, &mut app, format!("1-serit-{name}.png"));
        crate::tools_screens::press_caption(&mut snapshot, &mut app, "Ruhsat takımı", false, false);
        assert!(matches!(app.exchange, Some(Window::PermitSet(_))), "the button opens it");
        // A dwelling in a province under 1 000 000 people, its area from the drawing.
        residential(&mut app, "650.000", "400.000");
        set(&mut app, "nufus_kaynagi", "TÜİK ADNKS 2025");
        set(&mut app, "belediye", "Sivas Belediyesi");
        app.permit_scope_settle(TODAY);
        let middle = Point::new(size.width / 2.0, size.height / 2.0);
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(middle, -60.0));
        shot(&mut snapshot, &mut app, format!("2-kapsam-konut-{name}.png"));
        // A large province and municipality, 12 000 m²: PDF/A from the coming into force, IFC's first group.
        set(&mut app, "il_nufusu", "15.701.602");
        set(&mut app, "belediye_nufusu", "700.000");
        set(&mut app, "yapi_insaat_alani", "12000");
        set(&mut app, "epys_karar_tarihi", "01.01.2029");
        app.permit_scope_settle(TODAY);
        shot(&mut snapshot, &mut app, format!("3-kapsam-buyuk-il-{name}.png"));
        // The populations left out: the day turns on them, so the rows ask for them.
        set(&mut app, "il_nufusu", "");
        set(&mut app, "belediye_nufusu", "");
        app.permit_scope_settle(TODAY);
        shot(&mut snapshot, &mut app, format!("4-teyit-gerekli-{name}.png"));
    }
}
