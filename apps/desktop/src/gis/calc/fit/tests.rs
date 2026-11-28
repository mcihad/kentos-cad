//! Vektör oturtma's tests: the form's rules, then the window through the
//! app over `fixtures/interaction/v1/vector-fit.kcad` (docs/adr/0156 §7): a
//! parcel surveyed in a local system (layer `yerel`) and the same six points
//! measured in TUREF (layer `tm`), P5 with a 15 cm blunder. The steps of the
//! web's `vectorfit` scenes (shots.mjs), and their pictures.

use crate::gis;
use kentos_interaction::Format;

use super::*;
use crate::app::{App, Dialog as Asking};
use crate::calc::Field;
use crate::exchange::words::Kind as Line;
use crate::files_testing::last_said;

fn row(cells: [&str; 6]) -> [String; 9] {
    let mut out: [String; 9] = Default::default();
    for (i, c) in cells.into_iter().enumerate() {
        out[i] = c.to_owned();
    }
    out
}

/// The residuals come into their cells; a pair left out still has its
/// residual, faded, and the worst used pair is marked.
#[test]
fn the_table_is_solved_at_every_change() {
    let mut form = Form {
        rows: vec![
            row(["", "A", "0", "0", "100", "200"]),
            row(["", "B", "10", "0", "110", "200"]),
            row(["", "C", "0", "10", "100", "210.003"]),
            row(["0", "D", "10", "10", "110", "210.5"]),
            row(["", "", "", "", "", ""]),
        ],
        ..Form::default()
    };
    form.solve();
    let fit = form.fit().expect("solved");
    assert_eq!(form.used, 3);
    assert_eq!(fit.residuals.len(), 4, "the pair left out has its residual");
    assert!(form.rows.iter().take(4).all(|r| !r[RES].is_empty()));
    assert!(form.rows[4][RES].is_empty(), "an empty row has none");
    assert_eq!(form.mark(3), Some(Mark::Off));
    let worst = (0..3).find(|&r| form.mark(r) == Some(Mark::Worst));
    assert!(worst.is_some(), "one used pair is the worst");
    // Too few used pairs: no solution, nothing in the cells.
    form.rows[1][USE] = "0".to_owned();
    form.rows[2][USE] = "0".to_owned();
    form.solve();
    assert!(matches!(form.solution, Some(Err(FitError::TooFew(_)))));
    assert!(form.rows.iter().all(|r| r[RES].is_empty()));
    assert_eq!(form.mark(0), None);
}

#[test]
fn the_solution_goes_to_the_command_in_its_centred_form() {
    let mut form = Form {
        rows: vec![
            row(["", "A", "0", "0", "100", "200"]),
            row(["", "B", "10", "0", "110", "200"]),
            row(["", "C", "0", "10", "100", "210"]),
            row(["", "D", "10", "10", "110", "210"]),
        ],
        ..Form::default()
    };
    for kind in [Kind::Helmert, Kind::Affine, Kind::Projective] {
        form.kind = kind;
        form.solve();
        let fit = form.fit().expect("solved");
        let t = transform_of(fit).expect("its numbers");
        let ok = match (kind, &t) {
            (Kind::Helmert, Transform::Similarity { a, b, .. }) => {
                (a - 1.0).abs() < 1e-12 && b.abs() < 1e-12
            }
            (Kind::Affine, Transform::Affine { m, .. }) => {
                (m[0] - 1.0).abs() < 1e-12 && (m[3] - 1.0).abs() < 1e-12
            }
            (Kind::Projective, Transform::Projective { h, .. }) => {
                h[6].abs() < 1e-12 && h[7].abs() < 1e-12
            }
            _ => false,
        };
        assert!(ok, "{kind:?}: {t:?}");
    }
}

#[test]
fn the_report_lists_the_pairs_and_the_solution() {
    let mut form = Form {
        rows: vec![
            row(["", "A", "0", "0", "100", "200"]),
            row(["0", "B", "10", "0", "110", "200"]),
            row(["", "C", "0", "10", "100", "210"]),
            row(["", "", "1", "", "", ""]),
        ],
        ..Form::default()
    };
    form.solve();
    let lines = form.points_report(&Format::default());
    assert_eq!(lines[0], ["Vektör oturtma", "Helmert"]);
    assert_eq!(lines[1].len(), 9);
    assert_eq!(lines[3][0], "hayır");
    assert_eq!(lines[3][1], "B");
    assert_eq!(lines.len(), 2 + 3 + 5, "three pairs, then the solution");
    // Two used pairs pass exactly: no m0.
    assert_eq!(lines[5], ["m0 (mm)", "—"]);
}

fn app_with_fit_drawing() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../../fixtures/interaction/v1/vector-fit.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn calc(app: &mut App, e: Calc) {
    let _ = app.update(Message::Gis(gis::Event::Calc(e)));
}

fn fit(app: &mut App, e: Event) {
    calc(app, Calc::Fit(e));
}

/// The scene's start (the web's `matchFit`): Helmert, Adla eşle from the
/// local points to the TUREF ones, Uygula on the local layer.
fn matched() -> App {
    let mut app = app_with_fit_drawing();
    let _ = app.run("transform.fit");
    assert_eq!(app.dialog, Some(Asking::Gis(gis::GisDialogKind::Calc)));
    fit(&mut app, Event::Kind(Kind::Helmert));
    fit(&mut app, Event::Source("yerel".into()));
    fit(&mut app, Event::Target("tm".into()));
    fit(&mut app, Event::Match);
    fit(&mut app, Event::Scope(Scope::Layer));
    fit(&mut app, Event::Layer("yerel".into()));
    app
}

