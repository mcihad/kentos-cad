//! Pafta eğrileri (sheet_contours_window.rs, docs/adr/1820) on the shared cases' drawing of
//! `road.terrains.sheetcontours`: the window guesses the contour layer and the detail layers, its plan is the shared
//! case's; a rule unticked changes the plan; Çiz writes the lines and heights in one undo step; Harita offers it; the
//! summit's height ticked is the shared case's, the building's vertices left out (docs/adr/2320); the auxiliary lines
//! take the package's 5/1 mm pattern as a project line type (docs/adr/2720); the slopes' tops and bottoms guessed
//! from their layers' names and their corners' heights the shared case's (docs/adr/3120); the built-up areas' and roads'
//! heights (m. 63(2)) and the chosen points' check against m. 93(1)(c)'s limit the shared case's, the drawing kept and
//! stale once its surface changes (docs/adr/4740). And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::sheet_contours_window::Event as E;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.sheetcontours.json")).expect("cases")
}

fn opened(program: Option<Module>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn the_plan_is_the_shared_case_s_and_draw_writes_in_one_step() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("terrain.sheetContours"));
    assert_eq!(app.dialog, Some(Dialog::SheetContours));
    let s = &app.sheet_contours;
    assert_eq!(s.layer, "esyukselti");
    assert_eq!(s.details, vec!["bina".to_owned(), "yol".to_owned(), "sev".to_owned()]);
    let r = s.answer.clone().expect("an answer").expect("planned");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    assert_eq!(Some(r.lines.len()), want["lines"].as_array().map(Vec::len));
    assert_eq!(Some(r.labels.len()), want["labels"].as_array().map(Vec::len));
    assert!((r.cut - want["cut"].as_f64().expect("cut")).abs() < 1e-9);
    // The auxiliary lines unticked: none of them in the plan.
    let _ = app.update(Message::SheetContours(E::Auxiliary(false)));
    let off = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    assert!(off.lines.iter().all(|l| l.kind != "auxiliary") && off.lines.len() < r.lines.len());
    let _ = app.update(Message::SheetContours(E::Auxiliary(true)));
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::SheetContours(E::Draw));
    let after = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    assert_eq!(after, before + r.lines.len() + r.labels.len());
    assert_eq!(app.dialog, None);
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Pafta eş yükselti eğrileri"));
}

/// docs/adr/2720: the auxiliary lines drawn by Çiz take the package's 5/1 mm pattern (Ek-3 0003) as the project line
/// type the plan names, added to the project's types; undo takes the lines and leaves the type (a project setting).
#[test]
fn the_auxiliary_lines_take_the_package_s_pattern() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("terrain.sheetContours"));
    let r = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    let name = r.pattern.clone().expect("a pattern");
    assert_eq!(name, "Yardımcı eğri (bohhbuy-2018-m78)");
    let _ = app.update(Message::SheetContours(E::Draw));
    let doc = &app.document.as_ref().expect("open").model;
    let t = doc.settings().line_types.iter().find(|t| t.name == name).expect("the type");
    let lengths: Vec<f64> = t.elements.iter().map(|e| e.length).collect();
    assert_eq!((lengths, t.unit), (vec![5.0, -1.0], kentos_commands::PatternUnit::Paper));
    let auxiliary = r.lines.iter().filter(|l| l.kind == "auxiliary").count();
    let patterned: Vec<_> =
        doc.entities().filter(|e| e.base().look.line_pattern.as_deref() == Some(name.as_str())).collect();
    assert_eq!(patterned.len(), auxiliary);
    assert!(patterned.iter().all(|e| e.base().look.line_type == Some(kentos_commands::LineType::Dashed)));
    let doc = &mut app.document.as_mut().expect("open").model;
    assert_eq!(doc.undo().as_deref(), Some("Pafta eş yükselti eğrileri"));
    assert!(doc.entities().all(|e| e.base().look.line_pattern.is_none()), "the lines gone");
    assert!(doc.settings().line_types.iter().any(|t| t.name == name), "the type stays");
}

