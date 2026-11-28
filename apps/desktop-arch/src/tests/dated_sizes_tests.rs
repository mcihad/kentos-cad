//! Kat yüksekliği ve kapılar başvuru gününün hâliyle (docs/adr/8083; Planlı Alanlar İmar Yönetmeliği 28/1-c, 39/1) on a
//! two-storey house worked by hand: Zemin kat ±0.00 and 1. kat +3.80, each 3.80 m floor to floor; one flat (Daire) on a
//! 10 × 5 m plan inside 25 cm walls; on Zemin kat a Hol and a WC parted by a 10 cm wall, the WC's door of 1.00 m rough
//! (0.86 m clear), the entrance of 1.20 m rough (1.06 m clear); a bedroom on 1. kat.
//!
//! - **28/1-c:** 1.10.2017's text asks at most 3.60 m in a dwelling area; 12.8.2023's adds 4.00 m in a building of no
//!   more than three storeys without its basements: two storeys of 3.80 m miss the first and keep the second.
//! - **39/1:** 1.10.2017's text lets a toilet's door be 0.80 m; 13.8.2025's asks 0.90 m of every room's door but the
//!   entrances: the WC's 0.86 m keeps the first and misses the second. The entrance (one flat, no common area: the
//!   unit's, 1.00 m) keeps both.
//! - **Sivas 28/1-c and 39/1 (10.11.2022):** 3.60 m without the 4.00 m, a toilet's door 0.80 m.

use crate::arch::ArchState;
use crate::arch_ext::*;
use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S};
use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

fn run(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// The house; its WC's door and its entrance by their persistent ids.
pub(crate) fn house() -> (App, String, String) {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(
        Some(Module::Architecture),
        None,
        Settings::memory(),
        Recovery::off(),
    );
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            {"id": "k00", "name": "Zemin kat", "elevation": 0.0, "height": 3.8},
            {"id": "k01", "name": "1. kat", "elevation": 3.8, "height": 3.8}
        ] } }),
    );
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [{"id": "d1", "number": "1", "nature": "Daire"}]}}),
    );
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let mut ground: Vec<Value> = Vec::new();
    for level in ["k00", "k01"] {
        let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}});
        let mut walls = vec![
            wall((0.125, 0.125), (9.875, 0.125), 0.25),
            wall((9.875, 0.125), (9.875, 4.875), 0.25),
            wall((9.875, 4.875), (0.125, 4.875), 0.25),
            wall((0.125, 4.875), (0.125, 0.125), 0.25),
        ];
        if level == "k00" {
            walls.push(wall((8.0, 0.25), (8.0, 4.75), 0.1));
        }
        let made = run(
            &mut app,
            "cad.walls.create",
            json!({"layerId": layer, "walls": walls}),
        );
        if level == "k00" {
            ground = made["output"]["uids"]
                .as_array()
                .cloned()
                .unwrap_or_default();
        }
    }
    let host = |i: usize| ground[i].as_str().expect("a wall").to_owned();
    let doors = run(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [
            {"host": host(4), "at": 2.0, "form": "door", "width": 1.0, "head": 2.1},
            {"host": host(0), "at": 3.0, "form": "door", "width": 1.2, "head": 2.1}
        ]}),
    );
    let uid = |i: usize| {
        doors["output"]["uids"][i]
            .as_str()
            .expect("a door")
            .to_owned()
    };
    let (wc, entrance) = (uid(0), uid(1));
    let room = |x: f64, name: &str, level: &str| json!({"seed": p(x, 2.5), "name": name, "unit": "d1", "arch": {"level": level}});
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [room(4.0, "Hol", "k00"), room(9.0, "WC", "k00"), room(5.0, "Yatak odası", "k01")]}),
    );
    (app, wc, entrance)
}

