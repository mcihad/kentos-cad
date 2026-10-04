//! Karne editörü's flow on the sample book and a text book, and its pictures.

use std::path::PathBuf;

use kentos_contracts::SurveySettings;

use super::{Event, NAME, USE};
use crate::app::{App, Message};
use crate::calc::grid::Table;
use crate::calc::{Event as CalcEvent, Window};

fn sample() -> PathBuf {
    sample_named("sample.gsi")
}

/// A sample book of fixtures/field/v1 (scripts/fixtures/field_samples.py).
fn sample_named(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/field/v1")
        .join(name)
}

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::Calc(CalcEvent::FieldBook(e)));
}

/// An app with a drawing whose project checks the two faces' slope
/// distances against 5 mm, Karne editörü open on the sample book.
fn opened() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.settings().clone();
    settings.survey = Some(SurveySettings {
        face_slope: Some(0.005),
        ..SurveySettings::default()
    });
    doc.model.set_settings(settings);
    let _ = app.update(Message::Run("calc.fieldbook"));
    send(&mut app, Event::Picked(Some(sample())));
    app
}

/// The sample GSI book (docs/adr/0169 §6): read by its content, its
/// first station reduced, faces paired, 103's slope difference above the
/// project's 5 mm; an observation left out and one renamed reduce again.
#[test]
fn a_gsi_book_is_read_reduced_and_edited() {
    let mut app = opened();
    assert_eq!(app.calc.open, Some(Window::FieldBook));
    let form = &app.calc.fieldbook;
    let book = form.book.as_ref().expect("read");
    assert_eq!((book.format.as_str(), book.stations.len()), ("gsi", 2));
    let r = form.reduction.as_ref().expect("reduced");
    let rows: Vec<(&str, usize)> = r
        .rows
        .iter()
        .map(|x| (x.target.as_str(), x.faces))
        .collect();
    assert_eq!(
        rows,
        [
            ("P2", 2),
            ("101", 2),
            ("102", 2),
            ("103", 2),
            ("104", 1),
            ("ST2", 2)
        ]
    );
    let over: Vec<&str> = r
        .rows
        .iter()
        .filter(|x| !x.over.is_empty())
        .map(|x| x.target.as_str())
        .collect();
    assert_eq!(over, ["103"]);
    // P2's face I left out: its face II reads alone.
    let _ = app.update(Message::Calc(CalcEvent::Cell(0, USE, "0".to_owned())));
    let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
    assert_eq!((r.rows[5].target.as_str(), r.rows[5].faces), ("P2", 1));
    // 104 renamed.
    let _ = app.update(Message::Calc(CalcEvent::Cell(4, NAME, "105".to_owned())));
    let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
    assert!(r.rows.iter().any(|x| x.target == "105"));
    assert_eq!(Table::get(&app.calc.fieldbook, 4, NAME), "105");
    // The second station.
    send(&mut app, Event::Station(1));
    let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
    assert_eq!(r.rows.len(), 5);
}

/// The same book from other instruments (docs/adr/0169 §1, step 5): the
/// Sokkia SDR33, Topcon GTS-7 and Nikon RAW samples are told by their
/// content, named so, and reduce into the GSI sample's rows, station by
/// station.
#[test]
fn an_sdr_book_reduces_as_the_gsi_book() {
    let mut app = opened();
    let rows = |app: &App| {
        let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
        r.rows
            .iter()
            .map(|x| {
                (
                    x.target.clone(),
                    x.faces,
                    x.hz,
                    x.zenith,
                    x.horizontal,
                    x.dh,
                )
            })
            .collect::<Vec<_>>()
    };
    let gsi = [rows(&app), {
        send(&mut app, Event::Station(1));
        rows(&app)
    }];
    for (file, format, name) in [
        ("sample.sdr", "sdr", "Sokkia SDR"),
        ("sample.gt7", "gts7", "Topcon GTS-7"),
        ("sample-nikon.raw", "nikon", "Nikon RAW"),
    ] {
        send(&mut app, Event::Picked(Some(sample_named(file))));
        let book = app.calc.fieldbook.book.as_ref().expect("read");
        assert_eq!((book.format.as_str(), book.stations.len()), (format, 2));
        assert_eq!(super::format_name(&book.format), name);
        send(&mut app, Event::Station(0));
        assert_eq!(rows(&app), gsi[0], "{file}");
        send(&mut app, Event::Station(1));
        assert_eq!(rows(&app), gsi[1], "{file}");
    }
}

