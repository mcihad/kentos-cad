//! The Hesap windows through the app: what they compute, what they write,
//! the tables and Çizimden (docs/adr/0070, 0071); and their pictures.

use kentos_contracts::Entity;

use super::*;
use crate::files_testing::{app_with_drawing, last_said};

fn calc(app: &mut App, e: Event) {
    let _ = app.update(Message::Calc(e));
}

/// Önden kestirme from A (0, 0) and B (100, 0), 50 g at each: P lies
/// right of A → B, at (50, −50). Worked out by hand.
#[test]
fn forward_intersection_adds_its_point_in_one_step() {
    let mut app = app_with_drawing();
    let _ = app.run("calc.forward");
    assert_eq!(app.dialog, Some(Asking::Calc));
    calc(&mut app, Event::Known(Field::A, "0,0".into()));
    calc(&mut app, Event::Known(Field::B, "100;0".into()));
    calc(&mut app, Event::Alpha("50".into()));
    calc(&mut app, Event::Beta("50,0".into()));
    let doc = app.document.as_ref().expect("open");
    let found = app
        .calc
        .intersection
        .compute(&doc.model)
        .result
        .expect("a point");
    assert!(
        (found.p.x - 50.0).abs() < 1e-9 && (found.p.y + 50.0).abs() < 1e-9,
        "{:?}",
        found.p
    );
    assert_eq!(found.name, "P");
    let count = doc.model.entities().count();
    calc(&mut app, Event::AddPoints);
    assert_eq!(app.dialog, None, "the window closes");
    let doc = app.document.as_mut().expect("open");
    assert_eq!(doc.model.entities().count(), count + 1);
    let Some(Entity::Point(p)) = app.selection.ids().first().and_then(|&s| doc.model.get(s)) else {
        panic!("the new point, selected");
    };
    assert_eq!(p.base.label.as_deref(), Some("P"));
    assert_eq!(
        p.base.attrs.get("Tür").map(String::as_str),
        Some("Kestirme noktası")
    );
    assert_eq!(
        last_said(&app),
        "Önden kestirme: P noktası çizime eklendi (Ctrl+Z geri alır)."
    );
    let doc = app.document.as_mut().expect("open");
    assert_eq!(doc.model.undo().as_deref(), Some("Önden kestirme"));
}

#[test]
fn missing_fields_are_said_in_the_webs_order() {
    let mut app = app_with_drawing();
    let _ = app.run("calc.resection");
    calc(&mut app, Event::Known(Field::B, "Q9".into()));
    calc(&mut app, Event::Alpha("x".into()));
    let doc = app.document.as_ref().expect("open");
    let computed = app.calc.intersection.compute(&doc.model);
    assert_eq!(
        computed.errors,
        [
            "A noktası verilmedi.",
            "B noktası: “Q9” adlı nokta çizimde yok. Adını denetleyin ya da Y,X yazın.",
            "C noktası verilmedi.",
            "α açısını yazın.",
            "β açısını yazın.",
        ]
    );
}

/// Çizimden: the window closes, the pick tool asks, a typed point comes
/// back into the field and the window opens again as it was.
#[test]
fn a_point_shown_on_the_drawing_comes_back_to_its_field() {
    let mut app = app_with_drawing();
    let _ = app.run("calc.stakeout");
    calc(&mut app, Event::Known(Field::Back, "12,34".into()));
    calc(&mut app, Event::Pick(Field::Station));
    assert_eq!(app.dialog, None);
    assert_eq!(
        app.session.prompt().text(),
        "Aplikasyon: Durulan nokta (istasyon): haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]"
    );
    let _ = app.submit_line("486513.341,4420189.522");
    assert!(!app.session.is_running());
    assert_eq!(app.dialog, Some(Asking::Calc));
    // The demo drawing's point P1 lies exactly there: its name is taken.
    assert_eq!(app.calc.stakeout.station, "P1");
    assert_eq!(app.calc.stakeout.back, "12,34", "the rest as it was");
    // Esc: nothing picked, the window opens again.
    calc(&mut app, Event::Pick(Field::Back));
    app.cancel();
    assert_eq!(app.dialog, Some(Asking::Calc));
    assert_eq!(app.calc.stakeout.back, "12,34");
}

