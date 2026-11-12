//! Uygulama kontrolü › İzleme's Dönem dizisi, docs/adr/4840 (plan/map-cad.md MAP-19): Doğru uydurma at the confidence
//! typed and the two thresholds (m, m a year) give the shared case's tracks — each mark's line, its state and the
//! limits it goes beyond; Doğru ve eşikler shows each mark's velocities ± σ, the moving components and the state, the
//! summary the count of each state; a warning not below its alarm and a confidence that is no number are said. And its
//! pictures.

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::asbuilt_trend::Change as C;
use crate::asbuilt_window::Event as A;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.series.json")).expect("cases")
}

fn case(all: &Value, prefix: &str) -> Value {
    all["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(prefix))).expect("the case").clone()
}

/// Every number in `got` within the case's `within` of `want`'s, the rest the same (the reference moves the marks by
/// their small local numbers, the drawing holds them on large coordinates whose last bits move a σ by 1e-9).
fn same(got: &Value, want: &Value, at: &str, within: f64) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            assert!((a.as_f64().expect("a number") - b.as_f64().expect("a number")).abs() <= within, "{at}: {a} ≠ {b}")
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "{at}");
            for (k, v) in b {
                same(&a[k], v, &format!("{at}.{k}"), within);
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(x, y, &format!("{at}[{i}]"), within);
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

/// The four-epoch drawing of the shared cases, each epoch's layer given its day as its Veri kümesi bilgisi.
fn opened() -> App {
    let all = cases();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(case(&all, "doğru ve eşikler birlikte")["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    for (layer, day) in [("donem1", "2026-01-15"), ("donem2", "2026-04-15"), ("donem3", "2026-07-15"), ("donem4", "2026-10-15")] {
        crate::program_scenes::command(&mut app, "cad.datasets.edit", json!({"layer": layer, "dataset": {"updated": day}}));
    }
    app
}

/// The combined case's ask typed into the window: three earlier epochs, the line at 95 %, both levels.
fn typed(app: &mut App) {
    let mut events = vec![
        A::Kind(2),
        A::Measured("donem4".into()),
        A::Design("donem3".into()),
        A::Horizontal("0,008".into()),
        A::Vertical("0,01".into()),
        A::Control("K1, K2".into()),
        A::Epochs("Birinci dönem, İkinci dönem, Üçüncü dönem".into()),
        A::Trend(C::On(true)),
        A::Trend(C::Confidence("95".into())),
    ];
    for (k, (warning, alarm)) in [("0,0075", "0,03"), ("0,012", "0,04"), ("0,015", "0,06"), ("0,05", "0,08")].into_iter().enumerate() {
        events.push(A::Trend(C::Warning(k, warning.into())));
        events.push(A::Trend(C::Alarm(k, alarm.into())));
    }
    for e in events {
        let _ = app.update(Message::Asbuilt(e));
    }
}

#[test]
fn the_line_and_the_levels_are_the_shared_case_s() {
    let mut app = opened();
    let _ = app.update(Message::Run("asbuilt.check"));
    typed(&mut app);
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    let all = cases();
    let step = &case(&all, "doğru ve eşikler birlikte")["steps"][0];
    let within = step["within"].as_f64().expect("the case's tolerance");
    same(&serde_json::to_value(&r.tracks).expect("json"), &step["result"]["output"]["tracks"], "tracks", within);
    assert_eq!(
        crate::asbuilt_trend::line(&r.tracks).as_deref(),
        Some("Eşikler: 2 nokta alarmda, 2 uyarıda, 5 olağan. Eşikler sizindir; izleme için yönetmelik değeri yoktur.")
    );
    // M2's row: four epochs, its velocities ± σ as the case's, both horizontal components moving, at the alarm.
    let m2 = r.tracks.iter().find(|t| t.name == "M2").expect("M2");
    let fit = m2.fit.as_ref().expect("a line");
    let cells = crate::asbuilt_trend::cells(m2);
    assert_eq!(cells[1], "4");
    assert_eq!(cells[2], format!("{:+.4} ±{:.4}", fit.east, fit.sigma_east).replace('.', ","));
    // A velocity under half the last digit is shown unsigned (M3 does not move sideways, M4 barely east).
    let m3 = r.tracks.iter().find(|t| t.name == "M3").expect("M3");
    assert!(crate::asbuilt_trend::cells(m3)[2].starts_with("0,0000 ±"), "{:?}", crate::asbuilt_trend::cells(m3));
    assert_eq!((cells[6].as_str(), cells[7].as_str()), ("doğu, kuzey", "Alarm (yatay, yatay hız)"));
    let m4 = r.tracks.iter().find(|t| t.name == "M4").expect("M4");
    assert_eq!(crate::asbuilt_trend::cells(m4)[6..], ["yok".to_owned(), "Olağan".to_owned()]);
    // The movements' table is a choice away, and choosing it checks nothing again.
    assert_eq!(app.asbuilt.trend.view, crate::asbuilt_trend::View::Line);
    let _ = app.update(Message::Asbuilt(A::Trend(C::View(crate::asbuilt_trend::View::Series))));
    assert_eq!(app.asbuilt.trend.view, crate::asbuilt_trend::View::Series);
    let _ = app.update(Message::Asbuilt(A::Trend(C::View(crate::asbuilt_trend::View::Line))));
    // A warning not below its alarm is the command's refusal, said in the window.
    let _ = app.update(Message::Asbuilt(A::Trend(C::Warning(2, "0,06".into()))));
    let said = app.asbuilt.answer.clone().expect("an answer").expect_err("refused");
    assert!(said.contains("Yatay hızın uyarı eşiği alarm eşiğinden küçük olmalı."), "{said}");
    // A confidence that is no number is the window's.
    let _ = app.update(Message::Asbuilt(A::Trend(C::Warning(2, "0,015".into()))));
    let _ = app.update(Message::Asbuilt(A::Trend(C::Confidence("yüzde".into()))));
    let said = app.asbuilt.answer.clone().expect("an answer").expect_err("refused");
    assert!(said.contains("Güven düzeyi “yüzde” bir sayı değil."), "{said}");
    // The line off and the boxes empty: the series as before, no line, no state.
    let _ = app.update(Message::Asbuilt(A::Trend(C::On(false))));
    for k in 0..4 {
        let _ = app.update(Message::Asbuilt(A::Trend(C::Warning(k, String::new()))));
        let _ = app.update(Message::Asbuilt(A::Trend(C::Alarm(k, String::new()))));
    }
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    assert!(r.tracks.iter().all(|t| t.fit.is_none() && t.state.is_none() && t.exceeded.is_empty()));
    assert!(crate::asbuilt_trend::line(&r.tracks).is_none());
}

/// `.run/gui-evidence/dogru-esik-4840/`: the monitoring form's Dönem dizisi section (form), then the window scrolled
/// down to the summary and the series table's new columns (tablo).
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_trend_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dogru-esik-4840");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for step in ["form", "tablo"] {
            let mut app = opened();
            let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("asbuilt.check"));
            typed(&mut app);
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            if step == "tablo" {
                let at = Point::new(size.width * 0.5, size.height * 0.45);
                snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -40.0));
            }
            let file = out.join(format!("dogru-esik-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
