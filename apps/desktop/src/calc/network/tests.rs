//! Yatay ağ and Kot ağı dengelemesi (docs/adr/0203 §6–§8) on
//! fixtures/interaction/v1/network-adjust.kcad: the windows' tables give the
//! reference's answers (fixtures/network-adjust/v1), what does not hold is
//! said, Çizime yaz writes in one step, Karne editörü fills both windows;
//! and pictures for the owner.

use std::path::PathBuf;

use kentos_contracts::Entity;
use serde_json::Value;

use super::{
    Cells, Event, HEIGHT_COLS, KNOWN_COLS, LEVEL_COLS, LEVEL_TITLE, LevelKind, NETWORK_TITLE,
    POINT_KIND, ROW_COLS, Sheet,
};
use crate::app::{App, Message};
use crate::calc::{Event as CalcEvent, Window};
use crate::files_testing::last_said;

/// The app with the scene drawing on screen.
fn app_with_scene() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../fixtures/interaction/v1/network-adjust.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The reference's case whose name starts so.
fn case(start: &str) -> Value {
    let file: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/network-adjust/v1/cases.json"
    ))
    .expect("cases.json reads");
    file["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(start)))
        .cloned()
        .expect("the case")
}

/// A number as it is typed: the shortest text that reads back; nothing for none.
fn typed(v: &Value) -> String {
    v.as_f64().map(|x| x.to_string()).unwrap_or_default()
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_owned()
}

/// A σ in metres as the table takes it, in millimetres.
fn sigma_mm(k: &Value) -> String {
    k.get("sigma")
        .and_then(Value::as_f64)
        .map(|s| (s * 1000.0).to_string())
        .unwrap_or_default()
}

/// Yatay ağ dengelemesi open with the case typed in: its known points by
/// their names alone when `by_name` (the drawing has them), else with Y, X
/// and σ.
fn network_with(app: &mut App, c: &Value, by_name: bool) {
    let _ = app.update(Message::Run("calc.network"));
    assert_eq!(app.calc.open, Some(Window::Network));
    let known = c["known"]
        .as_array()
        .expect("known")
        .iter()
        .map(|k| {
            if by_name {
                vec![text(&k["name"])]
            } else {
                vec![
                    text(&k["name"]),
                    typed(&k["y"]),
                    typed(&k["x"]),
                    sigma_mm(k),
                ]
            }
        })
        .collect();
    app.calc.network.known = Cells::of(KNOWN_COLS, known);
    let rows = c["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| {
            vec![
                text(&r["station"]),
                text(&r["target"]),
                typed(&r["direction"]),
                typed(&r["distance"]),
            ]
        })
        .collect();
    app.calc.network.rows = Cells::of(ROW_COLS, rows);
    app.network_solve();
}

/// Kot ağı dengelemesi open with the case typed in, its known heights by their names alone when `by_name`.
fn level_with(app: &mut App, c: &Value, by_name: bool) {
    let _ = app.update(Message::Run("calc.levelNetwork"));
    assert_eq!(app.calc.open, Some(Window::Level));
    app.calc.level.kind = match c["levelKind"].as_str() {
        Some("trigonometric") => LevelKind::Trigonometric,
        _ => LevelKind::Geometric,
    };
    let known = c["known"]
        .as_array()
        .expect("known")
        .iter()
        .map(|k| {
            if by_name {
                vec![text(&k["name"])]
            } else {
                vec![text(&k["name"]), typed(&k["h"]), sigma_mm(k)]
            }
        })
        .collect();
    app.calc.level.known = Cells::of(HEIGHT_COLS, known);
    let rows = c["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| {
            vec![
                text(&r["from"]),
                text(&r["to"]),
                typed(&r["dh"]),
                typed(&r["length"]),
            ]
        })
        .collect();
    app.calc.level.rows = Cells::of(LEVEL_COLS, rows);
    app.network_solve();
}

fn near(got: f64, want: f64, abs: f64, rel: f64) -> bool {
    (got - want).abs() <= abs.max(rel * want.abs())
}

/// The drawing's point named `name`.
fn point<'a>(app: &'a App, name: &str) -> Option<&'a kentos_contracts::PointEntity> {
    super::named_points(&app.document.as_ref()?.model, name)
        .first()
        .copied()
}