/// From (0, 0) oriented on (0, 100): the point (100, 0) is 100 g east,
/// 100 m away, 100 g clockwise from the back point.
#[test]
fn stakeout_values_and_a_target_at_the_station() {
    let mut app = app_with_drawing();
    let _ = app.run("calc.stakeout");
    calc(&mut app, Event::Known(Field::Station, "0,0".into()));
    calc(&mut app, Event::Known(Field::Back, "0,100".into()));
    calc(&mut app, Event::Cell(0, 0, "100,0".into()));
    let doc = app.document.as_ref().expect("open");
    let read = app.calc.stakeout.compute(&doc.model);
    let stakes = read.stakes.expect("values");
    assert!((stakes[0].bearing - 100.0).abs() < 1e-9);
    assert!((stakes[0].distance - 100.0).abs() < 1e-9);
    assert!(stakes[0].angle.is_some_and(|a| (a - 100.0).abs() < 1e-9));
    assert_eq!(read.names, ["100,0"]);
    calc(&mut app, Event::Cell(1, 0, "0,0".into()));
    let doc = app.document.as_ref().expect("open");
    assert_eq!(
        app.calc.stakeout.compute(&doc.model).errors,
        ["2. satırdaki nokta durulan noktayla aynı yerde; semt tanımsız."]
    );
}

/// A closed traverse round the square (0, 0), (100, 0), (100, 100),
/// (0, 100), oriented on (−100, 0), every leg 100 m; the angles clockwise
/// from the point before to the one after: 200 g at the start, 100 g at
/// the corners, 300 g back at the start. The angle at P2 is 40 cc too
/// big: fβ = 0.004 g, each of the five angles takes −0.0008 g, and the
/// adjusted legs close on the start. Worked out by hand.
#[test]
fn a_closed_traverse_closes_and_adds_its_points_in_one_step() {
    use traverse::{ANGLE, DISTANCE, NAME};
    let mut app = app_with_drawing();
    let _ = app.run("calc.traverse");
    calc(&mut app, Event::TraverseKind(traverse::Kind::Closed));
    calc(&mut app, Event::Known(Field::Start, "0,0".into()));
    calc(&mut app, Event::Known(Field::Back, "-100,0".into()));
    // The start's row, then the corners: pasted as a spreadsheet gives them.
    calc(&mut app, Event::Cell(0, ANGLE, "200".into()));
    calc(&mut app, Event::Cell(0, DISTANCE, "100".into()));
    calc(
        &mut app,
        Event::Pasted(
            1,
            NAME,
            Some("P1\t100\t100\r\nP2\t100,0040\t100\nP3\t100\t100\n".into()),
        ),
    );
    let form = &app.calc.traverse;
    assert_eq!(form.rows.len(), 3, "a row added before the end station");
    assert_eq!(form.rows[1], ["P2", "100,0040", "100"].map(String::from));
    // The end station: the start again, its angle typed, no leg.
    assert_eq!(form.last[NAME], "A");
    assert!(form.readonly(4, NAME) && form.readonly(4, DISTANCE));
    assert!(!form.readonly(4, ANGLE));
    calc(&mut app, Event::Cell(4, ANGLE, "300".into()));
    let doc = app.document.as_ref().expect("open");
    let c = app.calc.traverse.compute(&doc.model);
    assert!(c.errors.is_empty(), "{:?}", c.errors);
    let r = c.result.expect("a result");
    assert!(r.angle_misclosure.is_some_and(|f| (f - 0.004).abs() < 1e-9));
    assert!(
        r.angle_correction
            .is_some_and(|v| (v + 0.0008).abs() < 1e-9)
    );
    let (sy, sx) = r
        .legs
        .iter()
        .fold((0.0, 0.0), |(y, x), l| (y + l.dy, x + l.dx));
    assert!(sy.abs() < 1e-9 && sx.abs() < 1e-9, "closes: {sy} {sx}");
    for (p, (x, y)) in r
        .points
        .iter()
        .zip([(100.0, 0.0), (100.0, 100.0), (0.0, 100.0)])
    {
        assert!((p.x - x).abs() < 0.01 && (p.y - y).abs() < 0.01, "{p:?}");
    }
    assert_eq!(c.names, ["P1", "P2", "P3"]);
    let format = Format::of(doc.settings());
    let report = app
        .calc
        .traverse
        .report(&doc.model, &format)
        .expect("a report");
    assert_eq!(
        report[report.len() - 2][..2],
        [
            "Açı kapanma hatası".to_owned(),
            "0.00400 g (40.0 cc)".to_owned()
        ]
    );
    let count = doc.model.entities().count();
    calc(&mut app, Event::AddPoints);
    assert_eq!(app.dialog, None);
    assert_eq!(
        last_said(&app),
        "Poligon hesabı: 3 poligon noktası çizime eklendi (Ctrl+Z geri alır)."
    );
    let doc = app.document.as_mut().expect("open");
    assert_eq!(doc.model.entities().count(), count + 3);
    assert_eq!(app.selection.ids().len(), 3);
    assert_eq!(doc.model.undo().as_deref(), Some("Poligon hesabı"));
}