/// The facts the rules read: each storey 3.80 m and the bedroom upstairs; the WC's door between two rooms of the flat,
/// a toilet's, 0.86 m clear and 2.10 m high; the entrance from nothing into the Hol, 1.06 m clear; one unit, no common
/// area.
#[test]
fn the_facts_carry_the_storeys_heights_and_what_each_door_enters() {
    let (app, wc, entrance) = house();
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let storeys: Vec<(&str, Option<f64>, u32)> = facts
        .storeys
        .iter()
        .map(|s| (s.level.as_str(), s.height, s.bedrooms))
        .collect();
    assert_eq!(storeys, [("k00", Some(3.8), 0), ("k01", Some(3.8), 1)]);
    assert_eq!((facts.units, facts.dwellings, facts.commons), (1, 1, 0));
    let door = |uid: &str| {
        facts
            .doors
            .iter()
            .find(|d| d.uid == uid)
            .expect("the door")
            .clone()
    };
    let w = door(&wc);
    assert_eq!(
        (w.clear, w.height, w.toilet, w.balcony),
        (Some(0.86), Some(2.1), true, false),
        "{w:?}"
    );
    assert_eq!(
        w.sides,
        [Some("unit".to_owned()), Some("unit".to_owned())],
        "{w:?}"
    );
    let e = door(&entrance);
    assert_eq!(e.clear, Some(1.06), "{e:?}");
    let mut sides = e.sides.clone();
    sides.sort();
    assert_eq!(sides, [None, Some("unit".to_owned())], "{e:?}");
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(
        app.modules.arch().arch_audit.regulation_failed,
        None,
        "the national regulation's rule answers"
    );
}

/// The two articles' findings in the tab: code, state, message; and their rows: rule, version, outcome.
type Rows = Vec<(String, Option<String>, O)>;
fn said(app: &App) -> (Vec<(String, Option<S>, String)>, Rows) {
    let audited = app
        .modules
        .arch()
        .arch_audit
        .audited
        .as_ref()
        .expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| {
            f.code.starts_with("paiy_storey_")
                || f.code.starts_with("paiy_door_")
                || f.rule
                    .as_deref()
                    .is_some_and(|r| r.contains("28/1-c") || r.contains("39/1"))
        })
        .map(|f| (f.code.clone(), f.state, f.message.clone()))
        .collect();
    let set = app
        .modules
        .arch()
        .arch_audit
        .rule_set
        .as_ref()
        .expect("the rule set");
    let rows = set
        .rules
        .iter()
        .filter(|r| r.rule.contains("28/1-c") || r.rule.contains("39/1"))
        .map(|r| (r.rule.clone(), r.version.clone(), r.state))
        .collect();
    (found, rows)
}

fn row(rule: &str, version: Option<&str>, outcome: O) -> (String, Option<String>, O) {
    (rule.to_owned(), version.map(str::to_owned), outcome)
}