/// A named point's place on a layer.
fn named(app: &App, layer: &str, name: &str) -> kentos_commands::Vec2 {
    let doc = app.document.as_ref().expect("open");
    doc.model
        .by_layer(layer)
        .find_map(|e| match e {
            Entity::Point(p) if p.base.label.as_deref() == Some(name) => Some(p.p),
            _ => None,
        })
        .expect("the point")
}

fn gap(app: &App, name: &str) -> f64 {
    let (a, b) = (named(app, "yerel", name), named(app, "tm", name));
    (a.x - b.x).hypot(a.y - b.y)
}

/// The summary's first line's m0 in millimetres.
fn m0_mm(app: &App) -> f64 {
    app.gis.calc.fit.fit().and_then(|f| f.m0).expect("an m0") * 1000.0
}

/// On opening, the layers holding named points are offered (the drawing's
/// own layer has none); Adla eşle makes the six pairs, P5 the worst.
#[test]
fn adla_esle_pairs_the_named_points_and_p5_reads_worst() {
    let mut app = app_with_fit_drawing();
    let _ = app.run("transform.fit");
    let form = &app.gis.calc.fit;
    assert_eq!(form.source.as_deref(), Some("yerel"));
    assert_eq!(form.target.as_deref(), Some("tm"));
    // GIS-038: nothing selected, Uygula takes the source points' layer, never the target's.
    assert_eq!(form.scope, Scope::Layer, "nothing is selected");
    assert_eq!(form.layer.as_deref(), Some("yerel"), "the source's layer");
    assert!(form.fit().is_none(), "four empty rows");
    let app = matched();
    assert_eq!(last_said(&app), "Vektör oturtma: 6 çift adla eşlendi.");
    let form = &app.gis.calc.fit;
    let names: Vec<&str> = form.rows.iter().map(|r| r[NAME].as_str()).collect();
    assert_eq!(names, ["P1", "P2", "P3", "P4", "P5", "P6"]);
    assert_eq!(form.rows[1][SOURCE_Y], "1062.5");
    assert_eq!(form.rows[1][SOURCE_X], "2003.25");
    assert_eq!(form.rows[0][USE], "1");
    assert_eq!(form.mark(4), Some(Mark::Worst));
    assert!(m0_mm(&app) > 20.0, "the blunder spreads: {}", m0_mm(&app));
}

/// Kullan off for P5: the solution comes again without it; its row fades
/// and keeps its residual (still the blunder's size).
#[test]
fn leaving_p5_out_brings_m0_down_to_millimetres() {
    let mut app = matched();
    calc(&mut app, Calc::Cell(4, USE, "0".into()));
    let form = &app.gis.calc.fit;
    assert_eq!(form.mark(4), Some(Mark::Off));
    assert!(m0_mm(&app) < 5.0, "{}", m0_mm(&app));
    let v5: f64 = form.rows[4][RES].parse().expect("a residual");
    assert!(v5 > 100.0, "{v5}");
    assert!((0..6).any(|r| form.mark(r) == Some(Mark::Worst)));
    // Back on: the blunder again.
    calc(&mut app, Calc::Cell(4, USE, "1".into()));
    assert!(m0_mm(&app) > 20.0);
}

/// Uygula with P5 left out: the local layer lands on the TUREF points in
/// one step (Oturt), the window closes; the circle stays a circle.
#[test]
fn uygula_fits_the_local_layer_in_one_step() {
    let mut app = matched();
    calc(&mut app, Calc::Cell(4, USE, "0".into()));
    let before = gap(&app, "P1");
    assert!(before > 1000.0);
    fit(&mut app, Event::Apply);
    assert_eq!(app.dialog, None, "the window closes");
    assert!(gap(&app, "P1") < 0.01, "{}", gap(&app, "P1"));
    assert!(
        gap(&app, "P5") > 0.1,
        "the blunder stays where it was measured"
    );
    let said = last_said(&app);
    assert!(
        said.starts_with("Vektör oturtma: 10 nesne Helmert dönüşümle oturtuldu (m0 ±")
            && said.ends_with(&crate::keys::shown(" mm). Ctrl+Z geri alır.")),
        "{said}"
    );
    let doc = app.document.as_mut().expect("open");
    assert!(
        doc.model
            .by_layer("yerel")
            .any(|e| matches!(e, Entity::Circle(_)))
    );
    assert_eq!(doc.model.undo().as_deref(), Some("Oturt"));
    assert!(gap(&app, "P1") > 1000.0, "undone in one step");
}

/// Kopya: the copies land on the TUREF points and are selected; the local
/// layer stays.
#[test]
fn copies_are_fitted_and_selected() {
    let mut app = matched();
    calc(&mut app, Calc::Cell(4, USE, "0".into()));
    fit(&mut app, Event::Copy(true));
    let count = app.document.as_ref().expect("open").model.len();
    fit(&mut app, Event::Apply);
    assert!(gap(&app, "P1") > 1000.0, "the originals stay");
    let doc = app.document.as_ref().expect("open");
    assert_eq!(doc.model.len(), count + 10);
    assert_eq!(app.selection.len(), 10);
    let copied_p1 = app
        .selection
        .ids()
        .iter()
        .find_map(|&s| match doc.model.get(s) {
            Some(Entity::Point(p)) if p.base.label.as_deref() == Some("P1") => Some(p.p),
            _ => None,
        });
    let tm = named(&app, "tm", "P1");
    let p = copied_p1.expect("P1's copy, selected");
    assert!((p.x - tm.x).hypot(p.y - tm.y) < 0.01);
    assert!(last_said(&app).starts_with("Vektör oturtma: 10 nesnenin kopyası Helmert"));
}