/// The braced quadrilateral with K1 and K2 by their names alone: the
/// drawing's points, the reference's answer (docs/adr/0203 §3–§4); the
/// summary's counts, m0 and model test; Y1 drawn 4 cm away, Y2 new.
#[test]
fn the_braced_quadrilateral_adjusts_as_the_reference() {
    let mut app = app_with_scene();
    let c = case("Çaprazlı dörtgen");
    network_with(&mut app, &c, true);
    let form = &app.calc.network;
    assert!(
        form.solved.problems.is_empty(),
        "{:?}",
        form.solved.problems
    );
    let Some(Ok(r)) = &form.solved.result else {
        panic!("adjusted: {:?}", form.solved.result);
    };
    let want = &c["expect"];
    assert_eq!(
        (r.n, r.u, r.f, r.worst, r.passed),
        (18, 8, 10, None, Some(true))
    );
    for (got, w) in r
        .points
        .iter()
        .zip(want["points"].as_array().expect("points"))
    {
        assert_eq!(got.name, text(&w["name"]));
        assert!(
            near(got.y, w["y"].as_f64().unwrap_or(f64::NAN), 1e-6, 0.0),
            "{} y",
            got.name
        );
        assert!(
            near(got.x, w["x"].as_f64().unwrap_or(f64::NAN), 1e-6, 0.0),
            "{} x",
            got.name
        );
        for (g, k) in [(got.sp, "sp"), (got.a, "a"), (got.b, "b")] {
            assert!(
                near(g, w[k].as_f64().unwrap_or(f64::NAN), 1e-9, 1e-6),
                "{} {k}",
                got.name
            );
        }
    }
    assert_eq!(r.points.len(), 2);
    let model = &app.document.as_ref().expect("a drawing").model;
    let format = kentos_interaction::Format::of(model.settings()).metric();
    let rows = form.point_rows(r, model, &format);
    // Y1 is drawn 37.5 mm from its adjusted place; Y2 is new.
    assert_eq!((rows[0][9].as_str(), rows[1][9].as_str()), ("37.5", "yeni"));
    let lines = super::summary_lines(super::Counts::from(r), None);
    assert_eq!(
        lines[0].1,
        format!(
            "18 gözlem, 8 bilinmeyen, serbestlik derecesi 10; {} yineleme.",
            r.iterations
        )
    );
    assert!(lines[1].1.starts_with("m₀ = "), "{}", lines[1].1);
    assert!(lines[1].1.contains("model testi geçti"), "{}", lines[1].1);
}

/// A distance 6 cm long: the test flags it, the summary names it the way
/// the table has it, the model test fails, the observations' table marks
/// its row (docs/adr/0203 §4).
#[test]
fn a_blunder_is_named_and_marked() {
    let mut app = app_with_scene();
    network_with(&mut app, &case("Uyuşumsuz ölçü"), false);
    let form = &app.calc.network;
    let Some(Ok(r)) = &form.solved.result else {
        panic!("adjusted: {:?}", form.solved.result);
    };
    assert_eq!((r.worst, r.passed), (Some(8), Some(false)));
    let o = &r.observations[8];
    assert_eq!(form.observation_name(o), "K2 → Y1 kenarı");
    let said = super::worst_line(o, &form.observation_name(o));
    assert!(
        said.starts_with("Uyuşumsuz ölçü olabilir: K2 → Y1 kenarı (w ")
            && said.ends_with("); önce bu ölçüyü denetleyin."),
        "{said}"
    );
    let (rows, marked) = form.observation_rows(r, kentos_contracts::AngleUnit::Grad);
    assert!(marked[8], "the distance's row is marked");
    assert_eq!(
        (
            rows[8][0].as_str(),
            rows[8][1].as_str(),
            rows[8][2].as_str(),
            rows[8][3].as_str()
        ),
        ("K2", "Y1", "Kenar", "495.8425")
    );
    let lines = super::summary_lines(super::Counts::from(r), Some(said.clone()));
    assert!(lines[1].1.contains("model testi kaldı"), "{}", lines[1].1);
    assert_eq!(lines.last().map(|l| l.1.clone()), Some(said));
}