/// One drawing, four application dates: 1.8.2023 its storeys are 1.10.2017's 3.60 m's; 14.8.2023 they keep
/// 12.8.2023's 4.00 m and the WC's door 1.10.2017's 0.80 m; 14.8.2025 the door misses 13.8.2025's 0.90 m; no date both
/// turn on it (eksik veri, each storey and the door named). With Sivas on 14.8.2025 its 3.60 m binds and the door is a
/// conflict.
#[test]
fn the_application_date_chooses_the_storey_height_and_the_door_version() {
    let (mut app, _, _) = house();
    let v = Some;
    crate::regulation_dates_tests::dated(&mut app, Some("01.08.2023"));
    checked(&mut app);
    let (found, rows) = said(&app);
    assert_eq!(
        found,
        [
            (
                "paiy_storey_height_over".to_owned(),
                None,
                "“Zemin kat” kat yüksekliği 3.80 m > 3.60 m (konut bölgesi, 2 kat) — PAİY 28/1-c (1.10.2017)".to_owned()
            ),
            (
                "paiy_storey_height_over".to_owned(),
                None,
                "“1. kat” kat yüksekliği 3.80 m > 3.60 m (konut bölgesi, 2 kat) — PAİY 28/1-c (1.10.2017)".to_owned()
            ),
        ]
    );
    assert_eq!(
        rows,
        [
            row("PAİY 28/1-c", v("1.10.2017"), O::Failed),
            row("PAİY 39/1", v("1.10.2017"), O::Passed)
        ]
    );
    crate::regulation_dates_tests::dated(&mut app, Some("14.08.2023"));
    checked(&mut app);
    let (found, rows) = said(&app);
    assert_eq!(found, Vec::new());
    assert_eq!(
        rows,
        [
            row("PAİY 28/1-c", v("12.8.2023"), O::Passed),
            row("PAİY 39/1", v("1.10.2017"), O::Passed)
        ]
    );
    crate::regulation_dates_tests::dated(&mut app, Some("14.08.2025"));
    checked(&mut app);
    let (found, rows) = said(&app);
    assert_eq!(
        found,
        [(
            "paiy_door_width_below".to_owned(),
            None,
            "Tuvalet kapısının net genişliği 0.86 m < 0.90 m — PAİY 39/1-c (13.8.2025)".to_owned()
        )]
    );
    assert_eq!(
        rows,
        [
            row("PAİY 28/1-c", v("12.8.2023"), O::Passed),
            row("PAİY 39/1", v("13.8.2025"), O::Failed)
        ]
    );
    // No date: each storey and the door turn on it.
    crate::regulation_dates_tests::dated(&mut app, None);
    checked(&mut app);
    let (found, rows) = said(&app);
    assert_eq!(found.len(), 3, "{found:#?}");
    assert!(
        found
            .iter()
            .all(|(c, s, _)| c == "regulation_date_needed" && *s == Some(S::MissingData)),
        "{found:#?}"
    );
    assert!(
        found[0]
            .2
            .contains("12.8.2023 ve sonrası başvurusunda bulgu yok"),
        "{found:#?}"
    );
    assert!(found[2].2.contains(
        "13.8.2025 ve sonrası başvurusunda Tuvalet kapısının net genişliği 0.86 m < 0.90 m"
    ));
    assert_eq!(
        rows,
        [
            row("PAİY 28/1-c", None, O::MissingData),
            row("PAİY 39/1", None, O::MissingData)
        ]
    );
    // Sivas over PAİY on 14.8.2025.
    crate::regulation_dates_tests::dated(&mut app, Some("14.08.2025"));
    let doc = app.document.as_ref().expect("a drawing");
    let mut arch = doc.model.settings().arch.clone().unwrap_or_default();
    arch.local_rules = Some(kentos_commands::LocalRules::Sivas);
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": serde_json::to_value(&arch).expect("serialises") }),
    );
    checked(&mut app);
    let (found, rows) = said(&app);
    let messages: Vec<&str> = found.iter().map(|(_, _, m)| m.as_str()).collect();
    assert_eq!(
        messages,
        [
            "“Zemin kat” kat yüksekliği 3.80 m > 3.60 m (konut bölgesi, 2 kat) — Sivas 28/1-c; PAİY 28/1-c (12.8.2023): \
             4.00 m",
            "“1. kat” kat yüksekliği 3.80 m > 3.60 m (konut bölgesi, 2 kat) — Sivas 28/1-c; PAİY 28/1-c (12.8.2023): \
             4.00 m",
            "Tuvalet kapısının net genişliği 0.86 m: Sivas 39/1-c en az 0.80 m ister, uyuyor; PAİY 39/1-c (13.8.2025) en \
             az 0.90 m ister, uymuyor — çelişkide idare karar verir",
        ]
    );
    assert_eq!(found[2].0, "regulation_conflict");
    assert_eq!(
        rows,
        [
            row("PAİY 28/1-c", v("12.8.2023"), O::Passed),
            row("Sivas 28/1-c", v("10.11.2022"), O::Failed),
            row("PAİY 39/1", v("13.8.2025"), O::Failed),
            row("Sivas 39/1", v("10.11.2022"), O::Passed),
        ]
    );
}

/// The house's ground storey and Denetim on 1.8.2023 (the storeys' finding) and on 14.8.2025 (the WC's door), light
/// and dark; `.run/gui-evidence/kat-kapi-8083/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib dated_sizes_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/kat-kapi-8083");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (date, code, picture) in [
        ("01.08.2023", "paiy_storey_height_over", "1-kat-2023"),
        ("14.08.2025", "paiy_door_width_below", "2-kapi-2025"),
    ] {
        for (theme, size, name) in [
            ("light", Size::new(1440.0, 900.0), "acik"),
            ("dark", Size::new(1280.0, 800.0), "koyu"),
        ] {
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            let (mut app, _, _) = house();
            crate::regulation_dates_tests::dated(&mut app, Some(date));
            let _ = app.update(Message::ActiveLevel(Some("k00".into())));
            let _ = app.update(Message::LevelView(
                kentos_interaction::level_view::LevelView::Active,
            ));
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            checked(&mut app);
            let audited = app
                .modules
                .arch()
                .arch_audit
                .audited
                .as_ref()
                .expect("checked");
            app.modules.arch_mut().arch_audit.selected =
                audited.findings.iter().position(|f| f.code == code);
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut update);
            app.selection.set(Vec::new());
            let _ = app.update(Message::Run("view.zoomExtents"));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{picture}-{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