/// `.run/gui-evidence/yardimci-egri-deseni-2720/`: the drawn contours near an auxiliary line at the plot scale, its
/// 5 mm dashes and 1 mm gaps, light and dark.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib sheet_contours_window_tests::pattern_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn pattern_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yardimci-egri-deseni-2720");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("terrain.sheetContours"));
        let r = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
        let _ = app.update(Message::SheetContours(E::Draw));
        // Around the longest auxiliary line, 60 m wide: at 1:1000 its 5 mm dashes are 5 m.
        let aux = r.lines.iter().filter(|l| l.kind == "auxiliary").max_by_key(|l| l.points.len()).expect("an auxiliary line");
        let mid = aux.points[aux.points.len() / 2];
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        // The viewport's size is known after a first frame; the camera is fitted then.
        snapshot.settle(&mut app, App::view, &mut update);
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: mid[0] - 30.0, min_y: mid[1] - 18.0, max_x: mid[0] + 30.0, max_y: mid[1] + 18.0 },
            12.0,
        );
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-yardimci-egri-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/2320: Tepe ve çukur kotları ticked gives the shared case's summit (the details ticked: the building's
/// vertices left out); Çiz writes its point and height beside the lines and labels.
#[test]
fn the_summit_height_is_the_shared_case_s() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("terrain.sheetContours"));
    let _ = app.update(Message::SheetContours(E::Spots(true)));
    let r = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    let case = cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "Tepe kotu detaylarla: binanın içindeki köşe sayılmaz")
        .expect("the case")["steps"][0]["result"]["output"]["spots"]
        .clone();
    assert_eq!(r.spots.len(), 1);
    assert_eq!(serde_json::to_value(&r.spots).expect("json"), case);
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::SheetContours(E::Draw));
    let after = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    assert_eq!(after, before + r.lines.len() + r.labels.len() + 2);
}

/// docs/adr/3120: the slopes' top and bottom layers guessed from their names and left out of the details; Şev kotları
/// ticked gives the shared case's corners and heights; with no bottom layer only the top's; Çiz writes a point and a
/// height for each.
#[test]
fn the_slope_heights_are_the_shared_case_s() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("terrain.sheetContours"));
    let s = &app.sheet_contours;
    assert_eq!((s.slope_top.as_str(), s.slope_bottom.as_str()), ("sev-ust", "sev-dip"));
    assert_eq!(s.details, vec!["bina".to_owned(), "yol".to_owned(), "sev".to_owned()]);
    assert!(s.answer.clone().expect("an answer").expect("planned").spots.is_empty(), "not ticked: none");
    let _ = app.update(Message::SheetContours(E::Slopes(true)));
    let r = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    let case = cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Şev kotları: üst ve dip")))
        .expect("the case")["steps"][0]["result"]["output"]["spots"]
        .clone();
    // The places, kinds and texts the case's; the heights within the cases' 10⁻⁹ m (Triangle's plane and KentOS's).
    let want = case.as_array().expect("spots");
    assert_eq!(r.spots.len(), want.len());
    for (got, w) in r.spots.iter().zip(want) {
        assert_eq!((got.kind.as_str(), got.x, got.y, got.text.as_str()), (w["kind"].as_str().expect("kind"), w["x"].as_f64().expect("x"), w["y"].as_f64().expect("y"), w["text"].as_str().expect("text")));
        assert!((got.z - w["z"].as_f64().expect("z")).abs() < 1e-9, "{} vs {}", got.z, w["z"]);
    }
    assert!(r.notes.iter().any(|n| n.contains("Yüzeyin dışındaki 1 köşe yazılmadı")), "{:?}", r.notes);
    let _ = app.update(Message::SheetContours(E::SlopeBottom(String::new())));
    let top = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    assert!(top.spots.len() == 4 && top.spots.iter().all(|s| s.kind == "slopeTop"), "{:?}", top.spots);
    let _ = app.update(Message::SheetContours(E::SlopeBottom("sev-dip".into())));
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::SheetContours(E::Draw));
    let after = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    assert_eq!(after, before + r.lines.len() + r.labels.len() + 2 * r.spots.len());
}

/// docs/adr/3520: Seçilen noktalar's layer guessed from its name; ticked, the chosen points' heights are the shared
/// case's (the measured one with the surface's beside it, the others from the surface; the one off the surface without
/// a height said); Çiz writes their heights alone, the points being the surveyor's.
#[test]
fn the_chosen_points_heights_are_the_shared_case_s() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("terrain.sheetContours"));
    assert_eq!(app.sheet_contours.chosen_layer, "secilen");
    let _ = app.update(Message::SheetContours(E::Chosen(true)));
    let r = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    let case = cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Seçilen noktalar: kendi")))
        .expect("the case")["steps"][0]["result"]["output"]["spots"]
        .clone();
    let want = case.as_array().expect("spots");
    assert_eq!(r.spots.len(), want.len());
    for (got, w) in r.spots.iter().zip(want) {
        assert_eq!((got.kind.as_str(), got.x, got.y, got.text.as_str()), ("chosen", w["x"].as_f64().expect("x"), w["y"].as_f64().expect("y"), w["text"].as_str().expect("text")));
        assert!((got.z - w["z"].as_f64().expect("z")).abs() < 1e-9);
        assert_eq!(got.surface.is_some(), w.get("surface").is_some());
    }
    assert!(r.notes.iter().any(|n| n.contains("en çok 0.145 m ayrılıyor") && n.contains("1 nokta yazılmadı")), "{:?}", r.notes);
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::SheetContours(E::Draw));
    let after = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    assert_eq!(after, before + r.lines.len() + r.labels.len() + r.spots.len(), "one height each");
}

