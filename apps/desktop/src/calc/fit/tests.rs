//! Vektör oturtma's tests: the form's rules, then the window through the
//! app over `fixtures/interaction/v1/vector-fit.kcad` (docs/adr/0156 §7): a
//! parcel surveyed in a local system (layer `yerel`) and the same six points
//! measured in TUREF (layer `tm`), P5 with a 15 cm blunder. The steps of the
//! web's `vectorfit` scenes (shots.mjs), and their pictures.

use kentos_interaction::Format;

use super::*;
use crate::app::{App, Dialog as Asking};
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
    for kind in [FitKind::Helmert, FitKind::Affine, FitKind::Projective] {
        form.kind = kind;
        form.solve();
        let fit = form.fit().expect("solved");
        let t = transform_of(fit).expect("its numbers");
        let ok = match (kind, &t) {
            (FitKind::Helmert, Transform::Similarity { a, b, .. }) => {
                (a - 1.0).abs() < 1e-12 && b.abs() < 1e-12
            }
            (FitKind::Affine, Transform::Affine { m, .. }) => {
                (m[0] - 1.0).abs() < 1e-12 && (m[3] - 1.0).abs() < 1e-12
            }
            (FitKind::Projective, Transform::Projective { h, .. }) => {
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
    let lines = form.report(&Format::default());
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
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../fixtures/interaction/v1/vector-fit.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn calc(app: &mut App, e: Calc) {
    let _ = app.update(Message::Calc(e));
}

fn fit(app: &mut App, e: Event) {
    calc(app, Calc::Fit(e));
}

/// The scene's start (the web's `matchFit`): Helmert, Adla eşle from the
/// local points to the TUREF ones, Uygula on the local layer.
fn matched() -> App {
    let mut app = app_with_fit_drawing();
    let _ = app.run("transform.fit");
    assert_eq!(app.dialog, Some(Asking::Calc));
    fit(&mut app, Event::Kind(FitKind::Helmert));
    fit(&mut app, Event::Source("yerel".into()));
    fit(&mut app, Event::Target("tm".into()));
    fit(&mut app, Event::Match);
    fit(&mut app, Event::Scope(Scope::Layer));
    fit(&mut app, Event::Layer("yerel".into()));
    app
}

/// A named point's place on a layer.
fn named(app: &App, layer: &str, name: &str) -> kentos_contracts::Vec2 {
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
    app.calc.fit.fit().and_then(|f| f.m0).expect("an m0") * 1000.0
}

/// On opening, the layers holding named points are offered (the drawing's
/// own layer has none); Adla eşle makes the six pairs, P5 the worst.
#[test]
fn adla_esle_pairs_the_named_points_and_p5_reads_worst() {
    let mut app = app_with_fit_drawing();
    let _ = app.run("transform.fit");
    let form = &app.calc.fit;
    assert_eq!(form.source.as_deref(), Some("yerel"));
    assert_eq!(form.target.as_deref(), Some("tm"));
    assert_eq!(form.scope, Scope::All, "nothing is selected");
    assert_eq!(form.layer.as_deref(), Some("cizim"), "the active layer");
    assert!(form.fit().is_none(), "four empty rows");
    let app = matched();
    assert_eq!(last_said(&app), "Vektör oturtma: 6 çift adla eşlendi.");
    let form = &app.calc.fit;
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
    let form = &app.calc.fit;
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
            && said.ends_with(" mm). Ctrl+Z geri alır."),
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
    assert_eq!(app.dialog, Some(Asking::Calc), "the window stays");
    let status = app.calc.fit.status.clone().expect("the reason");
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
    assert_eq!(app.dialog, Some(Asking::Calc));
    let row = &app.calc.fit.rows[0];
    assert_eq!(
        (row[SOURCE_Y].as_str(), row[SOURCE_X].as_str()),
        ("1062.5", "2003.25")
    );
    assert_eq!(row[NAME], "P2", "the point it lies on");
    // The target of the same row: the name stays.
    fit(&mut app, Event::Pick(0, Side::Target));
    let _ = app.submit_line("487177.949,4420235.009");
    let row = &app.calc.fit.rows[0];
    assert_eq!(row[NAME], "P2");
    assert_eq!(row[TARGET_Y], "487177.949");
    // Esc: nothing changes, the window opens again.
    fit(&mut app, Event::Pick(1, Side::Source));
    app.cancel();
    assert_eq!(app.dialog, Some(Asking::Calc));
    assert!(app.calc.fit.rows[1][SOURCE_Y].is_empty());
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
    let form = &app.calc.fit;
    assert_eq!(form.rows[0][USE], "", "Kullan takes nothing pasted");
    assert_eq!(form.rows[0][NAME], "A");
    assert_eq!(form.rows[1][TARGET_X], "200");
    assert!(form.fit().is_some(), "two pairs: Helmert");
}

/// Snaps every scrollable to its end (the dialog's body has no id).
struct SnapAll;

impl iced::advanced::widget::Operation for SnapAll {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        _bounds: iced::Rectangle,
        _content_bounds: iced::Rectangle,
        _translation: iced::Vector,
        state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        state.snap_to(iced::widget::scrollable::RelativeOffset::END.into());
    }
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
            ] {
                if !only.is_empty() && !only.split(',').any(|o| o == name) {
                    continue;
                }
                let mut app = if name == "oturt" {
                    let mut app = app_with_fit_drawing();
                    let _ = app.run("transform.fit");
                    app
                } else {
                    matched()
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
                    "oturt-afin" => fit(&mut app, Event::Kind(FitKind::Affine)),
                    "oturt-uygulandi" => {
                        calc(&mut app, Calc::Cell(4, USE, "0".into()));
                        fit(&mut app, Event::Apply);
                        let _ = app.update(Message::Run("view.zoomExtents"));
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "oturt-uygulama" {
                    snapshot.operate(app.view(), Box::new(SnapAll));
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