/// An open traverse: its last new point has neither an angle nor a leg;
/// without new points it says so.
#[test]
fn an_open_traverse_ends_on_its_last_new_point() {
    use traverse::{ANGLE, DISTANCE, NAME};
    let mut app = app_with_drawing();
    let _ = app.run("calc.traverse");
    calc(&mut app, Event::TraverseKind(traverse::Kind::Open));
    calc(&mut app, Event::Known(Field::Start, "P1".into()));
    calc(&mut app, Event::Known(Field::Back, "0,0".into()));
    let form = &app.calc.traverse;
    assert_eq!(form.first[NAME], "P1", "the start's row takes its name");
    assert_eq!(Table::rows(form), 3, "no end station");
    assert!(form.readonly(2, ANGLE) && form.readonly(2, DISTANCE));
    assert!(!form.readonly(2, NAME));
    let doc = app.document.as_ref().expect("open");
    assert_eq!(
        app.calc.traverse.compute(&doc.model).errors,
        [
            "Başlangıç satırında kırılma açısı yok.",
            "Başlangıç satırında kenar uzunluğu yok.",
            "Açık poligonda en az bir yeni nokta olmalı.",
        ]
    );
    // Enter under the last row adds one; it is the new last point.
    let _ = app.update(Message::Calc(Event::Submit(2, NAME)));
    assert_eq!(Table::rows(&app.calc.traverse), 4);
    calc(&mut app, Event::RemoveRow(3));
    calc(&mut app, Event::RemoveRow(0));
    assert_eq!(Table::rows(&app.calc.traverse), 3, "the start stays");
}

/// Kutupsal alım adds its points with their heights, as one step.
#[test]
fn polar_points_come_with_their_heights() {
    use polar::{DISTANCE, NAME, READING, ZENITH};
    let mut app = app_with_drawing();
    let _ = app.run("calc.polar");
    calc(&mut app, Event::Known(Field::Station, "0,0".into()));
    calc(&mut app, Event::Known(Field::Back, "0,100".into()));
    calc(&mut app, Event::StationZ("100".into()));
    calc(&mut app, Event::Cell(0, NAME, "K1".into()));
    calc(&mut app, Event::Cell(0, READING, "100".into()));
    calc(&mut app, Event::Cell(0, DISTANCE, "10".into()));
    calc(&mut app, Event::Cell(0, ZENITH, "100".into()));
    let doc = app.document.as_ref().expect("open");
    let points = app.calc.polar.points(&doc.model);
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].name, "K1");
    assert!((points[0].p.x - 10.0).abs() < 1e-9 && points[0].p.y.abs() < 1e-9);
    assert!(points[0].z.is_some_and(|z| (z - 100.0).abs() < 1e-9));
    calc(&mut app, Event::AddPoints);
    assert_eq!(
        last_said(&app),
        "Kutupsal alım: 1 nokta çizime eklendi (Ctrl+Z geri alır)."
    );
    let doc = app.document.as_mut().expect("open");
    let Some(Entity::Point(p)) = app.selection.ids().first().and_then(|&s| doc.model.get(s)) else {
        panic!("the new point, selected");
    };
    assert!(p.z.is_some_and(|z| (z - 100.0).abs() < 1e-9));
    assert_eq!(
        p.base.attrs.get("Z (m)").map(String::as_str),
        Some("100.000")
    );
    assert_eq!(
        p.base.attrs.get("Tür").map(String::as_str),
        Some("Alım noktası")
    );
}

/// The layer the windows write to, chosen as the web's `layerChoice`
/// does on opening: kept while the drawing has it.
#[test]
fn the_layer_is_chosen_on_opening_and_kept() {
    let mut app = app_with_drawing();
    let _ = app.run("calc.polar");
    let doc = app.document.as_ref().expect("open");
    let active = doc.model.layers().active().to_owned();
    assert_eq!(app.calc.polar.layer.as_deref(), Some(active.as_str()));
    let other = leaves(&doc.model)
        .into_iter()
        .position(|(id, _, locked)| id != active && !locked)
        .expect("another layer");
    calc(&mut app, Event::Layer(other));
    let chosen = app.calc.polar.layer.clone();
    assert_ne!(chosen.as_deref(), Some(active.as_str()));
    calc(&mut app, Event::Close);
    let _ = app.run("calc.polar");
    assert_eq!(app.calc.polar.layer, chosen);
}

