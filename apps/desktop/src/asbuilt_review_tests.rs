//! Uygulama kontrolü › İzleme's İnceleme and Grafik, docs/adr/5220 (plan/map-cad.md MAP-19): the shared case's job typed
//! into the window and kept with the reviewer's statement gives the case's review (its job, digest, marks' states and
//! the reviewer); a threshold without its source is said; the review is listed as current, then as belonging to the
//! earlier result once a mark is moved, and removed. Grafik draws the chosen mark's measurements against the epochs'
//! days with the thresholds typed. And their pictures.

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::asbuilt_review_form::Change as R;
use crate::asbuilt_trend::Change as C;
use crate::asbuilt_window::Event as A;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.asbuilt.review.json")).expect("cases")
}

fn case(all: &Value, prefix: &str) -> Value {
    all["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(prefix))).expect("the case").clone()
}

/// The review cases' drawing, each epoch's layer given its day as its Veri kümesi bilgisi.
fn opened() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    for (layer, day) in [("donem1", "2026-01-15"), ("donem2", "2026-04-15"), ("donem3", "2026-07-15")] {
        crate::program_scenes::command(&mut app, "cad.datasets.edit", json!({"layer": layer, "dataset": {"updated": day}}));
    }
    app
}

/// The case's job and statement typed into the window: two earlier epochs, the movement limits, both levels with
/// their source, the control engineer's “Kontrol edilmiştir”.
fn typed(app: &mut App, source: &str) {
    let _ = app.update(Message::Run("asbuilt.check"));
    let mut events = vec![
        A::Kind(2),
        A::Measured("donem3".into()),
        A::Design("donem2".into()),
        A::Horizontal("0,008".into()),
        A::Vertical("0,01".into()),
        A::Control("K1, K2".into()),
        A::Epochs("Birinci dönem, İkinci dönem".into()),
        A::Trend(C::Warning(0, "0,0075".into())),
        A::Trend(C::Warning(1, "0,012".into())),
        A::Trend(C::Alarm(0, "0,03".into())),
        A::Trend(C::Alarm(1, "0,04".into())),
        A::Trend(C::Source(source.into())),
        A::Review(R::Name("Ekim incelemesi".into())),
        A::Review(R::Role(1)),
        A::Review(R::Title("Harita Mühendisi".into())),
        A::Review(R::Person("A. Yılmaz".into())),
        A::Review(R::Date("2026-10-20".into())),
    ];
    for e in events.drain(..) {
        let _ = app.update(Message::Asbuilt(e));
    }
}

#[test]
fn the_review_kept_from_the_window_is_the_case_s() {
    let all = cases();
    let want = case(&all, "üç dönemli izleme")["steps"][0]["result"]["output"]["kept"].clone();
    let mut app = opened();
    // A threshold without its source is said; nothing kept.
    typed(&mut app, "");
    let _ = app.update(Message::Asbuilt(A::Keep));
    assert!(app.document.as_ref().expect("a drawing").model.monitoring_reviews().is_empty());
    let source = want["job"]["thresholds"]["source"].as_str().expect("a source");
    let _ = app.update(Message::Asbuilt(A::Trend(C::Source(source.into()))));
    let _ = app.update(Message::Asbuilt(A::Keep));
    let kept = app.document.as_ref().expect("a drawing").model.monitoring_reviews().to_vec();
    assert_eq!(kept.len(), 1);
    let got = serde_json::to_value(&kept[0]).expect("json");
    // The layers are the drawing's ids, as the case's are; the job, digest and reviewer the case's.
    assert_eq!(got["digest"], want["digest"]);
    assert_eq!(got["job"], want["job"]);
    assert_eq!(got["reviewer"], want["reviewer"]);
    let states = |v: &Value| v["marks"].as_array().expect("marks").iter().map(|m| (m["name"].clone(), m["state"].clone())).collect::<Vec<_>>();
    assert_eq!(states(&got), states(&want));
    assert_eq!(app.asbuilt.reviews.len(), 1);
    assert!(app.asbuilt.reviews[0].current);
    // M1 of this epoch moved 4 mm: the review belongs to the earlier result.
    let m1 = {
        let doc = app.document.as_ref().expect("a drawing");
        let layer = "donem3";
        doc.model
            .entities()
            .find(|e| e.base().layer_id == layer && e.base().attrs.get("Ad").map(String::as_str) == Some("M1"))
            .map(|e| doc.model.uid(kentos_domain::Slot(e.base().id)).expect("a uid").to_string())
            .expect("M1")
    };
    crate::program_scenes::command(&mut app, "cad.entities.transform", json!({"uids": [m1], "transform": {"kind": "move", "dx": 0.004, "dy": 0.0}}));
    let _ = app.update(Message::Asbuilt(A::Horizontal("0,008".into())));
    assert!(!app.asbuilt.reviews[0].current, "{:?}", app.asbuilt.reviews);
    assert!(crate::asbuilt_review_form::lead(&app.asbuilt.reviews).contains("eski sonuca aittir"));
    // Removed by its name.
    let _ = app.update(Message::Asbuilt(A::Remove));
    assert!(app.document.as_ref().expect("a drawing").model.monitoring_reviews().is_empty());
    assert!(app.asbuilt.reviews.is_empty());
}