/// A refusal stays in the window: the local layer locked.
#[test]
fn a_locked_layer_is_said_in_the_window() {
    let mut app = matched();
    app.document
        .as_mut()
        .expect("open")
        .model
        .toggle_layer_locked("yerel");
    fit(&mut app, Event::Apply);
    assert_eq!(
        app.dialog,
        Some(Asking::Gis(gis::GisDialogKind::Calc)),
        "the window stays"
    );
    let status = app.gis.calc.fit.status.clone().expect("the reason");
    assert!(status.contains("kilitli"), "{status}");
    assert!(gap(&app, "P1") > 1000.0, "nothing written");
}

/// A row's source shown on the drawing: the window closes, the pick tool
/// asks, the point and the name of the point it lies on come back.
#[test]
fn a_rows_point_shown_on_the_drawing_comes_back_with_its_name() {
    let mut app = app_with_fit_drawing();
    let _ = app.run("transform.fit");
    fit(&mut app, Event::Pick(0, Side::Source));
    assert_eq!(app.dialog, None);
    assert_eq!(
        app.session.prompt().text(),
        "Vektör oturtma: 1. çiftin kaynağı: haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]"
    );
    let _ = app.submit_line("1062.5,2003.25");
    assert_eq!(app.dialog, Some(Asking::Gis(gis::GisDialogKind::Calc)));
    let row = &app.gis.calc.fit.rows[0];
    assert_eq!(
        (row[SOURCE_Y].as_str(), row[SOURCE_X].as_str()),
        ("1062.5", "2003.25")
    );
    assert_eq!(row[NAME], "P2", "the point it lies on");
    // The target of the same row: the name stays.
    fit(&mut app, Event::Pick(0, Side::Target));
    let _ = app.submit_line("487177.949,4420235.009");
    let row = &app.gis.calc.fit.rows[0];
    assert_eq!(row[NAME], "P2");
    assert_eq!(row[TARGET_Y], "487177.949");
    // Esc: nothing changes, the window opens again.
    fit(&mut app, Event::Pick(1, Side::Source));
    app.cancel();
    assert_eq!(app.dialog, Some(Asking::Gis(gis::GisDialogKind::Calc)));
    assert!(app.gis.calc.fit.rows[1][SOURCE_Y].is_empty());
}

/// Pasted lines fill the pairs; nothing goes into Kullan.
#[test]
fn pasted_lines_fill_the_pairs_past_kullan() {
    let mut app = app_with_fit_drawing();
    let _ = app.run("transform.fit");
    calc(
        &mut app,
        Calc::Pasted(
            0,
            USE,
            Some("0\tA\t0\t0\t100\t200\n0\tB\t10\t0\t110\t200\n".into()),
        ),
    );
    let form = &app.gis.calc.fit;
    assert_eq!(form.rows[0][USE], "", "Kullan takes nothing pasted");
    assert_eq!(form.rows[0][NAME], "A");
    assert_eq!(form.rows[1][TARGET_X], "200");
    assert!(form.fit().is_some(), "two pairs: Helmert");
}

/// Parametrelerle from the matched scene: the base and the numbers typed.
fn with_params(base: &str, numbers: &[(Param, &str)]) -> App {
    let mut app = matched();
    fit(&mut app, Event::Method(Method::Parameters));
    calc(&mut app, Calc::Known(Field::Base, base.into()));
    for (p, text) in numbers {
        fit(&mut app, Event::Param(*p, (*text).into()));
    }
    app
}