/// The windows filled in, for the owner. Not run by default:
/// `cargo test -p kentos-desktop calc::tests::screens -- --ignored --nocapture`
/// (`KENTOS_SHOTS=hesap-poligon,…` for some of them).
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS").unwrap_or_default();
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "hesap-poligon",
                "hesap-poligon-uyari",
                "hesap-kutupsal",
                "hesap-onden",
                "hesap-geriden",
                "hesap-aplikasyon",
            ] {
                if !only.is_empty() && !only.split(',').any(|o| o == name) {
                    continue;
                }
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let calc = |app: &mut App, e: Event| {
                    let _ = app.update(Message::Calc(e));
                };
                match name {
                    "hesap-poligon" => {
                        use traverse::{ANGLE, DISTANCE};
                        // A connected traverse worked out from the points'
                        // coordinates, with 9 cc and 4 mm of error in it.
                        let _ = app.run("calc.traverse");
                        calc(&mut app, Event::Known(Field::Start, "P1".into()));
                        calc(
                            &mut app,
                            Event::Known(Field::Back, "486535.757,4420188.723".into()),
                        );
                        calc(
                            &mut app,
                            Event::Known(Field::End, "486538.221,4420218.986".into()),
                        );
                        calc(
                            &mut app,
                            Event::Known(Field::Fore, "486514.344,4420220.532".into()),
                        );
                        calc(&mut app, Event::Cell(0, ANGLE, "330,3870".into()));
                        calc(&mut app, Event::Cell(0, DISTANCE, "13.304".into()));
                        calc(
                            &mut app,
                            Event::Pasted(
                                1,
                                0,
                                Some("Y1\t224,5472\t10.905\nY2\t188.9591\t14.809\n".into()),
                            ),
                        );
                        calc(&mut app, Event::Cell(3, ANGLE, "57.9557".into()));
                    }
                    "hesap-poligon-uyari" => {
                        let _ = app.run("calc.traverse");
                        calc(&mut app, Event::TraverseKind(traverse::Kind::Open));
                        calc(&mut app, Event::Known(Field::Start, "P1".into()));
                        calc(&mut app, Event::Cell(0, 1, "12x".into()));
                    }
                    "hesap-kutupsal" => {
                        let _ = app.run("calc.polar");
                        calc(&mut app, Event::Known(Field::Station, "P1".into()));
                        calc(
                            &mut app,
                            Event::Known(Field::Back, "486535.757,4420188.723".into()),
                        );
                        calc(&mut app, Event::StationZ("812,40".into()));
                        calc(&mut app, Event::InstrumentHeight("1.55".into()));
                        calc(
                            &mut app,
                            Event::Pasted(
                                0,
                                0,
                                Some(
                                    "K1\t327.7849\t17.727\t98.4410\t1.70\nK2\t372,1872\t18.225\t\t\nK3\t288.3598\t25.661\t101.2215\t1.70\n"
                                        .into(),
                                ),
                            ),
                        );
                    }
                    "hesap-onden" => {
                        let _ = app.run("calc.forward");
                        calc(&mut app, Event::Known(Field::A, "P1".into()));
                        calc(
                            &mut app,
                            Event::Known(Field::B, "486535.757,4420188.723".into()),
                        );
                        calc(&mut app, Event::Alpha("62,5".into()));
                        calc(&mut app, Event::Beta("58.25".into()));
                        calc(&mut app, Event::Name("Y1".into()));
                    }
                    "hesap-geriden" => {
                        let _ = app.run("calc.resection");
                        calc(&mut app, Event::Known(Field::A, "P1".into()));
                        calc(
                            &mut app,
                            Event::Known(Field::B, "486535.757,4420188.723".into()),
                        );
                        calc(&mut app, Event::Alpha("42".into()));
                    }
                    _ => {
                        let _ = app.run("calc.stakeout");
                        calc(&mut app, Event::Known(Field::Station, "P1".into()));
                        calc(
                            &mut app,
                            Event::Known(Field::Back, "486535.757,4420188.723".into()),
                        );
                        calc(&mut app, Event::Cell(0, 0, "486538.221,4420218.986".into()));
                        calc(&mut app, Event::Cell(1, 0, "486514.344 4420220.532".into()));
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
