//! İstatistik's Kapsam over a bound layer's part (docs/adr/0581 §6): Kaynağın
//! tümü asks the whole table for the chosen field, the answer is shown for the
//! layer and the field it was asked for, Durdur and Çalışma kopyası leave it.

use std::path::PathBuf;

use kentos_commands::{CallOp, QueryScope, SourcesQuery};
use kentos_native_application::sources::trouble::Trouble;
use kentos_native_application::{ExecutionContext, registry};

use super::Event;
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, scratch};
use crate::source_trouble::Refused;

const LAYER: &str = "import-nokta";

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Statistics(e));
}

/// A drawing with `sorgu.gpkg`'s `nokta` bound by a part (its first 12 points), İstatistik open on it.
fn scene(name: &str) -> App {
    let file = scratch(name).join("sorgu.gpkg");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sources/v1/sorgu.gpkg"),
        &file,
    )
    .expect("copied");
    let mut app = app_with_drawing();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        "cad.sources.edit",
        CallOp::Execute,
        serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": "nokta",
                                      "extent": [486995.0, 4419995.0, 487105.0, 4420025.0] } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let _ = app.update(Message::Run("data.statistics"));
    ev(&mut app, Event::Layer(LAYER.into()));
    app
}

/// What the thread asks, asked here and handed to the tab as the thread hands it.
fn answer(app: &mut App) {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let link = doc.settings().sources[0].clone();
    let field = app.charts.field.clone().expect("a field");
    let input = SourcesQuery {
        layer: Some(LAYER.into()),
        scope: Some(QueryScope::Source),
        fields: Some(vec![field.clone()]),
        limit: Some(0),
        count: Some(true),
        statistics: Some(vec![field.clone()]),
        ..SourcesQuery::default()
    };
    let told = Refused::of(
        kentos_native_application::sources_query::ask_source(
            doc.settings(),
            link.kind,
            &link.path,
            &link.table,
            LAYER,
            &input,
            doc.revision().to_string(),
            &|| false,
        ),
        "durduruldu",
    );
    ev(
        app,
        Event::WholeAnswered(LAYER.into(), field, Box::new(told)),
    );
}

#[test]
fn the_whole_source_is_asked_for_the_chosen_field() {
    let mut app = scene("istatistik-kapsam");
    assert!(app.stats_bound().is_some());
    ev(&mut app, Event::Field("kot".into()));
    // The working copy: the part's 12 points.
    app.stats_refresh();
    assert_eq!(app.charts.stats.borrow().objects, 12);
    ev(&mut app, Event::Scope(true));
    assert!(app.charts.whole.running.is_some());
    assert_eq!(
        app.charts.whole.asked,
        Some((LAYER.to_owned(), "kot".to_owned()))
    );
    answer(&mut app);
    assert!(app.charts.whole.running.is_none());
    let told = app
        .charts
        .whole
        .answer
        .clone()
        .expect("an answer")
        .expect("read");
    // 250 rows; 14 without a value (every 17th), one a text ('ölçülmedi').
    assert_eq!(told.total, Some(250));
    let st = &told.statistics[0];
    assert_eq!((st.read, st.skipped), (235, 1));
    assert_eq!(told.read, 250);
    // An answer for another field is not this one's.
    ev(
        &mut app,
        Event::WholeAnswered(
            LAYER.into(),
            "no".into(),
            Box::new(Err(Refused {
                code: "x".into(),
                why: "başka".into(),
            })),
        ),
    );
    assert!(app.charts.whole.answer.as_ref().is_some_and(Result::is_ok));
    // Çalışma kopyası leaves it.
    ev(&mut app, Event::Scope(false));
    assert!(app.charts.whole.answer.is_none() && app.charts.whole.asked.is_none());
}

/// A source that could not be read (docs/adr/0740 §2): the refusal is kept with what helps; Yeniden dene asks again
/// once the file is back, and a relinked layer asks again by itself.
#[test]
fn a_source_not_read_is_said_and_asked_again() {
    let mut app = scene("istatistik-sorun");
    let file = std::path::PathBuf::from(
        &app.document
            .as_ref()
            .expect("a drawing")
            .model
            .settings()
            .sources[0]
            .path,
    );
    let away = file.with_file_name("uzakta.gpkg");
    std::fs::rename(&file, &away).expect("moved away");
    ev(&mut app, Event::Field("kot".into()));
    ev(&mut app, Event::Scope(true));
    answer(&mut app);
    let refused = app
        .charts
        .whole
        .answer
        .clone()
        .expect("an answer")
        .expect_err("refused");
    assert_eq!(refused.code, "file_not_found");
    assert_eq!(refused.way(), Some(Trouble::Relink));
    // Back: Yeniden dene asks the same field again.
    std::fs::rename(&away, &file).expect("moved back");
    ev(&mut app, Event::WholeRetry);
    assert!(app.charts.whole.running.is_some());
    answer(&mut app);
    let told = app
        .charts
        .whole
        .answer
        .clone()
        .expect("an answer")
        .expect("read");
    assert_eq!(told.total, Some(250));
    // Moved for good: relinked to the new place, the question is asked again by itself.
    let moved = file.with_file_name("yeni.gpkg");
    std::fs::rename(&file, &moved).expect("moved");
    ev(&mut app, Event::WholeRetry);
    answer(&mut app);
    assert!(app.charts.whole.answer.as_ref().is_some_and(Result::is_err));
    let id = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .sources[0]
        .id
        .clone();
    let _ = app.update(Message::Bound(crate::bound::Event::Relinked(
        id,
        Some(moved),
    )));
    assert!(app.charts.whole.running.is_some(), "asked again");
    answer(&mut app);
    assert!(app.charts.whole.answer.as_ref().is_some_and(Result::is_ok));
}

#[test]
fn durdur_stops_the_question() {
    let mut app = scene("istatistik-durdur");
    ev(&mut app, Event::Field("kot".into()));
    ev(&mut app, Event::Scope(true));
    let stop = app.charts.whole.running.clone().expect("running");
    ev(&mut app, Event::WholeStop);
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed));
    assert!(app.charts.whole.running.is_none());
}

/// İstatistik with Kapsam: the part's statistics, the whole source's, and the source being read
/// (`.run/shots/istatistik-kapsam-*`).
#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let mut app = scene(&format!("istatistik-kapsam-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(Size::new(width, height)));
        ev(&mut app, Event::Field("kot".into()));
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            // Every summary counted, as the frames count them.
            let _ = app.statistics_seen();
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                if !matches!(message, Message::Statistics(Event::Scope(_))) {
                    let _ = app.update(message);
                }
            });
            let path = out.join(format!(
                "istatistik-kapsam-{mode}-{label}-{width}x{height}.png"
            ));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        picture(&mut app, "kopya");
        ev(&mut app, Event::Scope(true));
        picture(&mut app, "okunuyor");
        answer(&mut app);
        picture(&mut app, "tumu");
        // The source moved away: why, and the way out (docs/adr/0740 §2).
        let file = std::path::PathBuf::from(
            &app.document
                .as_ref()
                .expect("a drawing")
                .model
                .settings()
                .sources[0]
                .path,
        );
        let away = file.with_file_name("uzakta.gpkg");
        std::fs::rename(&file, &away).expect("moved away");
        ev(&mut app, Event::WholeRetry);
        answer(&mut app);
        picture(&mut app, "okunamadi");
        std::fs::rename(&away, &file).expect("moved back");
    }
}