/// The summary's words now.
fn said_now(app: &App) -> Vec<String> {
    let doc = app.document.as_ref().expect("open");
    let format = Format::of(doc.settings());
    app.gis
        .calc
        .fit
        .summary_lines(&doc.model, &format)
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

/// Whether the log has said `text`.
fn logged(app: &App, text: &str) -> bool {
    app.log.lines().any(|l| l.text == text)
}

/// Parametrelerle: what is wrong is said in the fields' order, and nothing
/// is written while it is.
#[test]
fn parametrelerle_says_what_is_wrong_in_the_fields_order() {
    let mut app = matched();
    fit(&mut app, Event::Method(Method::Parameters));
    assert_eq!(
        said_now(&app),
        ["Taban noktasını yazın ya da çizimden seçin."]
    );
    let mut app = with_params(
        "P9",
        &[
            (Param::ScaleY, "0"),
            (Param::ScaleX, "x"),
            (Param::Rotation, "1e400"),
        ],
    );
    assert_eq!(
        said_now(&app),
        [
            "Taban noktası: “P9” adlı nokta çizimde yok. Adını denetleyin ya da Y,X yazın.",
            "Y ölçeği sıfır olamaz; eksi ölçek aynalar.",
            "X ölçeği sayı olmalı.",
            "Dönüklük sayı olmalı.",
        ]
    );
    fit(&mut app, Event::Apply);
    assert_eq!(
        app.dialog,
        Some(Asking::Gis(gis::GisDialogKind::Calc)),
        "the window stays"
    );
    assert!(gap(&app, "P1") > 1000.0, "nothing written");
}

/// About P1, east doubled: the local layer stretches in one step (Oturt),
/// the base stays, the circle becomes an ellipse and the log says the
/// curves and the shapes kept.
#[test]
fn parametrelerle_stretches_the_local_layer_about_its_base() {
    let mut app = with_params("P1", &[(Param::ScaleY, "2")]);
    assert_eq!(
        said_now(&app),
        [
            "Y ölçeği 2, X ölçeği 1, dönüklük 0.0000 g, öteleme ΔY 0.000 m, ΔX 0.000 m.",
            "Taban noktası Y 1000.000  X 2000.000 → Y 1000.000  X 2000.000.",
            "Ölçekler farklı: afin dönüşüm; daireler ve yaylar elips olur, yazılar, bloklar ve ölçüler yerinde biçimini korur.",
        ]
    );
    fit(&mut app, Event::Apply);
    assert_eq!(app.dialog, None, "the window closes");
    let close =
        |a: kentos_commands::Vec2, x: f64, y: f64| (a.x - x).abs() < 1e-9 && (a.y - y).abs() < 1e-9;
    assert!(
        close(named(&app, "yerel", "P1"), 1000.0, 2000.0),
        "the base stays"
    );
    assert!(close(named(&app, "yerel", "P2"), 1125.0, 2003.25));
    assert!(close(named(&app, "yerel", "P4"), 996.5, 2038.0));
    assert!(logged(
        &app,
        &crate::keys::shown("Vektör oturtma: 10 nesne parametrelerle oturtuldu. Ctrl+Z geri alır.")
    ));
    let doc = app.document.as_mut().expect("open");
    assert!(
        doc.model
            .by_layer("yerel")
            .any(|e| matches!(e, Entity::Ellipse(_)))
            && !doc
                .model
                .by_layer("yerel")
                .any(|e| matches!(e, Entity::Circle(_))),
        "the circle is an ellipse now"
    );
    assert_eq!(doc.model.undo().as_deref(), Some("Oturt"));
    assert!(close(named(&app, "yerel", "P2"), 1062.5, 2003.25), "undone");
}

/// Equal scales, a quarter turn (100 g) and a shift: a similarity; every
/// kind moves exactly, the circle stays a circle, nothing is warned.
#[test]
fn equal_scales_a_turn_and_a_shift_keep_every_shape() {
    let mut app = with_params(
        "1000,2000",
        &[
            (Param::Rotation, "100"),
            (Param::ShiftY, "10"),
            (Param::ShiftX, "20"),
        ],
    );
    assert_eq!(
        said_now(&app)[2],
        "Ölçekler eşit: benzerlik; her nesne biçimini korur."
    );
    fit(&mut app, Event::Apply);
    assert_eq!(
        last_said(&app),
        crate::keys::shown("Vektör oturtma: 10 nesne parametrelerle oturtuldu. Ctrl+Z geri alır.")
    );
    let p2 = named(&app, "yerel", "P2");
    assert!(
        (p2.x - 1006.75).abs() < 1e-9 && (p2.y - 2082.5).abs() < 1e-9,
        "{p2:?}"
    );
    let doc = app.document.as_ref().expect("open");
    assert!(
        doc.model
            .by_layer("yerel")
            .any(|e| matches!(e, Entity::Circle(_)))
    );
}

/// The base point shown on the drawing takes the name of the point it lies
/// on; the window opens again on Parametrelerle.
#[test]
fn the_base_point_is_shown_on_the_drawing() {
    let mut app = app_with_fit_drawing();
    let _ = app.run("transform.fit");
    fit(&mut app, Event::Method(Method::Parameters));
    calc(&mut app, Calc::Pick(Field::Base));
    assert_eq!(app.dialog, None);
    assert_eq!(
        app.session.prompt().text(),
        "Vektör oturtma: Taban noktası: haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]"
    );
    let _ = app.submit_line("1062.5,2003.25");
    assert_eq!(app.dialog, Some(Asking::Gis(gis::GisDialogKind::Calc)));
    assert_eq!(app.gis.calc.fit.params.base, "P2");
    assert_eq!(app.gis.calc.fit.method, Method::Parameters);
}

/// Parametrelerle's report: the base, the numbers as read and the linear
/// part; none while a number is wrong.
#[test]
fn parametrelerle_reports_its_numbers() {
    let mut app = with_params("P1", &[(Param::ScaleY, "1,5"), (Param::Rotation, "50")]);
    let report = |app: &App| {
        let doc = app.document.as_ref().expect("open");
        app.gis
            .calc
            .fit
            .report(&doc.model, &Format::of(doc.settings()))
    };
    let lines = report(&app).expect("a report");
    assert_eq!(lines[0], ["Vektör oturtma", "Parametrelerle"]);
    assert_eq!(lines[1], ["Taban Y", "1000", "Taban X", "2000"]);
    assert_eq!(lines[2], ["Y ölçeği", "1.5"]);
    assert_eq!(lines[3], ["X ölçeği", "1"]);
    assert_eq!(lines[4], ["Dönüklük (g)", "50"]);
    assert_eq!(lines[5], ["Öteleme ΔY (m)", "0"]);
    assert_eq!(lines[7].len(), 5, "the four numbers");
    fit(&mut app, Event::Param(Param::ScaleX, "0".into()));
    assert_eq!(report(&app), None);
}

/// Kauçuk levha over the matched pairs: every used pair a link the sheet
/// meets, Helmert's residuals in the cells (the local corrections), P5's
/// the largest; Sabit on every row.
#[test]
fn kaucuk_levha_takes_the_used_pairs_as_links() {
    let mut app = matched();
    let helmert: Vec<String> = app
        .gis
        .calc
        .fit
        .rows
        .iter()
        .map(|r| r[RES].clone())
        .collect();
    fit(&mut app, Event::Kind(Kind::Rubber));
    let form = &app.gis.calc.fit;
    let cells: Vec<String> = form.rows.iter().map(|r| r[RES].clone()).collect();
    assert_eq!(cells, helmert, "the residuals are Helmert's");
    assert_eq!(form.mark(4), Some(Mark::Worst));
    let doc = app.document.as_ref().expect("open");
    let lines = form.summary_lines(&doc.model, &Format::default());
    assert_eq!(
        lines[0].1,
        "6 bağ (0 sabit nokta); levha her bağdan tam geçer."
    );
    assert!(
        lines[1]
            .1
            .starts_with("Yerel düzeltme (Helmert'e göre) en çok ")
            && lines[1].1.contains(" mm: P5; ortalama ")
            && lines[1].1.contains(", Helmert m0 = ±"),
        "{}",
        lines[1].1
    );
    let plan = form.plan(&doc.model, &Format::default()).expect("a sheet");
    let Transform::Rubbersheet { links } = plan.transform else {
        panic!("{:?}", plan.transform)
    };
    assert_eq!(links.len(), 6);
    assert_eq!(plan.how, "kauçuk levhayla");
    // P5 left out: five links.
    calc(&mut app, Calc::Cell(4, USE, "0".into()));
    let doc = app.document.as_ref().expect("open");
    let lines = app
        .gis
        .calc
        .fit
        .summary_lines(&doc.model, &Format::default());
    assert!(
        lines[0].1.starts_with("5 bağ (0 sabit nokta)"),
        "{}",
        lines[0].1
    );
}

/// Sabit writes the row's source into its target; a row without its
/// source keeps its target and says why.
#[test]
fn sabit_makes_a_row_a_fixed_point() {
    let mut app = matched();
    fit(&mut app, Event::Kind(Kind::Rubber));
    fit(&mut app, Event::Fix(2));
    let form = &app.gis.calc.fit;
    assert_eq!(form.rows[2][TARGET_Y], form.rows[2][SOURCE_Y]);
    assert_eq!(form.rows[2][TARGET_X], form.rows[2][SOURCE_X]);
    assert_eq!(form.status, None);
    let doc = app.document.as_ref().expect("open");
    let lines = form.summary_lines(&doc.model, &Format::default());
    assert!(
        lines[0].1.starts_with("6 bağ (1 sabit nokta)"),
        "{}",
        lines[0].1
    );
    // A new row has no source: its target stays.
    app.gis
        .calc
        .fit
        .rows
        .push(row(["1", "Q", "", "", "487100", "4420200"]));
    fit(&mut app, Event::Fix(6));
    let form = &app.gis.calc.fit;
    assert_eq!(form.rows[6][TARGET_Y], "487100");
    assert_eq!(
        form.status.as_deref(),
        Some("7. satırın kaynağı eksik; önce kaynağını yazın ya da çizimden seçin.")
    );
}

/// Two links left: the least is three, and Uygula has nothing to write.
#[test]
fn kaucuk_levha_needs_three_links() {
    let mut app = matched();
    fit(&mut app, Event::Kind(Kind::Rubber));
    for r in 1..5 {
        calc(&mut app, Calc::Cell(r, USE, "0".into()));
    }
    let form = &app.gis.calc.fit;
    let doc = app.document.as_ref().expect("open");
    let lines = form.summary_lines(&doc.model, &Format::default());
    assert_eq!(
        lines,
        [(
            Line::Info,
            "Kauçuk levha için en az 3 kullanılan bağ gerekir; şimdi 2. Koordinatları yazın, yapıştırın ya da çizimden seçin."
                .to_owned()
        )]
    );
    assert!(form.plan(&doc.model, &Format::default()).is_none());
}

/// The two steps of a fit (docs/adr/0158): Oturt by Helmert with P5 left
/// out, then the window again over the moved local layer: Adla eşle,
/// Kauçuk levha, P5 left out again, P3 a fixed point. P3's place before
/// the sheet.
fn sheet_after_helmert() -> (App, kentos_commands::Vec2) {
    let mut app = matched();
    calc(&mut app, Calc::Cell(4, USE, "0".into()));
    fit(&mut app, Event::Apply);
    assert_eq!(app.dialog, None, "Oturt wrote");
    let _ = app.run("transform.fit");
    assert_eq!(app.dialog, Some(Asking::Gis(gis::GisDialogKind::Calc)));
    fit(&mut app, Event::Kind(Kind::Rubber));
    fit(&mut app, Event::Match);
    calc(&mut app, Calc::Cell(4, USE, "0".into()));
    fit(&mut app, Event::Fix(2));
    let p3 = named(&app, "yerel", "P3");
    (app, p3)
}

/// Uygula: the used links met exactly, the fixed point held; one step,
/// Kauçuk levha; the window closes.
#[test]
fn kaucuk_levha_meets_every_link_in_one_step() {
    let (mut app, p3) = sheet_after_helmert();
    let doc = app.document.as_ref().expect("open");
    let lines = app
        .gis
        .calc
        .fit
        .summary_lines(&doc.model, &Format::default());
    assert_eq!(
        lines[0].1,
        "5 bağ (1 sabit nokta); levha her bağdan tam geçer."
    );
    let links = ["P1", "P2", "P4", "P6"];
    let worst = |app: &App| links.iter().map(|n| gap(app, n)).fold(0.0, f64::max);
    assert!(worst(&app) > 1e-4, "Helmert left millimetres");
    fit(&mut app, Event::Apply);
    assert_eq!(app.dialog, None, "the window closes");
    assert!(worst(&app) < 1e-6, "{}", worst(&app));
    let held = named(&app, "yerel", "P3");
    assert!((held.x - p3.x).hypot(held.y - p3.y) < 1e-6, "P3 stays");
    assert!(gap(&app, "P5") > 0.1, "the blunder is no link");
    assert!(
        logged(
            &app,
            &crate::keys::shown(
                "Vektör oturtma: 10 nesne kauçuk levhayla oturtuldu. Ctrl+Z geri alır."
            )
        ),
        "{:?}",
        app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
    let doc = app.document.as_mut().expect("open");
    assert!(
        doc.model
            .by_layer("yerel")
            .any(|e| matches!(e, Entity::Circle(_))),
        "the circle stays a circle"
    );
    assert_eq!(doc.model.undo().as_deref(), Some("Kauçuk levha"));
    assert!(worst(&app) > 1e-4, "undone in one step");
}

/// The report: the method, the pairs with their local corrections, the
/// links, the corrections and Helmert's numbers.
#[test]
fn the_rubber_report_says_the_links_and_helmert() {
    let mut app = matched();
    fit(&mut app, Event::Kind(Kind::Rubber));
    let doc = app.document.as_ref().expect("open");
    let lines = app
        .gis
        .calc
        .fit
        .report(&doc.model, &Format::default())
        .expect("a report");
    assert_eq!(lines[0], ["Vektör oturtma", "kauçuk levha"]);
    assert_eq!(lines[1][0], "Yöntem");
    assert_eq!(lines[2].len(), 9);
    assert_eq!(lines[3][1], "P1");
    assert_eq!(lines[9], ["Bağ", "6", "Sabit nokta", "0"]);
    assert_eq!(lines[10][0], "En büyük yerel düzeltme (mm)");
    assert_eq!(lines[10][2], "P5");
    assert_eq!(lines[11][0], "Ortalama yerel düzeltme (mm)");
    assert_eq!(lines[12][0], "Helmert m0 (mm)");
    assert!(lines[13][1].starts_with("Ölçek "), "{:?}", lines[13]);
    assert_eq!(lines.len(), 14);
}

/// The window in the web's scenes, for the owner. Not run by default:
/// `cargo test -p kentos-desktop calc::fit::tests::screens -- --ignored --nocapture`
/// (`KENTOS_SHOTS=oturt,…` for some of them); the web's are
/// `node apps/web/scripts/e2e/shots.mjs vectorfit`.
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
                "oturt",
                "oturt-adla-eslendi",
                "oturt-uygulama",
                "oturt-p5-cikti",
                "oturt-afin",
                "oturt-uygulandi",
                "oturt-parametre",
                "oturt-parametre-uyari",
                "oturt-parametre-uygulandi",
                "oturt-levha",
                "oturt-levha-sabit",
                "oturt-levha-uygulandi",
                "oturt-levha-uyari",
            ] {
                if !only.is_empty() && !only.split(',').any(|o| o == name) {
                    continue;
                }
                let mut app = match name {
                    "oturt" => {
                        let mut app = app_with_fit_drawing();
                        let _ = app.run("transform.fit");
                        app
                    }
                    "oturt-levha-sabit" | "oturt-levha-uygulandi" => sheet_after_helmert().0,
                    _ => matched(),
                };
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "oturt-p5-cikti" => calc(&mut app, Calc::Cell(4, USE, "0".into())),
                    "oturt-afin" => fit(&mut app, Event::Kind(Kind::Affine)),
                    "oturt-levha" => fit(&mut app, Event::Kind(Kind::Rubber)),
                    "oturt-levha-uygulandi" => {
                        fit(&mut app, Event::Apply);
                        let _ = app.update(Message::Layer(crate::layering::Event::ZoomTo(
                            "yerel".into(),
                        )));
                    }
                    "oturt-levha-uyari" => {
                        fit(&mut app, Event::Kind(Kind::Rubber));
                        for r in 1..5 {
                            calc(&mut app, Calc::Cell(r, USE, "0".into()));
                        }
                    }
                    "oturt-uygulandi" => {
                        calc(&mut app, Calc::Cell(4, USE, "0".into()));
                        fit(&mut app, Event::Apply);
                        let _ = app.update(Message::Run("view.zoomExtents"));
                    }
                    // Parametrelerle: about P1, the scales of the solution
                    // with P5 left out, its turn, no shift.
                    "oturt-parametre" => {
                        fit(&mut app, Event::Method(Method::Parameters));
                        calc(&mut app, Calc::Known(Field::Base, "P1".into()));
                        for (p, t) in [
                            (Param::ScaleY, "1.0002"),
                            (Param::ScaleX, "0.9997"),
                            (Param::Rotation, "0.5"),
                        ] {
                            fit(&mut app, Event::Param(p, t.into()));
                        }
                    }
                    "oturt-parametre-uyari" => {
                        fit(&mut app, Event::Method(Method::Parameters));
                        calc(&mut app, Calc::Known(Field::Base, "P9".into()));
                        fit(&mut app, Event::Param(Param::ScaleY, "0".into()));
                        fit(&mut app, Event::Param(Param::ShiftX, "12,5x".into()));
                    }
                    "oturt-parametre-uygulandi" => {
                        fit(&mut app, Event::Method(Method::Parameters));
                        calc(&mut app, Calc::Known(Field::Base, "P1".into()));
                        fit(&mut app, Event::Param(Param::ScaleY, "1.5".into()));
                        fit(&mut app, Event::Apply);
                        // The local layer stays far from the TUREF points: the view on it.
                        let _ = app.update(Message::Layer(crate::layering::Event::ZoomTo(
                            "yerel".into(),
                        )));
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if matches!(name, "oturt-uygulama" | "oturt-levha-uyari") {
                    snapshot.operate(app.view(), Box::new(crate::files_testing::SnapAll));
                    snapshot.settle(&mut app, App::view, &mut update);
                }
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

/// UR2-051: the pairs, the solution and the layer to fit are the drawing's: in another drawing opened Vektör
/// oturtma starts empty, and Uygula writes nothing there.
#[test]
fn another_drawing_starts_the_fit_empty() {
    let mut app = matched();
    assert!(
        app.gis.calc.fit.rows.iter().any(|r| !r[0].is_empty()),
        "pairs matched"
    );
    calc(&mut app, Calc::Close);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(2),
    )))));
    let _ = app.run("transform.fit");
    assert!(
        app.gis
            .calc
            .fit
            .rows
            .iter()
            .all(|r| r.iter().all(String::is_empty)),
        "no pairs from the last drawing"
    );
    let doc = app.document.as_ref().expect("open");
    let (count, revision) = (doc.model.len(), doc.model.revision());
    fit(&mut app, Event::Apply);
    let doc = app.document.as_ref().expect("open");
    assert_eq!(
        (doc.model.len(), doc.model.revision()),
        (count, revision),
        "nothing written"
    );
}

