//! Zaman ayarları's field and words (the CBS review's 19th item, docs/adr/9335) on a GPX-like layer: nine points with
//! Ad and a UTC Zaman a minute apart, and a track without a time. The window opens on Zaman, which holds the dates, not
//! Ad, the first name; the fields holding dates come first with how many and one of them as read; it says that the
//! track always shows and that the clock is UTC. The slider's moment is said with UTC beside its clock.

use serde_json::json;

use crate::app::{App, Message};
use crate::files_testing::drive;

/// A CBS project whose layer `sample` holds nine points (Ad P1…P9, Zaman 2024-05-12T09:31:00Z…09:39:00Z) and a track
/// (Ad İz) without a time.
fn gpx() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = crate::tools_scenes::Objects::new();
    for i in 0..9 {
        let id = o.point("sample", [10.0 * f64::from(i), 0.0], Some(1000.0));
        let name = format!("P{}", i + 1);
        let at = format!("2024-05-12T09:{:02}:00Z", 31 + i);
        o.data(id, &[("Ad", name.as_str()), ("Zaman", at.as_str())], None);
    }
    let track = o.path("sample", &[[0.0, 5.0], [80.0, 5.0]], false);
    o.data(track, &[("Ad", "İz")], None);
    crate::tools_scenes::open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        d["layers"] = json!([
            { "id": "sample", "name": "sample", "type": "layer", "visible": true, "locked": false, "expanded": true,
              "style": { "color": "#0090FF", "lineType": "continuous", "lineWeight": 0.35 }, "children": [] }
        ]);
        d["activeLayer"] = json!("sample");
    });
    app
}

#[test]
fn the_window_opens_on_the_field_holding_dates_and_says_the_untimed_and_utc() {
    let mut app = gpx();
    app.open_time_layer(Some("sample".into()));
    let w = app.gis.time.layer.clone().expect("the window");
    assert_eq!(
        w.start, "Zaman",
        "the field holding the dates, not the first name"
    );
    let names: Vec<&str> = w.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["Zaman", "Ad"], "the fields holding dates first");
    assert_eq!(w.fields[0].detail(), "9 tarih · ör. 12.05.2024 09:31 UTC");
    assert_eq!(w.fields[1].detail(), "tarih değil (10 değer)");
    let (ready, lines) = app.time_layer_summary(&w);
    assert!(ready);
    let lines: Vec<String> = lines.into_iter().map(|(_, l)| l).collect();
    assert!(
        lines[0].starts_with("10 nesne: 9 zamanlı, 1 zamansız"),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"1 zamansız nesne sürgü açıkken de hep görünür.".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("Saatler UTC'dir")),
        "{lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("tarih olarak okunamadı")),
        "{lines:?}"
    );
}

/// Zaman ayarları saved on Zaman, the slider on.
fn timed() -> App {
    let mut app = gpx();
    app.open_time_layer(Some("sample".into()));
    let w = app.gis.time.layer.clone().expect("the window");
    app.write_time(&w.layer, w.chosen());
    let task = app.update(Message::Run("time.slider"));
    drive(&mut app, task);
    app
}

/// The pictures (`.run/gui-evidence/kullanim-9335/`): Zaman ayarları on the GPX layer, its fields with their dates and
/// its words; the slider with UTC beside its clock.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib temporal::layer::tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9335", "01-zaman-ayarlari", || {
        let mut app = gpx();
        app.open_time_layer(Some("sample".into()));
        app
    });
    crate::files_testing::usage_shots("9335", "02-zaman-surgusu-utc", timed);
}
