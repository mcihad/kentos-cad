//! Kesit yöntemi in Yüzey analizi › Fark (docs/adr/4340; plan/map-cad.md MAP-15): the method's third choice reads the
//! sections across the axis (spacing, azimuth, average end area or prismoidal) and the window's answer is the shared
//! case's; a wrong azimuth is said; and its pictures (the sections' chart, the summary, the sections' table).

use crate::app::{App, Message};
use crate::surface_analysis_window::{Event as W, Mode, Read, Tab};

fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.compare.sections.json")).expect("cases")
}

/// The case named `name`'s answer.
fn case(name: &str) -> serde_json::Value {
    let cases = cases();
    let c = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone();
    c["steps"][0]["result"]["output"].clone()
}

/// The section cases' drawing: the compare lattices.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::SurfaceAnalysis(e));
}

fn read(app: &App) -> Read {
    match &app.surface_analysis_window.as_ref().expect("open").preview {
        Some(Ok(r)) => r.clone(),
        other => panic!("a reading: {other:?}"),
    }
}

/// Fark of the cut over the plane by sections across an east-pointing axis every 2.5 m.
fn by_sections(app: &mut App) {
    let _ = app.update(Message::Run("terrain.analysis"));
    send(app, W::Mode(Mode::Compare));
    send(app, W::Surface("t-5".into()));
    send(app, W::Comparison("t-8".into()));
    send(app, W::VolumeMethod(2));
    send(app, W::Field(17, "2,5".into()));
    send(app, W::Field(18, "90".into()));
    send(app, W::Preview);
}

fn close(a: f64, b: &serde_json::Value) -> bool {
    (a - b.as_f64().expect("a number")).abs() < 1e-6
}

#[test]
fn fark_by_sections_is_the_shared_case_s() {
    let mut app = app();
    by_sections(&mut app);
    let Read::Compare(c) = read(&app) else { panic!("a difference") };
    let want = case("Kesitler kuzey–güney (semt 90°) her 2,5 m");
    assert_eq!(c.sections.len(), want["sections"].as_array().expect("sections").len());
    assert!(close(c.cut, &want["cut"]) && close(c.fill, &want["fill"]), "{c:?}");
    assert!(close(c.exact.as_ref().expect("the exact TIN beside").cut, &want["exact"]["cut"]));
    for (s, w) in c.sections.iter().zip(want["sections"].as_array().expect("sections")) {
        assert!(close(s.station, &w["station"]) && close(s.cut_area, &w["cutArea"]) && close(s.fill_area, &w["fillArea"]), "{s:?}");
    }
    // Prizmoidal: the middle sections.
    send(&mut app, W::Prismoidal(true));
    send(&mut app, W::Preview);
    let Read::Compare(p) = read(&app) else { panic!("a difference") };
    let want = case("Prizmoidal: ortadaki kesitle");
    assert!(close(p.cut, &want["cut"]), "{} ≠ {}", p.cut, want["cut"]);
    assert_eq!(p.formula, Some(kentos_commands::SectionFormula::Prismoidal));
    // A spacing typed as nothing is 10 m; an azimuth of 400° is said, nothing read.
    send(&mut app, W::Field(17, String::new()));
    send(&mut app, W::Preview);
    let Read::Compare(d) = read(&app) else { panic!("a difference") };
    assert_eq!(d.interval, Some(10.0));
    send(&mut app, W::Field(18, "400".into()));
    send(&mut app, W::Preview);
    assert!(matches!(&app.surface_analysis_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("semt")));
}

/// `.run/gui-evidence/kesit-yontemi-4340/`: the sections' chart, the summary with the exact TIN beside and the method's
/// difference, the sections' table.
///
/// ```text
/// cargo test -p kentos-desktop --lib compare_sections_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kesit-yontemi-4340");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::RibbonTab("terrain"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        by_sections(&mut app);
        send(&mut app, W::Field(17, "1".into()));
        send(&mut app, W::Preview);
        for (tab, list, which) in [(Tab::Chart, "Özet", "grafik"), (Tab::Content, "Özet", "icerik-ozet"), (Tab::Content, "Kesitler", "icerik-kesitler")] {
            send(&mut app, W::Tab(tab));
            send(&mut app, W::List(list));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("fark-kesit-{which}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