/// The target points' places, by name.
fn targets_of(app: &App) -> Vec<(String, kentos_commands::Vec2)> {
    let doc = app.document.as_ref().expect("open");
    doc.model
        .by_layer("tm")
        .filter_map(|e| match e {
            Entity::Point(p) => Some((p.base.label.clone().unwrap_or_default(), p.p)),
            _ => None,
        })
        .collect()
}

/// GIS-038: with nothing selected and the window left as it opens (Uygula on the source's layer), Adla eşle and
/// Uygula move the ten local objects and leave the six TUREF points exactly where they are; Tümü leaves the target
/// layer too and says so; the line before Uygula names each layer moved with its count; asking for the target layer
/// on purpose says the control points move. Undo brings the local points back.
#[test]
fn the_default_fit_moves_the_source_and_never_the_targets() {
    let mut app = app_with_fit_drawing();
    let _ = app.run("transform.fit");
    fit(&mut app, Event::Kind(Kind::Helmert));
    fit(&mut app, Event::Match);
    let form = &app.gis.calc.fit;
    assert_eq!(
        (form.scope, form.layer.as_deref()),
        (Scope::Layer, Some("yerel"))
    );
    let model = &app.document.as_ref().expect("open").model;
    let (reach, target) = form.reach(model, &[]);
    assert_eq!(reach, [("yerel".to_owned(), 10)]);
    assert!(!target);
    // Tümü: every layer but the target's.
    fit(&mut app, Event::Scope(Scope::All));
    let form = &app.gis.calc.fit;
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(form.all_count(model), model.len() - model.count("tm"));
    let (reach, target) = form.reach(model, &[]);
    assert!(reach.iter().all(|(l, _)| l != "tm"), "{reach:?}");
    assert!(!target);
    // The target layer asked on purpose: said.
    fit(&mut app, Event::Scope(Scope::Layer));
    fit(&mut app, Event::Layer("tm".into()));
    let model = &app.document.as_ref().expect("open").model;
    assert!(
        app.gis.calc.fit.reach(model, &[]).1,
        "the control points move: said"
    );
    // Back to Tümü and Uygula: the targets stay to the last digit.
    fit(&mut app, Event::Scope(Scope::All));
    let before = targets_of(&app);
    let p1 = gap(&app, "P1");
    fit(&mut app, Event::Apply);
    assert_eq!(app.dialog, None, "applied");
    assert_eq!(targets_of(&app), before, "the targets are not moved");
    assert!(gap(&app, "P1") < p1, "the source moved onto them");
    let doc = app.document.as_mut().expect("open");
    assert_eq!(doc.model.undo().as_deref(), Some("Oturt"));
    assert!((gap(&app, "P1") - p1).abs() < 1e-9, "undone");
}

