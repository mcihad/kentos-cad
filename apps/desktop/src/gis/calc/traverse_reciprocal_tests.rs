//! Poligon hesabı's reciprocal zenith angles (traverse.rs, docs/adr/4500): an open traverse of two 100 m legs, the
//! first sighted back from its far end too. With Karşılıklı on the back sight's columns show; the leg's height
//! difference is the mean of its two one-way differences (worked out here: the project's k falls out), the report
//! gives the pair's k (Baselga et al. 2014, eq. 11, worked out here) and the work carries the back sight; the second
//! leg, one way, keeps the project's k. A back sight left without its heights is taken as one chord with the forward
//! sight.

use super::tests::{calc, last_work};
use super::*;
use crate::files_testing::app_with_drawing;

/// One way: S·cot z + i − t (+ the curvature and refraction for k), z in grad.
fn one_way(z: f64, i: f64, t: f64) -> f64 {
    let z = z * std::f64::consts::PI / 200.0;
    100.0 * z.cos() / z.sin() + i - t
}

/// The pair's k (Baselga et al. 2014, eq. 11): ½ sin z + ½ sin z′ + (2R/s) cos((z + z′)/2), s = S / sin z.
fn pair_k(z: f64, zb: f64) -> f64 {
    let (z, zb) = (z * std::f64::consts::PI / 200.0, zb * std::f64::consts::PI / 200.0);
    z.sin() / 2.0 + zb.sin() / 2.0 + 2.0 * 6_371_000.0 / (100.0 / z.sin()) * ((z + zb) / 2.0).cos()
}

fn typed(app: &mut App) {
    use traverse::{ANGLE, BACK_INSTRUMENT, BACK_TARGET, BACK_ZENITH, DISTANCE, INSTRUMENT, NAME, TARGET, ZENITH};
    let _ = app.run("calc.traverse");
    calc(app, Event::TraverseKind(traverse::Kind::Open));
    calc(app, Event::Known(Field::Start, "0,0".into()));
    calc(app, Event::Known(Field::Back, "-100,0".into()));
    calc(app, Event::TraverseHeights(true));
    calc(app, Event::TraverseStartZ("100".into()));
    calc(app, Event::TraverseReciprocal(true));
    for (col, v) in [
        (ANGLE, "200"),
        (DISTANCE, "100"),
        (ZENITH, "99,4"),
        (INSTRUMENT, "1,5"),
        (TARGET, "1,6"),
        (BACK_ZENITH, "100,6009"),
        (BACK_INSTRUMENT, "1,6"),
        (BACK_TARGET, "1,5"),
    ] {
        calc(app, Event::Cell(0, col, v.into()));
    }
    for (col, v) in [(NAME, "P1"), (ANGLE, "200"), (DISTANCE, "100"), (ZENITH, "100,2"), (INSTRUMENT, "1,5"), (TARGET, "1,6")] {
        calc(app, Event::Cell(1, col, v.into()));
    }
    calc(app, Event::Cell(2, NAME, "P2".into()));
}

#[test]
fn a_leg_sighted_back_takes_the_mean_and_says_its_pair_s_k() {
    let mut app = app_with_drawing();
    typed(&mut app);
    assert_eq!(super::grid::Table::columns(&app.calc.traverse), 9, "the back sight's columns shown");
    let doc = app.document.as_ref().expect("open");
    let c = app.calc.traverse.compute(&doc.model);
    assert!(c.errors.is_empty(), "{:?}", c.errors);
    let r = c.result.expect("a result");
    let k = doc.settings().refraction();
    let first = (one_way(99.4, 1.5, 1.6) - one_way(100.6009, 1.6, 1.5)) / 2.0;
    let second = one_way(100.2, 1.5, 1.6) + (1.0 - k) * 100.0 * 100.0 / (2.0 * 6_371_000.0);
    assert!((r.legs[0].dh.expect("a difference") - first).abs() < 1e-9, "{:?}", r.legs[0]);
    assert!((r.legs[1].dh.expect("a difference") - second).abs() < 1e-9, "{:?}", r.legs[1]);
    let pair = r.legs[0].refraction.expect("one chord: its pair's k");
    assert!((pair - pair_k(99.4, 100.6009)).abs() < 1e-9, "{pair}");
    assert!(r.legs[1].refraction.is_none(), "one way");
    let format = Format::of(doc.settings());
    let report = app.calc.traverse.report(&doc.model, &format).expect("a report");
    assert!(report.iter().any(|row| row.last().is_some_and(|c| c == "k")), "{report:?}");
    calc(&mut app, Event::AddPoints);
    let (_, work) = last_work(&app);
    assert_eq!(work["heights"]["back"][0]["zenith"], serde_json::json!(100.6009));
    assert_eq!(work["heights"]["back"][1], serde_json::Value::Null);
}

#[test]
fn a_back_sight_without_its_heights_is_one_chord_with_the_forward_sight() {
    use traverse::{BACK_INSTRUMENT, BACK_TARGET};
    let mut app = app_with_drawing();
    typed(&mut app);
    calc(&mut app, Event::Cell(0, BACK_INSTRUMENT, String::new()));
    calc(&mut app, Event::Cell(0, BACK_TARGET, String::new()));
    let doc = app.document.as_ref().expect("open");
    let c = app.calc.traverse.compute(&doc.model);
    let b = c.back.expect("sighted back")[0].clone().expect("the first leg");
    assert_eq!((b.instrument, b.target), (1.6, 1.5), "the forward target's and instrument's heights");
    assert!(c.result.expect("a result").legs[0].refraction.is_some());
}

/// `.run/gui-evidence/karsilikli-basucu-4500/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib calc::traverse_reciprocal_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/karsilikli-basucu-4500");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        typed(&mut app);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-karsilikli-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// Docs/adr/4900: the pair's k is the measured k; Projeye yaz makes it the project's, and the leg sighted one way is
/// worked out with it: its curvature and refraction term (1 − k)·S²/2R the test's own.
#[test]
fn projeye_yaz_makes_the_measured_k_the_project_s() {
    let mut app = app_with_drawing();
    typed(&mut app);
    let doc = app.document.as_ref().expect("open");
    let m = app.calc.traverse.measured(&doc.model).expect("a pair");
    let pair = pair_k(99.4, 100.6009);
    assert_eq!(m.pairs, 1);
    assert!((m.mean - pair).abs() < 1e-9 && m.sd.is_none(), "{m:?}");
    calc(&mut app, Event::TraverseRefraction);
    let doc = app.document.as_ref().expect("open");
    assert!((doc.settings().refraction() - pair).abs() < 1e-12, "the project's k now");
    let r = app.calc.traverse.compute(&doc.model).result.expect("a result");
    let second = one_way(100.2, 1.5, 1.6) + (1.0 - pair) * 100.0 * 100.0 / (2.0 * 6_371_000.0);
    assert!((r.legs[1].dh.expect("a difference") - second).abs() < 1e-9, "{:?}", r.legs[1]);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|l| l.starts_with("Projenin kırılma katsayısı k =")), "{said:?}");
}

/// `.run/gui-evidence/olculen-k-4900/`: the measured k under the summary, with Projeye yaz.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib traverse_reciprocal_tests::screens_measured -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_measured() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/olculen-k-4900");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        typed(&mut app);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-olculen-k-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