/// The JobXML sample (docs/adr/0169 §1, step 5c) is the same book in
/// decimal degrees: told by its content, named so, and reduced into the GSI
/// sample's rows with their angles turned into degrees (a gon is 0.9°),
/// lengths and height differences to the micrometre.
#[test]
fn a_jobxml_book_reduces_as_the_gsi_book_in_degrees() {
    let mut app = opened();
    type Row = (String, usize, f64, Option<f64>, Option<f64>, Option<f64>);
    let rows = |app: &App| -> Vec<Row> {
        let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
        r.rows
            .iter()
            .map(|x| {
                (
                    x.target.clone(),
                    x.faces,
                    x.hz,
                    x.zenith,
                    x.horizontal,
                    x.dh,
                )
            })
            .collect()
    };
    let gsi = [rows(&app), {
        send(&mut app, Event::Station(1));
        rows(&app)
    }];
    send(&mut app, Event::Picked(Some(sample_named("sample.jxl"))));
    let book = app.calc.fieldbook.book.as_ref().expect("read");
    assert_eq!((book.format.as_str(), book.stations.len()), ("jobxml", 2));
    assert_eq!(super::format_name(&book.format), "Trimble JobXML");
    let near = |a: Option<f64>, b: Option<f64>, tolerance: f64| match (a, b) {
        (Some(a), Some(b)) => (a - b).abs() < tolerance,
        (a, b) => a.is_none() && b.is_none(),
    };
    for (at, want) in gsi.iter().enumerate() {
        send(&mut app, Event::Station(at));
        let got = rows(&app);
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(want) {
            assert_eq!((&g.0, g.1), (&w.0, w.1));
            assert!((g.2 - w.2 * 0.9).abs() < 1e-9, "{g:?} {w:?}");
            assert!(near(g.3, w.3.map(|z| z * 0.9), 1e-9), "{g:?} {w:?}");
            assert!(near(g.4, w.4, 1e-6) && near(g.5, w.5, 1e-6), "{g:?} {w:?}");
        }
    }
}