/// The window after Adla eşle with Tümü: the line before Uygula, picture `gis-038-oturtma-kapsam`.
#[test]
fn the_reach_line_on_the_screen() {
    let mut app = app_with_fit_drawing();
    let _ = app.run("transform.fit");
    fit(&mut app, Event::Kind(Kind::Helmert));
    fit(&mut app, Event::Match);
    fit(&mut app, Event::Scope(Scope::All));
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 1000.));
    shot.settle(&mut app, App::view, &mut |app: &mut App, m| {
        drop(app.update(m))
    });
    let found = crate::files_testing::find_texts(&mut shot, &app, "Uygula");
    assert!(!found.is_empty());
    crate::files_testing::evidence(&mut shot, &app, "cbs-kullanim", "gis-038-oturtma-kapsam");
}

/// docs/adr/9330: a CBS project's Oturt with nothing selected starts on the active layer that has objects, never on
/// every object; what Uygula takes is said beside it; Parametrelerle's ΔY 10 m with Kopya moves copies of that layer's
/// three points alone, and one undo takes them back.
#[test]
fn a_cbs_projects_oturt_starts_on_the_active_layer_and_moves_it_alone() {
    use crate::tools_scenes::{E, N, Objects, open_edited};
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = Objects::new();
    for x in [0.0, 30.0, 60.0] {
        o.point("parsel", [x, 0.0], None);
    }
    o.line("yol", [0.0, 10.0], [60.0, 10.0]);
    o.line("cizim", [0.0, 20.0], [60.0, 20.0]);
    open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = serde_json::json!("gis")
    });
    let doc = app.document.as_mut().expect("open");
    assert!(doc.model.set_active_layer("parsel"));
    app.gis.calc.fit.method = Method::Parameters;
    let _ = app.run("transform.fit");
    let doc = app.document.as_ref().expect("open");
    let form = &app.gis.calc.fit;
    assert_eq!(
        (form.scope, form.layer.as_deref()),
        (Scope::Layer, Some("parsel")),
        "never Tümü"
    );
    assert_eq!(
        form.reach_words(&doc.model, &[]),
        "Uygula: 1 katman, 3 nesne"
    );
    calc(&mut app, Calc::Known(Field::Base, format!("{E},{N}")));
    for (p, t) in [
        (Param::ScaleY, "1"),
        (Param::ScaleX, "1"),
        (Param::Rotation, "0"),
        (Param::ShiftY, "10"),
        (Param::ShiftX, "0"),
    ] {
        fit(&mut app, Event::Param(p, t.into()));
    }
    fit(&mut app, Event::Copy(true));
    fit(&mut app, Event::Apply);
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(
        (
            model.count("parsel"),
            model.count("yol"),
            model.count("cizim")
        ),
        (6, 1, 1),
        "copies of the layer alone"
    );
    let mut east: Vec<f64> = model
        .by_layer("parsel")
        .filter_map(|e| match e {
            Entity::Point(p) => Some(p.p.x - E),
            _ => None,
        })
        .collect();
    east.sort_by(f64::total_cmp);
    let want = [0.0, 10.0, 30.0, 40.0, 60.0, 70.0];
    assert!(
        east.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-6),
        "{east:?}"
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(
        app.document.as_ref().expect("open").model.count("parsel"),
        3,
        "one undo"
    );
}