/// What the tables cannot give the core is said, the table's line named,
/// and nothing is adjusted; the core's refusal is said as it is.
#[test]
fn what_does_not_hold_is_said() {
    let mut app = app_with_scene();
    let _ = app.update(Message::Run("calc.network"));
    let cells = |rows: &[&[&str]]| -> Vec<Vec<String>> {
        rows.iter()
            .map(|r| r.iter().map(|t| (*t).to_owned()).collect())
            .collect()
    };
    app.calc.network.known = Cells::of(
        KNOWN_COLS,
        cells(&[
            &["K9"],
            &["K1", "487000"],
            &["K2", "", "", "-2"],
            &["", "1", "2"],
        ]),
    );
    app.calc.network.rows = Cells::of(
        ROW_COLS,
        cells(&[
            &["K1", "", "10"],
            &["K1", "Y1"],
            &["K1", "Y1", "abc"],
            &["K1", "Y1", "", "1,2,3"],
        ]),
    );
    app.network_solve();
    let form = &app.calc.network;
    assert!(form.solved.result.is_none());
    assert_eq!(
        form.solved.problems,
        [
            "K9: çizimde bu adla nokta yok; Y ve X yazın.",
            "K1: Y ve X birlikte yazılır.",
            "K2: σ sıfırdan büyük bir sayı olmalı (mm).",
            "4. bilinen noktanın adı yok.",
            "1. gözlemde durulan ya da bakılan yok.",
            "2. gözlemde doğrultu ya da kenar yok.",
            "3. gözlemde doğrultu bir sayı değil.",
            "4. gözlemde kenar bir sayı değil.",
        ]
    );
    // The core's own refusal: trilateration with points the drawing has not.
    network_with(&mut app, &case("Trilaterasyon çizimsiz"), false);
    let form = &app.calc.network;
    assert_eq!(
        form.solved.result,
        Some(Err(text(&case("Trilaterasyon çizimsiz")["error"])))
    );
    // Kot ağı: a known height the drawing has not, a row without its length.
    let _ = app.update(Message::Run("calc.levelNetwork"));
    app.calc.level.known = Cells::of(HEIGHT_COLS, cells(&[&["N3"], &["R1", "x"]]));
    app.calc.level.rows = Cells::of(
        LEVEL_COLS,
        cells(&[&["R1", "N1", "1.2"], &["R1", "", "1", "2"]]),
    );
    app.network_solve();
    assert_eq!(
        app.calc.level.solved.problems,
        [
            "N3: çizimde bu adla kotlu nokta yok; kotu yazın.",
            "R1: kot bir sayı değil.",
            "1. gözlemde kot farkı ya da uzunluk yok.",
            "2. gözlemde başlangıç ya da bitiş yok.",
        ]
    );
}

/// Çizime yaz (docs/adr/0203 §8): Y1 moves to its adjusted place with the
/// line's vertex on it, Y2 is added on Poligon as an Ağ noktası, K1 and K2
/// stay; one undo step named after the window takes it all back.
#[test]
fn cizime_yaz_moves_and_adds_in_one_step() {
    let mut app = app_with_scene();
    network_with(&mut app, &case("Çaprazlı dörtgen"), true);
    assert_eq!(app.calc.network.layer.as_deref(), Some("poligon"));
    let Some(Ok(r)) = app.calc.network.solved.result.clone() else {
        panic!("adjusted");
    };
    let k1 = point(&app, "K1").cloned().expect("K1");
    let _ = app.update(Message::Calc(CalcEvent::AddPoints));
    assert_eq!(app.calc.open, None, "the window closes");
    assert_eq!(
        last_said(&app),
        format!(
            "{NETWORK_TITLE}: 1 nokta dengelenmiş yerine taşındı, 1 nokta eklendi (Ctrl+Z geri alır)."
        )
    );
    let y1 = point(&app, "Y1").cloned().expect("Y1");
    assert_eq!((y1.p.x, y1.p.y), (r.points[0].y, r.points[0].x));
    let y2 = point(&app, "Y2").cloned().expect("Y2");
    assert_eq!((y2.p.x, y2.p.y), (r.points[1].y, r.points[1].x));
    assert_eq!(y2.base.layer_id, "poligon");
    assert_eq!(
        y2.base.attrs.get("Tür").map(String::as_str),
        Some(POINT_KIND)
    );
    assert_eq!(y2.base.attrs.get("Ad").map(String::as_str), Some("Y2"));
    assert_eq!(point(&app, "K1").cloned(), Some(k1));
    let doc = app.document.as_mut().expect("a drawing");
    let line = doc.model.entities().find_map(|e| match e {
        Entity::Polyline(l) if l.base.id == 4 => Some(l.clone()),
        _ => None,
    });
    let line = line.expect("the line");
    assert_eq!((line.pts[1].x, line.pts[1].y), (y1.p.x, y1.p.y));
    assert_eq!(app.selection.len(), 2);
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(doc.model.undo().as_deref(), Some(NETWORK_TITLE));
    assert!(point(&app, "Y2").is_none());
    let y1 = point(&app, "Y1").expect("Y1");
    assert_eq!((y1.p.x, y1.p.y), (487030.02, 4420349.97));
}

