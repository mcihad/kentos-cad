//! Düşey kurp boyu (profile_lengths_window.rs, plan/road.md Y31.3): opened from Boykesit on the profile as typed, it
//! reads `road.profiles.lengths`; Kısaları uzat sets the short curves in Boykesit's PVI table, which Kaydet writes;
//! the values typed are kept when it is closed; and its pictures.

use crate::app::{App, Message};
use crate::profile_lengths_window::Event as L;
use crate::profile_window::Event as W;
use crate::road;

/// A road project with `road.profiles.query`'s drawing and no profile.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.profiles.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    setup["settings"]["road"].as_object_mut().expect("road").remove("profiles");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::Profiles(e)));
}

fn lengths(app: &mut App, e: L) {
    send(app, W::Lengths(e));
}

/// A profile on the main alignment: its ends on the terrain, a short parabola at km 200, a kink at km 420 and a
/// circle (R 800 m) at km 560.
fn made(app: &mut App) {
    let _ = app.update(Message::Run("profile.new"));
    send(app, W::FromGround);
    for (j, station, elevation) in [(1, "200", "104"), (2, "420", "96"), (3, "560", "103")] {
        send(app, W::Add);
        send(app, W::Pvi(j, 1, station.into()));
        send(app, W::Pvi(j, 2, elevation.into()));
    }
    send(app, W::Curve(1, 1));
    send(app, W::Pvi(1, 3, "40".into()));
    send(app, W::Curve(3, 3));
    send(app, W::Pvi(3, 3, "800".into()));
}

/// 80 km/h, AASHTO's 2,5 s and 3,4 m/s², 1,08 m eye, 0,60 m object, 0,60 m headlights at 1°, 0,3 m/s², rounded up to
/// 10 m: the user's numbers, typed.
fn typed(app: &mut App) {
    send(app, W::OpenLengths);
    for (k, v) in [(0, "80"), (1, "2,5"), (2, "3,4"), (3, "1,08"), (4, "0,6"), (6, "0,6"), (7, "1"), (8, "0,3"), (10, "10")] {
        lengths(app, L::Field(k, v.into()));
    }
}

fn window(app: &App) -> &crate::profile_lengths_window::Lengths {
    app.road
        .profile_window
        .as_ref()
        .and_then(|w| w.lengths.as_deref())
        .expect("open")
}

#[test]
fn short_curves_are_lengthened_in_their_kind_and_saved() {
    let mut app = app();
    made(&mut app);
    typed(&mut app);
    let out = window(&app).result.clone().expect("read").expect("reads");
    assert_eq!(out.curves.len(), 3, "{out:?}");
    let s = out.stopping.expect("stopping");
    assert!((s - (80.0 / 3.6 * 2.5 + (80.0f64 / 3.6).powi(2) / (2.0 * 3.4))).abs() < 1e-9, "{s}");
    let [p1, p2, p3] = [&out.curves[0], &out.curves[1], &out.curves[2]];
    assert!(p1.crest && !p2.crest && !p3.crest, "the terrain's end is high: {out:?}");
    assert_eq!((p1.length, p2.length), (Some(40.0), None), "a parabola 40 m, a kink");
    assert!((p3.length.unwrap() - 800.0 * p3.a.abs() / 100.0).abs() < 1e-9, "a circle's length is R·A");
    assert!(p1.short && p2.short && p3.short, "{out:?}");
    assert_eq!(p1.governing.as_deref(), Some("stopping"));
    assert_eq!(p2.governing.as_deref(), Some("headlight"), "the sag's headlights ask more than comfort");
    assert!(out.curves.iter().all(|c| c.suggested >= c.required && (c.suggested / 10.0).fract().abs() < 1e-9));
    assert!(window(&app).warnings.iter().any(|w| w.contains("kısa")), "{:?}", window(&app).warnings);

    // v2 and v3 are 140 m apart and their suggestions ask 240 m of arms: the window says so before the press.
    assert_eq!(window(&app).cut, ["v2", "v3"]);

    // Kısaları uzat: back to Boykesit; v1 takes its suggestion, the kink a parabola, the circle stays a circle, the
    // two cut to share the gap; the profile still builds.
    lengths(&mut app, L::Apply);
    assert!(
        app.road.profile_window.as_ref().unwrap().lengths.is_none(),
        "back to Boykesit"
    );
    assert!(
        app.log.lines().any(|l| l.text.contains("v2, v3")),
        "says which were cut"
    );
    send(&mut app, W::OpenLengths);
    let again = window(&app).result.clone().expect("read").expect("builds");
    assert!(!again.curves[0].short && again.curves[0].length == Some(p1.suggested), "{again:?}");
    let (l2, l3) = (again.curves[1].length.expect("a parabola now"), again.curves[2].length.expect("a circle"));
    assert!(l2 / 2.0 + l3 / 2.0 <= 140.0 + 1e-9 && l2 / 2.0 + l3 / 2.0 > 139.99, "{l2} {l3}");
    assert!(l2 > l3 && l3 > p3.length.unwrap(), "shared in their ratio, not under what it had: {l2} {l3}");
    lengths(&mut app, L::Back);
    send(&mut app, W::Save);
    let doc = &app.document.as_ref().unwrap().model;
    let pvis = &doc.road_profiles()[0].pvis;
    assert_eq!((pvis[1].length, pvis[2].length), (p1.suggested, l2));
    assert!(pvis[3].radius > 800.0, "the circle's radius grew: {}", pvis[3].radius);
    let _ = p2;
}