/// A text book is read only to its first line until its point and
/// horizontal reading are mapped; the mapping is remembered.
#[test]
fn a_text_book_is_mapped_then_read() {
    let dir = std::env::temp_dir().join(format!("kentos-karne-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("karne.csv");
    std::fs::write(
        &file,
        "İstasyon;Nokta;Hz;V;SD\nS1;A;10;99;100\nS1;A;210,001;301;100,002\n",
    )
    .expect("written");
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Run("calc.fieldbook"));
    send(&mut app, Event::Picked(Some(file.clone())));
    let book = app.calc.fieldbook.book.as_ref().expect("read");
    assert_eq!((book.format.as_str(), book.stations.len()), ("csv", 0));
    assert_eq!(book.first_line, ["İstasyon", "Nokta", "Hz", "V", "SD"]);
    for (field, col) in [(0, 0), (2, 1), (3, 2), (4, 3), (5, 4)] {
        send(&mut app, Event::Map(field, Some(col)));
    }
    let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
    assert_eq!((r.rows.len(), r.rows[0].faces), (1, 2));
    // Opened again, the same mapping reads it at once.
    send(&mut app, Event::Picked(Some(file)));
    assert_eq!(
        app.calc.fieldbook.book.as_ref().map(|b| b.stations.len()),
        Some(1)
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// The project's traverse tolerances of the pictures: the leg's two ends
/// 3 mm (the sample's differ by more), the misclosures 60 cc and 30 mm.
fn with_traverse_tolerances(app: &mut App) {
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.settings().clone();
    settings.survey = Some(SurveySettings {
        face_slope: Some(0.005),
        two_way: Some(0.003),
        traverse_angle: Some(60.0 * std::f64::consts::PI / 2_000_000.0),
        traverse_coord: Some(0.03),
        ..SurveySettings::default()
    });
    doc.model.set_settings(settings);
}

/// Karne editörü's pictures: the sample book in the light theme at
/// 1440×900 and the dark at 1100×650, the same book from a Sokkia SDR33, a
/// Topcon GTS-7, a Nikon RAW and a Trimble JobXML file, a text book's
/// mapping, Kutupsal alım
/// filled from the first station, the traverse's leg above its two-way
/// tolerance and Poligon hesabı filled from both stations.
#[test]
#[ignore = "writes pictures: cargo test -p kentos-desktop calc::fieldbook::tests::screens -- --ignored --nocapture"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let dir = std::env::temp_dir().join(format!("kentos-karne-shots-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let csv = dir.join("arazi-karnesi.csv");
    std::fs::write(
        &csv,
        "İstasyon;Alet;Nokta;Hz;V;SD;Prizma;Kod\nS1;1,55;A;10;99;100;1,6;POL\n",
    )
    .expect("written");
    for (theme, w, h) in [("light", 1440.0, 900.0), ("dark", 1100.0, 650.0)] {
        for name in [
            "gsi", "sdr", "gts7", "nikon", "jobxml", "csv", "polar", "kenar", "poligon",
        ] {
            let mut app = opened();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            if name == "polar" {
                send(&mut app, Event::Transfer);
            }
            if name == "kenar" || name == "poligon" {
                with_traverse_tolerances(&mut app);
                send(&mut app, Event::Fore(Some(4)));
            }
            if name == "poligon" {
                send(&mut app, Event::TransferTraverse);
            }
            if name == "csv" {
                app.calc.fieldbook.mapping = super::Mapping::default();
                send(&mut app, Event::Picked(Some(csv.clone())));
            }
            if name == "sdr" {
                send(&mut app, Event::Picked(Some(sample_named("sample.sdr"))));
            }
            if name == "gts7" {
                send(&mut app, Event::Picked(Some(sample_named("sample.gt7"))));
            }
            if name == "nikon" {
                send(
                    &mut app,
                    Event::Picked(Some(sample_named("sample-nikon.raw"))),
                );
            }
            if name == "jobxml" {
                send(&mut app, Event::Picked(Some(sample_named("sample.jxl"))));
            }
            app.follow.flash = None;
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            if name == "kenar" {
                // The traverse's legs at the body's foot.
                snapshot.operate(app.view(), Box::new(crate::files_testing::SnapAll));
                snapshot.settle(&mut app, App::view, &mut update);
            }
            let file = out.join(format!("karne-{name}-{w}x{h}-{theme}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// Kutupsal alım'a aktar (docs/adr/0169 §3): the station by the file's
/// coordinates (the drawing has no ST1), the first row the back sight, the
/// other targets its shots; with any back point the window's height
/// differences are the field book's.
#[test]
fn a_station_goes_to_kutupsal_alim() {
    let mut app = opened();
    let dh: Vec<(String, f64)> = app
        .calc
        .fieldbook
        .reduction
        .as_ref()
        .expect("reduced")
        .rows
        .iter()
        .skip(1)
        .map(|r| (r.target.clone(), r.dh.expect("a height difference")))
        .collect();
    send(&mut app, Event::Transfer);
    assert_eq!(app.calc.open, Some(Window::Polar));
    let polar = &app.calc.polar;
    assert_eq!(
        (
            polar.station.as_str(),
            polar.back.as_str(),
            polar.instrument_height.as_str()
        ),
        ("412350,4521800", "P2", "1.552")
    );
    assert_eq!(
        (polar.back_reading.as_str(), polar.station_z.as_str()),
        ("0.0019", "105.2")
    );
    let names: Vec<&str> = polar.rows.iter().map(|r| r[0].as_str()).collect();
    assert_eq!(names, ["101", "102", "103", "104", "ST2"]);
    assert_eq!(
        polar.rows[0],
        ["101", "87.4329", "63.215", "101.2337", "1.7"]
    );
    // Any back point: the heights do not depend on the orientation.
    app.calc.polar.back = "412350,4522000".to_owned();
    let doc = app.document.as_ref().expect("a drawing");
    let points = app.calc.polar.compute(&doc.model).points.expect("computed");
    for ((name, want), p) in dh.iter().zip(&points) {
        let got = p.dz.expect("a height difference");
        assert!((got - want).abs() < 1e-6, "{name}: {got} ≠ {want}");
    }
}

/// The two-way tolerance (docs/adr/0169 §3): a leg whose distances from
/// its two ends differ by more is marked and counted under the tables.
#[test]
fn a_legs_two_way_difference_is_checked() {
    let mut app = opened();
    let diff = app.calc.fieldbook.traverse().expect("two stations").legs[0]
        .diff
        .expect("measured from both ends")
        .abs();
    assert!(diff > 0.0, "the sample's leg differs");
    let says = |app: &App| -> Vec<String> {
        let doc = app.document.as_ref().expect("a drawing");
        app.calc
            .fieldbook
            .summary_lines(doc.settings(), doc.settings().angle_unit)
            .into_iter()
            .map(|(_, t)| t)
            .collect()
    };
    for (tolerance, over) in [(diff / 2.0, true), (diff * 2.0, false)] {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.survey = Some(SurveySettings {
            face_slope: Some(0.005),
            two_way: Some(tolerance),
            ..SurveySettings::default()
        });
        doc.model.set_settings(settings);
        send(&mut app, Event::Station(0));
        let t = app.calc.fieldbook.traverse().expect("two stations");
        assert_eq!(t.legs[0].over, over, "{tolerance}");
        let lines = says(&app);
        assert!(
            lines.iter().any(|l| l.contains("kenarın iki yönden farkı")),
            "{lines:?}"
        );
        assert_eq!(
            lines
                .iter()
                .any(|l| l == "1 kenarda iki yönden fark toleransı aşıldı."),
            over,
            "{lines:?}"
        );
    }
}

/// Poligon hesabı'na aktar (docs/adr/0169 §3): the two stations a connected
/// traverse, ST1 oriented on P2, ST2 on P9; each angle and the leg's mean
/// are the core's, the stations by the file's coordinates.
#[test]
fn the_stations_go_to_poligon_hesabi() {
    use crate::calc::traverse::Kind;

    let mut app = opened();
    let t = app.calc.fieldbook.traverse().expect("two stations").clone();
    assert_eq!(t.stations, ["ST1", "ST2"]);
    assert_eq!(t.angles[1], None, "no fore sight chosen yet");
    // P9 is ST2's fifth reduced row.
    send(&mut app, Event::Fore(Some(4)));
    let t = app.calc.fieldbook.traverse().expect("two stations").clone();
    assert!(t.missing.is_empty());
    let leg = t.legs[0].mean.expect("measured from both ends");
    send(&mut app, Event::TransferTraverse);
    assert_eq!(app.calc.open, Some(Window::Traverse));
    let tr = &app.calc.traverse;
    assert_eq!((tr.kind, tr.end_oriented), (Kind::Connected, true));
    assert_eq!(
        (
            tr.start.as_str(),
            tr.back.as_str(),
            tr.end.as_str(),
            tr.fore.as_str()
        ),
        ("412350,4521800", "P2", "412410.512,4521742.208", "P9")
    );
    assert!(tr.rows.is_empty(), "two stations: no new point");
    let angle = |v: Option<f64>| super::exact(v.expect("an angle"), 8);
    assert_eq!(tr.first[1], angle(t.angles[0]));
    assert_eq!(tr.first[2], super::exact(leg, 6));
    assert_eq!(tr.last[1], angle(t.angles[1]));
}