/// The output of the shared case whose name starts so.
fn case_output(start: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(start))).expect(start)["steps"][0]
        ["result"]["output"]
        .clone()
}

/// docs/adr/4740 (m. 63(2)): Yerleşik alan's layers guessed from their names (the road's centre line left out of the
/// details); ticked, with only the building cut, the road's and the built-up area's heights are the shared case's and
/// the summary counts them; Çiz keeps the drawing with its surface and layers.
#[test]
fn the_built_up_heights_are_the_shared_case_s() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("terrain.sheetContours"));
    let s = &app.sheet_contours;
    assert_eq!((s.built_area.as_str(), s.built_road.as_str()), ("yerlesik", "yol-ekseni"));
    assert!(!s.details.contains(&"yol-ekseni".to_owned()));
    for d in ["yol", "sev"] {
        let _ = app.update(Message::SheetContours(E::Detail(d.into(), false)));
    }
    let _ = app.update(Message::SheetContours(E::Built(true)));
    let r = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    let want = case_output("m. 63(2): yolda");
    let spots = want["spots"].as_array().expect("spots");
    assert_eq!(r.spots.len(), spots.len());
    for (got, w) in r.spots.iter().zip(spots) {
        assert_eq!((got.kind.as_str(), got.text.as_str()), (w["kind"].as_str().expect("kind"), w["text"].as_str().expect("text")));
        assert!((got.x - w["x"].as_f64().expect("x")).abs() < 1e-9 && (got.y - w["y"].as_f64().expect("y")).abs() < 1e-9);
        assert!((got.z - w["z"].as_f64().expect("z")).abs() < 1e-9);
    }
    let _ = app.update(Message::SheetContours(E::Draw));
    let model = &app.document.as_ref().expect("a drawing").model;
    let d = model.road_drawings().last().expect("the drawing kept");
    assert_eq!(d.command, "road.terrains.sheetcontours");
    let kinds: Vec<_> = d.sources.iter().map(|s| (s.kind, s.id.as_str())).collect();
    use kentos_commands::RoadSourceKind as K;
    assert_eq!(kinds, vec![(K::Surface, "t-1"), (K::Layer, "bina"), (K::Layer, "yerlesik"), (K::Layer, "yol-ekseni")]);
}

/// docs/adr/4740 (m. 93(1)(c)): the control layer over the limit as Seçilen noktalar: the check is the shared case's,
/// said as a warning line.
#[test]
fn the_section_check_is_the_shared_case_s() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("terrain.sheetContours"));
    let _ = app.update(Message::SheetContours(E::ChosenLayer("kontrol-asan".into())));
    let _ = app.update(Message::SheetContours(E::Chosen(true)));
    let r = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    let c = r.check.expect("compared");
    let want = &case_output("m. 93(1)(c): farkların")["check"];
    assert_eq!((c.compared, c.corners, c.within), (4, 1, false));
    assert!((c.rms - want["rms"].as_f64().expect("rms")).abs() < 1e-9);
    assert!((c.limit - want["limit"].as_f64().expect("limit")).abs() < 1e-12);
}