/// Kot ağı dengelemesi with R1 and R2 by their names (the drawing's
/// heights): the reference's heights; Çizime yaz gives N1 and N2 theirs,
/// their Z (m) and the vertex on N1, says N3 is not drawn; one undo step.
#[test]
fn kot_agi_writes_the_heights_in_one_step() {
    let mut app = app_with_scene();
    let c = case("Nivelman halkaları");
    level_with(&mut app, &c, true);
    let form = &app.calc.level;
    let Some(Ok(r)) = form.solved.result.clone() else {
        panic!("adjusted: {:?}", form.solved);
    };
    assert_eq!((r.n, r.u, r.f, r.passed), (7, 3, 4, Some(true)));
    for (got, w) in r
        .points
        .iter()
        .zip(c["expect"]["points"].as_array().expect("points"))
    {
        assert_eq!(got.name, text(&w["name"]));
        assert!(
            near(got.h, w["h"].as_f64().unwrap_or(f64::NAN), 1e-7, 0.0),
            "{}",
            got.name
        );
        assert!(
            near(got.sh, w["sh"].as_f64().unwrap_or(f64::NAN), 1e-9, 1e-6),
            "{}",
            got.name
        );
    }
    let _ = app.update(Message::Calc(CalcEvent::AddPoints));
    assert_eq!(
        last_said(&app),
        format!("{LEVEL_TITLE}: N3 çizimde yok; yazılmadı.")
    );
    let n1 = point(&app, "N1").cloned().expect("N1");
    assert_eq!(n1.z, Some(r.points[0].h));
    assert_eq!(
        n1.base.attrs.get("Z (m)").cloned(),
        Some(kentos_interaction::fixed(r.points[0].h, 3))
    );
    assert_eq!(point(&app, "N2").and_then(|p| p.z), Some(r.points[1].h));
    let doc = app.document.as_mut().expect("a drawing");
    let zs = doc.model.entities().find_map(|e| match e {
        Entity::Polyline(l) if l.base.id == 9 => l.zs.clone(),
        _ => None,
    });
    assert_eq!(zs, Some(vec![Some(102.4567), Some(r.points[0].h)]));
    assert_eq!(doc.model.undo().as_deref(), Some(LEVEL_TITLE));
    assert_eq!(point(&app, "N1").and_then(|p| p.z), Some(105.0));
}

/// A typed cell solves again at once; the kind's switch is the window's.
#[test]
fn a_cell_typed_solves_again() {
    let mut app = app_with_scene();
    level_with(&mut app, &case("Nivelman halkaları"), true);
    let send = |app: &mut App, e: Event| {
        let _ = app.update(Message::Calc(CalcEvent::Network(e)));
    };
    send(
        &mut app,
        Event::Cell(Sheet::LevelRows, 2, 2, "-3.62482".to_owned()),
    );
    let Some(Ok(r)) = &app.calc.level.solved.result else {
        panic!("adjusted");
    };
    assert_eq!((r.worst, r.passed), (Some(2), Some(false)));
    assert_eq!(
        app.calc.level.observation_name(&r.observations[2]),
        "N2 → R2 kot farkı"
    );
    send(&mut app, Event::Kind(LevelKind::Trigonometric));
    assert_eq!(app.calc.level.kind, LevelKind::Trigonometric);
    assert!(matches!(app.calc.level.solved.result, Some(Ok(_))));
}