/// Where the PVIs leave room, every suggestion is reached.
#[test]
fn with_room_every_short_curve_reaches_its_suggestion() {
    let mut app = app();
    made(&mut app);
    send(&mut app, W::Pvi(3, 1, "600".into()));
    typed(&mut app);
    lengths(&mut app, L::Field(0, "40".into()));
    assert!(window(&app).result.clone().unwrap().unwrap().curves.iter().any(|c| c.short));
    assert!(window(&app).cut.is_empty(), "{:?}", window(&app).result);
    let before = window(&app).result.clone().unwrap().unwrap();
    lengths(&mut app, L::Apply);
    send(&mut app, W::OpenLengths);
    let after = window(&app).result.clone().unwrap().unwrap();
    for (b, a) in before.curves.iter().zip(&after.curves) {
        assert!(!a.short, "{a:?}");
        if b.short {
            assert!((a.length.unwrap() - b.suggested).abs() < 2e-3, "{b:?} → {a:?}");
        }
    }
}

#[test]
fn a_missing_value_says_what_to_type_and_the_values_are_kept() {
    let mut app = app();
    made(&mut app);
    send(&mut app, W::OpenLengths);
    let why = window(&app).result.clone().expect("read").expect_err("no criterion");
    assert!(why.contains("ölçüt"), "{why}");
    // An empty speed is the alignment's (docs/adr/0986); this one has none, so the command asks for it.
    lengths(&mut app, L::Field(9, "0,5".into()));
    let why = window(&app).result.clone().expect("read").expect_err("no speed anywhere");
    assert!(why.contains("proje hızını (speed) verin ya da aksa proje hızı yazın"), "{why}");
    lengths(&mut app, L::Field(0, "80".into()));
    let out = window(&app).result.clone().expect("read").expect("the bisector alone");
    assert!(out.curves.iter().all(|c| c.governing.as_deref() == Some("bisector")));
    // Geri keeps the typed values; the window opens again with them.
    lengths(&mut app, L::Back);
    send(&mut app, W::OpenLengths);
    assert_eq!((window(&app).fields[0].as_str(), window(&app).fields[9].as_str()), ("80", "0,5"));
}

/// The window, wide and narrow, light and dark; `.run/gui-evidence/yol-y26/y31-3-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib profile_lengths_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y26");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        // A fresh app for each state: a snapshot keeps the tree, and a switch's slide stops half way.
        for step in ["bos", "oneri", "boykesit"] {
            let mut app = app();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            let _ = app.update(Message::RibbonTab("alignment"));
            made(&mut app);
            match step {
                "bos" => send(&mut app, W::OpenLengths),
                "oneri" => typed(&mut app),
                _ => {
                    typed(&mut app);
                    lengths(&mut app, L::Apply);
                }
            }
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("y31-3-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