/// docs/adr/4740: the window with Yerleşik alan ticked, the heights drawn in the built-up area, the window over the
/// limit of m. 93(1)(c), and the project tree with the kept drawing stale after a ground height changed; light wide and
/// dark narrow, `.run/gui-evidence/yerlesik-alan-4740/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib sheet_contours_window_tests::built_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn built_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yerlesik-alan-4740");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        // 1: m. 93(1)(c) over the limit.
        let _ = app.update(Message::Run("terrain.sheetContours"));
        let _ = app.update(Message::SheetContours(E::ChosenLayer("kontrol-asan".into())));
        let _ = app.update(Message::SheetContours(E::Chosen(true)));
        shot(&mut app, "1-kesit-siniri");
        // 2: Yerleşik alan ticked, the building alone cut.
        let _ = app.update(Message::SheetContours(E::Chosen(false)));
        for d in ["yol", "sev"] {
            let _ = app.update(Message::SheetContours(E::Detail(d.into(), false)));
        }
        let _ = app.update(Message::SheetContours(E::Built(true)));
        shot(&mut app, "2-pencere");
        // 3: drawn, the built-up area close.
        let _ = app.update(Message::SheetContours(E::Draw));
        let b = kentos_commands::Bounds { min_x: 175.0, min_y: 95.0, max_x: 345.0, max_y: 225.0 };
        app.navigating(|a| a.zoom_to_bounds(&b));
        shot(&mut app, "3-yerlesik-alan");
        // 4: a ground point raised: the kept drawing stale in the project tree.
        let k = {
            let model = &app.document.as_ref().expect("a drawing").model;
            model
                .entities()
                .filter(|e| e.base().layer_id == "zemin")
                .min_by(|a, b| {
                    let d = |e: &kentos_commands::Entity| match e {
                        kentos_commands::Entity::Point(p) => (p.p.x - 330.0).powi(2) + (p.p.y - 250.0).powi(2),
                        _ => f64::INFINITY,
                    };
                    d(a).total_cmp(&d(b))
                })
                .cloned()
                .expect("a ground point")
        };
        if let kentos_commands::Entity::Point(p) = &k {
            let uid = app.document.as_ref().and_then(|d| d.model.uid(kentos_domain::Slot(p.base.id))).expect("its uid");
            let input = serde_json::json!({"operation": "elevation", "changes": [{"kind": "update", "uid": uid.to_string(),
                "geometry": {"kind": "point", "p": {"x": p.p.x, "y": p.p.y}, "z": p.z.unwrap_or(0.0) + 0.6}}]});
            crate::program_scenes::command(&mut app, "cad.entities.edit", input);
        }
        app.docks.update(kentos_ui::widget::docking::Event::Selected(crate::app::Panel::RoadProject));
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "4-proje-agaci-eskidi");
    }
}

/// docs/adr/3520: the window with Nokta kotları ticked, the drawing after Çiz and close to the measured point; light
/// wide and dark narrow, `.run/gui-evidence/secilen-noktalar-3520/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib sheet_contours_window_tests::chosen_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn chosen_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/secilen-noktalar-3520");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("terrain.sheetContours"));
        let _ = app.update(Message::SheetContours(E::Chosen(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::SheetContours(E::Draw));
        let b = kentos_commands::Bounds { min_x: 50.0, min_y: 45.0, max_x: 115.0, max_y: 150.0 };
        app.navigating(|a| a.zoom_to_bounds(&b));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-cizimde-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // Close to the point with its own height (858.62 written 858.6, the surface 0.145 m off).
        let b = kentos_commands::Bounds { min_x: 84.0, min_y: 52.0, max_x: 116.0, max_y: 68.0 };
        app.navigating(|a| a.zoom_to_bounds(&b));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("3-yakin-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// `.run/gui-evidence/sev-kotlari-3120/`: the window with Şev kotları ticked, and the drawing at the scarp after Çiz.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib sheet_contours_window_tests::slope_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn slope_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/sev-kotlari-3120");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("terrain.sheetContours"));
        let _ = app.update(Message::SheetContours(E::Slopes(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::SheetContours(E::Draw));
        let b = kentos_commands::Bounds { min_x: 70.0, min_y: 22.0, max_x: 130.0, max_y: 84.0 };
        app.navigating(|a| a.zoom_to_bounds(&b));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-sevde-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// `.run/gui-evidence/tepe-kotu-2320/`: the window with the summit's height, and the drawing at the hilltop after Çiz.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib sheet_contours_window_tests::spot_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn spot_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tepe-kotu-2320");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("terrain.sheetContours"));
        let _ = app.update(Message::SheetContours(E::Spots(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let spot = app.sheet_contours.answer.clone().and_then(Result::ok).and_then(|r| r.spots.first().cloned());
        let _ = app.update(Message::SheetContours(E::Draw));
        if let Some(s) = spot {
            let b = kentos_commands::Bounds { min_x: s.x - 70.0, min_y: s.y - 45.0, max_x: s.x + 70.0, max_y: s.y + 45.0 };
            app.navigating(|a| a.zoom_to_bounds(&b));
        }
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-tepede-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

#[test]
fn harita_offers_it() {
    assert!(opened(Some(Module::Map)).offers("terrain.sheetContours"));
}

/// `.run/gui-evidence/pafta-egrileri-1820/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib sheet_contours_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/pafta-egrileri-1820");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("terrain.sheetContours"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::SheetContours(E::Draw));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-cizildi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
    // Where it sits: Arazi › Yüzey.
    let mut app = opened(Some(Module::Map));
    let size = Size::new(1600.0, 900.0);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::RibbonTab("terrain"));
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("3-serit-arazi.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}