#[test]
fn the_chart_draws_the_mark_against_the_days_with_the_thresholds() {
    let mut app = opened();
    typed(&mut app, "Şartname");
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    // M2 is the first in alarm: chosen when no mark is.
    let track = crate::asbuilt_chart::chosen(&r, "").expect("a mark");
    assert_eq!(track.name, "M2");
    assert_eq!(app.asbuilt.days.len(), 3);
    let pts = crate::asbuilt_chart::points(track, &app.asbuilt.days, crate::asbuilt_chart::Part::Horizontal);
    assert_eq!(pts.len(), 3);
    assert_eq!(pts.iter().map(|p| p.1.map(|d| d - pts[0].1.expect("a day"))).collect::<Vec<_>>(), vec![Some(0), Some(90), Some(181)]);
    let t = crate::asbuilt_trend::ask(&app.asbuilt.trend).expect("asks").1;
    assert_eq!(crate::asbuilt_chart::limits(t.as_ref(), crate::asbuilt_chart::Part::Horizontal), (Some(0.0075), Some(0.03)));
    assert_eq!(crate::asbuilt_chart::limits(t.as_ref(), crate::asbuilt_chart::Part::East), (None, None));
    // Choosing a mark and a part checks nothing again.
    let before = app.asbuilt.answer.clone();
    let _ = app.update(Message::Asbuilt(A::Trend(C::View(crate::asbuilt_trend::View::Chart))));
    let _ = app.update(Message::Asbuilt(A::Trend(C::Mark("M3".into()))));
    let _ = app.update(Message::Asbuilt(A::Trend(C::Part(crate::asbuilt_chart::Part::Up))));
    assert_eq!(app.asbuilt.answer, before);
    assert_eq!(crate::asbuilt_chart::chosen(&r, &app.asbuilt.trend.mark).expect("M3").name, "M3");
}

/// `.run/gui-evidence/izleme-inceleme-5220/`: the İnceleme section with the project's reviews (inceleme), and Grafik
/// (grafik): M2's horizontal movement against the days with its warning and alarm bands.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_review_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/izleme-inceleme-5220");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for step in ["inceleme", "grafik", "grafik-kot"] {
            let mut app = opened();
            let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            typed(&mut app, "İdarenin izleme şartnamesi (2026/14), madde 7");
            let _ = app.update(Message::Asbuilt(A::Keep));
            if step != "inceleme" {
                let _ = app.update(Message::Asbuilt(A::Trend(C::View(crate::asbuilt_trend::View::Chart))));
            }
            if step == "grafik-kot" {
                let _ = app.update(Message::Asbuilt(A::Trend(C::Mark("M3".into()))));
                let _ = app.update(Message::Asbuilt(A::Trend(C::Part(crate::asbuilt_chart::Part::Up))));
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let at = Point::new(size.width * 0.5, size.height * 0.45);
            snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, if step == "inceleme" { -7.0 } else { -60.0 }));
            let file = out.join(format!("{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