/// Karne editörü's Ağ dengelemesine aktar and Kot ağına aktar (docs/adr/0203
/// §6): every station's rows, the directions in the project's unit; the
/// windows open, the known points stay.
#[test]
fn karne_editoru_fills_both_windows() {
    let mut app = crate::files_testing::app_with_drawing();
    app.calc.network.known = Cells::of(KNOWN_COLS, vec![vec!["ST1".to_owned()]]);
    let _ = app.update(Message::Run("calc.fieldbook"));
    let fb = |app: &mut App, e: crate::calc::fieldbook::Event| {
        let _ = app.update(Message::Calc(CalcEvent::FieldBook(e)));
    };
    let sample =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/field/v1/sample.gsi");
    fb(
        &mut app,
        crate::calc::fieldbook::Event::Picked(Some(sample)),
    );
    fb(&mut app, crate::calc::fieldbook::Event::TransferNetwork);
    assert_eq!(app.calc.open, Some(Window::Network));
    let rows: Vec<&Vec<String>> = app.calc.network.rows.filled().map(|(_, r)| r).collect();
    assert!(rows.len() >= 10, "{} rows", rows.len());
    assert_eq!((rows[0][0].as_str(), rows[0][1].as_str()), ("ST1", "P2"));
    assert!(rows.iter().any(|r| r[0] == "ST2"));
    assert_eq!(
        last_said(&app),
        format!(
            "Karne editörü: {} gözlem Yatay ağ dengelemesi'ne aktarıldı.",
            rows.len()
        )
    );
    assert_eq!(
        app.calc.network.known.rows[0][0], "ST1",
        "the known points stay"
    );
    let _ = app.update(Message::Run("calc.fieldbook"));
    fb(&mut app, crate::calc::fieldbook::Event::TransferLevels);
    assert_eq!(app.calc.open, Some(Window::Level));
    assert_eq!(app.calc.level.kind, LevelKind::Trigonometric);
    let n = app.calc.level.rows.filled().count();
    assert!(n >= 5, "{n} rows");
    assert_eq!(
        last_said(&app),
        format!("Karne editörü: {n} kot farkı Kot ağı dengelemesi'ne aktarıldı.")
    );
}

/// The windows for the owner at 1440×900 and 1100×650, light and dark
/// (.run/shots/ag-*): the braced quadrilateral solved, its results, the
/// blunder, what does not hold, the levelling rings and a blunder in them.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    for (theme, w, h) in [
        ("light", 1440.0, 900.0),
        ("dark", 1440.0, 900.0),
        ("light", 1100.0, 650.0),
        ("dark", 1100.0, 650.0),
    ] {
        for name in [
            "yatay",
            "yatay-sonuc",
            "uyusumsuz",
            "sorunlar",
            "kot",
            "kot-uyusumsuz",
        ] {
            if only
                .as_deref()
                .is_some_and(|o| !o.split(',').any(|x| x == name))
            {
                continue;
            }
            let mut app = app_with_scene();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            match name {
                "yatay" | "yatay-sonuc" => network_with(&mut app, &case("Çaprazlı dörtgen"), true),
                "uyusumsuz" => network_with(&mut app, &case("Uyuşumsuz ölçü"), false),
                "sorunlar" => {
                    network_with(&mut app, &case("Çaprazlı dörtgen"), true);
                    app.calc.network.known.rows[1] =
                        vec!["K9".to_owned(), String::new(), String::new(), String::new()];
                    app.calc.network.rows.rows[3][2] = "abc".to_owned();
                    app.network_solve();
                }
                "kot" => level_with(&mut app, &case("Nivelman halkaları"), true),
                _ => level_with(&mut app, &case("Uyuşumsuz kot farkı"), true),
            }
            app.follow.flash = None;
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            if name.ends_with("-sonuc") || name.ends_with("uyusumsuz") {
                // The results at the body's foot.
                snapshot.operate(app.view(), Box::new(crate::files_testing::SnapAll));
                snapshot.settle(&mut app, App::view, &mut update);
            }
            let file = out.join(format!("ag-{name}-{w}x{h}-{theme}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