/// docs/adr/9330: a locked layer among the ones moved is said before Uygula, which waits.
#[test]
fn a_locked_layer_in_reach_is_said_before_uygula() {
    let mut app = matched();
    app.document
        .as_mut()
        .expect("open")
        .model
        .toggle_layer_locked("yerel");
    let doc = app.document.as_ref().expect("open");
    let locked = app.gis.calc.fit.locked_in_reach(&doc.model, &[]);
    assert_eq!(locked, ["yerel"]);
    let words = super::words::locked_words(&["“Yerel ölçü”".to_owned()]);
    assert!(
        words.starts_with("“Yerel ölçü” katmanı kilitli;"),
        "{words}"
    );
}

/// The pictures (`.run/gui-evidence/kullanim-9330/`): a CBS project's Oturt with nothing selected, on its active layer
/// with what Uygula takes beside it; the same with the layer locked, said before Uygula; Dönüştür's target system and
/// operation apart.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib calc::fit::tests::screens_9330 -- --ignored --nocapture"]
fn screens_9330() {
    use crate::tools_scenes::{E, N, Objects, open_edited};
    crate::drawing_fonts::load();
    let scene = |locked: bool| {
        let mut app = crate::files_testing::app_with_drawing();
        let mut o = Objects::new();
        for x in [0.0, 30.0, 60.0] {
            o.point("parsel", [x, 0.0], None);
        }
        o.line("yol", [0.0, 10.0], [60.0, 10.0]);
        open_edited(&mut app, o, |d| {
            d["settings"]["workspace"] = serde_json::json!("gis")
        });
        let doc = app.document.as_mut().expect("open");
        doc.model.set_active_layer("parsel");
        app.gis.calc.fit.method = Method::Parameters;
        let _ = app.run("transform.fit");
        calc(&mut app, Calc::Known(Field::Base, format!("{E},{N}")));
        for (p, t) in [
            (Param::ScaleY, "1"),
            (Param::ScaleX, "1"),
            (Param::Rotation, "0"),
            (Param::ShiftY, "10"),
            (Param::ShiftX, "0"),
        ] {
            fit(&mut app, Event::Param(p, t.into()));
        }
        if locked {
            app.document
                .as_mut()
                .expect("open")
                .model
                .toggle_layer_locked("parsel");
        }
        app
    };
    crate::files_testing::usage_shots("9330", "01-oturt-etkin-katman", || scene(false));
    crate::files_testing::usage_shots("9330", "02-oturt-kilitli", || scene(true));
    crate::files_testing::usage_shots("9330", "03-donustur", || {
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.update(Message::Run("crs.transform"));
        let send = |app: &mut App, e: crate::calc::convert::Event| {
            drop(app.update(crate::calc::convert::message(e)))
        };
        send(
            &mut app,
            crate::calc::convert::Event::From(crate::calc::convert::Pick::Srid(5256)),
        );
        send(
            &mut app,
            crate::calc::convert::Event::To(crate::calc::convert::Pick::Srid(4326)),
        );
        send(&mut app, crate::calc::convert::Event::A("500000".into()));
        send(&mut app, crate::calc::convert::Event::B("4320000".into()));
        app
    });
}
